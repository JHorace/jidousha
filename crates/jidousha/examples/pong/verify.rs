//! `--verify`: script the input, run the game headless, assert on what the
//! world did and on what was drawn.
//!
//! It registers the *same* systems and the same config the window does. What
//! differs is only what a person would otherwise supply: the keys come from a
//! script, so the run is the same on every machine and on every day.
//!
//! Nothing here opens a window or touches a GPU. `FrameRecorder` runs the Draw
//! phase and keeps every frame as structured data, which is what makes "was the
//! ball drawn where the world put it" a question with an answer.

use crate::{
    BALL_SIZE, BALL_START_SPEED, Ball, Control, FIELD, PADDLE_SIZE, Paddle, Round, SERVE_TICKS,
    Score, Side, Volley, WINNING_SCORE, config, register,
};
use jidousha::prelude::*;
use jidousha::testing::{FrameRecorder, InputScript, PhysicalSize};
use std::cmp::Ordering;

/// How long the scripted session runs: fifteen seconds at 60 ticks a second.
///
/// Long enough for several points, which is what makes serving, scoring and
/// the pause between them things this run actually sees.
const TICKS: u64 = 900;

/// The surface the headless frames are drawn to, matching the window's.
const VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// How far the player's paddle may travel from the centre, from `main.rs`.
const PLAYER_LIMIT: f32 = FIELD.y - PADDLE_SIZE.y * 0.5;

/// How far the ball's centre may be from the middle, vertically, once a tick
/// has finished.
const BALL_Y_LIMIT: f32 = FIELD.y - BALL_SIZE * 0.5;

/// Fail with the engine's message shape, and a non-zero exit.
fn fail(what: &str, specifics: &str) -> ! {
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

/// `a > b`, and false when either is NaN.
///
/// Spelled out rather than written as the negation of a `<=`, because the
/// negation of a float comparison silently means something else: a NaN that
/// crept into the ball's position would satisfy every plain `<=` here and pass
/// this whole verification.
fn greater(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(Ordering::Greater))
}

/// Within a thousandth, and false when either is NaN.
fn near(a: f32, b: f32) -> bool {
    greater(0.001, (a - b).abs())
}

/// Where in `track` the largest value first appears.
fn peak_at(track: &[f32], pick: fn(f32, f32) -> bool) -> usize {
    let mut best = 0;
    for (index, value) in track.iter().enumerate() {
        if pick(*value, track[best]) {
            best = index;
        }
    }
    best
}

/// Hold each key long enough to jam the paddle against its clamp, and use both
/// control schemes, so a run that broke one of them fails here.
fn script() -> InputScript {
    InputScript::new()
        .hold(Key::S, 20..140)
        .hold(Key::W, 150..320)
        .hold(Key::ArrowDown, 400..520)
        .hold(Key::ArrowUp, 560..700)
}

/// What one scripted session did, tick by tick.
struct Run {
    /// The player's paddle Y after each tick.
    player_track: Vec<f32>,
    /// The computer's paddle Y after each tick.
    cpu_track: Vec<f32>,
    /// The ball's position and velocity after each tick.
    ball_track: Vec<(Vec2, Vec2)>,
    /// What the game was doing after each tick.
    round_track: Vec<Round>,
    /// The longest rally, in paddle touches.
    volley_max: u32,
    /// The score at the end.
    score: Score,
    /// How many frames were drawn.
    frames: usize,
    /// What the camera could see, as (top-left, bottom-right).
    visible: (Vec2, Vec2),
}

/// How long the second, closed-loop session may run before it is judged to
/// have stalled: a hundred seconds, which is many times a full game.
const TRACKING_LIMIT: u64 = 6000;

