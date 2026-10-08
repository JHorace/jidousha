//! The simulation: the components, the match state, the read-only snapshot
//! every decision reads, and the Update systems in the order `main.rs`
//! registers them.
//!
//! Every system here applies a rule from `rules.rs` rather than deciding one,
//! so a check can ask the rule directly and then watch the system obey it.

use jidousha::prelude::*;

use crate::npc;
use crate::rules::{
    BallView, CLUB_COOLDOWN_TICKS, CUP_RADIUS, Circle, Course, Fate, GOLFER_RADIUS,
    GRACE_TICKS, GolferView, HOLE_POINTS, Item, Landing, PAD_OPENS_TICK, PickupView, SLEDGE_POINTS,
    SLOTS, SURVIVOR_BONUS, WALK_SPEED, ball_start, bank_now, banked, course, effects, in_reach,
    inset, landing_of, next_zone, polar, redraw_cup, rolled, seed_course, shot_speed_for, zone_at,
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
    Over {
        at: u64,
        winner: Option<usize>,
    },
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

fn live(world: &World) -> bool {
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
            let axis = |minus: Key, plus: Key| {
                f32::from(input.held(plus)) - f32::from(input.held(minus))
            };
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
        .map(|seat| if seat == 0 { Intent::IDLE } else { npc::decide(&snapshot, seat) })
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
                golfer.aim = Radians(golfer.aim.as_f32() + f32::from(sign) * crate::rules::AIM_RATE.as_f32() * dt);
            }
            Aim::Set(angle) => golfer.aim = angle,
        }
        if intent.space_held {
            golfer.charge = (golfer.charge + 1).min(crate::rules::CHARGE_TICKS);
        }
    }
}

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
        let others: Vec<Vec2> = (0..3).filter(|i| *i != cup).map(|i| course.cups[i]).collect();
        let into = next_zone(&course, tick).unwrap_or(zone);
        let pad = course.pad.center;
        course.cups[cup] = redraw_cup(world.resource_mut::<Rng>(), into, &others, pad);
    }
    for (_, golfer) in world.query_mut::<&mut Golfer>() {
        if golfer.fate.alive() {
            golfer.points += HOLE_POINTS * scored.iter().filter(|s| **s == golfer.seat).count() as u32;
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
        let ball = balls.iter().find(|(o, _)| *o == golfer.seat).map(|(_, p)| *p);
        let out = !zone.contains(transform.pos) || ball.is_none_or(|p| !zone.contains(p));
        golfer.out_ticks = if out { golfer.out_ticks + 1 } else { 0 };
        if golfer.out_ticks >= GRACE_TICKS {
            golfer.fate = Fate::Eliminated { at: tick };
        }
    }
}

/// When nobody is left playing — or one golfer is — the match is over.
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
    if alive.len() > 1 {
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
