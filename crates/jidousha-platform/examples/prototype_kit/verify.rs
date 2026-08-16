//! The §5 loop: script input, run headless, assert, print the transcript.
//!
//! This is what `tools/verify prototype_kit` runs, and it is the engine's whole
//! thesis in one function — an agent drives the game, asks what the world did
//! and what was drawn, and never opens a window (input.md §5, renderer.md §9).
//!
//! It runs the *same* systems and the same config the window does. What differs
//! is only what a person would otherwise supply: the input comes from a script,
//! and the art from a store with scripted arrival ticks, so the run is the same
//! on every machine and on every day.

use crate::{Paddle, config, register};
use jidousha_assets::{Assets, MemorySource, decode_png};
use jidousha_core::math::Vec2;
use jidousha_core::{Transform, headless};
use jidousha_input::{Input, InputScript, Key};
use jidousha_render_core::{
    BackendTextureId, Camera, FONT_TEXTURE, FramePlan, NullBackend, PhysicalSize, RenderBackend,
    Sprite, create_builtin_textures, plan_frame,
};
use std::cmp::Ordering;

/// How long the scripted session runs.
///
/// Long enough for the script below to push the paddle into *both* ends of
/// its clamp — a shorter run would still pass every assertion here, and the
/// clamp would be asserted only in the sense of never having been reached.
const TICKS: u64 = 130;

/// The tick the art is scripted to arrive on.
///
/// Partway through, so the run spends time on both sides of it and the
/// placeholder is part of what gets verified.
const ART_ARRIVES: u64 = 30;

/// How far the paddle may travel from the centre, matching its component.
const LIMIT: f32 = 7.0;

/// The camera the headless run uses, so the transcript is the same everywhere.
const HEADLESS_VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// The world height the game's camera is set to (`main.rs`'s `VIEW_HEIGHT`).
const HEADLESS_VIEW_HEIGHT: f32 = crate::VIEW_HEIGHT;

/// The score's text size, from `draw_the_readout`.
const SCORE_SIZE: f32 = 1.6;

/// Where the ball is after `TICKS` ticks, in world units.
///
/// A number, checked in. The ball's X is a sine of simulated time and its Y
/// never moves, so after a fixed number of fixed-length ticks it is in exactly
/// one place — and that is the whole determinism claim (core.md §7, ADR-0009)
/// reduced to something a verification can compare against.
const BALL_X_AT_END: f32 = -2.3294;
const BALL_Y: f32 = -4.0;

/// Fail with the engine's message shape, and a non-zero exit.
pub(super) fn fail(what: &str, specifics: &str) -> ! {
    eprintln!(
        "{}",
        jidousha_core::message(
            what,
            specifics,
            "the game changed, or the engine did",
            "run `cargo run -p jidousha-platform --example prototype_kit` and watch it, then \
             compare with the assertion above",
        )
    );
    std::process::exit(1);
}

/// `a > b`, and false when either is NaN.
///
/// Spelled out rather than written `!(a > b)` because the negation of a
/// float comparison silently means something else — a NaN that crept into a
/// position would satisfy every plain `<=` check and pass this verification
/// (the same reason `circle_quads` spells its radius test out).
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

/// The art, arriving on a scripted tick rather than whenever a disk says.
///
/// The real PNG, baked into the binary. A verify run reads no files at all —
/// it is the same on a machine that checked the repository out somewhere
/// else, and the bytes are the ones the window would have shown, so a
/// picture that stopped decoding fails here rather than passing on a stub.
fn store() -> Assets {
    let Ok(hero) = decode_png(include_bytes!("../../../../assets/sprites/hero.png")) else {
        fail(
            "the example's own art no longer decodes",
            "assets/sprites/hero.png is baked into this binary and read by nothing else",
        );
    };
    let mut source = MemorySource::new();
    source.insert_texture("sprites/hero.png", hero);
    source.complete_at("sprites/hero.png", ART_ARRIVES);
    Assets::new(source)
}

/// Hold S until the paddle jams against the bottom, then W until it jams
/// against the top.
///
/// Both holds deliberately last longer than the travel they have available,
/// so the clamp is *exercised* rather than merely not violated.
fn script() -> InputScript {
    InputScript::new().hold(Key::S, 5..45).hold(Key::W, 50..130)
}

/// What one scripted run of the game did.
///
/// Returned rather than asserted on inside the loop, so the *same* loop can be
/// played through two different backends and the results compared — which is
/// how this file checks renderer.md §1's contract that everything above the
/// seam is backend-agnostic.
pub(super) struct Run {
    /// The paddle's Y after each tick.
    pub(super) paddle_track: Vec<f32>,
    /// Where the paddle ended up.
    pub(super) paddle_pos: Vec2,
    /// Where the ball ended up.
    pub(super) ball_pos: Vec2,
    /// How many frames drew the checkered placeholder.
    pub(super) placeholder_frames: u32,
    /// How many frames were submitted.
    pub(super) frames: usize,
}

