//! `--verify`: the game played headless by three players, the decision rows
//! asserted on fixed scenarios, every screen judged, one frame captured.
//!
//! The order of the run: the tables and the rules' contracts (no world), the
//! schedule, the three players' matches, the reader over more seeds, the three
//! decision-row scenarios (`scenarios.rs`), the result screens staged, and the
//! picture. The verdict line, then one indented line per fact.

use std::collections::VecDeque;
use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder,
};
use jidousha::ui::{frame_text_floor, judge_frame, judge_panel};

use crate::cards::Card;
use crate::checks::Checks;
use crate::duel::{Duel, Event, Outcome, Side, legal_targets, preview};
use crate::npc::{self, Action};
use crate::screen::{self, Flat, Ui};
use crate::{Game, config, register};

use crate::scenarios;

/// The most ticks a match may take before the run calls it stuck.
const MATCH_TICKS: u64 = 40_000;
/// Seeds the reader plays beyond the first, and how many it must win.
const READER_SEEDS: u64 = 6;
const READER_MUST_WIN: usize = 5;

/// Who presses the keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Player {
    /// Reads the stack: rolls each option forward with the Brute's own rule.
    Reader,
    /// A first-timer: plays its biggest threat on an empty stack, never answers.
    Hitter,
    /// Passes everything.
    Idle,
}

/// Presses keys and taps one at a time: down on one tick, up on the next.
pub(crate) struct Driver {
    builder: SnapshotBuilder,
    queue: VecDeque<InputEvent>,
    up: Option<InputEvent>,
}

impl Driver {
    pub(crate) fn new() -> Driver {
        Driver {
            builder: SnapshotBuilder::new(),
            queue: VecDeque::new(),
            up: None,
        }
    }

    pub(crate) fn idle(&self) -> bool {
        self.queue.is_empty() && self.up.is_none()
    }

    pub(crate) fn key(&mut self, key: Key) {
        self.queue.push_back(InputEvent::KeyPressed(key));
    }

    /// A tap at a design-space point, through the camera the game draws with.
    pub(crate) fn tap(&mut self, at: Vec2) {
        let screen = screen::camera().world_to_screen(at);
        self.queue.push_back(InputEvent::PointerMoved {
            id: PointerId::PRIMARY,
            screen,
        });
        self.queue.push_back(InputEvent::ButtonPressed {
            id: PointerId::PRIMARY,
            button: PointerButton::Primary,
        });
    }

    /// The keys that make the game take `action` for the Weaver.
    pub(crate) fn act(&mut self, duel: &Duel, action: Action) {
        const SLOTS: [Key; 6] = [
            Key::Digit1,
            Key::Digit2,
            Key::Digit3,
            Key::Digit4,
            Key::Digit5,
            Key::Digit6,
        ];
        match action {
            Action::Pass => self.key(Key::Space),
            Action::Play { slot, target } => {
                self.key(SLOTS[slot]);
                if let Some(id) = target {
                    let card = duel.seat(Side::You).hand[slot];
                    let place = legal_targets(duel, card)
                        .iter()
                        .position(|mark| *mark == id)
                        .unwrap_or(0);
                    for _ in 0..place {
                        self.key(Key::ArrowDown);
                    }
                }
                self.key(Key::Enter);
            }
        }
    }

    /// One tick: release what was pressed last tick, or press the next thing.
    pub(crate) fn tick(&mut self, sim: &mut HeadlessSim) {
        if let Some(up) = self.up.take() {
            self.builder.record(up);
        } else if let Some(event) = self.queue.pop_front() {
            self.up = match event {
                InputEvent::KeyPressed(key) => Some(InputEvent::KeyReleased(key)),
                InputEvent::ButtonPressed { id, button } => {
                    Some(InputEvent::ButtonReleased { id, button })
                }
                _ => None,
            };
            self.builder.record(event);
        }
        let snapshot = self.builder.first_tick_snapshot();
        sim.world_mut().insert_resource(Input::new(snapshot));
        sim.tick();
    }
}

/// What a player would do with priority.
pub(crate) fn decide(player: Player, duel: &Duel) -> Action {
    match player {
        Player::Idle => Action::Pass,
        Player::Hitter => hitter(duel),
        Player::Reader => reader(duel),
    }
}

