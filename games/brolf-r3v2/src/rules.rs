//! Every pure decision the sim, the HUD and the checks all read: the zone at a
//! tick, one tick of a rolling ball and where a shot will stop, who a swing
//! hits and what it does, what equipment does, and what an ending keeps.
//!
//! No `World` in any signature (DESIGN.md, Systems: rules). Each decision row
//! of the spec has its one function here — `zone_at`, `contact_target` with
//! `resolve_contact`, `effects`, `outcome_of` — so the surface that shows a
//! fact and the system that acts on it cannot disagree.

use jidousha::prelude::*;

use crate::model::*;

/// The safe zone: a circle around the cup.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    /// Its centre.
    pub center: Vec2,
    /// Its radius.
    pub radius: f32,
}

/// The zone at `tick`: linear between `ZONE_TABLE` rows, radius 0 after the last.
pub fn zone_at(tick: u64) -> Zone {
    let mut radius = 0.0;
    for pair in ZONE_TABLE.windows(2) {
        let ((t0, r0), (t1, r1)) = (pair[0], pair[1]);
        if tick >= t0 && tick <= t1 {
            let share = (tick - t0) as f32 / (t1 - t0) as f32;
            radius = r0 + (r1 - r0) * share;
            break;
        }
    }
    Zone {
        center: CUP,
        radius,
    }
}

/// Whether `point` is inside `zone`, edge included.
pub fn inside_zone(zone: Zone, point: Vec2) -> bool {
    zone.center.distance(point) <= zone.radius
}

/// The first tick at which `point` is outside the zone, or `None` if never.
///
/// Solved segment by segment from the table rather than scanned; the contracts
/// check it against a scan.
pub fn zone_excludes_at(point: Vec2) -> Option<u64> {
    let distance = CUP.distance(point);
    for pair in ZONE_TABLE.windows(2) {
        let ((t0, r0), (t1, r1)) = (pair[0], pair[1]);
        if r1 >= distance {
            continue;
        }
        if r0 < distance {
            return Some(t0);
        }
        // Radius falls through `distance` inside this segment: the first whole
        // tick past the crossing, then nudged so the answer agrees with
        // `zone_at` to the bit.
        let cross = t0 as f32 + (r0 - distance) / (r0 - r1) * (t1 - t0) as f32;
        let mut tick = (cross.floor() as u64).saturating_sub(1).max(t0);
        while inside_zone(zone_at(tick), point) {
            tick += 1;
        }
        return Some(tick);
    }
    // Only the cup itself stays inside a zone of radius 0.
    None
}

/// Whether the cup takes a ball on `tick`.
pub fn cup_open(tick: u64) -> bool {
    tick >= CUP_OPENS
}

/// One tick of a ball.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BallStep {
    /// Where it is.
    pub pos: Vec2,
    /// How it is moving.
    pub vel: Vec2,
    /// Whether it is over the cup slowly enough to drop — if the cup is open.
    pub sunk: bool,
}

/// One tick of a rolling ball: slow, stop if slow enough, move by the new
/// velocity, bounce off the course edge, and sink if slow and over the cup.
pub fn ball_step(pos: Vec2, vel: Vec2, dt: Seconds) -> BallStep {
    let dt = dt.as_f32();
    let speed = vel.length();
    let mut slowed = (speed - BALL_DECEL * dt).max(0.0);
    if slowed < REST_SPEED {
        slowed = 0.0;
    }
    let mut vel = if speed > 0.0 {
        vel * (slowed / speed)
    } else {
        Vec2::ZERO
    };
    let mut pos = pos + vel * dt;
    let min = COURSE.min + Vec2::splat(BALL_RADIUS);
    let max = COURSE.max - Vec2::splat(BALL_RADIUS);
    if pos.x < min.x {
        pos.x = 2.0 * min.x - pos.x;
        vel.x = -vel.x;
    } else if pos.x > max.x {
        pos.x = 2.0 * max.x - pos.x;
        vel.x = -vel.x;
    }
    if pos.y < min.y {
        pos.y = 2.0 * min.y - pos.y;
        vel.y = -vel.y;
    } else if pos.y > max.y {
        pos.y = 2.0 * max.y - pos.y;
        vel.y = -vel.y;
    }
    let sunk = pos.distance(CUP) <= CUP_RADIUS && slowed < SINK_SPEED;
    BallStep { pos, vel, sunk }
}

