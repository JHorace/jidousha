//! The check: play a whole match with nobody watching, then ask what the world
//! did and what was drawn.
//!
//! `cargo run -p jidousha --example pong -- --verify` runs this. It registers
//! the *same* systems and the same config the window does — a check that built
//! a different game would be checking a different program — and replaces only
//! what a person would otherwise supply: the hands on the keyboard.
//!
//! The controller is the part to distrust. A script cannot play Pong at all
//! (it never returns a ball), and a controller that merely tracks the ball
//! returns it dead flat down the middle against an opponent that tracks too,
//! so the rally never ends and the run reports a correct game as unplayable.
//! So this one plays to *win*: it pushes every contact point it could reach
//! through the game's own bounce function and takes the one that lands
//! furthest from where the opponent will be. And because "the best shot
//! available" is always the one struck at the very tip — where any error at
//! all is a clean miss — it constrains before it optimises: only offsets well
//! inside the paddle, and only positions it can actually reach in time.
//!
//! None of that is trustworthy because it is written down. It is trustworthy
//! because the run checks the controller's own contract on the numbers it
//! picked — how often the aim actually met the ball, and how far the return it
//! got was from the return it planned — and reports a miss as *the
//! controller's* fault rather than as a fact about the game.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FrameRecord, FrameRecorder, InputEvent, InputSnapshot, SnapshotBuilder,
};

use crate::checks::{Checks, fail, greater, near, sizes_covering, within};
use crate::rules::{
    BALL_LIMIT, BALL_RADIUS, FIELD_HALF, MAX_BALL_SPEED, MAX_BOUNCE, PADDLE_LIMIT, PADDLE_SIZE,
    PLAYER_SPEED, SERVE_SPEED, SPEEDUP, Side, advance_ball, bounce_velocity, contact_offset,
    fold_between_walls, opponent_step, sweep_contact,
};
use crate::{
    Ball, EVERY_LITERAL, HINT, HINT_SIZE, HINT_TOP, Paddle, SCORE_OFFSET, SCORE_SIZE, SCORE_TOP,
    Scoreboard, Stage, VIEW_HEIGHT, VIEWPORT, WINNING_SCORE, config, register,
};

/// How long the played match is given to finish.
///
/// A match is five points; at the speeds in `rules.rs` a point takes a couple
/// of seconds, so this is roughly three times what a finished match needs. It
/// is a ceiling on the run, not a target — the check that matters is that the
/// match *ended*, and by how much room to spare.
const MATCH_TICKS: u64 = 2600;

/// How long the idle run gets to lose.
const IDLE_TICKS: u64 = 1500;

/// How far up the paddle from its centre the controller will aim, as a
/// fraction of the paddle's half-height.
///
/// Strictly inside 1.0, and that is the whole point. The sharpest return a
/// paddle can produce is the one struck at its very tip, so an optimiser with
/// the tip on its menu picks the tip every time — and on that boundary half a
/// tick of overshoot is a clean miss rather than a worse return.
const CONTACT_MARGIN: f32 = 0.70;

/// How much of the travel the controller believes it has, when asking whether
/// it can reach a place in time.
const REACH_SAFETY: f32 = 0.85;

/// How many contact points the controller tries per decision.
const CANDIDATES: usize = 13;

/// How far the achieved contact may be from the planned one before the run
/// calls it a controller fault, as a fraction of the paddle's half-height.
const AIM_SLACK: f32 = 0.30;

/// The least fraction of approaches the controller must actually meet.
const MIN_CONTACT_RATE: f32 = 0.75;

/// The least fraction of met balls that must land where they were aimed.
const MIN_AIM_RATE: f32 = 0.75;

/// What the controller can see of the world at the top of a tick.
struct Sight {
    ball: Vec2,
    vel: Vec2,
    speed: f32,
    mine: f32,
    theirs: f32,
    live: bool,
}

/// What the controller decided: where it wants its paddle, and — when it has
/// one — which contact point that position is aiming for.
struct Aim {
    target: f32,
    offset: Option<f32>,
}

/// How many ticks a rolled-forward shot is followed before it is given up on.
///
/// The slowest crossing this game allows is a serve struck at the steepest
/// angle: half its speed goes sideways, so it needs a little over three
/// seconds. This is twice that.
const ROLL_LIMIT: u32 = 400;

/// Where a ball ends up, and where the opponent is when it gets there.
struct Rolled {
    /// The ball's y as it reaches the plane.
    ball: f32,
    /// The opponent's paddle centre at that moment.
    opponent: f32,
    seconds: f32,
}

/// Roll the ball and the opponent forward, through the game's own functions,
/// until the ball reaches `side`'s paddle face.
///
/// This is the controller's whole model of the future and it is not a model at
/// all: `advance_ball`, `opponent_step` and `sweep_contact` are the functions
/// the `Update` systems call, in the order the systems call them. A closed
/// form would have been shorter and would have had to be kept in step with the
/// game by hand — and the opponent chases where the ball *is*, so there is no
/// closed form for where it ends up anyway.
fn roll_forward(ball: Vec2, vel: Vec2, opponent: f32, side: Side, dt: f32) -> Option<Rolled> {
    let plane = side.contact_plane();
    let approach = -side.hits_toward();
    let (mut ball, mut vel, mut opponent) = (ball, vel, opponent);
    for tick in 0..ROLL_LIMIT {
        // The systems run steer, then move, then ball — so the opponent reacts
        // to where the ball was at the top of the tick.
        opponent = opponent_step(opponent, ball.y, dt);
        let to = ball + vel * dt;
        // An infinite span asks only "did it reach the plane this tick", which
        // is the question here: how far the paddle is from it is the answer
        // being measured rather than a condition on it.
        if let Some(t) = sweep_contact(ball, to, plane, approach, opponent, f32::INFINITY) {
            return Some(Rolled {
                ball: fold_between_walls(ball.y + (to.y - ball.y) * t, BALL_LIMIT),
                opponent,
                seconds: (tick as f32 + t) * dt,
            });
        }
        let stepped = advance_ball(ball, vel, dt);
        ball = stepped.0;
        vel = stepped.1;
    }
    None
}

