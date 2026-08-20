//! The check: three players, a fixed number of headless ticks, and assertions
//! about what the world did and what was drawn.
//!
//! Run it: `cargo run -p jidousha --example pong -- --verify`
//!
//! It runs the *same* systems and the same config the window does. What differs
//! is only what a person would otherwise supply: the input comes from
//! `controller.rs` rather than a keyboard, and there is no window anywhere.
//!
//! Three sessions, because one controller cannot measure a game's difficulty:
//! the rollout player has to win, the do-nothing player has to lose, and the
//! chaser — a person on their first try — is the one that says whether the game
//! is worth playing at all. A match that only the rollout player is asked about
//! passes just as happily for a game with a groove neither side can leave.
//!
//! Every failed check is collected rather than exited on. An instrument that
//! stops at the first bad reading costs a cycle per fault, and one broken
//! constant here produces half a dozen readings with the diagnostic one in the
//! middle.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder};

use crate::checks::{Checks, fail, greater, near, sizes_covering, within};
use crate::controller::{Brain, Controller, Report};
use crate::{
    BALL_RADIUS, BANNER_HINT, Ball, COURT, COURT_HALF_X, COURT_HALF_Y, DASH_HEIGHT, Face,
    HALF_WIDTH, HINT, HINT_SIZE, HINT_TOP, MARKING, MAX_SPEED, PADDLE_SIZE, PADDLE_X, Paddle,
    SCORE_INSET, SCORE_SIZE, SERVE_SPEED, Scoreboard, Side, VIEW_HEIGHT, WIN_SCORE, config,
    face_crossing, register,
};

/// How long each scripted session runs.
///
/// A minute at the default timestep — long enough for a five-point match
/// at every speed the ball reaches, with room for the serve pauses.
pub(super) const TICKS: u64 = 3600;

/// The viewport every headless run draws to.
///
/// The same 16:9 shape `GameConfig::window_size` opens at, which is the shape
/// `main.rs`'s layout constants were written for. The recorder's viewport
/// overrides the `Camera` resource's, so giving it the one the game's camera
/// already implies is what makes every bounds assertion below about the
/// picture a player sees.
const HEADLESS_VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// The viewport every headless run draws to, for `capture.rs` to check its own
/// aspect against rather than carry a second copy of the number.
pub(super) const fn viewport() -> PhysicalSize {
    HEADLESS_VIEWPORT
}

/// What fraction of the balls reaching it the opponent has to send back.
///
/// Stated as a measurement of the game as played rather than as arithmetic
/// about the game at its limit: "can the player reach the fastest ball this
/// game produces" is a question about a speed a rally touches only at its very
/// end, and it passes for opponents nobody can score against inside a minute.
const OPPONENT_MUST_RETURN: f32 = 0.5;

/// What one session came out as.
pub(super) struct Outcome {
    /// The player's points.
    left: u32,
    /// The machine's points.
    right: u32,
    /// Who won, if the match finished inside [`TICKS`].
    winner: Option<Side>,
    /// The most paddle touches any one rally had.
    longest_rally: u32,
    /// The fastest the ball went, in world units per second.
    top_speed: f32,
    /// How many balls the opponent sent back.
    returned: u32,
    /// How many got past it.
    missed: u32,
    /// The furthest the ball ever got from the centre, in world units.
    ball_reach: Vec2,
    /// What the controller has to say about itself.
    report: Report,
}

impl Outcome {
    /// The score, as it would be read out.
    fn score(&self) -> String {
        format!("{}-{}", self.left, self.right)
    }
}

/// The last frame drawn while a point was actually being played, and the world
/// it was drawn from.
///
/// Not the run's last frame: that one is whatever the match ended on, which is
/// the winner's banner rather than a picture of the game. Every assertion about
/// the ordinary layout reads this one instead.
struct Live {
    /// The frame itself.
    frame: FrameRecord,
    /// Which tick it was drawn on.
    tick: u64,
    /// Where the ball was on that tick.
    ball: Vec2,
    /// Where each paddle was on that tick.
    paddles: Vec<(Side, Vec2)>,
    /// The score on that tick.
    score: (u32, u32),
}

/// Everything a recorded session leaves behind besides its score.
struct Recorded {
    /// The simulation, still alive, for staging the frames the run never
    /// reached.
    sim: HeadlessSim,
    /// The recorder, likewise.
    recorder: FrameRecorder,
    /// The last frame drawn while play was live.
    live: Live,
    /// How many frames were recorded.
    frames: usize,
    /// The game's camera, with the viewport this run drew at stamped on.
    camera: Camera,
    /// Every phase and its systems, in run order.
    schedule: String,
}

