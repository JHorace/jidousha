//! The check: play a whole match headless, assert on what the world did and on
//! what was drawn, then leave a picture behind.
//!
//! `cargo run -p jidousha --example pong -- --verify` runs this. It registers
//! the *same* systems and the same config the window does, so what is verified
//! is what a person plays. What differs is only what a person would otherwise
//! supply: the left paddle is moved by the controller below.
//!
//! # The controller is the instrument, so it checks itself
//!
//! A blind `InputScript` never returns a ball, so it can prove the controls
//! work and say nothing about whether the game is playable. This one looks at
//! the world every tick and plays to win — and, because a mediocre controller
//! does not report "unplayable" but a plausible wrong number that sends you off
//! to tune the game, it reports its own hit rate as a check of its own. `met
//! N of N approaches` is what says which half of the program to open when the
//! score comes out wrong.
//!
//! It aims by **constrain, then optimise**: only contact points that really
//! land on the paddle with margin, and that this paddle can reach before the
//! ball arrives, are scored at all. The optimum otherwise sits on the boundary
//! of what the paddle can do — the sharpest return is always the one struck at
//! the very tip — where half a tick of overshoot is a clean miss.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, InputEvent, InputScript, SnapshotBuilder};

use crate::checks::{Checks, disc_union, fail, greater, near, sizes_covering, within};
use crate::{
    BALL_RADIUS, Ball, COURT, Control, HINT, HINT_OVER, MAX_BOUNCE, OPPONENT_SPEED, PADDLE,
    PADDLE_LIMIT, PADDLE_X, PLAYER_SPEED, Paddle, Phase, SCORE_SIZE, SCORE_X, SERVE_SPEED,
    SPEED_PER_TOUCH, Scoreboard, Side, TOP_SPEED, WIN_SCORE, WINDOW, banner_text, bounce_off,
    config, face_of, fold_into_court, machine_push, palette, register, step_paddle,
    travel_one_tick,
};

/// The longest match the check will sit through, in ticks.
///
/// A minute of game at the default timestep. The run asserts the match ended
/// well inside this and prints the tick it ended on, so a game that got slower
/// shows up as a number rather than as a timeout.
const TICKS: u64 = 3600;

/// How far ahead the controller replays the ball, in ticks.
///
/// The longest crossing the game can produce is the court's width at serve
/// speed, about 105 ticks; this is comfortably past it and bounds the search
/// so a ball that somehow stopped moving cannot hang the run.
const LOOKAHEAD: u32 = 400;

/// How much of a paddle the controller will aim with, as a fraction of the
/// distance from its centre to where a ball stops touching it.
///
/// Not 1.0, and that is the whole point: the best shot available is always the
/// one struck at the very tip, because that is where the bounce is widest — so
/// an unconstrained search stands the paddle on its last millimetre every time
/// and any error at all becomes a clean miss instead of a worse return.
const AIM_MARGIN: f32 = 0.72;

/// How many contact points across the paddle the controller tries.
const AIM_POINTS: usize = 9;

/// How much of the travel the controller believes it has, when asking whether
/// it can reach a contact point in time.
const REACH_SAFETY: f32 = 0.85;

/// How far off its chosen line the controller will sit rather than correct, so
/// it does not chatter a key on and off around the target.
///
/// This is also, unavoidably, how wrong its aim is: a keyboard paddle moves in
/// steps of `PLAYER_SPEED * fixed_dt` and cannot stand anywhere in between, so
/// it arrives somewhere within this of where it meant to be.
const CONTROL_DEADZONE: f32 = 0.2;

/// How wrong the controller assumes its own contact point will be, as a
/// fraction of the paddle's reach.
///
/// `CONTROL_DEADZONE` expressed in the units a bounce is computed in. It is
/// about five degrees of bounce angle, which over a court this wide is seven
/// units of landing — measured, not guessed: the first controller to plan an
/// exact landing produced shots that arrived 7.43 units from where it planned,
/// on a court 17.1 units tall. The flight is chaotic in the aim angle, so no
/// amount of care in the prediction fixes that; what fixes it is scoring a shot
/// by its *worst* outcome across the error the controller knows it has.
const AIM_UNCERTAINTY: f32 = CONTROL_DEADZONE / (PADDLE.y + BALL_RADIUS);

/// What one played match came out as.
struct Session {
    /// The tick the match was decided on, if it was.
    ended_at: Option<u64>,
    /// The scoreboard at the end.
    board: Scoreboard,
    /// How many times the ball came at the controller's paddle.
    approaches: u32,
    /// How many of those it met.
    met: u32,
    /// How far from where the opponent will be standing the controller's chosen
    /// return was predicted to land, worst and best over the match.
    threat: (f32, f32),
    /// How many returns it planned at all.
    planned: u32,
    /// The furthest the ball ever was from the opponent's centre at the moment
    /// the opponent touched it: how stretched it ever actually got.
    their_stretch: f32,
    /// How many balls the opponent touched.
    their_touches: u32,
    /// How far the ball actually landed from where the plan said it would:
    /// worst, and the running total.
    aim_error: (f32, f32),
    /// How many shots that was measured over.
    shots: u32,
    /// The furthest the ball ever got from the centre line, in each axis.
    ball_extent: Vec2,
    /// The lowest and highest the controller's paddle ever sat.
    player_span: (f32, f32),
    /// The lowest and highest the opponent's paddle ever sat.
    opponent_span: (f32, f32),
    /// The last frame drawn while the ball was live, and what the world held
    /// when it was drawn.
    rally: Option<Snapshot>,
    /// The very last frame drawn.
    last_frame: Option<FrameRecord>,
    /// Enough of the final world to compare two runs bit for bit.
    fingerprint: Vec<u32>,
    /// The average of those predicted landings.
    mean_threat: f32,
}

/// A frame, and the world positions it should be a picture of.
struct Snapshot {
    /// The frame.
    frame: FrameRecord,
    /// Where the ball was.
    ball: Vec2,
    /// Where each paddle was.
    paddles: Vec<(Side, Vec2)>,
}

