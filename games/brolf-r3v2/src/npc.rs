//! The NPCs' decisions, as one pure function over a `Snap`.
//!
//! `npc_decide` hands back the same `Intent` the keyboard produces, so the
//! player and the NPCs go through one set of apply systems, and the verify
//! run's good player is this function pressing keys (DESIGN.md, Systems: npc).
//! Every choice of shot is "constrain first, then optimise"
//! (`docs/api/jidousha-controllers.md`): only landings inside the zone as it
//! will be on arrival are on the menu.

use jidousha::prelude::*;

use crate::model::*;
use crate::rules::*;
use crate::world::{Snap, bearing};

/// What a golfer means to do this tick — from the keyboard or from an NPC.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Intent {
    /// Unit direction to walk in, or zero.
    pub walk: Vec2,
    /// Turn the aim: -1, 0 or +1 (the keyboard's arrows).
    pub turn: i8,
    /// Step the power: -1, 0 or +1.
    pub power_step: i8,
    /// Shoot the ball.
    pub shoot: bool,
    /// Swing at whatever `contact_target` names.
    pub contact: bool,
    /// Take the pickup underfoot.
    pub take: bool,
    /// Hold extract.
    pub extract: bool,
    /// The aim an NPC sets directly. NPCs skip the turn rate (DESIGN.md Non-goals).
    pub aim_to: Option<Radians>,
    /// The power an NPC sets directly.
    pub power_to: Option<Power>,
}

/// What an NPC is playing for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    /// Hole out, survive, and (a Hunter) club whoever comes near.
    Compete,
    /// Get ball and self into the gate and hold extract.
    Extract,
}

/// Ticks to walk `distance` at walking speed.
fn walk_ticks(distance: f32, dt: Seconds) -> u64 {
    (distance / (WALK_SPEED * dt.as_f32())).ceil() as u64
}

/// A Banker extracts once it holds two items and the gate is still reachable
/// in time; everyone else competes.
pub fn temperament_goal(temperament: Temperament, me: &GolferSnap, view: &Snap) -> Goal {
    if temperament != Temperament::Banker || me.kit.items().len() < 2 {
        return Goal::Compete;
    }
    let walk = walk_ticks(me.pos.distance(GATE_CENTER), view.dt);
    let needed = view.now + 2 * walk + u64::from(EXTRACT_HOLD);
    let deadline = zone_excludes_at(GATE_CENTER).map_or(u64::MAX, |t| t + u64::from(GRACE_TICKS));
    if needed < deadline {
        Goal::Extract
    } else {
        Goal::Compete
    }
}

/// Walk toward `to`, or stand if already there.
fn toward(from: Vec2, to: Vec2) -> Vec2 {
    if from.distance(to) < 0.1 {
        Vec2::ZERO
    } else {
        (to - from).normalize_or_zero()
    }
}

/// Go to my ball; at it, aim at `target` and shoot the power `choose` picks.
fn play_ball(
    me: &GolferSnap,
    view: &Snap,
    target: Vec2,
    choose: impl Fn(&[(Power, RollOut)]) -> Option<Power>,
) -> Intent {
    let Some(ball) = view.ball(me.idx) else {
        return Intent::default();
    };
    if me.pos.distance(ball.pos) > SHOT_REACH * 0.8 {
        return Intent {
            walk: toward(me.pos, ball.pos),
            ..Intent::default()
        };
    }
    if ball.vel != Vec2::ZERO || me.cooldown > 0 {
        return Intent::default();
    }
    let aim = bearing(ball.pos, target);
    let fx = effects(&me.kit);
    let rolls: Vec<(Power, RollOut)> = Power::ALL
        .iter()
        .map(|&power| {
            let vel = shot_velocity(aim, power, fx);
            (power, roll_out(ball.pos, vel, view.dt, view.now + 1))
        })
        .collect();
    let power = choose(&rolls).unwrap_or(Power::Chip);
    Intent {
        shoot: true,
        aim_to: Some(aim),
        power_to: Some(power),
        ..Intent::default()
    }
}

/// Whether a roll ends safe: inside the zone as it will be when I get there.
fn rests_safe(me: &GolferSnap, view: &Snap, roll: &RollOut) -> bool {
    let arrival = arrival_tick(view.now, me.pos, roll, view.dt);
    roll.sunk || inside_zone(zone_at(arrival), roll.rest)
}

