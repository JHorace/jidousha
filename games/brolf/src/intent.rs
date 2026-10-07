//! What a golfer wants to do this tick: one `Intent` for the keyboard and for the NPCs.
//!
//! Key types: `Intent`. Key functions: `intent_from_input`, `npc_intent`, `safe_aim`.
//! Depends on: `contact`, `outcome`, `shots`, `world`, `zone`. Never depended on outside the game.
//! INVARIANT: a human and an NPC are subject to the same rules because both produce an
//! `Intent` and one set of apply systems acts on it.

use jidousha::prelude::*;

use crate::contact::{Contact, PlayerSnap, SWING_REACH, contact_target};
use crate::items::Item;
use crate::outcome::{HOLE_OPENS, PAD_RADIUS, PadState};
use crate::shots::{MAX_SHOT, REACH_BALL, aim_point, flight_ticks};
use crate::world::{Course, Persona};
use crate::zone::{landing_safe, next_stop, zone_at};

/// What one golfer decided to do this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Intent {
    /// A direction to walk, of length at most 1.
    pub walk: Vec2,
    /// A point to shoot toward, if shooting.
    pub shoot: Option<Vec2>,
    /// Whether to swing.
    pub swing: bool,
    /// Whether to start extracting.
    pub extract: bool,
}

impl Intent {
    /// Standing still, doing nothing.
    pub const NONE: Intent = Intent {
        walk: Vec2::ZERO,
        shoot: None,
        swing: false,
        extract: false,
    };
}

/// The pointer in world units.
pub fn pointer_world(input: &Input, camera: &Camera) -> Vec2 {
    camera.screen_to_world(input.pointer().screen)
}

/// WASD walks, a click shoots at the pointer, Space swings, E extracts.
pub fn intent_from_input(input: &Input, camera: &Camera) -> Intent {
    let walk = Vec2::new(
        f32::from(input.held(Key::D)) - f32::from(input.held(Key::A)),
        f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    )
    .clamp_length_max(1.0);
    let pointer = pointer_world(input, camera);
    Intent {
        walk,
        shoot: input
            .pointer()
            .just_pressed(PointerButton::Primary)
            .then_some(pointer),
        swing: input.just_pressed(Key::Space),
        extract: input.just_pressed(Key::E),
    }
}

/// A walk toward `to`, stopping short of it.
fn toward(from: Vec2, to: Vec2) -> Vec2 {
    let delta = to - from;
    if delta.length() < 0.1 {
        Vec2::ZERO
    } else {
        delta.clamp_length_max(1.0)
    }
}

/// Where `me` should hit their ball so it lands in the zone as it will be.
///
/// The hole once it is open, else a lay-up point 1.5 from it on the ball's side;
/// if that landing would be outside the zone by the time the ball has landed and
/// the golfer has had five seconds to reach it, the next zone's centre instead.
pub fn safe_aim(me: &PlayerSnap, course: &Course, tick: u64) -> Vec2 {
    let reach = MAX_SHOT * me.effects.shot_reach * 0.9;
    let from_hole = me.ball - course.hole;
    let target = if tick >= HOLE_OPENS {
        course.hole
    } else if from_hole.length() <= 1.5 {
        me.ball
    } else {
        course.hole + from_hole.normalize_or_zero() * 1.5
    };
    let candidate = aim_point(me.ball, target, reach);
    let arrive = tick + flight_ticks((candidate - me.ball).length()) + 300;
    if landing_safe(&course.schedule, candidate, arrive) {
        return candidate;
    }
    let centre = next_stop(&course.schedule, tick).map_or_else(
        || zone_at(&course.schedule, tick).center,
        |(_, zone)| zone.center,
    );
    aim_point(me.ball, centre, reach)
}

/// Walk to my ball; if in reach and it is at rest, shoot `aim`.
fn play_ball(me: &PlayerSnap, aim: Vec2, tick: u64) -> Intent {
    let min_shot = if tick >= HOLE_OPENS { 0.05 } else { 0.5 };
    let in_reach = (me.ball - me.pos).length() <= REACH_BALL;
    let wants_shot = me.ball_resting && (aim - me.ball).length() >= min_shot;
    Intent {
        walk: if in_reach && wants_shot {
            Vec2::ZERO
        } else {
            toward(me.pos, me.ball)
        },
        shoot: (in_reach && wants_shot).then_some(aim),
        ..Intent::NONE
    }
}