/// Play the scripted session through `backend`, drawing every tick.
///
/// `viewport` is the camera's, which decides the frame's aspect ratio and
/// nothing else — the world is the same whatever it is set to, which is the
/// point of the comparison the caller makes.
pub(super) fn play(backend: &mut dyn RenderBackend, viewport: PhysicalSize) -> Run {
    let mut sim = headless(config(), register);
    // Before Startup, which is what `set_the_scene` checks for.
    sim.world_mut().insert_resource(store());

    let script = script();
    let mut textures = create_builtin_textures(backend);
    let mut paddle_track = Vec::new();
    let mut paddle_pos = Vec2::ZERO;
    let mut ball_pos = Vec2::ZERO;
    let mut placeholder_frames = 0;
    let mut frames = 0;

    for tick in 1..=TICKS {
        let Some(assets) = sim.world_mut().find_resource_mut::<Assets>() else {
            fail(
                "the store vanished",
                "Startup installs one and nothing removes it",
            );
        };
        assets.commit(tick);
        jidousha_render_core::upload_ready_textures(assets, backend, &mut textures);

        sim.world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        sim.tick();

        let paddle = sim
            .world()
            .query::<(&Transform, &Paddle)>()
            .map(|(_, transform, _)| transform.pos)
            .next();
        match paddle {
            Some(pos) => {
                paddle_pos = pos;
                paddle_track.push(pos.y);
            }
            None => fail("the paddle is gone", "Startup spawns exactly one"),
        }
        match sim
            .world()
            .query::<(&Transform, &Sprite)>()
            .map(|(_, transform, _)| transform.pos)
            .next()
        {
            Some(pos) => ball_pos = pos,
            None => fail("the ball is gone", "Startup spawns exactly one sprite"),
        }

        // Draw every tick, so the transcript covers the frames before the
        // art arrives as well as the ones after.
        let camera = Camera {
            viewport,
            ..*sim.world().resource::<Camera>()
        };
        let quads = sim.draw().quads().to_vec();
        let plan = plan_frame(&camera, &quads, &textures);
        if plan
            .batches
            .iter()
            .any(|batch| batch.texture == textures.placeholder())
        {
            placeholder_frames += 1;
        }
        if let Err(error) = backend.render(&plan) {
            fail("a backend refused a frame", &error.to_string());
        }
        frames += 1;
    }

    Run {
        paddle_track,
        paddle_pos,
        ball_pos,
        placeholder_frames,
        frames,
    }
}