/// Run the ball and the opponent forward together until the ball reaches
/// `side`'s paddle face.
///
/// Returns where the ball's centre is when it gets there, how many ticks that
/// took, and where the opponent is standing by then. Every step goes through
/// the game's own `travel_one_tick`, `machine_push` and `step_paddle`, in the
/// order the game's systems run them, so this is a prediction *by simulation*
/// rather than a closed form to keep in step by hand.
///
/// `None` if the ball is not going that way, or does not get there inside
/// `LOOKAHEAD`.
fn run_to_face(
    mut ball: Vec2,
    mut velocity: Vec2,
    mut opponent_y: f32,
    side: Side,
    dt: f32,
) -> Option<(f32, u32, f32)> {
    let sign = side.sign();
    let face = face_of(side);
    if velocity.x * sign <= 0.0 {
        return None;
    }
    for tick in 0..LOOKAHEAD {
        let lead = ball.x + sign * BALL_RADIUS;
        if (lead - face) * sign >= 0.0 {
            return Some((ball.y, tick, opponent_y));
        }
        // The game's order: steer, move the paddles, then move the ball.
        let push = machine_push(opponent_y, ball, velocity, Side::Right);
        opponent_y = step_paddle(opponent_y, push, OPPONENT_SPEED, dt);
        let raw = ball + velocity * dt;
        if (raw.x + sign * BALL_RADIUS - face) * sign >= 0.0 {
            let fraction = ((face - lead) / (raw.x - ball.x)).clamp(0.0, 1.0);
            // Folded rather than taken raw: a tick that crosses the face may
            // also cross a wall, and the game folds before it does anything
            // else.
            let (y, _) = fold_into_court(ball.y + velocity.y * dt * fraction);
            return Some((y, tick, opponent_y));
        }
        let (next, going) = travel_one_tick(ball, velocity, dt);
        ball = next;
        velocity = going;
    }
    None
}

/// One planned return: where to stand, and what that shot is expected to do.
#[derive(Clone, Copy, Debug)]
struct Plan {
    /// Where the paddle should be when the ball arrives.
    target: f32,
    /// Where the return is predicted to reach the opponent's face.
    landing: f32,
    /// Where the opponent is predicted to be standing when it does.
    opponent_at_landing: f32,
}

impl Plan {
    /// How far past the opponent this shot is expected to land.
    fn threat(&self) -> f32 {
        (self.landing - self.opponent_at_landing).abs()
    }
}

/// Where the controller's paddle should be sitting this tick.
///
/// Constrain, then optimise. Every contact point that survives both constraints
/// is scored by how far from the opponent its return lands — with "the
/// opponent" meaning where that paddle will have walked to by the time the ball
/// gets there, which is a thing this can only know by running the opponent's
/// own rule forward beside the ball's.
///
/// Two cheaper objectives were tried first and both are wrong here. "Away from
/// where it is standing now" is worth nothing against a paddle that walks back
/// to the middle between shots. "Furthest from the middle" is the fix for
/// *that*, and it is wrong against a paddle that chases: the shots that land
/// furthest from the middle are the steep ones, which get there by rebounding
/// off a wall straight into the path the chaser is already following. Aiming
/// that way met every ball, aimed every return to within a tenth of the wall,
/// and could not finish a match in a minute.
fn aim(
    ball: Vec2,
    velocity: Vec2,
    speed: f32,
    paddle_y: f32,
    opponent_y: f32,
    dt: f32,
) -> (f32, Option<Plan>) {
    // Forward to the moment of contact, opponent and all. Predicting the shot
    // against where the opponent is standing *now* is the bug that hid inside
    // the first two versions of this controller: the plan is made a second
    // before the ball arrives, and the opponent spends that second walking back
    // to the middle. Every candidate below is scored against where it will
    // actually be.
    let Some((arrival, ticks, opponent_then)) =
        run_to_face(ball, velocity, opponent_y, Side::Left, dt)
    else {
        // Not coming: stand in the middle, which is the best place to be for
        // whatever comes next.
        return (0.0, None);
    };
    let reach = PADDLE.y + BALL_RADIUS;
    let travel = PLAYER_SPEED * dt * ticks as f32 * REACH_SAFETY;
    let struck = (speed + SPEED_PER_TOUCH).min(TOP_SPEED);

    let mut best: Option<Plan> = None;
    for step in 0..AIM_POINTS {
        let offset = (step as f32 / (AIM_POINTS - 1) as f32 * 2.0 - 1.0) * AIM_MARGIN;
        let target = arrival - offset * reach;
        // Constraint one: the paddle is allowed to be there at all.
        if target.abs() > PADDLE_LIMIT {
            continue;
        }
        // Constraint two: it can get there before the ball does.
        if (target - paddle_y).abs() > travel {
            continue;
        }
        // Only now is it worth asking how good the shot is: how far from the
        // opponent, as the opponent will actually have moved, it lands.
        // Minimax over the error the controller knows it has. Scoring the
        // nominal shot alone picks whichever candidate happens to land in a
        // gap, which is worth nothing when the shot actually produced is five
        // degrees away from the one scored. The worst of the three is a shot
        // that works whichever of them comes out.
        let contact = Vec2::new(face_of(Side::Left) + BALL_RADIUS, arrival);
        let mut worst: Option<Plan> = None;
        for wobble in [-AIM_UNCERTAINTY, 0.0, AIM_UNCERTAINTY] {
            let returned = bounce_off(Side::Left, offset + wobble, struck);
            let Some((landing, _, waiting)) =
                run_to_face(contact, returned, opponent_then, Side::Right, dt)
            else {
                worst = None;
                break;
            };
            let plan = Plan {
                target,
                landing,
                opponent_at_landing: waiting,
            };
            if worst.is_none_or(|so_far| greater(so_far.threat(), plan.threat())) {
                worst = Some(plan);
            }
        }
        let Some(plan) = worst else { continue };
        if best.is_none_or(|so_far| greater(plan.threat(), so_far.threat())) {
            best = Some(plan);
        }
    }
    // Nothing survived both constraints: run at the ball, which is the only
    // thing left that can still touch it.
    //
    // The score comes back too. "Did it meet the ball" clears the controller as
    // a returner and says nothing about it as an attacker, and those are two
    // different contracts: a controller that returns everything dead flat down
    // the middle produces exactly the endless rally an unbeatable opponent
    // does. This is the number that tells them apart.
    match best {
        None => (arrival, None),
        Some(plan) => (plan.target, Some(plan)),
    }
}

