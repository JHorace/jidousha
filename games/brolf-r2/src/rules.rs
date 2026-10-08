//! The rules of brolf as free functions: the zone's schedule, the course's
//! seeding, the roll, the equipment table, reach and the outcome.
//!
//! No `World` appears in this file. The preview, the NPCs, the sim and the
//! verify gates all call the same functions, so a ring drawn on screen, a
//! landing disc's colour, a grace countdown and an elimination cannot
//! disagree — each decision row of the spec names its one function here:
//! `zone_at` (row 1), `in_reach` (row 2), `effects` (row 3), `banked` (row 4).

use jidousha::prelude::*;

/// The world rectangle balls and golfers live in: the fence.
pub const COURSE_MIN: Vec2 = Vec2::new(-15.5, -7.5);
/// The fence's bottom-right corner.
pub const COURSE_MAX: Vec2 = Vec2::new(6.5, 8.5);
/// The course's centre, and zone 0's.
pub const COURSE_CENTER: Vec2 = Vec2::new(-4.5, 0.5);

/// The fence as a `Rect` — built where used, since `Rect` has no `const fn`.
pub fn course() -> Rect {
    Rect {
        min: COURSE_MIN,
        max: COURSE_MAX,
    }
}

/// `rect` shrunk by `by` on every side.
pub fn inset(rect: Rect, by: f32) -> Rect {
    Rect {
        min: rect.min + Vec2::splat(by),
        max: rect.max - Vec2::splat(by),
    }
}

// --- the zone ---------------------------------------------------------------

/// Each zone's radius, phase by phase; the last is the closed zone.
pub const ZONE_RADII: [f32; 5] = [7.8, 5.0, 3.2, 1.8, 0.0];
/// How long each phase's zone holds before it shrinks, in ticks.
pub const HOLD_TICKS: [u64; 4] = [1200, 900, 720, 600];
/// How long each phase's shrink into the next zone takes, in ticks.
pub const SHRINK_TICKS: [u64; 4] = [720, 600, 480, 360];
/// The first tick the extraction pad is open: phase 1's first tick.
pub const PAD_OPENS_TICK: u64 = 1921;
/// The phase from which every NPC heads for the pad.
pub const EXTRACT_PHASE: usize = 2;
/// How long a golfer may have itself or its ball outside the zone, in ticks.
pub const GRACE_TICKS: u64 = 360;

/// A circle: a zone, the pad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Circle {
    /// Whether `point` is inside, the rim included.
    pub fn contains(self, point: Vec2) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }
}

/// Where the match stands on a tick: which phase, and what it is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// The zone holds, for this many more ticks.
    Hold(u64),
    /// The zone shrinks, arriving at the next one in this many ticks.
    Shrink(u64),
    /// The zone has closed to nothing.
    Closed,
}

/// The phase `tick` falls in, and what it is doing. Phase 0 starts on tick 1.
pub fn phase_of(tick: u64) -> (usize, Phase) {
    let mut start = 1;
    for k in 0..4 {
        let shrink_start = start + HOLD_TICKS[k];
        let end = shrink_start + SHRINK_TICKS[k];
        if tick < shrink_start {
            return (k, Phase::Hold(shrink_start - tick.max(start)));
        }
        if tick < end {
            return (k, Phase::Shrink(end - tick));
        }
        start = end;
    }
    (4, Phase::Closed)
}

/// The course: its five zones, its cups, its pad and its four seats.
#[derive(Clone, Debug, PartialEq)]
pub struct Course {
    pub zones: [Circle; 5],
    pub cups: [Vec2; 3],
    pub pad: Circle,
    pub seats: [Vec2; 4],
}
impl Resource for Course {}

