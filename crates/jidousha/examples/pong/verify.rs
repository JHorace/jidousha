//! The check: a whole match played headless, and assertions on both halves.
//!
//! What differs from the window is only what a person would otherwise supply.
//! The systems, the config and the seed are the game's own, the left paddle is
//! driven by a controller that reads the world and decides, and the frames go
//! into a `FrameRecorder` instead of onto a screen.
//!
//! Run it: `cargo run -p jidousha --example pong -- --verify`

use jidousha::prelude::*;
use jidousha::testing::{FrameRecorder, InputEvent, InputSnapshot, PhysicalSize, SnapshotBuilder};

use crate::{Ball, MatchState, Paddle, Side, Stage, config, register};

/// The surface the recorder draws to.
///
/// The same size the game's camera carries, so the bounds the assertions read
/// out of `Camera::visible_bounds` are the bounds the frames were planned
/// against.
const VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// How long the run is allowed to take before it is judged a failure.
///
/// Ninety seconds at sixty ticks a second. A match this controller plays takes
/// well under a third of that; the rest is headroom, so a tuning change that
/// makes rallies longer reports a slow game rather than a broken one.
const TICK_BUDGET: u64 = 5400;

/// The steepest return the controller will play, as a fraction of the paddle's
/// half-height.
///
/// Short of 1.0, which is the very tip: a controller that aimed at the edge of
/// its own paddle would miss the ball whenever it was a hair out of position.
const AIM: f32 = 0.85;

/// How many returns either side of flat the controller weighs up.
const AIM_CANDIDATES: i8 = 6;

/// How far the paddle may sit from where the controller wants it before the
/// controller bothers to press a key.
///
/// A little over one tick of travel (24 units/s at 1/60 is 0.4), so the paddle
/// settles instead of drumming between two keys.
const DEADZONE: f32 = 0.45;

/// Fail with the engine's four-part message shape, and a non-zero exit.
fn fail(what: &str, specifics: &str) -> ! {
    eprintln!(
        "{}",
        message(
            what,
            specifics,
            "the game's tuning changed, or the engine did",
            "run `cargo run -p jidousha --example pong` and watch it, then compare with \
             the numbers above",
        )
    );
    std::process::exit(1);
}

/// Everything one headless match produced.
struct Played {
    /// The match as it stood when the run stopped.
    state: MatchState,
    /// How many ticks it took.
    ticks: u64,
    /// The extremes the player's paddle reached, as (highest, lowest).
    player_extremes: (f32, f32),
    /// The extremes the ball reached along X, as (leftmost, rightmost).
    ball_extremes: (f32, f32),
    /// The furthest the ball ever got from the centre line, vertically.
    ball_max_y: f32,
    /// Where the ball and both paddles were on the last tick.
    last_positions: (Vec2, Vec2, Vec2),
    /// The last frame drawn, kept as `draw` handed it back — `frames()` would
    /// borrow the recorder, and the staged screens below still need to draw.
    last_frame: Option<jidousha::testing::FrameRecord>,
}

