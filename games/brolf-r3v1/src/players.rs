//! The three players a `--verify` run plays with, and the keyboard they
//! press.
//!
//! `docs/api/jidousha-controllers.md`: a good player (clears the mechanics),
//! a chaser (what a first-time player does: play the ball at the nearest cup
//! and ignore everything else), and an idle one (proves the match can be
//! lost). Each decides an `Intent` from the `Match`, and `Keyboard` turns it
//! into key events through `SnapshotBuilder`, so the game reads them through
//! the same `intent_from` the window does.

use jidousha::prelude::*;
use jidousha::testing::{InputEvent, SnapshotBuilder};

use crate::events::Intent;
use crate::npc::{angle_between, npc_intent};
use crate::rules::{ADDRESS, REACH};
use crate::sim::{Match, POWER_RATE, TURN_RATE, rules_aim_toward};

/// Which player is at the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    /// The NPC brain's extracting line, with a perfect aim.
    Good,
    /// Walk to the ball, play it at the nearest cup, nothing else.
    Chaser,
    /// Touches nothing.
    Idle,
}

impl Player {
    /// The name in the summary.
    pub fn name(self) -> &'static str {
        match self {
            Player::Good => "good",
            Player::Chaser => "chaser",
            Player::Idle => "idle",
        }
    }

    /// What this player wants this tick.
    pub fn decide(self, game: &Match) -> Intent {
        match self {
            // The player's brain is `Brain::Player`, which `npc_intent` plays
            // as a banker that never starts a fight.
            Player::Good => npc_intent(game, 0, 0.0),
            Player::Chaser => chase(game),
            Player::Idle => Intent::default(),
        }
    }
}

/// The first-timer: to the ball, then at the nearest cup at full stretch.
fn chase(game: &Match) -> Intent {
    let me = &game.golfers[0];
    let to_ball = me.ball.pos - me.pos;
    if !game.addressing(0) || to_ball.length() > ADDRESS * 0.7 {
        return Intent {
            walk: to_ball.normalize_or_zero(),
            ..Intent::default()
        };
    }
    let cup = (0..game.cups.len())
        .filter(|&cup| game.cup_open(cup))
        .min_by(|&a, &b| {
            (game.cups[a] - me.ball.pos)
                .length()
                .total_cmp(&(game.cups[b] - me.ball.pos).length())
        });
    let target = cup.map_or(Vec2::ZERO, |cup| game.cups[cup]);
    let aim = rules_aim_toward(me.ball.pos, target);
    let power = ((target - me.ball.pos).length() / REACH).min(1.0);
    let turn = angle_between(me.aim, aim) / (TURN_RATE.to_radians() / 60.0);
    let push = (power - me.power) / (POWER_RATE / 60.0);
    Intent {
        turn: turn.clamp(-1.0, 1.0),
        power: push.clamp(-1.0, 1.0),
        shoot: turn.abs() <= 1.0 && push.abs() <= 1.0,
        ..Intent::default()
    }
}

/// A keyboard driven by intents: held keys for the axes, a press and a
/// release for each commit.
pub struct Keyboard {
    builder: SnapshotBuilder,
    down: Vec<Key>,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            builder: SnapshotBuilder::new(),
            down: Vec::new(),
        }
    }
}

impl Keyboard {
    /// The keys `intent` wants held, and the ones it wants tapped.
    fn keys_for(intent: Intent) -> (Vec<Key>, Vec<Key>) {
        let mut held = Vec::new();
        let axis = |value: f32, minus: Key, plus: Key, held: &mut Vec<Key>| {
            if value > 0.38 {
                held.push(plus);
            } else if value < -0.38 {
                held.push(minus);
            }
        };
        axis(intent.walk.x, Key::A, Key::D, &mut held);
        axis(intent.walk.y, Key::W, Key::S, &mut held);
        axis(intent.turn, Key::ArrowLeft, Key::ArrowRight, &mut held);
        axis(intent.power, Key::ArrowDown, Key::ArrowUp, &mut held);
        let mut tapped = Vec::new();
        for (wanted, key) in [
            (intent.shoot, Key::Space),
            (intent.contact, Key::F),
            (intent.take, Key::E),
            (intent.extract, Key::X),
        ] {
            if wanted {
                tapped.push(key);
            }
        }
        (held, tapped)
    }

    /// This tick's input for `intent`. A tap is released on the tick after
    /// it, so asking twice in a row presses once and then waits a tick.
    pub fn input_for(&mut self, intent: Intent) -> Input {
        let (held, tapped) = Self::keys_for(intent);
        let mut next: Vec<Key> = held.clone();
        for key in self.down.clone() {
            if !held.contains(&key) {
                self.builder.record(InputEvent::KeyReleased(key));
            }
        }
        for key in &held {
            if !self.down.contains(key) {
                self.builder.record(InputEvent::KeyPressed(*key));
            }
        }
        for key in tapped {
            if !self.down.contains(&key) {
                self.builder.record(InputEvent::KeyPressed(key));
                next.push(key);
            }
        }
        self.down = next;
        Input::new(self.builder.first_tick_snapshot())
    }
}