/// Play one whole match, drawing every tick into `recorder` if there is one.
fn play(recorder: Option<&mut FrameRecorder>) -> Session {
    let mut sim = headless(config(), register);
    let mut recorder = recorder;
    let mut keyboard = SnapshotBuilder::new();
    let (mut holding_up, mut holding_down) = (false, false);

    let mut session = Session {
        ended_at: None,
        board: Scoreboard::default(),
        approaches: 0,
        met: 0,
        threat: (f32::MAX, 0.0),
        planned: 0,
        their_stretch: 0.0,
        their_touches: 0,
        aim_error: (0.0, 0.0),
        shots: 0,
        ball_extent: Vec2::ZERO,
        player_span: (0.0, 0.0),
        opponent_span: (0.0, 0.0),
        rally: None,
        last_frame: None,
        fingerprint: Vec::new(),
        mean_threat: 0.0,
    };
    let mut approach_open = false;
    let mut points_conceded = 0;
    let mut threat_total = 0.0f32;
    let mut was_going_right = false;
    let mut last_plan: Option<Plan> = None;
    let mut pending: Option<Plan> = None;

    for tick in 1..=TICKS {
        // The read at the top of the loop happens once against a world that
        // does not exist yet: `Startup` runs *inside* the first `tick()`, so
        // this is `find_resource` and a query that may yield nothing.
        let state = sim
            .world()
            .query::<(&Transform, &Ball)>()
            .map(|(_, transform, ball)| (transform.pos, ball.velocity, ball.speed))
            .next();
        let mine = sim
            .world()
            .query::<(&Transform, &Paddle)>()
            .find(|(_, _, paddle)| paddle.control == Control::Keys)
            .map(|(_, transform, _)| transform.pos.y);
        let theirs = sim
            .world()
            .query::<(&Transform, &Paddle)>()
            .find(|(_, _, paddle)| paddle.control == Control::Machine)
            .map_or(0.0, |(_, transform, _)| transform.pos.y);
        let live = sim
            .world()
            .find_resource::<Scoreboard>()
            .is_some_and(|board| matches!(board.phase, Phase::Rally));
        let dt = sim
            .world()
            .find_resource::<Time>()
            .map_or(1.0 / 60.0, |time| time.fixed_dt.as_f32());

        let push = match (state, mine) {
            (Some((ball, velocity, speed)), Some(paddle_y)) => {
                let (target, threat) = if live {
                    aim(ball, velocity, speed, paddle_y, theirs, dt)
                } else {
                    (0.0, None)
                };
                if let Some(plan) = threat {
                    last_plan = Some(plan);
                }
                let gap = target - paddle_y;
                if gap.abs() < CONTROL_DEADZONE {
                    0.0
                } else {
                    gap.signum()
                }
            }
            _ => 0.0,
        };

        // Events, not states: that is what makes a key held for a hundred ticks
        // press exactly once, and it is why the two flags are here at all.
        let (want_up, want_down) = (push < 0.0, push > 0.0);
        for (want, holding, key) in [
            (want_up, &mut holding_up, Key::W),
            (want_down, &mut holding_down, Key::S),
        ] {
            if want != *holding {
                keyboard.record(if want {
                    InputEvent::KeyPressed(key)
                } else {
                    InputEvent::KeyReleased(key)
                });
                *holding = want;
            }
        }

        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();

        // --- what the tick did -------------------------------------------
        let Some(board) = sim.world().find_resource::<Scoreboard>().copied() else {
            fail(
                "the scoreboard is gone",
                "Startup inserts exactly one and nothing removes it",
            );
        };
        session.board = board;
        let Some((ball, velocity, _)) = sim
            .world()
            .query::<(&Transform, &Ball)>()
            .map(|(_, transform, b)| (transform.pos, b.velocity, b.speed))
            .next()
        else {
            fail("the ball is gone", "Startup spawns exactly one");
        };
        let paddles: Vec<(Side, Vec2)> = sim
            .world()
            .query::<(&Transform, &Paddle)>()
            .map(|(_, transform, paddle)| (paddle.side, transform.pos))
            .collect();
        if paddles.len() != 2 {
            fail(
                "there are not two paddles",
                &format!(
                    "Startup spawns one per side; this world has {}",
                    paddles.len()
                ),
            );
        }

        // The opponent has just hit it if the ball turned back this tick. The
        // ball is on their face at that moment, so this is exactly how far they
        // had to stretch — the measurement that says whether the shots the
        // controller *plans* are the shots it actually produces.
        // The controller has just hit it if the ball turned away this tick. The
        // plan standing at that moment is the one this shot came from.
        if !was_going_right
            && velocity.x > 0.0
            && let Some(plan) = last_plan
        {
            session.planned += 1;
            session.threat.0 = session.threat.0.min(plan.threat());
            session.threat.1 = session.threat.1.max(plan.threat());
            threat_total += plan.threat();
            pending = Some(plan);
        }
        if was_going_right && velocity.x < 0.0 {
            let theirs = paddles
                .iter()
                .find(|(side, _)| *side == Side::Right)
                .map_or(0.0, |(_, position)| position.y);
            session.their_stretch = session.their_stretch.max((ball.y - theirs).abs());
            session.their_touches += 1;
            // And the whole question in one number: did the shot land where the
            // plan said it would?
            if let Some(plan) = pending.take() {
                let error = (plan.landing - ball.y).abs();
                session.aim_error.0 = session.aim_error.0.max(error);
                session.aim_error.1 += error;
                session.shots += 1;
            }
        }
        was_going_right = velocity.x > 0.0;

        let rally = matches!(board.phase, Phase::Rally);
        if rally {
            session.ball_extent = session.ball_extent.max(ball.abs());
        }
        for (side, position) in &paddles {
            let span = match side {
                Side::Left => &mut session.player_span,
                Side::Right => &mut session.opponent_span,
            };
            span.0 = span.0.min(position.y);
            span.1 = span.1.max(position.y);
        }

        // The controller's own contract, measured on the choices it made: an
        // approach opens when the ball turns towards this paddle and closes
        // when the ball turns away again (met) or the opponent scores (missed).
        let conceded = board.right;
        if conceded != points_conceded {
            approach_open = false;
            points_conceded = conceded;
        } else if rally && velocity.x < 0.0 && !approach_open {
            approach_open = true;
            session.approaches += 1;
        } else if approach_open && velocity.x > 0.0 {
            approach_open = false;
            session.met += 1;
        }

        if let Some(recorder) = recorder.as_mut() {
            let frame = recorder.draw(&mut sim);
            if rally {
                session.rally = Some(Snapshot {
                    frame: frame.clone(),
                    ball,
                    paddles: paddles.clone(),
                });
            }
            session.last_frame = Some(frame);
        }

        if let Phase::Over { .. } = board.phase {
            session.ended_at = Some(tick);
            session.fingerprint = vec![
                ball.x.to_bits(),
                ball.y.to_bits(),
                velocity.x.to_bits(),
                velocity.y.to_bits(),
                board.left,
                board.right,
                board.longest_rally,
                board.top_speed.to_bits(),
            ];
            break;
        }
    }
    if session.planned > 0 {
        session.mean_threat = threat_total / session.planned as f32;
    }
    session
}

