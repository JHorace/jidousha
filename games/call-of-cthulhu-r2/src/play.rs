//! The state machine: a run's phases and the moves between them, on plain data.
//!
//! Nothing here reads input or a `World`. The input system turns a key or a tap
//! into one of these calls, and a check can make the same calls directly — so
//! what a scripted player does and what a person does go through one door.

use jidousha::prelude::*;

use crate::lore::{AnswerKind, Being};
use crate::rules::{self, DAYS, Night, Outcome, Run, SILENCE_TICKS, SOUND_SLEEP, START_INTEREST};

/// Which screen the run is on, and what that screen holds — one value.
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// Choosing the morning's one action.
    Morning,
    /// On the line.
    Call(CallState),
    /// A call has just ended; waiting for the player to go on.
    Hangup(Hangup),
    /// The run is over.
    Ended(Ending),
}

/// One call in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct CallState {
    /// Which of tonight's calls this is.
    pub slot: usize,
    pub being: Being,
    /// How much more it wants to talk; the call ends at zero.
    pub interest: i32,
    /// How many questions it has asked so far (the next is `asked`).
    pub asked: usize,
    /// Ticks since the last answer, toward the silence cost.
    pub quiet: u32,
    /// Sanity this call has cost so far.
    pub spent: i32,
    pub exchanges: u32,
    /// What happened on the last exchange, for the screen.
    pub last: Option<String>,
}

/// The screen between a call and what follows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Hangup {
    pub slot: usize,
    pub being: Being,
    pub spent: i32,
    pub exchanges: u32,
    pub last: String,
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// Sanity ran out on this day, on this being's call.
    Lost { day: u32, being: Being },
    /// Survived the last night with this much sanity.
    Won { sanity: i32 },
}

/// The whole game: the run, tonight's plan, and the screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    pub run: Run,
    /// Tonight's calls, fixed when the morning is chosen — `rules::tonight`.
    pub night: Night,
    pub phase: Phase,
    /// What happened, one line per event, oldest first — the run's transcript.
    pub log: Vec<String>,
}
impl Resource for Game {}

