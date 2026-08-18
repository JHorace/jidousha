//! The player: a controller that decides as it goes, and plays to win.
//!
//! `InputScript` is the session written down in advance, and it is the wrong
//! tool here — a blind script never returns a ball, so it can prove W and S
//! move a paddle and still say nothing about whether the game is playable.
//! `SnapshotBuilder` is the driver's own accumulator, so a controller built on
//! it goes through the same edge rules a real keyboard does. It sends *events*
//! and therefore has to remember what it is already holding, which is what
//! makes a key held for four hundred ticks press exactly once.
//!
//! And it plays to **win** rather than to survive. A controller that merely
//! tracks the ball returns it dead flat down the middle, the machine tracks it
//! straight back, and the run reports an unplayable 0-0 with a two-hundred-touch
//! rally — a verdict about the controller wearing the costume of a verdict about
//! the game. So every return this paddle can physically produce is simulated
//! through the game's own `contact`, and the one landing furthest from anywhere
//! the machine can reach in time is the one it takes.

use std::cmp::Ordering;

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder};

use crate::checks::{Checks, check_nothing_off_screen};
use crate::{
    BALL_RADIUS, MACHINE_REACTION, MACHINE_SPEED, PADDLE_SIZE, PADDLE_TRAVEL, PLAYER_SPEED, WALL_Y,
};
use crate::{Contact, Control, Paddle, Play, Scoreboard, Side, ball_state, config, register};

/// How long the checked session runs, in ticks.
///
/// A minute at the default timestep. A match the controller wins takes well
/// under that; the surplus is what makes "the match never finished" a real
/// failure rather than a run that was simply cut short.
pub(crate) const TICKS: u64 = 3600;

/// How often a frame is drawn and kept.
///
/// The recorder keeps every frame it is given and has no way to forget one, so
/// a tick-for-tick recording of a minute-long match is tens of megabytes of
/// vertices. Every second tick is still hundreds of frames of bounds checking,
/// and the frames that matter — the last one, and the screens built by hand
/// afterwards — are drawn deliberately rather than sampled.
const DRAW_EVERY: u64 = 2;

/// How far off its aim the controller will sit rather than chase.
///
/// Well under one tick of travel at the player's speed, so the paddle settles
/// on its target instead of stopping half a tick short of it — which, with the
/// margin below, is the difference between a return and a miss.
const AIM_DEAD_BAND: f32 = 0.12;

/// How much of the paddle the planner will aim with, as a fraction of the half
/// that reaches either side of its centre.
///
/// Not all of it: a contact at the very tip is the sharpest return available
/// and also the one that a fraction of a unit of error turns into a whole miss.
const SAFE_CONTACT: f32 = 0.78;

/// How many contact points along the paddle the shot planner tries.
const SHOT_SAMPLES: i32 = 13;

/// What one session did, in the numbers a failing check needs to quote.
pub(crate) struct Session {
    pub(crate) board: Scoreboard,
    /// The tick the match was decided on, if it was.
    pub(crate) decided_at: Option<u64>,
    /// The furthest the ball ever got from the middle, per axis.
    pub(crate) ball_extent: Vec2,
    /// The furthest a paddle centre ever got from the middle.
    pub(crate) paddle_extent: f32,
    /// Ticks with the ball live.
    pub(crate) live_ticks: u64,
    /// How many times the player's paddle sent the ball back.
    pub(crate) returns: u32,
    /// How many times the machine's did.
    pub(crate) machine_returns: u32,
}

/// The last frame drawn, and where the world had everything when it was.
pub(crate) struct Snapshot {
    pub(crate) frame: FrameRecord,
    pub(crate) paddles: Vec<(Side, Vec2)>,
    pub(crate) ball: Vec2,
}

/// The face a ball meets when it reaches this side's paddle: the paddle's
/// inner edge, pushed out by the ball's radius, so contact is a point against
/// a plane.
fn face_of(side: Side) -> f32 {
    side.paddle_x() + side.outward() * (PADDLE_SIZE.x * 0.5 + BALL_RADIUS)
}

/// Where the ball crosses `face_x`, folded back inside the walls; `None` when
/// it is not going that way at all.
fn crossing(pos: Vec2, velocity: Vec2, face_x: f32) -> Option<f32> {
    if velocity.x == 0.0 {
        return None;
    }
    let flight = (face_x - pos.x) / velocity.x;
    if flight < 0.0 {
        return None;
    }
    Some(fold(pos.y + velocity.y * flight))
}

/// Reflect a y back inside the walls as often as it takes — the closed form of
/// "bounce off the top, then the bottom, then the top again".
fn fold(y: f32) -> f32 {
    let limit = WALL_Y - BALL_RADIUS;
    let span = 2.0 * limit;
    let mut folded = (y + limit).rem_euclid(2.0 * span);
    if folded > span {
        folded = 2.0 * span - folded;
    }
    folded - limit
}

