//! The rules as free functions: the ball's roll, the shot, contact, equipment
//! and what an ending keeps.
//!
//! Each is the single place its question is answered (DESIGN.md, Systems):
//! the aim line draws `landing`, the cue draws `contact_for`, the pickup text
//! is formatted from `effect`, and the status bar and the result screen both
//! print `settle`. The sim in `sim.rs` calls the same functions.

use jidousha::prelude::*;

use crate::sim::{Ball, Golfer, Match};
use crate::zone::TICKS_PER_SECOND;

/// The course, in world units: where golfers may walk and balls may roll.
pub const COURSE: Rect = Rect {
    min: Vec2::new(-31.0, -15.0),
    max: Vec2::new(31.0, 15.0),
};

/// How hard a rolling ball slows, in world units per second squared.
pub const DECEL: f32 = 12.0;

/// How far a full-power shot rolls, before equipment.
pub const REACH: f32 = 14.0;

/// How close a golfer must stand to its ball to play it.
pub const ADDRESS: f32 = 1.5;

/// How close a rival must be to be clubbed.
pub const CLUB_REACH: f32 = 2.0;

/// How close a rival's ball must be to be struck.
pub const STRIKE_REACH: f32 = 1.6;

/// How far a struck ball rolls, before equipment.
pub const KNOCK: f32 = 9.0;

/// How long a club stuns, before equipment: three seconds.
pub const STUN_TICKS: u32 = 180;

/// How long after a stun ends a golfer cannot be clubbed again: two seconds,
/// so nobody is stunned for good.
pub const GUARD_TICKS: u32 = 120;

/// How long a golfer must wait between two contacts: one second.
pub const CONTACT_COOLDOWN: u32 = 60;

/// How long a club roots the golfer swinging it.
pub const SWING_LOCK: u32 = 20;

/// How many tokens a club takes.
pub const STEAL: u32 = 1;

/// How many tokens a cup is worth.
pub const CUP_TOKENS: u32 = 3;

/// How many cups make a champion.
pub const CHAMPION_HOLES: u32 = 3;

/// What a win adds to the stash.
pub const WIN_BONUS: u32 = 5;

/// How close to a cup a slow ball drops in.
pub const CUP_RADIUS: f32 = 0.5;

/// How slow a ball must be to drop in.
pub const CUP_SPEED: f32 = 3.0;

/// How long a hit makes you the victim's last hitter: ten seconds.
pub const BOUNTY_TICKS: u64 = 600;

/// How long a golfer or its ball may be outside the zone: five seconds.
pub const GRACE_TICKS: u32 = 300;

/// When the extraction pads open: 32 seconds in.
pub const PADS_OPEN: u64 = 32 * TICKS_PER_SECOND;

/// How close to a pad's centre counts as standing on it.
pub const PAD_RADIUS: f32 = 1.5;

/// How close to a pickup counts as standing on it.
pub const PICKUP_RADIUS: f32 = 1.2;

/// One tick, in seconds.
pub const DT: f32 = 1.0 / 60.0;

/// The three pieces of equipment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// Your ball is hard to move.
    Heavy,
    /// Clubs hurt you less.
    Helmet,
    /// Your shots go further.
    Driver,
}

impl Item {
    /// The name drawn on the course and in the status bar.
    pub fn name(self) -> &'static str {
        match self {
            Item::Heavy => "HEAVY",
            Item::Helmet => "HELMET",
            Item::Driver => "DRIVER",
        }
    }
}

/// What holding something does. The one equipment-effect answer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    /// How far your ball rolls when a rival strikes it, as a share of `KNOCK`.
    pub knock_scale: f32,
    /// How long a club stuns you, in ticks.
    pub stun_ticks: u32,
    /// How many tokens a club takes from you.
    pub steal: u32,
    /// How far your shots reach, as a share of `REACH`.
    pub reach_scale: f32,
}

/// What `item` does to the golfer holding it — the sim and the description
/// both read this.
pub fn effect(item: Option<Item>) -> Effect {
    let bare = Effect {
        knock_scale: 1.0,
        stun_ticks: STUN_TICKS,
        steal: STEAL,
        reach_scale: 1.0,
    };
    match item {
        None => bare,
        Some(Item::Heavy) => Effect {
            knock_scale: 0.25,
            ..bare
        },
        Some(Item::Helmet) => Effect {
            stun_ticks: 60,
            steal: 0,
            ..bare
        },
        Some(Item::Driver) => Effect {
            reach_scale: 1.5,
            ..bare
        },
    }
}