/// Play one match with the controller below, optionally recording every frame.
fn play(recorder: Option<&mut FrameRecorder>) -> Played {
    let mut sim = headless(config(), register);
    // Startup runs inside the first `tick()`, and the systems read input rather
    // than asking whether it exists.
    sim.world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));

    let mut keyboard = SnapshotBuilder::new();
    let mut holding: Option<Key> = None;
    let mut recorder = recorder;
    let mut last_frame = None;

    let mut player_extremes = (0.0f32, 0.0f32);
    let mut ball_extremes = (0.0f32, 0.0f32);
    let mut ball_max_y = 0.0f32;
    let mut last_positions = (Vec2::ZERO, Vec2::ZERO, Vec2::ZERO);
    let mut ticks = 0;

    for tick in 1..=TICK_BUDGET {
        // The decision, taken from the world as it stands. On the way into tick
        // one there is nothing to look at yet — Startup has not run — so this
        // has to cope with an empty world rather than index into it.
        let want = choose_a_key(&sim);
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
        ticks = tick;

        let Some(scene) = read_the_scene(&sim) else {
            fail(
                "the world is missing a paddle or the ball",
                "Startup spawns two paddles and one ball, and nothing despawns them",
            );
        };
        player_extremes.0 = player_extremes.0.min(scene.left);
        player_extremes.1 = player_extremes.1.max(scene.left);
        ball_extremes.0 = ball_extremes.0.min(scene.ball.x);
        ball_extremes.1 = ball_extremes.1.max(scene.ball.x);
        ball_max_y = ball_max_y.max(scene.ball.y.abs());
        last_positions = (
            scene.ball,
            Vec2::new(-crate::PADDLE_X, scene.left),
            Vec2::new(crate::PADDLE_X, scene.right),
        );

        if let Some(recorder) = recorder.as_deref_mut() {
            let view = camera_of(&sim).visible_bounds();
            let frame = recorder.draw(&mut sim);
            assert_on_screen(frame.quads(), view, tick);
            last_frame = Some(frame);
        }

        if matches!(
            sim.world().resource::<MatchState>().stage,
            Stage::Over { .. }
        ) {
            break;
        }
    }

    Played {
        state: *sim.world().resource::<MatchState>(),
        ticks,
        player_extremes,
        ball_extremes,
        ball_max_y,
        last_positions,
        last_frame,
    }
}

/// What the controller and the assertions read off the world each tick.
struct Scene {
    /// The player's paddle Y.
    left: f32,
    /// The opponent's paddle Y.
    right: f32,
    /// Where the ball is.
    ball: Vec2,
    /// Where the ball is going.
    ball_velocity: Vec2,
}

/// The two paddles and the ball, or `None` before Startup has run.
fn read_the_scene(sim: &HeadlessSim) -> Option<Scene> {
    let world = sim.world();
    let (mut left, mut right) = (None, None);
    for (_, transform, paddle) in world.query::<(&Transform, &Paddle)>() {
        match paddle.side {
            Side::Left => left = Some(transform.pos.y),
            Side::Right => right = Some(transform.pos.y),
        }
    }
    let (_, ball, ball_velocity) = world
        .query::<(&Transform, &Ball)>()
        .map(|(entity, transform, ball)| (entity, transform.pos, ball.velocity))
        .next()?;
    Some(Scene {
        left: left?,
        right: right?,
        ball,
        ball_velocity,
    })
}

/// The game's camera, with the recorder's viewport, so bounds and frames agree.
fn camera_of(sim: &HeadlessSim) -> Camera {
    match sim.world().find_resource::<Camera>() {
        Some(camera) => Camera {
            viewport: VIEWPORT,
            ..*camera
        },
        None => Camera {
            viewport: VIEWPORT,
            ..Camera::default()
        },
    }
}

/// Which key the controller wants held this tick, if any.
///
/// It plays to win: it works out where the ball will cross its own face, and
/// then stands so that the ball meets the *half* of the paddle that sends it
/// away from wherever the opponent is standing. A controller that centred every
/// return would groove a rally neither side could lose, and the run would say
/// the game is unplayable when the fault was in the driver.
fn choose_a_key(sim: &HeadlessSim) -> Option<Key> {
    let scene = read_the_scene(sim)?;
    let reach = crate::PADDLE_SIZE.y * 0.5 + crate::BALL_RADIUS;

    let target = if scene.ball_velocity.x < 0.0 {
        let arriving = intercept(scene.ball, scene.ball_velocity, PLAYER_FACE);
        // Stand so the ball meets the part of the paddle that sends it where
        // the opponent is not. `offset` is the same number the game's own
        // bounce uses: the fraction of the paddle's half-height the ball is
        // struck away from centre.
        arriving - best_offset(&scene, arriving) * reach
    } else {
        // Nothing to answer: stand in the middle, where every reply is short.
        0.0
    };

    let gap = target.clamp(-crate::PADDLE_LIMIT, crate::PADDLE_LIMIT) - scene.left;
    if gap.abs() < DEADZONE {
        None
    } else if gap > 0.0 {
        Some(Key::S)
    } else {
        Some(Key::W)
    }
}

