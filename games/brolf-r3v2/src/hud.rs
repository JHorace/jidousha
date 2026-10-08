//! Every string the game draws: six HUD lines, the result banner, the legend.
//!
//! `hud_lines` is the one reader Draw and the checks both call, over the same
//! `Snap` the systems read, so what a line says is what the sim will do
//! (DESIGN.md, Systems: hud). Each line answers one decision row: 1 and 4 the
//! zone, 5 contact, 6 equipment, 2 and 3 extraction.

use jidousha::prelude::*;

use crate::model::*;
use crate::rules::*;
use crate::world::{Snap, snap};

/// The key legend under the course.
pub const LEGEND: &str =
    "WASD walk  Left/Right aim  Up/Down power  Space shoot  C contact  F take  E extract";

/// Whole seconds from `now` to `then`, rounded up.
fn seconds_until(now: u64, then: u64, dt: Seconds) -> u64 {
    (then.saturating_sub(now) as f32 * dt.as_f32()).ceil() as u64
}

/// The six HUD lines, read off the world.
pub fn hud_lines(view: &WorldView<'_>) -> [String; 6] {
    hud_lines_of(&snap(view))
}

/// The six HUD lines for a `Snap`.
pub fn hud_lines_of(view: &Snap) -> [String; 6] {
    let Some(me) = view.golfer(0).copied() else {
        return Default::default();
    };
    [
        zone_line(view, &me),
        kit_line(view, &me),
        stakes_line(&me),
        aim_line(view, &me),
        contact_line(view, &me),
        pickup_line(view, &me),
    ]
}

/// Line 1: the zone now and next, my grace, the gate.
fn zone_line(view: &Snap, me: &GolferSnap) -> String {
    let now = zone_at(view.now);
    let next = ZONE_TABLE
        .iter()
        .find(|(tick, radius)| *tick > view.now && (*radius - now.radius).abs() > 1e-4);
    let (next_r, next_s) = next.map_or((0.0, 0), |(tick, radius)| {
        (*radius, seconds_until(view.now, *tick, view.dt))
    });
    let grace = if me.outside_ticks == 0 {
        "ok".to_owned()
    } else {
        let left = GRACE_TICKS.saturating_sub(me.outside_ticks) as f32 * view.dt.as_f32();
        format!("{left:.1}s")
    };
    let gate = match zone_excludes_at(GATE_CENTER) {
        Some(tick) if tick > view.now => {
            format!("open {}s", seconds_until(view.now, tick, view.dt))
        }
        Some(_) => "closed".to_owned(),
        None => "open".to_owned(),
    };
    format!(
        "ZONE r={:.1} next r={next_r:.1} in {next_s}s | GRACE {grace} | GATE {gate}",
        now.radius
    )
}

/// An item's name, or `-` for an empty slot.
fn slot(item: Option<Item>) -> &'static str {
    item.map_or("-", Item::name)
}

/// Line 2: my kit, my ball's distance to the cup, the rivals left.
fn kit_line(view: &Snap, me: &GolferSnap) -> String {
    let cup = view.ball(0).map_or(0.0, |ball| ball.pos.distance(CUP));
    let rivals = view
        .golfers
        .iter()
        .filter(|g| g.idx != 0 && g.alive)
        .count();
    let state = if cup_open(view.now + 1) {
        "open".to_owned()
    } else {
        format!("opens {}s", seconds_until(view.now, CUP_OPENS, view.dt))
    };
    format!(
        "KIT {}, {}, {} | cup {cup:.1}u ({state}) | {rivals} rivals",
        slot(me.kit.club),
        slot(me.kit.ball),
        slot(me.kit.head)
    )
}

/// Items joined by `joint`, or `nothing`.
pub fn names(items: &[Item], joint: &str) -> String {
    if items.is_empty() {
        return "nothing".to_owned();
    }
    items
        .iter()
        .map(|item| item.name())
        .collect::<Vec<_>>()
        .join(joint)
}

/// Line 3: what each way of ending would keep and score.
fn stakes_line(me: &GolferSnap) -> String {
    let extract = outcome_of(EndKind::Extracted, &me.kit);
    let hole = outcome_of(EndKind::Holed, &me.kit);
    format!(
        "EXTRACT keeps {} = {} | HOLE OUT = {} | LAST ONE = {}",
        names(&extract.kept, "+"),
        extract.points,
        hole.points,
        outcome_of(EndKind::LastStanding, &me.kit).points
    )
}

/// Where the aimed shot will stop, and when I will get there.
pub struct Aim {
    /// The roll.
    pub roll: RollOut,
    /// The tick I reach the ball afterwards.
    pub arrival: u64,
    /// The zone then.
    pub zone: Zone,
}

