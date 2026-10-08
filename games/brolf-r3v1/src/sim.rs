//! The match as one plain value, and the tick that advances it.
//!
//! `Match` is `Clone` and owns its own `Rng`, so a controller in `--verify`
//! rolls a copy forward with the same `step` the game runs. Golfers act in
//! index order every tick (the player is 0), then balls roll, then the zone
//! judges, then endings — the order is stated here once and `step` is the
//! only place it lives.

use jidousha::prelude::*;

use crate::events::{Event, Intent};
use crate::rules::{
    self, BOUNTY_TICKS, CHAMPION_HOLES, CONTACT_COOLDOWN, COURSE, CUP_RADIUS, CUP_SPEED,
    CUP_TOKENS, Contact, DT, Ending, GRACE_TICKS, Item, Kept, PAD_RADIUS, PADS_OPEN, PICKUP_RADIUS,
    SWING_LOCK,
};
use crate::zone::{Schedule, inside_at};

/// How fast a golfer walks, in world units per second.
pub const WALK_SPEED: f32 = 7.0;

/// How fast the aim turns, in degrees per second.
pub const TURN_RATE: f32 = 120.0;

/// How fast power changes, per second.
pub const POWER_RATE: f32 = 0.8;

/// The weakest shot.
pub const MIN_POWER: f32 = 0.1;

/// How many golfers play a match.
pub const GOLFERS: usize = 6;

/// How many cups the course has.
pub const CUPS: usize = 5;

/// When each cup opens, in seconds: one at a time, so every cup is a race.
pub const CUP_OPENS: [u64; CUPS] = [0, 12, 24, 36, 48];

/// A ball: where it is and how fast it is going. At rest when `vel` is zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    /// Where.
    pub pos: Vec2,
    /// How fast, in world units per second.
    pub vel: Vec2,
}

/// Who decides what a golfer does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Brain {
    /// The keyboard (or a `--verify` player).
    Player,
    /// Clubs and strikes whenever it can.
    Brute,
    /// Chases cups for the champion win.
    Holer,
    /// Extracts as soon as it has something to keep.
    Banker,
    /// Takes equipment, then plays cups.
    Rover,
    /// Does nothing; for staged checks.
    Idle,
}

/// One golfer and its ball.
#[derive(Clone, Debug)]
pub struct Golfer {
    /// Where the golfer stands.
    pub pos: Vec2,
    /// Its ball.
    pub ball: Ball,
    /// Which way the next shot goes.
    pub aim: Radians,
    /// How hard, 0.1 to 1.0.
    pub power: f32,
    /// Who drives it.
    pub brain: Brain,
    /// Tokens carried.
    pub stash: u32,
    /// Cups sunk.
    pub holes: u32,
    /// Shots taken.
    pub shots: u32,
    /// What it holds.
    pub item: Option<Item>,
    /// Ticks left stunned.
    pub stun: u32,
    /// Ticks until it may be clubbed again: the stun plus `GUARD_TICKS`.
    pub guard: u32,
    /// Ticks until it may make contact again.
    pub cooldown: u32,
    /// Ticks left rooted by its own swing.
    pub lock: u32,
    /// Consecutive ticks it or its ball has been outside the zone.
    pub out_ticks: u32,
    /// The last rival to club it or strike its ball, and when.
    pub last_hit: Option<(usize, u64)>,
    /// How its match ended, what it kept, and on which tick.
    pub ending: Option<(Ending, Kept, u64)>,
}

/// A piece of equipment lying on the course.
#[derive(Clone, Copy, Debug)]
pub struct Pickup {
    /// Where.
    pub pos: Vec2,
    /// What.
    pub item: Item,
    /// Whether it has been taken.
    pub taken: bool,
}

/// The whole match.
#[derive(Clone, Debug)]
pub struct Match {
    /// The seed it was drawn from.
    pub seed: u64,
    /// Ticks played.
    pub tick: u64,
    /// The match's own random numbers.
    pub rng: Rng,
    /// The zone.
    pub schedule: Schedule,
    /// Everyone, the player first.
    pub golfers: Vec<Golfer>,
    /// The cups.
    pub cups: [Vec2; CUPS],
    /// Who sank each cup; a cup takes one ball and is then closed.
    pub cup_taken_by: [Option<usize>; CUPS],
    /// Equipment on the course.
    pub pickups: Vec<Pickup>,
    /// The two extraction pads.
    pub pads: [Vec2; 2],
    /// Everything that happened, in order, with its tick.
    pub log: Vec<(u64, Event)>,
}

