//! The two decision functions and the arithmetic they share.
//!
//! `answer_outcome` is what one answer does to a call — the hint beside each
//! answer and the call's resolution both read it. `tonight` derives a night's
//! calls from the day's state — the morning's preview and the evening both
//! read it, and `effect_of` states an option's effect by diffing two of its
//! answers. Pure functions over plain data: a check calls them directly.

use jidousha::prelude::*;

use crate::lore::{Being, Kind, QuestionId, questions_of};

/// Sanity at the start of a run.
pub(crate) const SANITY_START: i32 = 100;
/// How many days a run lasts; surviving the last night wins.
pub(crate) const DAYS: u32 = 5;
/// How many calls ring each evening.
pub(crate) const CALLS_PER_NIGHT: usize = 2;
/// Extra sanity every exchange costs per point of the caller's anger.
pub(crate) const ANGER_SURCHARGE: i32 = 2;
/// What a being's wrath costs, on top of the exchange that caused it.
pub(crate) const WRATH: i32 = 10;
/// The most composure training can give.
pub(crate) const MAX_COMPOSURE: i32 = 2;
/// Sanity at or above this at the end of the last night is the better ending.
pub(crate) const BETTER_ENDING: i32 = 30;

/// What one answer does to a call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    /// Added to the call's line; the call ends at zero.
    pub(crate) line_change: i32,
    /// Added to the caller's anger.
    pub(crate) anger_change: i32,
    /// Sanity this exchange costs, wrath included.
    pub(crate) sanity_cost: i32,
    /// Whether the caller's anger reaches its temper, ending the call in wrath.
    pub(crate) wrath: bool,
}

/// What one exchange drains before anger, with this much composure.
pub(crate) fn drain(being: Being, composure: i32) -> i32 {
    (being.spec().drain - composure).max(1)
}

/// The order a question's answers are shown in: indices into its table row.
///
/// Rotated per question so a position never means a kind.
pub(crate) fn shown_order(id: QuestionId) -> [usize; 3] {
    let turn = (id.being.index() + id.fact * 2 + id.which) % 3;
    [turn, (turn + 1) % 3, (turn + 2) % 3]
}

/// The kind of the answer shown at `choice` (0..3).
pub(crate) fn kind_at(id: QuestionId, choice: usize) -> Kind {
    let row = shown_order(id)[choice.min(2)];
    id.question().answers[row].kind
}

/// The text of the answer shown at `choice` (0..3).
pub(crate) fn text_at(id: QuestionId, choice: usize) -> &'static str {
    let row = shown_order(id)[choice.min(2)];
    id.question().answers[row].text
}

/// What answering `id` with the answer shown at `choice` does, to a caller
/// already at `anger`, for a player with `composure`.
///
/// CONTRACT: the hint on the call screen and the call's resolution both call
/// this, so a hint cannot promise what the call does not do.
pub(crate) fn answer_outcome(id: QuestionId, choice: usize, anger: i32, composure: i32) -> Outcome {
    let (line_change, anger_change) = match kind_at(id, choice) {
        Kind::Lore => (-2, 0),
        Kind::Wrong => (-1, 0),
        Kind::Insult => (0, 1),
    };
    let anger_after = anger + anger_change;
    let wrath = anger_after >= id.being.spec().temper;
    let mut sanity_cost = drain(id.being, composure) + ANGER_SURCHARGE * anger_after;
    if wrath {
        sanity_cost += WRATH;
    }
    Outcome {
        line_change,
        anger_change,
        sanity_cost,
        wrath,
    }
}

/// Everything a night's calls are derived from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DayState {
    pub(crate) seed: u64,
    /// 1..=DAYS.
    pub(crate) day: u32,
    /// How many of each being's facts are learned, in `Being::ALL` order.
    /// Facts are learned in table order, so this is also which ones.
    pub(crate) known: [usize; 3],
    pub(crate) composure: i32,
    /// The being studied this morning, if one was.
    pub(crate) studied: Option<Being>,
    /// The being whose cult was bargained with this morning, if one was.
    pub(crate) bargain: Option<Being>,
}

