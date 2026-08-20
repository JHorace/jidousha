//! `--verify`: play a whole match headless, assert on what the world did and
//! on what was drawn, capture one frame as a picture, print a verdict.
//!
//! It runs the *same* systems and the same config the window does. What differs
//! is only what a person would otherwise supply: the left paddle is driven by
//! [`crate::controller`] instead of by hands.
//!
//! Three kinds of check live here, and they answer different questions:
//!
//! - **The match.** Somebody wins, both sides score, rallies happen, the
//!   opponent returns a reasonable share of what reaches it. This is the only
//!   evidence that the game is a *game* rather than a correct simulation of a
//!   walkover.
//! - **The contracts a played session cannot reach.** The safety margins a game
//!   is built on are exactly the states a correct game never enters: the swept
//!   contact test never does anything a naive position test would not, because
//!   the speed cap means the ball cannot tunnel. So [`crate::crossing`] is
//!   asked its contract directly, and the cap is asserted against the timestep
//!   the engine actually handed the game rather than against the 1/60 it was
//!   written for.
//! - **The screens the run never reaches.** A controller good enough to win is
//!   a controller that never loses, so the losing banner — the longest string
//!   in the game — is the one string a played session never measures. Those are
//!   staged by hand, three lines each.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, DrawnQuad, FrameRecord, FrameRecorder, InputEvent, InputScript,
    InputSnapshot, SnapshotBuilder,
};

use crate::checks::{Checks, disc_span, fail, greater, near, sizes_covering, within};
use crate::controller::{Approach, Controller, OPPONENT_REACH};
use crate::{
    BALL_LIMIT, BALL_RADIUS, BALL_SPEED_MAX, BALL_SPEED_START, Ball, CONTACT_REACH, CONTACT_X,
    COURT_HALF_HEIGHT, Flight, GOAL_LINE, HINT, HINT_SIZE, PADDLE_LIMIT, PADDLE_SIZE, Paddle,
    SCORE_SIZE, SCORE_TOP, Scoreboard, Side, Stage, VIEW_HEIGHT, WINNING_SCORE, advance,
    ball_flight, config, crossing, dash_y, register,
};

/// How many ticks the match is given to finish.
///
/// A generous ceiling rather than a schedule: the run stops the tick somebody
/// reaches [`WINNING_SCORE`], and a run that uses all of these has failed a
/// check about the match being winnable in a sitting.
pub(crate) const TICKS: u64 = 7_000;

/// How long a match may take and still be a prototype somebody would play.
///
/// Thirty seconds is the honest bar; this is a minute and a half of ticks, so
/// the assertion fires on a game that has become a war of attrition rather than
/// on a slow rally.
const PLAYABLE_TICKS: u64 = 2_700;

/// How many ticks the do-nothing run gets to lose a point.
const LOSING_TICKS: u64 = 900;

/// The surface the headless run draws to.
///
/// The same shape and size the window opens at, so the camera the assertions
/// read and the viewport the frames were planned with are the same rectangle.
const VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// Where the top of the hint line sits, from `draw_the_furniture`.
const HINT_TOP: f32 = VIEW_HEIGHT * 0.5 - HINT_SIZE - 0.25;

/// What one played match produced.
pub(crate) struct Session {
    /// The score and the stage at the end.
    board: Scoreboard,
    /// How many ticks it took.
    ticks: u64,
    /// One entry per ball that came at the player.
    approaches: Vec<Approach>,
    /// How many balls reached each side's contact plane.
    reached: [u32; 2],
    /// How many of those each side actually returned.
    returned: [u32; 2],
    /// The most touches one point took.
    longest_rally: u32,
    /// The fastest the ball ever went, in world units per second.
    top_speed: f32,
    /// The furthest a single tick ever moved the ball, in world units.
    worst_travel: f32,
    /// The furthest outside the court the ball's centre ever got, in Y.
    worst_escape: f32,
    /// The furthest past a goal line the ball ever got before the point ended.
    worst_overrun: f32,
    /// How many frames were recorded.
    frames: usize,
    /// The first quad that was drawn off screen, and which tick drew it.
    off_screen: Option<(u64, Rect)>,
    /// The last frame drawn while the ball was live.
    ///
    /// Not the very last frame: that one has the winner's banner over it, and
    /// every geometric assertion below wants a picture of the game being
    /// *played*. The banner gets its own staged frames further down.
    last: FrameRecord,
    /// The score on the frame `last` recorded.
    shown: Scoreboard,
    /// Where the two paddles were on that frame.
    paddles: [Vec2; 2],
    /// Where the ball was on that frame.
    ball: Vec2,
    /// Which backend texture the font landed on.
    font: BackendTextureId,
    /// The camera the frames were planned with.
    camera: Camera,
}

/// Both paddles' Y, indexed by [`Side::index`].
fn paddles_of(world: &World) -> [f32; 2] {
    let mut out = [0.0_f32; 2];
    for (_, transform, paddle) in world.query::<(&Transform, &Paddle)>() {
        out[paddle.side.index()] = transform.pos.y;
    }
    out
}

/// Both paddles' whole position, indexed by [`Side::index`].
fn paddle_points(world: &World) -> [Vec2; 2] {
    let mut out = [Vec2::ZERO; 2];
    for (_, transform, paddle) in world.query::<(&Transform, &Paddle)>() {
        out[paddle.side.index()] = transform.pos;
    }
    out
}

