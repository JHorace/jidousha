//! The check on the game: script the input, run headless, assert, print.
//!
//! `cargo run -p jidousha --example pong -- --verify` runs this. It registers
//! the *same* systems and the same config the window does — a check that built
//! a different game would be checking a different program. What differs is only
//! what a person would otherwise supply: the input comes from a script.
//!
//! Two halves, and both matter. "What the world did" is the simulation: the
//! paddle obeyed its clamp, the ball stayed on the field, a rally happened, a
//! point was scored, the match ended and restarted. "What was drawn" is the
//! frame: a paddle-shaped quad where the world says the paddle is, a ball-sized
//! one where the ball is, and glyphs where the score is laid out. A game that
//! simulated perfectly and drew nothing would pass the first half alone.

use crate::{
    BALL_HALF, Ball, Bout, FIELD, PADDLE_LIMIT, PADDLE_SIZE, Paddle, SCORE_GAP, SCORE_TOP, Score,
    Side, TARGET, Tally, config, register, score_style,
};
use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FramePlan, InputScript, NullBackend, PhysicalSize,
    RenderBackend, create_builtin_textures, plan_frame,
};
use std::cmp::Ordering;

/// How long the scripted session runs.
///
/// Long enough for the opponent to win a whole match off an unhelpful player
/// and for the restart below to start another one — the full loop, not a
/// slice of it.
const TICKS: u64 = 1500;

/// The tick the script asks for a new match.
///
/// After the match is certainly over: the assertions check that it really was,
/// so a game that got faster or slower fails here rather than quietly
/// verifying a restart that restarted nothing.
const RESTART_TICK: u64 = 1200;

/// The viewport the headless run uses, so the frame is the same everywhere.
///
/// 16:9, matching the window: the field is nineteen world units tall whatever
/// the window is, so a squarer viewport would crop the paddles out and the
/// drawing assertions would be about a different picture.
const HEADLESS_VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

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

/// `a > b`, and false when either is NaN.
///
/// Spelled out rather than written as the negation of `<=`, because a NaN that
/// crept into the ball's position would satisfy every plain `<=` bound below
/// and sail through the whole verification.
fn greater(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(Ordering::Greater))
}

