//! What a played session is judged against.
//!
//! `cargo run -p jidousha --example pong -- --verify` runs this. It builds the
//! *same* systems and the same config the window does, so what passes here is
//! what a person plays.
//!
//! Every failure prints the numbers it judged, because nobody running this can
//! look at the game. "No one won" says nothing; "no one won: score 3-0 after
//! 5400 ticks, longest rally 27, top ball speed 26.0 units/s" says the rallies
//! are too long for the field, and says it immediately.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecorder, InputSnapshot};

use crate::draw::{BANNER_SIZE, BANNER_TOP};
use crate::session::{SLACK, TICKS, VIEWPORT, escaped, on_screen, play};
use crate::{
    BALL_RADIUS, FIELD, MAX_SPEED, PADDLE_LIMIT, PADDLE_SIZE, Round, SERVE_SPEED, Scoreboard, Side,
    VIEW_HEIGHT, WINNING_SCORE, config, greater, register,
};

/// Fail with the engine's message shape, and a non-zero exit.
pub(super) fn fail(what: &str, specifics: &str) -> ! {
    eprintln!(
        "{}",
        message(
            what,
            specifics,
            "the game changed, or the engine did",
            "run `cargo run -p jidousha --example pong` and watch it, then compare with \
             the assertion above",
        )
    );
    std::process::exit(1);
}

/// Draw the screen a played session never reaches, and check it fits.
///
/// The controller here is a perfect tracker that also aims, so it wins every
/// time and the machine's own victory banner — the longest string in the game —
/// is never drawn by `play`. It is put on screen by hand instead: one tick to
/// let Startup run, then the scoreboard is set to the result that a person will
/// eventually see and the frame is checked like any other.
fn check_the_losing_screen() {
    let mut sim = headless(config(), register);
    sim.world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));
    sim.tick();

    let board = sim.world_mut().resource_mut::<Scoreboard>();
    board.left = WINNING_SCORE - 3;
    board.right = WINNING_SCORE;
    board.round = Round::Over {
        winner: Side::Right,
    };

    let mut recorder = FrameRecorder::new(VIEWPORT);
    let font = recorder.font_texture();
    let frame = recorder.draw(&mut sim);
    if let Some(bounds) = escaped(frame, on_screen()) {
        let (top_left, bottom_right) = on_screen();
        fail(
            "the machine's victory screen does not fit on screen",
            &format!(
                "a quad covers {:.2},{:.2} to {:.2},{:.2} against a camera showing \
                 {:.2},{:.2} to {:.2},{:.2} — this is the one screen a won match never \
                 draws, and width_of is exact and silent",
                bounds.min.x,
                bounds.min.y,
                bounds.max.x,
                bounds.max.y,
                top_left.x,
                top_left.y,
                bottom_right.x,
                bottom_right.y,
            ),
        );
    }
    // Fitting on screen is not the same as being there: a headline that was
    // never drawn is trivially inside every boundary. So ask whether a glyph
    // actually landed in the band the banner occupies.
    let headline = frame.quads().iter().any(|quad| {
        let bounds = quad.bounds();
        quad.texture == font
            && !greater(BANNER_TOP - SLACK, bounds.min.y)
            && !greater(bounds.max.y, BANNER_TOP + BANNER_SIZE + SLACK)
    });
    if !headline {
        let glyphs = frame
            .quads()
            .iter()
            .filter(|quad| quad.texture == font)
            .count();
        fail(
            "the machine's victory screen has no headline on it",
            &format!(
                "{glyphs} glyphs were drawn, and none of them lands between y = \
                 {BANNER_TOP:.2} and {:.2}, which is the band \"the machine wins\" is \
                 drawn in",
                BANNER_TOP + BANNER_SIZE,
            ),
        );
    }
}

