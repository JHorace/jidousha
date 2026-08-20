//! Assertions about the course and about what was drawn.
//!
//! Each one returns `Err(message)` rather than panicking, so `verify.rs` can
//! collect every failure and print them together. An instrument that stops at
//! the first bad reading costs a cycle per fault, and the useful one is rarely
//! the first.
//!
//! Every message states the numbers it judged. Nobody running this can look at
//! the game, so the assertion is the only instrument there is.

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord, FrameRecorder};

use crate::{
    COURSE_HALF_WIDTH, DESCENT_SPEED, DRIFT_AMPLITUDE, FIRST_GATE_Y, GATE_HALF_GAP, GATE_SPACING,
    GATES, GLIDE_SPEED, GLIDER_HALF_WIDTH, gate_center_at, gate_depth,
};

/// When the glider reaches gate `index`, in seconds from the start.
///
/// The descent rate is fixed and is not the player's to change, which is what
/// makes this answerable in closed form — and what lets both the checks below
/// ask about a gate without simulating the course to reach it.
fn arrival(index: u32) -> f32 {
    gate_depth(index) / DESCENT_SPEED
}

/// One failure, as a sentence a reader can act on.
pub(crate) type Check = Result<(), String>;

/// **Can this course be flown at all, by anything?**
///
/// The arithmetic that decides whether a game is winnable, stated before any
/// controller is blamed for failing to win it. Between two gates the glider has
/// `GATE_SPACING / DESCENT_SPEED` seconds and covers `GLIDE_SPEED` units a
/// second; if the gate centres can move further than that in the same interval,
/// no pilot can be in both places and the course is impossible.
///
/// This is the check to reach for the moment a run reports "unplayable". It is
/// about the *game*, and it passes or fails whatever the controller does.
pub(crate) fn the_course_is_completable(phase: f32) -> Check {
    let seconds = GATE_SPACING / DESCENT_SPEED;
    let travel = GLIDE_SPEED * seconds;
    let mut worst = (0_u32, 0.0_f32);
    for index in 1..GATES {
        // Each gate where it will be *when the glider gets to it*. Comparing
        // both at the same instant would answer a question about a course
        // nobody flies.
        let jump = (gate_center_at(index, phase, arrival(index))
            - gate_center_at(index - 1, phase, arrival(index - 1)))
        .abs();
        if jump > worst.1 {
            worst = (index, jump);
        }
    }
    // The glider only has to get its own width inside the gap, so it may fall
    // short of the centre by the slack the gap gives it.
    let slack = GATE_HALF_GAP - GLIDER_HALF_WIDTH;
    if worst.1 <= travel + slack {
        return Ok(());
    }
    Err(format!(
        "the course cannot be flown: gate {} moves {:.2} units from its predecessor, \
         and a glider covers {:.2} in the {:.2}s between them, with {:.2} of slack in the gap. \
         No controller can clear this — lower DRIFT_PER_GATE, raise GLIDE_SPEED, or widen \
         GATE_SPACING",
        worst.0, worst.1, travel, seconds, slack,
    ))
}

/// **Is it hard enough to be worth flying?**
///
/// The other half of the same question. A course a chaser clears completely is a
/// course with no decision in it; one it clears none of is a course a person
/// cannot start. Stated as a band on the *measured* gap between the two pilots
/// rather than as arithmetic, because this is about the game as played.
pub(crate) fn the_gap_between_pilots_is_a_game(pilot: usize, chaser: usize) -> Check {
    if pilot == 0 {
        return Err("the pilot cleared nothing, so there is no ratio to judge".to_string());
    }
    // A band on the *ratio*, not on the difference, and both edges say what they
    // are for. A count would have to be restated every time `GATES` changed; a
    // ratio is the requirement itself. The numbers are the two ways a difficulty
    // curve fails, and neither is arbitrary: at the top, chasing is as good as
    // planning and the game asks nothing; at the bottom, a first-time player
    // clears so little that they never see the course.
    let share = chaser as f32 / pilot as f32;
    if share > 0.85 {
        return Err(format!(
            "planning bought almost nothing: the pilot cleared {pilot} of {GATES} and a \
             chaser cleared {chaser}, {:.0}% of it. Either the course has no decision \
             in it — raise DRIFT_PER_SECOND until a gate outruns GLIDE_SPEED — or the \
             pilot is not making one",
            share * 100.0
        ));
    }
    if share < 0.25 {
        return Err(format!(
            "the course is punishing rather than hard: the pilot cleared {pilot} of \
             {GATES} and a chaser cleared only {chaser}, {:.0}% of it. Somebody's first \
             try should get most of the way down. Widen GATE_HALF_GAP or lower \
             DRIFT_PER_SECOND",
            share * 100.0
        ));
    }
    Ok(())
}