/// Play one session with `brain` at the keyboard.
///
/// `record` is what decides whether frames are kept: only the rollout run needs
/// them, and the other two are about the score alone.
fn play(brain: Brain, record: bool) -> (Outcome, Option<Recorded>) {
    let mut sim = headless(config(), register);
    let mut controller = Controller::new(brain);
    let mut recorder = FrameRecorder::new(HEADLESS_VIEWPORT);
    let mut live: Option<Live> = None;
    let mut frames = 0;
    let mut ball_reach = Vec2::ZERO;

    for tick in 1..=TICKS {
        // Read, decide, press, tick — in that order, so the input this tick
        // sees was decided from the world the last one left.
        let snapshot = controller.decide(sim.world());
        sim.world_mut().insert_resource(Input::new(snapshot));
        sim.tick();

        let Some(ball) = ball_of(sim.world()) else {
            fail(
                "the ball is gone",
                "Startup spawns exactly one and nothing despawns it",
            );
        };
        ball_reach = ball_reach.max(ball.abs());

        if record {
            frames += 1;
            let frame = recorder.draw(&mut sim);
            let board = sim.world().resource::<Scoreboard>();
            if board.winner.is_none() && board.serve_in == 0 {
                live = Some(Live {
                    frame,
                    tick,
                    ball,
                    paddles: paddles_of(sim.world()),
                    score: (board.left, board.right),
                });
            }
        }
        if sim.world().resource::<Scoreboard>().winner.is_some() {
            break;
        }
    }

    let board = sim.world().resource::<Scoreboard>();
    let outcome = Outcome {
        left: board.left,
        right: board.right,
        winner: board.winner,
        longest_rally: board.longest_rally,
        top_speed: board.top_speed,
        returned: board.returned_by_opponent,
        missed: board.missed_by_opponent,
        ball_reach,
        report: controller.report,
    };
    if !record {
        return (outcome, None);
    }
    let Some(live) = live else {
        fail(
            "no frame was drawn while a point was in play",
            "every tick between a serve and the point it settles is a live frame",
        );
    };
    let camera = Camera {
        viewport: HEADLESS_VIEWPORT,
        ..*sim.world().resource::<Camera>()
    };
    let schedule = sim.schedule_debug();
    (
        outcome,
        Some(Recorded {
            sim,
            recorder,
            live,
            frames,
            camera,
            schedule,
        }),
    )
}

/// Where the ball is, or `None` before Startup has run.
fn ball_of(world: &World) -> Option<Vec2> {
    world
        .query::<(&Transform, With<Ball>)>()
        .map(|(_, transform, _)| transform.pos)
        .next()
}

/// Where both paddles are, sorted by the side they play rather than by the
/// order the query happened to yield them in.
fn paddles_of(world: &World) -> Vec<(Side, Vec2)> {
    let mut found: Vec<(Side, Vec2)> = world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (paddle.side, transform.pos))
        .collect();
    found.sort_by_key(|(side, _)| *side == Side::Right);
    found
}

/// Draw one frame of a world arranged by hand, for a screen the run never
/// reached.
///
/// Corrective rather than additive: every caller sets the whole scoreboard,
/// including the winner it is not asking about, because whatever the session
/// left behind is otherwise still set and a banner would draw over the frame.
fn stage(session: &mut Recorded, board: Scoreboard, ball: Vec2) -> FrameRecord {
    session.sim.world_mut().insert_resource(board);
    let entities: Vec<Entity> = session
        .sim
        .world()
        .query::<With<Ball>>()
        .map(|(entity, _)| entity)
        .collect();
    for entity in entities {
        session
            .sim
            .world_mut()
            .component_mut::<Transform>(entity)
            .pos = ball;
    }
    session.recorder.draw(&mut session.sim)
}

/// A scoreboard with nothing in play, for a staged frame.
fn staged_board(winner: Option<Side>) -> Scoreboard {
    Scoreboard {
        left: if winner == Some(Side::Left) {
            WIN_SCORE
        } else {
            2
        },
        right: if winner == Some(Side::Right) {
            WIN_SCORE
        } else {
            3
        },
        serve_in: 0,
        winner,
        ..Scoreboard::new()
    }
}

