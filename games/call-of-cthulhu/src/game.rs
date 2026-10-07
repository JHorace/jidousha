//! The game's one value: the day, the sanity, what is known, the night's calls and the
//! call on the line. Every rule that changes it is in `rules.rs`; this file is the shape
//! and the single door in, `Game::choose`.
//!
//! INVARIANT: nothing here draws from the world's `Rng`. A night's callers and every
//! question's answer order come from `Rng::from_seed(mix(seed, ...))` in `rules.rs`, so
//! asking "what would tonight be?" costs no draw and a replay is exact.

use jidousha::prelude::Resource;

use crate::lore::{BEINGS, Being, FACTS_PER_BEING, Fact, fact};

/// Where the run is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Choosing what to do with the day.
    Morning,
    /// On the line.
    Call,
    /// Finished.
    Over(Ending),
}

/// How the run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// Five days survived with sanity to spare.
    Sound,
    /// Five days survived, barely.
    Frayed,
    /// Sanity ran out.
    Lost,
}

/// One call in a night: who, at what temper, and the order its questions come in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallPlan {
    /// Who is calling.
    pub being: Being,
    /// The temper it picks up at (a grudge starts above calm).
    pub temper: i32,
    /// The order it asks its facts in; the first is what was studied this morning, if it was.
    pub order: Vec<usize>,
}

/// The call on the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    /// What the night planned for it.
    pub plan: CallPlan,
    /// Which call of the night this is, from 0.
    pub slot: usize,
    /// How many exchanges it has left.
    pub remaining: i32,
    /// How angry the being is now.
    pub temper: i32,
    /// How many questions have been answered.
    pub exchange: usize,
    /// What the being said last.
    pub reply: String,
}

impl Call {
    /// The fact the current question tests.
    pub fn fact_index(&self) -> usize {
        self.plan.order[self.exchange % FACTS_PER_BEING]
    }

    /// The current question and its answers.
    pub fn fact(&self) -> Option<&'static Fact> {
        fact(self.plan.being, self.fact_index())
    }
}

/// What one answer did, kept for the checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// The day it happened on.
    pub day: usize,
    /// Who was on the line.
    pub being: Being,
    /// What was done.
    pub kind: crate::rules::Kind,
    /// What the rules said it would do.
    pub outcome: crate::rules::Outcome,
    /// Sanity before.
    pub sanity_before: i32,
    /// Sanity after.
    pub sanity_after: i32,
}

/// A choice the player makes: an option on the screen, from 0, or "go on".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Option `n` of the morning list, or answer `n` on the line (3 is hanging up).
    Option(usize),
    /// Go on, where there is nothing to choose.
    Continue,
}

/// Why a choice did nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused(pub String);

/// The run.
#[derive(Clone, Debug)]
pub struct Game {
    /// What every seeded pick is derived from.
    pub seed: u64,
    /// The day, from 1.
    pub day: usize,
    /// What is left to lose.
    pub sanity: i32,
    /// How steady the voice is: each point takes one off every exchange's drain.
    pub composure: i32,
    /// Which facts are known, by being and fact.
    pub known: [[bool; FACTS_PER_BEING]; BEINGS],
    /// The facts studied this morning, if any: whose, and which.
    pub studied_today: Option<(Being, Vec<usize>)>,
    /// Whose cult was appeased this morning: they do not call tonight.
    pub appeased: [bool; BEINGS],
    /// A being that was hung up on or left in wrath: it calls first tonight, already angry.
    pub grudge: Option<Being>,
    /// Where the run is.
    pub stage: Stage,
    /// Tonight's calls, fixed when the morning ends.
    pub night: Vec<CallPlan>,
    /// The call on the line, if there is one.
    pub call: Option<Call>,
    /// The last thing worth telling the player: why a choice did nothing, how a night ended.
    pub note: String,
    /// Every answer given, oldest first.
    pub record: Vec<Record>,
}

impl Resource for Game {}

impl Game {
    /// A fresh run from `seed`: day 1, morning, nothing known.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            day: 1,
            sanity: crate::rules::START_SANITY,
            composure: 0,
            known: [[false; FACTS_PER_BEING]; BEINGS],
            studied_today: None,
            appeased: [false; BEINGS],
            grudge: None,
            stage: Stage::Morning,
            night: Vec::new(),
            call: None,
            note: "The phone has been ringing at night. Prepare.".to_owned(),
            record: Vec::new(),
        }
    }

    /// Which facts of `being` are known.
    pub fn known_facts(&self, being: Being) -> Vec<usize> {
        (0..FACTS_PER_BEING)
            .filter(|&f| self.known[being.index()][f])
            .collect()
    }

    /// The single door in: what the player chose.
    pub fn choose(&mut self, choice: Choice) -> Result<(), Refused> {
        crate::rules::choose(self, choice)
    }
}
