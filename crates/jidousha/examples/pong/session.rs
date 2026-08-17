//! One played session of Pong, and the controller that plays it.
//!
//! The input is not a script. A script says what the player does before the run
//! starts, and a blind script never returns a ball — it can prove the controls
//! move a paddle and still say nothing about whether the game is playable. So
//! the left paddle is driven by a controller that looks at the ball and records
//! key events through `SnapshotBuilder`, which is the driver's own accumulator
//! and therefore the same edge rules a real keyboard goes through.
//!
//! Nothing here asserts. `play` returns what the session did and `verify.rs`
//! judges it, so the *same* loop can be played twice and the two runs compared
//! — which is the whole determinism claim.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, InputEvent, InputSnapshot, SnapshotBuilder};

use crate::verify::fail;
use crate::{
    BALL_RADIUS, Ball, Control, PADDLE_LIMIT, PADDLE_SIZE, Paddle, Round, SCORE_SIZE, SCORE_TOP,
    Scoreboard, Side, VIEW_HEIGHT, Velocity, config, greater, register,
};

/// How long the session runs.
///
/// Ninety seconds of game at sixty ticks a second — long enough for a match to
/// be won at five points, for the winner's banner to be left up and looked at,
/// and for a second match to start after Space. A shorter run would pass every
/// assertion except the ones about finishing, which are the ones worth having.
pub(super) const TICKS: u64 = 5400;

/// The surface the frames are recorded against.
///
/// The same size the game's camera already has, because the recorder's viewport
/// overrides the camera's and a bounds check comparing quads against a camera
/// of another shape is comparing against the wrong rectangle.
pub(super) const VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// Floating point slack for a position that should be inside a boundary.
pub(super) const SLACK: f32 = 1e-3;

/// How close the controller tries to get to where it is aiming for.
const TRACKING_DEADZONE: f32 = 0.15;

/// How far up the paddle, as a fraction of its reach, the controller tries to
/// meet the ball. Short of 1.0 so a near miss is still a hit — aiming with the
/// very edge is a shot that goes wrong when the tracking is a tick late.
const AIM: f32 = 0.7;

/// How long the controller leaves the winner's screen up before pressing
/// Space. Long enough that its layout is drawn — and therefore bounds-checked —
/// for a while rather than for the single tick an instant restart would give
/// it.
const LINGER: u64 = 90;

/// What one played session did.
///
/// Returned rather than asserted on inside the loop, so the same loop can be
/// played twice and the two runs compared — which is the determinism claim.
pub(super) struct Run {
    /// The ball's position after every tick, as bits, for the replay check.
    pub(super) ball_track: Vec<[u32; 2]>,
    /// The furthest the ball got from the centre, per axis.
    pub(super) ball_extent: Vec2,
    /// The furthest a paddle's centre got from the middle of the field.
    pub(super) paddle_extent: f32,
    /// The final scoreboard.
    pub(super) left: u32,
    pub(super) right: u32,
    pub(super) left_hits: u32,
    pub(super) right_hits: u32,
    pub(super) top_speed: f32,
    pub(super) longest_rally: u32,
    /// The tick a match was first won on, and the tick a new one started.
    pub(super) won_at: Option<u64>,
    pub(super) restarted_at: Option<u64>,
    /// How many frames were drawn, and how many of them drew a glyph.
    pub(super) frames: usize,
    pub(super) frames_with_text: usize,
    /// The first quad that fell outside the camera, if any.
    pub(super) escaped: Option<(u64, Rect)>,
    /// Whether a glyph covered the middle of the score, on the last frame.
    pub(super) score_drawn: bool,
    /// The last frame as text.
    pub(super) transcript: String,
    /// The timestep the engine ran on, read back rather than assumed.
    pub(super) fixed_dt: f32,
}

/// The world rectangle the game is drawn into.
///
/// The camera the game sets in Startup, given the recorder's viewport — because
/// the recorder's viewport overrides the camera's, and a bounds check against a
/// camera of another shape is checking the wrong rectangle.
pub(super) fn on_screen() -> (Vec2, Vec2) {
    Camera {
        height: VIEW_HEIGHT,
        viewport: VIEWPORT,
        ..Camera::default()
    }
    .visible_bounds()
}