/// Whether `p` is on the pad.
fn on_pad(p: Vec2, pad: Vec2) -> bool {
    (p - pad).length() <= PAD_RADIUS
}

/// What an NPC does this tick: the priority list, as a free function.
pub fn npc_intent(
    me: &PlayerSnap,
    all: &[PlayerSnap],
    pickups: &[(Vec2, Item)],
    course: &Course,
    tick: u64,
) -> Intent {
    // 1. Dazed or already on the pad: stand.
    if me.dazed(tick) || me.extracting || me.extracted {
        return Intent::NONE;
    }
    let zone = zone_at(&course.schedule, tick);
    // 2. My ball or body is about to be outside the zone: get the ball somewhere safe.
    let ball_unsafe = me.ball_resting && !landing_safe(&course.schedule, me.ball, tick + 120);
    if ball_unsafe || !zone.contains(me.pos) {
        return play_ball(me, safe_aim(me, course, tick), tick);
    }
    // 3. A swing that pays: a club on someone holding something, a strike on a ball nearer
    // the hole than mine. A hunter swings at anything in reach: a daze costs a rival time.
    let hunter = me.persona == Persona::Hunter;
    match contact_target(me, all, tick) {
        Some(Contact::Club { who, .. })
            if hunter || all.iter().any(|p| p.index == who && p.kit.count() > 0) =>
        {
            return Intent {
                swing: true,
                ..Intent::NONE
            };
        }
        Some(Contact::Strike { whose, .. }) => {
            let theirs = all.iter().find(|p| p.index == whose);
            if hunter
                || theirs.is_some_and(|p| {
                    (p.ball - course.hole).length() < (me.ball - course.hole).length()
                })
            {
                return Intent {
                    swing: true,
                    ..Intent::NONE
                };
            }
        }
        _ => {}
    }
    // 4. Hunters close on a rival body within five units, or else on a rival's resting
    // ball its owner has left more than a swing's reach behind.
    if hunter {
        let nearest = |a: Vec2, b: Vec2| (a - me.pos).length().total_cmp(&(b - me.pos).length());
        let prey = all
            .iter()
            .filter(|p| p.index != me.index && !p.extracted)
            .filter(|p| (p.pos - me.pos).length() <= 5.0)
            .min_by(|a, b| nearest(a.pos, b.pos).then(a.index.cmp(&b.index)));
        if let Some(prey) = prey {
            return Intent {
                walk: toward(me.pos, prey.pos),
                ..Intent::NONE
            };
        }
        let exposed = all
            .iter()
            .filter(|p| p.index != me.index && !p.extracted && p.ball_resting)
            .filter(|p| (p.ball - p.pos).length() > SWING_REACH)
            .filter(|p| (p.ball - me.pos).length() <= 5.0)
            .min_by(|a, b| nearest(a.ball, b.ball).then(a.index.cmp(&b.index)));
        if let Some(prey) = exposed {
            return Intent {
                walk: toward(me.pos, prey.ball),
                ..Intent::NONE
            };
        }
    }
    // 4b. Equipment for a slot I have not filled.
    let sight = if me.persona == Persona::Extractor {
        6.0
    } else {
        3.0
    };
    let wanted = pickups
        .iter()
        .filter(|(_, item)| me.kit.in_slot(item.slot()).is_none())
        .filter(|(at, _)| (*at - me.pos).length() <= sight)
        .min_by(|a, b| (a.0 - me.pos).length().total_cmp(&(b.0 - me.pos).length()));
    if let Some((at, _)) = wanted {
        return Intent {
            walk: toward(me.pos, *at),
            ..Intent::NONE
        };
    }
    // 5. Extractors with something to keep head for the pad.
    if me.persona == Persona::Extractor
        && me.kit.count() >= 1
        && matches!(course.pad_state(tick), PadState::Open { .. })
    {
        let ball_on_pad = on_pad(me.ball, course.pad);
        if me.ball_resting && !ball_on_pad && (me.ball - me.pos).length() <= REACH_BALL {
            return Intent {
                shoot: Some(course.pad),
                ..Intent::NONE
            };
        }
        let goal = if ball_on_pad { course.pad } else { me.ball };
        return Intent {
            walk: toward(me.pos, goal),
            extract: ball_on_pad && on_pad(me.pos, course.pad) && me.ball_resting,
            ..Intent::NONE
        };
    }
    // 6. Otherwise play golf.
    play_ball(me, safe_aim(me, course, tick), tick)
}