/// What one tick of a player who is actually watching the ball looks like.
///
/// `InputScript` is built in advance and read per tick, which is exactly right
/// for a fixed script and no use at all for a controller that has to see the
/// game before it decides. A one-tick script, rebuilt every tick, is the only
/// way to close that loop with what `jidousha::testing` exposes.
fn tracking_input(tick: u64, paddle_y: f32, cpu_y: f32, ball: (Vec2, Vec2)) -> Input {
    let (pos, vel) = ball;
    // Chase only what is coming this way, and go back to the middle otherwise.
    //
    // The aim matters more than it looks. A player who puts the ball in the
    // *middle* of the paddle returns it dead flat, straight back to a computer
    // that then never has to move — a perfectly-centring player produces an
    // unloseable rally for both sides and a game that never ends. That is what
    // this check found on its first run. A person does what this does instead:
    // meets the ball with the half of the paddle that sends it away from the
    // other player.
    let target = if vel.x < 0.0 {
        let away = if cpu_y < pos.y { -1.0 } else { 1.0 };
        pos.y + away * PADDLE_SIZE.y * 0.3
    } else {
        0.0
    };
    let delta = target - paddle_y;
    let script = if delta.abs() < 0.35 {
        InputScript::new()
    } else if greater(delta, 0.0) {
        InputScript::new().hold(Key::S, tick..tick + 1)
    } else {
        InputScript::new().hold(Key::W, tick..tick + 1)
    };
    Input::new(script.snapshot_at(tick))
}

/// The ball's position and velocity, if there is one.
fn ball_state(world: &World) -> Option<(Vec2, Vec2)> {
    world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| (transform.pos, ball.vel))
        .next()
}

/// What a session played by someone who can see came to.
struct Tracked {
    /// The longest rally, in paddle touches.
    volley_max: u32,
    /// The fastest the ball ever went, in world units per second.
    fastest: f32,
    /// Who won, and on which tick, if anyone did.
    winner: Option<(Side, u64)>,
    /// The score when the run stopped.
    score: Score,
}

/// Play the game with a player who tracks the ball, until somebody wins.
///
/// The scripted session above proves the controls and the drawing; it cannot
/// prove the *game*, because a blind script never returns a ball and every
/// point looks the same. This one asks the question that matters: does a rally
/// happen, does the ball get faster while it does, and can the player win?
///
/// No frames are recorded — what is drawn is the other session's question.
fn play_tracking() -> Tracked {
    let mut sim = headless(config(), register);
    let mut tracked = Tracked {
        volley_max: 0,
        fastest: 0.0,
        winner: None,
        score: Score::default(),
    };

    for tick in 1..=TRACKING_LIMIT {
        // The world is still empty on the first tick: Startup runs *inside* it.
        let input = match (
            paddle_y(sim.world(), Control::Keys),
            paddle_y(sim.world(), Control::Cpu),
            ball_state(sim.world()),
        ) {
            (Some(y), Some(cpu), Some(ball)) => tracking_input(tick, y, cpu, ball),
            _ => Input::new(InputScript::new().snapshot_at(tick)),
        };
        sim.world_mut().insert_resource(input);
        sim.tick();

        let world = sim.world();
        tracked.volley_max = tracked.volley_max.max(world.resource::<Volley>().0);
        if let Some((_, vel)) = ball_state(world) {
            tracked.fastest = tracked.fastest.max(vel.length());
        }
        tracked.score = *world.resource::<Score>();
        if let Round::Over { winner } = *world.resource::<Round>() {
            tracked.winner = Some((winner, tick));
            break;
        }
    }
    tracked
}

/// Where a paddle under `control` is right now.
fn paddle_y(world: &World, control: Control) -> Option<f32> {
    world
        .query::<(&Transform, &Paddle)>()
        .find(|(_, _, paddle)| paddle.control == control)
        .map(|(_, transform, _)| transform.pos.y)
}