/// Every quad in `frame` that sampled the font.
fn glyphs(frame: &FrameRecord, font: jidousha::testing::BackendTextureId) -> Vec<Rect> {
    frame
        .quads()
        .iter()
        .filter(|quad| quad.texture == font)
        .map(|quad| quad.bounds())
        .collect()
}

/// The union of a list of rectangles.
fn union(rects: impl IntoIterator<Item = Rect>) -> Option<Rect> {
    rects.into_iter().reduce(|so_far, next| Rect {
        min: so_far.min.min(next.min),
        max: so_far.max.max(next.max),
    })
}

/// Everything drawn where the ball is, taken as one shape.
///
/// `ctx.circle` submits sixteen wedges and nothing the size of the disc is
/// drawn anywhere, so "a quad the size of the thing" is the wrong question. All
/// sixteen share the centre as a corner and all sixteen fit inside the circle's
/// bounding box, so the union of the quads covering the centre — filtered to
/// that box, because a centre-line dash may run through it too — is exactly
/// `2r x 2r`.
fn disc_at(frame: &FrameRecord, at: Vec2, radius: f32) -> Option<Rect> {
    let box_of_it = Rect::from_center_size(at, Vec2::splat(radius * 2.0));
    let inside = frame.covering(at).into_iter().filter(|quad| {
        let drawn = quad.bounds();
        // Written out rather than as `Rect::contains`, which is half-open and
        // would throw away the wedges reaching the far edge.
        greater(drawn.min.x, box_of_it.min.x - 0.001)
            && greater(drawn.min.y, box_of_it.min.y - 0.001)
            && greater(box_of_it.max.x + 0.001, drawn.max.x)
            && greater(box_of_it.max.y + 0.001, drawn.max.y)
    });
    union(inside.map(|quad| quad.bounds()))
}

