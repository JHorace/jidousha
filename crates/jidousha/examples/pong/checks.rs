//! The assertions, as functions that report rather than panic.
//!
//! Every one returns `Result<(), String>` so that `verify::run` can collect the
//! failures instead of exiting on the first: an instrument that stops at the
//! first bad reading costs a cycle per fault, and the diagnostic failure is
//! routinely not the first one printed.
//!
//! Every message states the numbers it judged. Nobody writing a game this way
//! can look at it, so "the score is in the wrong place" costs a whole cycle to
//! turn into a diagnosis and "the score band spans -9.00..-6.60 against a court
//! whose top third ends at -3.00" does not.

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, DrawnQuad, FrameRecord, find_bounds};

use crate::rules;
use crate::rules::{
    BALL_HALF, BALL_SPEED_MAX, BALL_SPEED_START, CONTACT_REACH, COURT, HALF_H, HALF_W, MAX_BOUNCE,
    OPPONENT_SPEED, PADDLE_HALF_X, PADDLE_HALF_Y, PADDLE_X, Side,
};

/// How much slack a geometric comparison gets, in world units.
const SLACK: f32 = 1e-3;

/// Nothing at all is drawn outside the camera, and how close it came.
///
/// The highest-value check a game of shapes and text can write. It is also a
/// cliff — it answers yes or no, so a layout 0.03 units from the edge reads
/// exactly like one 3.0 units clear — so the clearance comes back with the
/// verdict rather than being left for a person to wonder about.
pub(crate) fn nothing_is_drawn_outside_the_camera(
    frame: &FrameRecord,
    camera: &Camera,
) -> Result<f32, String> {
    let view = camera.visible_bounds();
    let quads = frame.quads();
    if quads.is_empty() {
        return Err("nothing was drawn at all, so there is nothing to judge".to_owned());
    }
    for quad in &quads {
        let bounds = quad.bounds();
        // `contains_rect` is closed on all four sides: a quad flush against the
        // camera's edge is on screen.
        if !view.contains_rect(bounds) {
            return Err(format!(
                "drawn off screen: a quad spanning {:?}..{:?} against a camera showing \
                 {:?}..{:?} — text centred by width_of is the usual culprit",
                bounds.min, bounds.max, view.min, view.max
            ));
        }
    }
    Ok(quads
        .into_iter()
        .map(|quad| {
            let bounds = quad.bounds();
            let gap = (bounds.min - view.min).min(view.max - bounds.max);
            gap.x.min(gap.y)
        })
        .fold(f32::MAX, f32::min))
}

/// The frame cleared to the colour the camera was given.
///
/// Half of a pair. On its own this is
/// `assert_eq!(what_was_drawn, the_constant_that_drew_it)` and a mutation of
/// `COURT` walks straight through it, because the check and the thing it checks
/// move together. It is still worth writing: it catches a camera built from the
/// *wrong* constant. [`the_court_is_dark_enough_for_a_white_ball`] is the other
/// half, and it names the requirement instead.
pub(crate) fn the_court_is_cleared_to_its_colour(frame: &FrameRecord) -> Result<(), String> {
    if frame.plan.clear_color == COURT {
        return Ok(());
    }
    Err(format!(
        "the frame cleared to {:?}, not to the court colour {COURT:?}",
        frame.plan.clear_color
    ))
}

/// The court is dark enough for a white ball to read against it.
///
/// The game's own requirement, stated in numbers and naming no constant the game
/// owns — so it survives somebody changing `COURT`, which is exactly what the
/// check above cannot do.
pub(crate) fn the_court_is_dark_enough_for_a_white_ball(frame: &FrameRecord) -> Result<(), String> {
    let cleared = frame.plan.clear_color;
    let brightness = cleared.r.max(cleared.g).max(cleared.b);
    if brightness < 0.25 && cleared.a > 0.99 {
        return Ok(());
    }
    Err(format!(
        "the court is not dark enough to see a white ball on: brightest channel \
         {brightness:.3} at alpha {:.2}",
        cleared.a
    ))
}