/// What a pickup says it does, written from `effect` so it cannot drift.
pub fn describe(item: Item) -> String {
    let it = effect(Some(item));
    match item {
        Item::Heavy => format!(
            "HEAVY: a struck ball rolls {:.2} not {KNOCK:.1}",
            KNOCK * it.knock_scale
        ),
        Item::Helmet => format!(
            "HELMET: clubbed = stun {:.1}s, lose {} tokens",
            it.stun_ticks as f32 / 60.0,
            it.steal
        ),
        Item::Driver => format!(
            "DRIVER: shots reach {:.0} not {REACH:.0}",
            REACH * it.reach_scale
        ),
    }
}

/// One tick of a rolling ball. A step travels the exact area under the
/// speed, so a ball launched at `v` comes to rest `v * v / (2 * DECEL)` away
/// unless the course edge stops it first.
pub fn roll_step(ball: Ball) -> Ball {
    let speed = ball.vel.length();
    // Written positive and negated whole, so a NaN speed stops the ball.
    let moving = speed > 0.0;
    if !moving {
        return Ball {
            vel: Vec2::ZERO,
            ..ball
        };
    }
    let direction = ball.vel / speed;
    let slowing = DECEL * DT;
    let (travel, after) = if speed <= slowing {
        (speed * speed / (2.0 * DECEL), 0.0)
    } else {
        ((speed - slowing * 0.5) * DT, speed - slowing)
    };
    let pos = ball.pos + direction * travel;
    let kept = Vec2::new(
        pos.x.clamp(COURSE.min.x, COURSE.max.x),
        pos.y.clamp(COURSE.min.y, COURSE.max.y),
    );
    if kept != pos {
        return Ball {
            pos: kept,
            vel: Vec2::ZERO,
        };
    }
    Ball {
        pos,
        vel: direction * after,
    }
}

/// The launch speed that rolls a ball `distance` units.
pub fn launch_speed(distance: f32) -> f32 {
    (2.0 * DECEL * distance.max(0.0)).sqrt()
}

/// A shot at `aim` and `power`, as a velocity, for a golfer holding `item`.
pub fn shot_velocity(aim: Radians, power: f32, item: Option<Item>) -> Vec2 {
    let (sin, cos) = sin_cos(aim);
    let distance = REACH * effect(item).reach_scale * power.clamp(0.0, 1.0);
    Vec2::new(cos, sin) * launch_speed(distance)
}

/// How far a shot may stray, at full power: degrees either side of the aim.
pub const SPREAD_DEGREES: f32 = 8.0;

/// How far a shot may stray, at full power: a share of its length either way.
pub const SPREAD_LENGTH: f32 = 0.12;

/// A shot as it actually leaves the club: the aimed shot turned by up to
/// `SPREAD_DEGREES` and lengthened or shortened by up to `SPREAD_LENGTH`,
/// both scaled by power, by `wobble` (each -1..1, drawn by the match).
pub fn struck(aim: Radians, power: f32, item: Option<Item>, wobble: (f32, f32)) -> Vec2 {
    let power = power.clamp(0.0, 1.0);
    let turned = Radians(aim.0 + (SPREAD_DEGREES * power * wobble.0).to_radians());
    let length = 1.0 + SPREAD_LENGTH * power * wobble.1;
    shot_velocity(turned, power, item) * length.sqrt()
}

/// How far from the aimed landing a shot can stop: the landing area's radius.
pub fn spread_radius(power: f32, item: Option<Item>) -> f32 {
    let power = power.clamp(0.0, 1.0);
    let length = REACH * effect(item).reach_scale * power;
    let (sin, _) = sin_cos(Radians((SPREAD_DEGREES * power).to_radians()));
    length * sin.max(SPREAD_LENGTH * power)
}

/// Where a ball comes to rest. The aim line draws this and the sim rolls the
/// same steps.
pub fn rest_of(ball: Ball) -> Vec2 {
    let mut ball = ball;
    for _ in 0..2000 {
        if ball.vel == Vec2::ZERO {
            break;
        }
        ball = roll_step(ball);
    }
    ball.pos
}

/// Where a shot from `from` would stop.
pub fn landing(from: Vec2, aim: Radians, power: f32, item: Option<Item>) -> Vec2 {
    rest_of(Ball {
        pos: from,
        vel: shot_velocity(aim, power, item),
    })
}

