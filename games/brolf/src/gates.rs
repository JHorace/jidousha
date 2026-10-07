//! Staging helpers shared by every session, and Session A: the zone row.
//!
//! Key functions: `freeze`, `place`, `side`, `session_a`.
//! Depends on: `verify`, `checks`, `draw`, `shots`, `text`, `world`, `zone`. Never
//! depended on outside the game's check.
//! INVARIANT: staging writes `Transform`s and components through `world_mut()` between
//! two ticks, and leaves no stale `Flight`, exposure or extraction behind it.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecorder, InputScript};

use crate::checks::{Checks, disc_at, dots_at_radius, font_quads_in, within};
use crate::draw::{LAND_SAFE, REACH_RING, ZONE_DOT, ZONE_NEXT, ZONE_NOW};
use crate::items::Kit;
use crate::outcome::{Outcome, Placing};
use crate::shots::flight_ticks;
use crate::text::{aim_landing, look};
use crate::verify::{Gallery, MAX_TICKS, Run};
use crate::world::{Ball, Course, MatchState, Player};
use crate::zone::{landing_safe, zone_at};
use crate::{COURSE_HALF, LINE_Y, TEXT_SIZE};

/// Which side of the course the hole is on: staging puts the human on the other.
pub fn side(world: &World) -> f32 {
    if world.resource::<Course>().hole.x >= 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// The body and ball entities of golfer `index`.
fn entities(world: &World, index: usize) -> Option<(Entity, Entity)> {
    let body = world
        .query::<&Player>()
        .find(|(_, p)| p.index == index)
        .map(|(e, _)| e)?;
    let ball = world
        .query::<&Ball>()
        .find(|(_, b)| b.owner == index)
        .map(|(e, _)| e)?;
    Some((body, ball))
}

/// Teleport golfer `index` and their ball, clearing any flight, exposure or extraction.
pub fn place(world: &mut World, index: usize, body: Vec2, ball: Vec2) {
    let Some((b, s)) = entities(world, index) else {
        return;
    };
    world.component_mut::<Transform>(b).pos = body;
    world.component_mut::<Transform>(s).pos = ball;
    world.component_mut::<Ball>(s).flight = None;
    let player = world.component_mut::<Player>(b);
    player.exposure = 0;
    player.extracting_since = None;
    player.swing_ready_at = 0;
}

/// Park every rival not in `keep` where no zone excludes it and no ball can hole out,
/// and daze it for ever.
pub fn freeze(world: &mut World, keep: &[usize]) {
    let hole = world.resource::<Course>().hole;
    for k in 1..=5usize {
        if keep.contains(&k) {
            continue;
        }
        let dx = (k as f32 - 3.0) * 0.4;
        place(
            world,
            k,
            hole + Vec2::new(dx, 0.6),
            hole + Vec2::new(dx, 0.75),
        );
        if let Some((b, _)) = entities(world, k) {
            world.component_mut::<Player>(b).dazed_until = u64::MAX;
        }
    }
}

/// Let a frozen rival play again.
pub fn thaw(world: &mut World, index: usize) {
    if let Some((b, _)) = entities(world, index) {
        world.component_mut::<Player>(b).dazed_until = 0;
    }
}

/// Dim a rival for ever without moving them.
pub fn stun(world: &mut World, index: usize) {
    if let Some((b, _)) = entities(world, index) {
        world.component_mut::<Player>(b).dazed_until = u64::MAX;
    }
}

/// Give golfer `index` a kit.
pub fn set_kit(world: &mut World, index: usize, kit: Kit) {
    if let Some((b, _)) = entities(world, index) {
        world.component_mut::<Player>(b).kit = kit;
    }
}

/// The body and ball positions of golfer `index`, if still in play.
pub fn positions(world: &World, index: usize) -> Option<(Vec2, Vec2)> {
    let (b, s) = entities(world, index)?;
    Some((
        world.component::<Transform>(b).pos,
        world.component::<Transform>(s).pos,
    ))
}

/// A player component copy, if still in play.
pub fn player(world: &World, index: usize) -> Option<Player> {
    entities(world, index).map(|(b, _)| *world.component::<Player>(b))
}

/// A ball's flight, if in the air.
pub fn flight(world: &World, index: usize) -> Option<crate::shots::Flight> {
    entities(world, index).and_then(|(_, s)| world.component::<Ball>(s).flight)
}

/// A new run with its first tick taken (so `Startup` has dealt the match) and every
/// rival not in `keep` frozen.
pub fn staged(keep: &[usize]) -> Run {
    let mut run = Run::new();
    run.step();
    freeze(run.sim.world_mut(), keep);
    run
}

/// Session A: the zone row. The idle human is eliminated on the tick the zone function says.
///
/// Returns the index of the aiming frame in the gallery, for the capture and the transcript.
pub fn session_a(checks: &mut Checks, gallery: &mut Gallery) -> usize {
    // Pass 1, no recorder: find the first tick the zone excludes the human and watch the grace run out.
    let mut run = staged(&[]);
    let course = *run.world().resource::<Course>();
    let mut first_out: Option<u64> = None;
    let mut inside_again: Option<u64> = None;
    let mut alive_before = None;
    let mut over_at = None;
    while run.tick < MAX_TICKS && over_at.is_none() {
        run.step();
        let t = run.tick;
        if let Some((body, ball)) = positions(run.world(), 0) {
            let zone = zone_at(&course.schedule, t);
            let outside = !zone.contains(body) || !zone.contains(ball);
            if outside && first_out.is_none() {
                first_out = Some(t);
            }
            if !outside && first_out.is_some() && inside_again.is_none() {
                inside_again = Some(t);
            }
        }
        if let Some(f) = first_out {
            if t == f + 298 {
                alive_before = Some(
                    positions(run.world(), 0).is_some()
                        && *run.world().resource::<MatchState>() == MatchState::Playing,
                );
            }
            if t == f + 299 {
                over_at = Some(t);
            }
        }
    }
    let first_out = first_out.unwrap_or(0);
    checks.require(
        (1201..=3900).contains(&first_out),
        "the human was never outside the zone in the window the zone schedule promises",
        format!("first tick the zone excluded the idle human: {first_out}, expected 1201..=3900"),
    );
    checks.require(
        inside_again.is_none(),
        "the idle human was back inside the zone after leaving it",
        format!("first out {first_out}, inside again at {inside_again:?}"),
    );
    checks.require(
        alive_before == Some(true),
        "the human was not alive and playing on the tick before the grace ended",
        format!(
            "alive at first_out + 298 = {}: {alive_before:?}",
            first_out + 298
        ),
    );
    let expected = MatchState::Over {
        outcome: Outcome::Eliminated,
        kept: Kit::default(),
        placing: Placing::Out { place: 6 },
    };
    checks.require(
        over_at == Some(first_out + 299) && *run.world().resource::<MatchState>() == expected,
        "the human was not eliminated on the grace's last tick with an empty kit in 6th place",
        format!(
            "first out {first_out}; over at {over_at:?} (want {}); state {:?}",
            first_out + 299,
            run.world().resource::<MatchState>()
        ),
    );
    gallery.say(format!(
        "zone: the idle human is first outside at tick {first_out}, eliminated at tick {} (grace 299 after)",
        first_out + 299
    ));

    // Pass 2: the same session, with frames on the ticks that matter and a sample of the rest.
    let mut run = staged(&[]);
    let mut recorder = FrameRecorder::new(crate::verify::HEADLESS_VIEWPORT);
    let wanted = [first_out.saturating_sub(1), first_out + 60];
    let mut at_before = None;
    let mut at_after = None;
    while run.tick < first_out + 299 {
        run.step();
        if run.tick.is_multiple_of(300) {
            gallery.snap(&mut recorder, &mut run, "zone, sampled");
        }
        if run.tick == wanted[0] {
            at_before = Some(gallery.snap(
                &mut recorder,
                &mut run,
                "zone, the tick before the human is out",
            ));
        }
        if run.tick == wanted[1] {
            at_after =
                Some(gallery.snap(&mut recorder, &mut run, "zone, sixty ticks into the grace"));
        }
    }
    let line1_quads = |gallery: &Gallery, at: usize| {
        let shot = &gallery.shots[at];
        (
            shot.strings[0].clone(),
            font_quads_in(&shot.frame, shot.font, LINE_Y[0], LINE_Y[0] + TEXT_SIZE),
        )
    };
    if let (Some(before), Some(after)) = (at_before, at_after) {
        let (line, quads) = line1_quads(gallery, after);
        checks.require(
            line.contains("OUT 4.0s"),
            "the grace countdown is not on the status line sixty ticks into the grace",
            format!("status line 1 on tick {} reads {line:?}", first_out + 60),
        );
        checks.require(
            quads == line.chars().count(),
            "the status line's characters are not what is drawn in the top band's first line",
            format!(
                "{} characters in {line:?}, {quads} font quads in the line's strip",
                line.chars().count()
            ),
        );
        let (line, _) = line1_quads(gallery, before);
        checks.require(
            !line.contains("OUT"),
            "the countdown was shown before the zone excluded the human",
            format!("status line 1 on tick {} reads {line:?}", first_out - 1),
        );
    }

    // The aiming frame: A3 and A4.
    aiming_frame(checks, gallery)
}

/// A3 and A4: the human at their ball inside the second zone, the pointer out on the course.
fn aiming_frame(checks: &mut Checks, gallery: &mut Gallery) -> usize {
    let mut run = staged(&[]);
    let course = *run.world().resource::<Course>();
    let (c1, c2) = (course.schedule.stops[1].1, course.schedule.stops[2].1);
    let away = (c1 - course.hole).normalize_or_zero();
    let away = if away == Vec2::ZERO { Vec2::X } else { away };
    let (body, ball) = (c1 + away * 2.8, c1 + away * 2.0);
    place(run.sim.world_mut(), 0, body, ball);
    let target = ball + Vec2::new(4.0, -3.0);
    let screen = run.camera().world_to_screen(target);
    run.script = InputScript::new().pointer_at(2, screen);
    run.to(2900);
    let mut recorder = FrameRecorder::new(crate::verify::HEADLESS_VIEWPORT);
    let at = gallery.snap(
        &mut recorder,
        &mut run,
        "aiming inside the second zone, tick 2900",
    );
    let shot = &gallery.shots[at];
    let frame = &shot.frame;
    let zone = zone_at(&course.schedule, 2900);
    let now_dots = dots_at_radius(frame, ZONE_NOW, ZONE_DOT, zone.center, 8.0, 0.12);
    checks.require(
        now_dots >= 24 && within(zone.radius, 8.0, 1e-3),
        "the current zone is not drawn as a ring of its radius while aiming",
        format!(
            "{now_dots} zone-coloured dots at distance 8.0 of {:?}; zone radius {}",
            zone.center, zone.radius
        ),
    );
    let next_dots = dots_at_radius(frame, ZONE_NEXT, ZONE_DOT, c2, 5.0, 0.12);
    checks.require(
        next_dots >= 18,
        "the next zone is not drawn as a ring of its radius while aiming",
        format!("{next_dots} white dots at distance 5.0 of the next stop's centre {c2:?}"),
    );
    // The aim line spans from the ball to the aim point (the literal reach is 7.0; the shot is 5.0).
    let aim = ball + Vec2::new(4.0, -3.0);
    let spans = frame.quads().iter().any(|q| {
        let b = q.bounds();
        q.tint == jidousha::prelude::Color::WHITE
            && b.min.x <= ball.x.min(aim.x) + 0.1
            && b.max.x >= ball.x.max(aim.x) - 0.1
            && b.min.y <= ball.y.min(aim.y) + 0.1
            && b.max.y >= ball.y.max(aim.y) - 0.1
    });
    checks.require(
        spans,
        "no line is drawn from the ball to the aim point",
        format!("ball {ball:?}, aim point {aim:?}"),
    );
    // The centre is read from the preview, which draws at the pointer's own float arithmetic:
    // `covering` is exact, and a point a millionth off a fan's centre is in one wedge, not sixteen.
    let drawn_aim = aim_landing(&look(&run.world().view())).map_or(aim, |(at, _, _)| at);
    checks.require(
        (drawn_aim - aim).length() < 0.01,
        "the preview's aim point is not where the pointer was put",
        format!("preview {drawn_aim:?}, pointer {aim:?}"),
    );
    let disc = disc_at(frame, drawn_aim, 0.6);
    checks.require(
        disc.is_some_and(|size| within(size.x, 1.2, 0.01) && within(size.y, 1.2, 0.01)),
        "the landing's scatter disc is not drawn at 12% of the shot's length",
        format!("box round the disc at {aim:?}: {disc:?}, want 1.2 square"),
    );
    let mut expected = 0usize;
    let course_box = Rect::from_center_size(Vec2::ZERO, COURSE_HALF * 2.0);
    for k in 0..24 {
        let angle = Radians::TAU.as_f32() * k as f32 / 24.0;
        let at = ball + rotate(Vec2::X * 7.0, Radians(angle));
        if course_box.contains_rect(Rect::from_center_size(at, Vec2::splat(0.12))) {
            expected += 1;
        }
    }
    let reach_dots = dots_at_radius(frame, REACH_RING, 0.12, ball, 7.0, 0.05);
    checks.require(
        reach_dots == expected && expected > 0,
        "the reach ring is not the longest shot's circle clipped to the course",
        format!("{reach_dots} reach dots at distance 7.0 of the ball; {expected} of 24 lie inside the course"),
    );
    // A4: the cue says what the landing is.
    let l = look(&run.world().view());
    let cue = shot.strings[2].clone();
    let walk_ticks = ((aim - body).length() / 4.0 * 60.0).ceil() as u64;
    let safe = landing_safe(&course.schedule, aim, 2900 + flight_ticks(5.0) + walk_ticks);
    let verdict = if safe {
        "lands in zone"
    } else {
        "lands OUTSIDE the zone"
    };
    checks.require(
        cue.starts_with("CLICK: shoot 5.0") && cue.ends_with(verdict),
        "the cue line does not say where the shot lands",
        format!(
            "cue {cue:?}; the zone function says {verdict:?} (aim {aim:?}, walk {walk_ticks} ticks)"
        ),
    );
    let landing = aim_landing(&l);
    checks.require(
        landing.is_some_and(|(_, len, s)| within(len, 5.0, 1e-3) && s == safe),
        "the aim preview and the independent landing check disagree",
        format!("preview {landing:?}, independent verdict {safe}"),
    );
    gallery.say(format!(
        "aiming at tick 2900: {now_dots} current-zone dots, {next_dots} next-zone dots, {reach_dots}/{expected} reach dots, cue {cue:?}, landing disc {} (safe colour {:?})",
        if safe { "safe" } else { "unsafe" },
        LAND_SAFE
    ));
    at
}