/// W and S, held in turn, against a paddle that is otherwise not being driven.
///
/// The closed-loop controller above cannot answer this: it presses whichever
/// key gets it where it wants to go, so a game with W and S swapped would look
/// exactly the same through it. A blind script is the right instrument for the
/// one question that does not depend on the game answering back.
fn controls_move_the_right_way(checks: &mut Checks) {
    /// Long enough for each hold to run past the clamp rather than merely up to
    /// it, so the clamp is exercised and not just not violated.
    const RUN: u64 = 90;
    let mut sim = headless(config(), register);
    let script = InputScript::new().hold(Key::W, 1..30).hold(Key::S, 35..RUN);
    let mut track = Vec::new();
    for tick in 1..=RUN {
        sim.world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        sim.tick();
        let found = sim
            .world()
            .query::<(&Transform, &Paddle)>()
            .find(|(_, _, paddle)| paddle.control == Control::Keys)
            .map(|(_, transform, _)| transform.pos.y);
        match found {
            Some(y) => track.push(y),
            None => fail("the player's paddle is gone", "Startup spawns one per side"),
        }
    }
    let (mut top_at, mut bottom_at) = (0usize, 0usize);
    for (index, y) in track.iter().enumerate() {
        if greater(track[top_at], *y) {
            top_at = index;
        }
        if greater(*y, track[bottom_at]) {
            bottom_at = index;
        }
    }
    let (top, bottom) = (track[top_at], track[bottom_at]);
    // Y is down, so the top of the screen is the smaller number.
    checks.require(
        near(top, -PADDLE_LIMIT) && near(bottom, PADDLE_LIMIT),
        "the player's paddle did not come to rest against both ends of its field",
        format!(
            "it reached {top:.3} and {bottom:.3}; the clamp is +/-{PADDLE_LIMIT:.2} and each \
             hold lasts longer than the travel available"
        ),
    );
    // Both ends are reached either way round, so only the order tells a swap
    // apart: W is held first, so the top must come first.
    checks.require(
        top_at < bottom_at,
        "W and S move the paddle the wrong way round",
        format!(
            "the script holds W first, but the paddle was at the bottom on tick {} before it \
             was at the top on tick {}",
            bottom_at + 1,
            top_at + 1
        ),
    );
}

