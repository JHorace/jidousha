//! The Update systems that resolve a tick once every golfer has moved: the
//! acts, the rolls, the cups, the zone and the end of the match.
//!
//! Re-exported whole from `sim`, the one path `main.rs` registers them by;
//! split out only to keep `sim.rs` under the size convention.

use jidousha::prelude::*;

use super::{
    Ball, Golfer, Intent, Match, Pickup, Roll, Snapshot, live, read_snapshot, spawn_pickup,
};
use crate::rules::*;

/// Strikes, clubs, takes and extractions — every intent judged against one
/// snapshot taken before any of them lands, in seat order.
pub fn act(world: &mut World) {
    if !live(world) {
        return;
    }
    let Some(snap) = read_snapshot(&world.view()) else {
        return;
    };
    let tick = snap.tick;
    let mut strikes: Vec<(usize, Roll)> = Vec::new();
    let mut points = [0u32; 4];
    let mut stuns: Vec<(usize, u64, Vec2)> = Vec::new();
    let mut cooled: Vec<usize> = Vec::new();
    let mut reset_charge: Vec<usize> = Vec::new();
    let mut takes: Vec<(usize, usize)> = Vec::new();
    let mut extracts: Vec<(usize, u32)> = Vec::new();
    let intents: Vec<Intent> = {
        let mut list: Vec<(usize, Intent)> = world
            .query::<&Golfer>()
            .map(|(_, g)| (g.seat, g.intent))
            .collect();
        list.sort_by_key(|(seat, _)| *seat);
        list.into_iter().map(|(_, intent)| intent).collect()
    };
    for me in &snap.golfers {
        if me.fate != Fate::Playing {
            continue;
        }
        let seat = me.seat;
        let intent = intents[seat];
        let reach = snap.reach(seat);
        let released = me.charge > 0 && !intent.space_held;
        if released {
            reset_charge.push(seat);
            if let Some(owner) = reach.ball
                && !strikes.iter().any(|(o, _)| *o == owner)
            {
                let own = owner == seat;
                let speed = shot_speed_for(
                    &effects(&me.held),
                    &effects(&snap.golfer(owner).held),
                    own,
                    me.charge,
                );
                let from = snap.ball(owner).pos;
                let dir = polar(1.0, me.aim);
                let landing = landing_of(from, dir, speed, course(), snap.dt);
                strikes.push((
                    owner,
                    Roll {
                        from,
                        dir,
                        speed,
                        started: tick,
                        landing,
                    },
                ));
                if !own {
                    points[seat] += SLEDGE_POINTS;
                }
            }
        }
        if intent.club
            && me.cooldown == 0
            && let Some(target) = reach.club
            && !stuns.iter().any(|(t, _, _)| *t == target)
        {
            let victim = snap.golfer(target);
            let until = tick + effects(&victim.held).stun_ticks;
            let away = (victim.pos - me.pos).normalize_or_zero();
            let fence = course();
            let drop_at = (victim.pos + away * 0.6).clamp(fence.min, fence.max);
            stuns.push((target, until, drop_at));
            cooled.push(seat);
        }
        if intent.take
            && let Some(index) = reach.pickup
            && !takes.iter().any(|(_, i)| *i == index)
        {
            takes.push((seat, index));
        }
        let ball = snap.ball(seat);
        if intent.extract
            && snap.pad_open
            && ball.at_rest
            && snap.course.pad.contains(me.pos)
            && snap.course.pad.contains(ball.pos)
        {
            extracts.push((seat, bank_now(me.points, &me.held)));
        }
    }
    apply_acts(
        world,
        &snap,
        ActOutcome {
            strikes,
            points,
            stuns,
            cooled,
            reset_charge,
            takes,
            extracts,
        },
    );
}

