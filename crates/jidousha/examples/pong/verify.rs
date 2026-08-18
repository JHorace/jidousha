//! `--verify`: the same game, driven by a controller instead of a person, with
//! assertions instead of somebody watching.
//!
//! It runs the *same* `config()` and the *same* `register()` the window does.
//! What differs is only what a player would otherwise supply — the input.
//!
//! The controller uses `SnapshotBuilder` rather than `InputScript`, because a
//! written-down script never returns a ball: it can prove W and S move a paddle
//! and still say nothing about whether the game is playable. And it plays to
//! *win* rather than to survive. A controller that merely tracks the ball
//! returns it dead flat down the middle, the machine tracks it straight back,
//! and the run reports an unplayable 0-0 with a two-hundred-touch rally — a
//! verdict about the controller wearing the costume of a verdict about the
//! game. So every return this paddle can physically produce is simulated
//! through the game's own `contact`, and the one landing furthest from anywhere
//! the machine can reach in time is the one it takes.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{FrameRecorder, SnapshotBuilder};

use crate::checks::{
    Checks, EPSILON, VIEWPORT, check_every_literal, check_the_score_reads_as_two_numbers,
    check_the_screens_never_reached, disc_at, near,
};
use crate::controller::{Snapshot, TICKS, play};
use crate::draw;
use crate::{
    BALL_RADIUS, GOAL_X, MACHINE_SPEED, MAX_BALL_SPEED, PADDLE_SIZE, PADDLE_TRAVEL, PLAYER_SPEED,
    SERVE_SPEED, WALL_Y, WINNING_SCORE,
};
use crate::{Play, Scoreboard, Side, config, register};

/// The timestep this game's constants were reasoned about, for the one check
/// that compares the engine's real `fixed_dt` against an assumption.
const ASSUMED_HZ: f32 = 60.0;

/// The swept paddle test really does catch a ball that leaps over a paddle.
///
/// The game's speed ceiling means a played match never produces one, so no
/// amount of playing exercises this: swapping `contact` for a position-only
/// test passes the whole session. But the ceiling is a tuning constant and the
/// sweep is what makes it a *safety margin* rather than the only thing standing
/// between this game and a ball through the back wall. So the contract is asked
/// directly, with a tick of travel no ball here would ever take.
fn check_the_swept_test(checks: &mut Checks) {
    let step = 1.0 / ASSUMED_HZ;
    let face = Side::Left.paddle_x() + PADDLE_SIZE.x * 0.5 + BALL_RADIUS;
    // One tick, from well in front of the paddle to well behind it: eight units
    // of travel against a paddle 0.7 thick.
    let leap = crate::contact(
        Vec2::new(face + 4.0, 0.0),
        Vec2::new(face - 4.0, 0.0),
        Side::Left,
        0.0,
        step,
    );
    checks.require(
        leap.is_some(),
        "a ball that steps clean over a paddle is not caught",
        format!(
            "a tick of travel from x={:.2} to x={:.2} across a paddle face at x={face:.2} \
             registered no contact; nothing in v1 sweeps for you, so this is the game's own \
             eight lines and they are what the speed ceiling is a margin on top of",
            face + 4.0,
            face - 4.0
        ),
    );
    // The same leap, past the end of the paddle: still a miss.
    let past = crate::contact(
        Vec2::new(face + 4.0, PADDLE_SIZE.y),
        Vec2::new(face - 4.0, PADDLE_SIZE.y),
        Side::Left,
        0.0,
        step,
    );
    checks.require(
        past.is_none(),
        "a ball that misses the paddle is counted as a hit",
        format!(
            "a crossing at y={:.2} against a paddle at y=0 half {:.2} tall was called a \
             contact",
            PADDLE_SIZE.y,
            PADDLE_SIZE.y * 0.5
        ),
    );
    // And a ball leaving the paddle is never a contact, whatever it overlaps —
    // which is what stops a ball rattling inside a paddle it has already hit.
    let leaving = crate::contact(
        Vec2::new(face - 0.1, 0.0),
        Vec2::new(face + 0.1, 0.0),
        Side::Left,
        0.0,
        step,
    );
    checks.require(
        leaving.is_none(),
        "a ball on its way out of a paddle bounces again",
        "a crossing outward through the same face was called a contact, which is how a ball \
         gets stuck rattling inside a paddle"
            .to_string(),
    );
}