/// The question being asked on a call, and its answers in screen order.
#[derive(Clone, Debug, PartialEq)]
pub struct Asking {
    pub fact: usize,
    pub order: [AnswerKind; 3],
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let run = Run::new(seed);
        let night = rules::tonight(&run);
        Self {
            run,
            night,
            phase: Phase::Morning,
            log: vec![format!(
                "run seed {seed}: day 1 begins with sanity {}",
                rules::START_SANITY
            )],
        }
    }

    /// The question on the line now, if a call is in progress.
    pub fn asking(&self) -> Option<Asking> {
        let Phase::Call(call) = &self.phase else {
            return None;
        };
        let planned = self.night.calls.get(call.slot)?;
        let fact = *planned
            .questions
            .get(call.asked % planned.questions.len().max(1))?;
        let order = rules::answer_order(self.run.seed, self.run.day, call.being, call.asked);
        Some(Asking { fact, order })
    }

    /// What answering option `index` (0-based, screen order) would do now.
    pub fn preview(&self, index: usize) -> Option<(AnswerKind, Outcome)> {
        let Phase::Call(call) = &self.phase else {
            return None;
        };
        let asking = self.asking()?;
        let kind = *asking.order.get(index)?;
        Some((
            kind,
            rules::answer_outcome(
                call.being,
                kind,
                self.run.temper_of(call.being),
                self.run.composure,
            ),
        ))
    }

    /// Spend the morning on option `index` of `rules::morning_options`.
    pub fn choose_morning(&mut self, index: usize) {
        if self.phase != Phase::Morning {
            return;
        }
        let options = rules::morning_options(&self.run);
        let Some(action) = options.get(index).copied() else {
            return;
        };
        let effect = rules::effect_line(&self.run, action);
        self.run = rules::apply_morning(&self.run, action);
        self.night = rules::tonight(&self.run);
        self.log.push(format!(
            "day {} morning: {} ({effect})",
            self.run.day,
            rules::action_label(action)
        ));
        self.log.push(format!(
            "day {} night: {}",
            self.run.day,
            callers_line(&self.night.callers())
        ));
        self.ring(0);
    }

    /// Answer the question on the line with option `index`.
    pub fn answer(&mut self, index: usize) {
        let Some((kind, outcome)) = self.preview(index) else {
            return;
        };
        let Phase::Call(call) = &mut self.phase else {
            return;
        };
        let being = call.being;
        self.run.sanity -= outcome.sanity_cost;
        let temper = &mut self.run.temper[being.index()];
        *temper += outcome.temper_change;
        call.interest += outcome.interest_change;
        call.asked += 1;
        call.quiet = 0;
        call.spent += outcome.sanity_cost;
        call.exchanges += 1;
        let reaction = match kind {
            AnswerKind::Lore => "It is satisfied, and loses interest.",
            AnswerKind::Wrong => "It corrects you, at length.",
            AnswerKind::Insult => "It is ANGERED.",
        };
        let line = format!("{reaction} (-{} sanity)", outcome.sanity_cost);
        self.log.push(format!(
            "day {} {}: answered {:?}: interest {:+}, temper {:+}, sanity -{} -> {}",
            self.run.day,
            rules::short_name(being),
            kind,
            outcome.interest_change,
            outcome.temper_change,
            outcome.sanity_cost,
            self.run.sanity
        ));
        call.last = Some(line);
        self.settle_call();
    }

    /// One tick on the line without an answer.
    pub fn listen(&mut self) {
        let Phase::Call(call) = &mut self.phase else {
            return;
        };
        call.quiet += 1;
        if call.quiet < SILENCE_TICKS {
            return;
        }
        let being = call.being;
        let cost = rules::drain(being, self.run.temper_of(being), self.run.composure);
        call.quiet = 0;
        call.spent += cost;
        self.run.sanity -= cost;
        call.last = Some(format!("You said nothing. It listened. (-{cost} sanity)"));
        self.log.push(format!(
            "day {} {}: silence, sanity -{cost} -> {}",
            self.run.day,
            rules::short_name(being),
            self.run.sanity
        ));
        self.settle_call();
    }

    /// Go on from a screen that waits: the hang-up, or the end of a run.
    pub fn go_on(&mut self) {
        match &self.phase {
            Phase::Hangup(hangup) => {
                let next = hangup.slot + 1;
                if next < self.night.calls.len() {
                    self.ring(next);
                } else {
                    self.dawn();
                }
            }
            Phase::Ended(_) => {
                *self = Game::new(self.run.seed.wrapping_add(1));
            }
            Phase::Morning | Phase::Call(_) => {}
        }
    }

    /// Ring call `slot` of tonight, or end the night if there is none.
    fn ring(&mut self, slot: usize) {
        let Some(planned) = self.night.calls.get(slot) else {
            self.dawn();
            return;
        };
        self.phase = Phase::Call(CallState {
            slot,
            being: planned.being,
            interest: START_INTEREST,
            asked: 0,
            quiet: 0,
            spent: 0,
            exchanges: 0,
            last: None,
        });
    }

    /// After an exchange or a silence: has the call, or the run, ended?
    fn settle_call(&mut self) {
        let Phase::Call(call) = &self.phase else {
            return;
        };
        if self.run.sanity <= 0 {
            self.run.sanity = 0;
            let ending = Ending::Lost {
                day: self.run.day,
                being: call.being,
            };
            self.log.push(format!(
                "day {}: sanity gone; the run is lost",
                self.run.day
            ));
            self.phase = Phase::Ended(ending);
            return;
        }
        if call.interest > 0 {
            return;
        }
        let hangup = Hangup {
            slot: call.slot,
            being: call.being,
            spent: call.spent,
            exchanges: call.exchanges,
            last: call.last.clone().unwrap_or_default(),
        };
        self.log.push(format!(
            "day {} {}: hung up after {} exchanges, {} sanity",
            self.run.day,
            rules::short_name(call.being),
            call.exchanges,
            call.spent
        ));
        self.phase = Phase::Hangup(hangup);
    }

    /// The night is over: the next morning, or the end of the run.
    fn dawn(&mut self) {
        if self.run.day >= DAYS {
            self.log.push(format!(
                "day {}: survived the last night with sanity {}",
                self.run.day, self.run.sanity
            ));
            self.phase = Phase::Ended(Ending::Won {
                sanity: self.run.sanity,
            });
            return;
        }
        self.run.day += 1;
        self.run.studied = None;
        self.run.disrupted = None;
        self.night = rules::tonight(&self.run);
        self.phase = Phase::Morning;
    }
}

/// "Dagon, then Yog-Sothoth" — tonight's callers as one line.
pub fn callers_line(callers: &[Being]) -> String {
    if callers.is_empty() {
        return "nobody calls".to_owned();
    }
    callers
        .iter()
        .map(|being| rules::short_name(*being))
        .collect::<Vec<_>>()
        .join(", then ")
}

/// The ending's verdict and its one line, as the end screen prints them.
pub fn ending_lines(ending: Ending) -> (&'static str, String) {
    match ending {
        Ending::Lost { day, being } => (
            "YOUR MIND IS GONE",
            format!(
                "On night {day}, {} kept you on the line too long.",
                rules::short_name(being)
            ),
        ),
        Ending::Won { sanity } if sanity >= SOUND_SLEEP => (
            "YOU SURVIVED FIVE NIGHTS",
            "You unplug the phone and sleep through the sixth night.".to_owned(),
        ),
        Ending::Won { .. } => (
            "YOU SURVIVED FIVE NIGHTS",
            "You survived. The phone still rings in your dreams.".to_owned(),
        ),
    }
}
