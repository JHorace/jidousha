//! The `Update` phase, in the one order `register` adds it and a gate asserts:
//! intents (keyboard, then NPCs), contacts, walks, shots/takes/extracts, balls,
//! the zone's grace, the end of the match.
//!
//! Contacts resolve before walks, so the cue drawn last frame is what C hits
//! this tick; balls move before the zone judges, so the ball's position on a
//! tick counts on that tick (DESIGN.md, "Decisions already made"). Once the
//! match has a result every system returns at once.

use jidousha::prelude::*;

use crate::model::*;
use crate::npc::{Intent, npc_decide, temperament_goal};
use crate::rules::*;
use crate::world::{Ball, Golfer, Match, Pickup, snap};

/// This tick's intent for each golfer, by index.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Intents(pub [Intent; 4]);
impl Resource for Intents {}

/// Whether the match is over.
fn over(world: &World) -> bool {
    world.resource::<Match>().result.is_some()
}

/// The entity of golfer `idx`.
fn golfer_entity(world: &World, idx: u8) -> Option<Entity> {
    world
        .query::<&Golfer>()
        .find(|(_, golfer)| golfer.idx == idx)
        .map(|(entity, _)| entity)
}

/// The entity of golfer `idx`'s ball.
fn ball_entity(world: &World, idx: u8) -> Option<Entity> {
    world
        .query::<&Ball>()
        .find(|(_, ball)| ball.owner == idx)
        .map(|(entity, _)| entity)
}

/// The keyboard, read into golfer 0's intent.
pub fn player_intent(world: &mut World) {
    if over(world) {
        return;
    }
    let intent = match world.find_resource::<Input>() {
        None => Intent::default(),
        Some(input) => {
            let axis =
                |minus: Key, plus: Key| f32::from(input.held(plus)) - f32::from(input.held(minus));
            let step = |down: Key, up: Key| {
                i8::from(input.just_pressed(up)) - i8::from(input.just_pressed(down))
            };
            Intent {
                walk: Vec2::new(axis(Key::A, Key::D), axis(Key::W, Key::S)).normalize_or_zero(),
                turn: i8::from(input.held(Key::ArrowRight)) - i8::from(input.held(Key::ArrowLeft)),
                power_step: step(Key::ArrowDown, Key::ArrowUp),
                shoot: input.just_pressed(Key::Space),
                contact: input.just_pressed(Key::C),
                take: input.just_pressed(Key::F),
                extract: input.held(Key::E),
                aim_to: None,
                power_to: None,
            }
        }
    };
    world.resource_mut::<Intents>().0[0] = intent;
}

/// Every NPC's decision, from one read of the world.
pub fn npc_intents(world: &mut World) {
    if over(world) {
        return;
    }
    let view = snap(&world.view());
    let temperaments: Vec<(u8, Temperament)> = world
        .query::<&Golfer>()
        .map(|(_, golfer)| (golfer.idx, golfer.temperament))
        .collect();
    let mut intents = world.resource::<Intents>().0;
    for (idx, temperament) in temperaments {
        if temperament == Temperament::Player {
            continue;
        }
        let Some(me) = view.golfer(idx) else { continue };
        let goal = temperament_goal(temperament, me, &view);
        intents[usize::from(idx)] = npc_decide(me, &view, goal, temperament);
    }
    world.resource_mut::<Intents>().0 = intents;
}

/// Count the disable and cooldown timers down, then resolve every swing.
pub fn apply_contacts(world: &mut World) {
    if over(world) {
        return;
    }
    for (_, golfer) in world.query_mut::<&mut Golfer>() {
        golfer.stun_left = golfer.stun_left.saturating_sub(1);
        golfer.cooldown = golfer.cooldown.saturating_sub(1);
    }
    let intents = world.resource::<Intents>().0;
    for idx in 0..4u8 {
        if !intents[usize::from(idx)].contact {
            continue;
        }
        let view = snap(&world.view());
        let Some(me) = view.golfer(idx).copied() else {
            continue;
        };
        if !me.alive || me.stun_left > 0 || me.cooldown > 0 {
            continue;
        }
        let Some(contact) = contact_target(&me, &view.golfers, &view.balls, effects(&me.kit))
        else {
            continue;
        };
        let (Contact::Club(target) | Contact::Strike(target)) = contact;
        let Some(struck) = view.golfer(target).copied() else {
            continue;
        };
        let result = resolve_contact(contact, me.aim, effects(&struck.kit));
        if let Some(entity) = golfer_entity(world, idx) {
            world.component_mut::<Golfer>(entity).cooldown = SWING_COOLDOWN;
        }
        match contact {
            Contact::Club(_) => club(world, target, result),
            Contact::Strike(_) => {
                if let (Some(vel), Some(ball)) = (result.ball_vel, ball_entity(world, target)) {
                    world.component_mut::<Ball>(ball).vel = vel;
                }
            }
        }
        let line = format!(
            "{} swung at {}",
            NAMES[usize::from(idx)],
            NAMES[usize::from(target)]
        );
        world.resource_mut::<Match>().log.push(line);
    }
}