/// The zone on `tick` — the one zone-membership-at-time function (decision
/// row 1). The rings, the landing disc, the grace countdown and elimination
/// all call it.
pub fn zone_at(course: &Course, tick: u64) -> Circle {
    match phase_of(tick) {
        (k, Phase::Hold(_)) => course.zones[k],
        (k, Phase::Shrink(left)) => {
            let span = SHRINK_TICKS[k];
            // The shrink's first tick is one step in and its last is the next
            // zone exactly, not a lerp's rounding of it.
            let into = span + 1 - left;
            let (from, to) = (course.zones[k], course.zones[k + 1]);
            if into >= span {
                return to;
            }
            let t = into as f32 / span as f32;
            Circle {
                center: from.center.lerp(to.center, t),
                radius: from.radius + (to.radius - from.radius) * t,
            }
        }
        (_, Phase::Closed) => course.zones[4],
    }
}

/// The zone the current one is heading for, or `None` once it has closed.
pub fn next_zone(course: &Course, tick: u64) -> Option<Circle> {
    match phase_of(tick) {
        (_, Phase::Closed) => None,
        (k, _) => Some(course.zones[k + 1]),
    }
}

/// The status panel's second row: what the zone is doing and for how long.
pub fn phase_line(tick: u64) -> String {
    match phase_of(tick).1 {
        Phase::Hold(left) => format!("holds {:.1}s", left as f32 / 60.0),
        Phase::Shrink(left) => format!("shrinking, {:.1}s left", left as f32 / 60.0),
        Phase::Closed => "zone closed".to_owned(),
    }
}

// --- seeding ----------------------------------------------------------------

/// A point `rho` from the origin at `theta`, through the engine's `sin_cos`.
pub fn polar(rho: f32, theta: Radians) -> Vec2 {
    let (sin, cos) = sin_cos(theta);
    Vec2::new(cos * rho, sin * rho)
}

/// The angle of the line from `from` to `to`.
pub fn atan_to(from: Vec2, to: Vec2) -> Radians {
    let d = to - from;
    atan2(d.y, d.x)
}

/// The radius of the extraction pad.
pub const PAD_RADIUS: f32 = 1.0;
/// The radius of a cup.
pub const CUP_RADIUS: f32 = 0.5;
/// How many times a seeded placement is redrawn while it crowds something.
const REDRAWS: usize = 8;

/// The four golfers' fixed seats, seat 0 (the player) at 45 degrees.
pub fn seats() -> [Vec2; 4] {
    let mut out = [Vec2::ZERO; 4];
    for (i, seat) in out.iter_mut().enumerate() {
        let angle = Radians::from_degrees(45.0 * (2 * i + 1) as f32);
        *seat = COURSE_CENTER + polar(5.0, angle);
    }
    out
}

/// Where seat `i`'s ball starts: 0.8 from the seat, toward the course centre.
pub fn ball_start(seat: Vec2) -> Vec2 {
    seat + (COURSE_CENTER - seat).normalize_or_zero() * 0.8
}

/// A point uniform in `circle` shrunk by `margin`, from two draws.
fn draw_in(rng: &mut Rng, center: Vec2, room: f32) -> Vec2 {
    let theta = Radians(rng.next_f32() * Radians::TAU.as_f32());
    let rho = rng.next_f32().sqrt() * room.max(0.0);
    center + polar(rho, theta)
}

/// Whether `p` is within `gap` of any of `others`.
fn crowds(p: Vec2, others: &[Vec2], gap: f32) -> bool {
    others.iter().any(|other| p.distance(*other) < gap)
}

/// The kinds of the six pickups, in seeding order.
pub const PICKUP_KINDS: [Item; 6] = [
    Item::Driver,
    Item::Spikes,
    Item::Heavy,
    Item::Helmet,
    Item::Driver,
    Item::Spikes,
];

