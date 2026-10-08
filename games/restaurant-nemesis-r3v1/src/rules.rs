//! The rules as free functions, each the one place its question is answered.
//!
//! - `outcome` — what serving an order a given way does: satisfaction, the
//!   failure's theme and severity, whether it spawns a nemesis or feeds a
//!   following, and the money. The order row previews it; `cook_round`
//!   applies it.
//! - `progress` and `visit_result` — where a nemesis stands on each defeat
//!   path, and what one visit's serve does to them. The nemesis card prints
//!   them; `cook_round` applies `visit_result`.
//! - `forecast` — tonight's spends through followers, tier, and tomorrow's
//!   follower count and demand. The ledger prints it; `open_day` applies it.

use crate::content::{THEMES, Theme};
use crate::sim::{DefeatPath, Game, Order, Serve, Spends, Who};

/// Days in a run.
pub const DAYS: u32 = 7;
/// Rounds of service a day.
pub const ROUNDS: u32 = 4;
/// Kitchen capacity each round.
pub const CAPACITY: u32 = 4;
/// Money at the start.
pub const START_MONEY: i32 = 40;
/// Rent at each day's close.
pub const RENT: i32 = 25;
/// What a satisfied diner or follower pays.
pub const PAY: i32 = 6;
/// What a satisfied nemesis pays.
pub const NEMESIS_PAY: i32 = 10;
/// What an order served but not satisfied pays.
pub const GRUDGING_PAY: i32 = 2;
/// The refund per point of a failure's severity.
pub const REFUND_PER_SEVERITY: i32 = 1;
/// How much worse a failure is when the order was not served at all.
pub const SKIP_INSULT: u8 = 1;
/// The severity at which an ordinary diner's failure makes a nemesis.
pub const SPAWN_SEVERITY: u8 = 4;
/// At most this many nemeses at once.
pub const MAX_NEMESES: usize = 3;
/// A nemesis's own demand in their theme, before the sensitivity.
pub const NEMESIS_BASE: u8 = 2;
/// How much harder a nemesis is to please in their theme.
pub const SENSITIVITY: u8 = 2;
/// Satisfying visits that beat a nemesis.
pub const TALLY_TO_WIN: u8 = 3;
/// How far one visit must beat a nemesis's demand to beat them outright.
pub const OVERWHELM_MARGIN: u8 = 3;
/// Below this many followers a nemesis gives up.
pub const FOLLOWER_FLOOR: i32 = 10;
/// Followers a nemesis gains when failed in person.
pub const NEMESIS_FAIL: i32 = 25;
/// Followers a nemesis gains when one of their followers is failed.
pub const FOLLOWER_FAIL: i32 = 8;
/// Followers a satisfied nemesis loses (a grudging good review).
pub const NEMESIS_PLEASED: i32 = -5;
/// Followers the top nemesis gains when a spawn finds the cap full.
pub const CAP_REPOST: i32 = 20;
/// Followers every nemesis gains overnight.
pub const NIGHT_GROWTH: i32 = 10;
/// What a takedown costs.
pub const TAKEDOWN_COST: i32 = 15;
/// What a takedown removes.
pub const TAKEDOWN_CUT: i32 = 30;
/// What a temp cook costs.
pub const TEMP_COST: i32 = 20;
/// Most followers among one day's customers, all nemeses together.
pub const MAX_FOLLOWERS_A_DAY: usize = 8;

/// What a failure does beyond the refund.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Consequence {
    /// Nothing more.
    None,
    /// A new nemesis.
    Spawn,
    /// The cap is full: the top nemesis reposts it.
    Repost {
        /// Who.
        nemesis: usize,
    },
    /// A nemesis gains followers.
    Feeds {
        /// Who.
        nemesis: usize,
        /// How many.
        followers: i32,
        /// Whether it was the nemesis themself who was failed.
        in_person: bool,
    },
}

/// What one serve of one order does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Every demand met.
    pub satisfied: bool,
    /// The worst shortfall's theme and size, when not.
    pub failure: Option<(Theme, u8)>,
    /// What the failure sets off.
    pub consequence: Consequence,
    /// Money in (or out, for a refund).
    pub money: i32,
}

