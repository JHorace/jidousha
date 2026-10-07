//! The Update systems, registered in exactly the order they run.
//!
//! Key functions: `decide`, `walk`, `swing`, `shoot`, `fly`, `pickup`, `expose`,
//! `extract`, `settle`.
//! Depends on: `contact`, `intent`, `items`, `outcome`, `shots`, `world`, `zone`.
//! Never depended on outside the game.
//! INVARIANT: the order is the sequence of `add_system` calls in `main.rs`, and the
//! verify run asserts it: a swing resolves on positions after the walk, a ball
//! flies after it is hit, a pickup is taken after a club drops it, the zone is read
//! after everything has moved, and the match settles last.

use jidousha::prelude::*;

use crate::contact::{Contact, PlayerSnap, SWING_COOLDOWN, contact_target, snap_players};
use crate::intent::{Intent, intent_from_input, npc_intent, pointer_world};
use crate::items::Item;
use crate::outcome::{EXTRACT_TICKS, PAD_RADIUS, PadState, kept_on, settle as settle_match};
use crate::shots::{Flight, MAX_SHOT, REACH_BALL, aim_point, flight_ticks, into_course, scatter};
use crate::world::{
    Ball, Course, Eliminated, Intents, MatchState, Persona, Pickup, Player, Pointer, Stats,
};
use crate::zone::{GRACE_TICKS, zone_at};

/// How fast a golfer walks, in world units per second.
pub const WALK_SPEED: f32 = 4.0;
/// How near a golfer must be to a pickup to take it.
pub const PICKUP_RADIUS: f32 = 0.6;

/// Whether the match is still on.
fn playing(world: &World) -> bool {
    matches!(world.resource::<MatchState>(), MatchState::Playing)
}

/// The body entity of golfer `index`, if they are still in play.
fn player_entity(world: &World, index: usize) -> Option<Entity> {
    world
        .query::<&Player>()
        .find(|(_, player)| player.index == index)
        .map(|(entity, _)| entity)
}

/// The ball entity of golfer `index`, if they are still in play.
fn ball_entity(world: &World, index: usize) -> Option<Entity> {
    world
        .query::<&Ball>()
        .find(|(_, ball)| ball.owner == index)
        .map(|(entity, _)| entity)
}

