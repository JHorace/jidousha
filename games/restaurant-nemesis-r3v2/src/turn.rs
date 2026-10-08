//! Turn resolution: every player action, closing the kitchen, opening
//! tomorrow. Rewards apply at the key press; penalties, spawns and follower
//! growth at the close, in queue order, through `rules::outcome_of`.
//!
//! Key functions: `new_game`, `apply`, `close_kitchen`, `open_tomorrow`.

use jidousha::prelude::*;

use crate::rules::{
    Consequence, cut_followers, defeat_progress, draw_base_customers, outcome_of, social_outlook,
    tier_of, title, title_for,
};
use crate::sim::*;

/// Everything the player can do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    /// Serve the queue row (ordinary), or open its nemesis card.
    Serve(usize),
    /// On the open card: serve the nemesis, banking one toward the tally.
    ServeNemesis,
    /// On the open card: overwhelm the nemesis.
    Overwhelm,
    Unfocus,
    CloseKitchen,
    /// Spend on the nemesis in this slot (spawn order).
    Spend(usize),
    Prep,
    OpenTomorrow,
    Restart,
}

/// A fresh run: day 1's customers drawn from `rng`.
pub(crate) fn new_game(seed: u64, rng: &mut Rng) -> Game {
    let base = draw_base_customers(rng);
    let mut game = Game {
        seed,
        phase: Phase::Service { focus: None },
        day: 1,
        money: START_MONEY,
        rep: START_REP,
        capacity_left: BASE_CAPACITY,
        preps_tonight: 0,
        queue: Vec::new(),
        nemeses: Vec::new(),
        defeated: Vec::new(),
        spawned_per_theme: [0; 4],
        next_id: 1,
        tomorrow_base: Vec::new(),
        log: Vec::new(),
        tonight: Tonight::default(),
        notice: None,
        peak: (0, 1),
    };
    game.queue = queue_for(&game, &base);
    game
}

/// Today's queue: visiting nemeses first, in spawn order, then the six.
fn queue_for(game: &Game, base: &[Customer]) -> Vec<Order> {
    let mut queue: Vec<Order> = game
        .nemeses
        .iter()
        .filter(|n| n.next_visit == game.day)
        .map(|n| Order {
            kind: Kind::Nemesis(n.id),
            customer: Customer {
                theme: n.theme,
                need: NEMESIS_BASE_NEED + SENSITIVITY,
                temper: NEMESIS_TEMPER,
            },
            served: false,
        })
        .collect();
    let outlook = social_outlook(&game.nemeses, base);
    queue.extend(outlook.customers.into_iter().map(|(kind, customer)| Order {
        kind,
        customer,
        served: false,
    }));
    queue
}

/// Who an order is from, as the log and the rows say it.
pub(crate) fn who(game: &Game, row: usize) -> String {
    let order = &game.queue[row];
    let named = |id| {
        game.find_nemesis(id)
            .or_else(|| game.defeated.iter().find(|n| n.id == id))
            .map(title)
    };
    match order.kind {
        Kind::Nemesis(id) => named(id).unwrap_or("a nemesis").to_owned(),
        Kind::Follower { leader } => match game.find_nemesis(leader) {
            Some(n) => format!("follower of {}", title(n)),
            None => format!("customer {}", row + 1),
        },
        Kind::Ordinary => format!("customer {}", row + 1),
    }
}

fn defeat(game: &mut Game, id: NemesisId, line: &str) {
    if let Some(at) = game.nemeses.iter().position(|n| n.id == id) {
        let gone = game.nemeses.remove(at);
        game.log.push(format!("{line}: {}", title(&gone)));
        game.defeated.push(gone);
    }
}

fn clamp_rep(game: &mut Game) {
    game.rep = game.rep.clamp(0, MAX_REP);
}

/// Apply one action; a refused one says why in the notice and changes nothing else.
pub(crate) fn apply(game: &mut Game, action: Action, rng: &mut Rng) {
    game.notice = None;
    match (game.phase, action) {
        (Phase::Service { focus: None }, Action::Serve(row)) => serve(game, row),
        (Phase::Service { focus: Some(row) }, Action::ServeNemesis) => serve_nemesis(game, row),
        (Phase::Service { focus: Some(row) }, Action::Overwhelm) => overwhelm(game, row),
        (Phase::Service { focus: Some(_) }, Action::Unfocus) => {
            game.phase = Phase::Service { focus: None };
        }
        (Phase::Service { focus: None }, Action::CloseKitchen) => close_kitchen(game, rng),
        (Phase::Ledger, Action::Spend(slot)) => spend(game, slot),
        (Phase::Ledger, Action::Prep) => prep(game),
        (Phase::Ledger, Action::OpenTomorrow) => open_tomorrow(game),
        (Phase::Over { .. }, Action::Restart) => {
            let mut fresh = Rng::from_seed(game.seed);
            *game = new_game(game.seed, &mut fresh);
            *rng = fresh;
        }
        _ => {}
    }
}