fn hitter(duel: &Duel) -> Action {
    if !duel.stack.is_empty() {
        return Action::Pass;
    }
    let hand = &duel.seat(Side::You).hand;
    let threat = [Card::Blast, Card::Surge, Card::Bolt]
        .iter()
        .find_map(|card| {
            (0..hand.len()).find(|&slot| hand[slot] == *card && duel.playable(Side::You, slot))
        });
    threat.map_or(Action::Pass, |slot| Action::Play { slot, target: None })
}

/// Roll every option forward — the Brute by its own rule, the Weaver greedily
/// — to the end of the turn, and take the best lead.
fn reader(duel: &Duel) -> Action {
    let mut best = Action::Pass;
    let mut best_lead = i32::MIN;
    for action in npc::options(duel, Side::You) {
        let Some(mut ahead) = npc::after(duel, Side::You, action) else {
            continue;
        };
        let turn = ahead.turn;
        for _ in 0..80 {
            if ahead.over.is_some() || ahead.turn != turn {
                break;
            }
            let holder = ahead.priority;
            let next = npc::choose(&ahead, holder);
            match npc::after(&ahead, holder, next) {
                Some(moved) => ahead = moved,
                None => break,
            }
        }
        let [you, npc] = ahead.life();
        let lead = you - npc;
        if lead > best_lead {
            best = action;
            best_lead = lead;
        }
    }
    best
}

/// One frame, drawn into a recorder of its own so a long match keeps none.
pub(crate) fn snap(sim: &mut HeadlessSim) -> (FrameRecord, BackendTextureId) {
    let mut recorder = FrameRecorder::new(screen::WINDOW);
    let frame = recorder.draw(sim);
    (frame, recorder.font_texture())
}

/// A frame and the state it was drawn from.
pub(crate) struct Shot {
    pub(crate) frame: FrameRecord,
    pub(crate) font: BackendTextureId,
    pub(crate) duel: Duel,
    pub(crate) ui: Ui,
}

pub(crate) fn shoot(sim: &mut HeadlessSim) -> Shot {
    let (frame, font) = snap(sim);
    let game = sim.world().resource::<Game>();
    Shot {
        frame,
        font,
        duel: game.duel.clone(),
        ui: game.ui.clone(),
    }
}

/// What one match did.
pub(crate) struct Report {
    pub(crate) outcome: Option<Outcome>,
    pub(crate) life: [i32; 2],
    pub(crate) turns: u32,
    pub(crate) ticks: u64,
    /// Resolutions whose step equalled the preview's first step at the window before.
    pub(crate) matched: usize,
    pub(crate) resolutions: usize,
    /// Brute damage items played, and how many the Weaver aimed an answer at.
    pub(crate) threats: usize,
    pub(crate) answered: usize,
    pub(crate) live: Option<Shot>,
    pub(crate) end: Option<Shot>,
    pub(crate) log: Vec<Event>,
}

