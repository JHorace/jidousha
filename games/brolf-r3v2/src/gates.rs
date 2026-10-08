//! Decision rows 1 and 2: the zone, and contact (DESIGN.md, Gates).
//!
//! Expectations that a constant could move with are shipped literals here —
//! `28.7`, `GRACE 3.3s`, `disables 3.0s`, a stun of exactly 180 and 90 ticks —
//! and the ticks the zone bites on are found by scanning `zone_at`, never by
//! trusting the design's arithmetic.

use jidousha::prelude::*;

use crate::checks::{Checks, Driver, glyphs_at};
use crate::draw::HUD_AT;
use crate::hud::hud_lines_of;
use crate::model::*;
use crate::rules::*;
use crate::world::{Ball, Golfer, Match, bearing};

/// The entity of golfer `idx`.
pub fn golfer_entity(world: &World, idx: u8) -> Option<Entity> {
    world
        .query::<&Golfer>()
        .find(|(_, g)| g.idx == idx)
        .map(|(e, _)| e)
}

/// The entity of golfer `idx`'s ball.
pub fn ball_entity(world: &World, idx: u8) -> Option<Entity> {
    world
        .query::<&Ball>()
        .find(|(_, b)| b.owner == idx)
        .map(|(e, _)| e)
}

/// Move golfer `idx` (and set anything else on it) through `edit`.
pub fn edit_golfer(driver: &mut Driver, idx: u8, edit: impl FnOnce(&mut Transform, &mut Golfer)) {
    let world = driver.world_mut();
    let Some(entity) = golfer_entity(world, idx) else {
        return;
    };
    let mut transform = *world.component::<Transform>(entity);
    let mut golfer = *world.component::<Golfer>(entity);
    edit(&mut transform, &mut golfer);
    world.insert(entity, transform);
    world.insert(entity, golfer);
}

/// Put golfer `idx`'s ball at `at` with velocity `vel`.
pub fn place_ball(driver: &mut Driver, idx: u8, at: Vec2, vel: Vec2) {
    let world = driver.world_mut();
    if let Some(entity) = ball_entity(world, idx) {
        world.insert(entity, Transform::at(at));
        world.component_mut::<Ball>(entity).vel = vel;
    }
}

/// Keep every NPC disabled for the rest of a staged run, so no rival holes out
/// or swings while a gate watches one thing. A staged change (DESIGN.md, Gates).
pub fn freeze(driver: &mut Driver, idxs: &[u8]) {
    for &idx in idxs {
        edit_golfer(driver, idx, |_, golfer| golfer.stun_left = u32::MAX / 2);
    }
}

/// The number between `r=` and ` in` on HUD line 4.
fn arrival_radius(line: &str) -> Option<String> {
    let after = line.split("zone at arrival (r=").nth(1)?;
    Some(after.split(" in").next()?.to_owned())
}