/// The course a seed means, and the six pickups on it — drawn in exactly one
/// order, so a seed is one course.
pub fn seed_course(rng: &mut Rng) -> (Course, Vec<(Vec2, Item)>) {
    let mut zones = [Circle {
        center: COURSE_CENTER,
        radius: ZONE_RADII[0],
    }; 5];
    for k in 0..4 {
        let theta = Radians(rng.next_f32() * Radians::TAU.as_f32());
        let rho = rng.next_f32() * (ZONE_RADII[k] - ZONE_RADII[k + 1]);
        zones[k + 1] = Circle {
            center: zones[k].center + polar(rho, theta),
            radius: ZONE_RADII[k + 1],
        };
    }
    let seats = seats();
    let mut cups: Vec<Vec2> = Vec::new();
    for _ in 0..3 {
        let mut at = draw_in(rng, COURSE_CENTER, ZONE_RADII[0] - 1.0);
        for _ in 0..REDRAWS {
            if !(crowds(at, &seats, 2.0) || crowds(at, &cups, 2.0)) {
                break;
            }
            at = draw_in(rng, COURSE_CENTER, ZONE_RADII[0] - 1.0);
        }
        cups.push(at);
    }
    let pad_room = ZONE_RADII[1] - PAD_RADIUS - 0.2;
    let mut pad = draw_in(rng, zones[1].center, pad_room);
    for _ in 0..REDRAWS {
        if !crowds(pad, &cups, 1.5) {
            break;
        }
        pad = draw_in(rng, zones[1].center, pad_room);
    }
    let field = inset(course(), 1.0);
    let mut pickups = Vec::new();
    for item in PICKUP_KINDS {
        let mut taken: Vec<Vec2> = seats.to_vec();
        taken.extend_from_slice(&cups);
        taken.push(pad);
        let draw = |rng: &mut Rng| {
            let (u, v) = (rng.next_f32(), rng.next_f32());
            field.min + field.size() * Vec2::new(u, v)
        };
        let mut at = draw(rng);
        for _ in 0..REDRAWS {
            if !crowds(at, &taken, 1.5) {
                break;
            }
            at = draw(rng);
        }
        pickups.push((at, item));
    }
    let course = Course {
        zones,
        cups: [cups[0], cups[1], cups[2]],
        pad: Circle {
            center: pad,
            radius: PAD_RADIUS,
        },
        seats,
    };
    (course, pickups)
}

/// A redrawn cup after a hole: inside `zone`, clear of the other cups and the pad.
pub fn redraw_cup(rng: &mut Rng, zone: Circle, others: &[Vec2], pad: Vec2) -> Vec2 {
    let room = zone.radius - CUP_RADIUS;
    let mut at = draw_in(rng, zone.center, room);
    for _ in 0..REDRAWS {
        if !(crowds(at, others, 1.5) || crowds(at, &[pad], 1.0)) {
            break;
        }
        at = draw_in(rng, zone.center, room);
    }
    at
}

/// The `n` segments of `circle`'s rim, vertex `i` at `i * TAU / n` from +X.
pub fn ring_segments(circle: Circle, n: usize) -> Vec<(Vec2, Vec2)> {
    let vertex = |i: usize| {
        let angle = Radians(Radians::TAU.as_f32() * (i % n) as f32 / n as f32);
        circle.center + polar(circle.radius, angle)
    };
    (0..n).map(|i| (vertex(i), vertex(i + 1))).collect()
}

// --- the roll ---------------------------------------------------------------

/// The fastest a full-charge strike sends a ball, in units per second.
pub const MAX_SHOT_SPEED: f32 = 18.0;
/// How many ticks of holding Space make a full charge.
pub const CHARGE_TICKS: u32 = 60;
/// How fast a rolling ball slows, in units per second per second.
pub const FRICTION: f32 = 8.0;
/// A ball's radius.
pub const BALL_RADIUS: f32 = 0.16;

/// The speed a charge of `charge` ticks strikes at, before equipment.
pub fn shot_speed(charge: u32) -> f32 {
    charge.min(CHARGE_TICKS) as f32 / CHARGE_TICKS as f32 * MAX_SHOT_SPEED
}

/// How many ticks a ball struck at `speed` rolls before it stops.
pub fn roll_ticks(speed: f32, dt: Seconds) -> u64 {
    (speed / (FRICTION * dt.as_f32())).floor() as u64
}

