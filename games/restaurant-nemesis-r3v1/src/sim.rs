//! The run as one plain value, and the steps that move it: open a day, cook a
//! round, close a day.
//!
//! `Game` is `Clone` and owns its `Rng`, so a check can copy it, try a choice
//! and look. Every consequence of a choice is computed by a function in
//! `rules.rs` — `cook_round` applies `outcome` and `visit_result`, `open_day`
//! applies `forecast` — so what a screen previews is what happens.

use jidousha::prelude::*;

use crate::content::Theme;
use crate::rules::{
    self, CAP_REPOST, DAYS, NIGHT_GROWTH, RENT, ROUNDS, START_MONEY, TAKEDOWN_COST, TEMP_COST,
};

/// How many customers walk in each round: one more than the kitchen.
pub const PER_ROUND: usize = 5;

/// How many extra customers the day's lunch rush brings, all in one round.
pub const RUSH: usize = 3;

/// How many ordinary customers a day brings.
pub const PER_DAY: usize = PER_ROUND * ROUNDS as usize + RUSH;

/// How hard a kitchen tries for one order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serve {
    /// Not at all: the order goes unmet.
    Skip,
    /// The usual.
    Standard,
    /// With attention.
    Careful,
    /// Everything the kitchen has.
    AllOut,
}

impl Serve {
    /// How well it meets a demand.
    pub fn quality(self) -> u8 {
        match self {
            Serve::Skip => 0,
            Serve::Standard => 2,
            Serve::Careful => 4,
            Serve::AllOut => 7,
        }
    }

    /// How much of the round's kitchen it takes.
    pub fn cost(self) -> u32 {
        match self {
            Serve::Skip => 0,
            Serve::Standard => 1,
            Serve::Careful => 2,
            Serve::AllOut => 4,
        }
    }

    /// The word on a row.
    pub fn name(self) -> &'static str {
        match self {
            Serve::Skip => "SKIP",
            Serve::Standard => "STANDARD",
            Serve::Careful => "CAREFUL",
            Serve::AllOut => "ALL-OUT",
        }
    }
}

/// Who an order belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    /// An ordinary diner, by name.
    Diner(usize),
    /// A follower of nemesis `n`.
    Follower(usize),
    /// Nemesis `n` in person.
    Nemesis(usize),
}

/// One waiting order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Order {
    /// Whose.
    pub who: Who,
    /// The demand in each theme, by `Theme::index`; 0 is no demand.
    pub demands: [u8; 4],
    /// What the kitchen will do for it.
    pub serve: Serve,
}

/// How a nemesis was beaten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefeatPath {
    /// Satisfied on enough visits.
    Tally,
    /// Blown away on one visit.
    Overwhelmed,
    /// Their following dwindled.
    Following,
}

/// A problem customer.
#[derive(Clone, Debug)]
pub struct Nemesis {
    /// Their title.
    pub title: String,
    /// What they are about.
    pub theme: Theme,
    /// How bad the failure that made them was.
    pub severity: u8,
    /// How many people hang on their every post.
    pub followers: i32,
    /// Visits they left satisfied.
    pub tally: u8,
    /// The day and round they were made.
    pub born: (u32, u32),
    /// How and when they were beaten, if they were.
    pub defeated: Option<(DefeatPath, u32, u32)>,
}

impl Nemesis {
    /// Whether they still come.
    pub fn active(&self) -> bool {
        self.defeated.is_none()
    }
}

/// Where the run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Between days: spending.
    Ledger,
    /// A round of service.
    Service,
    /// The run is over; `true` if it was won.
    Over(bool),
}

/// What the player chose to buy tonight.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spends {
    /// The nemeses getting a takedown, by index into `Game::nemeses`.
    pub takedowns: Vec<usize>,
    /// Whether a temp cook comes in tomorrow.
    pub temp: bool,
}

impl Spends {
    /// What these cost.
    pub fn cost(&self) -> i32 {
        TAKEDOWN_COST * self.takedowns.len() as i32 + if self.temp { TEMP_COST } else { 0 }
    }
}