/// The score sits in the top third of what is on screen, evenly either side of
/// the centre line.
///
/// The requirement rather than the constant. A check spelled
/// `quad.min.y < SCORE_TOP + margin` moves with `SCORE_TOP`: put that constant
/// in the middle of the court and the check follows it down, passes, and leaves
/// the score across the play.
pub(crate) fn the_score_sits_in_the_top_third(
    frame: &FrameRecord,
    camera: &Camera,
    font: BackendTextureId,
) -> Result<(), String> {
    let view = camera.visible_bounds();
    // Y is down, so the top third is the third with the smallest y.
    let third = view.min.y + view.size().y / 3.0;
    let glyphs: Vec<DrawnQuad> = frame
        .quads()
        .into_iter()
        .filter(|quad| quad.texture == font)
        .collect();
    if glyphs.is_empty() {
        return Err("no text was drawn at all, so there is no score to place".to_owned());
    }
    let (left, right): (Vec<DrawnQuad>, Vec<DrawnQuad>) = glyphs
        .into_iter()
        .partition(|quad| quad.bounds().center().x < view.center().x);
    let (Some(left), Some(right)) = (find_bounds(left), find_bounds(right)) else {
        return Err(
            "the score is not one number either side of the centre line: \
                    all of the text drawn sits on one side"
                .to_owned(),
        );
    };
    for (name, bounds) in [("left", left), ("right", right)] {
        if bounds.max.y > third {
            return Err(format!(
                "the {name} score is not in the top third: it spans y {:.2}..{:.2} against a \
                 court whose top third ends at {third:.2}",
                bounds.min.y, bounds.max.y
            ));
        }
    }
    let (gap_left, gap_right) = (view.center().x - left.max.x, right.min.x - view.center().x);
    if (gap_left - gap_right).abs() > 0.25 {
        return Err(format!(
            "the score is not evenly set about the centre line: {gap_left:.2} units of gap on \
             the left against {gap_right:.2} on the right"
        ));
    }
    if gap_left <= 0.0 || gap_right <= 0.0 {
        return Err(format!(
            "the score crosses the centre line: gaps {gap_left:.2} and {gap_right:.2}"
        ));
    }
    Ok(())
}

/// Both paddles are drawn at their own positions, at their own size.
///
/// On the *bounds*, not on "something paddle-sized covers this point" — a paddle
/// displaced by half its height still covers its own centre, and a check written
/// that way passes for it.
///
/// The colour is what says *whose* paddle is where. Taking the expected side
/// from the game's own `Side::sign()` would move when `sign()` moved: both
/// paddles change ends and every geometric assertion stays green. So this holds
/// the colours itself.
pub(crate) fn each_paddle_is_drawn_where_it_stands(
    frame: &FrameRecord,
    left_y: f32,
    right_y: f32,
) -> Result<(), String> {
    let size = Vec2::new(PADDLE_HALF_X, PADDLE_HALF_Y) * 2.0;
    for (name, expected, tint) in [
        (
            "left",
            Vec2::new(-PADDLE_X, left_y),
            Color::rgb(0.45, 0.95, 0.75),
        ),
        (
            "right",
            Vec2::new(PADDLE_X, right_y),
            Color::rgb(0.98, 0.62, 0.45),
        ),
    ] {
        let want = Rect::from_center_size(expected, size);
        let found = frame
            .quads()
            .into_iter()
            .find(|quad| quad.tint == tint)
            .map(|quad| quad.bounds());
        let Some(found) = found else {
            return Err(format!(
                "no {name} paddle was drawn: nothing in the frame carries its tint {tint:?}"
            ));
        };
        let off = (found.min - want.min)
            .abs()
            .max((found.max - want.max).abs());
        if off.x > SLACK || off.y > SLACK {
            return Err(format!(
                "the {name} paddle is drawn spanning {:?}..{:?} but stands at {expected:?}, \
                 which wants {:?}..{:?}",
                found.min, found.max, want.min, want.max
            ));
        }
    }
    Ok(())
}