/// Ask the swept collision test its contract directly, in the cases a played
/// match never reaches.
///
/// The ball is capped at 0.45 units of travel a tick against a paddle 0.7
/// thick, so it *cannot* tunnel and no rally will ever distinguish this
/// function from a naive position test. The margin is real and a played game
/// cannot see it, so the function is asked rather than the match.
fn the_sweep_holds_its_contract(checks: &mut Checks) {
    let face = face_of(Side::Right);
    let reach = PADDLE.y + BALL_RADIUS;
    // Eight units of travel in one tick, straight through the paddle: many
    // times anything the game can produce.
    let from = Vec2::new(face - 8.0, 0.0);
    let to = Vec2::new(face + 2.0, 0.0);
    let hit = crate::paddle_contact(from, to, Side::Right, 0.0);
    match hit {
        None => checks.require(
            false,
            "a ball that crossed a paddle in one long tick was not caught",
            format!(
                "travelling from x={:.2} to x={:.2} past a face at x={face:.2} with the paddle \
                 at y=0: the position at the end of the tick is past it, which is exactly the \
                 case a position-only test misses",
                from.x, to.x
            ),
        ),
        Some(contact) => {
            checks.require(
                greater(contact.at_fraction, 0.0) && greater(1.0, contact.at_fraction),
                "the crossing was not found inside the tick",
                format!(
                    "it reports {:.4} of the way through, which is not between 0 and 1",
                    contact.at_fraction
                ),
            );
            checks.require(
                near(contact.centre.x, face - BALL_RADIUS),
                "the contact is not on the paddle's face",
                format!(
                    "the ball's centre is reported at x={:.4}; a ball of radius {BALL_RADIUS} \
                     touching a face at x={face:.2} has its centre at {:.4}",
                    contact.centre.x,
                    face - BALL_RADIUS
                ),
            );
            checks.require(
                near(contact.offset, 0.0),
                "a ball down the middle did not report a middle contact",
                format!("it reports an offset of {:.4}, wanted 0", contact.offset),
            );
        }
    }
    // Past the end of the paddle: the same travel, with the paddle moved out of
    // the way. This is a point, not a hit.
    let missed = crate::paddle_contact(from, to, Side::Right, reach + 1.0);
    checks.require(
        missed.is_none(),
        "a ball that went past the end of a paddle was counted as a hit",
        format!(
            "the paddle is at y={:.2} and the ball crosses at y=0, which is {:.2} past where it \
             stops touching; got {missed:?}",
            reach + 1.0,
            1.0
        ),
    );
    // Leaving through the same face, which is the ball already behind it.
    let leaving = crate::paddle_contact(to, from, Side::Right, 0.0);
    checks.require(
        leaving.is_none(),
        "a ball travelling away from a paddle was counted as a hit",
        format!(
            "crossing the same face from x={:.2} to x={:.2}; got {leaving:?}",
            to.x, from.x
        ),
    );
    // Not reaching it at all.
    let short = crate::paddle_contact(from, Vec2::new(face - 4.0, 0.0), Side::Right, 0.0);
    checks.require(
        short.is_none(),
        "a ball that never reached the paddle was counted as a hit",
        format!("it stopped {:.2} short of the face; got {short:?}", 4.0),
    );

    // And the bounce the contact feeds: the middle sends it straight back, the
    // end throws it at the full angle, and neither changes its speed.
    let flat = bounce_off(Side::Right, 0.0, 20.0);
    checks.require(
        near(flat.y, 0.0) && greater(0.0, flat.x) && near(flat.length(), 20.0),
        "a ball struck down the middle of a paddle did not come back straight",
        format!(
            "it leaves at ({:.4}, {:.4}), wanted (-20, 0)",
            flat.x, flat.y
        ),
    );
    let edge = bounce_off(Side::Right, 1.0, 20.0);
    let angle = atan2(edge.y, -edge.x).to_degrees();
    checks.require(
        within(angle, MAX_BOUNCE.to_degrees(), 0.01) && near(edge.length(), 20.0),
        "a ball struck at the end of a paddle did not leave at the full angle",
        format!(
            "it leaves {angle:.3} degrees off the straight at speed {:.3}; the widest the game \
             allows is {:.1} degrees at an unchanged 20",
            edge.length(),
            MAX_BOUNCE.to_degrees()
        ),
    );
}

/// The numbers the game is built on, stated as requirements rather than as the
/// constants that happen to satisfy them.
///
/// `assert_eq!(what_was_drawn, the_constant_that_drew_it)` moves when somebody
/// moves the constant. These do not: each one says what the game *needs* to be
/// true, in a form a changed constant can break.
fn the_game_is_winnable(checks: &mut Checks, their_touches: u32, conceded: u32) {
    // The other half of "winnable": it must not be *trivially* winnable, or the
    // opponent is scenery. This one is measured rather than derived, on purpose.
    // The derived version — "the opponent must be unbeatable at the speed a
    // ball is served at" — looks like the same statement and is not: it assumes
    // a shot can be placed exactly, and no keyboard controller can place one
    // within seven units on this court. It failed for a game whose rallies ran
    // to eighteen touches.
    let reached = their_touches + conceded;
    checks.require(
        reached > 0 && their_touches * 2 >= reached,
        "the opponent returned hardly anything that reached it",
        format!(
            "it touched {their_touches} of the {reached} balls that got to its end of the \
             court; below half it is scenery rather than an opponent, and the match says \
             nothing about whether the game can be played"
        ),
    );
    let dt = 1.0 / 60.0;
    // Nothing the ball can do steps through a paddle in one tick.
    checks.require(
        greater(PADDLE.x * 2.0, TOP_SPEED * dt),
        "the ball can step clean through a paddle in one tick",
        format!(
            "one tick at the top speed is {:.3} units against a paddle {:.3} thick; collisions \
             are only tested at tick boundaries",
            TOP_SPEED * dt,
            PADDLE.x * 2.0
        ),
    );
    // The steepest shot the game can produce climbs faster than the opponent
    // can follow. Without this the opponent is a wall and no controller, human
    // or otherwise, can ever score against it.
    let (steepest, _) = sin_cos(MAX_BOUNCE);
    let climb = TOP_SPEED * steepest;
    checks.require(
        greater(climb, OPPONENT_SPEED),
        "the opponent cannot be beaten by arithmetic, never mind by playing",
        format!(
            "the steepest ball the game can produce climbs at {climb:.2} units/s and the \
             opponent moves at {OPPONENT_SPEED:.2}, so it can follow anything"
        ),
    );
    // And the player's paddle can follow that same shot, so a miss is a
    // decision rather than a fact about the constants.
    checks.require(
        greater(PLAYER_SPEED, climb),
        "the player cannot reach the fastest ball the game can produce",
        format!(
            "the steepest ball climbs at {climb:.2} units/s and the player's paddle moves at \
             {PLAYER_SPEED:.2}"
        ),
    );
    // The opponent has to be beatable at a speed a rally actually reaches, and
    // that is a stronger statement than "beatable at the top speed" — which is
    // the version this check had first, and which passed happily for an
    // opponent the game could not be scored against at all. The ball only
    // touches its cap at the end of a long rally, so a requirement stated
    // there is a requirement about a case that hardly ever happens.
    //
    // The opponent drifts to the middle and starts chasing when the ball
    // reaches its half, so it answers a shot aimed at either end by covering
    // that distance in the time the ball takes to cross that half. Below the
    // speed where those two are equal it always gets there.
    //
    // The distance is *not* how far the ball lands from the middle. A paddle
    // defends everything within its own half-height plus the ball's radius, so
    // it gets that much for free and only has to travel the rest — 2.35 units
    // of head start here, which is a third of the court. Leaving it out is what
    // made this check pass for two opponents in a row that could not be scored
    // against inside a minute.
    let furthest_landing = COURT.y - BALL_RADIUS;
    let must_travel = furthest_landing - (PADDLE.y + BALL_RADIUS);
    let beatable_above = OPPONENT_SPEED * (PADDLE_X - PADDLE.x) / must_travel;
    let a_few_touches_in = SERVE_SPEED + 4.0 * SPEED_PER_TOUCH;
    checks.require(
        greater(a_few_touches_in, beatable_above),
        "the opponent cannot be beaten at any speed a rally reaches",
        format!(
            "it only starts missing above {beatable_above:.2} units/s; a ball serves at \
             {SERVE_SPEED:.1}, gains {SPEED_PER_TOUCH:.1} a touch and so is at \
             {a_few_touches_in:.2} four touches into a rally. it moves at {OPPONENT_SPEED:.1} \
             and has {:.2} units of ball travel in which to cover the {must_travel:.2} units \
             between where it waits and the furthest a shot can land ({furthest_landing:.2}, \
             less the {:.2} it defends without moving)",
            PADDLE_X - PADDLE.x,
            PADDLE.y + BALL_RADIUS
        ),
    );

    // The court is dark enough to see a white ball on. Stated as the
    // requirement, so it survives somebody changing the colour.
    let court = palette::COURT;
    let brightest = court.r.max(court.g).max(court.b);
    checks.require(
        greater(0.25, brightest) && greater(court.a, 0.99),
        "the court is not dark enough to see a white ball on",
        format!(
            "its brightest channel is {brightest:.3} at alpha {:.2}",
            court.a
        ),
    );
}