/// Where a ball comes to rest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RollOut {
    /// Where it stops (or sinks).
    pub rest: Vec2,
    /// How many ticks it rolls.
    pub ticks: u32,
    /// Whether it sinks.
    pub sunk: bool,
}

/// The most ticks a roll is followed; no shot this game makes rolls longer.
const ROLL_CAP: u32 = 2_000;

/// `ball_step` until the ball rests or sinks — the aim line's landing point.
///
/// `first_tick` is the tick the first step runs on, so the cup is open (or
/// shut) for the prediction exactly when it is for the roll.
pub fn roll_out(pos: Vec2, vel: Vec2, dt: Seconds, first_tick: u64) -> RollOut {
    let (mut pos, mut vel) = (pos, vel);
    let mut ticks = 0;
    while vel != Vec2::ZERO && ticks < ROLL_CAP {
        let step = ball_step(pos, vel, dt);
        let on = first_tick + u64::from(ticks);
        ticks += 1;
        pos = step.pos;
        vel = step.vel;
        if step.sunk && cup_open(on) {
            return RollOut {
                rest: pos,
                ticks,
                sunk: true,
            };
        }
    }
    RollOut {
        rest: pos,
        ticks,
        sunk: false,
    }
}

/// The unit vector along `aim`.
pub fn direction(aim: Radians) -> Vec2 {
    let (sine, cosine) = sin_cos(aim);
    Vec2::new(cosine, sine)
}

/// A shot's velocity: along `aim`, at the power's speed times the kit's scale.
pub fn shot_velocity(aim: Radians, power: Power, fx: Effects) -> Vec2 {
    direction(aim) * (power.speed() * fx.shot_scale)
}

/// The tick a golfer at `golfer_pos` reaches its ball after this roll.
pub fn arrival_tick(now: u64, golfer_pos: Vec2, roll: &RollOut, dt: Seconds) -> u64 {
    let walk = golfer_pos.distance(roll.rest) / (WALK_SPEED * dt.as_f32());
    now + u64::from(roll.ticks) + walk.ceil() as u64
}

/// What one golfer is, for the functions that decide about golfers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GolferSnap {
    /// Its index, 0 being You.
    pub idx: u8,
    /// Where it stands.
    pub pos: Vec2,
    /// Still in play.
    pub alive: bool,
    /// Ticks of disable left.
    pub stun_left: u32,
    /// Ticks of swing cooldown left.
    pub cooldown: u32,
    /// What it holds.
    pub kit: Kit,
    /// Where it aims.
    pub aim: Radians,
    /// How hard.
    pub power: Power,
    /// Ticks it has spent outside the zone in a row.
    pub outside_ticks: u32,
    /// Ticks it has held extract in the gate.
    pub extract_ticks: u32,
}

/// One ball, for the same functions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BallSnap {
    /// Whose it is.
    pub owner: u8,
    /// Where it is.
    pub pos: Vec2,
    /// How it moves.
    pub vel: Vec2,
}

/// What a swing will hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    /// The golfer with this index.
    Club(u8),
    /// The ball of the golfer with this index.
    Strike(u8),
}

/// The nearest rival golfer or rival ball within reach — the one function the
/// reach cue and the contact key both read. Ties go to the golfer.
pub fn contact_target(
    me: &GolferSnap,
    others: &[GolferSnap],
    balls: &[BallSnap],
    fx: Effects,
) -> Option<Contact> {
    let reach = CLUB_REACH * fx.reach_scale;
    let golfers = others
        .iter()
        .filter(|other| other.alive && other.idx != me.idx)
        .map(|other| (me.pos.distance(other.pos), 0, Contact::Club(other.idx)));
    let struck = balls
        .iter()
        .filter(|ball| ball.owner != me.idx)
        .map(|ball| (me.pos.distance(ball.pos), 1, Contact::Strike(ball.owner)));
    golfers
        .chain(struck)
        .filter(|(distance, _, _)| *distance <= reach)
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, _, contact)| contact)
}