/// The ball is drawn where it is, at the size it collides at.
pub(crate) fn the_ball_is_drawn_at_its_collider(
    frame: &FrameRecord,
    at: Vec2,
) -> Result<(), String> {
    let found = find_bounds(
        frame
            .covering(at)
            .into_iter()
            .filter(|quad| quad.tint == Color::WHITE),
    );
    let Some(found) = found else {
        return Err(format!(
            "nothing white is drawn where the ball is, at {at:?}"
        ));
    };
    let size = found.size();
    let wanted = BALL_HALF * 2.0;
    if (size.x - wanted).abs() > SLACK || (size.y - wanted).abs() > SLACK {
        return Err(format!(
            "the ball at {at:?} is drawn {size:?} across, not {wanted:.2} square — a ball \
             drawn at a different size from the box it collides as is a fault a picture \
             shows and no rally does"
        ));
    }
    if (found.center() - at).abs().max_element() > SLACK {
        return Err(format!(
            "the ball is drawn centred on {:?} but stands at {at:?}",
            found.center()
        ));
    }
    Ok(())
}

/// The paddles move before the ball does.
///
/// The swept contact treats a paddle as standing still at its **post-move**
/// position, which is only true if the paddles have already moved this tick.
/// The order is the sequence of `add_system` calls and nothing but a reader
/// protects it — so a tidy-up that moves one line reverses the decision
/// silently, and the world ends up in a legal state either way, one tick of a
/// paddle's travel apart. `schedule_debug` is the only instrument that sees it.
pub(crate) fn the_paddles_move_before_the_ball(schedule: &str) -> Result<(), String> {
    let ball = schedule.find("advance_the_ball");
    // Both names have to be *found*: two renamed systems give two `None`s, which
    // compare equal, and the check then passes while seeing nothing.
    let (Some(ball), Some(player), Some(opponent)) = (
        ball,
        schedule.find("drive_the_player"),
        schedule.find("drive_the_opponent"),
    ) else {
        return Err(format!(
            "the schedule does not name all three movers, so nothing here was checked:\n{schedule}"
        ));
    };
    if player < ball && opponent < ball {
        return Ok(());
    }
    Err(format!(
        "the ball moves before a paddle does: drive_the_player at {player}, \
         drive_the_opponent at {opponent}, advance_the_ball at {ball} — the swept contact \
         treats a paddle as post-move, so a paddle closing on the ball now leaks it through"
    ))
}

/// The swept contact answers its own contract, in the cases a rally never
/// reaches.
///
/// A played session cannot see this. The ball is capped below the paddle's
/// thickness, so the sweep never does anything a naive position test would not
/// — replace it with one and every assertion, the whole match and every drawn
/// frame are identical. So ask the function directly.
pub(crate) fn the_swept_contact_answers_its_contract() -> Result<(), String> {
    let side = Side::Right;
    let face = side.contact_x();
    // One tick of travel eight units long, straight across the paddle. A
    // position-only test sees the ball already past the paddle and reports
    // nothing.
    let from = Vec2::new(face - 4.0, 0.0);
    let to = Vec2::new(face + 4.0, 0.0);
    let Some(contact) = rules::paddle_contact(from, to, side, 0.0) else {
        return Err(format!(
            "a tick of travel from {from:?} to {to:?} straight through the paddle face at \
             {face:.2} reported no contact — the sweep is not sweeping"
        ));
    };
    if (contact.fraction - 0.5).abs() > SLACK || contact.at.y.abs() > SLACK {
        return Err(format!(
            "the crossing was reported at fraction {:.4} and {:?}, wanting 0.5 and y 0",
            contact.fraction, contact.at
        ));
    }
    // Past the end of the paddle: crosses the plane, misses the paddle.
    let high = CONTACT_REACH + 1.0;
    if let Some(missed) = rules::paddle_contact(
        Vec2::new(face - 4.0, high),
        Vec2::new(face + 4.0, high),
        side,
        0.0,
    ) {
        return Err(format!(
            "a ball crossing the face {high:.2} units off a paddle whose reach is \
             {CONTACT_REACH:.2} was reported as a contact at {:?}",
            missed.at
        ));
    }
    // Leaving through the same face: already past it, going the other way.
    if let Some(leaving) = rules::paddle_contact(
        Vec2::new(face + 0.1, 0.0),
        Vec2::new(face - 4.0, 0.0),
        side,
        0.0,
    ) {
        return Err(format!(
            "a ball leaving the paddle through its own face was reported as a contact at \
             {:?} — a rally would stick the ball to the paddle for ever",
            leaving.at
        ));
    }
    // And a velocity that has gone to NaN answers "no contact" rather than
    // taking a contact at a NaN fraction and leaving at a NaN position.
    if rules::paddle_contact(from, Vec2::new(f32::NAN, f32::NAN), side, 0.0).is_some() {
        return Err("a NaN destination was reported as a contact".to_owned());
    }
    Ok(())
}