/// Put the ball somewhere, for a frame nobody played.
fn place_ball(sim: &mut HeadlessSim, at: Vec2) {
    for (_, transform, _) in sim.world_mut().query_mut::<(&mut Transform, &Ball)>() {
        transform.pos = at;
    }
}

/// Play the match, recording every frame.
fn play() -> (Session, HeadlessSim, FrameRecorder) {
    let mut sim = headless(config(), register);
    let mut recorder = FrameRecorder::new(VIEWPORT);
    let font = recorder.font_texture();
    // `Time` is there before the first tick, which is what lets the controller
    // convert its speeds into steps without having ticked once.
    let dt = sim.world().resource::<Time>().fixed_dt.as_f32();

    let mut driver = Controller::new();
    let mut reached = [0_u32; 2];
    let mut returned = [0_u32; 2];
    let mut rally = 0_u32;
    let mut longest_rally = 0_u32;
    let mut top_speed = 0.0_f32;
    let mut worst_travel = 0.0_f32;
    let mut worst_escape = 0.0_f32;
    let mut worst_overrun = 0.0_f32;
    let mut off_screen = None;
    let mut live: Option<(FrameRecord, Scoreboard, [Vec2; 2], Vec2)> = None;
    let mut ticks = 0_u64;

    for tick in 1..=TICKS {
        let before = ball_flight(sim.world());
        let paddles = paddles_of(sim.world());
        let stage_before = sim.world().find_resource::<Scoreboard>().map(|b| b.stage);
        let snapshot = driver.snapshot(
            tick,
            before,
            paddles[Side::Left.index()],
            paddles[Side::Right.index()],
            dt,
        );
        sim.world_mut().insert_resource(Input::new(snapshot));
        sim.tick();
        ticks = tick;

        let Some(after) = ball_flight(sim.world()) else {
            fail(
                "the ball is gone",
                "Startup spawns exactly one and nothing despawns it",
            );
        };
        let now = paddles_of(sim.world());
        let board = *sim.world().resource::<Scoreboard>();

        // A paddle touch is the tick the ball's X reverses. Walls flip Y, and
        // a serve starts from a standstill, so neither is mistaken for one.
        if let (Some(before), Some(Stage::Rally)) = (before, stage_before)
            && matches!(board.stage, Stage::Rally)
        {
            let hit = if greater(0.0, before.vel.x) && greater(after.vel.x, 0.0) {
                Some(Side::Left)
            } else if greater(before.vel.x, 0.0) && greater(0.0, after.vel.x) {
                Some(Side::Right)
            } else {
                None
            };
            if let Some(side) = hit {
                reached[side.index()] += 1;
                returned[side.index()] += 1;
                rally += 1;
                longest_rally = longest_rally.max(rally);
                driver.saw_touch(side, after, now[Side::Right.index()], dt);
            }
        }

        // A point ends the rally, and the side whose goal it went into is the
        // side that had a ball reach it and did not return it.
        if matches!(stage_before, Some(Stage::Rally)) && !matches!(board.stage, Stage::Rally) {
            let conceded = if greater(before.map_or(0.0, |f| f.pos.x), 0.0) {
                Side::Right
            } else {
                Side::Left
            };
            reached[conceded.index()] += 1;
            rally = 0;
            driver.saw_dead_ball();
        }

        let speed = after.vel.length();
        top_speed = top_speed.max(speed);
        worst_travel = worst_travel.max(speed * dt);
        worst_escape = worst_escape.max(after.pos.y.abs() - BALL_LIMIT);
        worst_overrun = worst_overrun.max(after.pos.x.abs() - GOAL_LINE);

        let frame = recorder.draw(&mut sim);
        if off_screen.is_none() {
            let view = Camera {
                viewport: VIEWPORT,
                ..*sim.world().resource::<Camera>()
            }
            .visible_bounds();
            off_screen = frame
                .quads()
                .iter()
                .map(|quad| quad.bounds())
                .find(|bounds| !view.contains_rect(*bounds))
                .map(|bounds| (tick, bounds));
        }
        if matches!(board.stage, Stage::Rally) {
            live = Some((frame, board, paddle_points(sim.world()), after.pos));
        }

        if matches!(board.stage, Stage::Over { .. }) {
            break;
        }
    }

    let Some((last, shown, paddles, ball)) = live else {
        fail(
            "no frame was drawn with the ball in play",
            "the loop above draws every tick and the match spends most of them rallying",
        );
    };
    let session = Session {
        board: *sim.world().resource::<Scoreboard>(),
        ticks,
        approaches: driver.approaches.clone(),
        reached,
        returned,
        longest_rally,
        top_speed,
        worst_travel,
        worst_escape,
        worst_overrun,
        frames: ticks as usize,
        off_screen,
        last,
        shown,
        paddles,
        ball,
        font,
        camera: Camera {
            viewport: VIEWPORT,
            ..*sim.world().resource::<Camera>()
        },
    };
    (session, sim, recorder)
}

/// Play a second match in which the player does nothing at all.
///
/// Not the same as inserting no `Input`: this is a person sitting still, and it
/// is what proves the game can be *lost*. A run that only ever wins says
/// nothing about whether the opponent can score.
fn play_badly() -> Scoreboard {
    let mut sim = headless(config(), register);
    for _ in 1..=LOSING_TICKS {
        sim.world_mut()
            .insert_resource(Input::new(InputSnapshot::new()));
        sim.tick();
    }
    *sim.world().resource::<Scoreboard>()
}

