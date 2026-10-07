//! The rules, as pure functions over a `Game`: what an answer costs, who calls tonight,
//! what a morning action does, and the door `choose` that applies them.
//!
//! DELIBERATE: a morning option's stated effect (`describe_action`) is computed by
//! applying the action to a copy and diffing `plan_night` before and after, and the
//! evening's calls are `plan_night` of the game as the morning left it — so what the
//! morning promises and what the night does are one function, not two that agree.
//!
//! INVARIANT: no call here draws from the world `Rng`; every pick is
//! `Rng::from_seed(mix(seed, ..))`.

use jidousha::prelude::Rng;

use crate::game::{Call, CallPlan, Choice, Ending, Game, Record, Refused, Stage};
use crate::lore::{BEINGS, Being, FACTS_PER_BEING};

/// Sanity at the start of a run, and the most it can be.
pub const START_SANITY: i32 = 60;
/// How many days the run lasts.
pub const DAYS: usize = 5;
/// How many calls a night has, at most.
pub const CALLS_PER_NIGHT: usize = 2;
/// The most Composure there is.
pub const COMPOSURE_MAX: i32 = 2;
/// What resting gives back.
pub const REST_GAIN: i32 = 8;
/// What appeasing a cult costs.
pub const APPEASE_COST: i32 = 4;
/// How many exchanges every answer takes off the call by itself: the being gets its way in time.
pub const STEP: i32 = 1;
/// How many more exchanges a right answer takes off.
pub const RIGHT_EXTRA: i32 = 1;
/// How many drains an insult costs: anger costs more than the drain.
pub const INSULT_DRAINS: i32 = 2;
/// How many drains hanging up costs.
pub const HANGUP_DRAINS: i32 = 8;
/// What an exchange costs once the being is in wrath, as a multiple.
pub const WRATH_MULTIPLE: i32 = 2;
/// How many facts a morning's study teaches.
pub const STUDY_FACTS: usize = 3;
/// Sanity at the end that makes the ending sound rather than frayed.
pub const SOUND_AT: i32 = 12;

/// What an answer is, as far as the being is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The one the fact makes true.
    Right,
    /// A plausible mistake.
    Guess,
    /// An insult.
    Insult,
    /// Putting the phone down.
    HangUp,
}

/// What an answer does, all at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Exchanges added to the call (negative shortens it); every answer already includes `-STEP`.
    pub remaining_delta: i32,
    /// Change in the being's temper.
    pub temper_delta: i32,
    /// Sanity lost.
    pub sanity_cost: i32,
    /// Whether the call is over whatever else is true.
    pub ends_call: bool,
}

/// What one exchange costs: the being's drain less Composure, never under 1, and doubled
/// once the being is in wrath.
pub fn exchange_cost(being: Being, composure: i32, temper: i32) -> i32 {
    let drain = (being.drain() - composure).max(1);
    if temper >= being.wrath_at() {
        drain * WRATH_MULTIPLE
    } else {
        drain
    }
}

/// What hanging up costs: the being's own drain, not reduced by Composure (a rudeness
/// nothing steadies you against), times `HANGUP_DRAINS`, doubled in wrath.
fn hang_up_cost(being: Being, temper: i32) -> i32 {
    being.drain() * HANGUP_DRAINS * if temper >= being.wrath_at() { WRATH_MULTIPLE } else { 1 }
}

/// What `kind` does to a call with `being` at `temper`, for someone with `composure`.
/// The resolution and every hint on the screen call this.
pub fn answer_outcome(being: Being, composure: i32, temper: i32, kind: Kind) -> Outcome {
    let drain = exchange_cost(being, composure, temper);
    match kind {
        Kind::Right => Outcome {
            remaining_delta: -(STEP + RIGHT_EXTRA),
            temper_delta: -1,
            sanity_cost: drain,
            ends_call: false,
        },
        Kind::Guess => Outcome {
            remaining_delta: being.guess_penalty() - STEP,
            temper_delta: 0,
            sanity_cost: drain,
            ends_call: false,
        },
        Kind::Insult => Outcome {
            remaining_delta: being.guess_penalty() - STEP,
            temper_delta: 1,
            sanity_cost: drain * INSULT_DRAINS,
            ends_call: false,
        },
        Kind::HangUp => Outcome {
            remaining_delta: 0,
            temper_delta: 1,
            sanity_cost: hang_up_cost(being, temper),
            ends_call: true,
        },
    }
}

