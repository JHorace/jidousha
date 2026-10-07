//! The players the checks use: one that reads the lore and wins, one that answers the first
//! thing on the screen like a person on their first night, and one that only hangs up. Each
//! is a function from the game to a `Choice`, so a check can roll a whole run through the
//! same `Game::choose` the keys reach.
//!
//! The three together are the measurement: only the middle one can say whether the game is
//! worth playing, the last proves it can be lost, and the first that it can be won.

use crate::game::{Choice, Game, Stage};
use crate::lore::{Being, FACTS_PER_BEING};
use crate::rules::{ACTIONS, Action, COMPOSURE_MAX, Kind, START_SANITY, option_kinds, plan_night};

/// A way of choosing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    /// Studies who is due tonight, answers rightly what it knows, hangs up when it is lost.
    Reader,
    /// Never studies: trains, rests when low, and answers the first thing on the screen,
    /// like a person on their first night.
    Chaser,
    /// Rests, then hangs up on everyone.
    HangUp,
}

impl Player {
    /// Every player, best first.
    pub const ALL: [Player; 3] = [Player::Reader, Player::Chaser, Player::HangUp];

    /// What this player calls itself in a report.
    pub fn name(self) -> &'static str {
        match self {
            Player::Reader => "reader",
            Player::Chaser => "chaser",
            Player::HangUp => "hang-up",
        }
    }
}

/// The index of `action` in the morning list.
fn morning_index(action: Action) -> usize {
    ACTIONS
        .iter()
        .position(|a| *a == action)
        .unwrap_or(ACTIONS.len() - 1)
}

/// What `player` chooses now.
pub fn choose(player: Player, game: &Game) -> Choice {
    match &game.stage {
        Stage::Morning => Choice::Option(morning_index(morning(player, game))),
        Stage::Call => Choice::Option(on_the_line(player, game)),
        Stage::Over(_) => Choice::Continue,
    }
}

/// The reader rests when it is low, otherwise studies whichever caller tonight it knows
/// least about; failing that it trains, then rests. The others only rest.
fn morning(player: Player, game: &Game) -> Action {
    if player == Player::HangUp || player == Player::Chaser {
        return idle_morning(player, game);
    }
    if game.sanity <= LOW_SANITY {
        return Action::Rest;
    }
    let due = plan_night(game);
    let mut best: Option<(Being, usize)> = None;
    for call in due {
        let known = game.known_facts(call.being).len();
        if known == FACTS_PER_BEING {
            continue;
        }
        if best.is_none_or(|(_, least)| known < least) {
            best = Some((call.being, known));
        }
    }
    if let Some((being, _)) = best {
        return Action::Study(being);
    }
    if game.composure < COMPOSURE_MAX {
        return Action::Train;
    }
    Action::Rest
}

/// Below this the reader rests instead of studying.
const LOW_SANITY: i32 = 10;

/// The morning of a player that never studies: rests when it can, trains when it is rested,
/// and when there is nothing left of either appeases whoever is due first.
fn idle_morning(player: Player, game: &Game) -> Action {
    let low = player == Player::HangUp || game.sanity <= LOW_SANITY;
    if game.sanity < START_SANITY && low {
        return Action::Rest;
    }
    if game.composure < COMPOSURE_MAX {
        return Action::Train;
    }
    if game.sanity < START_SANITY {
        return Action::Rest;
    }
    match plan_night(game).first() {
        Some(call) => Action::Appease(call.being),
        None => Action::Rest,
    }
}

/// What to say on the line.
fn on_the_line(player: Player, game: &Game) -> usize {
    let Some(call) = &game.call else {
        return 3;
    };
    match player {
        Player::HangUp => 3,
        Player::Chaser => 0,
        Player::Reader => {
            let kinds = option_kinds(game.seed, game.day, call.slot, call.exchange);
            if game.known[call.plan.being.index()][call.fact_index()] {
                return kinds.iter().position(|k| *k == Kind::Right).unwrap_or(0);
            }
            // A guess: nothing on the screen says which answer offends. Hanging up costs
            // more than most calls do.
            0
        }
    }
}

/// What one run came to, in the numbers the checks print.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// How it ended.
    pub ending: crate::game::Ending,
    /// The day it ended on.
    pub days: usize,
    /// Sanity at the end.
    pub sanity: i32,
    /// Answers given.
    pub answers: usize,
    /// Of them, right.
    pub right: usize,
    /// Of them, insults.
    pub insults: usize,
    /// Calls ended by hanging up.
    pub hang_ups: usize,
    /// Choices the game refused: a player that only makes valid choices has none.
    pub refused: usize,
}

/// Play a whole run from `seed` with `player`, through `Game::choose` alone.
pub fn play(seed: u64, player: Player) -> Report {
    let mut game = Game::new(seed);
    let mut refused = 0;
    // Five days of at most a few dozen exchanges each; a bound so a fault cannot spin.
    for _ in 0..2000 {
        if matches!(game.stage, Stage::Over(_)) {
            break;
        }
        let choice = choose(player, &game);
        if game.choose(choice).is_err() {
            refused += 1;
            break;
        }
    }
    let ending = match game.stage {
        Stage::Over(e) => e,
        _ => crate::game::Ending::Lost,
    };
    Report {
        ending,
        days: game.day,
        sanity: game.sanity,
        answers: game.record.len(),
        right: game.record.iter().filter(|r| r.kind == Kind::Right).count(),
        insults: game
            .record
            .iter()
            .filter(|r| r.kind == Kind::Insult)
            .count(),
        hang_ups: game
            .record
            .iter()
            .filter(|r| r.kind == Kind::HangUp)
            .count(),
        refused,
    }
}

/// How many of `seeds` each ending was reached on, and the mean sanity of the survivors.
pub fn sweep(player: Player, seeds: std::ops::Range<u64>) -> (usize, usize, f64) {
    let mut won = 0;
    let mut total = 0;
    let mut sanity = 0;
    for seed in seeds {
        let report = play(seed, player);
        total += 1;
        if report.ending != crate::game::Ending::Lost {
            won += 1;
            sanity += report.sanity;
        }
    }
    (
        won,
        total,
        if won == 0 {
            0.0
        } else {
            f64::from(sanity) / won as f64
        },
    )
}