/// Play one whole match from `GameConfig::seed`, through the real Startup.
pub(crate) fn play_match(seed: u64, player: Player, shots: bool) -> Report {
    let mut sim = headless(GameConfig { seed, ..config() }, register);
    let mut driver = Driver::new();
    let mut report = Report {
        outcome: None,
        life: [0, 0],
        turns: 0,
        ticks: 0,
        matched: 0,
        resolutions: 0,
        threats: 0,
        answered: 0,
        live: None,
        end: None,
        log: Vec::new(),
    };
    for tick in 1..=MATCH_TICKS {
        let before = sim
            .world()
            .find_resource::<Game>()
            .map(|game| (preview(&game.duel), game.duel.log.len()));
        if let Some(game) = sim.world().find_resource::<Game>() {
            let duel = &game.duel;
            if duel.over.is_none()
                && duel.priority == Side::You
                && driver.idle()
                && game.ui.pick.is_none()
            {
                driver.act(duel, decide(player, duel));
            }
        }
        driver.tick(&mut sim);
        let game = sim.world().resource::<Game>();
        if let Some((ahead, seen)) = before {
            for event in &game.duel.log[seen..] {
                if let Event::Resolved(step) = event {
                    report.resolutions += 1;
                    report.matched += usize::from(ahead.first() == Some(step));
                }
            }
        }
        report.ticks = tick;
        if game.duel.over.is_some() {
            break;
        }
        if shots && tick % 120 == 0 {
            report.live = Some(shoot(&mut sim));
        }
    }
    let game = sim.world().resource::<Game>();
    report.outcome = game.duel.over;
    report.life = game.duel.life();
    report.turns = game.duel.turn;
    report.log = game.duel.log.clone();
    let threats: Vec<u32> = report
        .log
        .iter()
        .filter_map(|event| match *event {
            Event::Played {
                side: Side::Npc,
                card,
                item,
                ..
            } if card.deals_damage() => Some(item),
            _ => None,
        })
        .collect();
    report.threats = threats.len();
    report.answered = threats
        .iter()
        .filter(|id| {
            report.log.iter().any(|event| {
                matches!(*event, Event::Played { side: Side::You, target: Some(t), .. } if t == **id)
            })
        })
        .count();
    if shots {
        report.end = Some(shoot(&mut sim));
    }
    report
}

/// Judge one shot: the floors on its panel, the panel on its frame, the frame
/// inside the camera, the glyph floor, the clear colour. Returns the clearance.
pub(crate) fn judge(checks: &mut Checks, label: &str, shot: &Shot) -> f32 {
    let panel = screen::panel(&shot.duel, &shot.ui);
    let controls: Vec<(String, Rect)> = screen::spots(&shot.duel)
        .into_iter()
        .filter(|(spot, _)| *spot == screen::Spot::Pass)
        .map(|(spot, rect)| (format!("{spot:?}"), rect))
        .collect();
    for breach in judge_panel(&panel, &screen::FLOORS, &controls, &[]) {
        checks.require(
            false,
            "a screen breaks a readability floor",
            format!("{label}: {} - {}", breach.what, breach.detail),
        );
    }
    let view = screen::camera().visible_bounds();
    for breach in judge_frame(&panel, &shot.frame, shot.font, &Flat, view) {
        checks.require(
            false,
            "a row the panel names is not on the frame",
            format!("{label}: {} - {}", breach.what, breach.detail),
        );
    }
    for breach in frame_text_floor(&shot.frame, shot.font, screen::FLOORS.min_text) {
        checks.require(
            false,
            "a glyph is drawn below the floor",
            format!("{label}: {} - {}", breach.what, breach.detail),
        );
    }
    for text in panel.all_strings() {
        checks.require(
            !text.ends_with("..."),
            "a row on screen is cut short",
            format!("{label}: {text:?}"),
        );
        checks.require(
            text.chars().all(|c| (' '..='~').contains(&c)),
            "a string the font cannot draw",
            format!("{label}: {text:?}"),
        );
    }
    let mut clearance = f32::MAX;
    for quad in shot.frame.quads() {
        let bounds = quad.bounds();
        checks.require(
            view.contains_rect(bounds),
            "something is drawn off screen",
            format!("{label}: {bounds:?} against a camera showing {view:?}"),
        );
        let gap = (bounds.min - view.min).min(view.max - bounds.max);
        clearance = clearance.min(gap.x.min(gap.y));
    }
    let cleared = shot.frame.plan.clear_color;
    let brightest = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        brightest < 0.2 && cleared.a > 0.99,
        "the table is not dark enough for light text to read on",
        format!(
            "{label}: brightest channel {brightest:.3} at alpha {:.2}",
            cleared.a
        ),
    );
    clearance
}

