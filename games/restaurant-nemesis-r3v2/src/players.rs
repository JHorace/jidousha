//! The scripted players and the session that runs them.
//!
//! Three players (DESIGN.md §Gates, G4): the **sleeper** only presses Enter
//! (the run can be lost); the **order-taker** serves rows in order and never
//! spends (a first try); the **greedy chef** serves nemeses first, then the
//! worst failures, and spends on the most-followed nemesis. A **scenario**
//! player — the greedy chef's service, a fixed card key, Enter every night —
//! drives the decision-row gates.
//!
//! Every key goes through a `SnapshotBuilder`: pressed on one tick, released
//! on the next, so the game reads it through the real input path.
//!
//! Key items: `Session`, `Report`, `play`, `greedy`, `order_taker`, `sleeper`,
//! `scenario`.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder};

use crate::input::KEYS;
use crate::rules::{Consequence, defeat_progress, outcome_of, severity};
use crate::screen::{Art, screen};
use crate::sim::*;
use crate::{Seed, WINDOW, config, register};
use jidousha::ui::Panel;

/// Ticks a session may take before the run calls it stuck.
pub(crate) const SESSION_TICKS: u64 = 2000;

/// A policy: the next key a player presses, or none.
pub(crate) type Policy<'a> = &'a dyn Fn(&Game) -> Option<Key>;

fn digit(index: usize) -> Key {
    KEYS[index]
}

/// The greedy chef's service rule, with `card` pressed on an open card.
fn greedy_service(game: &Game, card: &dyn Fn(&Game, &Nemesis) -> Key) -> Option<Key> {
    let Phase::Service { focus } = game.phase else {
        return None;
    };
    if let Some(row) = focus {
        let nemesis = match game.queue[row].kind {
            Kind::Nemesis(id) => game.find_nemesis(id),
            _ => None,
        };
        return Some(match nemesis {
            Some(n) => card(game, n),
            None => Key::Escape,
        });
    }
    let cap = game.capacity_left;
    let nemesis_row = game.queue.iter().position(|o| {
        !o.served
            && matches!(o.kind, Kind::Nemesis(id) if game.find_nemesis(id).is_some())
            && o.customer.need <= cap
    });
    if let Some(row) = nemesis_row {
        return Some(digit(row));
    }
    let best = game
        .queue
        .iter()
        .enumerate()
        .filter(|(_, o)| !o.served && !matches!(o.kind, Kind::Nemesis(_)))
        .filter(|(_, o)| o.customer.need <= cap)
        .max_by_key(|(row, o)| (severity(&o.customer), usize::MAX - row));
    Some(best.map_or(Key::Enter, |(row, _)| digit(row)))
}

/// The greedy chef.
pub(crate) fn greedy(game: &Game) -> Option<Key> {
    match game.phase {
        Phase::Service { .. } => greedy_service(game, &|game, n| {
            let progress = defeat_progress(n, game.capacity_left);
            if progress.overwhelm.is_some() && game.money >= 50 {
                Key::O
            } else {
                Key::S
            }
        }),
        Phase::Ledger => {
            let most = game
                .nemeses
                .iter()
                .enumerate()
                .max_by_key(|(slot, n)| (n.followers, usize::MAX - slot))
                .map(|(slot, _)| slot);
            if let Some(slot) = most
                && game.money >= SOCIAL_COST + 15
            {
                return Some(digit(slot));
            }
            if game.preps_tonight == 0 && game.money >= 40 {
                return Some(Key::P);
            }
            Some(Key::Enter)
        }
        Phase::Over { .. } => None,
    }
}

/// The order-taker: rows in order while capacity allows, never spends.
pub(crate) fn order_taker(game: &Game) -> Option<Key> {
    match game.phase {
        Phase::Service { focus: Some(_) } => Some(Key::S),
        Phase::Service { focus: None } => {
            let cap = game.capacity_left;
            let next = game.queue.iter().position(|o| {
                let live = match o.kind {
                    Kind::Nemesis(id) => game.find_nemesis(id).is_some(),
                    _ => true,
                };
                !o.served && live && o.customer.need <= cap
            });
            Some(next.map_or(Key::Enter, digit))
        }
        Phase::Ledger => Some(Key::Enter),
        Phase::Over { .. } => None,
    }
}

/// The sleeper: Enter, always.
pub(crate) fn sleeper(game: &Game) -> Option<Key> {
    match game.phase {
        Phase::Over { .. } => None,
        _ => Some(Key::Enter),
    }
}