/// One return, computed with the game's *own* contact rule.
///
/// The two points bracket the paddle's face exactly, half a tick either side,
/// so `contact` sees a crossing at `arrival` carrying the ball's real speed.
/// Calling the game's function rather than copying its arithmetic is the point:
/// a planner with its own idea of how a paddle bounces is a planner that will
/// quietly stop predicting this game.
fn simulate_return(arrival: f32, velocity: Vec2, paddle_y: f32, step: f32) -> Option<Contact> {
    let at = Vec2::new(face_of(Side::Left), arrival);
    let half = velocity * step * 0.5;
    crate::contact(at - half, at + half, Side::Left, paddle_y, step)
}

/// Where the player's paddle should be to take the best shot available.
///
/// Two constraints first, then the choice. A paddle position is worth
/// considering only if it (a) actually touches the ball, with margin, and (b)
/// can be got to before the ball arrives. Within what survives both, every
/// contact point is tried and the return landing furthest from anywhere the
/// machine can reach in time wins.
///
/// The first constraint is the one that cost this file a whole cycle. Without
/// it the planner happily picked a contact at the very tip of the paddle,
/// because a tip contact is the sharpest angle and therefore the best shot on
/// paper — and then any error at all, including the half-tick of overshoot the
/// dead band leaves, turned it into a clean miss. It lost 0-5 and reported that
/// the machine was too fast. The machine was not too fast.
fn best_aim(here: f32, ball: Vec2, velocity: Vec2, machine_y: f32, step: f32) -> Option<f32> {
    let face = face_of(Side::Left);
    let arrival = crossing(ball, velocity, face)?;
    let flight = (face - ball.x) / velocity.x;
    let reach = (PADDLE_SIZE.y * 0.5 + BALL_RADIUS) * SAFE_CONTACT;
    let swing = PLAYER_SPEED * flight;

    // Positions that touch the ball, that the paddle can reach in time, and
    // that are inside its own clamp.
    let low = (arrival - reach).max(here - swing).max(-PADDLE_TRAVEL);
    let high = (arrival + reach).min(here + swing).min(PADDLE_TRAVEL);
    if low > high {
        // Nowhere satisfies all three: run at the ball and hope. Returning the
        // arrival itself is the closest a paddle can be to touching it.
        return Some(arrival.clamp(-PADDLE_TRAVEL, PADDLE_TRAVEL));
    }

    let far = face_of(Side::Right);
    let full_reach = PADDLE_SIZE.y * 0.5 + BALL_RADIUS;
    let mut best: Option<(f32, f32)> = None;
    for sample in 0..SHOT_SAMPLES {
        let across = f32::from(sample as i16) / f32::from((SHOT_SAMPLES - 1) as i16);
        let paddle_y = low + (high - low) * across;
        let Some(shot) = simulate_return(arrival, velocity, paddle_y, step) else {
            continue;
        };
        let Some(lands) = crossing(shot.position, shot.velocity, far) else {
            continue;
        };
        let return_flight = (far - shot.position.x) / shot.velocity.x;
        // How far past the end of its paddle the machine would be, having moved
        // as fast as it can for the whole flight.
        // The machine reads the ball every `MACHINE_REACTION` ticks and drives
        // at what it last saw, so it spends about that long heading the wrong
        // way after every change of direction. Docking the flight by one
        // reaction is a coarse model of an opponent this planner cannot
        // simulate exactly, and it is deliberately *optimistic* about the
        // machine: a planner that under-rated its opponent would pick shots
        // that only work against the opponent it imagined.
        let machine_swing =
            MACHINE_SPEED * (return_flight - MACHINE_REACTION as f32 * step).max(0.0);
        let closest = lands.clamp(machine_y - machine_swing, machine_y + machine_swing);
        let score = (lands - closest).abs() - full_reach;
        let better = match best {
            None => true,
            Some((so_far, _)) => matches!(score.partial_cmp(&so_far), Some(Ordering::Greater)),
        };
        if better {
            best = Some((score, paddle_y));
        }
    }
    best.map(|(_, paddle_y)| paddle_y)
}

