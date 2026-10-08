//! The simulation: the components, the match state, the read-only snapshot
//! every decision reads, and the Update systems in the order `main.rs`
//! registers them.
//!
//! Every system here applies a rule from `rules.rs` rather than deciding one,
//! so a check can ask the rule directly and then watch the system obey it.

use jidousha::prelude::*;

mod resolve;
pub use resolve::*;

use crate::npc;
use crate::rules::{
    BallView, Circle, Course, Fate, GOLFER_RADIUS, GolferView, Item, Landing, PAD_OPENS_TICK,
    PickupView, WALK_SPEED, ball_start, banked, course, effects, in_reach, inset, next_zone,
    seed_course, zone_at,
};
use crate::{TURF, VIEW_HEIGHT};

/// How a golfer means to turn its aim this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Aim {
    Keep,
    /// The player's arrows: -1 anticlockwise on screen, +1 clockwise.
    Rotate(i8),
    /// An NPC sets its aim outright.
    Set(Radians),
}

/// What a golfer means to do this tick — the one shape the keyboard and the
/// NPC policy both produce.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Intent {
    pub walk: Vec2,
    pub aim: Aim,
    pub space_held: bool,
    pub club: bool,
    pub take: bool,
    pub extract: bool,
}

impl Intent {
    /// Doing nothing.
    pub const IDLE: Intent = Intent {
        walk: Vec2::ZERO,
        aim: Aim::Keep,
        space_held: false,
        club: false,
        take: false,
        extract: false,
    };
}

/// A golfer: seat 0 is the player, 1..=3 the NPCs.
#[derive(Clone, Debug)]
pub struct Golfer {
    pub seat: usize,
    pub fate: Fate,
    pub points: u32,
    pub held: Vec<Item>,
    pub aim: Radians,
    pub charge: u32,
    pub cooldown: u64,
    pub out_ticks: u64,
    pub intent: Intent,
}
impl Component for Golfer {}

/// A ball in flight along a line, placed by the closed-form roll each tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Roll {
    pub from: Vec2,
    pub dir: Vec2,
    pub speed: f32,
    pub started: u64,
    pub landing: Landing,
}

/// A golfer's ball.
#[derive(Clone, Copy, Debug)]
pub struct Ball {
    pub owner: usize,
    pub roll: Option<Roll>,
    /// The tick it last came to rest on, so `settle_balls` sees it once.
    pub rested_at: u64,
}
impl Component for Ball {}

/// A piece of equipment lying on the course.
#[derive(Clone, Copy, Debug)]
pub struct Pickup {
    pub item: Item,
}
impl Component for Pickup {}

/// Whether the match is on, and who won it once it is not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Match {
    Live,
    Over { at: u64, winner: Option<usize> },
}
impl Resource for Match {}

/// The read-only projection every decision reads: the panel, the NPCs, the
/// players and the gates.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub tick: u64,
    pub dt: Seconds,
    pub course: Course,
    pub zone: Circle,
    pub next: Option<Circle>,
    pub pad_open: bool,
    /// Every golfer, by seat.
    pub golfers: Vec<GolferView>,
    /// Every ball, by owner.
    pub balls: Vec<BallView>,
    pub pickups: Vec<PickupView>,
    pub over: Option<(u64, Option<usize>)>,
}

impl Snapshot {
    /// The golfer in `seat`.
    pub fn golfer(&self, seat: usize) -> &GolferView {
        &self.golfers[seat]
    }

    /// The ball `seat` owns.
    pub fn ball(&self, seat: usize) -> &BallView {
        &self.balls[seat]
    }

    /// What `seat` banks if the match stopped now.
    pub fn banked(&self, seat: usize) -> u32 {
        let golfer = self.golfer(seat);
        banked(golfer.fate, golfer.points, &golfer.held)
    }

    /// What `seat` can act on.
    pub fn reach(&self, seat: usize) -> crate::rules::Reach {
        in_reach(self.golfer(seat), &self.golfers, &self.balls, &self.pickups)
    }
}

/// The one reader both phases call: the world as a `Snapshot`, or `None`
/// before `Startup` has run.
pub fn read_snapshot(world: &WorldView<'_>) -> Option<Snapshot> {
    let course = world.find_resource::<Course>()?.clone();
    let time = world.resource::<Time>();
    let tick = time.tick;
    let mut golfers: Vec<GolferView> = world
        .query::<(&Golfer, &Transform)>()
        .map(|(_, g, t)| GolferView {
            seat: g.seat,
            pos: t.pos,
            fate: g.fate,
            held: g.held.clone(),
            points: g.points,
            out_ticks: g.out_ticks,
            cooldown: g.cooldown,
            aim: g.aim,
            charge: g.charge,
        })
        .collect();
    golfers.sort_by_key(|g| g.seat);
    let mut balls: Vec<BallView> = world
        .query::<(&Ball, &Transform)>()
        .map(|(_, b, t)| BallView {
            owner: b.owner,
            pos: t.pos,
            at_rest: b.roll.is_none(),
            landing: b.roll.map(|roll| roll.landing),
        })
        .collect();
    balls.sort_by_key(|b| b.owner);
    let pickups = world
        .query::<(&Pickup, &Transform)>()
        .map(|(_, p, t)| PickupView {
            pos: t.pos,
            item: p.item,
        })
        .collect();
    let over = match world.find_resource::<Match>() {
        Some(Match::Over { at, winner }) => Some((*at, *winner)),
        _ => None,
    };
    Some(Snapshot {
        tick,
        dt: time.fixed_dt,
        zone: zone_at(&course, tick),
        next: next_zone(&course, tick),
        pad_open: tick >= PAD_OPENS_TICK,
        course,
        golfers,
        balls,
        pickups,
        over,
    })
}

