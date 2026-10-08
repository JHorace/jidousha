//! The restaurant as plain data: themes, customers, orders, nemeses, the one
//! `Game` resource, and every number DESIGN.md fixes, as a named constant.
//!
//! Nothing here decides anything; `rules.rs` reads it and `turn.rs` changes it.
//!
//! Key types: `Game`, `Order`, `Nemesis`, `Theme`, `Kind`, `Phase`.

use jidousha::prelude::*;

/// Days in a run; surviving day 7's check wins.
pub(crate) const DAYS: u32 = 7;
/// Ordinary customers each day.
pub(crate) const CUSTOMERS_PER_DAY: usize = 6;
/// Units of service a day before prep.
pub(crate) const BASE_CAPACITY: u32 = 5;
/// Capacity one prep adds to tomorrow.
pub(crate) const PREP_BONUS: u32 = 1;
/// Preps allowed a night.
pub(crate) const MAX_PREPS: u32 = 2;
/// Cost of one prep.
pub(crate) const PREP_COST: i32 = 15;
/// Severity at or above which an unmet order spawns a nemesis.
pub(crate) const SPAWN_THRESHOLD: u32 = 4;
/// The most nemeses active at once.
pub(crate) const MAX_NEMESES: usize = 3;
/// Followers a failure feeds a nemesis per point of severity, at the cap.
pub(crate) const CAP_FEED_PER_SEVERITY: u32 = 5;
/// Extra need a nemesis has in their theme.
pub(crate) const SENSITIVITY: u32 = 2;
/// A nemesis order's need before sensitivity.
pub(crate) const NEMESIS_BASE_NEED: u32 = 1;
/// A nemesis order's temper.
pub(crate) const NEMESIS_TEMPER: u32 = 3;
/// Days between a nemesis's visits (and from spawn to the first).
pub(crate) const RETURN_EVERY: u32 = 2;
/// Followers a nemesis spawns with.
pub(crate) const FOLLOWERS_AT_SPAWN: u32 = 30;
/// Followers a nemesis gains when their own order is unmet.
pub(crate) const FAIL_LEADER_FOLLOWERS: u32 = 30;
/// Followers a leader gains when one of their followers is failed.
pub(crate) const FAIL_FOLLOWER_FOLLOWERS: u32 = 5;
/// Followers at which a nemesis is a Local Menace.
pub(crate) const T2_AT: u32 = 30;
/// Followers at which a nemesis is a Trending Terror.
pub(crate) const T3_AT: u32 = 60;
/// Of tomorrow's customers, how many follow a nemesis of each tier.
pub(crate) const SHARE: [usize; 3] = [1, 2, 4];
/// Extra need a follower's adopted demand carries.
pub(crate) const FOLLOWER_EXTRA_NEED: u32 = 1;
/// Reputation every Trending Terror costs each night.
pub(crate) const TIER3_NIGHTLY_REP: i32 = 3;
/// Satisfactions that win a nemesis over.
pub(crate) const REPEAT_TALLY: u32 = 2;
/// Units beyond a nemesis's need that overwhelm them.
pub(crate) const OVERWHELM_EXTRA: u32 = 2;
/// Money a served ordinary or follower order earns per unit of need.
pub(crate) const PRICE_PER_UNIT: i32 = 10;
/// Money a served nemesis pays.
pub(crate) const NEMESIS_PAYS: i32 = 30;
/// Refund for an unmet ordinary or follower order.
pub(crate) const REFUND_ORDINARY: i32 = 8;
/// Refund for an unmet nemesis.
pub(crate) const REFUND_NEMESIS: i32 = 20;
/// Reputation for a served ordinary order, a served nemesis, winning one
/// over (on top), overwhelming one (on top), and failing one.
pub(crate) const REP_SERVED: i32 = 1;
pub(crate) const REP_NEMESIS_SERVED: i32 = 2;
pub(crate) const REP_WON_OVER: i32 = 5;
pub(crate) const REP_OVERWHELMED: i32 = 5;
pub(crate) const REP_NEMESIS_UNMET: i32 = 5;
/// One social-media spend: its price, and the followers it cuts.
pub(crate) const SOCIAL_COST: i32 = 25;
pub(crate) const SOCIAL_CUT: u32 = 15;
/// Where a run starts.
pub(crate) const START_MONEY: i32 = 40;
pub(crate) const START_REP: i32 = 30;
/// Reputation's ceiling.
pub(crate) const MAX_REP: i32 = 100;

/// The four failure themes, in their fixed order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Theme {
    Condiment,
    Temperature,
    Wait,
    Portion,
}