/// The shot I am aiming, if I am at my resting ball and free to shoot.
pub fn aiming(view: &Snap, me: &GolferSnap) -> Option<Aim> {
    let ball = view.ball(me.idx)?;
    let ready = view.result.is_none()
        && me.alive
        && me.stun_left == 0
        && me.cooldown == 0
        && ball.vel == Vec2::ZERO
        && me.pos.distance(ball.pos) <= SHOT_REACH;
    if !ready {
        return None;
    }
    let roll = roll_out(
        ball.pos,
        shot_velocity(me.aim, me.power, effects(&me.kit)),
        view.dt,
        view.now + 1,
    );
    let arrival = arrival_tick(view.now, me.pos, &roll, view.dt);
    Some(Aim {
        roll,
        arrival,
        zone: zone_at(arrival),
    })
}

/// Line 4: the shot, or why there is none.
fn aim_line(view: &Snap, me: &GolferSnap) -> String {
    let dt = view.dt.as_f32();
    if view.result.is_some() || !me.alive {
        return String::new();
    }
    if me.stun_left > 0 {
        return format!("DISABLED {:.1}s", me.stun_left as f32 * dt);
    }
    if me.cooldown > 0 {
        return format!("swing cooldown {:.1}s", me.cooldown as f32 * dt);
    }
    let Some(aim) = aiming(view, me) else {
        let away = view.ball(0).map_or(0.0, |ball| ball.pos.distance(me.pos));
        return format!("walk to your ball ({away:.1}u)");
    };
    let degrees = me.aim.to_degrees().round().rem_euclid(360.0);
    let lands = if aim.roll.sunk {
        "HOLES OUT".to_owned()
    } else {
        format!("rests {:.1}u from cup", aim.roll.rest.distance(CUP))
    };
    let safe = aim.roll.sunk || inside_zone(aim.zone, aim.roll.rest);
    format!(
        "AIM {degrees:.0}deg {} {lands}, {} zone at arrival (r={:.1} in {:.1}s)",
        me.power.name(),
        if safe { "inside" } else { "OUTSIDE" },
        aim.zone.radius,
        aim.arrival.saturating_sub(view.now) as f32 * dt
    )
}

/// Line 5: what C would do.
pub fn contact_line(view: &Snap, me: &GolferSnap) -> String {
    if view.result.is_some() || !me.alive {
        return String::new();
    }
    let Some(contact) = contact_target(me, &view.golfers, &view.balls, effects(&me.kit)) else {
        return String::new();
    };
    let (Contact::Club(idx) | Contact::Strike(idx)) = contact;
    let Some(target) = view.golfer(idx) else {
        return String::new();
    };
    let name = NAMES[usize::from(idx)];
    let result = resolve_contact(contact, me.aim, effects(&target.kit));
    match contact {
        Contact::Club(_) => {
            let drops = match ranked(&target.kit).first() {
                None => "nothing".to_owned(),
                Some(_) if !result.drops => "nothing (helmet)".to_owned(),
                Some(item) => item.name().to_owned(),
            };
            format!(
                "C: club {name} - disables {:.1}s, drops {drops}",
                result.stun_ticks as f32 * view.dt.as_f32()
            )
        }
        Contact::Strike(_) => match result.ball_vel {
            Some(_) => {
                format!("C: strike {name}'s ball - knocks it {STRIKE_SPEED:.0}u/s along your aim")
            }
            None => format!("C: strike {name}'s ball - no effect (lead ball)"),
        },
    }
}

/// Line 6: the nearest pickup, what it does, what it would replace.
fn pickup_line(view: &Snap, me: &GolferSnap) -> String {
    if view.result.is_some() || !me.alive {
        return String::new();
    }
    let nearest = view
        .pickups
        .iter()
        .filter(|(at, _)| at.distance(me.pos) <= READ_RANGE)
        .min_by(|a, b| a.0.distance(me.pos).total_cmp(&b.0.distance(me.pos)));
    let Some((_, item)) = nearest else {
        return String::new();
    };
    format!(
        "F: take {} - {} | replaces {}",
        item.name(),
        describe(*item),
        me.kit.get(item.category()).map_or("nothing", Item::name)
    )
}

/// The result banner's three rows.
pub fn result_lines(outcome: &Outcome) -> [String; 3] {
    let headline = match outcome.kind {
        EndKind::Holed => "HOLED OUT".to_owned(),
        EndKind::LastStanding => "LAST ONE STANDING".to_owned(),
        EndKind::Extracted => "EXTRACTED".to_owned(),
        EndKind::Eliminated => "ELIMINATED".to_owned(),
        EndKind::Lost { by } => format!("LOST - {} HOLED OUT", NAMES[usize::from(by)]),
    };
    [
        headline,
        format!("kept: {}", names(&outcome.kept, ", ")),
        format!("points: {}", outcome.points),
    ]
}