/// The first quad in `frame` that is not wholly inside `(top_left, bottom_right)`.
///
/// The highest-value check a game of shapes and text has: `TextStyle::width_of`
/// is exact and completely silent, so a banner one character too long runs off
/// both edges of the screen without a word from anything.
pub(super) fn escaped(frame: &FrameRecord, (top_left, bottom_right): (Vec2, Vec2)) -> Option<Rect> {
    frame
        .quads()
        .iter()
        .map(|quad| quad.bounds())
        .find(|bounds| {
            !(bounds.min.x >= top_left.x - SLACK
                && bounds.min.y >= top_left.y - SLACK
                && bounds.max.x <= bottom_right.x + SLACK
                && bounds.max.y <= bottom_right.y + SLACK)
        })
}

/// Play a whole session, drawing every tick.
pub(super) fn play() -> Run {
    let mut sim = headless(config(), register);
    // Startup runs inside the first tick, and the systems read input rather
    // than asking whether it exists.
    sim.world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));

    let mut recorder = FrameRecorder::new(VIEWPORT);
    // Read before the loop: `draw` borrows the recorder for as long as the
    // frame it hands back is alive.
    let font = recorder.font_texture();
    let bounds = on_screen();

    let mut keyboard = SnapshotBuilder::new();
    let mut holding: Option<Key> = None;
    let mut tapping_space = false;
    let mut over_since: Option<u64> = None;

    let mut run = Run {
        ball_track: Vec::new(),
        ball_extent: Vec2::ZERO,
        paddle_extent: 0.0,
        left: 0,
        right: 0,
        left_hits: 0,
        right_hits: 0,
        top_speed: 0.0,
        longest_rally: 0,
        won_at: None,
        restarted_at: None,
        frames: 0,
        frames_with_text: 0,
        escaped: None,
        score_drawn: false,
        transcript: String::new(),
        fixed_dt: 0.0,
    };

    for tick in 1..=TICKS {
        // --- the controller: look at the world, then press or let go -----
        let want = match (
            aim_for(sim.world()),
            paddle_at(sim.world(), Control::Keyboard),
        ) {
            (Some(target), Some(mine)) if greater(target, mine.y + TRACKING_DEADZONE) => {
                Some(Key::S)
            }
            (Some(target), Some(mine)) if greater(mine.y - TRACKING_DEADZONE, target) => {
                Some(Key::W)
            }
            _ => None,
        };
        if want != holding {
            if let Some(key) = holding {
                keyboard.record(InputEvent::KeyReleased(key));
            }
            if let Some(key) = want {
                keyboard.record(InputEvent::KeyPressed(key));
            }
            holding = want;
        }
        // One tap of Space, once, after the first match is won: a held key
        // would press once anyway, but tapping is what a person does and it is
        // the edge the restart is written against.
        let over = matches!(round(sim.world()), Some(Round::Over { .. }));
        let waited = match (over, over_since) {
            (true, None) => {
                over_since = Some(tick);
                0
            }
            (true, Some(since)) => tick - since,
            (false, _) => 0,
        };
        if over && waited >= LINGER && !tapping_space {
            keyboard.record(InputEvent::KeyPressed(Key::Space));
            tapping_space = true;
        } else if tapping_space {
            keyboard.record(InputEvent::KeyReleased(Key::Space));
            tapping_space = false;
        }

        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();

        // --- what the world did ------------------------------------------
        let Some(ball) = ball_at(sim.world()) else {
            fail(
                "the ball is gone",
                "Startup spawns exactly one and nothing despawns it",
            );
        };
        run.ball_track.push([ball.x.to_bits(), ball.y.to_bits()]);
        run.ball_extent = run.ball_extent.max(ball.abs());
        for side in [Side::Left, Side::Right] {
            match paddle_of(sim.world(), side) {
                Some(at) => run.paddle_extent = run.paddle_extent.max(at.y.abs()),
                None => fail(
                    "a paddle is gone",
                    &format!("Startup spawns one per side; {side:?} is missing"),
                ),
            }
        }
        {
            let board = sim.world().resource::<Scoreboard>();
            if run.won_at.is_none() && matches!(board.round, Round::Over { .. }) {
                run.won_at = Some(tick);
            } else if run.won_at.is_some()
                && run.restarted_at.is_none()
                && !matches!(board.round, Round::Over { .. })
            {
                run.restarted_at = Some(tick);
            }
            // Read the totals before the restart wipes them.
            if run.restarted_at.is_none() {
                run.left = board.left;
                run.right = board.right;
                run.left_hits = board.left_hits;
                run.right_hits = board.right_hits;
                run.top_speed = board.top_speed;
                run.longest_rally = board.longest_rally;
            }
        }

        // --- what was drawn ----------------------------------------------
        let frame = recorder.draw(&mut sim);
        run.frames += 1;
        if frame.quads().iter().any(|quad| quad.texture == font) {
            run.frames_with_text += 1;
        }
        if run.escaped.is_none() {
            run.escaped = escaped(frame, bounds).map(|quad| (tick, quad));
        }
    }

    // The score is drawn centred by `TextStyle::width_of`, and its middle
    // character is the colon — so a glyph covers this exact spot unless the
    // layout stopped centring or the camera stopped agreeing with it.
    let score_middle = Vec2::new(0.0, SCORE_TOP + SCORE_SIZE * 0.5);
    if let Some(last) = recorder.frames().last() {
        run.score_drawn = last
            .covering(score_middle)
            .into_iter()
            .any(|quad| quad.texture == font);
    }
    run.transcript = recorder.transcript();
    run.fixed_dt = sim.world().resource::<Time>().fixed_dt.as_f32();
    run
}