fn serve(game: &mut Game, row: usize) {
    let Some(order) = game.queue.get(row).copied() else {
        return;
    };
    if order.served {
        game.notice = Some(format!("row {} is already served", row + 1));
        return;
    }
    if let Kind::Nemesis(_) = order.kind {
        game.phase = Phase::Service { focus: Some(row) };
        return;
    }
    if order.customer.need > game.capacity_left {
        game.notice = Some(format!(
            "not enough capacity: needs {}, {} left",
            order.customer.need, game.capacity_left
        ));
        return;
    }
    let reward = outcome_of(&order, game).if_served;
    game.capacity_left -= order.customer.need;
    game.money += reward.money;
    game.rep += reward.rep;
    clamp_rep(game);
    game.queue[row].served = true;
    game.tonight.served += 1;
    let line = format!(
        "served {} (+${}, +{} rep)",
        who(game, row),
        reward.money,
        reward.rep
    );
    game.log.push(line);
}

fn nemesis_at(game: &Game, row: usize) -> Option<Nemesis> {
    match game.queue.get(row)?.kind {
        Kind::Nemesis(id) => game.find_nemesis(id).copied(),
        _ => None,
    }
}

fn serve_nemesis(game: &mut Game, row: usize) {
    let Some(nemesis) = nemesis_at(game, row) else {
        return;
    };
    let order = game.queue[row];
    if order.customer.need > game.capacity_left {
        game.notice = Some(format!(
            "not enough capacity: needs {}, {} left",
            order.customer.need, game.capacity_left
        ));
        return;
    }
    let progress = defeat_progress(&nemesis, game.capacity_left);
    let reward = outcome_of(&order, game).if_served;
    game.capacity_left -= order.customer.need;
    game.money += reward.money;
    game.rep += reward.rep;
    game.queue[row].served = true;
    game.tonight.served += 1;
    game.phase = Phase::Service { focus: None };
    if let Some(n) = game.nemeses.iter_mut().find(|n| n.id == nemesis.id) {
        n.tally += 1;
    }
    game.log.push(format!(
        "served {} (+${}, +{} rep)",
        title(&nemesis),
        reward.money,
        reward.rep
    ));
    if progress.defeated_if_served {
        game.rep += REP_WON_OVER;
        defeat(game, nemesis.id, "WON OVER");
    }
    clamp_rep(game);
}

fn overwhelm(game: &mut Game, row: usize) {
    let Some(nemesis) = nemesis_at(game, row) else {
        return;
    };
    let progress = defeat_progress(&nemesis, game.capacity_left);
    let Some(cost) = progress.overwhelm else {
        game.notice = Some(format!(
            "overwhelm needs {} units, only {} left",
            NEMESIS_BASE_NEED + SENSITIVITY + OVERWHELM_EXTRA,
            game.capacity_left
        ));
        return;
    };
    let reward = outcome_of(&game.queue[row], game).if_served;
    game.capacity_left -= cost;
    game.money += reward.money;
    game.rep += reward.rep + REP_OVERWHELMED;
    clamp_rep(game);
    game.queue[row].served = true;
    game.tonight.served += 1;
    game.phase = Phase::Service { focus: None };
    defeat(game, nemesis.id, "OVERWHELMED");
}

fn spawn(game: &mut Game, theme: Theme) {
    let index = game.spawned_per_theme[theme.index()];
    game.spawned_per_theme[theme.index()] += 1;
    let nemesis = Nemesis {
        id: NemesisId(game.next_id),
        theme,
        title_index: index,
        followers: FOLLOWERS_AT_SPAWN,
        tally: 0,
        spawned_day: game.day,
        next_visit: game.day + RETURN_EVERY,
    };
    game.next_id += 1;
    game.log.push(format!(
        "SPAWNED: {} ({}), sensitivity +{SENSITIVITY} {}, {FOLLOWERS_AT_SPAWN} fol, returns day {}",
        title_for(theme, index),
        theme.name(),
        theme.name(),
        nemesis.next_visit
    ));
    game.nemeses.push(nemesis);
}

