//! The NPC golfers: one pure function from the match to an intent.
//!
//! `npc_intent` reads nothing but the `Match` it is handed and the noise it is
//! given, so a `--verify` player can borrow it (the good player is a Banker
//! with no noise) and a check can ask what an NPC is about to do.

use jidousha::prelude::*;

use crate::events::Intent;
use crate::rules::{self, ADDRESS, Contact, PAD_RADIUS, PICKUP_RADIUS, REACH};
use crate::sim::{Brain, Match, rules_aim_toward};
use crate::zone::{next_zone, zone_at};

/// How far off an NPC's aim may be, in degrees either way.
pub const AIM_NOISE_DEGREES: f32 = 9.0;

/// How far an NPC will walk out of its way for equipment.
const ITEM_DETOUR: f32 = 7.0;

/// A walk toward `target`, or nothing if already there.
fn toward(from: Vec2, target: Vec2, close: f32) -> Vec2 {
    let d = target - from;
    if d.length() <= close {
        Vec2::ZERO
    } else {
        d.normalize_or_zero()
    }
}

/// How many tokens a brain wants before it extracts; `None` never extracts
/// early (it still extracts when the zone is late).
fn extract_threshold(brain: Brain) -> Option<u32> {
    match brain {
        Brain::Banker => Some(3),
        Brain::Player => Some(3),
        Brain::Brute => Some(4),
        Brain::Rover => Some(6),
        Brain::Holer | Brain::Idle => None,
    }
}

/// Whether a brain takes a contact the cue offers.
fn takes_contact(brain: Brain, contact: Contact, me: usize, game: &Match) -> bool {
    match brain {
        Brain::Brute => true,
        // Everyone else strikes only the leader's ball, and clubs only to
        // steal.
        Brain::Holer | Brain::Rover | Brain::Banker | Brain::Player => match contact {
            Contact::Club { steal, .. } => steal > 0,
            Contact::Strike { owner, .. } => {
                game.golfers[owner].holes > game.golfers[me].holes
                    || game.golfers[owner].stash > game.golfers[me].stash
            }
        },
        Brain::Idle => false,
    }
}

/// The open pad nearest `from`, if any.
fn nearest_open_pad(game: &Match, from: Vec2) -> Option<Vec2> {
    (0..2)
        .filter(|&index| game.pad_open(index))
        .map(|index| game.pads[index])
        .min_by(|a, b| (*a - from).length().total_cmp(&(*b - from).length()))
}

/// The shot an NPC wants from where its ball lies: an aim and a power.
pub fn plan_shot(game: &Match, who: usize) -> (Radians, f32) {
    let golfer = &game.golfers[who];
    let ball = golfer.ball.pos;
    let reach = REACH * rules::effect(golfer.item).reach_scale;
    let (next, _) = next_zone(&game.schedule, game.tick)
        .unwrap_or((zone_at(&game.schedule, game.tick), game.tick));
    // Keep a margin inside the next ring, so the ball is safe when it lands.
    let safe = (next.radius - 2.0).max(0.0);
    let to_center = next.center - ball;
    if to_center.length() > safe {
        let distance = (to_center.length() - safe * 0.5).min(reach);
        return (rules_aim_toward(ball, next.center), distance / reach);
    }
    // Safe already: play the nearest open cup that is safe too.
    let cup = (0..game.cups.len())
        .filter(|&cup| game.cup_open(cup) && next.contains(game.cups[cup]))
        .min_by(|&a, &b| {
            (game.cups[a] - ball)
                .length()
                .total_cmp(&(game.cups[b] - ball).length())
        });
    match cup {
        Some(cup) => {
            let distance = (game.cups[cup] - ball).length().min(reach);
            (rules_aim_toward(ball, game.cups[cup]), distance / reach)
        }
        // Nothing to play for: stay put.
        None => (golfer.aim, 0.0),
    }
}

/// A number in -1..1 fixed for one golfer's one shot, so an NPC's aim error
/// holds still while it lines up rather than changing every tick.
pub fn shot_noise(seed: u64, who: usize, shots: u32) -> f32 {
    let mut x = seed
        ^ (who as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::from(shots).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    (x % 2001) as f32 / 1000.0 - 1.0
}

/// The difference `to - from`, wrapped into a half turn either way.
pub fn angle_between(from: Radians, to: Radians) -> f32 {
    let tau = core::f32::consts::TAU;
    let d = (to.0 - from.0) % tau;
    if d > tau * 0.5 {
        d - tau
    } else if d < -tau * 0.5 {
        d + tau
    } else {
        d
    }
}

/// What NPC `who` does this tick, with its aim error scaled by `noise_scale`
/// (1 for the game's NPCs, 0 for a perfect player).
pub fn npc_intent(game: &Match, who: usize, noise_scale: f32) -> Intent {
    let golfer = &game.golfers[who];
    let noise = shot_noise(game.seed, who, golfer.shots) * noise_scale;
    if golfer.brain == Brain::Idle || !rules::free_to_act(golfer) {
        return Intent::default();
    }
    if let Some(contact) = rules::contact_for(game, who)
        && takes_contact(golfer.brain, contact, who, game)
    {
        return Intent {
            contact: true,
            ..Intent::default()
        };
    }
    // Extract when there is enough to keep, or when the zone is getting late.
    let late = zone_at(&game.schedule, game.tick).radius < 10.0;
    let wants_out = extract_threshold(golfer.brain).is_some_and(|need| golfer.stash >= need)
        || (late
            && golfer.brain != Brain::Holer
            && (golfer.stash > 0 || golfer.brain == Brain::Player));
    if wants_out && let Some(pad) = nearest_open_pad(game, golfer.pos) {
        let on = (pad - golfer.pos).length() <= PAD_RADIUS * 0.8;
        return Intent {
            walk: toward(golfer.pos, pad, PAD_RADIUS * 0.5),
            extract: on,
            ..Intent::default()
        };
    }
    // Equipment, if it holds none and some is near.
    if golfer.item.is_none()
        && let Some(pickup) = game
            .pickups
            .iter()
            .filter(|pickup| !pickup.taken && (pickup.pos - golfer.pos).length() < ITEM_DETOUR)
            .min_by(|a, b| {
                (a.pos - golfer.pos)
                    .length()
                    .total_cmp(&(b.pos - golfer.pos).length())
            })
    {
        let on = (pickup.pos - golfer.pos).length() <= PICKUP_RADIUS * 0.8;
        return Intent {
            walk: toward(golfer.pos, pickup.pos, PICKUP_RADIUS * 0.5),
            take: on,
            ..Intent::default()
        };
    }
    // Otherwise play the ball: walk to it, line up, shoot.
    if !game.addressing(who) || (golfer.ball.pos - golfer.pos).length() > ADDRESS * 0.7 {
        return Intent {
            walk: toward(golfer.pos, golfer.ball.pos, ADDRESS * 0.5),
            ..Intent::default()
        };
    }
    let (aim, power) = plan_shot(game, who);
    if power <= 0.0 {
        return Intent::default();
    }
    let aim = Radians(aim.0 + (AIM_NOISE_DEGREES * noise).to_radians());
    let turn = angle_between(golfer.aim, aim) / (crate::sim::TURN_RATE.to_radians() * rules::DT);
    let push = (power - golfer.power) / (crate::sim::POWER_RATE * rules::DT);
    let lined_up = turn.abs() <= 1.0 && push.abs() <= 1.0;
    Intent {
        turn: turn.clamp(-1.0, 1.0),
        power: push.clamp(-1.0, 1.0),
        shoot: lined_up,
        ..Intent::default()
    }
}