/// Where the ball's edge meets the player's paddle.
const PLAYER_FACE: f32 = -crate::PADDLE_X + crate::PADDLE_SIZE.x * 0.5 + crate::BALL_RADIUS;

/// Where the ball's edge meets the opponent's paddle.
const OPPONENT_FACE: f32 = crate::PADDLE_X - crate::PADDLE_SIZE.x * 0.5 - crate::BALL_RADIUS;

/// Which part of the paddle to hit the ball with.
///
/// The controller plays the shot a person would: it tries the returns the
/// paddle can actually produce, works out where each of them would reach the
/// far side, and takes the one that lands furthest from the middle — which is
/// where the opponent drifts back to between shots. A controller that returned
/// every ball flat down the middle would groove a rally neither side could
/// lose, and then report a game that never ends.
///
/// This has to re-derive the game's own bounce to do it. There is nothing to
/// ask: `advance` is the only thing that knows what a paddle does to a ball,
/// and it is a private function of the simulation.
fn best_offset(scene: &Scene, arriving: f32) -> f32 {
    let speed = (scene.ball_velocity.length() * crate::SPEED_GAIN).min(crate::MAX_BALL_SPEED);
    let mut best = (0.0, -1.0);
    for step in -AIM_CANDIDATES..=AIM_CANDIDATES {
        let offset = f32::from(step) / f32::from(AIM_CANDIDATES) * AIM;
        let (sine, cosine) = sin_cos(Radians(offset * crate::MAX_BOUNCE.0));
        let sent = Vec2::new(speed * cosine, speed * sine);
        let lands = intercept(Vec2::new(PLAYER_FACE, arriving), sent, OPPONENT_FACE);
        if lands.abs() > best.1 {
            best = (offset, lands.abs());
        }
    }
    best.0
}

/// Where a ball travelling in a straight line crosses `plane`, walls included.
fn intercept(pos: Vec2, velocity: Vec2, plane: f32) -> f32 {
    if velocity.x.abs() < f32::EPSILON {
        return pos.y;
    }
    let when = (plane - pos.x) / velocity.x;
    if when < 0.0 {
        return pos.y;
    }
    fold(
        pos.y + velocity.y * when,
        crate::FIELD_TOP + crate::BALL_RADIUS,
        crate::FIELD_BOTTOM - crate::BALL_RADIUS,
    )
}

/// `value` reflected back and forth between `lo` and `hi` — a bouncing ball's Y
/// without simulating the bounces.
fn fold(value: f32, lo: f32, hi: f32) -> f32 {
    let span = hi - lo;
    if span <= 0.0 {
        return lo;
    }
    let period = span * 2.0;
    let mut offset = (value - lo) % period;
    if offset < 0.0 {
        offset += period;
    }
    if offset > span {
        offset = period - offset;
    }
    lo + offset
}

/// Nothing was drawn outside what the camera can see.
///
/// The highest-value check a game of shapes and text has, and the one that
/// catches a banner one character too long — `TextStyle::width_of` is exact and
/// completely silent, so a line that overruns does so without a word.
fn assert_on_screen(quads: Vec<jidousha::testing::DrawnQuad>, view: Rect, tick: u64) {
    for quad in quads {
        let drawn = quad.bounds();
        if !view.contains_rect(drawn) {
            fail(
                "something was drawn off screen",
                &format!(
                    "on tick {tick} a quad covering ({:.2}, {:.2})..({:.2}, {:.2}) was drawn \
                     against a camera showing ({:.2}, {:.2})..({:.2}, {:.2}) — text centred \
                     by TextStyle::width_of is the usual culprit",
                    drawn.min.x,
                    drawn.min.y,
                    drawn.max.x,
                    drawn.max.y,
                    view.min.x,
                    view.min.y,
                    view.max.x,
                    view.max.y,
                ),
            );
        }
    }
}

/// Within a thousandth, and false when either side is NaN.
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.001
}