fn grow(game: &mut Game, id: NemesisId, followers: u32) {
    if let Some(n) = game.nemeses.iter_mut().find(|n| n.id == id) {
        n.followers += followers;
    }
}

/// Close the kitchen: every unserved order resolves unmet in queue order,
/// nightly effects, the viability check, then tomorrow's draw.
pub(crate) fn close_kitchen(game: &mut Game, rng: &mut Rng) {
    for row in 0..game.queue.len() {
        let order = game.queue[row];
        if order.served {
            continue;
        }
        let unmet = outcome_of(&order, game).if_unmet;
        let line = format!(
            "unmet {}: {} sev {}, -${}, -{} rep",
            who(game, row),
            unmet.theme.name(),
            unmet.severity,
            -unmet.money,
            -unmet.rep
        );
        game.log.push(line);
        game.money += unmet.money;
        game.rep += unmet.rep;
        game.tonight.unmet += 1;
        game.tonight.money_lost -= unmet.money;
        game.tonight.rep_lost -= unmet.rep;
        match order.kind {
            Kind::Nemesis(id) => grow(game, id, unmet.leader_followers),
            Kind::Follower { leader } => grow(game, leader, unmet.leader_followers),
            Kind::Ordinary => {}
        }
        match unmet.consequence {
            Consequence::Spawns => spawn(game, unmet.theme),
            Consequence::Feeds { id, followers } => {
                grow(game, id, followers);
                if let Some(n) = game.find_nemesis(id) {
                    let line = format!("FED: {} +{followers} fol (cap full)", title(n));
                    game.log.push(line);
                }
            }
            Consequence::Nothing => {}
        }
    }
    let day = game.day;
    for nemesis in &mut game.nemeses {
        if nemesis.next_visit == day {
            nemesis.next_visit += RETURN_EVERY;
        }
    }
    let trending: Vec<&'static str> = game
        .nemeses
        .iter()
        .filter(|n| tier_of(n.followers) == Tier::TrendingTerror)
        .map(title)
        .collect();
    for name in trending {
        game.rep -= TIER3_NIGHTLY_REP;
        game.log
            .push(format!("{name} is trending: -{TIER3_NIGHTLY_REP} rep"));
    }
    clamp_rep(game);
    game.phase = if game.money < 0 {
        Phase::Over {
            won: false,
            reason: "money",
        }
    } else if game.rep <= 0 {
        Phase::Over {
            won: false,
            reason: "reputation",
        }
    } else if game.day >= DAYS {
        Phase::Over {
            won: true,
            reason: "survived",
        }
    } else {
        game.tomorrow_base = draw_base_customers(rng);
        game.preps_tonight = 0;
        Phase::Ledger
    };
}

fn spend(game: &mut Game, slot: usize) {
    let Some(nemesis) = game.nemeses.get(slot).copied() else {
        return;
    };
    if game.money < SOCIAL_COST {
        game.notice = Some(format!(
            "not enough money: a spend is ${SOCIAL_COST}, you have ${}",
            game.money
        ));
        return;
    }
    let lands = defeat_progress(&nemesis, game.capacity_left).spends_to_ratio == 1;
    game.money -= SOCIAL_COST;
    cut_followers(&mut game.nemeses[slot]);
    if lands {
        defeat(game, nemesis.id, "RATIOED");
    }
}

fn prep(game: &mut Game) {
    if game.preps_tonight >= MAX_PREPS {
        game.notice = Some(format!("already prepped {MAX_PREPS} times tonight"));
        return;
    }
    if game.money < PREP_COST {
        game.notice = Some(format!(
            "not enough money: prep is ${PREP_COST}, you have ${}",
            game.money
        ));
        return;
    }
    game.money -= PREP_COST;
    game.preps_tonight += 1;
}

/// Open tomorrow: the queue from the nemeses visiting and `social_outlook`.
pub(crate) fn open_tomorrow(game: &mut Game) {
    game.day += 1;
    game.capacity_left = BASE_CAPACITY + PREP_BONUS * game.preps_tonight;
    game.preps_tonight = 0;
    game.log.clear();
    game.tonight = Tonight::default();
    let base = game.tomorrow_base.clone();
    game.queue = queue_for(game, &base);
    game.phase = Phase::Service { focus: None };
    if game.nemeses.len() > game.peak.0 {
        game.peak = (game.nemeses.len(), game.day);
    }
}