/// Everything `act` decided, to be written back.
struct ActOutcome {
    strikes: Vec<(usize, Roll)>,
    points: [u32; 4],
    stuns: Vec<(usize, u64, Vec2)>,
    cooled: Vec<usize>,
    reset_charge: Vec<usize>,
    takes: Vec<(usize, usize)>,
    extracts: Vec<(usize, u32)>,
}

fn apply_acts(world: &mut World, snap: &Snapshot, outcome: ActOutcome) {
    let pickup_entities: Vec<Entity> = world.query::<&Pickup>().map(|(e, _)| e).collect();
    let mut drops: Vec<(Vec2, Item)> = Vec::new();
    for (_, golfer) in world.query_mut::<&mut Golfer>() {
        let seat = golfer.seat;
        golfer.points += outcome.points[seat];
        if outcome.reset_charge.contains(&seat) {
            golfer.charge = 0;
        }
        if outcome.cooled.contains(&seat) {
            golfer.cooldown = CLUB_COOLDOWN_TICKS;
        }
        if let Some((_, until, drop_at)) = outcome.stuns.iter().find(|(t, _, _)| *t == seat) {
            golfer.fate = Fate::Stunned { until: *until };
            golfer.charge = 0;
            if let Some(item) = golfer.held.pop() {
                drops.push((*drop_at, item));
            }
        }
        if let Some((_, index)) = outcome.takes.iter().find(|(s, _)| *s == seat) {
            let picked = snap.pickups[*index];
            if golfer.held.len() >= SLOTS {
                let oldest = golfer.held.remove(0);
                drops.push((picked.pos, oldest));
            }
            golfer.held.push(picked.item);
        }
        if let Some((_, bank)) = outcome.extracts.iter().find(|(s, _)| *s == seat) {
            golfer.fate = Fate::Extracted { banked: *bank };
        }
    }
    for (_, index) in &outcome.takes {
        world.despawn(pickup_entities[*index]);
    }
    for (at, item) in drops {
        spawn_pickup(world, at, item);
    }
    for (_, ball, transform) in world.query_mut::<(&mut Ball, &mut Transform)>() {
        if let Some((_, roll)) = outcome.strikes.iter().find(|(o, _)| *o == ball.owner) {
            ball.roll = Some(*roll);
            transform.pos = roll.from;
        }
    }
}

/// Every rolling ball placed where the closed form says; stopped exactly on
/// its landing when the roll is spent.
pub fn roll_balls(world: &mut World) {
    if !live(world) {
        return;
    }
    let time = *world.resource::<Time>();
    for (_, ball, transform) in world.query_mut::<(&mut Ball, &mut Transform)>() {
        let Some(roll) = ball.roll else { continue };
        let into = time.tick - roll.started;
        if into >= roll.landing.ticks {
            transform.pos = roll.landing.at;
            ball.roll = None;
            ball.rested_at = time.tick;
        } else {
            let along = rolled(roll.speed, into, time.fixed_dt).min(roll.landing.distance);
            transform.pos = roll.from + roll.dir * along;
        }
    }
}