/// A point inside the course, at least `inset` from its edge.
fn keep_on_course(point: Vec2, inset: f32) -> Vec2 {
    Vec2::new(
        point.x.clamp(COURSE.min.x + inset, COURSE.max.x - inset),
        point.y.clamp(COURSE.min.y + inset, COURSE.max.y - inset),
    )
}

/// A random point within `radius` of `center`, on the course.
fn scatter(rng: &mut Rng, center: Vec2, radius: f32, inset: f32) -> Vec2 {
    let angle = Radians(rng.next_f32() * core::f32::consts::TAU);
    let (sin, cos) = sin_cos(angle);
    let distance = radius * rng.next_f32().sqrt();
    keep_on_course(center + Vec2::new(cos, sin) * distance, inset)
}

impl Match {
    /// A fresh match drawn from `seed`.
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::from_seed(seed);
        let schedule = Schedule::draw(&mut rng);
        let brains = [
            Brain::Player,
            Brain::Brute,
            Brain::Holer,
            Brain::Banker,
            Brain::Rover,
            Brain::Brute,
        ];
        let golfers = (0..GOLFERS)
            .map(|index| {
                let angle = Radians::from_degrees(90.0 + 60.0 * index as f32);
                let (sin, cos) = sin_cos(angle);
                let pos = Vec2::new(cos * 24.0, sin * 11.0);
                let ball = pos - pos.normalize_or_zero() * 1.0;
                Golfer {
                    pos,
                    ball: Ball {
                        pos: ball,
                        vel: Vec2::ZERO,
                    },
                    aim: rules_aim_toward(ball, schedule.circles[1].center),
                    power: 0.6,
                    brain: brains[index],
                    stash: 0,
                    holes: 0,
                    shots: 0,
                    item: None,
                    stun: 0,
                    guard: 0,
                    cooldown: 0,
                    lock: 0,
                    out_ticks: 0,
                    last_hit: None,
                    ending: None,
                }
            })
            .collect();
        // On a squashed ring around the third circle's centre, a fifth of a
        // turn apart with a little play, so no two sit together.
        let cups_center = schedule.circles[1].center;
        let ring_center = schedule.circles[2].center;
        let turn = rng.next_f32();
        let mut cups = [Vec2::ZERO; CUPS];
        for (index, cup) in cups.iter_mut().enumerate() {
            let angle = Radians(
                (index as f32 + turn + rng.next_f32() * 0.3) * core::f32::consts::TAU / CUPS as f32,
            );
            let (sin, cos) = sin_cos(angle);
            let reach = 7.0 + rng.next_f32() * 4.0;
            *cup = keep_on_course(
                ring_center + Vec2::new(cos * reach * 1.3, sin * reach * 0.75),
                2.0,
            );
        }
        let items = [Item::Heavy, Item::Helmet, Item::Driver];
        let pickups = (0..4)
            .map(|index| Pickup {
                pos: scatter(&mut rng, cups_center, 18.0, 2.0),
                item: items[if index < 3 {
                    index
                } else {
                    rng.below(3) as usize
                }],
                taken: false,
            })
            .collect();
        let pad_center = schedule.circles[2].center;
        let angle = Radians(rng.next_f32() * core::f32::consts::TAU);
        let (sin, cos) = sin_cos(angle);
        let offset = Vec2::new(cos * 8.0, sin * 5.0);
        let pads = [
            keep_on_course(pad_center + offset, 2.0),
            keep_on_course(pad_center - offset, 2.0),
        ];
        Self {
            seed,
            tick: 0,
            rng,
            schedule,
            golfers,
            cups,
            cup_taken_by: [None; CUPS],
            pickups,
            pads,
            log: Vec::new(),
        }
    }

    /// Whether the match is over for the player, which is when the result
    /// screen shows and the sim stops.
    pub fn over(&self) -> bool {
        self.golfers[0].ending.is_some()
    }

    /// Whether cup `cup` takes a ball on this tick: its time has come and
    /// nobody has taken it.
    pub fn cup_open(&self, cup: usize) -> bool {
        self.tick >= CUP_OPENS[cup] * 60 && self.cup_taken_by[cup].is_none()
    }

    /// Whether the pad at `index` is open on this tick: past the opening time
    /// and with its centre in the zone.
    pub fn pad_open(&self, index: usize) -> bool {
        self.tick >= PADS_OPEN && inside_at(&self.schedule, self.pads[index], self.tick)
    }

    /// The pad `who` stands on, if it is open.
    pub fn pad_underfoot(&self, who: usize) -> Option<usize> {
        let at = self.golfers[who].pos;
        (0..2).find(|&index| self.pad_open(index) && (self.pads[index] - at).length() <= PAD_RADIUS)
    }

    /// The pickup `who` stands on, if any.
    pub fn pickup_underfoot(&self, who: usize) -> Option<usize> {
        let at = self.golfers[who].pos;
        self.pickups
            .iter()
            .position(|pickup| !pickup.taken && (pickup.pos - at).length() <= PICKUP_RADIUS)
    }

    /// Whether `who` stands close enough to its resting ball to play it.
    pub fn addressing(&self, who: usize) -> bool {
        let golfer = &self.golfers[who];
        golfer.ball.vel == Vec2::ZERO && (golfer.ball.pos - golfer.pos).length() <= rules::ADDRESS
    }

    /// End `who`'s match with `ending`, keeping what `settle` says.
    fn end(&mut self, who: usize, ending: Ending) {
        if self.golfers[who].ending.is_some() {
            return;
        }
        let kept = rules::settle(self, who, ending);
        self.golfers[who].ending = Some((ending, kept, self.tick));
        self.log.push((self.tick, Event::Ended { who, ending }));
    }
}