/// A seed for one pick, from the run's seed and where the pick is.
pub fn mix(seed: u64, a: u64, b: u64, c: u64) -> u64 {
    let mut x = seed ^ 0x9E37_79B9_7F4A_7C15;
    for part in [a, b, c] {
        x = x.wrapping_add(part.wrapping_mul(0xBF58_476D_1CE4_E5B9));
        x ^= x >> 29;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 32;
    }
    x
}

/// A shuffle of `0..n` from `rng`: Fisher-Yates, drawing one number per place.
fn shuffled(n: usize, rng: &mut Rng) -> Vec<usize> {
    let mut items: Vec<usize> = (0..n).collect();
    for place in (1..n).rev() {
        let pick = rng.below(place as u32 + 1) as usize;
        items.swap(place, pick);
    }
    items
}

/// Which of the three answer slots holds which kind for this question, from the seed.
pub fn option_kinds(seed: u64, day: usize, slot: usize, exchange: usize) -> [Kind; 3] {
    let mut rng = Rng::from_seed(mix(seed, day as u64, slot as u64 * 64 + exchange as u64, 0xA115));
    let order = shuffled(3, &mut rng);
    let kinds = [Kind::Right, Kind::Guess, Kind::Insult];
    [kinds[order[0]], kinds[order[1]], kinds[order[2]]]
}

/// Tonight's calls, from the game as the morning has left it: a grudge first, then a
/// seeded pick of the beings that were not appeased, each asking its facts in a seeded
/// order with this morning's study first.
pub fn plan_night(game: &Game) -> Vec<CallPlan> {
    let mut callers: Vec<(Being, i32)> = Vec::new();
    if let Some(grudge) = game.grudge
        && !game.appeased[grudge.index()]
    {
        callers.push((grudge, 1));
    }
    let mut rng = Rng::from_seed(mix(game.seed, game.day as u64, 0, 0xCA11));
    let mut pool: Vec<Being> = Being::ALL
        .iter()
        .copied()
        .filter(|b| !game.appeased[b.index()])
        .collect();
    while callers.len() < CALLS_PER_NIGHT && !pool.is_empty() {
        // A grudge's being is not drawn twice in a row at the top of the list.
        let pick = rng.below(pool.len() as u32) as usize;
        let being = pool.remove(pick);
        if callers.iter().any(|(b, _)| *b == being) {
            continue;
        }
        callers.push((being, 0));
    }
    callers
        .into_iter()
        .enumerate()
        .map(|(slot, (being, temper))| {
            let mut order = shuffled(
                FACTS_PER_BEING,
                &mut Rng::from_seed(mix(game.seed, game.day as u64, slot as u64, 0x0DE4 + being.index() as u64)),
            );
            if let Some((studied, facts)) = &game.studied_today
                && *studied == being
            {
                order.retain(|f| !facts.contains(f));
                for (place, &fact) in facts.iter().enumerate() {
                    order.insert(place, fact);
                }
            }
            CallPlan {
                being,
                temper,
                order,
            }
        })
        .collect()
}

/// What the morning can do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Learn the being's next unknown fact.
    Study(Being),
    /// Keep the being busy tonight, for a price.
    Appease(Being),
    /// Steady the voice.
    Train,
    /// Get some sleep.
    Rest,
}

/// The morning's options, in the order the keys and the rows number them.
pub const ACTIONS: [Action; 8] = [
    Action::Study(Being::YogSothoth),
    Action::Study(Being::Dagon),
    Action::Study(Being::Nyarlathotep),
    Action::Appease(Being::YogSothoth),
    Action::Appease(Being::Dagon),
    Action::Appease(Being::Nyarlathotep),
    Action::Train,
    Action::Rest,
];

