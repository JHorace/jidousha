//! Session F: every recorded frame, the staged screens a played match never reaches,
//! the schedule order, and the capture.
//!
//! Key functions: `session_f`.
//! Depends on: `gates` (staging), `verify`, `checks`, `capture`, `items`, `outcome`.
//! Never depended on outside the game's check.
//! INVARIANT: the layout checks state the game's requirements (two bands, the course, a
//! dark court), not the constants that currently satisfy them.

use jidousha::prelude::*;
use jidousha::testing::FrameRecorder;

use crate::capture::{CAPTURE_SIZE, capture_a_frame};
use crate::checks::{Checks, within};
use crate::gates::{set_kit, staged};
use crate::items::{Item, Kit, Slot};
use crate::outcome::{Outcome, Placing};
use crate::verify::{Gallery, HEADLESS_VIEWPORT};
use crate::world::{Course, MatchState, Player};
use crate::{COURSE_HALF, LINE_Y, TEXT_SIZE, WINDOW};

/// The kit the staged result screens show.
fn kit() -> Kit {
    Kit {
        ball: Some(Item::LightBall),
        body: Some(Item::BigClub),
        last_taken: Some(Slot::Body),
    }
}

/// Stage the screens a played match never reaches, one frame each, and return the
/// schedule the sim ran.
fn stage_screens(gallery: &mut Gallery) -> String {
    let mut run = staged(&[]);
    let schedule = run.sim.schedule_debug();
    let mut recorder = FrameRecorder::new(HEADLESS_VIEWPORT);
    set_kit(run.sim.world_mut(), 0, kit());
    let screens = [
        (
            Outcome::Extracted,
            Placing::Left { still_in: 3 },
            "extracted",
        ),
        (Outcome::HoledOut, Placing::Won, "holed out"),
        (Outcome::LastStanding, Placing::Won, "last standing"),
        (Outcome::ClosestToPin, Placing::Won, "closest to pin"),
        (Outcome::Eliminated, Placing::Out { place: 4 }, "eliminated"),
        (Outcome::Lost { to: 3 }, Placing::Lost { to: 3 }, "lost"),
    ];
    for (outcome, placing, label) in screens {
        let kept = crate::outcome::kept_on(outcome, &kit());
        run.sim.world_mut().insert_resource(MatchState::Over {
            outcome,
            kept,
            placing,
        });
        gallery.snap(&mut recorder, &mut run, &format!("staged result, {label}"));
    }
    run.sim.world_mut().insert_resource(MatchState::Playing);
    let tick = run.sim.world().resource::<jidousha::prelude::Time>().tick;
    for (_, p) in run.sim.world_mut().query_mut::<&mut Player>() {
        if p.index == 0 {
            p.dazed_until = tick + 120;
        }
    }
    gallery.snap(&mut recorder, &mut run, "staged, the human dazed");
    for (_, p) in run.sim.world_mut().query_mut::<&mut Player>() {
        if p.index == 0 {
            p.dazed_until = 0;
        }
    }
    let pad_gone = run
        .world()
        .resource::<Course>()
        .pad_closes
        .map_or(8000, |t| t + 10);
    for (label, at) in [
        ("pad gone", pad_gone),
        ("the hole open", 5800),
        ("the final zone", 9100),
    ] {
        run.sim
            .world_mut()
            .resource_mut::<jidousha::prelude::Time>()
            .tick = at;
        gallery.snap(&mut recorder, &mut run, &format!("staged, {label}"));
    }
    schedule
}

