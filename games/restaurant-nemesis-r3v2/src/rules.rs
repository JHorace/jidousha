//! The three decision functions and their helpers — pure, over plain data.
//!
//! `outcome_of` (what serving or failing an order does), `defeat_progress`
//! (where a nemesis stands on the three defeat paths), `social_outlook`
//! (followers → tier → tomorrow's followers). Each is read by the screen that
//! previews the decision *and* by `turn.rs` when it resolves, so a preview and
//! the sim cannot disagree (DESIGN.md, "The three functions").
//!
//! Key functions: `outcome_of`, `defeat_progress`, `social_outlook`, `tier_of`,
//! `title_for`, `severity`, `cut_followers`, `draw_base_customers`.

use jidousha::prelude::*;

use crate::sim::*;

/// Three titles per theme, taken in turn by spawn count.
const TITLES: [[&str; 3]; 4] = [
    ["Mustard Monster", "The Ketchup Kaiser", "Mayo Mayhem"],
    ["The Lukewarm Baron", "Count Tepid", "The Scalding Sultan"],
    [
        "The Hangry Hourglass",
        "Sir Waits-a-Lot",
        "The Ticking Tyrant",
    ],
    ["The Crumb Countess", "Big Plate Pete", "Marquis de Morsel"],
];

/// A nemesis's title: the theme's titles in turn, by how many it has spawned.
pub(crate) fn title_for(theme: Theme, index: u32) -> &'static str {
    TITLES[theme.index()][index as usize % 3]
}

pub(crate) fn title(nemesis: &Nemesis) -> &'static str {
    title_for(nemesis.theme, nemesis.title_index)
}

/// The tier a follower count sets.
pub(crate) fn tier_of(followers: u32) -> Tier {
    if followers >= T3_AT {
        Tier::TrendingTerror
    } else if followers >= T2_AT {
        Tier::LocalMenace
    } else {
        Tier::Grumbler
    }
}

/// How bad leaving a customer unmet is.
pub(crate) fn severity(customer: &Customer) -> u32 {
    customer.need + customer.temper
}

/// One social-media spend's cut, floored at zero.
pub(crate) fn cut_followers(nemesis: &mut Nemesis) {
    nemesis.followers = nemesis.followers.saturating_sub(SOCIAL_CUT);
}

/// Six customers from the world generator, in the fixed draw order.
pub(crate) fn draw_base_customers(rng: &mut Rng) -> Vec<Customer> {
    (0..CUSTOMERS_PER_DAY)
        .map(|_| {
            let theme = Theme::ALL[rng.below(4) as usize];
            let need = [1, 1, 2][rng.below(3) as usize];
            let temper = rng.below(3) + 1;
            Customer {
                theme,
                need,
                temper,
            }
        })
        .collect()
}

/// What serving an order pays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Reward {
    pub(crate) money: i32,
    pub(crate) rep: i32,
}

/// What leaving an order unmet does to the restaurant's nemeses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Consequence {
    Spawns,
    Feeds { id: NemesisId, followers: u32 },
    Nothing,
}

/// What leaving an order unmet costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Unmet {
    pub(crate) theme: Theme,
    pub(crate) severity: u32,
    pub(crate) consequence: Consequence,
    pub(crate) money: i32,
    pub(crate) rep: i32,
    /// Followers the order's leader gains (a nemesis's own, or a follower's leader).
    pub(crate) leader_followers: u32,
}

/// Both sides of one order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OrderOutcome {
    pub(crate) if_served: Reward,
    pub(crate) if_unmet: Unmet,
}

/// The nemesis a failure feeds at the cap: the same theme's, else the most
/// followed, the oldest on ties.
fn feed_target(game: &Game, theme: Theme) -> Option<NemesisId> {
    if let Some(same) = game.nemeses.iter().find(|n| n.theme == theme) {
        return Some(same.id);
    }
    let mut best: Option<&Nemesis> = None;
    for nemesis in &game.nemeses {
        if best.is_none_or(|b| nemesis.followers > b.followers) {
            best = Some(nemesis);
        }
    }
    best.map(|n| n.id)
}