impl Theme {
    pub(crate) const ALL: [Theme; 4] = [
        Theme::Condiment,
        Theme::Temperature,
        Theme::Wait,
        Theme::Portion,
    ];

    pub(crate) fn index(self) -> usize {
        self as usize
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Theme::Condiment => "condiment",
            Theme::Temperature => "temperature",
            Theme::Wait => "wait",
            Theme::Portion => "portion",
        }
    }

    /// What a customer of this theme orders.
    pub(crate) fn dish(self) -> &'static str {
        match self {
            Theme::Condiment => "fries, mustard NOW",
            Theme::Temperature => "soup, hot means HOT",
            Theme::Wait => "the special, in a hurry",
            Theme::Portion => "a heroic portion",
        }
    }

    /// What a nemesis of this theme says on their card.
    pub(crate) fn complaint(self) -> &'static str {
        match self {
            Theme::Condiment => "\"Last time the mustard was a rumour.\"",
            Theme::Temperature => "\"I have had warmer handshakes.\"",
            Theme::Wait => "\"I aged. Visibly.\"",
            Theme::Portion => "\"I have seen bigger croutons.\"",
        }
    }
}

/// One customer's demand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Customer {
    pub(crate) theme: Theme,
    /// Units of capacity it takes to serve.
    pub(crate) need: u32,
    /// How badly an unmet order goes, 1..=3.
    pub(crate) temper: u32,
}

/// A nemesis's identity, stable for the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NemesisId(pub(crate) u32);

/// Who an order is from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Ordinary,
    Follower { leader: NemesisId },
    Nemesis(NemesisId),
}

/// One order in today's queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Order {
    pub(crate) kind: Kind,
    pub(crate) customer: Customer,
    pub(crate) served: bool,
}

/// A problem customer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Nemesis {
    pub(crate) id: NemesisId,
    pub(crate) theme: Theme,
    /// Which of the theme's three titles they took.
    pub(crate) title_index: u32,
    pub(crate) followers: u32,
    /// Satisfactions banked toward winning them over.
    pub(crate) tally: u32,
    pub(crate) spawned_day: u32,
    pub(crate) next_visit: u32,
}

/// The follower tiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tier {
    Grumbler,
    LocalMenace,
    TrendingTerror,
}

impl Tier {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Tier::Grumbler => "Grumbler",
            Tier::LocalMenace => "Local Menace",
            Tier::TrendingTerror => "Trending Terror",
        }
    }

    pub(crate) fn index(self) -> usize {
        self as usize
    }

    /// The ledger's post line for a nemesis of this tier.
    pub(crate) fn post(self) -> &'static str {
        match self {
            Tier::Grumbler => "posted a one-star review. Nobody liked it.",
            Tier::LocalMenace => "the whole street has seen the video.",
            Tier::TrendingTerror => "trending. A news van is outside.",
        }
    }
}

/// Which screen the restaurant is on — one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    /// Service; `focus` is the queue row whose nemesis card is open.
    Service {
        focus: Option<usize>,
    },
    Ledger,
    Over {
        won: bool,
        reason: &'static str,
    },
}

/// The night's tally, for the ledger's `tonight:` line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tonight {
    pub(crate) served: u32,
    pub(crate) unmet: u32,
    pub(crate) money_lost: i32,
    pub(crate) rep_lost: i32,
}

/// The whole restaurant.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Game {
    pub(crate) seed: u64,
    pub(crate) phase: Phase,
    pub(crate) day: u32,
    pub(crate) money: i32,
    pub(crate) rep: i32,
    pub(crate) capacity_left: u32,
    pub(crate) preps_tonight: u32,
    pub(crate) queue: Vec<Order>,
    /// Active nemeses, in spawn order.
    pub(crate) nemeses: Vec<Nemesis>,
    pub(crate) defeated: Vec<Nemesis>,
    pub(crate) spawned_per_theme: [u32; 4],
    pub(crate) next_id: u32,
    /// Tomorrow's six customers, drawn at close, before any spend.
    pub(crate) tomorrow_base: Vec<Customer>,
    /// Today's log, oldest first.
    pub(crate) log: Vec<String>,
    pub(crate) tonight: Tonight,
    pub(crate) notice: Option<String>,
    /// The most nemeses ever active on one day, and the first day it happened.
    pub(crate) peak: (usize, u32),
}
impl Resource for Game {}

impl Game {
    pub(crate) fn find_nemesis(&self, id: NemesisId) -> Option<&Nemesis> {
        self.nemeses.iter().find(|n| n.id == id)
    }
}