/// Where to stand, and what to aim for.
///
/// Constrain, then optimise. The set searched is the contact points that
/// really make contact, with margin, *and* that the paddle can reach before
/// the ball arrives; the score inside that set is how far past the opponent's
/// paddle the return would land, with the opponent rolled forward through its
/// own function rather than guessed at. When nothing survives both constraints,
/// run at the ball — a late return beats a beautiful one that is not there.
fn plan(sight: &Sight, dt: f32) -> Aim {
    let home = Aim {
        target: 0.0,
        offset: None,
    };
    if !sight.live {
        return home;
    }
    // Leg one: where the ball reaches me, and where the opponent has drifted
    // to by then. Candidate-independent, so it is rolled once.
    let Some(arrival) = roll_forward(sight.ball, sight.vel, sight.theirs, Side::Left, dt) else {
        // The ball is going the other way, or will not arrive: go back to the
        // middle, which is the position with the smallest worst case.
        return home;
    };
    let my_plane = Side::Left.contact_plane();
    let next_speed = (sight.speed + SPEEDUP).min(MAX_BALL_SPEED);
    let half = PADDLE_SIZE.y * 0.5;
    let reach = half + BALL_RADIUS;

    let mut best: Option<(f32, f32, f32)> = None;
    for index in 0..CANDIDATES {
        let offset =
            -CONTACT_MARGIN + 2.0 * CONTACT_MARGIN * index as f32 / (CANDIDATES - 1) as f32;
        let target = arrival.ball - offset * half;
        // (a) a position the paddle may legally occupy, so the contact really
        // happens where the offset says it does.
        if target.abs() > PADDLE_LIMIT {
            continue;
        }
        // (b) a position it can get to before the ball does.
        if (target - sight.mine).abs() > PLAYER_SPEED * arrival.seconds * REACH_SAFETY {
            continue;
        }
        // Leg two: the shot this contact would produce, followed to the other
        // end of the court with the opponent chasing it the whole way.
        let outgoing = bounce_velocity(offset, next_speed, Side::Left.hits_toward());
        let Some(landing) = roll_forward(
            Vec2::new(my_plane, arrival.ball),
            outgoing,
            arrival.opponent,
            Side::Right,
            dt,
        ) else {
            continue;
        };
        // How far past the opponent's paddle it lands. Positive is a point.
        let score = (landing.ball - landing.opponent).abs() - reach;
        if best.is_none_or(|(so_far, _, _)| score > so_far) {
            best = Some((score, target, offset));
        }
    }
    match best {
        Some((_, target, offset)) => Aim {
            target,
            offset: Some(offset),
        },
        // Nothing was both safe and reachable: get in front of it anyway.
        None => Aim {
            target: arrival.ball.clamp(-PADDLE_LIMIT, PADDLE_LIMIT),
            offset: None,
        },
    }
}

/// One frame kept out of the run, with the world state that produced it.
struct Keepsake {
    frame: FrameRecord,
    ball: Vec2,
    left: f32,
    right: f32,
    left_score: u32,
    right_score: u32,
}

/// Everything one played match left behind.
struct Session {
    ticks: u64,
    finished_on: Option<u64>,
    board: Scoreboard,
    fixed_dt: f32,
    /// The furthest the ball ever got, in each direction.
    ball_span: (Vec2, Vec2),
    /// The furthest either paddle's centre ever got from the middle.
    paddle_span: (f32, f32),
    /// Balls that arrived at the player's plane, and balls the player met.
    approaches: u32,
    contacts: u32,
    /// Contacts the controller had a plan for, and those that came out within
    /// `AIM_SLACK` of it.
    aimed: u32,
    aimed_well: u32,
    last: Keepsake,
    rally: Option<Keepsake>,
    /// The camera the game installed, read back out of the world rather than
    /// rebuilt here from the same constants -- a check carrying its own copy of
    /// the framing would keep passing after the framing changed.
    camera: Camera,
}

