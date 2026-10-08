//! The three players a `--verify` match is played by, and the match runner.
//!
//! - **reader** — reads the stack two moves deep: each of its moves, the
//!   rival's best reply to it (`rival::best_choice`), then `preview`.
//! - **raw power** — what a first try looks like: plays the hardest-hitting
//!   card it can afford, never touches the stack. The design claims this loses.
//! - **idle** — passes every time, which proves the duel can be lost.
//!
//! Every player presses keys through a `SnapshotBuilder`, so its moves go
//! through the same input path a keyboard does. Each prints the three numbers
//! (docs/api/jidousha-controllers.md): how often it answered the stack, what
//! margin its moves were planned to reach, and how far the margins it got
//! landed from the plan.
//!
//! Key items: `Player`, `MatchReport`, `play_match`, `choice_for`.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder};

use crate::rival::{Choice, apply, best_choice, options, score_now};
use crate::rules::{Card, Duel, Outcome, Side, preview};
use crate::{SLOT_KEYS, Table, WINDOW, config, register};

/// Ticks a match may take before the run calls it stuck.
pub(crate) const MATCH_TICKS: u64 = 40_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Player {
    Reader,
    RawPower,
    Idle,
}

impl Player {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Player::Reader => "reader",
            Player::RawPower => "raw power",
            Player::Idle => "idle",
        }
    }
}

/// The reader's rule: each move, answered by the rival's best reply, scored
/// by where the stack then resolves to.
fn reader_choice(duel: &Duel) -> (Choice, Duel) {
    let mut best = (Choice::Pass, i32::MIN, duel.clone());
    for choice in options(duel, Side::You) {
        let mine = apply(duel, Side::You, choice);
        let answered = if mine.priority == Side::Rival && mine.outcome.is_none() {
            apply(&mine, Side::Rival, best_choice(&mine, Side::Rival))
        } else {
            mine
        };
        let spent = match choice {
            Choice::Play { index, .. } => duel.you.hand[index].cost(),
            Choice::Pass => 0,
        };
        let score = score_now(&answered, Side::You) - spent;
        if score > best.1 {
            best = (choice, score, answered);
        }
    }
    (best.0, best.2)
}

/// Raw power: the most damage it can afford, else Ward, else pass.
fn raw_choice(duel: &Duel) -> Choice {
    let me = &duel.you;
    let pick = [Card::Haymaker, Card::Strike, Card::Ward]
        .into_iter()
        .find_map(|want| {
            me.hand
                .iter()
                .position(|&card| card == want && card.cost() <= me.focus)
        });
    match pick {
        Some(index) => Choice::Play {
            index,
            target: None,
        },
        None => Choice::Pass,
    }
}

/// What `player` does with priority, and the duel it expects that to leave.
pub(crate) fn choice_for(player: Player, duel: &Duel) -> (Choice, Duel) {
    let choice = match player {
        Player::Reader => return reader_choice(duel),
        Player::RawPower => raw_choice(duel),
        Player::Idle => Choice::Pass,
    };
    (choice, apply(duel, Side::You, choice))
}

/// The life margin (yours minus the rival's) `duel` resolves to.
pub(crate) fn margin(duel: &Duel) -> i32 {
    let ahead = preview(duel);
    ahead.you_life - ahead.rival_life
}

/// What one match did.
pub(crate) struct MatchReport {
    pub(crate) outcome: Option<Outcome>,
    pub(crate) rounds: u32,
    pub(crate) ticks: u64,
    pub(crate) you_life: i32,
    pub(crate) rival_life: i32,
    /// Windows where a rival item sat on the stack and a reply was affordable.
    pub(crate) windows: u32,
    /// Of those, how many it answered with a card.
    pub(crate) answered: u32,
    /// Mean margin its moves were planned to resolve to.
    pub(crate) planned: f32,
    /// Mean distance between a planned margin and the margin the stack
    /// actually emptied at.
    pub(crate) landed_off: f32,
    /// The last frame drawn while the stack held three or more items.
    pub(crate) busy_frame: Option<(FrameRecord, Table)>,
    /// The frame the result screen was first drawn on.
    pub(crate) result_frame: Option<(FrameRecord, Table)>,
    /// Every resolution, for the determinism check.
    pub(crate) resolved: usize,
    pub(crate) log_digest: u64,
}