pub(crate) fn live(world: &World) -> bool {
    matches!(world.find_resource::<Match>(), Some(Match::Live))
}

/// Startup: the camera, the seeded course, four golfers, their balls, six pickups.
pub fn set_the_course(world: &mut World) {
    world.insert_resource(Camera {
        center: Vec2::ZERO,
        height: VIEW_HEIGHT,
        clear_color: TURF,
        ..Camera::default()
    });
    let (course, pickups) = seed_course(world.resource_mut::<Rng>());
    for (seat, at) in course.seats.iter().enumerate() {
        let golfer = world.spawn();
        world.insert(golfer, Transform::at(*at));
        world.insert(
            golfer,
            Golfer {
                seat,
                fate: Fate::Playing,
                points: 0,
                held: Vec::new(),
                aim: crate::rules::atan_to(ball_start(*at), crate::rules::COURSE_CENTER),
                charge: 0,
                cooldown: 0,
                out_ticks: 0,
                intent: Intent::IDLE,
            },
        );
        let ball = world.spawn();
        world.insert(ball, Transform::at(ball_start(*at)));
        world.insert(
            ball,
            Ball {
                owner: seat,
                roll: None,
                rested_at: 0,
            },
        );
    }
    for (at, item) in pickups {
        spawn_pickup(world, at, item);
    }
    world.insert_resource(course);
    world.insert_resource(Match::Live);
}

/// A pickup lying at `at`.
pub fn spawn_pickup(world: &mut World, at: Vec2, item: Item) {
    let pickup = world.spawn();
    world.insert(pickup, Transform::at(at));
    world.insert(pickup, Pickup { item });
}

/// The player's intent, from the keyboard: WASD walk, arrows aim, Space
/// charges and releases, F clubs, E takes, X extracts.
pub fn read_player_intent(world: &mut World) {
    if !live(world) {
        return;
    }
    let intent = match world.find_resource::<Input>() {
        None => Intent::IDLE,
        Some(input) => {
            let axis =
                |minus: Key, plus: Key| f32::from(input.held(plus)) - f32::from(input.held(minus));
            let turn = axis(Key::ArrowLeft, Key::ArrowRight);
            Intent {
                walk: Vec2::new(axis(Key::A, Key::D), axis(Key::W, Key::S)).normalize_or_zero(),
                aim: if turn == 0.0 {
                    Aim::Keep
                } else {
                    Aim::Rotate(turn as i8)
                },
                space_held: input.held(Key::Space),
                club: input.just_pressed(Key::F),
                take: input.just_pressed(Key::E),
                extract: input.just_pressed(Key::X),
            }
        }
    };
    for (_, golfer) in world.query_mut::<&mut Golfer>() {
        if golfer.seat == 0 {
            golfer.intent = intent;
        }
    }
}

/// Seats 1..=3 decide through `npc::decide`, all from one snapshot.
pub fn decide_npcs(world: &mut World) {
    if !live(world) {
        return;
    }
    let Some(snapshot) = read_snapshot(&world.view()) else {
        return;
    };
    let intents: Vec<Intent> = (0..4)
        .map(|seat| {
            if seat == 0 {
                Intent::IDLE
            } else {
                npc::decide(&snapshot, seat)
            }
        })
        .collect();
    for (_, golfer) in world.query_mut::<&mut Golfer>() {
        if golfer.seat != 0 {
            golfer.intent = intents[golfer.seat];
        }
    }
}

/// Stuns wear off, cooldowns count down, golfers walk, aims turn, charges build.
pub fn walk(world: &mut World) {
    if !live(world) {
        return;
    }
    let time = *world.resource::<Time>();
    let dt = time.fixed_dt.as_f32();
    let fence = inset(course(), GOLFER_RADIUS);
    for (_, golfer, transform) in world.query_mut::<(&mut Golfer, &mut Transform)>() {
        if let Fate::Stunned { until } = golfer.fate
            && time.tick >= until
        {
            golfer.fate = Fate::Playing;
        }
        golfer.cooldown = golfer.cooldown.saturating_sub(1);
        if golfer.fate != Fate::Playing {
            continue;
        }
        let intent = golfer.intent;
        let step = intent.walk * WALK_SPEED * effects(&golfer.held).walk_scale * dt;
        transform.pos = (transform.pos + step).clamp(fence.min, fence.max);
        match intent.aim {
            Aim::Keep => {}
            Aim::Rotate(sign) => {
                golfer.aim = Radians(
                    golfer.aim.as_f32() + f32::from(sign) * crate::rules::AIM_RATE.as_f32() * dt,
                );
            }
            Aim::Set(angle) => golfer.aim = angle,
        }
        if intent.space_held {
            golfer.charge = (golfer.charge + 1).min(crate::rules::CHARGE_TICKS);
        }
    }
}