/// The ball cannot travel further in one tick than the thinnest thing it must
/// not pass through.
///
/// Asserted against the `fixed_dt` the engine actually hands us, not against the
/// 1/60 the game assumed.
pub(crate) fn the_ball_cannot_outrun_the_thinnest_collider(dt: Seconds) -> Result<(), String> {
    let step = BALL_SPEED_MAX * dt.as_f32();
    let thinnest = PADDLE_HALF_X * 2.0;
    if step < thinnest {
        return Ok(());
    }
    Err(format!(
        "the ball travels {step:.3} units in one tick at its top speed of {BALL_SPEED_MAX:.1}, \
         against a paddle {thinnest:.3} thick at a timestep of {:.5}s — thicken the paddle \
         before raising the speed",
        dt.as_f32()
    ))
}

/// A steep return outruns the opponent, at the speed a rally *starts* at.
///
/// `ball_speed * sin(steepest_bounce)` against `paddle_speed`, which is the
/// whole of "can this game be scored in". Below that line, following the ball
/// wins: neither side has to predict anything, the rally is flat and the match
/// is nil-nil, and no amount of speed tuning gets out of it. Stated at the slow
/// end because that is where a rally spends itself; at the top speed it is a
/// claim about a moment that arrives once.
pub(crate) fn a_steep_return_outruns_the_opponent() -> Result<(), String> {
    let (steepest, _) = sin_cos(MAX_BOUNCE);
    let vertical = BALL_SPEED_START * steepest;
    if vertical > OPPONENT_SPEED * 1.1 {
        return Ok(());
    }
    Err(format!(
        "the opponent can follow the ball: a serve-speed ball returned at {:.0} degrees \
         climbs {vertical:.2} units a second against a paddle that manages \
         {OPPONENT_SPEED:.2} — every rally is flat and no shot exists",
        MAX_BOUNCE.to_degrees()
    ))
}

/// Every string the game draws is inside the font's printable range.
///
/// No assertion over drawn quads can see a wrong character: the font draws an
/// identically sized box for one, so the geometry is right and the picture is
/// not. The check has to look at the string.
pub(crate) fn every_drawn_string_is_printable(strings: &[&str]) -> Result<(), String> {
    for text in strings {
        if let Some(bad) = text.chars().find(|c| !(' '..='~').contains(c)) {
            return Err(format!(
                "unprintable character {bad:?} in {text:?} — the font draws a box, and no \
                 assertion over what was drawn can tell the difference"
            ));
        }
    }
    Ok(())
}