pub(crate) fn run() -> ExitCode {
    let mut checks = Checks::default();
    scenarios::tables(&mut checks);
    scenarios::contracts(&mut checks);

    // The schedule: the player's input before the Brute's, as `register` says.
    let sim = headless(config(), register);
    let order = sim.schedule_debug();
    let player = order.find("player_acts");
    let brute = order.find("npc_acts");
    checks.require(
        player.is_some() && brute.is_some() && player < brute,
        "the player's input no longer runs before the Brute",
        format!("player_acts at {player:?}, npc_acts at {brute:?} in\n{order}"),
    );

    let reader = play_match(0, Player::Reader, true);
    let hitter = play_match(0, Player::Hitter, false);
    let idle = play_match(0, Player::Idle, false);
    for (name, report) in [("reader", &reader), ("hitter", &hitter), ("idle", &idle)] {
        checks.require(
            report.outcome.is_some(),
            "a match never reached its result",
            format!(
                "{name}: {} ticks, turn {}, life {:?}",
                report.ticks, report.turns, report.life
            ),
        );
        checks.require(
            report.matched == report.resolutions,
            "a resolution did something its preview did not say",
            format!(
                "{name}: {} of {} resolutions matched the preview at the window before",
                report.matched, report.resolutions
            ),
        );
    }
    checks.require(
        reader.outcome == Some(Outcome::Won(Side::You)),
        "the stack reader does not beat the Brute",
        format!(
            "{:?}, life {:?} after turn {}",
            reader.outcome, reader.life, reader.turns
        ),
    );
    checks.require(
        idle.outcome == Some(Outcome::Won(Side::Npc)),
        "a player who never plays does not lose",
        format!("{:?}, life {:?}", idle.outcome, idle.life),
    );
    let again = play_match(0, Player::Reader, false);
    checks.require(
        again.log == reader.log,
        "the same seed and the same player made a different match",
        format!("{} events, then {}", reader.log.len(), again.log.len()),
    );
    let mut wins = 0;
    let mut hitter_wins = 0;
    for seed in 1..=READER_SEEDS {
        let report = play_match(seed, Player::Reader, false);
        wins += usize::from(report.outcome == Some(Outcome::Won(Side::You)));
        let first = play_match(seed, Player::Hitter, false);
        hitter_wins += usize::from(first.outcome == Some(Outcome::Won(Side::You)));
    }
    checks.require(
        wins >= READER_MUST_WIN && wins > hitter_wins,
        "reading the stack is not what wins",
        format!(
            "the reader won {wins} of {READER_SEEDS} seeds and the hitter {hitter_wins}; \
             the reader must win {READER_MUST_WIN} and more than the hitter"
        ),
    );

    let mut clearance = f32::MAX;
    if let Some(live) = &reader.live {
        clearance = clearance.min(judge(&mut checks, "the reader's last live frame", live));
    }
    if let Some(end) = &reader.end {
        clearance = clearance.min(judge(&mut checks, "the reader's result screen", end));
        checks.require(
            end.duel.over.is_some()
                && screen::panel(&end.duel, &end.ui)
                    .all_strings()
                    .any(|text| text == screen::AGAIN),
            "the match's last frame is not its result screen",
            format!("over: {:?}", end.duel.over),
        );
    }
    let rows = scenarios::decision_rows(&mut checks);
    let staged = scenarios::result_screens(&mut checks);
    clearance = clearance.min(staged.1);

    let picture = match &rows.picture {
        Some(shot) => crate::capture::capture_a_frame(&mut checks, &shot.frame, shot.font),
        None => "skipped, no scenario frame to draw".to_owned(),
    };

    if checks.failed() == 0 {
        println!("verified stack_card_game_r2: {} checks", checks.asked);
    } else {
        println!(
            "verify stack_card_game_r2 FAILED: {} of {} checks",
            checks.failed(),
            checks.asked
        );
    }
    for (name, report) in [("reader", &reader), ("hitter", &hitter), ("idle", &idle)] {
        println!(
            "  {name}: {:?} WEAVER {} - BRUTE {} after turn {} ({} ticks); answered {} of {} \
             Brute threats; {} of {} resolutions matched the preview",
            report.outcome,
            report.life[0],
            report.life[1],
            report.turns,
            report.ticks,
            report.answered,
            report.threats,
            report.matched,
            report.resolutions
        );
    }
    println!("  reader over seeds 1..={READER_SEEDS}: won {wins}; hitter won {hitter_wins}");
    for line in &rows.lines {
        println!("  {line}");
    }
    println!("  result screens: {}", staged.0);
    println!("  closest quad to the edge: {clearance:.2} world units");
    println!("  capture: {picture}");
    checks.verdict()
}