/// Complain about every quad `frame` drew outside what the camera shows.
fn check_on_screen(checks: &mut Checks, frame: &FrameRecord, camera: &Camera, what: &str) {
    let view = camera.visible_bounds();
    let off: Vec<Rect> = frame
        .quads()
        .iter()
        .map(|quad| quad.bounds())
        .filter(|bounds| !view.contains_rect(*bounds))
        .collect();
    checks.require(
        off.is_empty(),
        "something was drawn outside what the camera shows",
        format!(
            "on {what}, {} of {} quads fall outside {view:?}; the first is {:?} — text \
             centred by TextStyle::width_of is the usual culprit",
            off.len(),
            frame.quad_count(),
            off.first(),
        ),
    );
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();

    // --- three players, because one cannot measure a game -----------------
    let (rollout, session) = play(Brain::Rollout, true);
    let Some(mut session) = session else {
        fail(
            "the recorded session came back without its frames",
            "`play` is called with `record` set, which is what builds one",
        );
    };
    let rollout_report = rollout.report.lines();
    let (chaser, _) = play(Brain::Chaser, false);
    let (idle, _) = play(Brain::Idle, false);

    checks.require(
        rollout.winner == Some(Side::Left),
        "the rollout player did not win the match",
        format!(
            "it finished {} after at most {TICKS} ticks, winner {:?}; longest rally {} \
             touches, top ball speed {:.1} units/s. {rollout_report}",
            rollout.score(),
            rollout.winner,
            rollout.longest_rally,
            rollout.top_speed,
        ),
    );
    // The do-nothing player is what proves the game can be *lost*: a game whose
    // ball never reaches the player's goal line passes every check about the
    // rollout run and is not a game.
    checks.require(
        idle.winner == Some(Side::Right) && idle.left == 0,
        "a player who does nothing at all was not beaten five-nil",
        format!(
            "the idle run finished {} with winner {:?}; a ball nobody returns has to \
             reach the left goal line",
            idle.score(),
            idle.winner,
        ),
    );
    // And the chaser is the one that says whether the game is worth playing.
    // Both sides centring on the ball is the classic groove: a rally with
    // nowhere to go, 0-0 for as long as you care to run it, with the rollout
    // player's win hiding it completely.
    checks.require(
        chaser.left + chaser.right > 0 && chaser.longest_rally < 40,
        "the chaser run is a groove rather than a game",
        format!(
            "a player that simply steers at the ball got {} in {TICKS} ticks with a \
             longest rally of {} touches; points nobody can score and rallies that never \
             end are the same fault, and the rollout player's win cannot see it",
            chaser.score(),
            chaser.longest_rally,
        ),
    );
    // Stated as a measurement of the game as played rather than as arithmetic
    // about the fastest ball the game can produce, which is a speed a rally
    // touches only at its very end.
    let reached = rollout.returned + rollout.missed;
    let return_rate = if reached == 0 {
        0.0
    } else {
        rollout.returned as f32 / reached as f32
    };
    checks.require(
        reached > 0 && greater(return_rate, OPPONENT_MUST_RETURN),
        "the opponent does not return enough of what reaches it to be an opponent",
        format!(
            "it sent back {} of the {reached} balls that got to its end ({:.0}%); an \
             opponent under {:.0}% is a wall with a hole in it rather than a game",
            rollout.returned,
            return_rate * 100.0,
            OPPONENT_MUST_RETURN * 100.0,
        ),
    );
    // The three numbers, checked rather than only printed: reading a
    // controller's contract is not the same as it holding.
    checks.require(
        rollout.report.approaches > 0 && rollout.report.met * 4 >= rollout.report.approaches * 3,
        "the rollout player could not reach the ball often enough to be measuring the game",
        format!(
            "{rollout_report}; below three quarters met, what the match says is about the \
             controller rather than about the game"
        ),
    );
    checks.require(
        rollout.report.shots() > 0 && greater(1.0, rollout.report.aim_error()),
        "the rollout player's shots are not the shots it planned",
        format!(
            "{rollout_report}; a shot that lands a long way from its plan means the \
             candidates being scored are positions the paddle cannot stand on"
        ),
    );

    // --- determinism ------------------------------------------------------
    let (again, _) = play(Brain::Rollout, false);
    checks.require(
        again.left == rollout.left
            && again.right == rollout.right
            && again.winner == rollout.winner
            && again.longest_rally == rollout.longest_rally
            && near(again.top_speed, rollout.top_speed),
        "the same session played twice did not come out the same",
        format!(
            "first {} (rally {}, top {:.4}), again {} (rally {}, top {:.4}); the seed, the \
             timestep and the engine's own trigonometry are what make this hold",
            rollout.score(),
            rollout.longest_rally,
            rollout.top_speed,
            again.score(),
            again.longest_rally,
            again.top_speed,
        ),
    );

    // --- the margins a played session cannot reach ------------------------
    //
    // A run only tests the states it reaches, and the safety margin the ball is
    // built on is exactly the state a correct game never gets into. Asked of
    // the `fixed_dt` the engine actually hands the game rather than of the 1/60
    // assumed while writing it.
    let dt = session.sim.world().resource::<Time>().fixed_dt.as_f32();
    let step = MAX_SPEED * dt;
    checks.require(
        greater(PADDLE_SIZE.x, step),
        "the ball can step clean through a paddle in one tick",
        format!(
            "at {MAX_SPEED} units/s and a {dt:.5}s tick it travels {step:.3} units, against \
             a paddle {:.2} thick; nothing in v1 sweeps for you and `Rect::overlaps` never \
             sees the frame where they touched",
            PADDLE_SIZE.x,
        ),
    );
    // And the sweep itself, asked its contract directly rather than hoped at
    // through play: this is the one check in the file that is not about a
    // match. One tick of travel eight units long across a paddle the ball
    // would be far past by the end of it — a position-only test says nothing
    // happened.
    let face = Face {
        plane_x: -PADDLE_X + PADDLE_SIZE.x * 0.5,
        approach: -1.0,
        centre_y: 0.0,
        reach: PADDLE_SIZE.y * 0.5 + BALL_RADIUS,
    };
    let across = face_crossing(
        Vec2::new(-11.0, 0.0),
        Vec2::new(-19.0, 0.0),
        BALL_RADIUS,
        face,
    );
    let want = (face.plane_x - (-11.0 - BALL_RADIUS)) / -8.0;
    let past_the_end = face_crossing(
        Vec2::new(-11.0, 6.0),
        Vec2::new(-19.0, 6.0),
        BALL_RADIUS,
        face,
    );
    let leaving = face_crossing(
        Vec2::new(-19.0, 0.0),
        Vec2::new(-11.0, 0.0),
        BALL_RADIUS,
        face,
    );
    // The third negative case, and the one a played match cannot reach: a ball
    // already behind the paddle and still going, on its way to the goal line.
    // Without the guard against it the crossing comes back at a *negative*
    // fraction of the tick — a contact extrapolated backwards out of this
    // tick's travel — and the ball is bounced off a paddle it went past two
    // ticks ago. A whole session survives that, because by then the ball is a
    // couple of ticks from the goal and the extrapolated contact usually lands
    // off the end of the paddle: deleting the guard changed no score here.
    let behind_it = face_crossing(
        Vec2::new(-16.0, 0.0),
        Vec2::new(-16.4, 0.0),
        BALL_RADIUS,
        face,
    );
    checks.require(
        across.is_some_and(|at| near(at, want))
            && past_the_end.is_none()
            && leaving.is_none()
            && behind_it.is_none(),
        "the swept contact test does not hold its contract",
        format!(
            "eight units of travel straight across the paddle gave {across:?}, wanted \
             Some({want:.4}); the same travel {:.0} units off the end gave {past_the_end:?}, \
             a ball leaving through the same face gave {leaving:?}, and a ball already \
             behind the paddle and still going gave {behind_it:?} — all three wanted None",
            6.0,
        ),
    );

    // --- the order the systems run in -------------------------------------
    //
    // Nothing else in this surface sees a swap of two `add_system` calls: the
    // world ends up in a legal state either way, one tick of paddle travel
    // apart, and every assertion about where things ended up passes. The
    // failure is a ball passing through a paddle closing on it.
    let schedule = &session.schedule;
    let player_at = schedule.find("drive_the_player");
    let opponent_at = schedule.find("drive_the_opponent");
    let ball_at = schedule.find("move_the_ball");
    checks.require(
        match (player_at, opponent_at, ball_at) {
            (Some(player), Some(opponent), Some(ball)) => player < ball && opponent < ball,
            _ => false,
        },
        "the paddles no longer move before the ball does",
        format!(
            "in the schedule, drive_the_player is at {player_at:?}, drive_the_opponent at \
             {opponent_at:?} and move_the_ball at {ball_at:?}; the sweep treats a paddle as \
             standing still at its post-move position, so both have to come first"
        ),
    );

    // --- the ball stayed on the court -------------------------------------
    checks.require(
        greater(COURT_HALF_Y, rollout.ball_reach.y)
            && greater(COURT_HALF_X + 1.0, rollout.ball_reach.x),
        "the ball left the court",
        format!(
            "over {TICKS} ticks it got {:.3} from the centre across and {:.3} down, on a \
             court {COURT_HALF_X} by {COURT_HALF_Y}",
            rollout.ball_reach.x, rollout.ball_reach.y,
        ),
    );
    checks.require(
        greater(rollout.top_speed, SERVE_SPEED) && greater(MAX_SPEED + 0.001, rollout.top_speed),
        "the ball does not speed up through a rally, or speeds past its cap",
        format!(
            "the fastest it went was {:.2} units/s; a serve leaves at {SERVE_SPEED} and the \
             cap is {MAX_SPEED}",
            rollout.top_speed,
        ),
    );

    // --- what was drawn, on a frame from a point actually being played -----
    //
    // Not the run's last frame: that one is the winner's banner.
    let camera = session.camera;
    let view = camera.visible_bounds();
    let font = session.recorder.font_texture();
    let live_frame = session.live.frame.clone();
    let live_ball = session.live.ball;
    let live_paddles = session.live.paddles.clone();
    let live_tick = session.live.tick;
    let live_score = session.live.score;

    // The layout is a layout for one aspect, and this is the line that says
    // which. Every named position in `main.rs` is measured against a court
    // this wide; a viewport of another shape moves the edges and nothing else
    // here would notice.
    checks.require(
        near(view.size().x * 0.5, HALF_WIDTH) && near(view.size().y, VIEW_HEIGHT),
        "the camera does not show the court the layout was written for",
        format!(
            "it shows {:.3} by {:.3} world units at {}x{}; the constants are laid out for              {:.3} by {VIEW_HEIGHT:.1}",
            view.size().x,
            view.size().y,
            HEADLESS_VIEWPORT.width,
            HEADLESS_VIEWPORT.height,
            HALF_WIDTH * 2.0,
        ),
    );
    checks.require(
        session.frames > 0 && session.frames as u64 <= TICKS,
        "the recorded run did not draw one frame per tick",
        format!("{} frames over at most {TICKS} ticks", session.frames),
    );

    // How big the court actually is, read off the markings that draw it rather
    // than off the constant that placed them — so the requirement below is
    // about the picture and not about a number the game owns.
    let drawn_court = union(
        live_frame
            .quads()
            .iter()
            .filter(|quad| quad.tint == MARKING)
            .map(|quad| quad.bounds()),
    );

    // Both paddles, by their *bounds* rather than by something being there. A
    // paddle-sized quad covers its own centre even when it is drawn a long way
    // out of position, so "a quad of the right size covers this point" passes
    // for a paddle half out of place.
    for (side, pos) in &live_paddles {
        let want = Rect::from_center_size(*pos, PADDLE_SIZE);
        let found = live_frame.covering(*pos).into_iter().find(|quad| {
            let bounds = quad.bounds();
            near(bounds.min.x, want.min.x)
                && near(bounds.min.y, want.min.y)
                && near(bounds.max.x, want.max.x)
                && near(bounds.max.y, want.max.y)
        });
        checks.require(
            found.is_some(),
            "a paddle is not drawn where the world puts it",
            format!(
                "the {side:?} paddle is at ({:.3}, {:.3}) and should span {want:?}; what \
                 covers that point on tick {live_tick} is {}",
                pos.x,
                pos.y,
                sizes_covering(&live_frame, *pos),
            ),
        );
    }

    // And a claim about the paddles that `PADDLE_SIZE` cannot move with. The
    // check above compares what was drawn against the number that drew it, so
    // it goes on passing after somebody changes that number — and a paddle half
    // the height of the goal behind it is a game with nothing to get past,
    // which every other check in this file survives.
    let paddle_share = drawn_court.map(|court| PADDLE_SIZE.y / court.size().y);
    checks.require(
        paddle_share.is_some_and(|share| greater(share, 0.12) && greater(0.35, share)),
        "a paddle is not a defensible fraction of the goal behind it",
        format!(
            "the paddles are {:.2} long against a court {:?} tall, which is {}; a paddle \
             wants between an eighth and a third of the goal it defends — much more and \
             there is nothing to get past, much less and nothing can be returned",
            PADDLE_SIZE.y,
            drawn_court.map(|court| court.size().y),
            paddle_share.map_or("nothing at all".to_owned(), |share| format!(
                "{:.0}%",
                share * 100.0
            )),
        ),
    );

    let disc = disc_at(&live_frame, live_ball, BALL_RADIUS);
    let disc_size = disc.map_or(Vec2::ZERO, |rect| rect.size());
    checks.require(
        near(disc_size.x, BALL_RADIUS * 2.0) && near(disc_size.y, BALL_RADIUS * 2.0),
        "no ball-sized disc is drawn where the ball is",
        format!(
            "on tick {live_tick} the world has it at ({:.3}, {:.3}); the wedges covering \
             that point span {:.3}x{:.3}, and a radius of {BALL_RADIUS} is {:.3} square. \
             Everything there is {}",
            live_ball.x,
            live_ball.y,
            disc_size.x,
            disc_size.y,
            BALL_RADIUS * 2.0,
            sizes_covering(&live_frame, live_ball),
        ),
    );
    // And a second claim the constant cannot move with: the ball has to be
    // small against the paddle it gets past, or the game is a different one.
    checks.require(
        greater(PADDLE_SIZE.y * 0.5, disc_size.y) && greater(disc_size.y, 0.2),
        "the ball is not a readable size against the paddles",
        format!(
            "it is drawn {:.3} across on a court {:.1} tall against paddles {:.1} long",
            disc_size.y,
            view.size().y,
            PADDLE_SIZE.y,
        ),
    );

    // --- the score: where it is, not merely that it is on screen ----------
    //
    // Checked against the requirement rather than against the constant that
    // placed it. `quad.min.y < SCORE_TOP + margin` moves with SCORE_TOP: put
    // that constant in the middle of the court and the check follows it down,
    // passes, and leaves the score across the play.
    let top_third = view.min.y + view.size().y / 3.0;
    let score_glyphs: Vec<Rect> = glyphs(&live_frame, font)
        .into_iter()
        .filter(|bounds| greater(top_third, bounds.max.y))
        .collect();
    let left_digits: Vec<Rect> = score_glyphs
        .iter()
        .copied()
        .filter(|bounds| greater(0.0, bounds.center().x))
        .collect();
    let right_digits: Vec<Rect> = score_glyphs
        .iter()
        .copied()
        .filter(|bounds| greater(bounds.center().x, 0.0))
        .collect();
    let expected_digits =
        live_score.0.to_string().chars().count() + live_score.1.to_string().chars().count();
    checks.require(
        score_glyphs.len() == expected_digits
            && !left_digits.is_empty()
            && !right_digits.is_empty(),
        "the score is not one number either side of the centre line, in the top third",
        format!(
            "the score on tick {live_tick} was {}-{}, which is {expected_digits} characters; \
             {} glyphs are in the top third of the court (above y {top_third:.2}), {} left \
             of the centre line and {} right of it",
            live_score.0,
            live_score.1,
            score_glyphs.len(),
            left_digits.len(),
            right_digits.len(),
        ),
    );
    if let (Some(left), Some(right)) = (union(left_digits), union(right_digits)) {
        checks.require(
            near(-left.max.x, right.min.x) && near(left.size().y, right.size().y),
            "the two halves of the score are not evenly set about the centre line",
            format!(
                "the left number ends at x {:.3} and the right begins at x {:.3}; they are \
                 {:.3} and {:.3} tall",
                left.max.x,
                right.min.x,
                left.size().y,
                right.size().y,
            ),
        );
        checks.require(
            near(right.min.x, SCORE_INSET) && near(left.size().y, SCORE_SIZE),
            "the score is not the size or the inset the game names",
            format!(
                "it is {:.3} tall and set {:.3} from the centre; the constants are \
                 {SCORE_SIZE} and {SCORE_INSET}",
                left.size().y,
                right.min.x,
            ),
        );
    }

    // --- the bands, where the sort disagrees with the submission order ----
    //
    // `register` submits the play *first* and the court *after* it for exactly
    // this reason: a frame carries the order quads were drawn in, not the
    // `Depth` that produced it, so a band is only visible where it changes that
    // order. Where a game's submission order already agrees with its bands, no
    // assertion over drawn quads can see a layer at all.
    let quads = live_frame.quads();
    let marking_at = quads.iter().position(|quad| quad.tint == MARKING);
    let ball_at = quads.iter().position(|quad| {
        let bounds = quad.bounds();
        quad.texture != font
            && within(bounds.center().y, live_ball.y, BALL_RADIUS)
            && within(bounds.center().x, live_ball.x, BALL_RADIUS)
    });
    checks.require(
        match (marking_at, ball_at) {
            (Some(marking), Some(ball)) => marking < ball,
            _ => false,
        },
        "the court's markings are not drawn behind the play",
        format!(
            "as indices into the draw order: the first marking is {marking_at:?} and the \
             ball {ball_at:?}; the game submits the court *after* the play, so only FIELD \
             sorting under PLAY can put it first. None means that band drew nothing"
        ),
    );

    // --- the frames a played session never produces -----------------------
    //
    // Staged rather than hoped for: the run never parks the ball on a dash or
    // under the hint, so the two band boundaries that matter are otherwise
    // untested. Every piece of state is set, including the winner nothing here
    // is asking about — whatever the match left behind is still set otherwise.
    let on_a_dash = Vec2::new(0.0, DASH_HEIGHT * 0.25);
    let dash_frame = stage(&mut session, staged_board(None), on_a_dash);
    let front_on_dash = dash_frame.covering(on_a_dash).into_iter().next();
    checks.require(
        front_on_dash.is_some_and(|quad| {
            quad.texture != font && within(quad.bounds().center().y, on_a_dash.y, BALL_RADIUS)
        }),
        "the ball is drawn behind the centre-line marking rather than over it",
        format!(
            "with the ball parked on a dash at ({:.2}, {:.2}), the front-most quad there is \
             {:?} spanning {:?}; PLAY has to sort over FIELD",
            on_a_dash.x,
            on_a_dash.y,
            front_on_dash.map(|quad| quad.tint),
            front_on_dash.map(|quad| quad.bounds()),
        ),
    );
    check_on_screen(&mut checks, &dash_frame, &camera, "a staged live frame");

    let under_hint = Vec2::new(0.0, HINT_TOP + HINT_SIZE * 0.5);
    let hint_frame = stage(&mut session, staged_board(None), under_hint);
    let front_under_hint = hint_frame.covering(under_hint).into_iter().next();
    checks.require(
        front_under_hint.is_some_and(|quad| quad.texture == font),
        "the hint is drawn behind the ball rather than over it",
        format!(
            "with the ball parked under the middle of the hint at ({:.2}, {:.2}), the \
             front-most quad there is {:?}; UI has to sort over PLAY",
            under_hint.x,
            under_hint.y,
            front_under_hint.map(|quad| quad.bounds()),
        ),
    );

    // Both end screens, because a rollout player good enough to win is a player
    // that never loses: the losing banner is the longest string in the game and
    // a played run draws it exactly never.
    for winner in [Side::Left, Side::Right] {
        let banner = stage(&mut session, staged_board(Some(winner)), Vec2::ZERO);
        check_on_screen(
            &mut checks,
            &banner,
            &camera,
            &format!("the {winner:?}-wins end screen"),
        );
        let drawn = glyphs(&banner, font).len();
        let want =
            winner.name().chars().count() + BANNER_HINT.chars().count() + HINT.chars().count() + 2;
        checks.require(
            drawn == want,
            "the winner's screen does not draw the words it is made of",
            format!(
                "the {winner:?}-wins screen drew {drawn} glyphs; \"{}\" plus \"{BANNER_HINT}\" \
                 plus the hint and a two-digit score is {want} characters",
                winner.name(),
            ),
        );
    }

    check_on_screen(&mut checks, &live_frame, &camera, "the last live frame");

    // --- the background, which leaves no quad behind ----------------------
    let cleared = live_frame.plan.clear_color;
    checks.require(
        cleared == COURT,
        "the court was cleared to a colour the game does not name",
        format!("the frame cleared to {cleared:?}; the game's constant is {COURT:?}"),
    );
    // And the requirement the colour exists to meet, which the constant cannot
    // move with: a pale ball has to read on it.
    let brightness = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        greater(0.25, brightness) && greater(cleared.a, 0.99),
        "the court is not dark enough for a pale ball to read against",
        format!(
            "its brightest channel is {brightness:.3} at alpha {:.2}",
            cleared.a
        ),
    );

    // --- the strings themselves -------------------------------------------
    //
    // No assertion over drawn quads can see a wrong character: the font draws
    // an unknown one as a box at exactly a letter's advance, so a stray em dash
    // or curly quote passes the glyph count, the centring and the bounds check
    // alike. The string is the only instrument there is.
    for (name, text) in [
        ("the hint", HINT),
        ("the banner's second line", BANNER_HINT),
        ("the winner's banner", Side::Left.name()),
        ("the loser's banner", Side::Right.name()),
    ] {
        let stray = text.chars().find(|glyph| !(' '..='~').contains(glyph));
        checks.require(
            stray.is_none(),
            "a string the game draws has a character the font cannot draw",
            format!(
                "{name} contains {stray:?}, which draws as a box at exactly a letter's \
                 width — no assertion over what was drawn can tell the difference"
            ),
        );
    }

    // --- a picture, for the half no assertion reaches ---------------------
    let captured = crate::capture::capture_a_frame(&mut checks, &live_frame, font);
    let failures = checks.failures();
    let verdict = checks.verdict();

    println!("verified pong over {TICKS} ticks, three players, {failures} problems");
    println!(
        "  {}: {} (winner {:?}), longest rally {} touches, top ball speed {:.1} units/s",
        Brain::Rollout.name(),
        rollout.score(),
        rollout.winner,
        rollout.longest_rally,
        rollout.top_speed,
    );
    println!("  {} controller: {rollout_report}", Brain::Rollout.name());
    println!(
        "  {}: {} (winner {:?}), longest rally {} touches",
        Brain::Chaser.name(),
        chaser.score(),
        chaser.winner,
        chaser.longest_rally,
    );
    println!(
        "  {}: {} (winner {:?})",
        Brain::Idle.name(),
        idle.score(),
        idle.winner
    );
    println!(
        "  opponent returned {} of {reached} balls that reached it ({:.0}%)",
        rollout.returned,
        return_rate * 100.0,
    );
    println!(
        "  live frame: tick {live_tick}, score {}-{}, {} quads, {} glyphs",
        live_score.0,
        live_score.1,
        live_frame.quad_count(),
        glyphs(&live_frame, font).len(),
    );
    println!("  capture: {captured}");
    println!(
        "  camera: {:.1} units tall at {}x{}, ball reach ({:.2}, {:.2})",
        VIEW_HEIGHT,
        HEADLESS_VIEWPORT.width,
        HEADLESS_VIEWPORT.height,
        rollout.ball_reach.x,
        rollout.ball_reach.y,
    );
    print!("{}", live_frame.transcript());
    verdict
}
