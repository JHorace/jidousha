//! The morning: tonight's spends paid, the follower forecast applied, and
//! the day's customers generated from it — followers placed by a seeded
//! shuffle, one round drawn as the lunch rush, and every nemesis's visit in
//! its round.

use jidousha::prelude::*;

use crate::content::{DINERS, THEMES};
use crate::rules::{self, ROUNDS};
use crate::sim::{
    DefeatPath, Event, Game, Order, PER_DAY, PER_ROUND, Phase, RUSH, Serve, Spends, Who,
};

/// An ordinary diner's demands: one theme (60%) or two, each at level 1
/// (40%), 2 (35%) or 3 (25%).
fn diner_demands(rng: &mut Rng) -> [u8; 4] {
    let mut demands = [0; 4];
    let themes = if rng.below(100) < 60 { 1 } else { 2 };
    let first = rng.below(4) as usize;
    let picks = [first, (first + 1 + rng.below(3) as usize) % 4];
    for &theme in &picks[..themes] {
        let roll = rng.below(100);
        demands[theme] = if roll < 40 {
            1
        } else if roll < 75 {
            2
        } else {
            3
        };
    }
    demands
}

/// Open the day: pay the spends, apply the forecast, and generate the day's
/// customers from it.
pub fn open_day(game: &mut Game) {
    let forecasts = rules::forecast(game, &game.spends.clone());
    game.money -= game.spends.cost();
    game.temp_today = game.spends.temp;
    let (day, round) = (game.day, 0);
    let mut slots: Vec<Option<(usize, u8)>> = vec![None; PER_DAY];
    let mut next = 0;
    for forecast in &forecasts {
        game.nemeses[forecast.nemesis].followers = forecast.followers;
        if forecast.defeated {
            game.nemeses[forecast.nemesis].defeated = Some((DefeatPath::Following, day, round));
            game.log.push((
                day,
                round,
                Event::Defeated {
                    nemesis: forecast.nemesis,
                    path: DefeatPath::Following,
                },
            ));
            let title = game.nemeses[forecast.nemesis].title.clone();
            game.feed
                .push(format!("{title} logged off for good. Followers scatter."));
            continue;
        }
        for _ in 0..forecast.count {
            slots[next] = Some((forecast.nemesis, forecast.level));
            next += 1;
        }
    }
    // Scatter the followers over the day: a seeded shuffle of the slots.
    for index in (1..slots.len()).rev() {
        let other = game.rng.below(index as u32 + 1) as usize;
        slots.swap(index, other);
    }
    // One round a day, drawn from the seed, is the lunch rush.
    let rush = game.rng.below(ROUNDS) as usize;
    let mut today: Vec<Vec<Order>> = Vec::new();
    let mut taken = 0;
    for round in 0..ROUNDS as usize {
        let size = PER_ROUND + if round == rush { RUSH } else { 0 };
        let mut orders = Vec::new();
        for slot in &slots[taken..taken + size] {
            let mut demands = diner_demands(&mut game.rng);
            let who = match *slot {
                Some((leader, level)) => {
                    demands[game.nemeses[leader].theme.index()] = level;
                    Who::Follower(leader)
                }
                None => Who::Diner(game.rng.below(DINERS.len() as u32) as usize),
            };
            orders.push(Order {
                who,
                demands,
                serve: Serve::Skip,
            });
        }
        taken += size;
        today.push(orders);
    }
    for index in game.active() {
        let nemesis = &game.nemeses[index];
        if nemesis.born.0 >= game.day {
            continue;
        }
        let visit = (nemesis.born.1 + (game.day - nemesis.born.0)) % ROUNDS;
        let mut demands = [0; 4];
        demands[nemesis.theme.index()] = rules::NEMESIS_BASE + rules::SENSITIVITY;
        let other = THEMES[(nemesis.theme.index() + 1 + game.rng.below(3) as usize) % 4];
        demands[other.index()] = 1;
        today[visit as usize].push(Order {
            who: Who::Nemesis(index),
            demands,
            serve: Serve::Skip,
        });
    }
    game.today = today;
    game.spends = Spends::default();
    game.round = 0;
    game.queue = game.today[0].clone();
    game.selected = 0;
    game.phase = Phase::Service;
    let followers: usize = forecasts
        .iter()
        .filter(|f| !f.defeated)
        .map(|f| f.count)
        .sum();
    game.feed.push(format!(
        "Day {}: {PER_DAY} customers, {followers} of them following someone. Lunch rush in round {}!",
        game.day,
        rush + 1
    ));
}