/// The power whose roll ends nearest `target`, among `rolls` that pass `keep`
/// (or among all of them, if none do).
fn nearest(
    rolls: &[(Power, RollOut)],
    target: Vec2,
    keep: impl Fn(&RollOut) -> bool,
) -> Option<Power> {
    let score = |roll: &RollOut| {
        if roll.sunk {
            -1.0
        } else {
            roll.rest.distance(target)
        }
    };
    let pick = |candidates: Vec<&(Power, RollOut)>| {
        candidates
            .into_iter()
            .min_by(|a, b| score(&a.1).total_cmp(&score(&b.1)))
            .map(|(power, _)| *power)
    };
    let safe: Vec<_> = rolls.iter().filter(|(_, roll)| keep(roll)).collect();
    if safe.is_empty() {
        pick(rolls.iter().collect())
    } else {
        pick(safe)
    }
}

/// An NPC's decision this tick.
pub fn npc_decide(me: &GolferSnap, view: &Snap, goal: Goal, temperament: Temperament) -> Intent {
    if !me.alive || me.stun_left > 0 {
        return Intent::default();
    }
    if goal == Goal::Extract {
        return extract_intent(me, view);
    }
    let ball = view.ball(me.idx);
    let soon = zone_at(view.now + 60);
    let ball_out = ball.is_some_and(|ball| !inside_zone(soon, ball.pos));
    if !inside_zone(soon, me.pos) || ball_out {
        return play_ball(me, view, CUP, |rolls| {
            let safe: Vec<_> = rolls
                .iter()
                .filter(|(_, roll)| rests_safe(me, view, roll))
                .collect();
            // The largest power that still lands safe; else the nearest the cup.
            safe.last()
                .map(|(power, _)| *power)
                .or_else(|| nearest(rolls, CUP, |_| true))
        });
    }
    if temperament == Temperament::Hunter {
        // Prey is a rival with loot a club would knock loose, and not already
        // disabled. DESIGN.md's rule (any rival within 8) re-clubs every
        // `SWING_COOLDOWN` while a club disables for three times that, which
        // locks the victim down for good — a permanent kill by another road
        // (FINDINGS.md G-074). Looting is the incentive DESIGN.md gives
        // aggression, so the Hunter hunts for it.
        let prey = view
            .golfers
            .iter()
            .filter(|other| other.alive && other.idx != me.idx && other.stun_left == 0)
            .filter(|other| !other.kit.items().is_empty() && effects(&other.kit).drops_when_clubbed)
            .filter(|other| other.pos.distance(me.pos) <= 8.0)
            .min_by(|a, b| a.pos.distance(me.pos).total_cmp(&b.pos.distance(me.pos)));
        if let Some(prey) = prey {
            let target = contact_target(me, &view.golfers, &view.balls, effects(&me.kit));
            return Intent {
                walk: toward(me.pos, prey.pos),
                contact: matches!(target, Some(Contact::Club(_))) && me.cooldown == 0,
                ..Intent::default()
            };
        }
    }
    let wanted = view
        .pickups
        .iter()
        .filter(|(at, _)| at.distance(me.pos) <= 6.0)
        .find(|(_, item)| {
            me.kit
                .get(item.category())
                .is_none_or(|held| item.value() > held.value())
        });
    if let Some(&(at, _)) = wanted {
        return Intent {
            walk: toward(me.pos, at),
            take: at.distance(me.pos) <= TAKE_RANGE,
            ..Intent::default()
        };
    }
    play_ball(me, view, CUP, |rolls| {
        nearest(rolls, CUP, |roll| rests_safe(me, view, roll))
    })
}

/// Get the ball into the gate, then stand in it and hold extract.
fn extract_intent(me: &GolferSnap, view: &Snap) -> Intent {
    let gate = gate();
    let Some(ball) = view.ball(me.idx) else {
        return Intent::default();
    };
    let ball_in = gate.contains(ball.pos) && ball.vel == Vec2::ZERO;
    if !ball_in {
        return play_ball(me, view, GATE_CENTER, |rolls| {
            nearest(rolls, GATE_CENTER, |_| true)
        });
    }
    Intent {
        walk: toward(me.pos, GATE_CENTER),
        extract: gate.contains(me.pos),
        ..Intent::default()
    }
}