/// The angle from `from` to `to`.
pub fn rules_aim_toward(from: Vec2, to: Vec2) -> Radians {
    let d = to - from;
    atan2(d.y, d.x)
}

/// One tick of the match. `intents[i]` is golfer `i`'s.
pub fn step(game: &mut Match, intents: &[Intent]) {
    if game.over() {
        return;
    }
    game.tick += 1;
    // Every clock runs down before anyone acts, so a stun lasts its full count
    // whichever golfer dealt it.
    for golfer in &mut game.golfers {
        golfer.stun = golfer.stun.saturating_sub(1);
        golfer.guard = golfer.guard.saturating_sub(1);
        golfer.cooldown = golfer.cooldown.saturating_sub(1);
        golfer.lock = golfer.lock.saturating_sub(1);
    }
    for who in 0..game.golfers.len() {
        let intent = intents.get(who).copied().unwrap_or_default();
        act(game, who, intent);
    }
    roll_the_balls(game);
    judge_the_zone(game);
    settle_the_endings(game);
}

/// Apply one golfer's intent, in a fixed order: aim, walk, take, contact,
/// shoot, extract.
fn act(game: &mut Match, who: usize, intent: Intent) {
    if !rules::free_to_act(&game.golfers[who]) {
        return;
    }
    let tick = game.tick;
    {
        let golfer = &mut game.golfers[who];
        golfer.aim =
            Radians(golfer.aim.0 + intent.turn.clamp(-1.0, 1.0) * TURN_RATE.to_radians() * DT);
        golfer.power =
            (golfer.power + intent.power.clamp(-1.0, 1.0) * POWER_RATE * DT).clamp(MIN_POWER, 1.0);
        let walk = intent.walk.clamp_length_max(1.0);
        golfer.pos = keep_on_course(golfer.pos + walk * WALK_SPEED * DT, 0.0);
    }
    if intent.take
        && let Some(index) = game.pickup_underfoot(who)
    {
        let item = game.pickups[index].item;
        game.pickups[index].taken = true;
        game.golfers[who].item = Some(item);
        game.log.push((tick, Event::Took { who, item }));
    }
    if intent.contact
        && let Some(contact) = rules::contact_for(game, who)
    {
        game.golfers[who].cooldown = CONTACT_COOLDOWN;
        match contact {
            Contact::Club {
                target,
                stun_ticks,
                steal,
            } => {
                game.golfers[who].lock = SWING_LOCK;
                game.golfers[target].stun = stun_ticks;
                game.golfers[target].guard = stun_ticks + rules::GUARD_TICKS;
                game.golfers[target].stash -= steal;
                game.golfers[who].stash += steal;
                game.golfers[target].last_hit = Some((who, tick));
                game.log.push((
                    tick,
                    Event::Club {
                        by: who,
                        target,
                        stolen: steal,
                    },
                ));
            }
            Contact::Strike { owner, vel, .. } => {
                game.golfers[owner].ball.vel = vel;
                game.golfers[owner].last_hit = Some((who, tick));
                game.log.push((tick, Event::Strike { by: who, owner }));
            }
        }
        return;
    }
    if intent.shoot && game.addressing(who) {
        let golfer = &mut game.golfers[who];
        let from = golfer.ball.pos;
        let planned = rules::landing(from, golfer.aim, golfer.power, golfer.item);
        let wobble = (
            game.rng.next_f32() * 2.0 - 1.0,
            game.rng.next_f32() * 2.0 - 1.0,
        );
        let golfer = &mut game.golfers[who];
        golfer.ball.vel = rules::struck(golfer.aim, golfer.power, golfer.item, wobble);
        golfer.shots += 1;
        game.log.push((tick, Event::Shot { who, from, planned }));
    }
    if intent.extract && game.pad_underfoot(who).is_some() {
        game.end(who, Ending::Extracted);
    }
}

