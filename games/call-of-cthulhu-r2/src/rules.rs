//! The rules: every number the game is tuned by, and the pure functions both
//! the screens and the simulation read.
//!
//! Two of these are the decision surfaces' "one function" (the handoff's
//! table): `answer_outcome` — what an answer does, read by the call screen's
//! hint and by the exchange — and `tonight` — tonight's callers and their
//! question order, read by the morning's preview and by the night itself.
//! Nothing here touches a `World`, so a check can ask any of them directly.

use jidousha::prelude::*;

use crate::lore::{AnswerKind, Being, FACTS};

/// How many days a run lasts. Survive the last night and you win.
pub const DAYS: u32 = 5;
/// Sanity at the start of a run.
pub const START_SANITY: i32 = 100;
/// Surviving with at least this much sanity is the better ending.
pub const SOUND_SLEEP: i32 = 50;
/// How much a being wants to talk when it calls. The call ends at zero.
pub const START_INTEREST: i32 = 5;
/// What a lore answer does to interest.
pub const LORE_INTEREST: i32 = -3;
/// What a wrong answer does to interest.
pub const WRONG_INTEREST: i32 = 1;
/// What an insult does to interest: nothing — the exchange is spent for no
/// progress.
pub const INSULT_INTEREST: i32 = 0;
/// The extra sanity an insult costs on the exchange it is made.
pub const ANGER_SHOCK: i32 = 6;
/// The most composure training can give.
pub const MAX_COMPOSURE: i32 = 1;
/// No exchange drains less than this, whatever composure says.
pub const MIN_DRAIN: i32 = 1;
/// The most a being's temper can reach.
pub const MAX_TEMPER: i32 = 5;
/// A being this angry cannot be held back by its cult: disrupting it is no
/// longer offered. Disrupting costs a temper point, so each cult can be
/// disrupted twice in a run.
pub const UNHELD_TEMPER: i32 = 2;
/// How long the player may be silent on the line before it costs a drain:
/// eight seconds of sixty-tick time.
pub const SILENCE_TICKS: u32 = 480;
/// How many beings call on an ordinary night, and on the last.
pub const CALLERS: usize = 2;
pub const LAST_NIGHT_CALLERS: usize = 3;

/// The run: everything the day loop carries from one day to the next.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub seed: u64,
    /// 1..=DAYS.
    pub day: u32,
    pub sanity: i32,
    pub composure: i32,
    /// Which facts of each being are known.
    pub known: [[bool; FACTS]; 3],
    /// Each being's temper, carried across the run.
    pub temper: [i32; 3],
    /// The fact studied this morning, if one was: asked first tonight.
    pub studied: Option<(Being, usize)>,
    /// The being whose cult was disrupted this morning: it does not call.
    pub disrupted: Option<Being>,
}

impl Run {
    /// Day one, before the first morning's choice.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            day: 1,
            sanity: START_SANITY,
            composure: 0,
            known: [[false; FACTS]; 3],
            temper: [0; 3],
            studied: None,
            disrupted: None,
        }
    }

    pub fn knows(&self, being: Being, fact: usize) -> bool {
        self.known[being.index()][fact]
    }

    pub fn temper_of(&self, being: Being) -> i32 {
        self.temper[being.index()]
    }

    /// The first fact of `being` not yet known: what studying it teaches.
    pub fn next_fact(&self, being: Being) -> Option<usize> {
        (0..FACTS).find(|fact| !self.knows(being, *fact))
    }
}

/// What one exchange on the line costs: sanity per answer, before any shock.
pub fn drain(being: Being, temper: i32, composure: i32) -> i32 {
    (being.lore().base_drain + temper - composure).max(MIN_DRAIN)
}

/// What an answer does — the one function the hint and the exchange both read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub interest_change: i32,
    pub temper_change: i32,
    pub sanity_cost: i32,
}

/// What answering `kind` does to a call with `being`, at this temper and
/// composure.
pub fn answer_outcome(being: Being, kind: AnswerKind, temper: i32, composure: i32) -> Outcome {
    let drain = drain(being, temper, composure);
    match kind {
        AnswerKind::Lore => Outcome {
            interest_change: LORE_INTEREST,
            temper_change: 0,
            sanity_cost: drain,
        },
        AnswerKind::Wrong => Outcome {
            interest_change: WRONG_INTEREST,
            temper_change: 0,
            sanity_cost: drain,
        },
        AnswerKind::Insult => Outcome {
            interest_change: INSULT_INTEREST,
            temper_change: if temper < MAX_TEMPER { 1 } else { 0 },
            sanity_cost: drain + ANGER_SHOCK,
        },
    }
}

/// One call tonight: who, and the order it asks its facts in (cycling).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedCall {
    pub being: Being,
    pub questions: Vec<usize>,
}

/// Tonight's calls, in the order the phone rings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Night {
    pub calls: Vec<PlannedCall>,
}

impl Night {
    pub fn callers(&self) -> Vec<Being> {
        self.calls.iter().map(|call| call.being).collect()
    }

    pub fn find(&self, being: Being) -> Option<&PlannedCall> {
        self.calls.iter().find(|call| call.being == being)
    }
}

/// A generator for one decision, from the run's seed and the decision's own
/// coordinates — so a choice is a pure function of where it is made, and a
/// morning's preview draws exactly what the night will.
fn rng_for(seed: u64, parts: &[u64]) -> Rng {
    let mut mixed = seed ^ 0x9E37_79B9_7F4A_7C15;
    for part in parts {
        mixed = mixed
            .rotate_left(17)
            .wrapping_mul(0xBF58_476D_1CE4_E5B9)
            .wrapping_add(*part);
    }
    Rng::from_seed(mixed)
}