impl DayState {
    /// The first morning of a run.
    pub(crate) fn opening(seed: u64) -> Self {
        Self {
            seed,
            day: 1,
            known: [0; 3],
            composure: 0,
            studied: None,
            bargain: None,
        }
    }

    /// Whether the player knows the fact `id` is about.
    pub(crate) fn knows(&self, id: QuestionId) -> bool {
        id.fact < self.known[id.being.index()]
    }
}

/// One call a night will ring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CallPlan {
    pub(crate) being: Being,
    /// The questions it asks, in order; a long call starts over from the top.
    pub(crate) questions: Vec<QuestionId>,
    /// What each exchange drains before anger.
    pub(crate) drain: i32,
}

impl CallPlan {
    /// The question asked at exchange `n` (0-based).
    pub(crate) fn question(&self, n: usize) -> QuestionId {
        self.questions[n % self.questions.len()]
    }
}

/// One night's calls, in the order they ring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Night {
    pub(crate) calls: Vec<CallPlan>,
}

impl Night {
    /// Who calls, in order.
    pub(crate) fn callers(&self) -> Vec<Being> {
        self.calls.iter().map(|call| call.being).collect()
    }

    /// The plan for `being`'s call, if it calls.
    pub(crate) fn call_of(&self, being: Being) -> Option<&CallPlan> {
        self.calls.iter().find(|call| call.being == being)
    }
}

/// Tonight's calls, derived from the day's state and nothing else.
///
/// CONTRACT: the morning preview and the evening both call this, so the
/// effect an option states is the effect the night has.
/// INVARIANT: the random draws do not depend on the morning's choice — every
/// being's question order is drawn whether or not it calls — so two mornings
/// that differ in one choice differ in tonight only where that choice reaches.
pub(crate) fn tonight(state: &DayState) -> Night {
    let mut rng = Rng::from_seed(
        state.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ u64::from(state.day).wrapping_mul(0xD1B5_4A32_D192_ED03),
    );
    let silent = Being::ALL[rng.below(3) as usize];
    let mut callers: Vec<Being> = Being::ALL
        .into_iter()
        .filter(|being| *being != silent)
        .collect();
    if rng.below(2) == 1 {
        callers.reverse();
    }
    let orders: Vec<Vec<QuestionId>> = Being::ALL
        .iter()
        .map(|being| {
            let mut pool = questions_of(*being);
            for top in (1..pool.len()).rev() {
                let pick = rng.below(top as u32 + 1) as usize;
                pool.swap(top, pick);
            }
            pool
        })
        .collect();
    if let Some(kept_silent) = state.bargain {
        for caller in &mut callers {
            if *caller == kept_silent {
                *caller = silent;
            }
        }
    }
    let calls = callers
        .into_iter()
        .take(CALLS_PER_NIGHT)
        .map(|being| {
            let mut questions = orders[being.index()].clone();
            if state.studied == Some(being) && state.known[being.index()] > 0 {
                // It can tell what you were reading, and opens with it.
                let fresh = state.known[being.index()] - 1;
                questions.sort_by_key(|id| id.fact != fresh);
            }
            CallPlan {
                being,
                questions,
                drain: drain(being, state.composure),
            }
        })
        .collect();
    Night { calls }
}

/// One way an option changes tonight.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Change {
    /// The `slot`th call is `to` instead of `from`.
    Caller { slot: usize, from: Being, to: Being },
    /// `being` opens its call on the fact `topic` (it did not before).
    Opens { being: Being, topic: &'static str },
    /// How many of `being`'s questions tonight are answerable from lore.
    Answerable {
        being: Being,
        from: usize,
        to: usize,
    },
    /// What each exchange with `being` drains.
    Drain { being: Being, from: i32, to: i32 },
}