/// Play one match with the controller at the left paddle, recording every tick.
fn play_a_match(recorder: &mut FrameRecorder) -> Session {
    let mut sim = headless(config(), register);
    let mut keyboard = SnapshotBuilder::new();
    let mut holding = (false, false);

    let mut finished_on = None;
    let mut ball_span = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    let mut paddle_span = (f32::MAX, f32::MIN);
    let (mut approaches, mut contacts) = (0, 0);
    let (mut aimed, mut aimed_well) = (0, 0);
    let mut planned: Option<f32> = None;
    let mut previous: Option<(Vec2, Vec2)> = None;
    let mut last: Option<Keepsake> = None;
    let mut rally: Option<Keepsake> = None;
    let mut ticks = 0;

    for tick in 1..=MATCH_TICKS {
        // On the way into tick 1 there is nothing to look at: Startup runs
        // inside that first tick, so every read here has to tolerate an empty
        // world rather than index into it.
        let sight = look(&sim);
        let aim = match &sight {
            Some(sight) => plan(sight, fixed_dt(&sim)),
            None => Aim {
                target: 0.0,
                offset: None,
            },
        };
        if let Some(sight) = &sight {
            if sight.live && aim.offset.is_some() {
                planned = aim.offset;
            }
            // Stop when a whole step would carry the paddle past the target:
            // chattering either side of it costs exactly the precision the
            // whole search was for.
            let dead = PLAYER_SPEED * fixed_dt(&sim) * 0.5;
            let gap = aim.target - sight.mine;
            let want = (gap < -dead, gap > dead);
            if want.0 != holding.0 {
                keyboard.record(key_event(Key::W, want.0));
                holding.0 = want.0;
            }
            if want.1 != holding.1 {
                keyboard.record(key_event(Key::S, want.1));
                holding.1 = want.1;
            }
        }

        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();
        ticks = tick;

        let Some(after) = look(&sim) else {
            fail(
                "the court is empty after a tick",
                "Startup spawns two paddles and a ball, and nothing despawns them",
            );
        };
        let board = sim.world().resource::<Scoreboard>().clone();
        if after.live {
            ball_span.0 = ball_span.0.min(after.ball);
            ball_span.1 = ball_span.1.max(after.ball);
        }
        paddle_span.0 = paddle_span.0.min(after.mine.min(after.theirs));
        paddle_span.1 = paddle_span.1.max(after.mine.max(after.theirs));

        // A ball that was coming at the player and is now going away was met;
        // one that was coming at the player and is now on the spot was missed.
        if let Some((_, was)) = previous
            && greater(0.0, was.x)
        {
            if greater(after.vel.x, 0.0) {
                approaches += 1;
                contacts += 1;
                // The angle the ball actually left at says exactly which part
                // of the paddle it met -- no guessing back from positions.
                let achieved = atan2(after.vel.y, after.vel.x).as_f32() / MAX_BOUNCE.as_f32();
                if let Some(wanted) = planned.take() {
                    aimed += 1;
                    if within(achieved, wanted, AIM_SLACK) {
                        aimed_well += 1;
                    }
                }
            } else if !after.live {
                approaches += 1;
                planned = None;
            }
        }
        previous = Some((after.ball, after.vel));

        let frame = recorder.draw(&mut sim);
        let keepsake = Keepsake {
            frame,
            ball: after.ball,
            left: after.mine,
            right: after.theirs,
            left_score: board.left,
            right_score: board.right,
        };
        // One frame of live play with both scores on the board and a rally
        // under way: the frame worth asserting the whole picture against, and
        // the frame worth looking at as a PNG.
        if rally.is_none() && after.live && board.touches >= 2 && board.left + board.right >= 2 {
            rally = Some(Keepsake::clone_of(&keepsake));
        }
        last = Some(keepsake);

        if matches!(board.stage, Stage::Over { .. }) {
            finished_on = Some(tick);
            break;
        }
    }

    let Some(last) = last else {
        fail(
            "no tick was run at all",
            "the loop above runs at least once",
        );
    };
    let camera = *sim.world().resource::<Camera>();
    Session {
        ticks,
        finished_on,
        board: sim.world().resource::<Scoreboard>().clone(),
        fixed_dt: fixed_dt(&sim),
        ball_span,
        paddle_span,
        approaches,
        contacts,
        aimed,
        aimed_well,
        last,
        rally,
        camera,
    }
}

impl Keepsake {
    fn clone_of(other: &Keepsake) -> Keepsake {
        Keepsake {
            frame: other.frame.clone(),
            ball: other.ball,
            left: other.left,
            right: other.right,
            left_score: other.left_score,
            right_score: other.right_score,
        }
    }
}

fn key_event(key: Key, down: bool) -> InputEvent {
    if down {
        InputEvent::KeyPressed(key)
    } else {
        InputEvent::KeyReleased(key)
    }
}

fn fixed_dt(sim: &HeadlessSim) -> f32 {
    sim.world()
        .find_resource::<Time>()
        .map_or(1.0 / 60.0, |time| time.fixed_dt.as_f32())
}

/// What the controller can see, or `None` before Startup has run.
fn look(sim: &HeadlessSim) -> Option<Sight> {
    let world = sim.world();
    let (ball, vel, speed) = world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| (transform.pos, ball.vel, ball.speed))
        .next()?;
    let mut mine = None;
    let mut theirs = None;
    for (_, transform, paddle) in world.query::<(&Transform, &Paddle)>() {
        match paddle.side {
            Side::Left => mine = Some(transform.pos.y),
            Side::Right => theirs = Some(transform.pos.y),
        }
    }
    let live = matches!(world.find_resource::<Scoreboard>()?.stage, Stage::Rally);
    Some(Sight {
        ball,
        vel,
        speed,
        mine: mine?,
        theirs: theirs?,
        live,
    })
}

/// Play with nobody at the keyboard at all, which is how a game proves it can
/// be *lost*.
///
/// Not the same thing as inserting no `Input`: the player is there, and doing
/// nothing.
fn play_idle() -> (Scoreboard, Vec2, u64) {
    let mut sim = headless(config(), register);
    let mut ball = Vec2::ZERO;
    let mut ticks = 0;
    for tick in 1..=IDLE_TICKS {
        sim.world_mut()
            .insert_resource(Input::new(InputSnapshot::new()));
        sim.tick();
        ticks = tick;
        if let Some((_, transform, _)) = sim.world().query::<(&Transform, &Ball)>().next() {
            ball = transform.pos;
        }
        if let Some(board) = sim.world().find_resource::<Scoreboard>()
            && matches!(board.stage, Stage::Over { .. })
        {
            break;
        }
    }
    let board = sim
        .world()
        .find_resource::<Scoreboard>()
        .cloned()
        .unwrap_or_else(|| {
            fail(
                "the scoreboard vanished",
                "Startup inserts one and only a restart replaces it",
            )
        });
    (board, ball, ticks)
}