/// Row 1: the zone now, the zone when I reach my ball, the grace count, and the
/// elimination on the tick the function says.
pub fn zone(
    checks: &mut Checks,
    last_frames: &mut Vec<(String, jidousha::testing::FrameRecord)>,
) -> String {
    let start = GOLFERS[0].0;
    let Some(t_out) = (1..20_000u64).find(|&t| !inside_zone(zone_at(t), start)) else {
        checks.require(
            false,
            "decision 1 zone: the start is never outside the zone",
            String::new(),
        );
        return "decision 1 zone: not run".to_owned();
    };
    let mut driver = Driver::new();
    driver.idle();
    freeze(&mut driver, &[1, 2, 3]);
    let (mut grace_line, mut eliminated_at) = (String::new(), None);
    let mut outside_first = 0;
    while driver.tick < t_out + 299 {
        driver.idle();
        let t = driver.tick;
        let view = driver.snap();
        let me = view.golfer(0).copied();
        if t == 2600 {
            let frame = driver.draw();
            let lines = hud_lines_of(&view);
            let ball = start + BALL_OFFSET;
            let vel = shot_velocity(bearing(ball, CUP), Power::Chip, Effects::BARE);
            let roll = roll_out(ball, vel, view.dt, 2601);
            let arrival = zone_at(arrival_tick(2600, start, &roll, view.dt)).radius;
            let shown = arrival_radius(&lines[3]);
            checks.require(
                lines[0].starts_with("ZONE r=28.7") && lines[0].contains("GRACE ok"),
                "decision 1 zone: line 1 does not show the zone now and an ok grace",
                format!("line 1 {:?}", lines[0]),
            );
            checks.require(
                shown.as_deref() == Some(format!("{arrival:.1}").as_str())
                    && arrival < zone_at(2600).radius,
                "decision 1 zone: line 4 does not show the zone as it will be on arrival",
                format!(
                    "line 4 {:?}; recomputed arrival radius {arrival:.1}",
                    lines[3]
                ),
            );
            let font = driver.font();
            for row in [0, 3] {
                let drawn = glyphs_at(&frame, font, HUD_AT.y + row as f32);
                checks.require(
                    drawn == lines[row].chars().count(),
                    "decision 1 zone: a HUD line is not all on the frame",
                    format!("row {row}: {drawn} glyphs for {:?}", lines[row]),
                );
            }
            let rings = [
                (palette::ZONE_RING, zone_at(2600).radius),
                (palette::NEXT_RING, arrival),
            ];
            for (tint, radius) in rings {
                let quads: Vec<_> = frame
                    .quads()
                    .into_iter()
                    .filter(|q| q.tint == tint)
                    .collect();
                let off = quads
                    .iter()
                    .flat_map(|q| q.corners)
                    .map(|corner| (corner.distance(CUP) - radius).abs())
                    .fold(0.0_f32, f32::max);
                checks.require(
                    !quads.is_empty() && off <= 0.3,
                    "decision 1 zone: a zone ring is not drawn at its radius",
                    format!(
                        "{} quads tinted {tint:?}, worst corner {off:.3} off radius {radius:.2}",
                        quads.len()
                    ),
                );
            }
            last_frames.push(("zone at tick 2600".to_owned(), frame));
        }
        if t == t_out {
            outside_first = me.map_or(0, |me| me.outside_ticks);
        }
        if t == t_out + 100 {
            grace_line = hud_lines_of(&view)[0].clone();
            last_frames.push(("zone, grace running".to_owned(), driver.draw()));
        }
        if t == t_out + 298 {
            let alive = me.is_some_and(|me| me.alive) && view.result.is_none();
            checks.require(
                alive,
                "decision 1 zone: eliminated before the grace ran out",
                format!("tick {t}: {me:?}, result {:?}", view.result),
            );
        }
        if eliminated_at.is_none() && me.is_some_and(|me| !me.alive) {
            eliminated_at = Some(t);
        }
    }
    let result = driver.world().resource::<Match>().result.clone();
    checks.require(
        outside_first == 1,
        "decision 1 zone: the grace count did not start on the tick the zone left the golfer",
        format!("outside_ticks {outside_first} on tick {t_out}"),
    );
    checks.require(
        grace_line.contains("GRACE 3.3s"),
        "decision 1 zone: line 1 does not count the grace down",
        format!("tick {}: {grace_line:?}", t_out + 100),
    );
    checks.require(
        eliminated_at == Some(t_out + 299)
            && result
                .as_ref()
                .is_some_and(|o| o.kind == EndKind::Eliminated),
        "decision 1 zone: the golfer was not eliminated on the tick the grace ran out",
        format!("outside from {t_out}; eliminated on {eliminated_at:?}; result {result:?}"),
    );
    format!(
        "decision 1 zone: outside from tick {t_out}, eliminated on {eliminated_at:?}, {grace_line:?}"
    )
}

/// Line 5 of the HUD now.
fn line5(driver: &Driver) -> String {
    hud_lines_of(&driver.snap())[4].clone()
}