/// Within a thousandth, and false when either is NaN.
fn near(a: f32, b: f32) -> bool {
    greater(0.001, (a - b).abs())
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

/// The whole session, written down.
///
/// The holds are longer than the travel they have available, so the clamp is
/// *exercised* rather than merely not violated, and they use both key pairs so
/// a broken arrow-key binding is a failure rather than a thing nobody tried.
/// After that the player stands still and loses, which is what gets the match
/// to its end inside a fixed number of ticks.
fn script() -> InputScript {
    InputScript::new()
        .hold(Key::S, 20..220)
        .hold(Key::W, 240..430)
        .hold(Key::ArrowDown, 450..560)
        .hold(Key::ArrowUp, 580..660)
        .press(Key::Space, RESTART_TICK)
}

/// What one scripted run of the game did, tick by tick.
pub(super) struct Run {
    /// The player's paddle Y after each tick.
    player_track: Vec<f32>,
    /// The opponent's paddle Y after each tick.
    opponent_track: Vec<f32>,
    /// Where the ball was after each tick.
    ball_track: Vec<Vec2>,
    /// How fast it was going after each tick.
    velocity_track: Vec<Vec2>,
    /// The score after each tick.
    score_track: Vec<Score>,
    /// Who had won, after each tick.
    winner_track: Vec<Option<Side>>,
    /// What had happened by each tick.
    tally_track: Vec<Tally>,
    /// How many frames were submitted.
    frames: usize,
}

/// Play the scripted session through `backend`, drawing every tick.
fn play(backend: &mut dyn RenderBackend, viewport: PhysicalSize) -> Run {
    let mut sim = headless(config(), register);
    let script = script();
    let textures = create_builtin_textures(backend);
    let mut run = Run {
        player_track: Vec::new(),
        opponent_track: Vec::new(),
        ball_track: Vec::new(),
        velocity_track: Vec::new(),
        score_track: Vec::new(),
        winner_track: Vec::new(),
        tally_track: Vec::new(),
        frames: 0,
    };

    for tick in 1..=TICKS {
        sim.world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        sim.tick();

        let mut player = None;
        let mut opponent = None;
        for (_, transform, paddle) in sim.world().query::<(&Transform, &Paddle)>() {
            match paddle.side {
                Side::Left => player = Some(transform.pos.y),
                Side::Right => opponent = Some(transform.pos.y),
            }
        }
        let (Some(player), Some(opponent)) = (player, opponent) else {
            fail(
                "a paddle is missing",
                "Startup spawns exactly one for each side and nothing removes them",
            );
        };
        run.player_track.push(player);
        run.opponent_track.push(opponent);

        match sim
            .world()
            .query::<(&Transform, &Ball)>()
            .map(|(_, transform, ball)| (transform.pos, ball.velocity))
            .next()
        {
            Some((pos, velocity)) => {
                run.ball_track.push(pos);
                run.velocity_track.push(velocity);
            }
            None => fail("the ball is gone", "Startup spawns exactly one"),
        }
        run.score_track.push(*sim.world().resource::<Score>());
        run.winner_track.push(sim.world().resource::<Bout>().winner);
        run.tally_track.push(*sim.world().resource::<Tally>());

        // Draw every tick, so the frame that gets asserted on is a frame of
        // the game in motion rather than of its first moment.
        let camera = Camera {
            viewport,
            ..*sim.world().resource::<Camera>()
        };
        let quads = sim.draw().quads().to_vec();
        let plan = plan_frame(&camera, &quads, &textures);
        if let Err(error) = backend.render(&plan) {
            fail("a backend refused a frame", &error.to_string());
        }
        run.frames += 1;
    }

    run
}

pub fn run() {
    let mut backend = NullBackend::new();
    let session = play(&mut backend, HEADLESS_VIEWPORT);
    let Run {
        player_track,
        opponent_track,
        ball_track,
        velocity_track,
        score_track,
        winner_track,
        tally_track,
        frames,
    } = &session;
    let tally = tally_track[tally_track.len() - 1];
    /// The index into every per-tick track of the tick Space is pressed.
    const RESTART_AT: usize = RESTART_TICK as usize - 1;

    // --- what the world did ------------------------------------------
    // Y is down, so the bottom of the screen is the larger number.
    let bottom_at = peak_at(player_track, greater);
    let top_at = peak_at(player_track, |a, b| greater(b, a));
    let (bottom, top) = (player_track[bottom_at], player_track[top_at]);
    if !near(bottom, PADDLE_LIMIT) || !near(top, -PADDLE_LIMIT) {
        fail(
            "the player's paddle did not come to rest against both ends of its field",
            &format!(
                "it reached {bottom:.3} and {top:.3}; the clamp is +/-{PADDLE_LIMIT:.2}, and \
                 the script holds each key long enough to run past it"
            ),
        );
    }
    // Down first, then up: S and W the right way round. Both extremes are
    // reached either way, so only the order tells a swap apart.
    if bottom_at >= top_at {
        fail(
            "S and W move the player's paddle the wrong way round",
            &format!(
                "the script holds S first, but the paddle was at the top on tick {} before \
                 it was at the bottom on tick {}",
                top_at + 1,
                bottom_at + 1
            ),
        );
    }
    // The arrow keys are a second binding on the same paddle, and the script
    // holds them after both letter keys are done. A paddle that stopped moving
    // at tick 430 means the arrows are wired to nothing.
    let arrow_span = &player_track[450..660];
    let arrow_low = arrow_span[peak_at(arrow_span, |a, b| greater(b, a))];
    let arrow_high = arrow_span[peak_at(arrow_span, greater)];
    if !greater(arrow_high - arrow_low, 1.0) {
        fail(
            "the arrow keys do not move the player's paddle",
            &format!(
                "the script holds ArrowDown then ArrowUp over ticks 450..660, and the paddle \
                 moved only between {arrow_low:.2} and {arrow_high:.2}"
            ),
        );
    }

    // The ball never leaves the field. This is the invariant a tunnelling bug
    // breaks, and it is checked on every tick rather than at the end, because
    // a ball that escaped and was re-served would look fine at the end.
    let wall = FIELD.y - BALL_HALF;
    for (index, at) in ball_track.iter().enumerate() {
        if greater(at.y.abs(), wall + 0.001) || greater(at.x.abs(), FIELD.x) {
            fail(
                "the ball left the field",
                &format!(
                    "on tick {} it was at ({:.3}, {:.3}); the field is +/-{:.1} by \
                     +/-{wall:.1}",
                    index + 1,
                    at.x,
                    at.y,
                    FIELD.x
                ),
            );
        }
    }

    // A rally happened: the ball came off both paddles at least once. Which
    // paddle sent it back is read off the tick its X velocity changed sign,
    // so this is "the opponent returned a ball", not "the opponent exists".
    let mut returned_by_player = 0;
    let mut returned_by_opponent = 0;
    // Where the ball's centre sits when its edge touches a paddle's edge.
    // A return that happens anywhere past this is a ball that turned round
    // *inside* the paddle, which looks exactly like a return until you watch
    // it in slow motion.
    let face = crate::PADDLE_X - PADDLE_SIZE.x * 0.5 - BALL_HALF;
    for (index, velocity) in velocity_track.iter().enumerate().skip(1) {
        let was = velocity_track[index - 1].x;
        if greater(was * velocity.x, 0.0) || was == 0.0 || velocity.x == 0.0 {
            continue;
        }
        let at = ball_track[index];
        let (by, legal) = if greater(velocity.x, 0.0) {
            returned_by_player += 1;
            ("the player", !greater(-face, at.x))
        } else {
            returned_by_opponent += 1;
            ("the opponent", !greater(at.x, face))
        };
        if !legal {
            fail(
                &format!("the ball turned round inside {by}'s paddle"),
                &format!(
                    "on tick {} it reversed at x={:.3}; the face of the paddle is at \
                     +/-{face:.2}",
                    index + 1,
                    at.x
                ),
            );
        }
    }
    if returned_by_player == 0 || returned_by_opponent == 0 {
        fail(
            "the ball did not come off both paddles",
            &format!(
                "the player returned it {returned_by_player} times and the opponent \
                 {returned_by_opponent}; a Pong where one paddle never connects is not \
                 a rally"
            ),
        );
    }
    if tally.wall_bounces == 0 {
        fail(
            "the ball never bounced off a wall",
            "every serve leaves the centre at an angle, so a run this long without a \
             single wall bounce means the top and bottom are not solid",
        );
    }

    // Every point that was scored is on the board. Checked per tick rather
    // than once at the end, because the board is wiped between matches and a
    // total taken at the end would be a total of the second match only.
    let final_score = score_track[score_track.len() - 1];
    for (index, score) in score_track.iter().enumerate() {
        let counted = score.left + score.right;
        let scored = tally_track[index].points;
        if counted > scored {
            fail(
                "the board shows points that were never scored",
                &format!(
                    "on tick {} it reads {}-{} against {scored} points scored all run",
                    index + 1,
                    score.left,
                    score.right
                ),
            );
        }
    }

    // The match ended, and the ending is the opponent's: a player who never
    // chases the ball loses, which is also how this run reaches an ending at
    // all inside a fixed number of ticks.
    let Some(won_at) = winner_track.iter().position(Option::is_some) else {
        fail(
            "nobody won the match",
            &format!(
                "the script leaves the player standing still, so the opponent should reach \
                 {TARGET} well inside {TICKS} ticks; {} points were scored in all",
                tally.points
            ),
        );
    };
    if winner_track[won_at] != Some(Side::Right) {
        fail(
            "the wrong side won",
            "the script never moves the player toward the ball after tick 660",
        );
    }
    if score_track[won_at].right != TARGET {
        fail(
            "the match ended at the wrong score",
            &format!(
                "it ended at {}-{}, and the target is {TARGET}",
                score_track[won_at].left, score_track[won_at].right
            ),
        );
    }
    if won_at as u64 >= RESTART_TICK {
        fail(
            "the match was still running when the script asked for a new one",
            &format!(
                "it ended on tick {}, and Space is pressed on tick {RESTART_TICK}",
                won_at + 1
            ),
        );
    }
    // The ball is frozen between the win and the restart: a won match is over,
    // not a match that keeps playing behind a banner.
    for tick in won_at..RESTART_AT {
        if ball_track[tick] != ball_track[won_at] {
            fail(
                "the ball kept moving after the match was won",
                &format!(
                    "it was at ({:.2}, {:.2}) on tick {} and ({:.2}, {:.2}) on tick {}",
                    ball_track[won_at].x,
                    ball_track[won_at].y,
                    won_at + 1,
                    ball_track[tick].x,
                    ball_track[tick].y,
                    tick + 1
                ),
            );
        }
    }

    // And Space started another one: the board is clear and the ball is live.
    if winner_track[RESTART_AT].is_some() || score_track[RESTART_AT] != Score::default() {
        fail(
            "Space did not start a new match",
            &format!(
                "on the tick of the press the board reads {}-{} and the winner is {:?}",
                score_track[RESTART_AT].left,
                score_track[RESTART_AT].right,
                winner_track[RESTART_AT]
            ),
        );
    }
    let moved_again = velocity_track[RESTART_AT..]
        .iter()
        .any(|velocity| greater(velocity.length(), 0.0));
    if !moved_again {
        fail(
            "the ball never moved again after the restart",
            &format!(
                "{} ticks passed between the restart and the end of the run",
                TICKS - RESTART_TICK
            ),
        );
    }
    // The second match's board adds up on its own: whatever it reads at the
    // end is exactly the points scored since the restart.
    let since_restart = tally.points - tally_track[RESTART_AT].points;
    if final_score.left + final_score.right != since_restart {
        fail(
            "the new match did not start from nothing",
            &format!(
                "the board reads {}-{} and {since_restart} points have been scored since \
                 the restart",
                final_score.left, final_score.right
            ),
        );
    }

    // The opponent tracked the ball rather than sitting still — and did it
    // within its own clamp, which a paddle that chased a steep ball off the
    // top of the field would not.
    let opponent_low = opponent_track[peak_at(opponent_track, |a, b| greater(b, a))];
    let opponent_high = opponent_track[peak_at(opponent_track, greater)];
    if !greater(opponent_high - opponent_low, 4.0) {
        fail(
            "the opponent's paddle barely moved",
            &format!("it stayed between {opponent_low:.2} and {opponent_high:.2}"),
        );
    }
    if greater(opponent_high, PADDLE_LIMIT + 0.001) || greater(-PADDLE_LIMIT - 0.001, opponent_low)
    {
        fail(
            "the opponent's paddle left the field",
            &format!(
                "it reached {opponent_low:.3}..{opponent_high:.3}, and the clamp is \
                 +/-{PADDLE_LIMIT:.2}"
            ),
        );
    }

    // --- what was drawn ----------------------------------------------
    if *frames != TICKS as usize {
        fail(
            "one frame per tick was expected",
            &format!("{frames} frames for {TICKS} ticks"),
        );
    }
    let Some(last) = backend.last_frame() else {
        fail("no frame was recorded", "the loop above draws every tick");
    };
    // The field markings, the paddles and the ball, and the text: the shapes
    // and the font do not sample the same texture.
    if last.plan.batches.len() < 2 {
        fail(
            "the last frame is too simple to be this game",
            &format!(
                "{} batches; expected shapes and text",
                last.plan.batches.len()
            ),
        );
    }

    // The paddles really are on screen where the world says they are. The
    // positions are read back out of the world rather than written down here,
    // so this asks whether drawing agrees with simulation. "Something is
    // drawn there" is not enough — the halfway line and the text wander over
    // much of the field — so the quad has to be the *size* of a paddle.
    for (name, at) in [
        (
            "the player's",
            Vec2::new(-crate::PADDLE_X, player_track[player_track.len() - 1]),
        ),
        (
            "the opponent's",
            Vec2::new(crate::PADDLE_X, opponent_track[opponent_track.len() - 1]),
        ),
    ] {
        let drawn = last.covering(at).into_iter().any(|quad| {
            let size = quad.bounds().size();
            near(size.x, PADDLE_SIZE.x) && near(size.y, PADDLE_SIZE.y)
        });
        if !drawn {
            fail(
                &format!("no paddle-shaped quad was drawn where {name} paddle is"),
                &format!(
                    "the world puts it at ({:.2}, {:.2}), {} by {}",
                    at.x, at.y, PADDLE_SIZE.x, PADDLE_SIZE.y
                ),
            );
        }
    }

    let ball_at = ball_track[ball_track.len() - 1];
    let ball_drawn = last.covering(ball_at).into_iter().any(|quad| {
        let size = quad.bounds().size();
        near(size.x, BALL_HALF * 2.0) && near(size.y, BALL_HALF * 2.0)
    });
    if !ball_drawn {
        fail(
            "no ball-sized quad was drawn where the ball is",
            &format!(
                "the world has it at ({:.2}, {:.2}), {} square",
                ball_at.x,
                ball_at.y,
                BALL_HALF * 2.0
            ),
        );
    }

    // Text is on screen, and where the game lays it out. The font atlas is a
    // texture like any other, so "was text drawn" is "did a quad sample the
    // font", and the score's own position is what says the layout ran rather
    // than something merely having been submitted.
    let Some(font) = font_texture(&last.plan) else {
        fail(
            "nothing on screen sampled the font atlas",
            "the score and the footer are both text, so a frame without a font batch has \
             lost both",
        );
    };
    let glyphs: usize = last
        .plan
        .batches
        .iter()
        .filter(|batch| batch.texture == font)
        .map(|batch| batch.quad_count())
        .sum();

    // The left score is laid out backwards from a fixed gap either side of the
    // halfway line, so the middle of its last digit is a point the layout
    // decides — not a constant this file made up.
    let style = score_style();
    let digits = format!("{}", final_score.left);
    let last_digit = Vec2::new(
        -SCORE_GAP - style.width_of(&digits) / digits.len() as f32 * 0.5,
        SCORE_TOP + style.size * 0.5,
    );
    let score_drawn = last
        .covering(last_digit)
        .into_iter()
        .any(|quad| quad.texture == font);
    if !score_drawn {
        fail(
            "the player's score is not where the game draws it",
            &format!(
                "no glyph covers ({:.2}, {:.2}), which is the middle of the last digit of a \
                 score laid out by TextStyle::width_of",
                last_digit.x, last_digit.y
            ),
        );
    }

    // --- and the same run, twice --------------------------------------
    // The whole point of a fixed timestep and a seeded generator: replay the
    // session and land on the same numbers, bit for bit.
    let mut again = NullBackend::new();
    let replay = play(&mut again, HEADLESS_VIEWPORT);
    let bits = |track: &[Vec2]| -> Vec<[u32; 2]> {
        track
            .iter()
            .map(|at| [at.x.to_bits(), at.y.to_bits()])
            .collect()
    };
    if bits(&replay.ball_track) != bits(ball_track) {
        fail(
            "the same script played two different games",
            "the timestep is fixed and the generator is seeded from GameConfig, so a \
             replay lands on the same numbers",
        );
    }

    println!("verified pong over {TICKS} ticks");
    println!(
        "  player paddle: -> {bottom:.2} (tick {}) -> {top:.2} (tick {}), clamped to \
         +/-{PADDLE_LIMIT:.2}",
        bottom_at + 1,
        top_at + 1,
    );
    println!("  opponent paddle: {opponent_low:.2}..{opponent_high:.2}");
    println!(
        "  rally: {returned_by_player} returns by the player, {returned_by_opponent} by the \
         opponent, {} wall bounces",
        tally.wall_bounces
    );
    println!(
        "  match: opponent won {}-{} on tick {}, restarted on tick {RESTART_TICK}, {}-{} at \
         the end",
        score_track[won_at].right,
        score_track[won_at].left,
        won_at + 1,
        final_score.left,
        final_score.right,
    );
    println!(
        "  last frame: {} batches, {glyphs} glyphs, ball at ({:.2}, {:.2})",
        last.plan.batches.len(),
        ball_at.x,
        ball_at.y
    );
    // --- and a game with someone playing it ---------------------------
    // The session above is one-sided on purpose, and a one-sided game leaves
    // most of Pong untouched: nothing there says a rally is possible, that a
    // paddle can put an angle on the ball, or that the opponent can be beaten.
    let rally = play_a_rally();
    if rally.longest < 4 {
        fail(
            "there are no rallies in this game of Pong",
            &format!(
                "with a player tracking the ball, the longest exchange in {RALLY_TICKS} \
                 ticks was {} hits",
                rally.longest
            ),
        );
    }
    if rally.tally.wall_bounces < 3 {
        fail(
            "the ball goes back and forth in a straight line",
            &format!(
                "{} wall bounces in {RALLY_TICKS} ticks; the angle a paddle puts on the \
                 ball is what makes this a game rather than a metronome",
                rally.tally.wall_bounces
            ),
        );
    }
    if !near(rally.top_speed, crate::MAX_SPEED) {
        fail(
            "the ball never reached its top speed",
            &format!(
                "it peaked at {:.2} against a cap of {:.1}; either the rallies got shorter \
                 or the speed-up stopped",
                rally.top_speed,
                crate::MAX_SPEED
            ),
        );
    }
    // The one assertion here that is about the *game* rather than the code: a
    // match that only ever goes one way is a demo. Both ends scored, so the
    // opponent can be beaten and the player can lose.
    if rally.score.left == 0 || rally.score.right == 0 {
        fail(
            "this match only goes one way",
            &format!(
                "after {RALLY_TICKS} ticks against a player who tracks the ball it stands \
                 at {}-{}; an opponent nobody can beat, or nobody can lose to, is not a \
                 game",
                rally.score.left, rally.score.right
            ),
        );
    }
    println!(
        "  rallying: longest exchange {} hits, {} wall bounces, top speed {:.1}, {}-{} after \
         {RALLY_TICKS} ticks",
        rally.longest,
        rally.tally.wall_bounces,
        rally.top_speed,
        rally.score.left,
        rally.score.right,
    );
    println!("  replayed the whole session: identical to the bit");

    // The frame itself, as text. There is no GPU on most of the machines this
    // runs on, so this is the picture: every quad the last tick submitted, in
    // world units, for a person or an agent to read the game's geometry off.
    print!("{}", last.transcript());
}

/// Thirty seconds of a game where the player is paying attention.
///
/// The honest bar for the prototype, in ticks.
const RALLY_TICKS: u64 = 1800;

/// What a session with a competent player looked like.
struct Rally {
    /// The longest unbroken exchange, in paddle hits.
    longest: u32,
    /// Every paddle hit, and every wall bounce.
    tally: Tally,
    /// The score at the end.
    score: Score,
    /// The fastest the ball ever went, in world units per second.
    top_speed: f32,
}

/// How far behind the ball the stand-in player is, in ticks.
///
/// A fifth of a second, which is roughly what a person's hand does. The lag is
/// the point rather than a concession to realism: a player who tracks the ball
/// *exactly* meets it with the middle of the paddle every time, and the middle
/// of the paddle returns the ball dead flat, so two exact trackers rally to the
/// heat death of the universe and neither the angles nor the scoring ever get
/// exercised. Lagging by a few ticks means contact lands off-centre, which is
/// what makes a rally a rally.
const REACTION_LAG: usize = 12;

/// A session with a hand on the player's paddle instead of on the keyboard.
///
/// `InputScript` is a pure function of the tick — which is exactly what makes
/// it replayable, and exactly why it cannot chase a ball. So this one reaches
/// into the world between ticks and steers the left paddle at where the ball
/// was, no faster than the paddle's own speed. It is not the input path and it
/// does not pretend to be: the scripted session above is what checks that keys
/// move the paddle. What this checks is everything a real rally touches and a
/// one-sided game never does — the angle a paddle puts on the ball, the
/// speed-up, the cap, whether a fast ball can slip through a paddle, and
/// whether the match is winnable from either end.
fn play_a_rally() -> Rally {
    let mut sim = headless(config(), register);
    let idle = InputScript::new();
    let mut rally = Rally {
        longest: 0,
        tally: Tally::default(),
        score: Score::default(),
        top_speed: 0.0,
    };
    let mut hits_before = 0;
    let mut seen: Vec<f32> = Vec::new();

    for tick in 1..=RALLY_TICKS {
        let dt = sim.world().resource::<Time>().fixed_dt.as_f32();
        if let Some(at) = sim
            .world()
            .query::<(&Transform, &Ball)>()
            .map(|(_, transform, _)| transform.pos.y)
            .next()
        {
            seen.push(at);
        }
        // Empty until Startup has run, which happens inside the first `tick`
        // rather than before it — so on tick one there is no ball to chase and
        // no paddle to move.
        let goal = seen
            .get(seen.len().saturating_sub(REACTION_LAG + 1))
            .copied();
        let player = sim
            .world()
            .query::<(&Transform, &Paddle)>()
            .find(|(_, _, paddle)| paddle.side == Side::Left)
            .map(|(entity, _, _)| entity);
        if let (Some(goal), Some(entity)) = (goal, player) {
            let reach = crate::PLAYER_SPEED * dt;
            let at = sim.world_mut().component_mut::<Transform>(entity);
            at.pos.y = (at.pos.y + (goal - at.pos.y).clamp(-reach, reach))
                .clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
        }

        sim.world_mut()
            .insert_resource(Input::new(idle.snapshot_at(tick)));
        sim.tick();

        let Some((at, velocity)) = sim
            .world()
            .query::<(&Transform, &Ball)>()
            .map(|(_, transform, ball)| (transform.pos, ball.velocity))
            .next()
        else {
            fail("the ball is gone", "Startup spawns exactly one");
        };
        let speed = velocity.length();
        if greater(speed, rally.top_speed) {
            rally.top_speed = speed;
        }
        if greater(speed, crate::MAX_SPEED + 0.001) {
            fail(
                "the ball went faster than its own limit",
                &format!(
                    "on tick {tick} it was doing {speed:.3}, and the cap is \
                     {:.1} — a ball past the cap can cross a paddle inside one tick",
                    crate::MAX_SPEED
                ),
            );
        }
        if greater(at.y.abs(), FIELD.y - BALL_HALF + 0.001) || greater(at.x.abs(), FIELD.x) {
            fail(
                "the ball left the field during a rally",
                &format!("on tick {tick} it was at ({:.3}, {:.3})", at.x, at.y),
            );
        }

        let tally = *sim.world().resource::<Tally>();
        let bout = *sim.world().resource::<Bout>();
        if tally.points > rally.tally.points {
            hits_before = tally.paddle_hits;
        }
        rally.longest = rally.longest.max(tally.paddle_hits - hits_before);
        rally.tally = tally;
        rally.score = *sim.world().resource::<Score>();
        if bout.winner.is_some() {
            break;
        }
    }

    rally
}

/// Which backend texture the font atlas landed on, read off the frame.
///
/// The table is gone by the time the assertions run, and the atlas is not at a
/// fixed id — it is whatever `create_builtin_textures` assigned. So: build a
/// table against a throwaway backend, in the same order, and ask it.
fn font_texture(plan: &FramePlan) -> Option<BackendTextureId> {
    let mut scratch = NullBackend::new();
    let table = create_builtin_textures(&mut scratch);
    let font = table.resolve(FONT_TEXTURE);
    plan.batches
        .iter()
        .any(|batch| batch.texture == font)
        .then_some(font)
}