/// Session F.
pub fn session_f(checks: &mut Checks, gallery: &mut Gallery, aiming: usize) {
    let schedule = stage_screens(gallery);
    let camera = Camera {
        viewport: HEADLESS_VIEWPORT,
        ..crate::camera()
    };
    let view = camera.visible_bounds();
    let course = Rect::from_center_size(Vec2::ZERO, COURSE_HALF * 2.0);
    let mut clearance = f32::MAX;
    let mut off_screen = 0usize;
    let mut stray_text = 0usize;
    let mut stray_shape = 0usize;
    let mut detail = String::new();
    for shot in &gallery.shots {
        for quad in shot.frame.quads() {
            let b = quad.bounds();
            if !view.contains_rect(b) {
                off_screen += 1;
                detail = format!("{}: quad {b:?} against {view:?}", shot.label);
            }
            let gap = (b.min - view.min).min(view.max - b.max);
            clearance = clearance.min(gap.x.min(gap.y));
            let in_course = course.contains_rect(b);
            if quad.texture == shot.font {
                let in_a_line = LINE_Y
                    .iter()
                    .any(|y| b.min.y >= *y - 1e-3 && b.max.y <= y + TEXT_SIZE + 1e-3);
                if !in_course && !in_a_line {
                    stray_text += 1;
                    detail = format!(
                        "{}: glyph {b:?} is in no band line and not on the course",
                        shot.label
                    );
                }
            } else {
                let slack = Rect {
                    min: course.min - Vec2::splat(0.05),
                    max: course.max + Vec2::splat(0.05),
                };
                if !slack.contains_rect(b) {
                    stray_shape += 1;
                    detail = format!("{}: shape {b:?} lies off the course", shot.label);
                }
            }
        }
    }
    checks.require(
        off_screen == 0,
        "something is drawn outside the camera",
        format!("{off_screen} quads; last: {detail}"),
    );
    checks.require(
        stray_text == 0,
        "text is drawn outside the two bands and the course",
        format!("{stray_text} glyphs; last: {detail}"),
    );
    checks.require(
        stray_shape == 0,
        "a zone dot, golfer, ball or pickup is drawn off the course",
        format!("{stray_shape} quads; last: {detail}"),
    );
    // F4: a dark court, by requirement and by the constant.
    let mut dark = true;
    for shot in &gallery.shots {
        let c = shot.frame.plan.clear_color;
        let brightest = c.r.max(c.g).max(c.b);
        if !(brightest < 0.25 && c.a > 0.99 && c == crate::COURT) {
            dark = false;
            detail = format!("{}: cleared to {c:?}", shot.label);
        }
    }
    checks.require(
        dark,
        "a frame is not cleared to a court dark enough for white and blue rings to read against",
        detail.clone(),
    );
    // F5: printable ASCII in everything drawn.
    let mut strings: Vec<String> = gallery
        .shots
        .iter()
        .flat_map(|s| s.strings.clone())
        .collect();
    for item in Item::ALL {
        strings.push(item.describe());
        strings.push(item.name().to_owned());
        strings.push(item.tag().to_owned());
    }
    for index in 0..6 {
        strings.push(crate::text::tag_of(index));
    }
    strings.push("DAZED".to_owned());
    let stray = strings
        .iter()
        .find(|s| s.chars().any(|c| !(' '..='~').contains(&c)));
    checks.require(
        stray.is_none(),
        "a string the game draws has a character the font cannot draw",
        format!("{stray:?}"),
    );
    // F6: the Update order.
    let names = [
        "decide", "walk", "swing", "shoot", "fly", "pickup", "expose", "extract", "settle",
    ];
    let at: Vec<Option<usize>> = names
        .iter()
        .map(|n| {
            schedule
                .find(&format!("systems::{n}"))
                .or_else(|| schedule.find(n))
        })
        .collect();
    let ordered = at.iter().all(Option::is_some) && at.windows(2).all(|w| w[0] < w[1]);
    checks.require(
        ordered,
        "the Update systems do not run in the order the rules need",
        format!("positions of {names:?} in the schedule: {at:?}\n{schedule}"),
    );
    // F7 is the staged screens above, under F1-F5. F8: the capture.
    checks.require(
        within(CAPTURE_SIZE.aspect(), WINDOW.aspect(), 1e-3),
        "the capture is not the window's shape",
        format!("{CAPTURE_SIZE:?} against {WINDOW:?}"),
    );
    let (frame, font) = {
        let shot = &gallery.shots[aiming];
        (shot.frame.clone(), shot.font)
    };
    let captured = capture_a_frame(checks, &frame, font);
    gallery.say(format!(
        "frames: {} recorded; closest quad to the edge: {clearance:.2} world units",
        gallery.shots.len()
    ));
    gallery.say(format!("capture: {captured}"));
}