/// A ball that came to rest in a cup scores, drops out beside it, and the cup
/// moves inside the next zone.
pub fn settle_balls(world: &mut World) {
    if !live(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    let mut course = world.resource::<Course>().clone();
    let mut rested: Vec<(usize, Entity, Vec2)> = world
        .query::<(&Ball, &Transform)>()
        .filter(|(_, b, _)| b.roll.is_none() && b.rested_at == tick && tick > 0)
        .map(|(e, b, t)| (b.owner, e, t.pos))
        .collect();
    rested.sort_by_key(|(owner, _, _)| *owner);
    let mut scored: Vec<usize> = Vec::new();
    for (owner, entity, pos) in rested {
        let Some(cup) = (0..3).find(|i| course.cups[*i].distance(pos) <= CUP_RADIUS) else {
            continue;
        };
        let at = course.cups[cup];
        let zone = zone_at(&course, tick);
        let away = (zone.center - at).normalize_or_zero();
        let away = if away == Vec2::ZERO { Vec2::X } else { away };
        world.component_mut::<Transform>(entity).pos = at + away;
        scored.push(owner);
        let others: Vec<Vec2> = (0..3)
            .filter(|i| *i != cup)
            .map(|i| course.cups[i])
            .collect();
        let into = next_zone(&course, tick).unwrap_or(zone);
        let pad = course.pad.center;
        course.cups[cup] = redraw_cup(world.resource_mut::<Rng>(), into, &others, pad);
    }
    for (_, golfer) in world.query_mut::<&mut Golfer>() {
        if golfer.fate.alive() {
            golfer.points +=
                HOLE_POINTS * scored.iter().filter(|s| **s == golfer.seat).count() as u32;
        }
    }
    world.insert_resource(course);
}

/// A golfer whose body or ball has been outside the zone for the grace
/// period is eliminated, on the tick `zone_at` says.
pub fn judge_zone(world: &mut World) {
    if !live(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    let zone = zone_at(world.resource::<Course>(), tick);
    let balls: Vec<(usize, Vec2)> = world
        .query::<(&Ball, &Transform)>()
        .map(|(_, b, t)| (b.owner, t.pos))
        .collect();
    for (_, golfer, transform) in world.query_mut::<(&mut Golfer, &Transform)>() {
        if !golfer.fate.alive() {
            continue;
        }
        let ball = balls
            .iter()
            .find(|(o, _)| *o == golfer.seat)
            .map(|(_, p)| *p);
        let out = !zone.contains(transform.pos) || ball.is_none_or(|p| !zone.contains(p));
        golfer.out_ticks = if out { golfer.out_ticks + 1 } else { 0 };
        if golfer.out_ticks >= GRACE_TICKS {
            golfer.fate = Fate::Eliminated { at: tick };
        }
    }
}

/// When nobody is left playing the match is over; when one golfer is left and
/// every rival was eliminated, it survives and banks the bonus.
///
/// A rival that extracted is not one the last golfer outlasted: it keeps
/// playing alone, and must extract or meet the closed zone. So standing still
/// is never a win (DESIGN.md departure: the design also crowned a survivor
/// whose rivals had extracted, and the idle player then won by doing nothing).
pub fn end_match(world: &mut World) {
    if !live(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    let alive: Vec<usize> = world
        .query::<&Golfer>()
        .filter(|(_, g)| g.fate.alive())
        .map(|(_, g)| g.seat)
        .collect();
    let all_eliminated = world
        .query::<&Golfer>()
        .filter(|(_, g)| !g.fate.alive())
        .all(|(_, g)| matches!(g.fate, Fate::Eliminated { .. }));
    if alive.len() > 1 || (alive.len() == 1 && !all_eliminated) {
        return;
    }
    if let [last] = alive[..] {
        for (_, golfer) in world.query_mut::<&mut Golfer>() {
            if golfer.seat == last {
                golfer.fate = Fate::Survived {
                    banked: bank_now(golfer.points, &golfer.held) + SURVIVOR_BONUS,
                };
            }
        }
    }
    let mut banks: Vec<(usize, u32)> = world
        .query::<&Golfer>()
        .map(|(_, g)| (g.seat, banked(g.fate, g.points, &g.held)))
        .collect();
    banks.sort_by_key(|(seat, _)| *seat);
    world.insert_resource(Match::Over {
        at: tick,
        winner: winner_of(&banks),
    });
}

/// The seat with the greatest bank, the lowest seat on a tie; nobody if the
/// greatest is nothing.
pub fn winner_of(banks: &[(usize, u32)]) -> Option<usize> {
    let mut best: Option<(usize, u32)> = None;
    for (seat, bank) in banks {
        if best.is_none_or(|(_, b)| *bank > b) {
            best = Some((*seat, *bank));
        }
    }
    best.filter(|(_, bank)| *bank > 0).map(|(seat, _)| seat)
}