// --- reading a drawn frame ------------------------------------------------

/// Every quad in `frame` that sampled the font.
fn glyphs(frame: &FrameRecord, font: BackendTextureId) -> Vec<Rect> {
    frame
        .quads()
        .into_iter()
        .filter(|quad| quad.texture == font)
        .map(|quad| quad.bounds())
        .collect()
}

/// Nothing was drawn outside what the camera can see.
///
/// The highest-value check a game of shapes and text has, and the one that
/// catches a banner centred by `width_of` running off both edges.
fn nothing_off_screen(checks: &mut Checks, frame: &FrameRecord, view: Rect, screen: &str) {
    let strays: Vec<String> = frame
        .quads()
        .into_iter()
        .map(|quad| quad.bounds())
        .filter(|bounds| !view.contains_rect(*bounds))
        .map(|bounds| {
            format!(
                "({:.2},{:.2})-({:.2},{:.2})",
                bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y
            )
        })
        .collect();
    checks.require(
        strays.is_empty(),
        "something was drawn off screen",
        format!(
            "on the {screen} screen, {} of {} quads fall outside a camera showing \
             ({:.2},{:.2})-({:.2},{:.2}): {}",
            strays.len(),
            frame.quad_count(),
            view.min.x,
            view.min.y,
            view.max.x,
            view.max.y,
            strays.join(" ")
        ),
    );
}

/// A quad the size of a paddle, centred where the world says the paddle is.
///
/// Bounds rather than "something covers this point": a paddle still covers its
/// own centre when it is drawn a long way out of position.
fn paddle_drawn(checks: &mut Checks, frame: &FrameRecord, at: Vec2, which: &str) {
    let found = frame.covering(at).into_iter().any(|quad| {
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
            "the {which} paddle is at ({:.3}, {:.3}) and is {} by {}; what covers that point \
             is {}",
            at.x,
            at.y,
            PADDLE_SIZE.x,
            PADDLE_SIZE.y,
            sizes_covering(frame, at)
        ),
    );
}