fn tap(keyboard: &mut SnapshotBuilder, key: Key) {
    keyboard.record(InputEvent::KeyPressed(key));
    keyboard.record(InputEvent::KeyReleased(key));
}

fn copy_table(table: &Table) -> Table {
    Table {
        duel: table.duel.clone(),
        choosing: table.choosing,
        rival_clock: table.rival_clock,
    }
}

/// Play one whole match with `player` on `seed`; frames only when `record`.
pub(crate) fn play_match(player: Player, seed: u64, record: bool) -> MatchReport {
    let mut sim = headless(GameConfig { seed, ..config() }, register);
    let mut recorder = record.then(|| FrameRecorder::new(WINDOW));
    let mut keyboard = SnapshotBuilder::new();
    let mut aim: Option<usize> = None;
    let (mut windows, mut answered) = (0, 0);
    let mut plans: Vec<i32> = Vec::new();
    let mut misses: Vec<i32> = Vec::new();
    let mut pending: Option<i32> = None;
    let mut busy_frame = None;
    let mut result_frame = None;
    let mut ticks = 0;

    for tick in 1..=MATCH_TICKS {
        ticks = tick;
        if let Some(table) = sim.world().find_resource::<Table>() {
            let duel = &table.duel;
            if duel.outcome.is_none() && duel.priority == Side::You {
                if let Some(slot) = aim.take() {
                    tap(&mut keyboard, SLOT_KEYS[slot]);
                } else {
                    let (choice, expected) = choice_for(player, duel);
                    let plan = margin(&expected);
                    let rival_item = duel.stack.iter().any(|i| i.controller == Side::Rival);
                    let can_reply = options(duel, Side::You).len() > 1;
                    if rival_item && can_reply {
                        windows += 1;
                        answered += u32::from(choice != Choice::Pass);
                    }
                    if choice != Choice::Pass {
                        plans.push(plan);
                        pending = Some(plan);
                    }
                    match choice {
                        Choice::Play { index, target } => {
                            tap(&mut keyboard, crate::CARD_KEYS[index]);
                            aim = target.and_then(|id| {
                                duel.stack.iter().rev().position(|item| item.id == id)
                            });
                        }
                        Choice::Pass => tap(&mut keyboard, Key::Space),
                    }
                }
            }
        }
        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();

        let table = sim.world().resource::<Table>();
        if let Some(plan) = pending
            && table.duel.stack.is_empty()
        {
            misses.push((table.duel.you.life - table.duel.rival.life - plan).abs());
            pending = None;
        }
        if let Some(recorder) = recorder.as_mut() {
            let busy = table.duel.stack.len() >= 3;
            let over = table.duel.outcome.is_some();
            if busy || (over && result_frame.is_none()) {
                let copy = copy_table(sim.world().resource::<Table>());
                let frame = recorder.draw(&mut sim);
                if over {
                    result_frame = Some((frame, copy));
                } else {
                    busy_frame = Some((frame, copy));
                }
            }
        }
        if sim.world().resource::<Table>().duel.outcome.is_some() {
            break;
        }
    }

    let duel = &sim.world().resource::<Table>().duel;
    let mean = |values: &[i32]| {
        if values.is_empty() {
            0.0
        } else {
            values.iter().sum::<i32>() as f32 / values.len() as f32
        }
    };
    MatchReport {
        outcome: duel.outcome,
        rounds: duel.round,
        ticks,
        you_life: duel.you.life,
        rival_life: duel.rival.life,
        windows,
        answered,
        planned: mean(&plans),
        landed_off: mean(&misses),
        busy_frame,
        result_frame,
        resolved: duel.resolved.len(),
        log_digest: digest(duel),
    }
}

/// A fingerprint of every resolution, for "the same seed plays the same match".
fn digest(duel: &Duel) -> u64 {
    let mut hash: u64 = 1469598103934665603;
    for step in &duel.resolved {
        for byte in format!("{:?}", step).bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(1099511628211);
        }
    }
    hash
}