/// Do `action` to `game`, or say why not and change nothing.
pub fn apply_action(game: &mut Game, action: Action) -> Result<(), Refused> {
    match action {
        Action::Study(being) => {
            let learned: Vec<usize> = (0..FACTS_PER_BEING)
                .filter(|&f| !game.known[being.index()][f])
                .take(STUDY_FACTS)
                .collect();
            if learned.is_empty() {
                return Err(Refused(format!(
                    "There is nothing left to learn about {}.",
                    being.name()
                )));
            }
            for &fact in &learned {
                game.known[being.index()][fact] = true;
            }
            game.studied_today = Some((being, learned));
        }
        Action::Appease(being) => {
            if game.sanity <= APPEASE_COST {
                return Err(Refused(format!(
                    "The rite would cost {APPEASE_COST} sanity and you have {}.",
                    game.sanity
                )));
            }
            game.sanity -= APPEASE_COST;
            game.appeased[being.index()] = true;
        }
        Action::Train => {
            if game.composure >= COMPOSURE_MAX {
                return Err(Refused("Your composure cannot be trained further.".to_owned()));
            }
            game.composure += 1;
        }
        Action::Rest => {
            if game.sanity >= START_SANITY {
                return Err(Refused("You are as rested as you will be.".to_owned()));
            }
            game.sanity = (game.sanity + REST_GAIN).min(START_SANITY);
        }
    }
    Ok(())
}

/// "Tonight: Dagon, Yog-Sothoth" — who is due, in the order they call.
pub fn tonight_line(plan: &[CallPlan]) -> String {
    if plan.is_empty() {
        return "Tonight: the phone is silent.".to_owned();
    }
    let names: Vec<&str> = plan.iter().map(|p| p.being.name()).collect();
    format!("Tonight: {}.", names.join(", then "))
}

/// A morning option as the screen states it: what it does, and what that does to tonight.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    /// The option's name.
    pub label: String,
    /// What it does, in the player's terms.
    pub text: String,
    /// Whether it can be done at all.
    pub available: bool,
}

/// What an action is called on the screen.
pub fn action_label(action: Action) -> String {
    match action {
        Action::Study(b) => format!("Study {}", b.name()),
        Action::Appease(b) => format!("Appease {}", b.cult()),
        Action::Train => "Train your composure".to_owned(),
        Action::Rest => "Rest".to_owned(),
    }
}

/// The effect of `action` on tonight, read by applying it to a copy and diffing the plans
/// before and after (the same `plan_night` the evening runs).
pub fn describe_action(game: &Game, action: Action) -> Effect {
    let label = action_label(action);
    let mut after = game.clone();
    if let Err(Refused(why)) = apply_action(&mut after, action) {
        return Effect {
            label,
            text: why,
            available: false,
        };
    }
    let before_plan = plan_night(game);
    let after_plan = plan_night(&after);
    let tonight = tonight_line(&after_plan);
    let text = match action {
        Action::Study(being) => {
            let Some((_, facts)) = &after.studied_today else {
                return Effect {
                    label,
                    text: "Learns nothing.".to_owned(),
                    available: false,
                };
            };
            let titles: Vec<&str> = facts
                .iter()
                .map(|&f| crate::lore::FACTS[being.index()][f].title)
                .collect();
            let first = after_plan
                .iter()
                .find(|p| p.being == being)
                .is_some_and(|p| p.order.starts_with(facts));
            let asks = if first {
                format!("{} asks about {} first.", being.name(), if facts.len() > 1 { "them" } else { "it" })
            } else {
                format!("{} is not due tonight.", being.name())
            };
            format!("Learn: {}. {asks} {tonight}", titles.join("; "))
        }
        Action::Appease(being) => {
            let was_due = before_plan.iter().any(|p| p.being == being);
            let now_due = after_plan.iter().any(|p| p.being == being);
            let change = if was_due && !now_due {
                format!("{} will not call tonight.", being.name())
            } else {
                format!("{} was not due tonight; this buys nothing today.", being.name())
            };
            format!("Costs {APPEASE_COST} sanity. {change} {tonight}")
        }
        Action::Train => {
            let c = after.composure;
            format!(
                "Composure {c}: every exchange costs {c} less sanity (never under 1). {tonight}"
            )
        }
        Action::Rest => format!("+{} sanity. {tonight}", after.sanity - game.sanity),
    };
    Effect {
        label,
        text,
        available: true,
    }
}

/// Start the evening: fix the night's calls and pick up the first, or let a silent night pass.
fn begin_night(game: &mut Game) {
    game.night = plan_night(game);
    if game.night.is_empty() {
        end_night(game, "Every cult kept its being busy. The phone stayed silent.");
        return;
    }
    game.stage = Stage::Call;
    start_call(game, 0);
}