/// A ball-sized disc where the world says the ball is.
///
/// `ctx.circle` submits sixteen wedges rather than one square, so nothing the
/// size of the ball is drawn anywhere. What is true is that every wedge has the
/// centre as a corner and fits inside the disc's box, so the union of the quads
/// covering the centre is exactly `2r` square.
fn ball_drawn(checks: &mut Checks, frame: &FrameRecord, at: Vec2) {
    let box_of_it = Rect::from_center_size(at, Vec2::splat(BALL_RADIUS * 2.0));
    let mut union: Option<Rect> = None;
    let mut wedges = 0;
    for quad in frame.covering(at) {
        let drawn = quad.bounds();
        // Written out rather than as `Rect::contains`, which is half-open and
        // would throw away the wedges reaching the far edge.
        let inside = drawn.min.x >= box_of_it.min.x - 1e-3
            && drawn.min.y >= box_of_it.min.y - 1e-3
            && drawn.max.x <= box_of_it.max.x + 1e-3
            && drawn.max.y <= box_of_it.max.y + 1e-3;
        if !inside {
            continue;
        }
        wedges += 1;
        union = Some(match union {
            None => drawn,
            Some(so_far) => Rect {
                min: so_far.min.min(drawn.min),
                max: so_far.max.max(drawn.max),
            },
        });
    }
    let span = union.map(|rect| rect.size()).unwrap_or(Vec2::ZERO);
    checks.require(
        near(span.x, BALL_RADIUS * 2.0) && near(span.y, BALL_RADIUS * 2.0),
        "no ball-sized disc where the world puts the ball",
        format!(
            "the ball is at ({:.3}, {:.3}); {wedges} wedges inside its box span \
             {:.3}x{:.3}, and a disc of radius {BALL_RADIUS} spans {:.3} square. \
             Everything covering that point is {}",
            at.x,
            at.y,
            span.x,
            span.y,
            BALL_RADIUS * 2.0,
            sizes_covering(frame, at)
        ),
    );
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let mut recorder = FrameRecorder::new(VIEWPORT);
    // A plain id, borrowing nothing, so the assertions below stay short.
    let font = recorder.font_texture();
    let session = play_a_match(&mut recorder);
    let board = session.board.clone();

    // --- the game is playable ----------------------------------------
    checks.require(
        session.finished_on.is_some(),
        "nobody won the match",
        format!(
            "after {} ticks it was {}-{}: {} rallies' worth of {} returns, longest rally \
             {} touches, top ball speed {:.1} u/s. A match that does not end in \
             {MATCH_TICKS} ticks is either too slow or unwinnable",
            session.ticks,
            board.left,
            board.right,
            board.left + board.right,
            board.returns,
            board.longest_rally,
            board.top_speed
        ),
    );
    checks.require(
        board.left >= WINNING_SCORE,
        "the controller did not win the match",
        format!(
            "it finished {}-{} on tick {:?}. The controller is the newer and worse-tested \
             of the two, so read its own contract below before touching the game's \
             constants: it met {} of {} approaches and landed {} of {} aimed returns",
            board.left,
            board.right,
            session.finished_on,
            session.contacts,
            session.approaches,
            session.aimed_well,
            session.aimed
        ),
    );
    checks.require(
        board.longest_rally >= 3,
        "no rally lasted long enough to be a rally",
        format!(
            "the longest was {} touches over {} returns in a {}-{} match; a Pong where \
             nobody returns anything twice is a scoring machine, not a game",
            board.longest_rally, board.returns, board.left, board.right
        ),
    );
    checks.require(
        greater(board.top_speed, SERVE_SPEED),
        "the ball never got faster than its serve",
        format!(
            "top speed {:.2} u/s against a serve of {SERVE_SPEED:.2}; every touch is \
             supposed to add {SPEEDUP}",
            board.top_speed
        ),
    );

    // --- the controller's own contract -------------------------------
    // The instrument reports on itself first. "The game is unwinnable" and
    // "my aim missed the ball on 94% of returns" are the same fault seen from
    // two ends, and only one of them sends somebody into the game's constants.
    let contact_rate = if session.approaches == 0 {
        0.0
    } else {
        session.contacts as f32 / session.approaches as f32
    };
    checks.require(
        session.approaches > 0 && contact_rate >= MIN_CONTACT_RATE,
        "the controller kept missing the ball, so nothing below is about the game",
        format!(
            "it met {} of {} balls that reached its plane ({:.0}%), against a floor of \
             {:.0}%. Suspect the reach test in `plan` and CONTACT_MARGIN before suspecting \
             the paddle's speed",
            session.contacts,
            session.approaches,
            contact_rate * 100.0,
            MIN_CONTACT_RATE * 100.0
        ),
    );
    let aim_rate = if session.aimed == 0 {
        0.0
    } else {
        session.aimed_well as f32 / session.aimed as f32
    };
    checks.require(
        session.aimed > 0 && aim_rate >= MIN_AIM_RATE,
        "the controller did not hit the ball where it meant to",
        format!(
            "{} of {} returns came off within {AIM_SLACK} of the planned contact offset \
             ({:.0}%), against a floor of {:.0}%. A shot search grading itself against \
             contacts it did not achieve is measuring nothing",
            session.aimed_well,
            session.aimed,
            aim_rate * 100.0,
            MIN_AIM_RATE * 100.0
        ),
    );

    // --- the world stayed inside its own rules -----------------------
    let (low, high) = session.ball_span;
    checks.require(
        greater(FIELD_HALF.x + 1.0, low.x.abs().max(high.x.abs())),
        "the ball left the court sideways and kept going",
        format!(
            "its x ran from {:.3} to {:.3}; the goal lines are at +/-{:.1} and a point \
             should be scored the tick it passes one",
            low.x, high.x, FIELD_HALF.x
        ),
    );
    checks.require(
        greater(BALL_LIMIT + 0.01, low.y.abs().max(high.y.abs())),
        "the ball went through a wall",
        format!(
            "its y ran from {:.3} to {:.3}; with a radius of {BALL_RADIUS} it may reach \
             +/-{BALL_LIMIT:.3}",
            low.y, high.y
        ),
    );
    checks.require(
        greater(PADDLE_LIMIT + 0.001, session.paddle_span.0.abs())
            && greater(PADDLE_LIMIT + 0.001, session.paddle_span.1.abs()),
        "a paddle left the court",
        format!(
            "paddle centres ran from {:.3} to {:.3}; the clamp is +/-{PADDLE_LIMIT:.3}",
            session.paddle_span.0, session.paddle_span.1
        ),
    );
    // The margin the fixed timestep needs, asked of the timestep the engine
    // actually handed the game rather than of the 1/60 it was picked against.
    let travel = MAX_BALL_SPEED * session.fixed_dt;
    checks.require(
        greater(PADDLE_SIZE.x, travel),
        "the ball can cross a paddle in one tick",
        format!(
            "at {MAX_BALL_SPEED} u/s and a timestep of {:.5}s it travels {travel:.3} per \
             tick, against a paddle {} thick. The swept test in rules.rs is what makes \
             the game correct anyway, so this is the margin that makes it correct twice",
            session.fixed_dt, PADDLE_SIZE.x
        ),
    );

    // --- the game can be lost ----------------------------------------
    let (idle, idle_ball, idle_ticks) = play_idle();
    checks.require(
        idle.right >= WINNING_SCORE,
        "a player who does nothing at all does not lose",
        format!(
            "after {idle_ticks} idle ticks it was {}-{}; a game that cannot be lost by \
             standing still has no goal line on the left",
            idle.left, idle.right
        ),
    );
    let (again, again_ball, _) = play_idle();
    checks.require(
        [idle_ball.x.to_bits(), idle_ball.y.to_bits()]
            == [again_ball.x.to_bits(), again_ball.y.to_bits()]
            && idle.left == again.left
            && idle.right == again.right,
        "the same game played twice did different things",
        format!(
            "the ball finished at ({:.6}, {:.6}) and then at ({:.6}, {:.6}), {}-{} against \
             {}-{}. The serve angle is the only random thing here and it comes from the \
             seeded Rng",
            idle_ball.x,
            idle_ball.y,
            again_ball.x,
            again_ball.y,
            idle.left,
            idle.right,
            again.left,
            again.right
        ),
    );

    // --- the rules' own contracts, asked directly --------------------
    // A played match only exercises the states a correct game reaches, and the
    // margins a game is built on are exactly the ones it never reaches. So ask.
    check_the_sweep(&mut checks);
    check_the_bounce(&mut checks);

    // --- what was drawn ----------------------------------------------
    // The recorder's viewport overrides the `Camera` resource's, and nothing
    // writes it back into the world -- so a check reading bounds from the world
    // and quads from the recorder compares against the wrong rectangle unless
    // the two agree. They are both VIEWPORT, and this asserts it rather than
    // remembering it.
    checks.require(
        session.camera.viewport == VIEWPORT && near(session.camera.height, VIEW_HEIGHT),
        "the recorder is not framing what the game's camera frames",
        format!(
            "the game's camera is {}x{} at {} world units tall and the recorder was given \
             {}x{}; every bounds check below judges against the camera's rectangle",
            session.camera.viewport.width,
            session.camera.viewport.height,
            session.camera.height,
            VIEWPORT.width,
            VIEWPORT.height,
        ),
    );
    let view = session.camera.visible_bounds();
    let Some(rally) = session.rally.as_ref().map(Keepsake::clone_of) else {
        fail(
            "no frame of live play was recorded",
            "the match ran, so some tick had a rally under way with both scores on the \
             board; without one there is no frame to judge the picture against",
        );
    };
    // The background, which is the one part of the picture that leaves no quad
    // behind: a frame drawn on the wrong colour is byte-identical under every
    // other assertion here. `FrameRecord::plan` carries it, so this is two
    // lines rather than something only a person looking at the capture can see.
    //
    // Two questions, and only the second survives the constant itself being
    // changed: that the frame cleared to what the camera asked for, and that
    // what it asked for is dark enough for a white ball to read against. The
    // first is the engine's contract; the second is the game's own, and is
    // spelled with numbers rather than with the constant it is judging.
    let cleared = rally.frame.plan.clear_color;
    checks.require(
        cleared == crate::palette::COURT,
        "the frame did not clear to the colour the camera asked for",
        format!(
            "the plan clears to {cleared:?} and the game's camera is set to {:?}",
            crate::palette::COURT
        ),
    );
    let brightness = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        greater(0.25, brightness) && near(cleared.a, 1.0),
        "the court is not dark enough to see a white ball on",
        format!(
            "its brightest channel is {brightness:.3} at alpha {:.2}; the ball, the walls \
             and the hint are all near-white, and nothing else in this run looks at the \
             background at all",
            cleared.a
        ),
    );
    nothing_off_screen(&mut checks, &rally.frame, view, "rally");
    nothing_off_screen(&mut checks, &session.last.frame, view, "final");
    paddle_drawn(
        &mut checks,
        &rally.frame,
        Vec2::new(Side::Left.paddle_x(), rally.left),
        "left",
    );
    paddle_drawn(
        &mut checks,
        &rally.frame,
        Vec2::new(Side::Right.paddle_x(), rally.right),
        "right",
    );
    ball_drawn(&mut checks, &rally.frame, rally.ball);
    check_the_text(&mut checks, &rally, font);
    check_the_layers(&mut checks, &rally, font);

    // --- the screens the run never reached ---------------------------
    // A controller good enough to finish the match is a controller that never
    // loses it, so the losing banner — the longest string in the game — is the
    // one string nothing above measured.
    let unreached = draw_the_unreached(&mut recorder, &mut checks, view);

    // Every literal, checked as a string rather than as quads: the font draws
    // anything outside space-through-tilde as a box at exactly a letter's
    // advance, so no assertion over what was drawn can tell one from a letter.
    for literal in EVERY_LITERAL {
        checks.require(
            literal.chars().all(|c| (' '..='~').contains(&c)),
            "a string the game draws has a character the font cannot draw",
            format!(
                "{literal:?} is outside space-through-tilde, which the font renders as a \
                 box at a letter's advance -- so it passes every check on the geometry"
            ),
        );
    }

    let captured = crate::capture::capture_a_frame(&mut checks, &rally.frame);
    let verdict = checks.verdict();

    println!(
        "verified pong: {}-{} to the {} on tick {:?}, {} ticks recorded",
        board.left,
        board.right,
        if board.left > board.right {
            "player"
        } else {
            "opponent"
        },
        session.finished_on,
        session.ticks,
    );
    println!(
        "  match: {} returns, longest rally {} touches, top ball speed {:.1} u/s, \
         {:.2}s of play",
        board.returns,
        board.longest_rally,
        board.top_speed,
        session.ticks as f32 * session.fixed_dt,
    );
    println!(
        "  controller: met {}/{} approaches ({:.0}%), aim landed {}/{} ({:.0}%)",
        session.contacts,
        session.approaches,
        contact_rate * 100.0,
        session.aimed_well,
        session.aimed,
        aim_rate * 100.0,
    );
    println!(
        "  idle run: lost {}-{} in {idle_ticks} ticks, and replayed bit for bit",
        idle.left, idle.right
    );
    println!(
        "  rally frame: {} quads at {}-{}, ball ({:.2}, {:.2})",
        rally.frame.quad_count(),
        rally.left_score,
        rally.right_score,
        rally.ball.x,
        rally.ball.y,
    );
    println!("  unreached screens: {unreached}");
    println!("  capture: {captured}");
    println!("  failures: {}", checks.failures());
    // Evidence, kept rather than shown: one frame, not the whole recorder.
    print!("{}", rally.frame.transcript());
    verdict
}