/// A seeded shuffle of `items`.
fn shuffled<T: Copy>(rng: &mut Rng, items: &[T]) -> Vec<T> {
    let mut left = items.to_vec();
    let mut out = Vec::with_capacity(left.len());
    while !left.is_empty() {
        let pick = rng.below(left.len() as u32) as usize;
        out.push(left.remove(pick));
    }
    out
}

/// Tonight's calls, from the day's state — the one function the morning
/// preview and the night both read.
///
/// The seed picks who calls (two beings; three on the last night) and in what
/// order; a disrupted cult's being is struck off. Each caller asks this
/// morning's studied fact first, if it was about this being, then the rest of
/// its facts in a seeded order.
pub fn tonight(run: &Run) -> Night {
    let count = if run.day >= DAYS {
        LAST_NIGHT_CALLERS
    } else {
        CALLERS
    };
    let mut rng = rng_for(run.seed, &[u64::from(run.day), 1]);
    let order = shuffled(&mut rng, &Being::ALL);
    let calls = order
        .into_iter()
        .take(count)
        .filter(|being| run.disrupted != Some(*being))
        .map(|being| {
            let mut facts_rng = rng_for(run.seed, &[u64::from(run.day), 2, being.index() as u64]);
            let all: Vec<usize> = (0..FACTS).collect();
            let mut questions = shuffled(&mut facts_rng, &all);
            if let Some((studied, fact)) = run.studied
                && studied == being
            {
                questions.retain(|question| *question != fact);
                questions.insert(0, fact);
            }
            PlannedCall { being, questions }
        })
        .collect();
    Night { calls }
}

/// The order a question's three answers are shown in, by the seed.
pub fn answer_order(seed: u64, day: u32, being: Being, asked: usize) -> [AnswerKind; 3] {
    let mut rng = rng_for(
        seed,
        &[u64::from(day), 3, being.index() as u64, asked as u64],
    );
    let order = shuffled(&mut rng, &AnswerKind::ALL);
    [order[0], order[1], order[2]]
}

/// One thing a morning can be spent on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MorningAction {
    Train,
    Study(Being),
    Disrupt(Being),
}

/// The actions open this morning, in the order the screen numbers them.
pub fn morning_options(run: &Run) -> Vec<MorningAction> {
    let mut options = Vec::new();
    if run.composure < MAX_COMPOSURE {
        options.push(MorningAction::Train);
    }
    for being in Being::ALL {
        if run.next_fact(being).is_some() {
            options.push(MorningAction::Study(being));
        }
    }
    for being in Being::ALL {
        if run.temper_of(being) < UNHELD_TEMPER {
            options.push(MorningAction::Disrupt(being));
        }
    }
    options
}

/// The day after `action`: what the morning changes, before the night.
pub fn apply_morning(run: &Run, action: MorningAction) -> Run {
    let mut next = run.clone();
    match action {
        MorningAction::Train => {
            next.composure = (next.composure + 1).min(MAX_COMPOSURE);
        }
        MorningAction::Study(being) => {
            if let Some(fact) = run.next_fact(being) {
                next.known[being.index()][fact] = true;
                next.studied = Some((being, fact));
            }
        }
        MorningAction::Disrupt(being) => {
            next.disrupted = Some(being);
            let temper = &mut next.temper[being.index()];
            *temper = (*temper + 1).min(MAX_TEMPER);
        }
    }
    next
}

/// The option's name, as the morning screen prints it.
pub fn action_label(action: MorningAction) -> String {
    match action {
        MorningAction::Train => "Train composure".to_owned(),
        MorningAction::Study(being) => format!("Study {}", being.lore().name),
        MorningAction::Disrupt(being) => format!("Disrupt {}", being.lore().cult),
    }
}

/// What the option does to tonight, in words — read off `tonight` before and
/// after the action, so the preview cannot say what the night will not do.
pub fn effect_line(run: &Run, action: MorningAction) -> String {
    let before = tonight(run);
    let after_run = apply_morning(run, action);
    let after = tonight(&after_run);
    match action {
        MorningAction::Train => {
            let drains: Vec<String> = after
                .calls
                .iter()
                .map(|call| {
                    let being = call.being;
                    format!(
                        "{} {}->{}",
                        short_name(being),
                        drain(being, run.temper_of(being), run.composure),
                        drain(being, after_run.temper_of(being), after_run.composure)
                    )
                })
                .collect();
            format!("drain per exchange: {}", drains.join(", "))
        }
        MorningAction::Study(being) => {
            let learned = run.next_fact(being);
            let title = learned.map_or("nothing new", |fact| being.lore().facts[fact].title);
            match after.find(being).and_then(|call| call.questions.first()) {
                Some(first) if Some(*first) == learned => {
                    format!("learn {title}; it calls tonight, asks this first")
                }
                Some(first) => format!(
                    "learn {title}; it calls tonight and asks about {} first",
                    being.lore().facts[*first].title
                ),
                None => format!("learn {title}; it does not call tonight"),
            }
        }
        MorningAction::Disrupt(being) => {
            let temper = format!(
                "temper {}->{}",
                run.temper_of(being),
                after_run.temper_of(being)
            );
            if before.find(being).is_some() && after.find(being).is_none() {
                format!("{} will not call tonight; {temper}", short_name(being))
            } else {
                format!(
                    "{} was not calling tonight anyway; {temper}",
                    short_name(being)
                )
            }
        }
    }
}

/// The being's name, short enough for a line that lists several.
pub fn short_name(being: Being) -> &'static str {
    match being {
        Being::Dagon => "Dagon",
        Being::Nyarlathotep => "Nyarlathotep",
        Being::YogSothoth => "Yog-Sothoth",
    }
}