/// Every moving ball rolls one tick; a slow ball over an open cup drops in,
/// and the cup closes.
fn roll_the_balls(game: &mut Match) {
    let tick = game.tick;
    for who in 0..game.golfers.len() {
        if game.golfers[who].ending.is_some() || game.golfers[who].ball.vel == Vec2::ZERO {
            continue;
        }
        let ball = rules::roll_step(game.golfers[who].ball);
        game.golfers[who].ball = ball;
        let slow = ball.vel.length() < CUP_SPEED;
        let cup = (0..CUPS)
            .find(|&cup| game.cup_open(cup) && (game.cups[cup] - ball.pos).length() <= CUP_RADIUS);
        if slow && let Some(cup) = cup {
            let golfer = &mut game.golfers[who];
            golfer.ball = Ball {
                pos: game.cups[cup],
                vel: Vec2::ZERO,
            };
            golfer.holes += 1;
            golfer.stash += CUP_TOKENS;
            game.cup_taken_by[cup] = Some(who);
            game.log.push((tick, Event::Sunk { who, cup }));
        }
    }
}

/// Count each golfer's ticks outside the zone and eliminate anyone past the
/// grace; a recent hitter collects the stash.
fn judge_the_zone(game: &mut Match) {
    let tick = game.tick;
    for who in 0..game.golfers.len() {
        if game.golfers[who].ending.is_some() {
            continue;
        }
        let golfer = &game.golfers[who];
        let safe = inside_at(&game.schedule, golfer.pos, tick)
            && inside_at(&game.schedule, golfer.ball.pos, tick);
        let golfer = &mut game.golfers[who];
        golfer.out_ticks = if safe { 0 } else { golfer.out_ticks + 1 };
        if golfer.out_ticks <= GRACE_TICKS {
            continue;
        }
        let last_hit = golfer.last_hit;
        let hitter = last_hit
            .filter(|&(_, when)| tick - when <= BOUNTY_TICKS)
            .map(|(by, _)| by)
            .filter(|&by| game.golfers[by].ending.is_none());
        let bounty = game.golfers[who].stash;
        game.golfers[who].stash = 0;
        if let Some(by) = hitter {
            game.golfers[by].stash += bounty;
        }
        game.log.push((
            tick,
            Event::Eliminated {
                who,
                to: hitter,
                bounty: if hitter.is_some() { bounty } else { 0 },
            },
        ));
        game.end(who, Ending::Eliminated);
    }
}

/// A champion ends everyone else's match; so does being the last one in.
fn settle_the_endings(game: &mut Match) {
    let champion = (0..game.golfers.len()).find(|&who| {
        game.golfers[who].ending.is_none() && game.golfers[who].holes >= CHAMPION_HOLES
    });
    if let Some(champion) = champion {
        game.end(champion, Ending::Champion);
        for who in 0..game.golfers.len() {
            game.end(who, Ending::Outlasted);
        }
        return;
    }
    let left: Vec<usize> = (0..game.golfers.len())
        .filter(|&who| game.golfers[who].ending.is_none())
        .collect();
    let anyone_out = game
        .golfers
        .iter()
        .any(|golfer| matches!(golfer.ending, Some((Ending::Eliminated, _, _))));
    if let [last] = left[..]
        && anyone_out
    {
        game.end(last, Ending::LastStanding);
    }
}