/// Fill `Intents`: the human from the keyboard, every rival from `npc_intent`.
pub fn decide(world: &mut World) {
    if !playing(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    let snaps = snap_players(&world.view());
    let pickups: Vec<(Vec2, Item)> = world
        .query::<(&Transform, &Pickup)>()
        .map(|(_, transform, pickup)| (transform.pos, pickup.0))
        .collect();
    let course = *world.resource::<Course>();
    let human = match (
        world.find_resource::<Input>(),
        world.find_resource::<Camera>(),
    ) {
        (Some(input), Some(camera)) => Some((
            intent_from_input(input, camera),
            pointer_world(input, camera),
        )),
        _ => None,
    };
    let intents = snaps
        .iter()
        .map(|snap| {
            let intent = if snap.persona == Persona::Human {
                human.map_or(Intent::NONE, |(intent, _)| intent)
            } else {
                npc_intent(snap, &snaps, &pickups, &course, tick)
            };
            (snap.index, intent)
        })
        .collect();
    if let Some((_, pointer)) = human {
        world.insert_resource(Pointer(pointer));
    }
    world.insert_resource(Intents(intents));
}

/// Walk: dazed and extracting golfers stand where they are.
pub fn walk(world: &mut World) {
    if !playing(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    let intents = world.resource::<Intents>().0.clone();
    for (_, transform, player) in world.query_mut::<(&mut Transform, &Player)>() {
        let Some((_, intent)) = intents.iter().find(|(index, _)| *index == player.index) else {
            continue;
        };
        if tick < player.dazed_until || player.extracting_since.is_some() {
            continue;
        }
        let speed = WALK_SPEED * player.kit.effects().walk;
        transform.pos = into_course(transform.pos + intent.walk * speed * dt);
    }
}

/// Swing: resolve each swing intent through `contact_target` on the positions after the walk.
pub fn swing(world: &mut World) {
    if !playing(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    let intents = world.resource::<Intents>().0.clone();
    for (index, _) in intents.iter().filter(|(_, intent)| intent.swing) {
        let snaps = snap_players(&world.view());
        let Some(me) = snaps.iter().find(|snap| snap.index == *index) else {
            continue;
        };
        let Some(contact) = contact_target(me, &snaps, tick) else {
            continue;
        };
        match contact {
            Contact::Club {
                who,
                daze_ticks,
                drops,
            } => club(world, me, who, tick + daze_ticks, drops),
            Contact::Strike { whose, lands_at } => strike(world, whose, lands_at, tick),
        }
        if let Some(entity) = player_entity(world, *index) {
            let wait = (SWING_COOLDOWN as f32 * me.effects.swing_cooldown).round() as u64;
            world.component_mut::<Player>(entity).swing_ready_at = tick + wait;
        }
    }
}

/// A club lands: the victim is dazed, loses a robbable item at the clubber's feet.
fn club(world: &mut World, me: &PlayerSnap, who: usize, dazed_until: u64, drops: Option<Item>) {
    let Some(victim) = player_entity(world, who) else {
        return;
    };
    let victim_pos = world.component::<Transform>(victim).pos;
    {
        let player = world.component_mut::<Player>(victim);
        player.dazed_until = dazed_until;
        player.extracting_since = None;
        if drops.is_some() {
            player.kit.remove_most_recent();
        }
    }
    if let Some(item) = drops {
        let at = into_course(victim_pos + (me.pos - victim_pos).normalize_or_zero());
        let pickup = world.spawn();
        world.insert(pickup, Transform::at(at));
        world.insert(pickup, Pickup(item));
    }
    if who != 0 {
        world.resource_mut::<Stats>().npc_dazed += 1;
    }
}

/// A strike lands: the ball takes a flight to where the strike sends it.
fn strike(world: &mut World, whose: usize, lands_at: Vec2, tick: u64) {
    let Some(entity) = ball_entity(world, whose) else {
        return;
    };
    let from = world.component::<Transform>(entity).pos;
    world.component_mut::<Ball>(entity).flight = Some(Flight {
        from,
        to: lands_at,
        start: tick,
        ticks: flight_ticks((lands_at - from).length()),
    });
    if whose != 0 {
        world.resource_mut::<Stats>().npc_balls_struck += 1;
    }
}

/// Shoot: a golfer within reach of their resting ball sends it to a scattered landing.
pub fn shoot(world: &mut World) {
    if !playing(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    let intents = world.resource::<Intents>().0.clone();
    for (index, want) in intents
        .iter()
        .filter_map(|(index, intent)| intent.shoot.map(|want| (*index, want)))
    {
        let snaps = snap_players(&world.view());
        let Some(me) = snaps.iter().find(|snap| snap.index == index) else {
            continue;
        };
        if me.dazed(tick)
            || me.extracting
            || !me.ball_resting
            || (me.ball - me.pos).length() > REACH_BALL
        {
            continue;
        }
        let aim = aim_point(me.ball, want, MAX_SHOT * me.effects.shot_reach);
        let to = scatter(aim, me.ball, world.resource_mut::<Rng>());
        let (Some(ball), Some(body)) = (ball_entity(world, index), player_entity(world, index))
        else {
            continue;
        };
        world.component_mut::<Ball>(ball).flight = Some(Flight {
            from: me.ball,
            to,
            start: tick,
            ticks: flight_ticks((to - me.ball).length()),
        });
        world.component_mut::<Player>(body).shots += 1;
    }
}

/// Fly: every ball in the air moves along its flight, and rests where it lands.
pub fn fly(world: &mut World) {
    if !playing(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        if let Some(flight) = ball.flight {
            transform.pos = flight.at(tick);
            if flight.done(tick) {
                ball.flight = None;
            }
        }
    }
}

/// Pickup: the nearest live, undazed golfer in reach takes the item; what it displaces
/// is put down a unit away so that it is not taken straight back.
pub fn pickup(world: &mut World) {
    if !playing(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    let mut takers: Vec<(Entity, usize, Vec2)> = world
        .query::<(&Transform, &Player)>()
        .filter(|(_, _, player)| tick >= player.dazed_until && !player.extracted)
        .map(|(entity, transform, player)| (entity, player.index, transform.pos))
        .collect();
    takers.sort_by_key(|(_, index, _)| *index);
    let items: Vec<(Entity, Vec2, Item)> = world
        .query::<(&Transform, &Pickup)>()
        .map(|(entity, transform, pickup)| (entity, transform.pos, pickup.0))
        .collect();
    for (entity, at, item) in items {
        let mut best: Option<(f32, Entity, Vec2)> = None;
        for (taker, _, pos) in &takers {
            let distance = (*pos - at).length();
            if distance <= PICKUP_RADIUS && best.is_none_or(|(d, _, _)| distance < d) {
                best = Some((distance, *taker, *pos));
            }
        }
        let Some((_, taker, pos)) = best else {
            continue;
        };
        let displaced = world.component_mut::<Player>(taker).kit.take(item);
        world.despawn(entity);
        if let Some(old) = displaced {
            let away = (at - pos).normalize_or_zero();
            let away = if away == Vec2::ZERO { Vec2::X } else { away };
            let put = world.spawn();
            world.insert(put, Transform::at(into_course(at + away)));
            world.insert(put, Pickup(old));
        }
    }
}

/// Expose: count each golfer's consecutive ticks outside the zone, and eliminate at grace.
pub fn expose(world: &mut World) {
    if !playing(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    let zone = zone_at(&world.resource::<Course>().schedule, tick);
    let balls: Vec<(usize, Vec2)> = world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| (ball.owner, transform.pos))
        .collect();
    let mut out: Vec<usize> = Vec::new();
    for (_, transform, player) in world.query_mut::<(&Transform, &mut Player)>() {
        let ball = balls
            .iter()
            .find(|(owner, _)| *owner == player.index)
            .map_or(transform.pos, |(_, pos)| *pos);
        let outside = !zone.contains(transform.pos) || !zone.contains(ball);
        player.exposure = if outside { player.exposure + 1 } else { 0 };
        if player.exposure == GRACE_TICKS {
            out.push(player.index);
        }
    }
    out.sort_unstable();
    for index in out {
        if let Some(body) = player_entity(world, index) {
            world.despawn(body);
        }
        if let Some(ball) = ball_entity(world, index) {
            world.despawn(ball);
        }
        world.resource_mut::<Eliminated>().0.push(index);
        if index != 0 {
            world.resource_mut::<Stats>().npc_zone_eliminations += 1;
        }
    }
}

/// Extract: body and ball on an open pad for `EXTRACT_TICKS` ticks, undisturbed.
pub fn extract(world: &mut World) {
    if !playing(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    let course = *world.resource::<Course>();
    let open = matches!(course.pad_state(tick), PadState::Open { .. });
    let intents = world.resource::<Intents>().0.clone();
    let balls: Vec<(usize, Vec2, bool)> = world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| (ball.owner, transform.pos, ball.flight.is_none()))
        .collect();
    let mut done: Vec<usize> = Vec::new();
    for (_, transform, player) in world.query_mut::<(&Transform, &mut Player)>() {
        let Some((_, ball, resting)) = balls.iter().find(|(owner, _, _)| *owner == player.index)
        else {
            continue;
        };
        let on_pad = open
            && *resting
            && (transform.pos - course.pad).length() <= PAD_RADIUS
            && (*ball - course.pad).length() <= PAD_RADIUS
            && tick >= player.dazed_until;
        match player.extracting_since {
            Some(since) if !on_pad => {
                let _ = since;
                player.extracting_since = None;
            }
            Some(since) => {
                if tick - since >= EXTRACT_TICKS {
                    player.extracted = true;
                    player.extracting_since = None;
                    done.push(player.index);
                }
            }
            None => {
                let wants = intents
                    .iter()
                    .any(|(index, intent)| *index == player.index && intent.extract);
                if wants && on_pad {
                    player.extracting_since = Some(tick);
                }
            }
        }
    }
    for index in done.into_iter().filter(|index| *index != 0) {
        if let Some(body) = player_entity(world, index) {
            world.despawn(body);
        }
        if let Some(ball) = ball_entity(world, index) {
            world.despawn(ball);
        }
        world.resource_mut::<Stats>().npc_extractions += 1;
    }
}

/// Settle: the first match-ending event for the human ends the match.
pub fn settle(world: &mut World) {
    if !playing(world) {
        return;
    }
    let tick = world.resource::<Time>().tick;
    let snaps = snap_players(&world.view());
    let eliminated = world.resource::<Eliminated>().0.clone();
    let hole = world.resource::<Course>().hole;
    if let Some((outcome, placing)) = settle_match(&snaps, &eliminated, hole, tick) {
        let kit = snaps
            .iter()
            .find(|snap| snap.index == 0)
            .map_or_else(Default::default, |snap| snap.kit);
        world.insert_resource(MatchState::Over {
            outcome,
            kept: kept_on(outcome, &kit),
            placing,
        });
    }
}