/// **Nothing is drawn outside the camera.**
///
/// The highest-value check a game of shapes and text can write, and the one that
/// catches text centred by `width_of` running off both edges.
pub(crate) fn everything_is_on_screen(frame: &FrameRecord, camera: &Camera) -> Check {
    let view = camera.visible_bounds();
    for quad in frame.quads() {
        let bounds = quad.bounds();
        if !view.contains_rect(bounds) {
            return Err(format!(
                "drawn off screen: {bounds:?} against a camera showing {view:?} \
                 — text centred by width_of is the usual culprit"
            ));
        }
    }
    Ok(())
}

/// **The glider is drawn where the world says it is.**
///
/// `ctx.circle` submits sixteen wedges rather than one square, so nothing the
/// size of the glider is drawn anywhere. What is true is that the wedges share
/// the centre as a corner and all fit the bounding box, so the union of those
/// covering the centre is exactly `2r × 2r`.
pub(crate) fn the_glider_is_drawn_at(frame: &FrameRecord, at: Vec2, radius: f32) -> Check {
    let box_of_it = Rect::from_center_size(at, Vec2::splat(radius * 2.0));
    let mut union: Option<Rect> = None;
    for quad in frame.covering(at) {
        let drawn = quad.bounds();
        let inside = drawn.min.x >= box_of_it.min.x - 1e-3
            && drawn.min.y >= box_of_it.min.y - 1e-3
            && drawn.max.x <= box_of_it.max.x + 1e-3
            && drawn.max.y <= box_of_it.max.y + 1e-3;
        if !inside {
            continue; // a gate or a wall behind the glider, not the glider
        }
        union = Some(match union {
            None => drawn,
            Some(so_far) => Rect {
                min: so_far.min.min(drawn.min),
                max: so_far.max.max(drawn.max),
            },
        });
    }
    let Some(size) = union.map(|u| u.size()) else {
        return Err(format!(
            "nothing at all was drawn where the glider is, {at:?}"
        ));
    };
    if (size.x - radius * 2.0).abs() > 1e-3 || (size.y - radius * 2.0).abs() > 1e-3 {
        return Err(format!(
            "no glider-sized disc at {at:?}: the quads covering it span {size:?}, \
             want {:.3} square",
            radius * 2.0
        ));
    }
    Ok(())
}

/// **The glider is drawn in front of the gates.**
///
/// A band is only visible where it changes the order, so this asks about a point
/// two bands cover: `covering(p)[0]` is what a player looking at `p` sees, and
/// it must be the glider rather than the gate behind it.
pub(crate) fn the_glider_is_in_front(
    frame: &FrameRecord,
    at: Vec2,
    font: BackendTextureId,
) -> Check {
    let Some(front) = frame.covering(at).first().map(|q| (q.texture, q.bounds())) else {
        return Err(format!("nothing is drawn at {at:?} at all"));
    };
    if front.0 == font {
        return Err(format!(
            "the front-most quad at {at:?} is a glyph — the HUD is painting over the glider"
        ));
    }
    let box_of_it = Rect::from_center_size(at, Vec2::splat(GLIDER_HALF_WIDTH * 2.0));
    if !box_of_it.contains_rect(front.1) {
        return Err(format!(
            "the front-most quad at {at:?} spans {:?}, which is bigger than the glider's \
             {:?} — a gate is drawn over it",
            front.1, box_of_it
        ));
    }
    Ok(())
}