/// The one order-outcome function.
pub fn outcome(game: &Game, order: &Order, serve: Serve) -> Outcome {
    let quality = serve.quality();
    let mut failure: Option<(Theme, u8)> = None;
    for theme in THEMES {
        let demand = order.demands[theme.index()];
        if demand > quality {
            let short = demand - quality;
            if failure.is_none_or(|(_, worst)| short > worst) {
                failure = Some((theme, short));
            }
        }
    }
    // A diner sent away unfed is insulted on top of the shortfall.
    if serve == Serve::Skip {
        failure = failure.map(|(theme, short)| (theme, short + SKIP_INSULT));
    }
    let Some((_, severity)) = failure else {
        let money = match order.who {
            Who::Nemesis(_) => NEMESIS_PAY,
            _ => PAY,
        };
        let consequence = match order.who {
            Who::Nemesis(nemesis) => Consequence::Feeds {
                nemesis,
                followers: NEMESIS_PLEASED,
                in_person: false,
            },
            _ => Consequence::None,
        };
        return Outcome {
            satisfied: true,
            failure: None,
            consequence,
            money,
        };
    };
    let paid = if serve == Serve::Skip {
        0
    } else {
        GRUDGING_PAY
    };
    let consequence = match order.who {
        Who::Diner(_) if severity >= SPAWN_SEVERITY => {
            let active = game.active();
            if active.len() < MAX_NEMESES {
                Consequence::Spawn
            } else {
                let top = active
                    .iter()
                    .copied()
                    .max_by_key(|&index| (game.nemeses[index].followers, usize::MAX - index));
                match top {
                    Some(nemesis) => Consequence::Repost { nemesis },
                    None => Consequence::Spawn,
                }
            }
        }
        Who::Diner(_) => Consequence::None,
        Who::Follower(nemesis) => Consequence::Feeds {
            nemesis,
            followers: FOLLOWER_FAIL,
            in_person: false,
        },
        Who::Nemesis(nemesis) => Consequence::Feeds {
            nemesis,
            followers: NEMESIS_FAIL,
            in_person: true,
        },
    };
    Outcome {
        satisfied: false,
        failure,
        consequence,
        money: paid - REFUND_PER_SEVERITY * i32::from(severity),
    }
}

/// How many followers a fresh nemesis starts with.
pub fn spawn_followers(severity: u8) -> i32 {
    10 + 5 * i32::from(severity)
}

/// A nemesis's tier for a follower count: 0 below the floor (beaten).
pub fn tier(followers: i32) -> u8 {
    if followers < FOLLOWER_FLOOR {
        0
    } else if followers < 50 {
        1
    } else if followers < 100 {
        2
    } else {
        3
    }
}

/// Where a nemesis stands on the three defeat paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    /// Satisfying visits so far, and how many win.
    pub tally: (u8, u8),
    /// Their demand in their theme, and the quality that beats them outright.
    pub overwhelm: (u8, u8),
    /// Their followers, and the floor below which they give up.
    pub following: (i32, i32),
}

/// The defeat-progress reading the card prints.
pub fn progress(game: &Game, nemesis: usize) -> Progress {
    let them = &game.nemeses[nemesis];
    let demand = NEMESIS_BASE + SENSITIVITY;
    Progress {
        tally: (them.tally, TALLY_TO_WIN),
        overwhelm: (demand, demand + OVERWHELM_MARGIN),
        following: (them.followers, FOLLOWER_FLOOR),
    }
}

/// What one visit's serve does to a nemesis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisitResult {
    /// They left satisfied.
    pub satisfied: bool,
    /// The visit beats them, and how.
    pub defeats: Option<DefeatPath>,
}

/// The defeat resolution: reads `progress` and `outcome`, and nothing else.
pub fn visit_result(game: &Game, nemesis: usize, order: &Order, serve: Serve) -> VisitResult {
    let reading = progress(game, nemesis);
    let satisfied = outcome(game, order, serve).satisfied;
    let defeats = if satisfied && serve.quality() >= reading.overwhelm.1 {
        Some(DefeatPath::Overwhelmed)
    } else if satisfied && reading.tally.0 + 1 >= reading.tally.1 {
        Some(DefeatPath::Tally)
    } else {
        None
    };
    VisitResult { satisfied, defeats }
}

/// The words for a defeat path.
pub fn path_name(path: DefeatPath) -> &'static str {
    match path {
        DefeatPath::Tally => "3rd visit",
        DefeatPath::Overwhelmed => "outright",
        DefeatPath::Following => "fans gone",
    }
}

/// Tomorrow, for one nemesis, given tonight's spends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Forecast {
    /// Which nemesis.
    pub nemesis: usize,
    /// Followers after the spends.
    pub followers: i32,
    /// The tier that sets.
    pub tier: u8,
    /// Whether they give up in the morning.
    pub defeated: bool,
    /// How many of tomorrow's customers follow them.
    pub count: usize,
    /// The demand those followers carry in the nemesis's theme.
    pub level: u8,
}

/// The one follower function: spends → followers → tier → tomorrow's
/// follower count and demand, for every active nemesis in spawn order.
pub fn forecast(game: &Game, spends: &Spends) -> Vec<Forecast> {
    let mut room = MAX_FOLLOWERS_A_DAY;
    game.active()
        .into_iter()
        .map(|nemesis| {
            let cut = if spends.takedowns.contains(&nemesis) {
                TAKEDOWN_CUT
            } else {
                0
            };
            let followers = (game.nemeses[nemesis].followers - cut).max(0);
            let tier = tier(followers);
            let count = (2 * usize::from(tier)).min(room);
            room -= count;
            Forecast {
                nemesis,
                followers,
                tier,
                defeated: tier == 0,
                count,
                level: 1 + tier,
            }
        })
        .collect()
}