/// Put call `slot` of the night on the line.
fn start_call(game: &mut Game, slot: usize) {
    let plan = game.night[slot].clone();
    game.call = Some(Call {
        remaining: plan.being.call_length(),
        temper: plan.temper,
        exchange: 0,
        reply: crate::lore::greeting(plan.being).to_owned(),
        slot,
        plan,
    });
}

/// The night is over: the next day's morning, or the run's end.
fn end_night(game: &mut Game, note: &str) {
    game.call = None;
    game.night.clear();
    game.studied_today = None;
    game.appeased = [false; BEINGS];
    if game.day >= DAYS {
        let ending = if game.sanity >= SOUND_AT {
            Ending::Sound
        } else {
            Ending::Frayed
        };
        game.stage = Stage::Over(ending);
        game.note = format!("{note} The fifth night ends. You are still here.");
        return;
    }
    game.day += 1;
    game.stage = Stage::Morning;
    game.note = format!("{note} Day {} begins.", game.day);
}

/// Apply the player's choice to the game, or say why it did nothing.
pub fn choose(game: &mut Game, choice: Choice) -> Result<(), Refused> {
    match (game.stage.clone(), choice) {
        (Stage::Morning, Choice::Option(n)) => {
            let Some(&action) = ACTIONS.get(n) else {
                return Err(Refused(format!("There is no morning option {}.", n + 1)));
            };
            apply_action(game, action).inspect_err(|Refused(why)| game.note.clone_from(why))?;
            game.note = format!("You chose: {}.", action_label(action));
            begin_night(game);
            Ok(())
        }
        (Stage::Call, Choice::Option(n)) => answer(game, n),
        (Stage::Over(_), _) => Err(Refused("The run is over.".to_owned())),
        (_, Choice::Continue) => Err(Refused("There is nothing to go on from.".to_owned())),
    }
}

/// Answer the question on the line with option `n` (0-2 are answers, 3 is hanging up).
fn answer(game: &mut Game, n: usize) -> Result<(), Refused> {
    let Some(call) = game.call.clone() else {
        return Err(Refused("No one is on the line.".to_owned()));
    };
    let kind = match n {
        0..=2 => option_kinds(game.seed, game.day, call.slot, call.exchange)[n],
        3 => Kind::HangUp,
        _ => return Err(Refused(format!("There is no answer {}.", n + 1))),
    };
    let being = call.plan.being;
    let outcome = answer_outcome(being, game.composure, call.temper, kind);
    let sanity_before = game.sanity;
    game.sanity -= outcome.sanity_cost;
    game.record.push(Record {
        day: game.day,
        being,
        kind,
        outcome,
        sanity_before,
        sanity_after: game.sanity,
    });
    if game.sanity <= 0 {
        game.call = None;
        game.stage = Stage::Over(Ending::Lost);
        game.note = format!("{} will not let go. Your sanity is gone.", being.name());
        return Ok(());
    }
    let temper = (call.temper + outcome.temper_delta).clamp(0, being.wrath_at() + 1);
    let remaining = call.remaining + outcome.remaining_delta;
    let reply = match kind {
        Kind::Right => call.fact().map_or("", |f| f.pleased),
        Kind::Guess => crate::lore::guess_reply(being),
        Kind::Insult => crate::lore::insult_reply(being),
        Kind::HangUp => crate::lore::hangup_reply(being),
    };
    if outcome.ends_call || remaining <= 0 {
        // A being left in wrath, or hung up on, calls first tomorrow.
        if outcome.ends_call || temper >= being.wrath_at() {
            game.grudge = Some(being);
        } else if game.grudge == Some(being) {
            game.grudge = None;
        }
        let last = if outcome.ends_call {
            reply
        } else {
            crate::lore::goodbye(being)
        };
        let next = call.slot + 1;
        if next < game.night.len() {
            start_call(game, next);
            if let Some(on) = game.call.as_mut() {
                on.reply = format!("{last} {}", crate::lore::greeting(on.plan.being));
            }
        } else {
            end_night(game, last);
        }
        return Ok(());
    }
    if let Some(on) = game.call.as_mut() {
        on.remaining = remaining;
        on.temper = temper;
        on.exchange += 1;
        on.reply = reply.to_owned();
    }
    Ok(())
}