/// A player who never touches the keyboard loses, and loses to a machine that
/// really is putting the ball away.
///
/// The check above says the game can be won. Alone it says nothing about
/// whether it can be *lost*: a machine paddle that only ever pushed the ball
/// back would pass every assertion in this file and be no opponent at all. So:
/// the same game, the same systems, and an `Input` that reports a player doing
/// nothing on every tick — which is not the same as inserting no `Input`, and
/// is the honest way to say "they are at the keyboard and idle".
fn check_an_idle_player_loses(checks: &mut Checks) -> (u32, u32) {
    let mut sim = headless(config(), register);
    let idle = SnapshotBuilder::new().first_tick_snapshot();
    let mut decided = None;
    for tick in 1..=TICKS {
        sim.world_mut().insert_resource(Input::new(idle.clone()));
        sim.tick();
        if matches!(sim.world().resource::<Scoreboard>().play, Play::Over { .. }) {
            decided = Some(tick);
            break;
        }
    }
    let board = *sim.world().resource::<Scoreboard>();
    checks.require(
        matches!(
            board.play,
            Play::Over {
                winner: Side::Right
            }
        ),
        "an idle player was not beaten",
        format!(
            "with nobody touching the keys for {TICKS} ticks the match stands {}-{} ({:?}); \
             a machine paddle that cannot put the ball past an unmoving one is not an \
             opponent, and every other check here would pass without one",
            board.left, board.right, board.play
        ),
    );
    checks.require(
        decided.is_some_and(|tick| tick < TICKS),
        "beating an idle player took the machine the whole run",
        format!(
            "it was still going after {TICKS} ticks at {}-{}",
            board.left, board.right
        ),
    );
    (board.right, decided.unwrap_or(TICKS) as u32)
}

