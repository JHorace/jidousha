//! The contact half of the rules: the equipment table and its effects, the
//! golfers' read-only views, reach, and what a golfer banks.
//!
//! Re-exported whole from `rules`, which is the one path the game names them
//! by; this file exists only to keep `rules.rs` under the size convention.

use jidousha::prelude::*;

use super::{Landing, shot_speed};

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