/// A club landing on golfer `target`: disable it, knock its best item loose.
fn club(world: &mut World, target: u8, result: ContactResult) {
    let Some(entity) = golfer_entity(world, target) else {
        return;
    };
    let at = world.component::<Transform>(entity).pos;
    let golfer = world.component_mut::<Golfer>(entity);
    golfer.stun_left = result.stun_ticks;
    let dropped = if result.drops {
        ranked(&golfer.kit).first().copied()
    } else {
        None
    };
    if let Some(item) = dropped {
        golfer.kit.take(item.category());
        let pickup = world.spawn();
        world.insert(pickup, Transform::at(at));
        world.insert(pickup, Pickup { item });
    }
}

/// Walk, turn the aim and step the power, unless disabled.
pub fn apply_walks(world: &mut World) {
    if over(world) {
        return;
    }
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    let intents = world.resource::<Intents>().0;
    let min = COURSE.min + Vec2::splat(GOLFER_RADIUS);
    let max = COURSE.max - Vec2::splat(GOLFER_RADIUS);
    for (_, transform, golfer) in world.query_mut::<(&mut Transform, &mut Golfer)>() {
        if !golfer.alive || golfer.stun_left > 0 {
            continue;
        }
        let intent = intents[usize::from(golfer.idx)];
        if let Some(aim) = intent.aim_to {
            golfer.aim = aim;
        }
        if let Some(power) = intent.power_to {
            golfer.power = power;
        }
        let turned = golfer.aim.as_f32() + f32::from(intent.turn) * AIM_RATE.as_f32() * dt;
        golfer.aim = Radians(turned.rem_euclid(Radians::TAU.as_f32()));
        golfer.power = golfer.power.stepped(intent.power_step);
        let step = intent.walk.normalize_or_zero() * WALK_SPEED * dt;
        transform.pos = (transform.pos + step).clamp(min, max);
    }
}

/// Shots, pickups and extraction, in golfer order.
pub fn apply_shots_takes_extracts(world: &mut World) {
    if over(world) {
        return;
    }
    let intents = world.resource::<Intents>().0;
    for idx in 0..4u8 {
        let intent = intents[usize::from(idx)];
        let view = snap(&world.view());
        let Some(me) = view.golfer(idx).copied() else {
            continue;
        };
        if !me.alive || me.stun_left > 0 {
            continue;
        }
        let ball = view.ball(idx).copied();
        if intent.shoot
            && me.cooldown == 0
            && let Some(ball) = ball
            && ball.vel == Vec2::ZERO
            && me.pos.distance(ball.pos) <= SHOT_REACH
            && let Some(entity) = ball_entity(world, idx)
        {
            world.component_mut::<Ball>(entity).vel =
                shot_velocity(me.aim, me.power, effects(&me.kit));
        }
        if intent.take {
            take(world, idx, me.pos);
        }
        extract(world, idx, &me, ball, intent.extract);
    }
}

/// Take the nearest pickup within reach; drop what it replaces where I stand.
fn take(world: &mut World, idx: u8, at: Vec2) {
    let nearest = world
        .query::<(&Transform, &Pickup)>()
        .filter(|(_, transform, _)| transform.pos.distance(at) <= TAKE_RANGE)
        .min_by(|a, b| a.1.pos.distance(at).total_cmp(&b.1.pos.distance(at)))
        .map(|(entity, _, pickup)| (entity, pickup.item));
    let (Some((pickup, item)), Some(golfer)) = (nearest, golfer_entity(world, idx)) else {
        return;
    };
    world.despawn(pickup);
    let replaced = world.component_mut::<Golfer>(golfer).kit.put(item);
    if let Some(old) = replaced {
        let dropped = world.spawn();
        world.insert(dropped, Transform::at(at));
        world.insert(dropped, Pickup { item: old });
    }
}