/// Play the scripted session, drawing a frame per tick into `recorder`.
fn play(recorder: &mut FrameRecorder) -> Run {
    let mut sim = headless(config(), register);
    let script = script();
    let mut run = Run {
        player_track: Vec::new(),
        cpu_track: Vec::new(),
        ball_track: Vec::new(),
        round_track: Vec::new(),
        volley_max: 0,
        score: Score::default(),
        frames: 0,
        visible: (Vec2::ZERO, Vec2::ZERO),
    };

    for tick in 1..=TICKS {
        sim.world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        sim.tick();

        let world = sim.world();
        match (
            paddle_y(world, Control::Keys),
            paddle_y(world, Control::Cpu),
        ) {
            (Some(player), Some(cpu)) => {
                run.player_track.push(player);
                run.cpu_track.push(cpu);
            }
            _ => fail(
                "a paddle is missing",
                "Startup spawns exactly one of each, and nothing despawns them",
            ),
        }
        match world
            .query::<(&Transform, &Ball)>()
            .map(|(_, transform, ball)| (transform.pos, ball.vel))
            .next()
        {
            Some(state) => run.ball_track.push(state),
            None => fail("the ball is gone", "Startup spawns exactly one"),
        }
        run.round_track.push(*world.resource::<Round>());
        run.visible = world.resource::<Camera>().visible_bounds();
        run.volley_max = run.volley_max.max(world.resource::<Volley>().0);
        run.score = *world.resource::<Score>();

        recorder.draw(&mut sim);
        run.frames += 1;
    }
    run
}

