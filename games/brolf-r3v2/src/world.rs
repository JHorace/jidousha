//! What the world holds — golfers, balls, pickups, the match — and the
//! `Startup` system that lays the course out; plus `snap`, the one read of the
//! world that the Update systems, the HUD and the checks all take.

use jidousha::prelude::*;

use crate::model::*;
use crate::rules::{BallSnap, GolferSnap};

/// A golfer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Golfer {
    /// 0 is You.
    pub idx: u8,
    /// How it plays.
    pub temperament: Temperament,
    /// Where it aims.
    pub aim: Radians,
    /// How hard.
    pub power: Power,
    /// Ticks of disable left.
    pub stun_left: u32,
    /// Ticks of swing cooldown left.
    pub cooldown: u32,
    /// Ticks in a row it (or its ball) has been outside the zone.
    pub outside_ticks: u32,
    /// Ticks it has held extract in the gate.
    pub extract_ticks: u32,
    /// What it holds.
    pub kit: Kit,
    /// Still in play.
    pub alive: bool,
}
impl Component for Golfer {}

/// A ball, and whose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    /// Its golfer's index.
    pub owner: u8,
    /// How it moves.
    pub vel: Vec2,
}
impl Component for Ball {}

/// A piece of equipment lying on the course.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pickup {
    /// What it is.
    pub item: Item,
}
impl Component for Pickup {}

/// The match: how it ended for You, if it has, and what happened.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Match {
    /// The result, once there is one.
    pub result: Option<Outcome>,
    /// One line per notable event, oldest first.
    pub log: Vec<String>,
}
impl Resource for Match {}

/// The angle from `from` to `to`.
pub fn bearing(from: Vec2, to: Vec2) -> Radians {
    let d = to - from;
    atan2(d.y, d.x)
}

/// Lay out the course: the camera, four golfers and their balls, six pickups.
pub fn set_up_course(world: &mut World) {
    world.insert_resource(Camera {
        center: Vec2::ZERO,
        height: HALF_H * 2.0,
        clear_color: palette::CLEAR,
        ..Camera::default()
    });
    world.insert_resource(Match::default());
    world.insert_resource(crate::systems::Intents::default());
    for (idx, (start, temperament, _)) in GOLFERS.into_iter().enumerate() {
        let golfer = world.spawn();
        world.insert(golfer, Transform::at(start));
        world.insert(
            golfer,
            Golfer {
                idx: idx as u8,
                temperament,
                aim: bearing(start + BALL_OFFSET, CUP),
                power: Power::Chip,
                stun_left: 0,
                cooldown: 0,
                outside_ticks: 0,
                extract_ticks: 0,
                kit: Kit::default(),
                alive: true,
            },
        );
        let ball = world.spawn();
        world.insert(ball, Transform::at(start + BALL_OFFSET));
        world.insert(
            ball,
            Ball {
                owner: idx as u8,
                vel: Vec2::ZERO,
            },
        );
    }
    let mut placed: Vec<Vec2> = Vec::new();
    for item in PICKUPS {
        let at = place_pickup(world.resource_mut::<Rng>(), &placed);
        placed.push(at);
        let pickup = world.spawn();
        world.insert(pickup, Transform::at(at));
        world.insert(pickup, Pickup { item });
    }
}

/// A pickup's spot: uniform in the course inset 3, away from everything placed.
fn place_pickup(rng: &mut Rng, placed: &[Vec2]) -> Vec2 {
    let min = COURSE.min + Vec2::splat(3.0);
    let size = COURSE.size() - Vec2::splat(6.0);
    let mut at = Vec2::ZERO;
    for _ in 0..64 {
        at = min + Vec2::new(rng.next_f32() * size.x, rng.next_f32() * size.y);
        let crowded = GOLFERS
            .iter()
            .map(|(start, _, _)| *start)
            .chain([CUP, GATE_CENTER])
            .chain(placed.iter().copied())
            .any(|other| other.distance(at) < 5.0);
        if !crowded {
            break;
        }
    }
    at
}

/// The world, read once: every golfer, ball and pickup, and the tick.
#[derive(Clone, Debug, PartialEq)]
pub struct Snap {
    /// The tick this was read on.
    pub now: u64,
    /// One tick's length.
    pub dt: Seconds,
    /// The four golfers, by index.
    pub golfers: Vec<GolferSnap>,
    /// Every ball still in play.
    pub balls: Vec<BallSnap>,
    /// Every pickup: where, and what.
    pub pickups: Vec<(Vec2, Item)>,
    /// The match's result, if over.
    pub result: Option<Outcome>,
}

impl Snap {
    /// Golfer `idx`.
    pub fn golfer(&self, idx: u8) -> Option<&GolferSnap> {
        self.golfers.iter().find(|golfer| golfer.idx == idx)
    }

    /// Golfer `idx`'s ball, if it is still in play.
    pub fn ball(&self, idx: u8) -> Option<&BallSnap> {
        self.balls.iter().find(|ball| ball.owner == idx)
    }
}

/// Read the world into a `Snap` — the one projection Update and Draw share.
pub fn snap(world: &WorldView<'_>) -> Snap {
    let now = world.find_resource::<Time>().map_or(0, |time| time.tick);
    let dt = world
        .find_resource::<Time>()
        .map_or(Seconds(1.0 / 60.0), |time| time.fixed_dt);
    let mut golfers: Vec<GolferSnap> = world
        .query::<(&Transform, &Golfer)>()
        .map(|(_, transform, golfer)| GolferSnap {
            idx: golfer.idx,
            pos: transform.pos,
            alive: golfer.alive,
            stun_left: golfer.stun_left,
            cooldown: golfer.cooldown,
            kit: golfer.kit,
            aim: golfer.aim,
            power: golfer.power,
            outside_ticks: golfer.outside_ticks,
            extract_ticks: golfer.extract_ticks,
        })
        .collect();
    golfers.sort_by_key(|golfer| golfer.idx);
    let mut balls: Vec<BallSnap> = world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| BallSnap {
            owner: ball.owner,
            pos: transform.pos,
            vel: ball.vel,
        })
        .collect();
    balls.sort_by_key(|ball| ball.owner);
    let mut pickups: Vec<(Vec2, Item)> = world
        .query::<(&Transform, &Pickup)>()
        .map(|(_, transform, pickup)| (transform.pos, pickup.item))
        .collect();
    // Deterministic, but not stable across despawns: order by place.
    pickups.sort_by(|a, b| a.0.y.total_cmp(&b.0.y).then(a.0.x.total_cmp(&b.0.x)));
    let result = world
        .find_resource::<Match>()
        .and_then(|game| game.result.clone());
    Snap {
        now,
        dt,
        golfers,
        balls,
        pickups,
        result,
    }
}