/// Something that happened, for the log a check reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Nemesis `n` was made from a failure in `theme`.
    Spawned { nemesis: usize, theme: Theme },
    /// Nemesis `n` was beaten.
    Defeated { nemesis: usize, path: DefeatPath },
    /// An order was resolved.
    Served {
        who: Who,
        serve: Serve,
        satisfied: bool,
        money: i32,
    },
}

/// The whole run.
#[derive(Clone, Debug)]
pub struct Game {
    /// The seed it was drawn from.
    pub seed: u64,
    /// The run's own random numbers.
    pub rng: Rng,
    /// The day, from 1.
    pub day: u32,
    /// The round within the day, from 0.
    pub round: u32,
    /// Money in the till.
    pub money: i32,
    /// Where the run is.
    pub phase: Phase,
    /// Every nemesis ever made, in spawn order.
    pub nemeses: Vec<Nemesis>,
    /// Today's customers, round by round.
    pub today: Vec<Vec<Order>>,
    /// This round's queue.
    pub queue: Vec<Order>,
    /// Which row of the queue is picked.
    pub selected: usize,
    /// Tonight's spends.
    pub spends: Spends,
    /// Whether the temp cook is in today.
    pub temp_today: bool,
    /// What the feed shows, newest last.
    pub feed: Vec<String>,
    /// Everything that happened, with its day and round.
    pub log: Vec<(u32, u32, Event)>,
}

impl Game {
    /// A fresh run, at the first morning's ledger.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            rng: Rng::from_seed(seed),
            day: 1,
            round: 0,
            money: START_MONEY,
            phase: Phase::Ledger,
            nemeses: Vec::new(),
            today: Vec::new(),
            queue: Vec::new(),
            selected: 0,
            spends: Spends::default(),
            temp_today: false,
            feed: vec!["Day 1. The doors are open. What could go wrong?".to_owned()],
            log: Vec::new(),
        }
    }

    /// The nemeses still coming, by index, in spawn order.
    pub fn active(&self) -> Vec<usize> {
        (0..self.nemeses.len())
            .filter(|&index| self.nemeses[index].active())
            .collect()
    }

    /// The kitchen this round: how much there is, and how much the queue's
    /// serves use.
    pub fn kitchen(&self) -> (u32, u32) {
        let capacity = rules::CAPACITY + u32::from(self.temp_today);
        let used = self.queue.iter().map(|order| order.serve.cost()).sum();
        (capacity, used)
    }

    fn say(&mut self, line: String) {
        self.feed.push(line);
    }
}

/// Make a nemesis from a failure in `theme` of `severity`.
pub fn spawn(game: &mut Game, theme: Theme, severity: u8) -> usize {
    let taken: Vec<&str> = game
        .active()
        .iter()
        .map(|&index| game.nemeses[index].title.as_str())
        .collect();
    let titles = theme.titles();
    let start = game.rng.below(4) as usize;
    let pick = (0..4)
        .map(|offset| titles[(start + offset) % 4])
        .find(|title| !taken.contains(title));
    let title = match pick {
        Some(title) => title.to_owned(),
        None => format!("{} II", titles[start]),
    };
    let index = game.nemeses.len();
    game.nemeses.push(Nemesis {
        title: title.clone(),
        theme,
        severity,
        followers: rules::spawn_followers(severity),
        tally: 0,
        born: (game.day, game.round),
        defeated: None,
    });
    game.log.push((
        game.day,
        game.round,
        Event::Spawned {
            nemesis: index,
            theme,
        },
    ));
    game.say(format!(
        "A NEMESIS IS BORN: {title} ({} {}, {} followers)",
        theme.name(),
        severity,
        rules::spawn_followers(severity)
    ));
    index
}