/// **The course fits inside the camera, whatever the glider is doing.**
///
/// A requirement that names no constant the game owns, paired with the walls
/// being drawn at `COURSE_HALF_WIDTH`. Written the other way round —
/// `assert_eq!(wall.min.x, -COURSE_HALF_WIDTH)` — the check moves with the
/// constant and a mutation walks straight through it.
pub(crate) fn the_course_fits_the_view(camera: &Camera) -> Check {
    let view = camera.visible_bounds();
    let needed = COURSE_HALF_WIDTH * 2.0;
    if view.size().x < needed {
        return Err(format!(
            "the course is wider than the camera: {needed:.2} units of course in a view \
             {:.2} across, so a wall is off screen at this aspect",
            view.size().x
        ));
    }
    Ok(())
}

/// **The gates stay inside the course.**
///
/// The requirement, not the constant: a gate whose post is outside the wall is
/// a gate nothing can be flown through, and `DRIFT_AMPLITUDE` moving would not
/// tell you.
pub(crate) fn the_gates_stay_inside(phase: f32) -> Check {
    for index in 0..GATES {
        let center = gate_center_at(index, phase, arrival(index));
        if center.abs() + GATE_HALF_GAP > COURSE_HALF_WIDTH {
            return Err(format!(
                "gate {index} sits at {center:.2} with a half-gap of {GATE_HALF_GAP:.2}, \
                 outside a course {COURSE_HALF_WIDTH:.2} wide (drift amplitude \
                 {DRIFT_AMPLITUDE:.2}, first gate at {FIRST_GATE_Y:.1})"
            ));
        }
    }
    Ok(())
}

/// **The systems run in the order this game chose.**
///
/// `steer` moves the glider and `judge_the_gates` decides what it passed
/// through, so the glider is judged at its post-move position. Swap the two and
/// the world still ends up in a legal state, one tick of travel out, and every
/// assertion about where things ended up still passes. `schedule_debug` is the
/// only instrument in the surface that can see it.
pub(crate) fn the_schedule_is_the_one_we_chose(schedule: &str) -> Check {
    let (Some(steer), Some(judge)) = (schedule.find("steer"), schedule.find("judge_the_gates"))
    else {
        return Err(format!(
            "the schedule names neither `steer` nor `judge_the_gates`:\n{schedule}"
        ));
    };
    if steer >= judge {
        return Err(format!(
            "`judge_the_gates` runs before `steer`, so a gate is judged against last tick's \
             position. This game chose post-move; the schedule now says otherwise:\n{schedule}"
        ));
    }
    Ok(())
}

/// **Every character the game draws is one the font has.**
///
/// The font covers space through `~` and draws everything else as a box at
/// exactly the advance of a letter, so a stray em dash produces a quad the right
/// size in the right place and every geometric check passes. Only the string can
/// tell you.
pub(crate) fn every_glyph_is_printable(text: &str) -> Check {
    if text.chars().all(|c| (' '..='~').contains(&c)) {
        return Ok(());
    }
    Err(format!(
        "unprintable character in {text:?} — the font draws a box, and no assertion over \
         what was drawn can tell the difference"
    ))
}

/// **The frame cleared to a colour a pale glider reads against.**
///
/// The requirement, not `assert_eq!(clear_color, SKY)`: that form moves with the
/// constant it checks.
pub(crate) fn the_sky_is_dark_enough(frame: &FrameRecord) -> Check {
    let cleared = frame.plan.clear_color;
    let brightness = cleared.r.max(cleared.g).max(cleared.b);
    if brightness < 0.25 && cleared.a > 0.99 {
        return Ok(());
    }
    Err(format!(
        "the sky is not dark enough to see a pale glider on: brightest channel \
         {brightness:.3} at alpha {:.2}",
        cleared.a
    ))
}

/// The font's texture id, for the checks that need to tell text from shapes.
#[must_use]
pub(crate) fn font_of(recorder: &FrameRecorder) -> BackendTextureId {
    recorder.font_texture()
}