/// Every string this game draws, checked for characters the font actually has.
///
/// The font covers space through `~` and draws everything else as a box at
/// exactly a letter's advance, so a stray em dash or curly quote produces a
/// quad of the right size in the right place and no assertion over what was
/// drawn can tell the difference.
fn every_string_is_printable(checks: &mut Checks) {
    let banners = [
        banner_text(Side::Left, WIN_SCORE, 0),
        banner_text(Side::Right, 0, WIN_SCORE),
        format!("{WIN_SCORE}"),
    ];
    let strings: Vec<&str> = [HINT, HINT_OVER]
        .into_iter()
        .chain(banners.iter().map(String::as_str))
        .collect();
    for text in strings {
        checks.require(
            text.chars().all(|c| (' '..='~').contains(&c)),
            "a string this game draws has a character the font does not have",
            format!(
                "{text:?} — the font draws a box at a letter's width for anything outside \
                 space through ~, so no assertion over what was drawn can see this"
            ),
        );
    }
}

/// Nothing in `frame` is drawn outside what the camera shows.
fn nothing_is_off_screen(checks: &mut Checks, frame: &FrameRecord, view: Rect, screen: &str) {
    let mut worst: Option<Rect> = None;
    for quad in frame.quads() {
        let bounds = quad.bounds();
        if view.contains_rect(bounds) {
            continue;
        }
        worst = Some(bounds);
    }
    checks.require(
        worst.is_none(),
        "something is drawn off screen",
        format!(
            "on the {screen} screen, {worst:?} against a camera showing {view:?} — text centred \
             by width_of is the usual culprit"
        ),
    );
}