/// The swept contact, asked its contract directly.
///
/// A played match cannot reach these: the speed cap keeps the ball to well
/// under a paddle's thickness per tick, so the sweep never does anything a
/// position test would not, and replacing it with a position test would pass
/// the entire session above. The margin is real and the run cannot see it.
fn check_the_sweep(checks: &mut Checks) {
    let plane = Side::Left.contact_plane();
    let half = PADDLE_SIZE.y * 0.5 + BALL_RADIUS;
    // Eight units of travel in one tick, straight across a paddle 0.6 thick.
    let across = sweep_contact(
        Vec2::new(plane + 4.0, 0.0),
        Vec2::new(plane - 4.0, 0.0),
        plane,
        -1.0,
        0.0,
        half,
    );
    checks.require(
        across.is_some_and(|t| near(t, 0.5)),
        "a ball that crosses a paddle in one tick is not caught",
        format!(
            "eight units of travel centred on the paddle's face gave {across:?}, want \
             Some(0.5). Nothing in the engine sweeps, so a fast ball steps clean through \
             any test that only asks where it ended up"
        ),
    );
    // Past the end of it.
    let past = sweep_contact(
        Vec2::new(plane + 4.0, 0.0),
        Vec2::new(plane - 4.0, 0.0),
        plane,
        -1.0,
        8.0,
        half,
    );
    checks.require(
        past.is_none(),
        "a ball that crosses the plane past the end of the paddle counts as a hit",
        format!(
            "the paddle was at y=8.0 and the ball crossed at y=0.0, {} away from a paddle \
             reaching {half:.2}; got {past:?}",
            8.0
        ),
    );
    // Leaving through the same face, which is what a just-struck ball does.
    let leaving = sweep_contact(
        Vec2::new(plane, 0.0),
        Vec2::new(plane + 8.0, 0.0),
        plane,
        -1.0,
        0.0,
        half,
    );
    checks.require(
        leaving.is_none(),
        "a ball leaving a paddle is caught by it again",
        format!(
            "travel away from the face on the tick after a hit gave {leaving:?}; a ball \
             struck once and re-struck on the way out never leaves the paddle"
        ),
    );
    // And the wall fold, over a ball thrown far past several walls at once.
    let folded = fold_between_walls(BALL_LIMIT * 5.0, BALL_LIMIT);
    checks.require(
        greater(BALL_LIMIT + 1e-4, folded.abs()) && near(folded, BALL_LIMIT),
        "folding a ball back between the walls does not land it between them",
        format!(
            "five half-courts of travel folded to {folded:.4}, want {BALL_LIMIT:.4} -- \
             two reflections and a half"
        ),
    );
}