/// Row 2: the cue names what C will do, and C does exactly that.
pub fn contact(checks: &mut Checks) -> String {
    let wren = 3u8;
    let mut driver = Driver::new();
    driver.idle();
    freeze(&mut driver, &[1, 2]);
    edit_golfer(&mut driver, wren, |t, _| t.pos = Vec2::new(-20.0, 11.0));
    place_ball(&mut driver, wren, Vec2::new(-10.0, 11.0), Vec2::ZERO);
    driver.idle();
    let frame = driver.draw();
    let cue = line5(&driver);
    let drawn = glyphs_at(&frame, driver.font(), HUD_AT.y + 4.0);
    checks.require(
        cue == "C: club WREN - disables 3.0s, drops nothing" && drawn == cue.chars().count(),
        "decision 2 contact: the cue does not say what a club will do",
        format!("line 5 {cue:?}, {drawn} glyphs drawn"),
    );
    driver.tap(Key::C);
    let stun = |d: &Driver| d.snap().golfer(wren).map_or(0, |g| g.stun_left);
    let first = stun(&driver);
    let cooldown = driver.snap().golfer(0).map_or(0, |g| g.cooldown);
    let mut ran_out = None;
    let mut removed = None;
    while driver.tick < 400 {
        driver.idle();
        let left = stun(&driver);
        if ran_out.is_none() && left == 0 {
            ran_out = Some(driver.tick);
        }
        let entity = golfer_entity(driver.world(), wren);
        let alive = entity.is_some_and(|e| driver.world().is_alive(e))
            && driver.snap().golfer(wren).is_some_and(|g| g.alive);
        if removed.is_none() && !alive {
            removed = Some(driver.tick);
        }
    }
    checks.require(
        first == 180 && cooldown == 60 && ran_out == Some(183) && removed.is_none(),
        "decision 2 contact: a club did not disable for exactly the stated time, or removed the golfer",
        format!("stun {first} after tick 3, cooldown {cooldown}, ran out on {ran_out:?}, removed on {removed:?}"),
    );
    // A held item is knocked loose where the golfer stands.
    edit_golfer(&mut driver, wren, |t, g| {
        t.pos = Vec2::new(-20.0, 11.0);
        g.kit = Kit::only(Item::Driver);
    });
    driver.tap(Key::C);
    let view = driver.snap();
    let at = view.golfer(wren).map_or(Vec2::ZERO, |g| g.pos);
    let dropped = view
        .pickups
        .iter()
        .any(|(p, item)| *item == Item::Driver && p.distance(at) < 1e-3);
    let club_slot = view.golfer(wren).and_then(|g| g.kit.club);
    checks.require(
        dropped && club_slot.is_none(),
        "decision 2 contact: a club did not knock the Driver loose where the golfer stood",
        format!("Driver at the golfer {dropped}, club slot {club_slot:?}"),
    );
    for _ in 0..SWING_COOLDOWN {
        driver.idle();
    }
    edit_golfer(&mut driver, wren, |t, g| {
        t.pos = Vec2::new(-20.0, 11.0);
        g.kit = Kit::only(Item::Helmet);
    });
    let before = driver.snap().pickups.len();
    driver.tap(Key::C);
    let helmeted = stun(&driver);
    let after = driver.snap().pickups.len();
    checks.require(
        helmeted == 90 && after == before,
        "decision 2 contact: the Helmet did not halve the disable and keep the item",
        format!("stun {helmeted}, pickups {before} -> {after}"),
    );
    // The strike half: wait out the swing and the stun first.
    for _ in 0..100 {
        driver.idle();
    }
    edit_golfer(&mut driver, wren, |t, g| {
        t.pos = Vec2::new(-10.0, 11.0);
        g.kit = Kit::default();
    });
    edit_golfer(&mut driver, 0, |_, g| g.aim = Radians::ZERO);
    place_ball(&mut driver, wren, Vec2::new(-20.0, 11.0), Vec2::ZERO);
    driver.idle();
    let strike = line5(&driver);
    driver.tap(Key::C);
    let vel = driver.snap().ball(wren).map_or(Vec2::ZERO, |b| b.vel);
    let want = ball_step(
        Vec2::new(-20.0, 11.0),
        Vec2::new(20.0, 0.0),
        Seconds(1.0 / 60.0),
    )
    .vel;
    checks.require(
        strike == "C: strike WREN's ball - knocks it 20u/s along your aim"
            && vel.distance(want) < 1e-4,
        "decision 2 contact: a strike did not do what the cue said",
        format!("line 5 {strike:?}; ball velocity {vel:?}, want {want:?}"),
    );
    for _ in 0..SWING_COOLDOWN {
        driver.idle();
    }
    edit_golfer(&mut driver, wren, |t, g| {
        t.pos = Vec2::new(-10.0, 11.0);
        g.kit = Kit::only(Item::LeadBall);
    });
    place_ball(&mut driver, wren, Vec2::new(-20.0, 11.0), Vec2::ZERO);
    driver.idle();
    let immune = line5(&driver);
    driver.tap(Key::C);
    let still = driver.snap().ball(wren).map_or(Vec2::ONE, |b| b.vel);
    checks.require(
        immune.ends_with("no effect (lead ball)") && still == Vec2::ZERO,
        "decision 2 contact: the Lead Ball did not shrug off a strike",
        format!("line 5 {immune:?}; ball velocity {still:?}"),
    );
    format!(
        "decision 2 contact: {cue:?}; stunned 180 ticks (out on {ran_out:?}); helmet 90; {strike:?}"
    )
}