/// Where the ball is, or `None` if there is not exactly one.
fn ball_at(world: &World) -> Option<Vec2> {
    world
        .query::<(&Transform, With<Ball>)>()
        .map(|(_, transform, _)| transform.pos)
        .next()
}

/// How fast the ball is going, and which way.
fn ball_velocity(world: &World) -> Option<Vec2> {
    world
        .query::<(&Velocity, With<Ball>)>()
        .map(|(_, velocity, _)| velocity.0)
        .next()
}

/// Where the controller wants its paddle's centre to be, this tick.
///
/// Not "on the ball". A player who centres every return sends it back
/// perfectly flat, and two of those rally forever without either ever being
/// able to score — the same dead end `MIN_BOUNCE` exists to break. So this
/// aims: it puts the paddle *off* the ball by `AIM` of the paddle's reach, on
/// the side that sends the ball away from wherever the opponent is standing.
/// That is what a person does, and asserting the game can be won this way is
/// the difference between "the controls move a paddle" and "this is playable".
///
/// While the ball is going the other way there is nothing to aim at, so the
/// paddle comes back to the middle and waits.
fn aim_for(world: &World) -> Option<f32> {
    let ball = ball_at(world)?;
    let velocity = ball_velocity(world)?;
    let foe = paddle_at(world, Control::Machine)?;
    if !greater(0.0, velocity.x) {
        return Some(0.0);
    }
    // Positive offset means the ball landed below the paddle's centre, and a
    // positive offset sends it downwards. So: opponent above, aim down.
    let offset = if greater(0.0, foe.y) { AIM } else { -AIM };
    let reach = PADDLE_SIZE.y * 0.5 + BALL_RADIUS;
    Some((ball.y - offset * reach).clamp(-PADDLE_LIMIT, PADDLE_LIMIT))
}

/// Where the paddle somebody drives is.
fn paddle_at(world: &World, played_by: Control) -> Option<Vec2> {
    world
        .query::<(&Transform, &Paddle)>()
        .find(|(_, _, paddle)| paddle.played_by == played_by)
        .map(|(_, transform, _)| transform.pos)
}

/// Where one side's paddle is.
fn paddle_of(world: &World, side: Side) -> Option<Vec2> {
    world
        .query::<(&Transform, &Paddle)>()
        .find(|(_, _, paddle)| paddle.side == side)
        .map(|(_, transform, _)| transform.pos)
}

/// What the match is doing, or `None` before Startup has run.
///
/// Startup runs *inside* the first `tick()`, so on the way into tick 1 there
/// is no scoreboard yet — the controller has to ask rather than assume.
fn round(world: &World) -> Option<Round> {
    world.find_resource::<Scoreboard>().map(|board| board.round)
}