pub fn run() {
    let mut recorder = FrameRecorder::new(VIEWPORT);
    // Read before the loop: `draw` borrows the recorder for as long as the
    // frame it hands back is alive.
    let font = recorder.font_texture();
    let session = play(&mut recorder);

    // --- what the world did ------------------------------------------
    // Y is down, so the bottom of the screen is the larger number. The script
    // holds S first and W second, and both holds run past the clamp, so both
    // ends are reached either way round — only the *order* tells a swapped
    // pair of keys apart.
    let start = session.player_track[0];
    let bottom_at = peak_at(&session.player_track, greater);
    let top_at = peak_at(&session.player_track, |a, b| greater(b, a));
    let (bottom, top) = (
        session.player_track[bottom_at],
        session.player_track[top_at],
    );
    if !near(bottom, PLAYER_LIMIT) || !near(top, -PLAYER_LIMIT) {
        fail(
            "the player's paddle did not come to rest against both ends of its field",
            &format!(
                "it reached {bottom:.3} and {top:.3}; the clamp is +/-{PLAYER_LIMIT:.3}, and \
                 the script holds each key long enough to run past it"
            ),
        );
    }
    if bottom_at >= top_at {
        fail(
            "S and W move the player's paddle the wrong way round",
            &format!(
                "the script holds S first, but the paddle was at the top on tick {top} \
                 before it was at the bottom on tick {bottom}",
                top = top_at + 1,
                bottom = bottom_at + 1,
            ),
        );
    }
    if !greater(bottom, start) || !greater(start, top) {
        fail(
            "the player's paddle did not start between the two ends it reached",
            &format!("it started at {start:.3}, which is not between {top:.3} and {bottom:.3}"),
        );
    }
    // The arrow keys are the same control, and the script uses them after tick
    // 400. A paddle that stopped moving there would still pass every assertion
    // above, because W and S have already been held by then.
    let arrows = &session.player_track[399..];
    let arrow_span = arrows.iter().copied().fold(f32::MIN, f32::max)
        - arrows.iter().copied().fold(f32::MAX, f32::min);
    if !greater(arrow_span, PLAYER_LIMIT) {
        fail(
            "the arrow keys do not move the player's paddle",
            &format!(
                "over the ticks where only Up and Down are held it moved {arrow_span:.3} \
                 units, and holding either one for two seconds crosses the whole field"
            ),
        );
    }

    // The computer's paddle plays: it moved, and it stayed on its own side of
    // the clamp.
    let cpu_span = session.cpu_track.iter().copied().fold(f32::MIN, f32::max)
        - session.cpu_track.iter().copied().fold(f32::MAX, f32::min);
    if !greater(cpu_span, 1.0) {
        fail(
            "the computer's paddle never moved",
            &format!("it covered {cpu_span:.3} units over {TICKS} ticks"),
        );
    }
    for (index, y) in session.cpu_track.iter().enumerate() {
        if greater(y.abs(), PLAYER_LIMIT + 0.001) {
            fail(
                "the computer's paddle left the field",
                &format!("it was at {y:.3} on tick {}", index + 1),
            );
        }
    }

    // The ball stayed on the table. Vertically it is walled in; horizontally
    // it is put back to the centre on the same tick it goes past an end, so
    // after any completed tick both are true.
    for (index, (pos, _)) in session.ball_track.iter().enumerate() {
        if greater(pos.y.abs(), BALL_Y_LIMIT + 0.001) {
            fail(
                "the ball went through the top or bottom wall",
                &format!(
                    "it was at y {:.3} on tick {}, and the wall is at {BALL_Y_LIMIT:.3}",
                    pos.y,
                    index + 1
                ),
            );
        }
        if greater(pos.x.abs(), FIELD.x + 0.001) {
            fail(
                "the ball stayed past the end of the field",
                &format!(
                    "it was at x {:.3} on tick {}; a point is awarded and the ball reset on \
                     the tick it passes {:.1}",
                    pos.x,
                    index + 1,
                    FIELD.x
                ),
            );
        }
    }

    // Paddles return the ball, and it speeds up when they do.
    if session.volley_max == 0 {
        fail(
            "no paddle ever touched the ball",
            "the serve is aimed at the computer, which chases it, so at least one return \
             happens in the first seconds",
        );
    }
    let fastest = session
        .ball_track
        .iter()
        .map(|(_, vel)| vel.length())
        .fold(0.0f32, f32::max);
    if !greater(fastest, BALL_START_SPEED + 0.001) {
        fail(
            "the ball never sped up",
            &format!(
                "the fastest it went was {fastest:.3} and a serve leaves at \
                 {BALL_START_SPEED:.1}; every paddle it touches should add to that"
            ),
        );
    }

    // Points are scored, and each one is followed by a pause with the ball
    // parked at the centre.
    let points = session.score.left + session.score.right;
    if points == 0 {
        fail(
            "nobody scored in fifteen seconds",
            "a scripted paddle that is not tracking the ball concedes; a game where it
             never does has stopped awarding points",
        );
    }
    let first_serve_after_a_point = session
        .round_track
        .windows(2)
        .position(|pair| {
            !matches!(pair[0], Round::Serving { .. }) && matches!(pair[1], Round::Serving { .. })
        })
        .map(|index| index + 1);
    let Some(serve_at) = first_serve_after_a_point else {
        fail(
            "no serve followed a point",
            &format!("{points} points were scored, and each one should set up the next serve"),
        );
    };
    let (parked, parked_vel) = session.ball_track[serve_at];
    if !near(parked.x, 0.0) || !near(parked.y, 0.0) || !near(parked_vel.length(), 0.0) {
        fail(
            "the ball was not parked at the centre for the serve",
            &format!(
                "on tick {} it was at ({:.3}, {:.3}) moving at {:.3}",
                serve_at + 1,
                parked.x,
                parked.y,
                parked_vel.length()
            ),
        );
    }
    // And the pause actually lasts. The ball leaves on the tick the countdown
    // reaches zero, which is `SERVE_TICKS` ticks after the point.
    let held = session.round_track[serve_at..]
        .iter()
        .take_while(|round| matches!(round, Round::Serving { .. }))
        .count();
    if held != SERVE_TICKS as usize {
        fail(
            "the serve pause is not the length the game says it is",
            &format!("it lasted {held} ticks; SERVE_TICKS is {SERVE_TICKS}"),
        );
    }

    // --- is it a game ------------------------------------------------
    // Everything above is about controls and bookkeeping, all of which a
    // thoroughly unplayable Pong would also pass. This is the part that asks
    // whether there is a game here.
    let tracked = play_tracking();
    if tracked.volley_max < 6 {
        fail(
            "the ball is never rallied",
            &format!(
                "a player tracking the ball got the longest exchange to {} paddle touches; \
                 a Pong where a competent player cannot sustain a rally is one where the \
                 bounce angle or the paddle speed is wrong",
                tracked.volley_max
            ),
        );
    }
    if !greater(tracked.fastest, 30.0) {
        fail(
            "the ball does not get faster over a rally",
            &format!(
                "the fastest it went in a whole game was {:.2} units/s, from a serve at \
                 {BALL_START_SPEED:.1}; the ramp is what stops a long rally being dull",
                tracked.fastest
            ),
        );
    }
    let Some((winner, won_at)) = tracked.winner else {
        fail(
            "no one won inside a hundred seconds",
            &format!(
                "the score reached {} - {} of {WINNING_SCORE} in {TRACKING_LIMIT} ticks, with \
                 the longest rally running to {} paddle touches and the ball reaching \
                 {:.2} units/s; either the game cannot end, or a rally has become \
                 unloseable for both sides",
                tracked.score.left, tracked.score.right, tracked.volley_max, tracked.fastest
            ),
        );
    };
    if winner != Side::Left {
        fail(
            "a player who tracks the ball still loses",
            &format!(
                "the computer won {} - {}; it is meant to be beatable by someone paying \
                 attention, which is the whole difficulty setting",
                tracked.score.right, tracked.score.left
            ),
        );
    }

    // --- the same run twice ------------------------------------------
    // Same seed, same script, same systems: the same game, down to the bits.
    // This is the claim everything above rests on — an assertion about tick 900
    // means nothing if tick 900 is not always the same tick.
    let mut replay_recorder = FrameRecorder::new(VIEWPORT);
    let replay = play(&mut replay_recorder);
    let diverged = session
        .ball_track
        .iter()
        .zip(replay.ball_track.iter())
        .position(|(first, second)| {
            first.0.x.to_bits() != second.0.x.to_bits()
                || first.0.y.to_bits() != second.0.y.to_bits()
                || first.1.x.to_bits() != second.1.x.to_bits()
                || first.1.y.to_bits() != second.1.y.to_bits()
        });
    if let Some(tick) = diverged {
        fail(
            "the same session played twice did not produce the same game",
            &format!(
                "the ball first differs on tick {}: ({:.6}, {:.6}) against ({:.6}, {:.6})",
                tick + 1,
                session.ball_track[tick].0.x,
                session.ball_track[tick].0.y,
                replay.ball_track[tick].0.x,
                replay.ball_track[tick].0.y,
            ),
        );
    }
    if replay.score != session.score {
        fail(
            "the same session played twice did not reach the same score",
            &format!("{:?} against {:?}", session.score, replay.score),
        );
    }

    // --- what was drawn ----------------------------------------------
    if session.frames != TICKS as usize {
        fail(
            "one frame per tick was expected",
            &format!("{} frames for {TICKS} ticks", session.frames),
        );
    }
    let Some(last) = recorder.frames().last() else {
        fail("no frame was recorded", "the loop above draws every tick");
    };

    // Both paddles are on screen where the world says they are. "Something is
    // drawn there" is not enough — the halfway line and the field border cross
    // most of the screen — so the quad has to be the *size* of a paddle.
    let final_tick = session.player_track.len() - 1;
    for (name, y, x) in [
        (
            "player",
            session.player_track[final_tick],
            -(FIELD.x - crate::PADDLE_INSET),
        ),
        (
            "computer",
            session.cpu_track[final_tick],
            FIELD.x - crate::PADDLE_INSET,
        ),
    ] {
        let at = Vec2::new(x, y);
        let drawn = last.covering(at).into_iter().any(|quad| {
            let size = quad.bounds().size();
            near(size.x, PADDLE_SIZE.x) && near(size.y, PADDLE_SIZE.y)
        });
        if !drawn {
            fail(
                &format!("no paddle-shaped quad was drawn where the {name}'s paddle is"),
                &format!(
                    "the world puts it at ({:.2}, {:.2}), {} by {}",
                    at.x, at.y, PADDLE_SIZE.x, PADDLE_SIZE.y
                ),
            );
        }
    }

    let (ball_pos, _) = session.ball_track[final_tick];
    let ball_drawn = last.covering(ball_pos).into_iter().any(|quad| {
        let size = quad.bounds().size();
        near(size.x, BALL_SIZE) && near(size.y, BALL_SIZE)
    });
    if !ball_drawn {
        fail(
            "no ball-shaped quad was drawn where the ball is",
            &format!(
                "the world puts it at ({:.2}, {:.2}), {BALL_SIZE} square",
                ball_pos.x, ball_pos.y
            ),
        );
    }

    // Nothing is drawn off the edge of the screen.
    //
    // This is the assertion that pays for itself. The first game-over banner
    // was one 43-character line, centred — 43.5 world units across a screen
    // 35.6 wide, so it ran off both sides. Every other check here passed:
    // glyphs existed, the score was in place, the world was correct. Only
    // reading the transcript found it, and only this makes it stay found.
    let (top_left, bottom_right) = session.visible;
    let screen = Rect {
        min: top_left,
        max: bottom_right,
    };
    let escaped = last.quads().into_iter().find(|quad| {
        let bounds = quad.bounds();
        greater(screen.min.x, bounds.min.x)
            || greater(screen.min.y, bounds.min.y)
            || greater(bounds.max.x, screen.max.x)
            || greater(bounds.max.y, screen.max.y)
    });
    if let Some(quad) = escaped {
        let bounds = quad.bounds();
        fail(
            "something was drawn off the edge of the screen",
            &format!(
                "a quad spanning ({:.2}, {:.2}) to ({:.2}, {:.2}) against a camera that can \
                 see ({:.2}, {:.2}) to ({:.2}, {:.2}) — text centred by width_of is the \
                 usual culprit, and it overruns silently",
                bounds.min.x,
                bounds.min.y,
                bounds.max.x,
                bounds.max.y,
                screen.min.x,
                screen.min.y,
                screen.max.x,
                screen.max.y
            ),
        );
    }

    // The score is text, and text is a quad sampling the font atlas — nothing
    // else in this game can produce one.
    let glyphs = last
        .quads()
        .into_iter()
        .filter(|quad| quad.texture == font)
        .count();
    if glyphs == 0 {
        fail(
            "nothing on screen sampled the font atlas",
            "the score is the only way this game says what is happening, so a frame with \
             no glyphs has lost it",
        );
    }
    // And it is where the layout puts it: the left-hand score's last digit ends
    // 1.4 units left of the halfway line, whatever the score has grown to.
    let digits = TextStyle {
        size: 2.6,
        ..TextStyle::default()
    };
    let left_text = session.score.left.to_string();
    let left_middle = Vec2::new(
        -1.4 - digits.width_of(&left_text) * 0.5,
        -crate::VIEW_HEIGHT / 2.0 + 0.9 + digits.size / 2.0,
    );
    if !last
        .covering(left_middle)
        .into_iter()
        .any(|quad| quad.texture == font)
    {
        fail(
            "the left-hand score is not where the game draws it",
            &format!(
                "no glyph covers ({:.2}, {:.2}), which is the middle of a score placed by \
                 TextStyle::width_of",
                left_middle.x, left_middle.y
            ),
        );
    }

    println!("verified pong over {TICKS} ticks");
    println!(
        "  player paddle: {start:.2} -> {bottom:.2} (tick {}) -> {top:.2} (tick {}), clamped \
         to +/-{PLAYER_LIMIT:.2}",
        bottom_at + 1,
        top_at + 1,
    );
    println!(
        "  score: {} - {} over {points} points, longest rally {} touches",
        session.score.left, session.score.right, session.volley_max
    );
    println!(
        "  ball: fastest {fastest:.2} units/s, parked for {held} ticks before the serve on \
         tick {}",
        serve_at + 1
    );
    println!(
        "  frames: {}, last one {} quads, {glyphs} of them glyphs",
        session.frames,
        last.quad_count()
    );
    println!(
        "  played by someone watching the ball: won {} - {} on tick {won_at}, longest rally \
         {} touches, ball up to {:.2} units/s",
        tracked.score.left, tracked.score.right, tracked.volley_max, tracked.fastest
    );
    println!("  replayed {TICKS} ticks and got the same game, bit for bit");
    print!("{}", last.transcript());
}