/// Draw one frame of a screen the match never reached, and check it.
///
/// Three lines per screen and it is the only thing that ever measures the
/// losing banner: a controller good enough to win the match is a controller
/// that never loses it, so the longest string in the game is the one string a
/// successful run never draws.
fn staged_screen(
    checks: &mut Checks,
    sim: &mut HeadlessSim,
    recorder: &mut FrameRecorder,
    view: Rect,
    name: &str,
    board: Scoreboard,
) -> FrameRecord {
    sim.world_mut().insert_resource(board);
    let frame = recorder.draw(sim);
    nothing_is_off_screen(checks, &frame, view, name);
    checks.require(
        frame.quad_count() > 0,
        "a staged screen drew nothing at all",
        format!("the {name} screen produced an empty frame"),
    );
    frame
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let mut recorder = FrameRecorder::new(WINDOW);
    let font = recorder.font_texture();
    let session = play(Some(&mut recorder));

    let Session {
        ended_at,
        board,
        approaches,
        met,
        threat,
        planned,
        their_stretch,
        their_touches,
        aim_error,
        shots,
        ball_extent,
        player_span,
        opponent_span,
        rally,
        last_frame,
        fingerprint,
        mean_threat,
    } = session;

    // --- the controller's own contract, first ---------------------------
    // If this is bad, every number below it is the controller's fault rather
    // than the game's, and knowing which half to open is worth a whole cycle.
    checks.require(
        approaches > 0 && met * 10 >= approaches * 8,
        "the controller in verify.rs missed most of the balls that came at it",
        format!(
            "it met {met} of {approaches} approaches; below about 8 in 10 the run is measuring \
             the controller's aim rather than the game, so read verify.rs before touching the \
             game's constants"
        ),
    );

    // --- what the match did ---------------------------------------------
    let Some(ended_at) = ended_at else {
        // The controller's own reading goes in the message, because this exact
        // symptom — a long rally at 0-0 — is produced both by a controller that
        // cannot aim and by an opponent that cannot be beaten, and the two need
        // opposite fixes. `met N of N` says which.
        fail(
            "no one won the match",
            &format!(
                "after {TICKS} ticks the score is {}-{}, the longest rally was {} touches and \
                 the ball's top speed was {:.1} units/s (it serves at {SERVE_SPEED:.1} and is \
                 capped at {TOP_SPEED:.1}). the controller met {met} of {approaches} \
                 approaches and aimed its {planned} returns to land {:.2} from the opponent on \
                 average, at best {:.2}, while the opponent touched {their_touches} balls and \
                 stretched at most {their_stretch:.2} from its centre to do it (it covers \
                 {:.2}). those shots landed {:.2} from where they were planned to on average. \
                 if the controller met nearly every ball and its planned threat is much larger \
                 than the stretch the opponent was actually put under, the shots it plans are \
                 not the shots it produces and the fault is in verify.rs",
                board.left,
                board.right,
                board.longest_rally,
                board.top_speed,
                mean_threat,
                threat.1,
                PADDLE.y + BALL_RADIUS,
                if shots > 0 {
                    aim_error.1 / shots as f32
                } else {
                    0.0
                }
            ),
        );
    };
    checks.require(
        board.left == WIN_SCORE,
        "the controller did not win the match",
        format!(
            "it finished {}-{} on tick {ended_at}, having met {met} of {approaches} approaches; \
             a controller that meets nearly every ball and still loses is a game the player \
             cannot win",
            board.left, board.right
        ),
    );
    checks.require(
        matches!(board.phase, Phase::Over { winner: Side::Left }),
        "the match ended in a state that does not match the score",
        format!(
            "the score is {}-{} and the phase is {:?}",
            board.left, board.right, board.phase
        ),
    );
    checks.require(
        board.longest_rally >= 4,
        "no rally in the whole match lasted more than three touches",
        format!(
            "the longest was {} touches over {ended_at} ticks; a game whose ball never comes \
             back twice is not a game",
            board.longest_rally
        ),
    );
    checks.require(
        greater(board.top_speed, SERVE_SPEED) && !greater(board.top_speed, TOP_SPEED),
        "the ball did not speed up during play, or went past its cap",
        format!(
            "its top speed was {:.3} units/s; it serves at {SERVE_SPEED:.1}, gains \
             {SPEED_PER_TOUCH:.1} a touch and is capped at {TOP_SPEED:.1}",
            board.top_speed
        ),
    );
    // The ball stays on court. Y is a wall it must never pass; X it may reach
    // the very edge of, because that is a point.
    checks.require(
        !greater(ball_extent.y, COURT.y - BALL_RADIUS + 1e-3),
        "the ball went through the top or bottom of the court",
        format!(
            "it reached {:.4} from the centre line; the wall stops a ball of radius \
             {BALL_RADIUS} at {:.4}",
            ball_extent.y,
            COURT.y - BALL_RADIUS
        ),
    );
    checks.require(
        !greater(ball_extent.x, COURT.x + BALL_RADIUS + 1e-3),
        "the ball carried on past the end of the court",
        format!(
            "it reached {:.4} from the centre line; a point is scored by {:.4}",
            ball_extent.x,
            COURT.x + BALL_RADIUS
        ),
    );
    // Both paddles used the court, and neither left it.
    for (name, span) in [("player", player_span), ("opponent", opponent_span)] {
        checks.require(
            greater(0.0, span.0) && greater(span.1, 0.0),
            "a paddle spent the whole match on one side of the court",
            format!(
                "the {name}'s paddle ranged over {:.2}..{:.2}; a paddle that only ever moves \
                 one way is a paddle whose other direction is untested",
                span.0, span.1
            ),
        );
        checks.require(
            !greater(span.0.abs().max(span.1.abs()), PADDLE_LIMIT + 1e-3),
            "a paddle left the court",
            format!(
                "the {name}'s paddle ranged over {:.3}..{:.3} against a clamp of \
                 +/-{PADDLE_LIMIT:.3}",
                span.0, span.1
            ),
        );
    }

    controls_move_the_right_way(&mut checks);
    the_sweep_holds_its_contract(&mut checks);
    the_game_is_winnable(&mut checks, their_touches, board.left);
    every_string_is_printable(&mut checks);

    // --- the same match again, bit for bit -------------------------------
    let replay = play(None);
    checks.require(
        replay.fingerprint == fingerprint && replay.ended_at == Some(ended_at),
        "the same match played twice came out differently",
        format!(
            "it ended {}-{} on tick {:?} the second time and {}-{} on tick {ended_at} the \
             first; the seed, the timestep and the controller are all the same, so the world \
             has picked up something that is not",
            replay.board.left, replay.board.right, replay.ended_at, board.left, board.right
        ),
    );

    // --- what was drawn ---------------------------------------------------
    let Some(rally) = rally else {
        fail(
            "no frame was recorded while the ball was live",
            "the loop draws every tick, and a match has rallies in it",
        );
    };
    let Some(final_frame) = last_frame else {
        fail("no frame was recorded at all", "the loop draws every tick");
    };
    // A second world, one tick in, so `Startup` has run: it is where the
    // staged screens below are drawn, and it is also where the camera comes
    // from — read out of the game rather than restated here, so the two cannot
    // drift apart.
    let mut staged = headless(config(), register);
    staged.tick();
    let Some(camera) = staged.world().find_resource::<Camera>().copied() else {
        fail(
            "the game set no camera",
            "set_the_court inserts one, and a headless run is given no default",
        );
    };
    // The recorder's viewport overrides the camera's, so ask the bounds of a
    // camera that has the recorder's — otherwise this judges every quad against
    // a rectangle nothing was drawn into.
    let view = Camera {
        viewport: WINDOW,
        ..camera
    }
    .visible_bounds();

    nothing_is_off_screen(&mut checks, &rally.frame, view, "rally");
    nothing_is_off_screen(&mut checks, &final_frame, view, "final");

    // The clear colour, which leaves no quad behind and is otherwise invisible
    // to every check above.
    checks.require(
        rally.frame.plan.clear_color == palette::COURT,
        "the frame was cleared to the wrong colour",
        format!(
            "it cleared to {:?} and the court is {:?}",
            rally.frame.plan.clear_color,
            palette::COURT
        ),
    );

    // Each paddle is drawn at its size, centred where the world puts it.
    // "Something covers this point" is not enough: a paddle-sized quad covers
    // its own centre even when it is drawn a long way out of position.
    for (side, position) in &rally.paddles {
        let drawn = rally.frame.covering(*position).into_iter().any(|quad| {
            let bounds = quad.bounds();
            near(bounds.size().x, PADDLE.x * 2.0)
                && near(bounds.size().y, PADDLE.y * 2.0)
                && near(bounds.center().x, position.x)
                && near(bounds.center().y, position.y)
        });
        checks.require(
            drawn,
            "no paddle-shaped quad was drawn where a paddle is",
            format!(
                "the world puts the {} paddle at ({:.2}, {:.2}), {} by {}; what covers that \
                 point is {}",
                side.name(),
                position.x,
                position.y,
                PADDLE.x * 2.0,
                PADDLE.y * 2.0,
                sizes_covering(&rally.frame, *position)
            ),
        );
        // And it is where the game says a paddle lives, not merely on screen.
        checks.require(
            near(position.x, side.sign() * PADDLE_X),
            "a paddle has wandered off its own end of the court",
            format!(
                "the {} paddle is at x={:.3}; its end is at {:.3}",
                side.name(),
                position.x,
                side.sign() * PADDLE_X
            ),
        );
    }

    // The ball is a circle, so nothing the size of the ball is drawn anywhere:
    // sixteen wedges share its centre and fit inside its box, and their union
    // is exactly that box.
    let union = disc_union(&rally.frame, rally.ball, BALL_RADIUS * 2.0);
    match union {
        None => checks.require(
            false,
            "nothing at all was drawn where the ball is",
            format!(
                "the world has it at ({:.2}, {:.2}); what covers that point is {}",
                rally.ball.x,
                rally.ball.y,
                sizes_covering(&rally.frame, rally.ball)
            ),
        ),
        Some(union) => checks.require(
            near(union.size().x, BALL_RADIUS * 2.0) && near(union.size().y, BALL_RADIUS * 2.0),
            "no ball-sized disc is drawn where the ball is",
            format!(
                "the quads covering ({:.2}, {:.2}) span {:.4}x{:.4}, wanted {:.3} square",
                rally.ball.x,
                rally.ball.y,
                union.size().x,
                union.size().y,
                BALL_RADIUS * 2.0
            ),
        ),
    }

    // The score: a glyph where each side's digit is centred. The font atlas is
    // a texture like any other, so "was this text" is "did it sample the font".
    for side in [Side::Left, Side::Right] {
        let at = Vec2::new(side.sign() * SCORE_X, -COURT.y + 0.7 + SCORE_SIZE * 0.5);
        let drawn = rally
            .frame
            .covering(at)
            .into_iter()
            .any(|quad| quad.texture == font);
        checks.require(
            drawn,
            "a side's score is not where the game draws it",
            format!(
                "no glyph covers ({:.2}, {:.2}), the middle of the {} score as centred by \
                 TextStyle::width_of; what is there is {}",
                at.x,
                at.y,
                side.name(),
                sizes_covering(&rally.frame, at)
            ),
        );
    }

    // Draw order, which is the failure nothing else catches: the banner is in
    // the UI band, so the front-most thing at its centre is a glyph and not the
    // court behind it.
    let banner_at = Vec2::ZERO;
    let front = final_frame.covering(banner_at).into_iter().next();
    checks.require(
        front.is_some_and(|quad| quad.texture == font),
        "the winner's banner is behind the court instead of over it",
        format!(
            "the front-most quad at the middle of the screen is {:?}; covering() reads the \
             depth sort backwards, so its first entry is what a player actually sees",
            front.map(|quad| quad.bounds().size())
        ),
    );

    // --- the screens the run never reached --------------------------------
    let lost = staged_screen(
        &mut checks,
        &mut staged,
        &mut recorder,
        view,
        "losing",
        Scoreboard {
            left: 2,
            right: WIN_SCORE,
            phase: Phase::Over {
                winner: Side::Right,
            },
            ..Scoreboard::default()
        },
    );
    // The losing banner is the longest string the game can draw, and a winning
    // run never draws it. Check it is really there rather than merely on
    // screen.
    let losing_text = banner_text(Side::Right, 2, WIN_SCORE);
    checks.require(
        lost.covering(Vec2::ZERO)
            .into_iter()
            .any(|quad| quad.texture == font),
        "the losing banner was not drawn",
        format!("{losing_text:?} should be centred on the middle of the screen"),
    );
    let _ = staged_screen(
        &mut checks,
        &mut staged,
        &mut recorder,
        view,
        "serving",
        Scoreboard::default(),
    );

    let captured = crate::capture::capture_a_frame(&mut checks, &rally.frame);
    let verdict = checks.verdict();

    println!("verified pong over {ended_at} ticks");
    println!(
        "  match: {}-{} to the {}, longest rally {} touches, ball topped out at {:.1} units/s",
        board.left,
        board.right,
        Side::Left.name(),
        board.longest_rally,
        board.top_speed
    );
    println!(
        "  controller: met {met} of {approaches} approaches, planned {planned} returns aimed to \
         land {:.2}..{:.2} from the opponent (mean {mean_threat:.2}, its reach is {:.2})",
        threat.0,
        threat.1,
        PADDLE.y + BALL_RADIUS
    );
    println!(
        "  opponent: touched {their_touches} balls, stretched at most {their_stretch:.2} from \
         its centre to reach one (it covers {:.2})",
        PADDLE.y + BALL_RADIUS
    );
    println!(
        "  aim: {shots} shots landed {:.2} from where they were planned to on average, at worst \
         {:.2}",
        if shots > 0 {
            aim_error.1 / shots as f32
        } else {
            0.0
        },
        aim_error.0
    );
    println!(
        "  paddles: player {:.2}..{:.2}, opponent {:.2}..{:.2}, clamp +/-{PADDLE_LIMIT:.2}",
        player_span.0, player_span.1, opponent_span.0, opponent_span.1
    );
    println!(
        "  ball reached {:.2} across and {:.2} down from the centre; the court is {:.1}x{:.1}",
        ball_extent.x, ball_extent.y, COURT.x, COURT.y
    );
    println!(
        "  frames: {} quads in the last rally frame, {} on the winner's screen",
        rally.frame.quad_count(),
        final_frame.quad_count()
    );
    println!("  checks: {} failed", checks.failures());
    println!("  capture: {captured}");
    print!("{}", rally.frame.transcript());
    verdict
}