impl Change {
    /// The change as the morning screen states it.
    pub(crate) fn line(&self) -> String {
        match self {
            Change::Caller { from, to, .. } => {
                format!("{} calls instead of {}", to.spec().name, from.spec().name)
            }
            Change::Opens { being, topic } => format!("{} opens on {}", being.spec().name, topic),
            Change::Answerable { being, from, to } => {
                format!(
                    "{}: {to} of 6 questions answerable (was {from})",
                    being.spec().name
                )
            }
            Change::Drain { being, from, to } => {
                format!("{} drains {to} an exchange (was {from})", being.spec().name)
            }
        }
    }
}

/// How many of a call's questions the player can answer from lore.
pub(crate) fn answerable(state: &DayState, call: &CallPlan) -> usize {
    call.questions.iter().filter(|id| state.knows(**id)).count()
}

/// Every way `after` differs from `before` tonight, in slot order.
///
/// CONTRACT: built from two `tonight` answers, so the morning screen states
/// only what the evening will do.
pub(crate) fn effect_of(before: &DayState, after: &DayState) -> Vec<Change> {
    let (was, will) = (tonight(before), tonight(after));
    let mut changes = Vec::new();
    for (slot, (from, to)) in was.callers().into_iter().zip(will.callers()).enumerate() {
        if from != to {
            changes.push(Change::Caller { slot, from, to });
        }
    }
    for call in &will.calls {
        let Some(old) = was.call_of(call.being) else {
            continue;
        };
        let first = call.question(0);
        if first.fact != old.question(0).fact {
            changes.push(Change::Opens {
                being: call.being,
                topic: first.fact().topic,
            });
        }
        let (from, to) = (answerable(before, old), answerable(after, call));
        if from != to {
            changes.push(Change::Answerable {
                being: call.being,
                from,
                to,
            });
        }
        if old.drain != call.drain {
            changes.push(Change::Drain {
                being: call.being,
                from: old.drain,
                to: call.drain,
            });
        }
    }
    changes
}

/// What the player may do with a morning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Study(Being),
    Train,
    Bargain(Being),
}

impl Action {
    /// The option's name, as the morning screen prints it.
    pub(crate) fn name(self, state: &DayState) -> String {
        match self {
            Action::Study(being) => {
                let fact = &being.spec().facts[state.known[being.index()]];
                format!("Study {}: {}", being.spec().name, fact.topic)
            }
            Action::Train => format!(
                "Train composure ({} -> {})",
                state.composure,
                state.composure + 1
            ),
            Action::Bargain(being) => format!("Bargain with {}", being.spec().cult),
        }
    }
}

/// The options this morning offers, in the order the screen numbers them.
pub(crate) fn morning_options(state: &DayState) -> Vec<Action> {
    let mut options: Vec<Action> = Being::ALL
        .into_iter()
        .filter(|being| state.known[being.index()] < 3)
        .map(Action::Study)
        .collect();
    if state.composure < MAX_COMPOSURE {
        options.push(Action::Train);
    }
    options.extend(tonight(state).callers().into_iter().map(Action::Bargain));
    options
}

/// The day's state once `action` is taken.
pub(crate) fn apply(state: &DayState, action: Action) -> DayState {
    let mut next = *state;
    match action {
        Action::Study(being) => {
            next.known[being.index()] = (next.known[being.index()] + 1).min(3);
            next.studied = Some(being);
        }
        Action::Train => next.composure = (next.composure + 1).min(MAX_COMPOSURE),
        Action::Bargain(being) => next.bargain = Some(being),
    }
    next
}

/// The next morning's state: a new day, the morning's choices spent.
pub(crate) fn next_day(state: &DayState) -> DayState {
    DayState {
        day: state.day + 1,
        studied: None,
        bargain: None,
        ..*state
    }
}