/// A contact a golfer could make right now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Contact {
    /// Club golfer `target`: they are stunned `stun_ticks` and lose `steal`.
    Club {
        /// Who would be clubbed.
        target: usize,
        /// For how long.
        stun_ticks: u32,
        /// How many tokens move to the clubber.
        steal: u32,
    },
    /// Strike golfer `owner`'s ball: it rolls to `to`.
    Strike {
        /// Whose ball.
        owner: usize,
        /// The velocity it is given.
        vel: Vec2,
        /// Where it will stop.
        to: Vec2,
    },
}

/// Whether `golfer` can act this tick at all.
pub fn free_to_act(golfer: &Golfer) -> bool {
    golfer.ending.is_none() && golfer.stun == 0 && golfer.lock == 0
}

/// The contact `who` would make by pressing the contact input now — the reach
/// cue draws this and the sim applies it.
pub fn contact_for(game: &Match, who: usize) -> Option<Contact> {
    let me = &game.golfers[who];
    if !free_to_act(me) || me.cooldown > 0 {
        return None;
    }
    let mut club: Option<(f32, usize)> = None;
    for (index, rival) in game.golfers.iter().enumerate() {
        if index == who || rival.ending.is_some() || rival.guard > 0 {
            continue;
        }
        let distance = (rival.pos - me.pos).length();
        if distance <= CLUB_REACH && club.is_none_or(|(best, _)| distance < best) {
            club = Some((distance, index));
        }
    }
    if let Some((_, target)) = club {
        let it = effect(game.golfers[target].item);
        return Some(Contact::Club {
            target,
            stun_ticks: it.stun_ticks,
            steal: it.steal.min(game.golfers[target].stash),
        });
    }
    let mut strike: Option<(f32, usize)> = None;
    for (index, rival) in game.golfers.iter().enumerate() {
        if index == who || rival.ending.is_some() || rival.ball.vel != Vec2::ZERO {
            continue;
        }
        let distance = (rival.ball.pos - me.pos).length();
        if distance <= STRIKE_REACH && strike.is_none_or(|(best, _)| distance < best) {
            strike = Some((distance, index));
        }
    }
    let (_, owner) = strike?;
    let ball = game.golfers[owner].ball;
    let away = ball.pos - me.pos;
    let direction = if away.length() > 1e-4 {
        away / away.length()
    } else {
        Vec2::new(1.0, 0.0)
    };
    let distance = KNOCK * effect(game.golfers[owner].item).knock_scale;
    let vel = direction * launch_speed(distance);
    Some(Contact::Strike {
        owner,
        vel,
        to: rest_of(Ball { pos: ball.pos, vel }),
    })
}

/// How a golfer's match ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// Left from an open pad.
    Extracted,
    /// Sank three cups first.
    Champion,
    /// Every rival was eliminated or left.
    LastStanding,
    /// Outside the zone past the grace.
    Eliminated,
    /// Still in when a rival won.
    Outlasted,
}

impl Ending {
    /// The word the result screen uses.
    pub fn name(self) -> &'static str {
        match self {
            Ending::Extracted => "EXTRACTED",
            Ending::Champion => "CHAMPION",
            Ending::LastStanding => "LAST STANDING",
            Ending::Eliminated => "ELIMINATED",
            Ending::Outlasted => "OUTLASTED",
        }
    }
}

/// What a golfer takes out of the match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kept {
    /// Tokens.
    pub tokens: u32,
    /// The piece of equipment, if any.
    pub item: Option<Item>,
}

impl Kept {
    /// One line, the same on the status bar and the result screen.
    pub fn line(self) -> String {
        match self.item {
            Some(item) => format!("{} tokens + {}", self.tokens, item.name()),
            None => format!("{} tokens", self.tokens),
        }
    }
}

/// What `who` keeps if the match ends for them by `ending` now. The status
/// bar and the result screen both read this.
pub fn settle(game: &Match, who: usize, ending: Ending) -> Kept {
    let golfer = &game.golfers[who];
    match ending {
        Ending::Extracted => Kept {
            tokens: golfer.stash,
            item: golfer.item,
        },
        Ending::Champion | Ending::LastStanding => Kept {
            tokens: golfer.stash + WIN_BONUS,
            item: golfer.item,
        },
        Ending::Eliminated | Ending::Outlasted => Kept {
            tokens: 0,
            item: None,
        },
    }
}
