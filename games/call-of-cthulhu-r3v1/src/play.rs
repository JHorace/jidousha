//! The run: which screen is up, and what one command does to it.
//!
//! Plain data and one `step` function, with no world in it — the systems in
//! `main.rs` turn input into a `Command` and hand it here, and a check can
//! drive the same function directly. Every number a step applies comes from
//! `rules`.

use crate::lore::Being;
use crate::rules::{
    self, Action, BETTER_ENDING, DAYS, DayState, Night, Outcome, SANITY_START, answer_outcome,
};

/// What the player can do on any screen: pick a numbered row, or go on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// The numbered row, from 0.
    Choose(usize),
    /// Space, Enter, or a tap on a screen with nothing to choose.
    Continue,
}

/// A call in progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Call {
    /// Which of tonight's calls, from 0.
    pub(crate) slot: usize,
    pub(crate) being: Being,
    /// Exchanges left before it has said its piece.
    pub(crate) line: i32,
    pub(crate) anger: i32,
    /// Exchanges so far.
    pub(crate) asked: usize,
    /// Sanity this call has cost so far.
    pub(crate) cost: i32,
}

/// How a call ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ending {
    /// It said its piece and you hung up.
    HungUp,
    /// Its anger reached its temper.
    Wrath,
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fate {
    /// Sanity reached zero, on this day.
    Lost { day: u32 },
    /// Survived the last night, with this much sanity left.
    Won { sanity: i32 },
}

impl Fate {
    /// Whether this is the better of the two endings a win can have.
    pub(crate) fn better(self) -> bool {
        matches!(self, Fate::Won { sanity } if sanity >= BETTER_ENDING)
    }
}

/// Which screen is up — one value, so two screens cannot both be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Morning,
    Calling(Call),
    /// A call just ended; `last` is the final exchange's outcome.
    CallOver {
        call: Call,
        ending: Ending,
        last: Outcome,
    },
    End(Fate),
}

/// One whole run.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Run {
    pub(crate) state: DayState,
    pub(crate) sanity: i32,
    pub(crate) screen: Screen,
    /// Tonight's calls, fixed when the morning's choice is made.
    pub(crate) night: Night,
    /// Every answer given, as (being, the fact its question was about,
    /// outcome) — the run's own record, for the checks.
    pub(crate) answers: Vec<(Being, usize, Outcome)>,
}

impl Run {
    /// A fresh run on `seed`, at the first morning.
    pub(crate) fn new(seed: u64) -> Self {
        let state = DayState::opening(seed);
        Self {
            state,
            sanity: SANITY_START,
            screen: Screen::Morning,
            night: rules::tonight(&state),
            answers: Vec::new(),
        }
    }

    /// The morning's options, as the screen numbers them.
    pub(crate) fn options(&self) -> Vec<Action> {
        rules::morning_options(&self.state)
    }

    /// Apply one command. Commands a screen has no use for do nothing, which
    /// is what a key nobody asked for should do.
    pub(crate) fn step(&mut self, command: Command) {
        match (self.screen, command) {
            (Screen::Morning, Command::Choose(row)) => {
                let Some(action) = self.options().get(row).copied() else {
                    return;
                };
                self.state = rules::apply(&self.state, action);
                self.night = rules::tonight(&self.state);
                self.ring(0);
            }
            (Screen::Calling(call), Command::Choose(row)) if row < 3 => self.answer(call, row),
            (Screen::CallOver { call, .. }, Command::Continue) => {
                if call.slot + 1 < self.night.calls.len() {
                    self.ring(call.slot + 1);
                } else if self.state.day >= DAYS {
                    self.screen = Screen::End(Fate::Won {
                        sanity: self.sanity,
                    });
                } else {
                    self.state = rules::next_day(&self.state);
                    self.night = rules::tonight(&self.state);
                    self.screen = Screen::Morning;
                }
            }
            (Screen::End(_), Command::Continue) => *self = Run::new(self.state.seed),
            _ => {}
        }
    }

    /// Put tonight's `slot`th call on the line.
    fn ring(&mut self, slot: usize) {
        let being = self.night.calls[slot].being;
        self.screen = Screen::Calling(Call {
            slot,
            being,
            line: being.spec().line,
            anger: 0,
            asked: 0,
            cost: 0,
        });
    }

    /// Answer the question on the line with the answer shown at `row`.
    fn answer(&mut self, mut call: Call, row: usize) {
        let id = self.night.calls[call.slot].question(call.asked);
        let outcome = answer_outcome(id, row, call.anger, self.state.composure);
        self.answers.push((call.being, id.fact, outcome));
        call.line += outcome.line_change;
        call.anger += outcome.anger_change;
        call.asked += 1;
        call.cost += outcome.sanity_cost;
        self.sanity -= outcome.sanity_cost;
        self.screen = if self.sanity <= 0 {
            self.sanity = 0;
            Screen::End(Fate::Lost {
                day: self.state.day,
            })
        } else if outcome.wrath {
            Screen::CallOver {
                call,
                ending: Ending::Wrath,
                last: outcome,
            }
        } else if call.line <= 0 {
            Screen::CallOver {
                call,
                ending: Ending::HungUp,
                last: outcome,
            }
        } else {
            Screen::Calling(call)
        };
    }
}