/// Play a match with a left paddle that only chases the ball.
///
/// The middle of the three players this file runs, and the one that says
/// whether the game has a *gradient*: the rollout-driven controller above is
/// superhuman and the do-nothing run below is nobody, so neither of them can
/// answer "would this be a game". A paddle that simply follows the ball is
/// roughly what a person does on their first try, and it returns the ball dead
/// flat down the middle — which is precisely the play the opponent is best
/// against.
fn play_naively() -> (Scoreboard, u64) {
    let mut sim = headless(config(), register);
    let dt = sim.world().resource::<Time>().fixed_dt.as_f32();
    let step = crate::PLAYER_SPEED * dt;
    let mut keyboard = SnapshotBuilder::new();
    let mut holding: Option<Key> = None;
    let mut ticks = 0;
    for tick in 1..=TICKS {
        let ball = ball_flight(sim.world());
        let paddle = sim
            .world()
            .query::<(&Transform, &Paddle)>()
            .find(|(_, _, paddle)| paddle.side == Side::Left)
            .map(|(_, transform, _)| transform.pos.y);
        // Nothing to look at on the way into tick 1: Startup runs inside it.
        let want = match (ball, paddle) {
            (Some(ball), Some(at)) if greater(ball.pos.y - at, step * 0.5) => Some(Key::S),
            (Some(ball), Some(at)) if greater(at - ball.pos.y, step * 0.5) => Some(Key::W),
            _ => None,
        };
        if want != holding {
            if let Some(old) = holding {
                keyboard.record(InputEvent::KeyReleased(old));
            }
            if let Some(new) = want {
                keyboard.record(InputEvent::KeyPressed(new));
            }
            holding = want;
        }
        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();
        ticks = tick;
        if matches!(
            sim.world().resource::<Scoreboard>().stage,
            Stage::Over { .. }
        ) {
            break;
        }
    }
    (*sim.world().resource::<Scoreboard>(), ticks)
}

/// Drive the player's paddle into both ends of its travel, with a script.
///
/// `InputScript` rather than a controller, because this input does not depend
/// on anything the game does back — and both holds deliberately last far longer
/// than the travel available, so the clamp is *exercised* rather than merely
/// not violated. A match never reaches it: both players aim inside their own
/// limits, so deleting the clamp altogether changes nothing a played session
/// can see. (It was deleted, on purpose, and every other check in this file
/// went on passing.)
fn play_the_clamp() -> (Vec<f32>, usize, usize) {
    let script = InputScript::new()
        .hold(Key::S, 5..300)
        .hold(Key::W, 305..700);
    let mut sim = headless(config(), register);
    let mut track = Vec::new();
    for tick in 1..=script.last_tick() {
        sim.world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        sim.tick();
        track.push(paddles_of(sim.world())[Side::Left.index()]);
    }
    // Y is down, so the bottom of the screen is the larger number.
    let bottom = peak_at(&track, greater);
    let top = peak_at(&track, |a, b| greater(b, a));
    (track, bottom, top)
}

/// Where in `track` the value `pick` prefers first appears.
fn peak_at(track: &[f32], pick: fn(f32, f32) -> bool) -> usize {
    let mut best = 0;
    for (index, value) in track.iter().enumerate() {
        if pick(*value, track[best]) {
            best = index;
        }
    }
    best
}

/// Every quad in `frame` that sampled the font.
fn glyphs(frame: &FrameRecord, font: BackendTextureId) -> Vec<DrawnQuad> {
    frame
        .quads()
        .into_iter()
        .filter(|quad| quad.texture == font)
        .collect()
}

/// How many of `glyphs` sit with their top edge on `top`, within a glyph's
/// height of `size`.
fn glyphs_in_band(quads: &[DrawnQuad], top: f32, size: f32) -> usize {
    quads
        .iter()
        .filter(|quad| {
            let bounds = quad.bounds();
            greater(bounds.min.y, top - 0.01) && greater(top + size + 0.01, bounds.max.y)
        })
        .count()
}