/// The one order-outcome function: the service row's preview and the
/// resolution of a serve or a close both read it.
pub(crate) fn outcome_of(order: &Order, game: &Game) -> OrderOutcome {
    let customer = order.customer;
    let severity = severity(&customer);
    if let Kind::Nemesis(_) = order.kind {
        return OrderOutcome {
            if_served: Reward {
                money: NEMESIS_PAYS,
                rep: REP_NEMESIS_SERVED,
            },
            if_unmet: Unmet {
                theme: customer.theme,
                severity,
                consequence: Consequence::Nothing,
                money: -REFUND_NEMESIS,
                rep: -REP_NEMESIS_UNMET,
                leader_followers: FAIL_LEADER_FOLLOWERS,
            },
        };
    }
    let led = match order.kind {
        Kind::Follower { leader } => game.find_nemesis(leader).is_some(),
        _ => false,
    };
    let consequence = if severity < SPAWN_THRESHOLD {
        Consequence::Nothing
    } else if game.nemeses.len() < MAX_NEMESES {
        Consequence::Spawns
    } else {
        match feed_target(game, customer.theme) {
            Some(id) => Consequence::Feeds {
                id,
                followers: severity * CAP_FEED_PER_SEVERITY,
            },
            None => Consequence::Nothing,
        }
    };
    OrderOutcome {
        if_served: Reward {
            money: PRICE_PER_UNIT * customer.need as i32,
            rep: REP_SERVED,
        },
        if_unmet: Unmet {
            theme: customer.theme,
            severity,
            consequence,
            money: -REFUND_ORDINARY,
            rep: -(customer.temper as i32),
            leader_followers: if led { FAIL_FOLLOWER_FOLLOWERS } else { 0 },
        },
    }
}

/// Where a nemesis stands on all three defeat paths, with what each costs now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DefeatProgress {
    /// Satisfactions banked, of those needed.
    pub(crate) tally: (u32, u32),
    /// Serving them now wins them over.
    pub(crate) defeated_if_served: bool,
    /// What overwhelming them costs, when there is the capacity for it.
    pub(crate) overwhelm: Option<u32>,
    /// Capacity left for everyone else after an overwhelm.
    pub(crate) leaves_for_rest: u32,
    pub(crate) followers: u32,
    pub(crate) tier: Tier,
    /// Social-media spends that would ratio them to zero.
    pub(crate) spends_to_ratio: u32,
}

/// The one defeat-progress function: the card shows it, and serve, overwhelm
/// and spend resolve a defeat by reading it.
pub(crate) fn defeat_progress(nemesis: &Nemesis, capacity_left: u32) -> DefeatProgress {
    let cost = NEMESIS_BASE_NEED + SENSITIVITY + OVERWHELM_EXTRA;
    DefeatProgress {
        tally: (nemesis.tally, REPEAT_TALLY),
        defeated_if_served: nemesis.tally + 1 >= REPEAT_TALLY,
        overwhelm: (capacity_left >= cost).then_some(cost),
        leaves_for_rest: capacity_left.saturating_sub(cost),
        followers: nemesis.followers,
        tier: tier_of(nemesis.followers),
        spends_to_ratio: nemesis.followers.div_ceil(SOCIAL_CUT),
    }
}

/// One nemesis's line in the outlook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Reach {
    pub(crate) id: NemesisId,
    pub(crate) followers: u32,
    pub(crate) tier: Tier,
    /// How many of tomorrow's customers follow them.
    pub(crate) share: usize,
}

/// Tomorrow, as the followers make it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Outlook {
    pub(crate) per: Vec<Reach>,
    pub(crate) customers: Vec<(Kind, Customer)>,
}

/// The one follower function: the ledger's preview (on a copy with a spend
/// applied) and tomorrow's queue (on the real list) both read it.
pub(crate) fn social_outlook(nemeses: &[Nemesis], base: &[Customer]) -> Outlook {
    let mut customers: Vec<(Kind, Customer)> = base.iter().map(|&c| (Kind::Ordinary, c)).collect();
    let mut per = Vec::new();
    let mut next = 0;
    for nemesis in nemeses {
        let tier = tier_of(nemesis.followers);
        let wanted = if nemesis.followers == 0 {
            0
        } else {
            SHARE[tier.index()]
        };
        let mut share = 0;
        while share < wanted && next < customers.len() {
            let (kind, customer) = &mut customers[next];
            *kind = Kind::Follower { leader: nemesis.id };
            customer.theme = nemesis.theme;
            customer.need += FOLLOWER_EXTRA_NEED;
            share += 1;
            next += 1;
        }
        per.push(Reach {
            id: nemesis.id,
            followers: nemesis.followers,
            tier,
            share,
        });
    }
    Outlook { per, customers }
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod tests;