/// How far a ball struck at `speed` has rolled after `ticks` ticks — a
/// closed form, not an integration, so the preview, the NPC's plan and the
/// ball agree bit for bit.
pub fn rolled(speed: f32, ticks: u64, dt: Seconds) -> f32 {
    let decel = FRICTION * dt.as_f32();
    let n = ticks.min(roll_ticks(speed, dt)) as f32;
    dt.as_f32() * (n * speed - decel * n * (n + 1.0) / 2.0)
}

/// How far a ball struck at `speed` rolls before it stops, unfenced.
pub fn roll_distance(speed: f32, dt: Seconds) -> f32 {
    rolled(speed, roll_ticks(speed, dt), dt)
}

/// Where a struck ball stops, how many ticks it takes, and whether the fence
/// stopped it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Landing {
    pub at: Vec2,
    pub ticks: u64,
    /// How far along its line the ball travels: the roll, or the fence.
    pub distance: f32,
    pub fenced: bool,
}

/// Where a ball struck from `from` along the unit `dir` at `speed` stops, the
/// fence (`fence` inset by the ball's radius) stopping it dead.
pub fn landing_of(from: Vec2, dir: Vec2, speed: f32, fence: Rect, dt: Seconds) -> Landing {
    let inner = inset(fence, BALL_RADIUS);
    let full_ticks = roll_ticks(speed, dt);
    let full = rolled(speed, full_ticks, dt);
    let end = from + dir * full;
    let inside = end.x >= inner.min.x
        && end.x <= inner.max.x
        && end.y >= inner.min.y
        && end.y <= inner.max.y;
    if inside {
        return Landing {
            at: end,
            ticks: full_ticks,
            distance: full,
            fenced: false,
        };
    }
    // The slab test: the first boundary the segment crosses, and which axis.
    let mut hit = full;
    let mut axis: Option<(bool, f32)> = None;
    for (along, start, low, high, is_x) in [
        (dir.x, from.x, inner.min.x, inner.max.x, true),
        (dir.y, from.y, inner.min.y, inner.max.y, false),
    ] {
        let wall = if along > 0.0 {
            Some(high)
        } else if along < 0.0 {
            Some(low)
        } else {
            None
        };
        if let Some(wall) = wall {
            let d = ((wall - start) / along).max(0.0);
            if d < hit {
                hit = d;
                axis = Some((is_x, wall));
            }
        }
    }
    let mut at = from + dir * hit;
    if let Some((is_x, wall)) = axis {
        if is_x {
            at.x = wall;
        } else {
            at.y = wall;
        }
    }
    let mut ticks = 0;
    while ticks < full_ticks && rolled(speed, ticks, dt) < hit {
        ticks += 1;
    }
    Landing {
        at,
        ticks,
        distance: hit,
        fenced: true,
    }
}

/// The charge, 1 to 60, whose unfenced roll at `scale` comes closest to
/// `distance` — 60 when the distance is beyond a full charge.
pub fn charge_for(distance: f32, scale: f32, dt: Seconds) -> u32 {
    let mut best = CHARGE_TICKS;
    let mut best_gap = f32::MAX;
    for k in 1..=CHARGE_TICKS {
        let gap = (roll_distance(shot_speed(k) * scale, dt) - distance).abs();
        if gap < best_gap {
            best_gap = gap;
            best = k;
        }
    }
    best
}

// --- equipment --------------------------------------------------------------

/// One kind of equipment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Driver,
    Spikes,
    Heavy,
    Helmet,
}

/// What an item is called, what extracting with it is worth, and what it does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemSpec {
    pub item: Item,
    pub name: &'static str,
    pub worth: u32,
    pub line: &'static str,
}