pub(crate) fn run() -> ExitCode {
    let mut checks = Checks::default();
    let mut recorder = FrameRecorder::new(VIEWPORT);
    // Read out once, so every assertion below can ask "was this text?" without
    // repeating it. The id is a plain value and borrows nothing.
    let font = recorder.font_texture();

    check_every_literal(&mut checks);
    let (session, last) = play(&mut checks, &mut recorder);
    let Some(Snapshot {
        frame,
        paddles,
        ball,
    }) = last
    else {
        eprintln!("no frame was recorded at all, which the loop above cannot do");
        return ExitCode::FAILURE;
    };

    // --- what the world did ----------------------------------------------
    let board = session.board;
    checks.require(
        session.decided_at.is_some(),
        "no one won the match",
        format!(
            "after {TICKS} ticks the score is {}-{}: longest rally {} touches, top ball speed \
             {:.1} units/s across a {:.0}-unit table, ball live for {} of {TICKS} ticks, \
             {} returns by the player and {} by the machine",
            board.left,
            board.right,
            board.longest_rally,
            board.top_speed,
            GOAL_X * 2.0,
            session.live_ticks,
            session.returns,
            session.machine_returns,
        ),
    );
    checks.require(
        board.left >= WINNING_SCORE,
        "a paddle played to win did not win",
        format!(
            "the controller finished {}-{}. It plans every return the paddle can produce and \
             takes the one landing furthest from the machine, so losing to the machine means \
             the machine is too fast ({MACHINE_SPEED} against the player's {PLAYER_SPEED}) or \
             the bounce angle is too narrow to put the ball anywhere it cannot reach",
            board.left, board.right
        ),
    );
    checks.require(
        board.longest_rally >= 4,
        "the ball never went back and forth",
        format!(
            "the longest rally was {} touches over {} returns by the player and {} by the \
             machine; a rally of one is a serve nobody returned, which means the paddles are \
             not catching the ball at all",
            board.longest_rally, session.returns, session.machine_returns
        ),
    );
    checks.require(
        session.machine_returns >= 3,
        "the machine paddle is not really playing",
        format!(
            "it returned the ball {} times over a whole match; an opponent that returns \
             nothing is a wall with a score attached",
            session.machine_returns
        ),
    );
    checks.require(
        board.top_speed > SERVE_SPEED + 0.5 && board.top_speed <= MAX_BALL_SPEED + EPSILON,
        "the ball does not speed up, or does not stop speeding up",
        format!(
            "top speed was {:.2}; a serve leaves at {SERVE_SPEED} and the ceiling is \
             {MAX_BALL_SPEED}",
            board.top_speed
        ),
    );
    checks.require(
        session.ball_extent.x <= GOAL_X + EPSILON && session.ball_extent.y <= WALL_Y + EPSILON,
        "the ball left the table",
        format!(
            "it reached ({:.3}, {:.3}) from the middle; the table is {GOAL_X} by {WALL_Y}",
            session.ball_extent.x, session.ball_extent.y
        ),
    );
    checks.require(
        session.paddle_extent <= PADDLE_TRAVEL + EPSILON,
        "a paddle hung through a wall",
        format!(
            "a paddle centre reached {:.3}; the clamp is +/-{PADDLE_TRAVEL}, which is the wall \
             less half a paddle",
            session.paddle_extent
        ),
    );

    // --- the tunnelling budget, against the timestep the engine gave us ----
    // A fixed timestep means collisions are only ever tested at tick
    // boundaries, and nothing in v1 sweeps for you. The swept paddle test this
    // game writes covers that, but the wall bounce is position-based and the
    // margin is worth stating against the real `fixed_dt` rather than against
    // the 1/60 this was written with.
    let mut probe = headless(config(), register);
    probe.tick();
    let fixed_dt = probe.world().resource::<Time>().fixed_dt.as_f32();
    checks.require(
        near(fixed_dt, 1.0 / ASSUMED_HZ),
        "the timestep is not the one this game's constants were chosen for",
        format!(
            "fixed_dt is {fixed_dt} seconds, not 1/{ASSUMED_HZ}; every speed here is per second \
             so the game still plays the same, but the tunnelling margin below was reasoned \
             about at {ASSUMED_HZ} Hz"
        ),
    );
    let per_tick = MAX_BALL_SPEED * fixed_dt;
    checks.require(
        per_tick < PADDLE_SIZE.x,
        "the ball can cross a paddle in one tick",
        format!(
            "{MAX_BALL_SPEED} units/s is {per_tick:.3} units per tick against a paddle \
             {} thick",
            PADDLE_SIZE.x
        ),
    );

    // --- what was drawn --------------------------------------------------
    let glyphs = frame
        .quads()
        .iter()
        .filter(|quad| quad.texture == font)
        .count();
    checks.require(
        glyphs > 0,
        "nothing on screen sampled the font atlas",
        "the score and the hint line are both text, so a frame with no font quad has lost both"
            .to_string(),
    );

    // The paddles: a paddle-*shaped* quad at each paddle's own position, read
    // back out of the world rather than written down here, so this asks whether
    // drawing agrees with simulation. "Something is drawn there" is not enough
    // — the dashed middle and the score wander over most of the table, and that
    // question passes with a paddle deleted.
    for (side, at) in &paddles {
        // Not merely "a paddle-sized quad covers this point": a paddle drawn
        // half its own length out of position still covers its own centre, and
        // that version of this check passed a deliberately broken build. The
        // quad has to be *centred* on the paddle as well as the size of one.
        let shaped = frame.covering(*at).into_iter().any(|quad| {
            let bounds = quad.bounds();
            let size = bounds.size();
            near(size.x, PADDLE_SIZE.x)
                && near(size.y, PADDLE_SIZE.y)
                && near(bounds.center().x, at.x)
                && near(bounds.center().y, at.y)
        });
        checks.require(
            shaped,
            "no paddle-shaped quad was drawn centred where a paddle is",
            format!(
                "the world puts the {} paddle at ({:.2}, {:.2}) and it is {} by {}; the quads \
                 covering that point are {:?}",
                side.name(),
                at.x,
                at.y,
                PADDLE_SIZE.x,
                PADDLE_SIZE.y,
                frame
                    .covering(*at)
                    .into_iter()
                    .map(|quad| quad.bounds())
                    .collect::<Vec<_>>(),
            ),
        );
        checks.require(
            near(at.x, side.paddle_x()),
            "a paddle wandered off its end of the table",
            format!(
                "the {} paddle is at x={:.3}, and its end is x={:.3}",
                side.name(),
                at.x,
                side.paddle_x()
            ),
        );
    }

    // The ball, as the union of the wedges covering its centre.
    match disc_at(&frame, ball) {
        None => checks.require(
            false,
            "nothing at all was drawn where the ball is",
            format!("the world has it at ({:.2}, {:.2})", ball.x, ball.y),
        ),
        Some(size) => checks.require(
            near(size.x, BALL_RADIUS * 2.0) && near(size.y, BALL_RADIUS * 2.0),
            "no ball-sized disc where the ball is",
            format!(
                "the quads covering ({:.2}, {:.2}) span {:.4} by {:.4}; a disc of radius \
                 {BALL_RADIUS} is {:.4} square",
                ball.x,
                ball.y,
                size.x,
                size.y,
                BALL_RADIUS * 2.0
            ),
        ),
    }

    // The score, where the layout says it is: the middle of each digit's cell.
    for (side, x) in [(Side::Left, -draw::SCORE_X), (Side::Right, draw::SCORE_X)] {
        let middle = Vec2::new(x, draw::SCORE_TOP + draw::SCORE_SIZE * 0.5);
        checks.require(
            frame
                .covering(middle)
                .into_iter()
                .any(|quad| quad.texture == font),
            "a score digit is not where the game draws it",
            format!(
                "no glyph covers ({:.2}, {:.2}), the middle of the {} score's cell as \
                 TextStyle::width_of centres it",
                middle.x,
                middle.y,
                side.name()
            ),
        );
    }

    // "On screen" is not "in the right place": the hint belongs *outside* the
    // table, in the strip below the bottom wall, not written across the play.
    checks.require(
        draw::HINT_TOP > WALL_Y,
        "the hint line is drawn across the playfield",
        format!(
            "its top edge is at {:.2} and the bottom wall is at {WALL_Y}",
            draw::HINT_TOP
        ),
    );
    let hint_middle = Vec2::new(0.0, draw::HINT_TOP + draw::HINT_SIZE * 0.5);
    checks.require(
        frame
            .covering(hint_middle)
            .into_iter()
            .any(|quad| quad.texture == font),
        "the hint line is not where the game draws it",
        format!(
            "no glyph covers ({:.2}, {:.2}), the middle of a line centred by width_of",
            hint_middle.x, hint_middle.y
        ),
    );
    // And the table itself is inside the camera, which is what makes every
    // "inside the table" bound above worth anything.
    let table = draw::table();
    let view = Camera {
        viewport: VIEWPORT,
        height: crate::VIEW_HEIGHT,
        ..Camera::default()
    }
    .visible_bounds();
    checks.require(
        view.contains_rect(table),
        "the table does not fit in the camera",
        format!("the table is {table:?} and the camera shows {view:?}"),
    );

    check_the_score_reads_as_two_numbers(&mut checks, &frame, font);
    check_the_swept_test(&mut checks);
    check_the_screens_never_reached(&mut checks, &mut recorder);
    let (idle_loss, idle_ticks) = check_an_idle_player_loses(&mut checks);

    let verdict = checks.verdict();
    let decided = session.decided_at.unwrap_or(TICKS);
    println!(
        "verified pong: {}-{} to the {} paddle in {:.1}s of play",
        board.left,
        board.right,
        if board.left >= board.right {
            Side::Left.name()
        } else {
            Side::Right.name()
        },
        decided as f32 * fixed_dt,
    );
    println!(
        "  match: decided on tick {decided}, ball live for {} ticks, longest rally {} touches",
        session.live_ticks, board.longest_rally
    );
    println!(
        "  returns: {} by the player, {} by the machine; top ball speed {:.1} of a possible \
         {MAX_BALL_SPEED} units/s",
        session.returns, session.machine_returns, board.top_speed
    );
    println!(
        "  extents: ball reached ({:.2}, {:.2}) on a {GOAL_X} by {WALL_Y} table, paddles \
         reached {:.2} of {PADDLE_TRAVEL}",
        session.ball_extent.x, session.ball_extent.y, session.paddle_extent
    );
    println!(
        "  idle player: beaten {idle_loss}-0 in {:.1}s, so the match can be lost as well as won",
        idle_ticks as f32 * fixed_dt,
    );
    println!(
        "  last frame: {} quads, {glyphs} of them glyphs; {} frames recorded",
        frame.quad_count(),
        recorder.frames().len()
    );
    // `FrameRecord::transcript`, not `FrameRecorder::transcript`. The recorder's
    // renders *every* frame it holds, which for this run is 1263 of them and
    // 121,465 lines. The record's is the one frame, which is what the evidence
    // after a verdict is supposed to be.
    print!("{}", frame.transcript());
    verdict
}
