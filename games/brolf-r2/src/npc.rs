//! The NPC policy: one pure function from the snapshot to an intent.
//!
//! The NPCs play by the player's rules through the player's functions —
//! `in_reach`, `landing_of`, `zone_at`, `effects`, `bank_now` — so anything
//! they do, the player could have done. They walk to their balls, golf at cups
//! that lie in the next zone, rescue a ball the zone is about to leave behind,
//! loot nearby pickups, club a rival in reach who holds something or is
//! already outside, and extract once their bank reaches a threshold or the
//! third phase begins.
//!
//! DELIBERATE: NPCs set their aim instantly while the player's turns at 2.5
//! degrees a tick. The asymmetry keeps this policy pure (no aim to steer
//! toward across ticks) and the player's input one key per tick; the verify
//! `Planner` drives this same policy through the arrows and reports what the
//! turn costs it (DESIGN.md "Decisions already made").

use jidousha::prelude::*;

use crate::rules::{
    EXTRACT_PHASE, Landing, atan_to, bank_now, charge_for, course, effects, landing_of, phase_of,
    polar, shot_speed_for, walk_ticks, zone_at,
};
use crate::sim::{Aim, Intent, Snapshot};

/// The bank at which each seat heads for the pad once it is open (seat 0's is
/// the verify Planner's).
pub const EXTRACT_AT: [u32; 4] = [7, 6, 9, 5];
/// How far ahead the NPC looks at the zone when deciding what is safe, in ticks.
const LOOKAHEAD_TICKS: u64 = 120;
/// How far a pickup may be for an NPC to walk to it.
const LOOT_RANGE: f32 = 4.0;

/// What an NPC means to do, and the shot it is lining up, if any.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    pub intent: Intent,
    /// The point the shot is aimed at.
    pub target: Option<Vec2>,
    /// Where the planned shot lands, at the planned aim and charge.
    pub landing: Option<Landing>,
    /// The charge the shot is released at.
    pub want: u32,
}

impl Plan {
    fn walk(from: Vec2, to: Vec2) -> Self {
        Plan {
            intent: Intent {
                walk: (to - from).normalize_or_zero(),
                ..Intent::IDLE
            },
            target: None,
            landing: None,
            want: 0,
        }
    }

    fn idle() -> Self {
        Plan {
            intent: Intent::IDLE,
            target: None,
            landing: None,
            want: 0,
        }
    }
}

/// What `seat` does this tick.
pub fn decide(snap: &Snapshot, seat: usize) -> Intent {
    plan(snap, seat).intent
}

/// What `seat` does this tick, with the shot behind it.
pub fn plan(snap: &Snapshot, seat: usize) -> Plan {
    let me = snap.golfer(seat);
    if me.fate != crate::rules::Fate::Playing {
        return Plan::idle();
    }
    let reach = snap.reach(seat);
    let ball = snap.ball(seat);
    // 2. club a rival who holds something or is already outside.
    if let Some(target) = reach.club
        && me.cooldown == 0
    {
        let victim = snap.golfer(target);
        if !victim.held.is_empty() || victim.out_ticks > 0 {
            let mut plan = Plan::idle();
            plan.intent.club = true;
            return plan;
        }
    }
    let soon = zone_at(&snap.course, snap.tick + LOOKAHEAD_TICKS);
    // 3. extract.
    let pad = snap.course.pad;
    let bank = bank_now(me.points, &me.held);
    if snap.pad_open && (bank >= EXTRACT_AT[seat] || phase_of(snap.tick).0 >= EXTRACT_PHASE) {
        if pad.contains(me.pos) && ball.at_rest && pad.contains(ball.pos) {
            let mut plan = Plan::idle();
            plan.intent.extract = true;
            return plan;
        }
        if !(ball.at_rest && pad.contains(ball.pos)) {
            return shot_at(snap, seat, pad.center, false);
        }
        return Plan::walk(me.pos, pad.center);
    }
    // 4. rescue a ball the zone is leaving behind.
    let ball_end = ball.landing.map_or(ball.pos, |landing| landing.at);
    if !soon.contains(ball_end) {
        let to = snap.next.map_or(snap.zone.center, |zone| zone.center);
        return shot_at(snap, seat, to, true);
    }
    // 5. loot.
    if me.held.len() < crate::rules::SLOTS {
        let near = snap
            .pickups
            .iter()
            .enumerate()
            .filter(|(_, p)| p.pos.distance(me.pos) <= LOOT_RANGE && soon.contains(p.pos))
            .min_by(|(_, a), (_, b)| a.pos.distance(me.pos).total_cmp(&b.pos.distance(me.pos)));
        if let Some((index, pickup)) = near {
            if reach.pickup == Some(index) {
                let mut plan = Plan::idle();
                plan.intent.take = true;
                return plan;
            }
            return Plan::walk(me.pos, pickup.pos);
        }
    }
    // 6. golf at the cup nearest the ball, among those in the next zone.
    let target_zone = snap.next.unwrap_or(snap.zone);
    let mut cups: Vec<Vec2> = snap
        .course
        .cups
        .iter()
        .copied()
        .filter(|cup| target_zone.contains(*cup))
        .collect();
    if cups.is_empty() {
        cups = snap.course.cups.to_vec();
    }
    if let Some(cup) = cups
        .into_iter()
        .min_by(|a, b| a.distance(ball.pos).total_cmp(&b.distance(ball.pos)))
    {
        return shot_at(snap, seat, cup, true);
    }
    // 7. head for the safe middle.
    Plan::walk(me.pos, soon.center)
}

/// Line up and play a shot at `target`: walk to the ball, set the aim, charge
/// to the planned strength and release. With `safe`, the charge is lowered
/// until the ball lands inside the zone it will meet when it stops and the
/// golfer has walked to it — and aimed at the zone's centre if no charge does.
fn shot_at(snap: &Snapshot, seat: usize, target: Vec2, safe: bool) -> Plan {
    let me = snap.golfer(seat);
    let ball = snap.ball(seat);
    if !ball.at_rest {
        let to = ball.landing.map_or(ball.pos, |landing| landing.at);
        return Plan::walk(me.pos, to);
    }
    if snap.reach(seat).ball != Some(seat) {
        return Plan::walk(me.pos, ball.pos);
    }
    let mine = effects(&me.held);
    let fence = course();
    let land = |aim: Radians, charge: u32| {
        let speed = shot_speed_for(&mine, &mine, true, charge);
        landing_of(ball.pos, polar(1.0, aim), speed, fence, snap.dt)
    };
    let lands_safe = |landing: &Landing| {
        let walk = walk_ticks(landing.at.distance(me.pos), mine.walk_scale, snap.dt);
        zone_at(&snap.course, snap.tick + landing.ticks + walk).contains(landing.at)
    };
    let mut aim = atan_to(ball.pos, target);
    let mut want = charge_for(ball.pos.distance(target), mine.shot_scale, snap.dt);
    if safe {
        while want > 1 && !lands_safe(&land(aim, want)) {
            want -= 1;
        }
        if !lands_safe(&land(aim, want)) {
            let center = snap.next.map_or(snap.zone.center, |zone| zone.center);
            aim = atan_to(ball.pos, center);
            want = charge_for(ball.pos.distance(center), mine.shot_scale, snap.dt);
        }
    }
    let landing = land(aim, want);
    Plan {
        intent: Intent {
            aim: Aim::Set(aim),
            space_held: me.charge < want,
            ..Intent::IDLE
        },
        target: Some(target),
        landing: Some(landing),
        want,
    }
}