/// The equipment table: the pickup rows print `line`, the sim applies `effects`.
pub const ITEMS: [ItemSpec; 4] = [
    ItemSpec {
        item: Item::Driver,
        name: "Driver",
        worth: 3,
        line: "strike 1.5x faster",
    },
    ItemSpec {
        item: Item::Spikes,
        name: "Spikes",
        worth: 3,
        line: "walk 1.5x faster",
    },
    ItemSpec {
        item: Item::Heavy,
        name: "Heavy",
        worth: 4,
        line: "rivals hit your ball 0.5x",
    },
    ItemSpec {
        item: Item::Helmet,
        name: "Helmet",
        worth: 3,
        line: "club stuns 1.0s not 3.0s",
    },
];

/// The row of the equipment table for `item`.
pub fn spec(item: Item) -> ItemSpec {
    match item {
        Item::Driver => ITEMS[0],
        Item::Spikes => ITEMS[1],
        Item::Heavy => ITEMS[2],
        Item::Helmet => ITEMS[3],
    }
}

/// How many items a golfer can hold.
pub const SLOTS: usize = 2;
/// A Driver's strike multiplier.
pub const DRIVER_SHOT_SCALE: f32 = 1.5;
/// Spikes' walk multiplier.
pub const SPIKES_WALK_SCALE: f32 = 1.5;
/// How long a club stuns, in ticks.
pub const STUN_TICKS: u64 = 180;
/// How long a club stuns a golfer wearing a Helmet, in ticks.
pub const HELMET_STUN_TICKS: u64 = 60;
/// How fast a rival's strike sends a Heavy ball, as a share.
pub const HEAVY_SLEDGED_SCALE: f32 = 0.5;

/// What holding `held` does — the one equipment-effect function (row 3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effects {
    pub shot_scale: f32,
    pub walk_scale: f32,
    pub stun_ticks: u64,
    pub sledged_scale: f32,
}

/// The effects of holding `held`. An item is present or not; none stacks.
pub fn effects(held: &[Item]) -> Effects {
    let has = |item: Item| held.contains(&item);
    Effects {
        shot_scale: if has(Item::Driver) {
            DRIVER_SHOT_SCALE
        } else {
            1.0
        },
        walk_scale: if has(Item::Spikes) {
            SPIKES_WALK_SCALE
        } else {
            1.0
        },
        stun_ticks: if has(Item::Helmet) {
            HELMET_STUN_TICKS
        } else {
            STUN_TICKS
        },
        sledged_scale: if has(Item::Heavy) {
            HEAVY_SLEDGED_SCALE
        } else {
            1.0
        },
    }
}

/// The speed a strike sends a ball: the charge, the striker's club, and — when
/// it is somebody else's ball — the owner's Heavy.
pub fn shot_speed_for(striker: &Effects, owner: &Effects, own: bool, charge: u32) -> f32 {
    let resist = if own { 1.0 } else { owner.sledged_scale };
    shot_speed(charge) * striker.shot_scale * resist
}

// --- golfers, reach and the outcome -------------------------------------------

/// A golfer's fate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fate {
    Playing,
    Stunned { until: u64 },
    Extracted { banked: u32 },
    Survived { banked: u32 },
    Eliminated { at: u64 },
}

impl Fate {
    /// Still in the match: playing or stunned.
    pub fn alive(self) -> bool {
        matches!(self, Fate::Playing | Fate::Stunned { .. })
    }
}

/// One golfer, as every decision reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct GolferView {
    pub seat: usize,
    pub pos: Vec2,
    pub fate: Fate,
    pub held: Vec<Item>,
    pub points: u32,
    pub out_ticks: u64,
    pub cooldown: u64,
    pub aim: Radians,
    pub charge: u32,
}

/// One ball, as every decision reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BallView {
    pub owner: usize,
    pub pos: Vec2,
    pub at_rest: bool,
    pub landing: Option<Landing>,
}

/// One pickup, as every decision reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickupView {
    pub pos: Vec2,
    pub item: Item,
}