/// The bounce, asked its contract directly.
fn check_the_bounce(checks: &mut Checks) {
    let flat = bounce_velocity(0.0, 20.0, 1.0);
    checks.require(
        near(flat.y, 0.0) && near(flat.x, 20.0),
        "a ball struck dead centre does not come back flat",
        format!("got ({:.4}, {:.4}), want (20.0, 0.0)", flat.x, flat.y),
    );
    let tip = bounce_velocity(1.0, 20.0, -1.0);
    let angle = atan2(tip.y, -tip.x);
    checks.require(
        near(angle.as_f32(), MAX_BOUNCE.as_f32()) && greater(0.0, tip.x),
        "a ball struck at the tip does not leave at the steepest angle",
        format!(
            "off the right paddle's bottom tip it left at {:.2} degrees going x={:.2}; \
             want {:.2} degrees going left",
            angle.to_degrees(),
            tip.x,
            MAX_BOUNCE.to_degrees()
        ),
    );
    let clamped = bounce_velocity(4.0, 20.0, 1.0);
    checks.require(
        near(clamped.x, tip.x.abs()) && near(clamped.y, tip.y),
        "a contact off the end of the paddle is not clamped to the end of it",
        format!(
            "an offset of 4.0 gave ({:.3}, {:.3}); an offset of 1.0 gives ({:.3}, {:.3})",
            clamped.x,
            clamped.y,
            tip.x.abs(),
            tip.y
        ),
    );
    let outside = contact_offset(100.0, 0.0);
    checks.require(
        near(outside, 1.0),
        "an impossible contact point is not clamped",
        format!("a ball 100 units below the paddle gave an offset of {outside:.3}"),
    );
}

