//! The three players a `--verify` run plays with, and the keyboard they type
//! on.
//!
//! A good player (protects against spawns, goes after nemeses, buys
//! takedowns), a first-timer (standard for everyone in the order they came,
//! never spends), and an idle one (only ever opens the door). Each decides
//! one `Command` a tick from the `Game`; `Typist` presses its key and lets it
//! go, so the game reads it through the same `commands_from` the window does.

use jidousha::prelude::*;
use jidousha::testing::{InputEvent, SnapshotBuilder};

use crate::rules::{self, Consequence, RENT, TAKEDOWN_COST, TEMP_COST};
use crate::sim::{Game, Phase, Serve, Spends, Who};
use crate::{Command, key_for};

/// Which player is at the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    /// Plays to win.
    Good,
    /// Standard for everyone, first come first served.
    FirstTimer,
    /// Opens the door and watches.
    Idle,
}

impl Player {
    /// The name in the summary.
    pub fn name(self) -> &'static str {
        match self {
            Player::Good => "good",
            Player::FirstTimer => "first-timer",
            Player::Idle => "idle",
        }
    }

    /// What this player types next, or nothing once the run is over.
    pub fn decide(self, game: &Game) -> Option<Command> {
        match game.phase {
            Phase::Over(_) => None,
            Phase::Ledger => {
                let want = match self {
                    Player::Good => good_spends(game),
                    _ => Spends::default(),
                };
                Some(toward_spends(game, &want))
            }
            Phase::Service => {
                let plan = match self {
                    Player::Good => good_round(game),
                    Player::FirstTimer => first_come(game),
                    Player::Idle => vec![Serve::Skip; game.queue.len()],
                };
                Some(toward_plan(game, &plan))
            }
        }
    }
}

/// The cheapest serve that satisfies `order`, if any does.
pub fn cheapest_satisfying(game: &Game, index: usize) -> Option<Serve> {
    let order = game.queue[index];
    [Serve::Standard, Serve::Careful, Serve::AllOut]
        .into_iter()
        .find(|&serve| rules::outcome(game, &order, serve).satisfied)
}

/// The good player's round: nemeses first (all-out when it leaves room to
/// stop every spawn), then every order whose failure would spawn or feed a
/// following, then the cheapest of the rest.
pub fn good_round(game: &Game) -> Vec<Serve> {
    let (capacity, _) = game.kitchen();
    let mut plan = vec![Serve::Skip; game.queue.len()];
    let risk = |index: usize| {
        let order = game.queue[index];
        match rules::outcome(game, &order, Serve::Skip).consequence {
            Consequence::Spawn | Consequence::Repost { .. } => 0,
            Consequence::Feeds { .. } => 1,
            Consequence::None => 2,
        }
    };
    let spawn_floor: u32 = (0..game.queue.len())
        .filter(|&index| risk(index) == 0 && !matches!(game.queue[index].who, Who::Nemesis(_)))
        .map(|_| Serve::Standard.cost())
        .sum();
    let mut left = capacity;
    for (slot, order) in plan.iter_mut().zip(&game.queue) {
        if let Who::Nemesis(_) = order.who {
            let serve = if left >= Serve::AllOut.cost() + spawn_floor {
                Serve::AllOut
            } else {
                Serve::Careful
            };
            if serve.cost() <= left {
                *slot = serve;
                left -= serve.cost();
            }
        }
    }
    let mut rest: Vec<usize> = (0..game.queue.len())
        .filter(|&index| plan[index] == Serve::Skip)
        .collect();
    rest.sort_by_key(|&index| {
        let cost = cheapest_satisfying(game, index).map_or(9, Serve::cost);
        (risk(index), cost, index)
    });
    for index in rest {
        // A standard is the cheapest way under the spawn line and pays the
        // most per unit of kitchen; only a follower is worth more care, since
        // failing one feeds their leader.
        let wanted = match risk(index) {
            1 => cheapest_satisfying(game, index).unwrap_or(Serve::Standard),
            _ => Serve::Standard,
        };
        let serve = if wanted.cost() <= left {
            wanted
        } else if Serve::Standard.cost() <= left {
            Serve::Standard
        } else {
            continue;
        };
        plan[index] = serve;
        left -= serve.cost();
    }
    plan
}

/// The first-timer's round: standard for everyone until the kitchen is full.
pub fn first_come(game: &Game) -> Vec<Serve> {
    let (capacity, _) = game.kitchen();
    (0..game.queue.len())
        .map(|index| {
            if (index as u32 + 1) * Serve::Standard.cost() <= capacity {
                Serve::Standard
            } else {
                Serve::Skip
            }
        })
        .collect()
}

/// The good player's spends: a takedown that finishes a nemesis, or one on a
/// tier-2+ nemesis when the margin can carry it; a temp cook when rich.
pub fn good_spends(game: &Game) -> Spends {
    let mut spends = Spends::default();
    let margin = |spends: &Spends| game.money - spends.cost() - RENT;
    for forecast in rules::forecast(game, &Spends::default()) {
        let finishes = forecast.followers - rules::TAKEDOWN_CUT < rules::FOLLOWER_FLOOR;
        let worth = finishes || forecast.tier >= 2;
        if worth && margin(&spends) - TAKEDOWN_COST >= 5 {
            spends.takedowns.push(forecast.nemesis);
        }
    }
    if margin(&spends) - TEMP_COST >= 5 {
        spends.temp = true;
    }
    spends
}

/// The next command toward `want` at the ledger.
fn toward_spends(game: &Game, want: &Spends) -> Command {
    for (slot, nemesis) in game.active().into_iter().enumerate() {
        let have = game.spends.takedowns.contains(&nemesis);
        if have != want.takedowns.contains(&nemesis) {
            return Command::Digit(slot as u8 + 1);
        }
    }
    if game.spends.temp != want.temp {
        return Command::Temp;
    }
    Command::Go
}

/// The next command toward `plan` at service: lower serves before raising
/// them, so the kitchen never refuses one.
fn toward_plan(game: &Game, plan: &[Serve]) -> Command {
    let lowering =
        (0..plan.len()).find(|&index| plan[index].cost() < game.queue[index].serve.cost());
    let raising = (0..plan.len()).find(|&index| plan[index] != game.queue[index].serve);
    let Some(index) = lowering.or(raising) else {
        return Command::Go;
    };
    if index != game.selected {
        return Command::Pick(if index > game.selected { 1 } else { -1 });
    }
    Command::Digit(match plan[index] {
        Serve::Skip => 0,
        Serve::Standard => 1,
        Serve::Careful => 2,
        Serve::AllOut => 3,
    })
}

/// A keyboard that taps one key and lets it go on the next tick.
pub struct Typist {
    builder: SnapshotBuilder,
    down: Option<Key>,
}

impl Default for Typist {
    fn default() -> Self {
        Self {
            builder: SnapshotBuilder::new(),
            down: None,
        }
    }
}

impl Typist {
    /// This tick's input: a release if a key is down, else `command`'s press.
    pub fn input_for(&mut self, command: Option<Command>) -> Input {
        if let Some(key) = self.down.take() {
            self.builder.record(InputEvent::KeyReleased(key));
        } else if let Some(command) = command {
            let key = key_for(command);
            self.builder.record(InputEvent::KeyPressed(key));
            self.down = Some(key);
        }
        Input::new(self.builder.first_tick_snapshot())
    }

    /// Whether the next tick is a release tick (no command is read).
    pub fn releasing(&self) -> bool {
        self.down.is_some()
    }
}