/// The court markings are behind the ball, though they were submitted after it.
///
/// The band is only visible where it disagrees with the submission order, which
/// is why `register` submits the court last. Compared by index in `quads()`,
/// which comes back in the depth sort.
pub(crate) fn the_court_markings_stay_behind_the_play(frame: &FrameRecord) -> Result<(), String> {
    let quads = frame.quads();
    let marking = quads
        .iter()
        .position(|quad| quad.tint == Color::rgba(0.6, 0.85, 1.0, 0.18));
    let ball = quads.iter().position(|quad| quad.tint == Color::WHITE);
    let (Some(marking), Some(ball)) = (marking, ball) else {
        return Err(
            "the frame has no centre-line dash or no ball in it, so the court band \
                    was not checked"
                .to_owned(),
        );
    };
    if marking < ball {
        return Ok(());
    }
    Err(format!(
        "a centre-line dash is drawn at index {marking}, over the ball at {ball} — the court \
         band is not below the play band"
    ))
}

/// What a player standing at `at` actually sees there is `what`.
///
/// `covering(p)[0]` is the depth order read backwards, so it is the front-most
/// quad — the one question a geometric assertion cannot answer. Move the score
/// from the UI band to the play band and it paints over the ball, in the right
/// place, at the right size, with every other check still passing.
///
/// Matched on tint rather than texture because the pairs worth asking about here
/// are two untextured quads — a ball over a centre-line dash — which sample the
/// same white texel and differ only in colour.
pub(crate) fn the_front_most_thing_at(
    frame: &FrameRecord,
    at: Vec2,
    expected: Color,
    what: &str,
) -> Result<(), String> {
    let covering = frame.covering(at);
    let Some(front) = covering.first() else {
        return Err(format!(
            "nothing at all is drawn at {at:?}, where {what} should be"
        ));
    };
    if front.tint == expected {
        return Ok(());
    }
    Err(format!(
        "at {at:?} the front-most quad is tinted {:?}, not {what}'s {expected:?} — \
         {} quads cover that point",
        front.tint,
        covering.len()
    ))
}

/// The winning screen and the losing screen are different screens.
///
/// A property that lives *between* two states, so no per-state assertion
/// reaches it: each end screen can be individually correct — on screen,
/// printable, centred — and both congratulate the same side.
pub(crate) fn the_two_end_screens_differ(
    winning: &FrameRecord,
    losing: &FrameRecord,
    font: BackendTextureId,
) -> Result<(), String> {
    let count = |frame: &FrameRecord| {
        frame
            .quads()
            .into_iter()
            .filter(|quad| quad.texture == font)
            .count()
    };
    let (win, lose) = (count(winning), count(losing));
    if win != lose {
        return Ok(());
    }
    Err(format!(
        "the winning and losing screens draw the same {win} glyphs — they may be the same \
         screen, which every per-screen check passes for"
    ))
}

/// The court is wide enough that a ball beating a paddle is visibly a goal.
///
/// The requirement, not the constants: there has to be room behind each paddle
/// for the ball to be seen crossing the line, otherwise a point looks like the
/// ball vanishing into the wall.
pub(crate) fn there_is_room_behind_each_paddle() -> Result<(), String> {
    let behind = HALF_W - (PADDLE_X + PADDLE_HALF_X);
    if behind >= BALL_HALF * 4.0 {
        return Ok(());
    }
    Err(format!(
        "only {behind:.2} units sit between the back of a paddle and the goal line, against \
         a ball {:.2} across — a point would look like the ball hitting the wall",
        BALL_HALF * 2.0
    ))
}

/// A paddle at either end of its travel is still fully inside the court.
pub(crate) fn a_paddle_at_its_limit_stays_on_the_court() -> Result<(), String> {
    let reach = rules::PADDLE_Y_LIMIT + PADDLE_HALF_Y;
    if reach <= HALF_H + SLACK {
        return Ok(());
    }
    Err(format!(
        "a paddle at its limit reaches {reach:.2} from the middle, past a court half \
         {HALF_H:.2} tall"
    ))
}
