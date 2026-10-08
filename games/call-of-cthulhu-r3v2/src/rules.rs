//! The arithmetic, all pure: what one reply costs, and who calls tonight.
//!
//! The two decision rows each read one function here — `answer_outcome` for
//! the call screen and `tonight` for the morning — so the hint on screen and
//! the resolution of the choice cannot disagree.

use crate::beings::BeingId;
use jidousha::prelude::Rng;

/// Sanity at the start of a run.
pub const START_SANITY: u32 = 90;
/// How much progress a being needs before it hangs up satisfied.
pub const PATIENCE: u8 = 4;
/// The line dies after this many exchanges, whatever was said.
pub const MAX_EXCHANGES: u8 = 6;
/// What a reply from the being's own lore costs.
pub const LINE_NOISE: u32 = 1;
/// An offending reply costs this many ordinary exchanges.
pub const ANGER_MULT: u32 = 2;
/// At this temper a being screams, and its drain doubles.
pub const TEMPER_LIMIT: u8 = 3;
/// What a scream costs.
pub const WRATH: u32 = 25;
/// What hanging up does to a being's temper.
pub const HANG_UP_TEMPER: u8 = 2;
/// How many days a run lasts.
pub const DAYS: u8 = 5;
/// From this night on, two beings call.
pub const TWO_CALL_FROM: u8 = 4;

/// What kind of reply the player gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnswerKind {
    /// From the being's lore: what it wants to hear.
    Lore,
    /// Not offensive, not right: the call goes on.
    Wrong,
    /// Offends the being.
    Anger,
    /// Put the receiver down.
    HangUp,
}

/// What one reply does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Sanity this exchange costs, before any scream.
    pub sanity_cost: u32,
    /// How much the being's temper rises.
    pub temper_delta: u8,
    /// How much of the being's patience the reply uses up.
    pub progress: u8,
    /// Whether the reply ends the call by itself.
    pub hangs_up: bool,
}

/// What an ordinary exchange with `being` costs at `temper`, with or without
/// steadied nerves.
fn per_exchange(being: BeingId, temper: u8, meditated: bool) -> u32 {
    let doubled = if temper >= TEMPER_LIMIT { 2 } else { 1 };
    let steady = if meditated { 1 } else { 0 };
    (being.being().drain * doubled)
        .saturating_sub(steady)
        .max(1)
}

/// The one answer-outcome function: what giving a reply of `kind` to `being`
/// costs and does. The call screen's hint and the call's resolution both read
/// it (decision row 1).
pub fn answer_outcome(being: BeingId, temper: u8, meditated: bool, kind: AnswerKind) -> Outcome {
    let per = per_exchange(being, temper, meditated);
    match kind {
        AnswerKind::Lore => Outcome {
            sanity_cost: LINE_NOISE,
            temper_delta: 0,
            progress: 2,
            hangs_up: false,
        },
        AnswerKind::Wrong => Outcome {
            sanity_cost: per,
            temper_delta: 0,
            progress: 1,
            hangs_up: false,
        },
        AnswerKind::Anger => Outcome {
            sanity_cost: per * ANGER_MULT,
            temper_delta: 1,
            progress: 0,
            hangs_up: false,
        },
        AnswerKind::HangUp => Outcome {
            sanity_cost: 0,
            temper_delta: HANG_UP_TEMPER,
            progress: 0,
            hangs_up: true,
        },
    }
}

/// Whether a reply with `outcome`, given at `temper`, makes the being scream.
pub fn screams(temper: u8, outcome: Outcome) -> bool {
    outcome.temper_delta > 0 && temper + outcome.temper_delta >= TEMPER_LIMIT
}

/// Everything a reply costs in sanity, the scream included: what a player who
/// read the hint expects to pay.
pub fn total_cost(temper: u8, outcome: Outcome) -> u32 {
    outcome.sanity_cost + if screams(temper, outcome) { WRATH } else { 0 }
}

/// One call tonight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallPlan {
    pub being: BeingId,
    /// What an ordinary exchange on this call costs.
    pub per_exchange: u32,
}

/// Tonight's calls, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NightPlan {
    pub calls: Vec<CallPlan>,
}

/// The one schedule function: who calls on night `day`, and what each call
/// costs an exchange, from the day's state. The morning preview and the night
/// both read it (decision row 2).
pub fn tonight(
    day: u8,
    rotation: [BeingId; 3],
    tempers: [u8; 3],
    meditated: bool,
    cult: Option<BeingId>,
) -> NightPlan {
    let d = usize::from(day.max(1));
    let mut callers = vec![rotation[(d - 1) % 3]];
    if day >= TWO_CALL_FROM {
        callers.push(rotation[d % 3]);
    }
    if let Some(drawn) = cult
        && !callers.contains(&drawn)
    {
        callers[0] = drawn;
    }
    NightPlan {
        calls: callers
            .into_iter()
            .map(|being| CallPlan {
                being,
                per_exchange: answer_outcome(
                    being,
                    tempers[being.index()],
                    meditated,
                    AnswerKind::Wrong,
                )
                .sanity_cost,
            })
            .collect(),
    }
}

/// A seeded order for the three beings: Fisher-Yates over the ids.
pub fn rotation(rng: &mut Rng) -> [BeingId; 3] {
    let mut order = BeingId::ALL;
    for top in (1..order.len()).rev() {
        let pick = rng.below(top as u32 + 1) as usize;
        order.swap(top, pick);
    }
    order
}