pub(crate) fn run() -> ExitCode {
    let mut checks = Checks::default();
    let (session, mut sim, mut recorder) = play();
    let Session {
        board,
        ticks,
        approaches,
        reached,
        returned,
        longest_rally,
        top_speed,
        worst_travel,
        worst_escape,
        worst_overrun,
        frames,
        off_screen,
        last,
        shown,
        paddles,
        ball,
        font,
        camera,
    } = session;
    let dt = sim.world().resource::<Time>().fixed_dt.as_f32();
    let view = camera.visible_bounds();

    // --- the match ------------------------------------------------------
    let winner = match board.stage {
        Stage::Over { winner } => Some(winner),
        _ => None,
    };
    checks.require(
        winner.is_some(),
        "nobody won the match inside the tick budget",
        format!(
            "score {}-{} after {ticks} ticks; longest rally {longest_rally} touches, top ball \
             speed {top_speed:.1} units/s, the player met {} of {} approaches",
            board.left,
            board.right,
            approaches.iter().filter(|a| a.met).count(),
            approaches.len(),
        ),
    );
    checks.require(
        greater(PLAYABLE_TICKS as f32, ticks as f32),
        "the match took longer than anybody would sit through",
        format!(
            "{ticks} ticks, which is {:.0} seconds at {dt:.4}s a tick; a prototype match wants \
             to be over inside {:.0}",
            ticks as f32 * dt,
            PLAYABLE_TICKS as f32 * dt,
        ),
    );
    // Not "both sides scored". The controller above searches hundreds of ticks
    // of candidate futures on every decision and never mistimes a key; asking
    // it to concede points would be asking the game to be unwinnable, and an
    // earlier draft of this file did exactly that and reported a fault against
    // a game that was fine. What a game needs is a *gradient*, and three
    // players measure it: this one wins, the chasing player below is in a close
    // match, and the do-nothing player loses.
    checks.require(
        winner == Some(Side::Left),
        "a player who searches every shot it can play still cannot win",
        format!(
            "it finished {}-{} after {ticks} ticks; if the controller's three numbers below \
             are healthy then the opponent is the thing that cannot be beaten",
            board.left, board.right
        ),
    );
    checks.require(
        longest_rally >= 3,
        "no rally lasted long enough to be a rally",
        format!(
            "the longest point took {longest_rally} paddle touches over {} points; a ball that \
             is returned once and then dies is a serve, not a rally",
            board.left + board.right
        ),
    );
    // The commonest way a first game is broken is an opponent nobody can score
    // against. Measured rather than derived, and about the game as played
    // rather than about the game at its limit.
    let opponent_share = f32::from(returned[Side::Right.index()] as u16)
        / f32::from(reached[Side::Right.index()].max(1) as u16);
    checks.require(
        greater(opponent_share, 0.5) && greater(0.999, opponent_share),
        "the opponent is not an opponent: it returns too few of the balls that reach it, or \
         every single one",
        format!(
            "it returned {} of the {} that reached it ({:.0}%); under half is a punchbag and \
             all of them is a wall nobody can score past",
            returned[Side::Right.index()],
            reached[Side::Right.index()],
            opponent_share * 100.0,
        ),
    );
    let (naive, naive_ticks) = play_naively();
    checks.require(
        matches!(naive.stage, Stage::Over { .. }) && naive.left.min(naive.right) >= 2,
        "a player who only chases the ball is not in a game",
        format!(
            "the chasing player finished {}-{} in {naive_ticks} ticks. a match it wins without \
             conceding is a game with no opponent in it; one that never finishes is the flat \
             groove — both paddles centring on the ball, returning it down the middle, and a \
             rally with nowhere to go",
            naive.left, naive.right,
        ),
    );
    let lost = play_badly();
    checks.require(
        lost.right > 0,
        "a player who does nothing at all never loses a point",
        format!(
            "after {LOSING_TICKS} ticks of no input the score is {}-{}; a game that cannot be \
             lost has no stakes",
            lost.left, lost.right
        ),
    );

    // --- the controller, which is the newer and worse-tested of the two ---
    //
    // Three numbers, because one is not the contract: "met 27 of 27" prints
    // happily alongside a 0-0 match, and a correct controller and a broken one
    // produce the same nothing. Read together they say which half to open.
    let met = approaches.iter().filter(|a| a.met).count();
    let planned: Vec<f32> = approaches.iter().filter_map(|a| a.planned_gap).collect();
    let aimed: Vec<f32> = approaches.iter().filter_map(|a| a.aim_error).collect();
    let mean = |values: &[f32]| -> f32 {
        if values.is_empty() {
            return f32::NAN;
        }
        values.iter().sum::<f32>() / values.len() as f32
    };
    let planned_gap = mean(&planned);
    let aim_error = mean(&aimed);
    checks.require(
        met * 4 >= approaches.len() * 3,
        "the controller cannot reach the ball, so nothing it reports about the game means \
         anything",
        format!(
            "it met {met} of {} approaches; below three quarters it is the driver that is \
             broken, not the game",
            approaches.len()
        ),
    );
    checks.require(
        greater(planned_gap, OPPONENT_REACH),
        "the controller's chosen shots are not threats",
        format!(
            "its returns were planned to land {planned_gap:.2} units from the opponent, whose \
             reach is {OPPONENT_REACH:.2}; it meets the ball and hits where it aims, so the \
             objective is what is wrong"
        ),
    );
    checks.require(
        greater(view.size().y * 0.25, aim_error),
        "the controller is aiming at noise: the shots it plans are not the shots it produces",
        format!(
            "its returns landed {aim_error:.2} from where they were planned, on a court \
             {:.1} tall; constrain the candidates and score them by their worst case",
            view.size().y
        ),
    );

    // --- the contracts a played session never exercises -------------------
    //
    // The speed cap means the ball cannot tunnel, so the swept test never does
    // anything a position test would not and replacing it with one would pass
    // the entire match. Ask the function its contract directly instead.
    checks.require(
        greater(PADDLE_SIZE.x, BALL_SPEED_MAX * dt),
        "the ball can move further in one tick than a paddle is thick",
        format!(
            "{BALL_SPEED_MAX:.1} units/s at {dt:.5}s a tick is {:.3} units of travel against a \
             paddle {:.2} thick; collisions are only tested at tick boundaries, so nothing \
             below this margin is caught by geometry at all",
            BALL_SPEED_MAX * dt,
            PADDLE_SIZE.x,
        ),
    );
    let face = -CONTACT_X;
    let reach = (-CONTACT_REACH, CONTACT_REACH);
    let across = crossing(
        Vec2::new(face + 4.0, 0.0),
        Vec2::new(face - 4.0, 0.0),
        face,
        -1.0,
        reach,
    );
    let past_the_end = crossing(
        Vec2::new(face + 4.0, CONTACT_REACH + 0.5),
        Vec2::new(face - 4.0, CONTACT_REACH + 0.5),
        face,
        -1.0,
        reach,
    );
    let leaving = crossing(
        Vec2::new(face - 0.1, 0.0),
        Vec2::new(face - 0.5, 0.0),
        face,
        -1.0,
        reach,
    );
    let receding = crossing(
        Vec2::new(face + 0.1, 0.0),
        Vec2::new(face + 4.0, 0.0),
        face,
        -1.0,
        reach,
    );
    checks.require(
        across.is_some_and(|t| near(t, 0.5)),
        "the sweep misses a ball that crosses the whole paddle inside one tick",
        format!(
            "eight units of travel through a plane four units away answered {across:?}, want \
             the crossing at 0.5 of the tick; this is the case a position test cannot see and \
             the only reason the sweep exists"
        ),
    );
    checks.require(
        past_the_end.is_none(),
        "the sweep bats back a ball that passes the end of the paddle",
        format!(
            "a crossing {:.2} above a paddle reaching {CONTACT_REACH:.2} answered \
             {past_the_end:?}, want None",
            CONTACT_REACH + 0.5
        ),
    );
    checks.require(
        leaving.is_none(),
        "the sweep catches a ball that is leaving through the face it came in by",
        format!(
            "a ball already past the plane and still moving away answered {leaving:?}, want \
             None; catching it is how a ball sticks to a paddle"
        ),
    );
    checks.require(
        receding.is_none(),
        "the sweep catches a ball moving away from the paddle",
        format!("a ball travelling +X at a -X face answered {receding:?}, want None"),
    );
    // And the same case through `advance`, because the sweep is only useful if
    // the mover consults it: eight units in one tick, straight through.
    let tunnel = advance(
        Flight {
            pos: Vec2::new(-9.0, 0.0),
            vel: Vec2::new(-8.0 / dt, 0.0),
        },
        [0.0, 0.0],
        dt,
    );
    checks.require(
        tunnel.touched == Some(Side::Left) && greater(tunnel.flight.vel.x, 0.0),
        "a ball moving eight units in one tick goes straight through the paddle",
        format!(
            "it ended at ({:.2}, {:.2}) going ({:.1}, {:.1}) and touched {:?}; the whole travel \
             is on the far side of the paddle, so an end-of-tick overlap test finds nothing",
            tunnel.flight.pos.x,
            tunnel.flight.pos.y,
            tunnel.flight.vel.x,
            tunnel.flight.vel.y,
            tunnel.touched,
        ),
    );

    // The wall, asked the same way. A ball driven into it must come back with
    // its Y reversed and its centre inside the court — the case a played match
    // reaches constantly and the reason it is cheap to state exactly.
    let bounced = advance(
        Flight {
            pos: Vec2::new(0.0, BALL_LIMIT - 0.05),
            vel: Vec2::new(0.0, 20.0),
        },
        [PADDLE_LIMIT * 4.0, PADDLE_LIMIT * 4.0],
        dt,
    );
    checks.require(
        bounced.walled
            && greater(0.0, bounced.flight.vel.y)
            && greater(BALL_LIMIT + 0.001, bounced.flight.pos.y),
        "a ball driven into the wall does not come back",
        format!(
            "it ended at y {:.4} going {:.2}/s, walled = {}; the wall is at \
             {BALL_LIMIT:.2} for the ball's centre",
            bounced.flight.pos.y, bounced.flight.vel.y, bounced.walled,
        ),
    );

    // The order the systems run in, which is a claim `advance` makes in prose
    // and nothing else in this file can see. Swapping these two lines in
    // `register` makes the ball consult the paddles where the *previous* tick
    // left them, so a paddle closing on the ball is a paddle the ball passes
    // through — and every assertion about where things ended up goes on
    // passing, because the two orders differ by one tick of a paddle's travel.
    let schedule = sim.schedule_debug();
    let ordered = |first: &str, second: &str| match (schedule.find(first), schedule.find(second)) {
        (Some(a), Some(b)) => a < b,
        _ => false,
    };
    checks.require(
        ordered("run_the_clock", "drive_the_paddles")
            && ordered("drive_the_paddles", "move_the_ball")
            && ordered("move_the_ball", "keep_score"),
        "the Update systems do not run in the order the ball's arithmetic assumes",
        format!(
            "`advance` treats each paddle as standing still at its post-move position for the \
             whole tick, which is only true if the paddles move first. the schedule is:\n{schedule}"
        ),
    );

    // The paddle's clamp, driven into both ends by a script. A played match
    // never touches it: both players aim inside their own limits.
    let (track, bottom_at, top_at) = play_the_clamp();
    let (bottom, top) = (track[bottom_at], track[top_at]);
    checks.require(
        near(bottom, PADDLE_LIMIT) && near(top, -PADDLE_LIMIT),
        "the paddle does not come to rest against both ends of its travel",
        format!(
            "held against each end for hundreds of ticks it reached {bottom:.3} and {top:.3}; \
             the clamp is +/-{PADDLE_LIMIT:.2}, which is the wall at \
             {COURT_HALF_HEIGHT:.1} less half a paddle"
        ),
    );
    // Down first, then up. Both extremes are reached either way round, so only
    // the order tells a swapped pair of keys apart.
    checks.require(
        bottom_at < top_at,
        "S and W move the paddle the wrong way round",
        format!(
            "the script holds S first, but the paddle was at the top on tick {} before it was \
             at the bottom on tick {}",
            top_at + 1,
            bottom_at + 1,
        ),
    );

    // --- what the world did over the match -------------------------------
    checks.require(
        greater(0.001, worst_escape),
        "the ball got outside the court",
        format!(
            "its centre was {worst_escape:.4} units past the wall at its worst; the walls are \
             at +/-{COURT_HALF_HEIGHT:.1} and the ball's radius is {BALL_RADIUS:.2}, so its \
             centre may reach +/-{BALL_LIMIT:.2}"
        ),
    );
    checks.require(
        greater(worst_travel, 0.001) && greater(BALL_SPEED_MAX * dt + 0.001, worst_travel),
        "a tick moved the ball further than the speed cap allows, or never moved it at all",
        format!(
            "the worst tick moved it {worst_travel:.4} units; the cap is \
             {:.4} and a match that never moves the ball is not a match",
            BALL_SPEED_MAX * dt
        ),
    );
    checks.require(
        greater(top_speed, BALL_SPEED_START),
        "the ball never went faster than a serve, so rallies do not build",
        format!(
            "top speed {top_speed:.2} units/s against a serve of {BALL_SPEED_START:.1}; the \
             gain on a paddle touch is what makes a long point tense"
        ),
    );
    checks.require(
        greater(1.0, worst_overrun),
        "the ball ran a long way past the goal line before the point was noticed",
        format!(
            "{worst_overrun:.3} units past +/-{GOAL_LINE:.1}; one tick of travel is \
             {:.3}, so anything much larger means the point is being scored late",
            BALL_SPEED_MAX * dt
        ),
    );

    // --- what was drawn ---------------------------------------------------
    checks.require(
        frames == ticks as usize,
        "one frame per tick was expected",
        format!("{frames} frames for {ticks} ticks"),
    );
    checks.require(
        off_screen.is_none(),
        "something was drawn outside what the camera shows",
        format!(
            "the first was on tick {:?} at {:?}, against a camera showing {view:?}; text \
             centred by TextStyle::width_of is the usual culprit",
            off_screen.map(|(tick, _)| tick),
            off_screen.map(|(_, bounds)| bounds),
        ),
    );

    // Both paddles, by their *bounds* rather than by something being there: a
    // paddle drawn half out of position still covers its own centre, and a
    // "paddle-sized quad covers this point" check passes for it.
    for side in [Side::Left, Side::Right] {
        let at = paddles[side.index()];
        let found = last.covering(at).into_iter().any(|quad| {
            let bounds = quad.bounds();
            near(bounds.size().x, PADDLE_SIZE.x)
                && near(bounds.size().y, PADDLE_SIZE.y)
                && near(bounds.center().x, at.x)
                && near(bounds.center().y, at.y)
        });
        checks.require(
            found,
            "no paddle-shaped quad was drawn where a paddle is",
            format!(
                "{}'s paddle is at ({:.2}, {:.2}) and is {:.2}x{:.2}; what covers that point \
                 is {}",
                side.name(),
                at.x,
                at.y,
                PADDLE_SIZE.x,
                PADDLE_SIZE.y,
                sizes_covering(&last, at),
            ),
        );
    }

    // The ball is a circle, so "a quad the size of the thing" is the wrong
    // question: sixteen wedges share the centre and nothing ball-sized is
    // drawn anywhere. The union of the wedges covering the centre is 2r square.
    let disc = disc_span(&last, ball, BALL_RADIUS).map_or(Vec2::ZERO, |rect| rect.size());
    checks.require(
        near(disc.x, BALL_RADIUS * 2.0) && near(disc.y, BALL_RADIUS * 2.0),
        "no ball-sized disc is drawn where the ball is",
        format!(
            "the wedges covering ({:.2}, {:.2}) span {:.3}x{:.3}; a radius of {BALL_RADIUS:.2} \
             is {:.2} square. everything covering it: {}",
            ball.x,
            ball.y,
            disc.x,
            disc.y,
            BALL_RADIUS * 2.0,
            sizes_covering(&last, ball),
        ),
    );

    // The text, by band rather than by count alone: on screen is not in the
    // right place, and every one of these numbers is a layout constant the
    // game states once.
    let font_quads = glyphs(&last, font);
    // Characters, not bytes: `ctx.text` submits one quad per character, and a
    // stray multi-byte glyph would make `len()` and the count disagree — which
    // is exactly the case the printable-ASCII check further down exists for, so
    // the two must not contradict each other about how many quads to expect.
    let want_score =
        shown.left.to_string().chars().count() + shown.right.to_string().chars().count();
    let in_score = glyphs_in_band(&font_quads, SCORE_TOP, SCORE_SIZE);
    let in_hint = glyphs_in_band(&font_quads, HINT_TOP, HINT_SIZE);
    checks.require(
        in_score == want_score && in_hint == HINT.chars().count(),
        "the score or the hint is not in the band the layout puts it in",
        format!(
            "{in_score} glyphs in the score band at y {SCORE_TOP:.2}..{:.2} (want {want_score} \
             for \"{}-{}\") and {in_hint} in the hint band at y {HINT_TOP:.2}..{:.2} (want {}); \
             {} glyphs were drawn in all",
            SCORE_TOP + SCORE_SIZE,
            shown.left,
            shown.right,
            HINT_TOP + HINT_SIZE,
            HINT.chars().count(),
            font_quads.len(),
        ),
    );
    checks.require(
        in_score + in_hint == font_quads.len(),
        "text was drawn somewhere the layout does not put any",
        format!(
            "{} glyphs in all, {in_score} in the score band and {in_hint} in the hint band; \
             the difference is text nothing accounts for",
            font_quads.len()
        ),
    );
    // And the same layout stated as a *requirement* rather than as the constant
    // that produced it. The band check above reads SCORE_TOP, so it moves when
    // SCORE_TOP moves: a score dropped into the middle of the court passes it
    // happily, which is what a deliberately broken build proved. What a
    // scoreboard actually has to be is up out of the play and one number either
    // side of the centre line, evenly set — and none of that mentions
    // SCORE_TOP. The glyphs are picked out by being much taller than the hint's.
    let score_marks: Vec<Rect> = font_quads
        .iter()
        .map(|quad| quad.bounds())
        .filter(|bounds| greater(bounds.size().y, HINT_SIZE * 2.0))
        .collect();
    let top_third = -COURT_HALF_HEIGHT / 3.0;
    let lowest = score_marks
        .iter()
        .map(|bounds| bounds.max.y)
        .fold(f32::NEG_INFINITY, f32::max);
    checks.require(
        score_marks.len() == want_score && greater(top_third, lowest),
        "the score does not read as a scoreboard",
        format!(
            "{} big glyphs (want {want_score}), reaching down to y {lowest:.2}; a score belongs \
             in the top third of a court that runs to y {top_third:.2}, not in the middle of \
             the play",
            score_marks.len(),
        ),
    );
    let left_gap = score_marks
        .iter()
        .filter(|bounds| greater(0.0, bounds.max.x))
        .map(|bounds| -bounds.max.x)
        .fold(f32::INFINITY, f32::min);
    let right_gap = score_marks
        .iter()
        .filter(|bounds| greater(bounds.min.x, 0.0))
        .map(|bounds| bounds.min.x)
        .fold(f32::INFINITY, f32::min);
    checks.require(
        left_gap.is_finite() && right_gap.is_finite() && within(left_gap, right_gap, 0.01),
        "the two scores do not sit evenly either side of the centre line",
        format!(
            "the nearer edges are {left_gap:.3} left of the middle and {right_gap:.3} right of \
             it; an infinity means that side drew no score at all"
        ),
    );

    // The hint belongs outside the court, under the bottom wall, where a ball
    // can never be. "Inside the camera" would pass for a hint drawn across the
    // middle of the play.
    let hint_intrudes = font_quads
        .iter()
        .filter(|quad| greater(quad.bounds().min.y, HINT_TOP - 0.01))
        .any(|quad| greater(COURT_HALF_HEIGHT, quad.bounds().min.y));
    checks.require(
        !hint_intrudes,
        "the hint line is drawn inside the court, where the ball plays",
        format!(
            "its band starts at y {HINT_TOP:.2} and the bottom wall is at \
             {COURT_HALF_HEIGHT:.1}"
        ),
    );

    // --- the bands, where the sort disagrees with the submission order ---
    //
    // The court is submitted *after* the play, so a dash coming back before
    // the ball in the draw order is something only FIELD sorting under PLAY can
    // produce. Where a game's submission order already agrees with its layers,
    // no assertion over drawn quads can see a band at all.
    let quads = last.quads();
    let dash_at = quads.iter().position(|quad| quad.tint == crate::NET);
    let ball_at = quads.iter().position(|quad| quad.tint == crate::BALL_COLOR);
    checks.require(
        dash_at.is_some() && ball_at.is_some(),
        "the centre line or the ball drew nothing in the last frame",
        format!("as indices into the draw order: dash {dash_at:?}, ball {ball_at:?}"),
    );
    if let (Some(dash), Some(ball_index)) = (dash_at, ball_at) {
        checks.require(
            dash < ball_index,
            "the centre line is drawn over the ball instead of behind it",
            format!(
                "the dash is at index {dash} in the draw order and the ball at {ball_index}; \
                 the game submits the court *after* the play, so only FIELD sorting under PLAY \
                 can put it first"
            ),
        );
    }

    // --- the frames a played match never produces -------------------------
    //
    // Both band boundaries, staged so the two bands actually cover the same
    // point — `covering(p)[0]` is what a player looking at `p` sees, and it can
    // only answer when there is something to be in front of.
    //
    // With the match put back into a rally first: the run ends on a winner, and
    // a banner glyph sits over the middle of the court. Staging a frame means
    // staging *all* of it, not only the part being asked about.
    sim.world_mut().insert_resource(Scoreboard {
        left: 1,
        right: 2,
        stage: Stage::Rally,
    });
    let on_a_dash = Vec2::new(0.0, dash_y(crate::dash_count() / 2));
    place_ball(&mut sim, on_a_dash);
    let staged = recorder.draw(&mut sim);
    let front = staged.covering(on_a_dash).into_iter().next();
    checks.require(
        front.is_some_and(|quad| quad.tint == crate::BALL_COLOR),
        "the ball is drawn behind the centre line",
        format!(
            "parked on the dash at ({:.2}, {:.2}), the front-most quad is tinted {:?} rather \
             than the ball's {:?}",
            on_a_dash.x,
            on_a_dash.y,
            front.map(|quad| quad.tint),
            crate::BALL_COLOR,
        ),
    );
    let under_score = Vec2::new(
        -crate::SCORE_INSET - SCORE_SIZE * 7.0 / 9.0 * 0.5,
        SCORE_TOP + SCORE_SIZE * 0.5,
    );
    place_ball(&mut sim, under_score);
    let staged = recorder.draw(&mut sim);
    let front = staged.covering(under_score).into_iter().next();
    checks.require(
        front.is_some_and(|quad| quad.texture == font),
        "the ball is drawn over the score instead of under it",
        format!(
            "parked in the middle of the left digit at ({:.2}, {:.2}), the front-most quad is \
             {:?}; the score is on the UI band and the ball on PLAY",
            under_score.x,
            under_score.y,
            front.map(|quad| (quad.texture, quad.tint)),
        ),
    );

    // Both banners, including the one a run that wins never draws — which is
    // the longest string in the game and therefore the one most likely to run
    // off the edge.
    place_ball(&mut sim, Vec2::ZERO);
    for (name, staged_board) in [
        (
            "the winning banner",
            Scoreboard {
                left: WINNING_SCORE,
                right: 2,
                stage: Stage::Over { winner: Side::Left },
            },
        ),
        (
            "the losing banner",
            Scoreboard {
                left: 3,
                right: WINNING_SCORE,
                stage: Stage::Over {
                    winner: Side::Right,
                },
            },
        ),
    ] {
        sim.world_mut().insert_resource(staged_board);
        let frame = recorder.draw(&mut sim);
        let strays: Vec<Rect> = frame
            .quads()
            .iter()
            .map(|quad| quad.bounds())
            .filter(|bounds| !view.contains_rect(*bounds))
            .collect();
        checks.require(
            strays.is_empty(),
            "a banner the played match never draws runs off the screen",
            format!(
                "{name}: {} of {} quads fall outside {view:?}, the first at {:?}",
                strays.len(),
                frame.quad_count(),
                strays.first(),
            ),
        );
        let banner_glyphs = glyphs(&frame, font).len();
        let want = crate::BANNER_WON
            .chars()
            .count()
            .max(crate::BANNER_LOST.chars().count())
            + crate::BANNER_SUB.chars().count();
        checks.require(
            banner_glyphs > font_quads.len(),
            "a banner was staged and nothing extra was drawn",
            format!(
                "{name} produced {banner_glyphs} glyphs against {} in an ordinary frame; the \
                 banner and its subtitle are up to {want} more",
                font_quads.len()
            ),
        );
    }

    // --- the background, which leaves no quad behind ----------------------
    let cleared = last.plan.clear_color;
    checks.require(
        cleared == crate::COURT,
        "the court was cleared to a colour the game does not name",
        format!(
            "the frame cleared to {cleared:?}; the game's constant is {:?}",
            crate::COURT
        ),
    );
    // And a second check the constant cannot move with: the requirement, rather
    // than the number that was written to meet it.
    let brightness = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        greater(0.25, brightness) && greater(cleared.a, 0.99),
        "the court is not dark enough to see a near-white ball on",
        format!(
            "its brightest channel is {brightness:.3} at alpha {:.2}",
            cleared.a
        ),
    );

    // --- the strings themselves -------------------------------------------
    //
    // No assertion over drawn quads can see a wrong character: the font draws
    // an unknown one as a box at exactly a letter's advance, so a stray em dash
    // passes the glyph count, the band check and the bounds check alike.
    for (name, text) in [
        ("the hint", HINT),
        ("the winning banner", crate::BANNER_WON),
        ("the losing banner", crate::BANNER_LOST),
        ("the banner's subtitle", crate::BANNER_SUB),
    ] {
        let stray = text
            .chars()
            .find(|glyph| *glyph != '\n' && !(' '..='~').contains(glyph));
        checks.require(
            stray.is_none(),
            "a string the game draws has a character the font cannot draw",
            format!(
                "{name} contains {stray:?}, which draws as a box at exactly a letter's width — \
                 no assertion over what was drawn can tell the difference"
            ),
        );
    }

    // --- the layout constants, against the court they are supposed to fit --
    checks.require(
        within(view.size().y, VIEW_HEIGHT, 0.001)
            && greater(view.size().x, crate::COURT_HALF_WIDTH * 2.0),
        "the court does not fit the camera it is drawn in",
        format!(
            "the camera shows {:.2}x{:.2} and the court is {:.1} wide by {:.1} tall",
            view.size().x,
            view.size().y,
            crate::COURT_HALF_WIDTH * 2.0,
            COURT_HALF_HEIGHT * 2.0,
        ),
    );
    checks.require(
        greater(PADDLE_LIMIT, 0.0) && greater(COURT_HALF_HEIGHT, PADDLE_LIMIT),
        "a paddle cannot stay inside the court",
        format!(
            "its centre is clamped to +/-{PADDLE_LIMIT:.2} on a court reaching \
             +/-{COURT_HALF_HEIGHT:.1}, and it is {:.1} tall",
            PADDLE_SIZE.y
        ),
    );

    let captured = crate::capture::capture_a_frame(&mut checks, &last, font);
    let failures = checks.failures();
    let verdict = checks.verdict();

    println!(
        "verified pong: {} won {}-{} in {ticks} ticks ({:.1}s), {failures} checks failed",
        winner.map_or("nobody", Side::name),
        board.left,
        board.right,
        ticks as f32 * dt,
    );
    println!(
        "  match: {} points, longest rally {longest_rally} touches, top ball speed \
         {top_speed:.1} units/s",
        board.left + board.right
    );
    println!(
        "  gradient: the chasing player finished {}-{} in {naive_ticks} ticks, the idle one \
         {}-{}",
        naive.left, naive.right, lost.left, lost.right,
    );
    println!(
        "  opponent: returned {} of {} balls that reached it ({:.0}%)",
        returned[Side::Right.index()],
        reached[Side::Right.index()],
        opponent_share * 100.0,
    );
    println!("  controller: met {met} of {} approaches", approaches.len());
    println!("  controller: planned returns aimed to land {planned_gap:.2} from the opponent");
    println!("  controller: shots landed {aim_error:.2} from where they were planned to");
    println!(
        "  frames: {frames}, {} quads in the last one, {} of them glyphs",
        last.quad_count(),
        font_quads.len(),
    );
    println!("  capture: {captured}");
    print!("{}", last.transcript());
    verdict
}