/// Whether any quad covering `at` is the size of `size`.
fn shaped_like(frame: &jidousha::testing::FrameRecord, at: Vec2, size: Vec2) -> bool {
    frame.covering(at).into_iter().any(|quad| {
        let drawn = quad.bounds().size();
        near(drawn.x, size.x) && near(drawn.y, size.y)
    })
}

/// How far the quads covering `at` reach, counting only those small enough to
/// be part of a disc of `size` centred there.
///
/// `ctx.circle` submits a fan of wedge quads rather than one square, so a disc
/// is checked by its extent rather than by any single quad's size.
fn disc_drawn(frame: &jidousha::testing::FrameRecord, at: Vec2, size: Vec2) -> Option<Vec2> {
    let box_of_it = Rect::from_center_size(at, size);
    let mut union: Option<Rect> = None;
    for quad in frame.covering(at) {
        let drawn = quad.bounds();
        let inside = drawn.min.x >= box_of_it.min.x - 0.001
            && drawn.min.y >= box_of_it.min.y - 0.001
            && drawn.max.x <= box_of_it.max.x + 0.001
            && drawn.max.y <= box_of_it.max.y + 0.001;
        if !inside {
            continue;
        }
        union = Some(match union {
            None => drawn,
            Some(so_far) => Rect {
                min: so_far.min.min(drawn.min),
                max: so_far.max.max(drawn.max),
            },
        });
    }
    union.map(Rect::size)
}

