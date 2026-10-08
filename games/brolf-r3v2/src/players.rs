//! The verify run's players, and the keyboard they press.
//!
//! An `Intent` becomes key edges here — events, not states (jidousha-
//! testing.md, `SnapshotBuilder`) — so the good player is `npc_decide` pressing
//! the same keys a person would, through `player_intent`. The three players are
//! the controllers document's three: good, chaser (the first-try player: drive
//! at the cup, never look at the zone), idle.

use jidousha::prelude::*;

use crate::model::*;
use crate::npc::{Goal, Intent, npc_decide};
use crate::rules::*;
use crate::world::{Snap, bearing};

/// Which player is at the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    /// The NPC's own competing rule.
    Good,
    /// Walks to its ball and drives at the cup, whatever the zone does.
    Chaser,
    /// Presses nothing.
    Idle,
    /// The NPC's extract rule.
    Extractor,
}

/// What `player` wants to do this tick.
pub fn intent_of(player: Player, view: &Snap) -> Intent {
    let Some(me) = view.golfer(0) else {
        return Intent::default();
    };
    match player {
        Player::Good => npc_decide(me, view, Goal::Compete, Temperament::Player),
        Player::Extractor => npc_decide(me, view, Goal::Extract, Temperament::Player),
        Player::Idle => Intent::default(),
        Player::Chaser => chase(me, view),
    }
}

/// The chaser: to the ball, then a DRIVE straight at the cup.
fn chase(me: &GolferSnap, view: &Snap) -> Intent {
    let Some(ball) = view.ball(0) else {
        return Intent::default();
    };
    if me.pos.distance(ball.pos) > SHOT_REACH * 0.8 {
        return Intent {
            walk: (ball.pos - me.pos).normalize_or_zero(),
            ..Intent::default()
        };
    }
    Intent {
        shoot: ball.vel == Vec2::ZERO,
        aim_to: Some(bearing(ball.pos, CUP)),
        power_to: Some(Power::Drive),
        ..Intent::default()
    }
}

/// The signed angle from `from` to `to`, in `(-pi, pi]`.
fn turn_between(from: Radians, to: Radians) -> f32 {
    let tau = Radians::TAU.as_f32();
    let d = (to.as_f32() - from.as_f32()).rem_euclid(tau);
    if d > tau / 2.0 { d - tau } else { d }
}

/// The keys an intent holds this tick, and the keys it taps.
pub fn keys_for(intent: &Intent, me: &GolferSnap, dt: Seconds) -> (Vec<Key>, Vec<Key>) {
    let mut held = Vec::new();
    let mut taps = Vec::new();
    let lean = 0.38;
    if intent.walk.x > lean {
        held.push(Key::D);
    } else if intent.walk.x < -lean {
        held.push(Key::A);
    }
    if intent.walk.y > lean {
        held.push(Key::S);
    } else if intent.walk.y < -lean {
        held.push(Key::W);
    }
    if intent.extract {
        held.push(Key::E);
    }
    let mut ready = true;
    if let Some(aim) = intent.aim_to {
        let turn = turn_between(me.aim, aim);
        if turn.abs() > AIM_RATE.as_f32() * dt.as_f32() {
            held.push(if turn > 0.0 {
                Key::ArrowRight
            } else {
                Key::ArrowLeft
            });
            ready = false;
        }
    }
    if let Some(power) = intent.power_to
        && power != me.power
    {
        let up = Power::ALL.iter().position(|&p| p == power)
            > Power::ALL.iter().position(|&p| p == me.power);
        taps.push(if up { Key::ArrowUp } else { Key::ArrowDown });
        ready = false;
    }
    if intent.shoot && ready {
        taps.push(Key::Space);
    }
    if intent.contact {
        taps.push(Key::C);
    }
    if intent.take {
        taps.push(Key::F);
    }
    (held, taps)
}