pub fn run() {
    check_the_losing_screen();
    let played = play();

    // --- the tuning is physically possible ---------------------------
    // A ball that travels further in one tick than the paddle is thick steps
    // straight through it, and the symptom is a point nobody can explain. The
    // timestep comes from the engine rather than from a 1/60 written down
    // here, so raising `GameConfig::fixed_dt` is caught too.
    let step = MAX_SPEED * played.fixed_dt;
    let thickness = PADDLE_SIZE.x * 0.5 + BALL_RADIUS;
    if !greater(thickness, step) {
        fail(
            "the ball can pass through a paddle without touching it",
            &format!(
                "at {MAX_SPEED} units/s and a {:.4}s tick it moves {step:.3} units between \
                 collision checks, and a paddle is only {thickness:.3} units of overlap \
                 deep — raise the paddle's width, lower the speed, or shorten the tick",
                played.fixed_dt,
            ),
        );
    }

    // --- the ball stayed on the field --------------------------------
    let wall = FIELD.y - BALL_RADIUS;
    if greater(played.ball_extent.y, wall + SLACK) {
        fail(
            "the ball went through a wall",
            &format!(
                "it reached y = {:.4}; the walls are at +/-{:.2} and the ball's radius is \
                 {BALL_RADIUS}, so {wall:.2} is as far as its centre may get",
                played.ball_extent.y, FIELD.y,
            ),
        );
    }
    if greater(played.ball_extent.x, FIELD.x + SLACK) {
        fail(
            "the ball was left outside the goal line",
            &format!(
                "it reached x = {:.4}; a point is scored past +/-{:.2} and the ball is put \
                 back on the centre spot in the same tick",
                played.ball_extent.x, FIELD.x,
            ),
        );
    }
    if greater(played.paddle_extent, PADDLE_LIMIT + SLACK) {
        fail(
            "a paddle left the field",
            &format!(
                "one reached y = {:.4}; the clamp is +/-{PADDLE_LIMIT:.2}, which is the \
                 wall at {:.1} less half of the paddle's {} height",
                played.paddle_extent, FIELD.y, PADDLE_SIZE.y,
            ),
        );
    }

    // --- it is a game of Pong ----------------------------------------
    if played.left_hits < 3 || played.right_hits < 3 {
        fail(
            "the ball was not rallied",
            &format!(
                "the left paddle returned it {} times and the right {} over {TICKS} ticks; \
                 a game where neither side can return a serve is not playable",
                played.left_hits, played.right_hits,
            ),
        );
    }
    if greater(played.top_speed, MAX_SPEED + SLACK) || !greater(played.top_speed, SERVE_SPEED) {
        fail(
            "the ball's speed did not do what a rally does to it",
            &format!(
                "it topped out at {:.2} units/s; a serve leaves at {SERVE_SPEED} and every \
                 paddle touch speeds it up, capped at {MAX_SPEED}",
                played.top_speed,
            ),
        );
    }

    // --- somebody won, and Space started another match ----------------
    let Some(won_at) = played.won_at else {
        fail(
            "no one won",
            &format!(
                "score {}-{} after {TICKS} ticks, {} + {} paddle touches, longest rally {}, \
                 top ball speed {:.1} units/s — a match to {WINNING_SCORE} that does not \
                 finish in ninety seconds is too slow to be fun",
                played.left,
                played.right,
                played.left_hits,
                played.right_hits,
                played.longest_rally,
                played.top_speed,
            ),
        );
    };
    if played.left.max(played.right) != WINNING_SCORE {
        fail(
            "the match ended on the wrong score",
            &format!(
                "it was {}-{} when the winner's banner appeared on tick {won_at}; \
                 {WINNING_SCORE} is the target",
                played.left, played.right,
            ),
        );
    }
    // The controller plays a perfect tracker against a paddle that is slower
    // and starts late, so the person's side is the one that must be able to
    // win. A game only the machine can win is not a game.
    if played.left != WINNING_SCORE {
        fail(
            "the machine beat a player who never missed the ball",
            &format!(
                "final score {}-{}: the opponent moves at {} units/s against the player's \
                 {}, and does not start tracking until the ball crosses x = {}",
                played.left,
                played.right,
                crate::OPPONENT_SPEED,
                crate::PLAYER_SPEED,
                crate::OPPONENT_WAKES_AT,
            ),
        );
    }
    let Some(restarted_at) = played.restarted_at else {
        fail(
            "space did not start another match",
            &format!(
                "the match was won on tick {won_at} and the run went on to {TICKS} without \
                 leaving the winner's screen",
            ),
        );
    };

    // --- what was drawn ------------------------------------------------
    if played.frames != TICKS as usize {
        fail(
            "one frame per tick was expected",
            &format!("{} frames for {TICKS} ticks", played.frames),
        );
    }
    if played.frames_with_text != played.frames {
        fail(
            "the score was not on screen the whole time",
            &format!(
                "{} of {} frames drew a glyph; the score is drawn every frame, so a frame \
                 without one has lost it",
                played.frames_with_text, played.frames,
            ),
        );
    }
    if !played.score_drawn {
        fail(
            "the score is not where the game draws it",
            "no glyph covers the middle of a score centred by TextStyle::width_of, which is \
             where the colon between the two numbers belongs",
        );
    }
    if let Some((tick, bounds)) = played.escaped {
        fail(
            "something was drawn off screen",
            &format!(
                "on tick {tick} a quad covered {:.2},{:.2} to {:.2},{:.2}, and the camera \
                 shows {:.2} by {:.2} — text centred by width_of is the usual culprit",
                bounds.min.x,
                bounds.min.y,
                bounds.max.x,
                bounds.max.y,
                VIEW_HEIGHT * VIEWPORT.aspect(),
                VIEW_HEIGHT,
            ),
        );
    }

    // --- and it does the same thing twice ------------------------------
    let again = play();
    if again.ball_track != played.ball_track {
        let first = played
            .ball_track
            .iter()
            .zip(&again.ball_track)
            .position(|(a, b)| a != b);
        fail(
            "the same session played twice went two different ways",
            &format!(
                "the ball's path first differs on tick {:?}; the timestep is fixed and the \
                 generator is seeded from GameConfig, so two runs of one script are the \
                 same run",
                first.map(|index| index + 1),
            ),
        );
    }

    println!("verified pong over {TICKS} ticks");
    println!(
        "  match: {}-{} to the player, won on tick {won_at}, restarted on tick {restarted_at}",
        played.left, played.right,
    );
    println!(
        "  rallies: {} + {} paddle touches, longest {}, top ball speed {:.1} units/s",
        played.left_hits, played.right_hits, played.longest_rally, played.top_speed,
    );
    println!(
        "  bounds: ball reached ({:.2}, {:.2}), paddles reached {:.2} of {PADDLE_LIMIT:.2}",
        played.ball_extent.x, played.ball_extent.y, played.paddle_extent,
    );
    println!(
        "  frames: {}, all of them with text, nothing outside the camera",
        played.frames,
    );
    println!("  replayed the session: identical to the bit");
    print!("{}", played.transcript);
}