pub(crate) fn run() {
    let mut recorder = FrameRecorder::new(VIEWPORT);
    // Read before the loop: `draw` borrows the recorder for as long as the
    // frame it hands back is alive.
    let font = recorder.font_texture();
    let played = play(Some(&mut recorder));

    // --- what the world did ------------------------------------------
    let Stage::Over { winner } = played.state.stage else {
        fail(
            "nobody won",
            &format!(
                "after {} ticks the score is {}-{}, the longest rally was {} touches and the \
                 ball's top speed was {:.1} units/s (first to {})",
                played.ticks,
                played.state.left,
                played.state.right,
                played.state.longest_rally,
                played.state.top_speed,
                crate::WIN_SCORE,
            ),
        );
    };
    if played.state.points(winner) != crate::WIN_SCORE {
        fail(
            "the match ended on the wrong score",
            &format!(
                "{} won with {} points, and the match is first to {}",
                winner.name(),
                played.state.points(winner),
                crate::WIN_SCORE,
            ),
        );
    }
    if played.state.longest_rally < 2 {
        fail(
            "no rally ever came back",
            &format!(
                "the longest rally was {} paddle touches over {} ticks, so points are being \
                 won on the serve rather than played out",
                played.state.longest_rally, played.ticks,
            ),
        );
    }
    if played.state.top_speed <= crate::SERVE_SPEED {
        fail(
            "the ball never sped up",
            &format!(
                "its top speed was {:.2} units/s and a serve leaves at {:.2}; every paddle \
                 touch is supposed to multiply it by {}",
                played.state.top_speed,
                crate::SERVE_SPEED,
                crate::SPEED_GAIN,
            ),
        );
    }

    // The paddle is clamped to the field, and the controller pushed it into
    // both ends of that clamp rather than merely not violating it.
    let (highest, lowest) = played.player_extremes;
    if highest < -crate::PADDLE_LIMIT || lowest > crate::PADDLE_LIMIT {
        fail(
            "the player's paddle left the field",
            &format!(
                "it reached {highest:.3} and {lowest:.3}, against a clamp of +/-{:.1}",
                crate::PADDLE_LIMIT,
            ),
        );
    }
    // Not "did it touch both ends" — a controller that plays well is not
    // obliged to — but "did it have to work". A paddle that lived in the middle
    // would mean the ball never went anywhere, and every assertion below would
    // be passing on a game nothing happened in.
    let span = lowest - highest;
    let field = crate::PADDLE_LIMIT * 2.0;
    if span < field * 0.6 {
        fail(
            "the player's paddle never used the field it has",
            &format!(
                "it only ever moved between {highest:.3} and {lowest:.3} — {span:.2} units \
                 of the {field:.2} it can travel — so the ball is not being made to go \
                 anywhere",
            ),
        );
    }

    // The ball stayed on the field vertically, and left it horizontally only
    // far enough to score — the point is registered on the same tick, so it is
    // never drawn beyond the sideline.
    let ceiling = crate::FIELD_BOTTOM - crate::BALL_RADIUS;
    if played.ball_max_y > ceiling + 0.001 {
        fail(
            "the ball went through a wall",
            &format!(
                "it reached {:.3} from the centre line, and the wall is at {ceiling:.3}",
                played.ball_max_y,
            ),
        );
    }
    let (leftmost, rightmost) = played.ball_extremes;
    let sideline = crate::FIELD_EDGE + crate::MAX_BALL_SPEED / 60.0;
    if leftmost < -sideline || rightmost > sideline {
        fail(
            "the ball was still on the field a tick after it should have scored",
            &format!(
                "it reached {leftmost:.3} and {rightmost:.3}, against a sideline at \
                 +/-{:.1} and at most {sideline:.3} of overshoot in one tick",
                crate::FIELD_EDGE,
            ),
        );
    }

    // --- what was drawn ----------------------------------------------
    let frames = recorder.frames().len();
    if frames != played.ticks as usize {
        fail(
            "one frame per tick was expected",
            &format!("{frames} frames for {} ticks", played.ticks),
        );
    }
    let Some(last) = played.last_frame.as_ref() else {
        fail("no frame was recorded", "the loop draws every tick");
    };
    let glyphs = last
        .quads()
        .iter()
        .filter(|quad| quad.texture == font)
        .count();
    if glyphs == 0 {
        fail(
            "nothing on screen sampled the font atlas",
            "the score, the hint line and the winner's banner are all text, so a frame \
             without a glyph has lost all three",
        );
    }

    // Drawing agrees with simulation: the positions come back out of the world
    // rather than being written down here, so this asks whether what was drawn
    // is where the game says it is.
    let (ball, left_paddle, right_paddle) = played.last_positions;
    for (what, at) in [("left", left_paddle), ("right", right_paddle)] {
        if !shaped_like(last, at, crate::PADDLE_SIZE) {
            fail(
                "no paddle-shaped quad was drawn where a paddle is",
                &format!(
                    "the world puts the {what} paddle at ({:.2}, {:.2}), {} by {}",
                    at.x,
                    at.y,
                    crate::PADDLE_SIZE.x,
                    crate::PADDLE_SIZE.y,
                ),
            );
        }
    }
    // A circle is not one quad. `ctx.circle` tessellates into sixteen wedges,
    // so the paddle's "is a quad of this size drawn here" question has no
    // answer for the ball — what is drawn there is sixteen slivers whose union
    // is the ball. Union them and ask about that instead.
    let ball_size = Vec2::splat(crate::BALL_RADIUS * 2.0);
    match disc_drawn(last, ball, ball_size) {
        Some(size) if near(size.x, ball_size.x) && near(size.y, ball_size.y) => {}
        found => fail(
            "no ball-sized disc was drawn where the ball is",
            &format!(
                "the world puts it at ({:.2}, {:.2}) with radius {}, so the quads covering \
                 that point should span {:.2}x{:.2}; they span {}",
                ball.x,
                ball.y,
                crate::BALL_RADIUS,
                ball_size.x,
                ball_size.y,
                match found {
                    Some(size) => format!("{:.3}x{:.3}", size.x, size.y),
                    None => "nothing at all".to_owned(),
                },
            ),
        ),
    }

    // --- the screens this run never reached ---------------------------
    //
    // The bounds check above only judges frames that were drawn, and a
    // controller good enough to win is a controller that never sees the losing
    // banner. Three lines per screen builds the ones the match skipped.
    let mut staged = headless(config(), register);
    staged
        .world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));
    staged.tick(); // Startup, so the world exists.
    let view = camera_of(&staged).visible_bounds();
    // The same recorder the match used. `draw` hands back an owned frame, so
    // reading the match's last frame and drawing screens it never reached are
    // two things one function can do — this used to need a second recorder.
    let mut staged_screens = 0;
    for stage in [
        Stage::Over { winner: Side::Left },
        Stage::Over {
            winner: Side::Right,
        },
        Stage::Serving {
            ticks_left: crate::SERVE_TICKS,
            toward: Side::Left,
        },
        Stage::Serving {
            ticks_left: 1,
            toward: Side::Right,
        },
        Stage::Rally,
    ] {
        let mut state = played.state;
        state.stage = stage;
        // The widest score this game can draw, so the layout is judged at its
        // longest rather than at whatever the match happened to end on.
        state.left = crate::WIN_SCORE;
        state.right = crate::WIN_SCORE;
        staged.world_mut().insert_resource(state);
        let frame = recorder.draw(&mut staged);
        assert_on_screen(frame.quads(), view, 0);
        staged_screens += 1;
    }

    // --- the control only the last screen has -------------------------
    //
    // Enter starts the next match, and it is live on exactly one screen — so
    // the match above, which stops the moment that screen appears, is the one
    // run that can never press it.
    let mut restart = SnapshotBuilder::new();
    staged.world_mut().insert_resource(MatchState {
        stage: Stage::Over { winner: Side::Left },
        left: crate::WIN_SCORE,
        right: 2,
        rally: 0,
        longest_rally: 9,
        top_speed: 25.0,
    });
    restart.record(InputEvent::KeyPressed(Key::Enter));
    staged
        .world_mut()
        .insert_resource(Input::new(restart.first_tick_snapshot()));
    staged.tick();
    let restarted = *staged.world().resource::<MatchState>();
    let fresh = restarted.left == 0
        && restarted.right == 0
        && matches!(restarted.stage, Stage::Serving { .. });
    if !fresh {
        fail(
            "Enter did not start a new match",
            &format!(
                "from 5-2 and a winner's banner, one press of Enter left the game at {}-{} \
                 on {:?}",
                restarted.left, restarted.right, restarted.stage,
            ),
        );
    }

    // --- determinism --------------------------------------------------
    //
    // The controller is closed-loop, so a second run only lands in the same
    // place if every tick of the game did. Bit for bit, not nearly.
    let again = play(None);
    let (ball_again, left_again, right_again) = again.last_positions;
    let same = [
        ball.x.to_bits() == ball_again.x.to_bits(),
        ball.y.to_bits() == ball_again.y.to_bits(),
        left_paddle.y.to_bits() == left_again.y.to_bits(),
        right_paddle.y.to_bits() == right_again.y.to_bits(),
        played.ticks == again.ticks,
        played.state.left == again.state.left,
        played.state.right == again.state.right,
    ];
    if same.contains(&false) {
        fail(
            "the same game played twice did two different things",
            &format!(
                "first run: {}-{} in {} ticks, ball at ({:.4}, {:.4}); second run: {}-{} in \
                 {} ticks, ball at ({:.4}, {:.4})",
                played.state.left,
                played.state.right,
                played.ticks,
                ball.x,
                ball.y,
                again.state.left,
                again.state.right,
                again.ticks,
                ball_again.x,
                ball_again.y,
            ),
        );
    }

    println!("verified pong over {} ticks", played.ticks);
    println!(
        "  match:  {} beat {} {}-{} in {:.1}s of play",
        winner.name(),
        match winner {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
        .name(),
        played.state.left,
        played.state.right,
        played.ticks as f32 / 60.0,
    );
    println!(
        "  rally:  longest {} touches, ball topped out at {:.1} units/s",
        played.state.longest_rally, played.state.top_speed,
    );
    println!(
        "  paddle: reached {:.2} and {:.2}, clamped to +/-{:.1}",
        played.player_extremes.0,
        played.player_extremes.1,
        crate::PADDLE_LIMIT,
    );
    println!(
        "  drawn:  {frames} frames, all on screen; last frame has {} quads and {glyphs} glyphs",
        last.quad_count(),
    );
    println!("  staged: {staged_screens} screens the match never reached, all on screen");
    println!("  enter:  starts a new match from the winner's banner");
    println!("  replay: identical to the bit");
    print!("{}", last.transcript());
}