/// Text is where the layout says it is, not merely somewhere on screen.
///
/// "On screen" passes for a hint line drawn on top of a wall, so the bands the
/// layout is built from are what these judge against.
fn check_the_text(checks: &mut Checks, rally: &Keepsake, font: BackendTextureId) {
    let all = glyphs(&rally.frame, font);
    let score_band = (SCORE_TOP - 1e-3, SCORE_TOP + SCORE_SIZE + 1e-3);
    let hint_band = (HINT_TOP - 1e-3, HINT_TOP + HINT_SIZE + 1e-3);
    let in_band =
        |bounds: &Rect, band: (f32, f32)| bounds.min.y >= band.0 && bounds.max.y <= band.1;

    let score: Vec<&Rect> = all.iter().filter(|b| in_band(b, score_band)).collect();
    let hint: Vec<&Rect> = all.iter().filter(|b| in_band(b, hint_band)).collect();
    checks.require(
        score.len() + hint.len() == all.len() && !all.is_empty(),
        "a glyph was drawn outside every band the layout has",
        format!(
            "{} glyphs in all: {} in the score band [{:.2}, {:.2}], {} in the hint band \
             [{:.2}, {:.2}], and {} in neither. A frame of live play draws the score and \
             the hint and nothing else",
            all.len(),
            score.len(),
            score_band.0,
            score_band.1,
            hint.len(),
            hint_band.0,
            hint_band.1,
            all.len() - score.len() - hint.len(),
        ),
    );
    // Two single-digit scores, each its own quad, each centred on its column
    // and exactly a glyph's cell. The font is monospace: `size` tall and
    // `size * 7 / 9` wide.
    let cell = Vec2::new(SCORE_SIZE * 7.0 / 9.0, SCORE_SIZE);
    let placed = |column: f32| {
        score.iter().any(|bounds| {
            near(bounds.center().x, column)
                && near(bounds.center().y, SCORE_TOP + SCORE_SIZE * 0.5)
                && near(bounds.size().x, cell.x)
                && near(bounds.size().y, cell.y)
        })
    };
    checks.require(
        placed(-SCORE_OFFSET) && placed(SCORE_OFFSET),
        "a score is not in its column",
        format!(
            "the two scores are centred at x=+/-{SCORE_OFFSET} with cells {:.3}x{:.3}; \
             the {} glyphs in the score band are at {}",
            cell.x,
            cell.y,
            score.len(),
            score
                .iter()
                .map(|b| format!(
                    "({:.2},{:.2}) {:.2}x{:.2}",
                    b.center().x,
                    b.center().y,
                    b.size().x,
                    b.size().y
                ))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    );
    // The hint, centred by its own measured width. `width_of` is exact and
    // completely silent, so a line one character too long runs off both edges
    // with nothing to say about it.
    let style = TextStyle {
        size: HINT_SIZE,
        ..TextStyle::default()
    };
    let span = hint.iter().fold(None::<Rect>, |so_far, bounds| {
        Some(match so_far {
            None => **bounds,
            Some(rect) => Rect {
                min: rect.min.min(bounds.min),
                max: rect.max.max(bounds.max),
            },
        })
    });
    let (width, centre) = span.map_or((0.0, f32::MAX), |rect| (rect.size().x, rect.center().x));
    checks.require(
        near(width, style.width_of(HINT))
            && near(centre, 0.0)
            && hint.len() == HINT.chars().count(),
        "the hint line is not the width the layout measured, or is not centred",
        format!(
            "{} glyphs spanning {width:.3} centred at {centre:.3}; TextStyle::width_of \
             says {:.3}, the layout centres on x=0, and {:?} is {} characters -- spaces \
             included, because ctx.text submits a quad for one",
            hint.len(),
            style.width_of(HINT),
            HINT,
            HINT.chars().count(),
        ),
    );
}

/// The score is painted on the court, behind the play.
///
/// `quads()` comes back in the depth sort, so an index in it is a place in the
/// painter's sequence. Move `layers::SCORE` above `layers::PLAY` and the score
/// paints over the ball, in the right place, at the right size, with every
/// geometric assertion above still passing — this is the only check that sees
/// it.
fn check_the_layers(checks: &mut Checks, rally: &Keepsake, font: BackendTextureId) {
    let quads = rally.frame.quads();
    let score_band = (SCORE_TOP - 1e-3, SCORE_TOP + SCORE_SIZE + 1e-3);
    let last_score = quads
        .iter()
        .enumerate()
        .filter(|(_, quad)| {
            quad.texture == font
                && quad.bounds().min.y >= score_band.0
                && quad.bounds().max.y <= score_band.1
        })
        .map(|(index, _)| index)
        .next_back();
    let first_paddle = quads
        .iter()
        .position(|quad| near(quad.bounds().size().y, PADDLE_SIZE.y));
    checks.require(
        matches!((last_score, first_paddle), (Some(score), Some(paddle)) if score < paddle),
        "the score is not painted behind the play",
        format!(
            "in draw order the last score glyph is at {last_score:?} and the first paddle \
             at {first_paddle:?}, out of {} quads; the score belongs on the court under \
             the ball",
            quads.len()
        ),
    );
}

/// Build the screens the played match never reached, and judge them the same
/// way.
fn draw_the_unreached(recorder: &mut FrameRecorder, checks: &mut Checks, view: Rect) -> String {
    let mut sim = headless(config(), register);
    sim.world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));
    // One tick, so Startup has run and there is a world to draw.
    sim.tick();

    let screens = [
        (
            "opponent wins",
            Scoreboard {
                left: 3,
                right: WINNING_SCORE,
                stage: Stage::Over {
                    winner: Side::Right,
                },
                ..blank_board()
            },
        ),
        (
            "player wins",
            Scoreboard {
                left: WINNING_SCORE,
                right: 4,
                stage: Stage::Over { winner: Side::Left },
                ..blank_board()
            },
        ),
        (
            "serving",
            Scoreboard {
                left: 2,
                right: 2,
                stage: Stage::Serving {
                    ticks_left: crate::SERVE_PAUSE,
                    toward: 1.0,
                },
                ..blank_board()
            },
        ),
    ];
    for (name, board) in screens {
        sim.world_mut().insert_resource(board);
        let frame = recorder.draw(&mut sim);
        nothing_off_screen(checks, &frame, view, name);
        checks.require(
            frame.quad_count() > 0,
            "a screen drew nothing at all",
            format!("the {name} screen submitted no quads"),
        );
    }
    "opponent wins, player wins, serving".to_owned()
}

fn blank_board() -> Scoreboard {
    Scoreboard {
        left: 0,
        right: 0,
        stage: Stage::Rally,
        touches: 0,
        longest_rally: 0,
        top_speed: 0.0,
        returns: 0,
    }
}