pub fn run() {
    let mut backend = NullBackend::new();
    let transcript_run = play(&mut backend, HEADLESS_VIEWPORT);
    let Run {
        paddle_track,
        paddle_pos,
        ball_pos,
        placeholder_frames,
        frames,
    } = &transcript_run;
    let (paddle_track, paddle_pos, ball_pos) = (paddle_track.clone(), *paddle_pos, *ball_pos);
    let (placeholder_frames, frames) = (*placeholder_frames, *frames);

    // --- what the world did ------------------------------------------
    // Y is down (ADR-0010), so the bottom of the screen is the larger number.
    let start = paddle_track[0];
    let bottom_at = peak_at(&paddle_track, greater);
    let top_at = peak_at(&paddle_track, |a, b| greater(b, a));
    let (bottom, top) = (paddle_track[bottom_at], paddle_track[top_at]);

    if !near(bottom, LIMIT) || !near(top, -LIMIT) {
        fail(
            "the paddle did not come to rest against both ends of its field",
            &format!(
                "it reached {bottom:.3} and {top:.3}; the clamp is +/-{LIMIT:.1}, and the \
                 script holds each key long enough to run past it"
            ),
        );
    }
    // Down first, then up: S and W the right way round. Both extremes are
    // reached either way, so only the order tells a swap apart.
    if bottom_at >= top_at {
        fail(
            "S and W move the paddle the wrong way round",
            &format!(
                "the script holds S first, but the paddle was at the top on tick \
                 {top} before it was at the bottom on tick {bottom}",
                top = top_at + 1,
                bottom = bottom_at + 1,
            ),
        );
    }
    if !greater(bottom, start) {
        fail(
            "the paddle did not start between the two ends it reached",
            &format!("it started at {start:.3}, which is not above {bottom:.3}"),
        );
    }

    // --- what was drawn ----------------------------------------------
    if frames != TICKS as usize {
        fail(
            "one frame per tick was expected",
            &format!("{frames} frames for {TICKS} ticks"),
        );
    }
    let Some(last) = backend.last_frame() else {
        fail("no frame was recorded", "the loop above draws every tick");
    };
    // The field markings, the paddles, the ball and the text: several
    // batches, because they do not all sample the same texture.
    if last.plan.batches.len() < 3 {
        fail(
            "the last frame is too simple to be this game",
            &format!(
                "{} batches; expected shapes, art and text",
                last.plan.batches.len()
            ),
        );
    }
    if placeholder_frames == 0 {
        fail(
            "the placeholder never appeared",
            "the art is scripted to arrive partway through, so the frames before it must \
             show the checkered placeholder (renderer.md §5)",
        );
    }
    if placeholder_frames as u64 >= TICKS {
        fail(
            "the placeholder never went away",
            "the art is scripted to arrive, so the later frames must draw it",
        );
    }
    // And the paddle really is on screen where the world says it is — the
    // position is read back out of the world rather than written down here,
    // so this asks whether drawing agrees with simulation.
    //
    // "Something is drawn there" is not enough: the readout text wanders
    // across most of the field, so that question passes with the paddle
    // deleted. The quad has to be the *size* of a paddle.
    let paddle_shaped = last.covering(paddle_pos).into_iter().any(|quad| {
        let bounds = quad.bounds();
        near(bounds.max.x - bounds.min.x, crate::PADDLE_SIZE.x)
            && near(bounds.max.y - bounds.min.y, crate::PADDLE_SIZE.y)
    });
    if !paddle_shaped {
        fail(
            "no paddle-shaped quad was drawn where the paddle is",
            &format!(
                "the world puts it at ({:.2}, {:.2}), {} by {}",
                paddle_pos.x,
                paddle_pos.y,
                crate::PADDLE_SIZE.x,
                crate::PADDLE_SIZE.y
            ),
        );
    }

    // Text is on screen, and where the game puts it. The font atlas is a
    // texture like any other (renderer.md §6), so "was text drawn" is "did a
    // quad sample the font", and the score's own position is what says the
    // layout ran rather than something merely having been submitted.
    let font = textures_font_id(&last.plan);
    let Some(font) = font else {
        fail(
            "nothing on screen sampled the font atlas",
            "the score, the readout and the character sample are all text, so a frame \
             without a font batch has lost all three (renderer.md §6)",
        );
    };
    let glyphs: usize = last
        .plan
        .batches
        .iter()
        .filter(|batch| batch.texture == font)
        .map(|batch| batch.quad_count())
        .sum();
    // The score is drawn centred at the top; its middle character is a dash,
    // whose cell straddles this point. A layout that stopped centring, or a
    // camera that stopped agreeing with it, moves the text off this spot.
    let score_middle = Vec2::new(0.0, -HEADLESS_VIEW_HEIGHT / 2.0 + 1.0 + SCORE_SIZE / 2.0);
    let score_drawn = last
        .covering(score_middle)
        .into_iter()
        .any(|quad| quad.texture == font);
    if !score_drawn {
        fail(
            "the score is not where the game draws it",
            &format!(
                "no glyph covers ({:.2}, {:.2}), which is the middle of a score centred by \
                 TextStyle::width_of",
                score_middle.x, score_middle.y
            ),
        );
    }

    // The ball is a sprite, and after a fixed number of ticks it is in a fixed
    // place — the whole determinism claim in one number (core.md §7). The
    // engine's own sin/cos is what puts it there (ADR-0009), so this is the
    // assertion that fails if the timestep, the seed of the clock, or the
    // trigonometry ever changes.
    if !near(ball_pos.x, BALL_X_AT_END) || !near(ball_pos.y, BALL_Y) {
        fail(
            "the ball is not where this many ticks should have put it",
            &format!(
                "after {TICKS} ticks it is at ({:.4}, {:.4}); it was ({BALL_X_AT_END:.4}, \
                 {BALL_Y:.4}) when this was written",
                ball_pos.x, ball_pos.y
            ),
        );
    }
    let ball_drawn = last
        .covering(ball_pos)
        .into_iter()
        .any(|quad| quad.texture != font && quad.bounds().size().x > 2.0);
    if !ball_drawn {
        fail(
            "the ball sprite is not drawn where the world puts it",
            &format!("the world has it at ({:.2}, {:.2})", ball_pos.x, ball_pos.y),
        );
    }

    let captured = crate::capture::capture_a_frame(&paddle_track);

    println!("verified prototype_kit over {TICKS} ticks");
    println!(
        "  paddle: {start:.2} -> {bottom:.2} (tick {}) -> {top:.2} (tick {}), clamped to \
         +/-{LIMIT:.1}",
        bottom_at + 1,
        top_at + 1,
    );
    println!("  frames: {frames}, {placeholder_frames} of them with the placeholder");
    println!(
        "  ball: ({:.3}, {:.3}) after {TICKS} ticks",
        ball_pos.x, ball_pos.y
    );
    println!(
        "  last frame: {} batches, {glyphs} glyphs",
        last.plan.batches.len()
    );
    println!("  capture: {captured}");
    print!("{}", last.transcript());
}

/// Which backend texture the font atlas landed on, read off the frame.
///
/// The table is gone by the time the assertions run, and the atlas is not at a
/// fixed id — it is whatever `create_builtin_textures` assigned. The glyph
/// quads are the ones whose UVs sit inside the atlas *and* whose batch is
/// neither the placeholder nor the flat white texel, which is more indirection
/// than it is worth; asking the plan for the batch with the most quads is not
/// robust either. So: rebuild a table against a throwaway backend, in the same
/// order, and ask it.
fn textures_font_id(plan: &FramePlan) -> Option<BackendTextureId> {
    let mut scratch = NullBackend::new();
    let table = create_builtin_textures(&mut scratch);
    let font = table.resolve(FONT_TEXTURE);
    plan.batches
        .iter()
        .any(|batch| batch.texture == font)
        .then_some(font)
}
