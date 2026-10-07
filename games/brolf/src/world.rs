//! The world's components and resources, and the one function that deals a match.
//!
//! Key types: `Player`, `Ball`, `Pickup`, `Course`, `MatchState`, `Stats`.
//! Key functions: `reset_match`.
//! Depends on: `items`, `intent`, `outcome`, `shots`, `zone`. Never depended on outside the game.
//! INVARIANT: `reset_match` draws from `Rng` in exactly this order: start phase, hole
//! angle, hole length, pickup phase, then the zone schedule's jitter; a seed names one course.

use jidousha::prelude::*;

use crate::intent::Intent;
use crate::items::{Item, Kit};
use crate::outcome::{Outcome, PAD_OPENS, PadState, Placing, pad_state};
use crate::shots::Flight;
use crate::zone::{Schedule, pad_closes_at};

/// How a golfer decides what to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Persona {
    /// The person at the keyboard.
    Human,
    /// Plays their ball and keeps it in the zone.
    Golfer,
    /// Goes after rivals who come within five units.
    Hunter,
    /// Gathers equipment and leaves through the pad.
    Extractor,
}

/// One golfer's body.
#[derive(Clone, Copy, Debug)]
pub struct Player {
    /// 0 is the human; 1..=5 are the rivals.
    pub index: usize,
    /// How they decide.
    pub persona: Persona,
    /// What they hold.
    pub kit: Kit,
    /// The tick their daze ends; they are dazed while the tick is less than this.
    pub dazed_until: u64,
    /// The first tick they may swing again.
    pub swing_ready_at: u64,
    /// How many consecutive ticks their body or ball has been outside the zone.
    pub exposure: u64,
    /// The tick they began standing on the pad, if they are.
    pub extracting_since: Option<u64>,
    /// How many shots they have taken.
    pub shots: u32,
    /// Whether they have left the match through the pad.
    pub extracted: bool,
}
impl Component for Player {}

/// One golfer's ball.
#[derive(Clone, Copy, Debug)]
pub struct Ball {
    /// Whose it is.
    pub owner: usize,
    /// Its flight, if it is in the air.
    pub flight: Option<Flight>,
}
impl Component for Ball {}

/// A piece of equipment lying on the course.
#[derive(Clone, Copy, Debug)]
pub struct Pickup(pub Item);
impl Component for Pickup {}

/// The course: where the hole and pad are, and where the zone goes.
#[derive(Clone, Copy, Debug)]
pub struct Course {
    /// The hole.
    pub hole: Vec2,
    /// The extraction pad's centre.
    pub pad: Vec2,
    /// The zone's schedule.
    pub schedule: Schedule,
    /// The tick the zone leaves the pad, computed once.
    pub pad_closes: Option<u64>,
}
impl Resource for Course {}

impl Course {
    /// The pad's state at `tick`.
    pub fn pad_state(&self, tick: u64) -> PadState {
        pad_state(self.pad_closes, tick)
    }
}

/// Whether the match is on, or how it ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchState {
    /// Play goes on.
    Playing,
    /// The match is over; the result screen shows this.
    Over {
        /// How it ended.
        outcome: Outcome,
        /// What the human keeps.
        kept: Kit,
        /// Where the human placed.
        placing: Placing,
    },
}
impl Resource for MatchState {}

/// Who the zone has taken, in order.
#[derive(Clone, Debug, Default)]
pub struct Eliminated(pub Vec<usize>);
impl Resource for Eliminated {}

/// This tick's decisions, one per golfer in play, written by `decide`.
#[derive(Clone, Debug, Default)]
pub struct Intents(pub Vec<(usize, Intent)>);
impl Resource for Intents {}

/// The pointer in world units, updated each tick so Draw can read it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pointer(pub Vec2);
impl Resource for Pointer {}

/// What has happened to the rivals, counted as it happens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Rivals dazed by a club.
    pub npc_dazed: u32,
    /// Rival balls struck.
    pub npc_balls_struck: u32,
    /// Rivals who extracted.
    pub npc_extractions: u32,
    /// Rivals the zone took.
    pub npc_zone_eliminations: u32,
}
impl Resource for Stats {}

/// The rivals' personas, for players 1 to 5.
const PERSONAS: [Persona; 5] = [
    Persona::Golfer,
    Persona::Hunter,
    Persona::Extractor,
    Persona::Golfer,
    Persona::Hunter,
];

/// The point `length` from the origin at `angle`.
fn polar(angle: f32, length: f32) -> Vec2 {
    rotate(Vec2::X * length, Radians(angle))
}

/// Deal a match from the world's `Rng`: six golfers, a hole, a pad, eight pickups.
pub fn reset_match(world: &mut World) {
    let (phase, hole, phase2, schedule) = {
        let rng = world.resource_mut::<Rng>();
        let phase = rng.next_f32() * Radians::TAU.as_f32();
        let hole_angle = rng.next_f32() * Radians::TAU.as_f32();
        let hole_length = 1.5 + 1.5 * rng.next_f32();
        let hole = polar(hole_angle, hole_length);
        let phase2 = rng.next_f32() * Radians::TAU.as_f32();
        let schedule = Schedule::seeded(rng, hole);
        (phase, hole, phase2, schedule)
    };
    let pad = hole + hole.normalize_or_zero() * 2.0;
    let pad_closes = pad_closes_at(&schedule, pad, PAD_OPENS);
    world.insert_resource(Course {
        hole,
        pad,
        schedule,
        pad_closes,
    });
    world.insert_resource(MatchState::Playing);
    world.insert_resource(Eliminated::default());
    world.insert_resource(Intents::default());
    world.insert_resource(Pointer::default());
    world.insert_resource(Stats::default());

    for index in 0..6 {
        let theta = phase + Radians::from_degrees(60.0 * index as f32).as_f32();
        let (sin, cos) = sin_cos(Radians(theta));
        let body = Vec2::new(12.0 * cos, 6.2 * sin);
        let ball = body + (-body).normalize_or_zero() * 0.8;
        let persona = if index == 0 {
            Persona::Human
        } else {
            PERSONAS[index - 1]
        };
        let player = world.spawn();
        world.insert(player, Transform::at(body));
        world.insert(
            player,
            Player {
                index,
                persona,
                kit: Kit::default(),
                dazed_until: 0,
                swing_ready_at: 0,
                exposure: 0,
                extracting_since: None,
                shots: 0,
                extracted: false,
            },
        );
        let ball_entity = world.spawn();
        world.insert(ball_entity, Transform::at(ball));
        world.insert(
            ball_entity,
            Ball {
                owner: index,
                flight: None,
            },
        );
    }
    for k in 0..8 {
        let phi = phase2 + Radians::from_degrees(45.0 * k as f32).as_f32();
        let (sin, cos) = sin_cos(Radians(phi));
        let pickup = world.spawn();
        world.insert(pickup, Transform::at(Vec2::new(7.0 * cos, 4.2 * sin)));
        world.insert(pickup, Pickup(Item::ALL[k % 4]));
    }
}