/// Play one whole session, checking every frame it draws along the way.
pub(crate) fn play(
    checks: &mut Checks,
    recorder: &mut FrameRecorder,
) -> (Session, Option<Snapshot>) {
    let mut sim = headless(config(), register);
    // A controller sends *events*, so it has to remember what it is already
    // holding. That is what a keyboard is, and it is what makes a key held for
    // four hundred ticks press exactly once.
    let mut keyboard = SnapshotBuilder::new();
    let mut holding: Option<Key> = None;

    let mut session = Session {
        board: Scoreboard::new(),
        decided_at: None,
        ball_extent: Vec2::ZERO,
        paddle_extent: 0.0,
        live_ticks: 0,
        returns: 0,
        machine_returns: 0,
    };
    let mut last = None;
    let mut rally_so_far = 0;

    for tick in 1..=TICKS {
        // On the way into tick 1 there is nothing to look at: `Startup` runs
        // *inside* that first `tick()`. Hence `find_resource` throughout, and a
        // query that is allowed to yield nothing.
        let step = sim.world().resource::<Time>().fixed_dt.as_f32();
        let live = sim
            .world()
            .find_resource::<Scoreboard>()
            .is_some_and(|board| matches!(board.play, Play::Rally));
        let ball = ball_state(sim.world());
        let (mut me, mut machine) = (None, None);
        for (_, transform, paddle) in sim.world().query::<(&Transform, &Paddle)>() {
            match paddle.control {
                Control::Keys => me = Some(transform.pos.y),
                Control::Machine => machine = Some(transform.pos.y),
            }
        }

        // Decide where the paddle wants to be, then which key gets it there.
        let goal = match (live, ball, machine) {
            (true, Some((pos, velocity)), Some(machine_y)) if velocity.x < 0.0 => {
                best_aim(me.unwrap_or(0.0), pos, velocity, machine_y, step).unwrap_or(0.0)
            }
            // Not coming this way, or not in play: wait in the middle, which is
            // where the next serve comes from.
            _ => 0.0,
        };
        let want = me.and_then(|here| {
            let gap = goal - here;
            if gap.abs() < AIM_DEAD_BAND {
                None
            } else if gap > 0.0 {
                Some(Key::S) // Y is down, so S is towards the bottom.
            } else {
                Some(Key::W)
            }
        });
        if want != holding {
            if let Some(key) = holding {
                keyboard.record(InputEvent::KeyReleased(key));
            }
            if let Some(key) = want {
                keyboard.record(InputEvent::KeyPressed(key));
            }
            holding = want;
        }

        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();

        // Observe.
        let board = *sim.world().resource::<Scoreboard>();
        if board.rally > rally_so_far {
            // Whichever paddle just hit it, the ball is now heading away from
            // that paddle — so its direction says who returned it.
            match ball_state(sim.world()) {
                Some((_, velocity)) if velocity.x > 0.0 => session.returns += 1,
                Some(_) => session.machine_returns += 1,
                None => {}
            }
        }
        rally_so_far = board.rally;
        if matches!(board.play, Play::Rally) {
            session.live_ticks += 1;
        }
        if session.decided_at.is_none() && matches!(board.play, Play::Over { .. }) {
            session.decided_at = Some(tick);
        }
        session.board = board;
        if let Some((pos, _)) = ball_state(sim.world()) {
            session.ball_extent = session.ball_extent.max(pos.abs());
        }
        for (_, transform, _) in sim.world().query::<(&Transform, &Paddle)>() {
            session.paddle_extent = session.paddle_extent.max(transform.pos.y.abs());
        }

        // Draw, and keep the world's own account of where things were, so the
        // "is it drawn where the world says" checks compare the frame against
        // the simulation rather than against a number written down here.
        let decided_now = session.decided_at == Some(tick);
        if tick.is_multiple_of(DRAW_EVERY) || decided_now {
            let paddles: Vec<(Side, Vec2)> = sim
                .world()
                .query::<(&Transform, &Paddle)>()
                .map(|(_, transform, paddle)| (paddle.side, transform.pos))
                .collect();
            let ball = ball_state(sim.world())
                .map(|(pos, _)| pos)
                .unwrap_or_default();
            let frame = recorder.draw(&mut sim);
            check_nothing_off_screen(checks, &sim, &frame, tick);
            last = Some(Snapshot {
                frame,
                paddles,
                ball,
            });
        }
        // A few ticks past the decision, so the banner is in the frame that is
        // kept. Then stop: everything after this is the restart check.
        if session
            .decided_at
            .is_some_and(|at| tick >= at + DRAW_EVERY * 2)
        {
            break;
        }
    }

    check_the_restart(checks, &mut sim);
    (session, last)
}

/// Space after a finished match puts the score back to nil-nil.
///
/// A game cannot close itself in v1 — there is no `App::quit` and nothing on
/// `World` or `Commands` — so the end of a match has to be a state a player can
/// leave rather than a program that exits.
fn check_the_restart(checks: &mut Checks, sim: &mut HeadlessSim) {
    let before = *sim.world().resource::<Scoreboard>();
    if !matches!(before.play, Play::Over { .. }) {
        // Nothing to report here: a session that ran out of ticks is already
        // the subject of "no one won the match", which prints the score, the
        // longest rally and the top ball speed. A second, vaguer complaint
        // about the same fact would be the first thing printed and the least
        // useful thing to read.
        return;
    }
    let mut keyboard = SnapshotBuilder::new();
    keyboard.record(InputEvent::KeyPressed(Key::Space));
    sim.world_mut()
        .insert_resource(Input::new(keyboard.first_tick_snapshot()));
    sim.tick();
    let after = *sim.world().resource::<Scoreboard>();
    checks.require(
        after.left == 0 && after.right == 0 && !matches!(after.play, Play::Over { .. }),
        "SPACE did not start a fresh match",
        format!(
            "the board read {}-{} before the press and {}-{} ({:?}) after it",
            before.left, before.right, after.left, after.right, after.play
        ),
    );
}