/// What a swing does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactResult {
    /// Ticks the struck golfer is disabled for.
    pub stun_ticks: u32,
    /// Whether its best item is knocked loose.
    pub drops: bool,
    /// The struck ball's new velocity, if it moves.
    pub ball_vel: Option<Vec2>,
}

/// Resolve a swing aimed along `aim` against a target holding `target_fx`.
pub fn resolve_contact(contact: Contact, aim: Radians, target_fx: Effects) -> ContactResult {
    match contact {
        Contact::Club(_) => ContactResult {
            stun_ticks: (CLUB_STUN as f32 * target_fx.stun_scale).round() as u32,
            drops: target_fx.drops_when_clubbed,
            ball_vel: None,
        },
        Contact::Strike(_) => ContactResult {
            stun_ticks: 0,
            drops: false,
            ball_vel: (!target_fx.strike_immune).then(|| direction(aim) * STRIKE_SPEED),
        },
    }
}

/// What a kit does — the one equipment-effect function.
pub fn effects(kit: &Kit) -> Effects {
    let mut fx = Effects::BARE;
    for item in kit.items() {
        match item {
            Item::Driver => fx.shot_scale *= 1.4,
            Item::LongClub => fx.reach_scale *= 1.6,
            Item::LeadBall => {
                fx.strike_immune = true;
                fx.shot_scale *= 0.7;
            }
            Item::Helmet => {
                fx.stun_scale *= 0.5;
                fx.drops_when_clubbed = false;
            }
        }
    }
    fx
}

/// An item's effect, in the words the HUD shows, built from `effects`.
pub fn describe(item: Item) -> String {
    let fx = effects(&Kit::only(item));
    match item {
        Item::Driver => format!("shot speed x{:.1}", fx.shot_scale),
        Item::LongClub => format!("club reach x{:.1}", fx.reach_scale),
        Item::LeadBall => format!("strike-proof, shot speed x{:.1}", fx.shot_scale),
        Item::Helmet => format!("stun x{:.1}, keeps items if clubbed", fx.stun_scale),
    }
}

/// The items worth the most, best first, ties in slot order Club, Ball, Head.
pub fn ranked(kit: &Kit) -> Vec<Item> {
    let mut items = kit.items();
    // A stable sort over slot order keeps the tie-break.
    items.sort_by_key(|item| std::cmp::Reverse(item.value()));
    items
}

/// What an extraction keeps: the `BAG_LIMIT` most valuable items.
pub fn kept_on_extract(kit: &Kit) -> Vec<Item> {
    ranked(kit).into_iter().take(BAG_LIMIT).collect()
}

/// What an ending is worth — the one outcome function the status line and the
/// result banner both read.
pub fn outcome_of(kind: EndKind, kit: &Kit) -> Outcome {
    let (kept, bonus) = match kind {
        EndKind::Holed | EndKind::LastStanding => (kit.items(), WIN_BONUS),
        EndKind::Extracted => (kept_on_extract(kit), 0),
        EndKind::Eliminated | EndKind::Lost { .. } => (Vec::new(), 0),
    };
    let points = if kept.is_empty() && bonus == 0 {
        0
    } else {
        kept.iter().map(|item| item.value()).sum::<u32>() + bonus
    };
    Outcome { kind, kept, points }
}

/// The part of segment `a`-`b` inside `COURSE`, or `None` (Liang-Barsky).
pub fn clip_to_course(a: Vec2, b: Vec2) -> Option<(Vec2, Vec2)> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0_f32, 1.0_f32);
    let edges = [
        (-d.x, a.x - COURSE.min.x),
        (d.x, COURSE.max.x - a.x),
        (-d.y, a.y - COURSE.min.y),
        (d.y, COURSE.max.y - a.y),
    ];
    for (p, q) in edges {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some((a + d * t0, a + d * t1))
}