/// Cook the round: resolve every order in the queue through `outcome`, then
/// move on to the next round or close the day.
pub fn cook_round(game: &mut Game) {
    if game.phase != Phase::Service {
        return;
    }
    let (day, round) = (game.day, game.round);
    let queue = game.queue.clone();
    for order in queue {
        let outcome = rules::outcome(game, &order, order.serve);
        game.money += outcome.money;
        game.log.push((
            day,
            round,
            Event::Served {
                who: order.who,
                serve: order.serve,
                satisfied: outcome.satisfied,
                money: outcome.money,
            },
        ));
        match outcome.consequence {
            rules::Consequence::None => {}
            rules::Consequence::Spawn => {
                if let Some((theme, severity)) = outcome.failure {
                    spawn(game, theme, severity);
                }
            }
            rules::Consequence::Repost { nemesis } => {
                game.nemeses[nemesis].followers += CAP_REPOST;
                let title = game.nemeses[nemesis].title.clone();
                game.say(format!(
                    "{title} reposts a fresh disaster: +{CAP_REPOST} followers"
                ));
            }
            rules::Consequence::Feeds {
                nemesis,
                followers,
                in_person,
            } => {
                game.nemeses[nemesis].followers += followers;
                if in_person {
                    let nemesis = &game.nemeses[nemesis];
                    let line = format!(
                        "@{}: \"{}\" (+{followers})",
                        nemesis.title.replace(' ', ""),
                        nemesis.theme.complaint()
                    );
                    game.say(line);
                }
            }
        }
        if let Who::Nemesis(index) = order.who {
            let result = rules::visit_result(game, index, &order, order.serve);
            let nemesis = &mut game.nemeses[index];
            if result.satisfied {
                nemesis.tally += 1;
            }
            if let Some(path) = result.defeats {
                nemesis.defeated = Some((path, day, round));
                let title = nemesis.title.clone();
                game.log.push((
                    day,
                    round,
                    Event::Defeated {
                        nemesis: index,
                        path,
                    },
                ));
                game.say(format!("{title} is DEFEATED ({})", rules::path_name(path)));
            }
        }
    }
    game.round += 1;
    if game.round >= ROUNDS {
        close_day(game);
        return;
    }
    game.queue = game.today[game.round as usize].clone();
    game.selected = 0;
}

/// Pay the rent; lose below zero, win after the last day; otherwise the
/// nemeses post overnight and the ledger opens.
fn close_day(game: &mut Game) {
    game.money -= RENT;
    game.queue.clear();
    if game.money < 0 {
        game.phase = Phase::Over(false);
        game.say(format!(
            "Rent day: ${}. The bank changed the locks.",
            game.money
        ));
        return;
    }
    if game.day >= DAYS {
        game.phase = Phase::Over(true);
        game.say("Seven days survived. The restaurant lives!".to_owned());
        return;
    }
    for index in game.active() {
        game.nemeses[index].followers += NIGHT_GROWTH;
    }
    game.day += 1;
    game.round = 0;
    game.phase = Phase::Ledger;
    game.say(format!("Night falls. Rent paid. ${} left.", game.money));
}

/// Toggle a takedown on the `slot`-th active nemesis.
pub fn toggle_takedown(game: &mut Game, slot: usize) {
    let Some(&index) = game.active().get(slot) else {
        return;
    };
    if let Some(at) = game.spends.takedowns.iter().position(|&n| n == index) {
        game.spends.takedowns.remove(at);
    } else {
        game.spends.takedowns.push(index);
    }
}

/// Set the selected order's serve, if the kitchen has room for it.
pub fn set_serve(game: &mut Game, serve: Serve) -> bool {
    let Some(order) = game.queue.get(game.selected).copied() else {
        return false;
    };
    let (capacity, used) = game.kitchen();
    if used - order.serve.cost() + serve.cost() > capacity {
        game.say(format!(
            "The kitchen is full: {} needs {}, {} free.",
            serve.name(),
            serve.cost(),
            capacity - used + order.serve.cost()
        ));
        return false;
    }
    game.queue[game.selected].serve = serve;
    true
}