/// The scenario player: the greedy chef's service with `card` on the card,
/// Enter every night.
pub(crate) fn scenario(card: Key) -> impl Fn(&Game) -> Option<Key> {
    move |game| match game.phase {
        Phase::Service { .. } => greedy_service(game, &|_, _| card),
        Phase::Ledger => Some(Key::Enter),
        Phase::Over { .. } => None,
    }
}

/// One headless run of the real game, keys through the real input path.
pub(crate) struct Session {
    pub(crate) sim: HeadlessSim,
    recorder: FrameRecorder,
    keyboard: SnapshotBuilder,
    held: Option<Key>,
    pub(crate) ticks: u64,
}

impl Session {
    pub(crate) fn new(seed: u64) -> Self {
        let mut sim = headless(config(seed), register);
        sim.world_mut().insert_resource(Seed(seed));
        let mut session = Self {
            sim,
            recorder: FrameRecorder::new(WINDOW),
            keyboard: SnapshotBuilder::new(),
            held: None,
            ticks: 0,
        };
        session.step();
        session
    }

    pub(crate) fn game(&self) -> &Game {
        self.sim.world().resource::<Game>()
    }

    pub(crate) fn game_mut(&mut self) -> &mut Game {
        self.sim.world_mut().resource_mut::<Game>()
    }

    fn step(&mut self) {
        let snapshot = self.keyboard.first_tick_snapshot();
        self.sim.world_mut().insert_resource(Input::new(snapshot));
        self.sim.tick();
        self.ticks += 1;
    }

    /// Press `key` on one tick and release it on the next.
    pub(crate) fn press(&mut self, key: Key) {
        if let Some(held) = self.held.take() {
            self.keyboard.record(InputEvent::KeyReleased(held));
        }
        self.keyboard.record(InputEvent::KeyPressed(key));
        self.held = Some(key);
        self.step();
        self.keyboard.record(InputEvent::KeyReleased(key));
        self.held = None;
        self.step();
    }

    /// The frame the game draws now, and the panel it says.
    pub(crate) fn frame(&mut self) -> (FrameRecord, Panel<Art>) {
        let panel = screen(self.game());
        (self.recorder.draw(&mut self.sim), panel)
    }

    pub(crate) fn font(&self) -> jidousha::testing::BackendTextureId {
        self.recorder.font_texture()
    }

    /// Let `policy` play until it stops, the game is over, or `until` holds.
    pub(crate) fn run_until(
        &mut self,
        policy: Policy,
        until: &dyn Fn(&Game) -> bool,
        stats: &mut Stats,
    ) -> bool {
        while self.ticks < SESSION_TICKS {
            if until(self.game()) {
                return true;
            }
            let Some(key) = policy(self.game()) else {
                return true;
            };
            stats.observe(self.game(), key);
            self.press(key);
        }
        false
    }
}

/// The three numbers (DESIGN.md G4), gathered at every close of the kitchen.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Stats {
    pub(crate) served: u32,
    pub(crate) orders: u32,
    pub(crate) risky_unmet: u32,
    pub(crate) used: u32,
    pub(crate) capacity: u32,
    day: u32,
    opened_with: u32,
}

impl Stats {
    fn observe(&mut self, game: &Game, key: Key) {
        let Phase::Service { focus: None } = game.phase else {
            return;
        };
        if game.day != self.day {
            self.day = game.day;
            self.opened_with = game.capacity_left;
        }
        if key != Key::Enter {
            return;
        }
        self.orders += game.queue.len() as u32;
        self.served += game.queue.iter().filter(|o| o.served).count() as u32;
        self.risky_unmet += game
            .queue
            .iter()
            .filter(|o| !o.served)
            .filter(|o| outcome_of(o, game).if_unmet.consequence != Consequence::Nothing)
            .count() as u32;
        self.used += self.opened_with - game.capacity_left;
        self.capacity += self.opened_with;
    }
}

/// What one whole run did.
pub(crate) struct Report {
    pub(crate) game: Game,
    pub(crate) finished: bool,
    pub(crate) ticks: u64,
    pub(crate) stats: Stats,
}

/// Play a whole run of `policy` on `seed`.
pub(crate) fn play(seed: u64, policy: Policy) -> Report {
    let mut session = Session::new(seed);
    let mut stats = Stats::default();
    let finished = session.run_until(
        policy,
        &|g| matches!(g.phase, Phase::Over { .. }),
        &mut stats,
    ) && matches!(session.game().phase, Phase::Over { .. });
    Report {
        game: session.game().clone(),
        finished,
        ticks: session.ticks,
        stats,
    }
}