/// How far a club reaches, centre to centre.
pub const CLUB_REACH: f32 = 1.2;
/// How far a golfer reaches to strike a ball.
pub const BALL_REACH: f32 = 0.9;
/// How far a golfer reaches to take a pickup.
pub const PICKUP_REACH: f32 = 0.9;
/// A golfer's radius.
pub const GOLFER_RADIUS: f32 = 0.35;
/// How long after a club before the next, in ticks.
pub const CLUB_COOLDOWN_TICKS: u64 = 90;

/// What a golfer can act on: a rival to club, a ball to strike, a pickup.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Reach {
    /// The seat of the nearest playing rival in club reach.
    pub club: Option<usize>,
    /// The owner of the nearest at-rest ball in reach.
    pub ball: Option<usize>,
    /// The index of the nearest pickup in reach.
    pub pickup: Option<usize>,
}

/// What `me` can act on — the one contact-resolution function (row 2). The
/// reach cue, the panel's rows and the sim's `act` all read it.
pub fn in_reach(
    me: &GolferView,
    golfers: &[GolferView],
    balls: &[BallView],
    pickups: &[PickupView],
) -> Reach {
    let nearest = |candidates: Vec<(usize, f32)>| {
        candidates
            .into_iter()
            .fold(None, |best: Option<(usize, f32)>, (id, d)| match best {
                Some((_, best_d)) if best_d <= d => best,
                _ => Some((id, d)),
            })
            .map(|(id, _)| id)
    };
    let alive = |seat: usize| golfers.iter().any(|g| g.seat == seat && g.fate.alive());
    let club = nearest(
        golfers
            .iter()
            .filter(|g| g.seat != me.seat && g.fate == Fate::Playing)
            .map(|g| (g.seat, g.pos.distance(me.pos)))
            .filter(|(_, d)| *d <= CLUB_REACH)
            .collect(),
    );
    // Own ball first, so it wins a tie.
    let mut ordered: Vec<&BallView> = balls.iter().filter(|b| b.owner == me.seat).collect();
    ordered.extend(balls.iter().filter(|b| b.owner != me.seat));
    let ball = nearest(
        ordered
            .into_iter()
            .filter(|b| b.at_rest && alive(b.owner))
            .map(|b| (b.owner, b.pos.distance(me.pos)))
            .filter(|(_, d)| *d <= BALL_REACH)
            .collect(),
    );
    let pickup = nearest(
        pickups
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.pos.distance(me.pos)))
            .filter(|(_, d)| *d <= PICKUP_REACH)
            .collect(),
    );
    Reach { club, ball, pickup }
}

/// Points for holing a cup.
pub const HOLE_POINTS: u32 = 2;
/// Points for striking a rival's ball.
pub const SLEDGE_POINTS: u32 = 2;
/// What the last golfer standing banks on top.
pub const SURVIVOR_BONUS: u32 = 6;

/// What extracting now would keep: points plus the worth of what is held.
pub fn bank_now(points: u32, held: &[Item]) -> u32 {
    points + held.iter().map(|item| spec(*item).worth).sum::<u32>()
}

/// What a golfer banks — the one outcome function (row 4). The status panel's
/// `bank now` and `X:` rows and the result screen all read it.
pub fn banked(fate: Fate, points: u32, held: &[Item]) -> u32 {
    match fate {
        Fate::Extracted { banked } | Fate::Survived { banked } => banked,
        Fate::Eliminated { .. } => 0,
        Fate::Playing | Fate::Stunned { .. } => bank_now(points, held),
    }
}

/// How fast a golfer walks, in units per second, before Spikes.
pub const WALK_SPEED: f32 = 4.0;
/// How fast the player's aim turns, per second.
pub const AIM_RATE: Radians = Radians::from_degrees(150.0);

/// How many ticks a golfer at `walk_scale` takes to walk `distance`.
pub fn walk_ticks(distance: f32, walk_scale: f32, dt: Seconds) -> u64 {
    (distance / (WALK_SPEED * walk_scale * dt.as_f32())).ceil() as u64
}