/// Count a held extract in the gate; at `EXTRACT_HOLD` the golfer leaves play.
fn extract(world: &mut World, idx: u8, me: &GolferSnap, ball: Option<BallSnap>, held: bool) {
    let gate = gate();
    let in_gate = gate.contains(me.pos) && ball.is_some_and(|ball| gate.contains(ball.pos));
    let Some(entity) = golfer_entity(world, idx) else {
        return;
    };
    let golfer = world.component_mut::<Golfer>(entity);
    golfer.extract_ticks = if in_gate && held {
        golfer.extract_ticks + 1
    } else {
        0
    };
    if golfer.extract_ticks < EXTRACT_HOLD {
        return;
    }
    golfer.alive = false;
    let kit = golfer.kit;
    if let Some(ball) = ball_entity(world, idx) {
        world.despawn(ball);
    }
    let game = world.resource_mut::<Match>();
    game.log
        .push(format!("{} extracted", NAMES[usize::from(idx)]));
    if idx == 0 {
        game.result = Some(outcome_of(EndKind::Extracted, &kit));
    }
}

/// Roll every ball one tick; a sunk ball ends the match.
pub fn move_balls(world: &mut World) {
    if over(world) {
        return;
    }
    let dt = world.resource::<Time>().fixed_dt;
    let open = cup_open(world.resource::<Time>().tick);
    let mut sunk = None;
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        if ball.vel == Vec2::ZERO {
            continue;
        }
        let step = ball_step(transform.pos, ball.vel, dt);
        transform.pos = step.pos;
        ball.vel = step.vel;
        if step.sunk && open && sunk.is_none() {
            ball.vel = Vec2::ZERO;
            sunk = Some(ball.owner);
        }
    }
    let Some(owner) = sunk else { return };
    let kit =
        golfer_entity(world, 0).map_or(Kit::default(), |you| world.component::<Golfer>(you).kit);
    let kind = if owner == 0 {
        EndKind::Holed
    } else {
        EndKind::Lost { by: owner }
    };
    let game = world.resource_mut::<Match>();
    game.log
        .push(format!("{} holed out", NAMES[usize::from(owner)]));
    game.result = Some(outcome_of(kind, &kit));
}

/// Count every golfer's time outside the zone; at `GRACE_TICKS` it is out.
pub fn zone_grace(world: &mut World) {
    if over(world) {
        return;
    }
    let now = world.resource::<Time>().tick;
    let zone = zone_at(now);
    let view = snap(&world.view());
    let mut eliminated = Vec::new();
    for (_, transform, golfer) in world.query_mut::<(&Transform, &mut Golfer)>() {
        if !golfer.alive {
            continue;
        }
        let ball_out = view
            .ball(golfer.idx)
            .is_some_and(|ball| !inside_zone(zone, ball.pos));
        if inside_zone(zone, transform.pos) && !ball_out {
            golfer.outside_ticks = 0;
            continue;
        }
        golfer.outside_ticks += 1;
        if golfer.outside_ticks >= GRACE_TICKS {
            golfer.alive = false;
            eliminated.push(golfer.idx);
        }
    }
    for idx in eliminated {
        if let Some(ball) = ball_entity(world, idx) {
            world.despawn(ball);
        }
        let game = world.resource_mut::<Match>();
        game.log.push(format!(
            "{} eliminated by the zone",
            NAMES[usize::from(idx)]
        ));
        if idx == 0 {
            game.result = Some(outcome_of(EndKind::Eliminated, &Kit::default()));
        }
    }
}

/// The last one standing wins; the course closing ends it for everyone.
pub fn end_match(world: &mut World) {
    if over(world) {
        return;
    }
    let view = snap(&world.view());
    let you = view.golfer(0).copied();
    let rivals = view
        .golfers
        .iter()
        .filter(|g| g.idx != 0 && g.alive)
        .count();
    let result = match you {
        Some(you) if you.alive && rivals == 0 => Some(outcome_of(EndKind::LastStanding, &you.kit)),
        _ if view.now >= MATCH_END_TICK => Some(outcome_of(EndKind::Eliminated, &Kit::default())),
        _ => None,
    };
    if result.is_some() {
        world.resource_mut::<Match>().result = result;
    }
}
