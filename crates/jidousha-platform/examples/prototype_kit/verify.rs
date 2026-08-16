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
    Camera, NullBackend, RenderBackend, create_builtin_textures, plan_frame,
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

/// Fail with the engine's message shape, and a non-zero exit.
fn fail(what: &str, specifics: &str) -> ! {
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

pub fn run() {
    let mut sim = headless(config(), register);
    // Before Startup, which is what `set_the_scene` checks for.
    sim.world_mut().insert_resource(store());

    let script = script();
    let mut backend = NullBackend::new();
    let mut textures = create_builtin_textures(&mut backend);
    let mut paddle_track = Vec::new();
    let mut paddle_pos = Vec2::ZERO;
    let mut placeholder_frames = 0;

    for tick in 1..=TICKS {
        let Some(assets) = sim.world_mut().find_resource_mut::<Assets>() else {
            fail(
                "the store vanished",
                "Startup installs one and nothing removes it",
            );
        };
        assets.commit(tick);
        jidousha_render_core::upload_ready_textures(assets, &mut backend, &mut textures);

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

        // Draw every tick, so the transcript covers the frames before the
        // art arrives as well as the ones after.
        let camera = *sim.world().resource::<Camera>();
        let quads = sim.draw().quads().to_vec();
        let plan = plan_frame(&camera, &quads, &textures);
        if plan
            .batches
            .iter()
            .any(|batch| batch.texture == textures.placeholder())
        {
            placeholder_frames += 1;
        }
        let Ok(()) = backend.render(&plan) else {
            fail(
                "the null backend refused a frame",
                "it cannot fail to record",
            );
        };
    }

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
    let frames = backend.frames().len();
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

    println!("verified prototype_kit over {TICKS} ticks");
    println!(
        "  paddle: {start:.2} -> {bottom:.2} (tick {}) -> {top:.2} (tick {}), clamped to \
         +/-{LIMIT:.1}",
        bottom_at + 1,
        top_at + 1,
    );
    println!("  frames: {frames}, {placeholder_frames} of them with the placeholder");
    println!("  last frame: {} batches", last.plan.batches.len());
    print!("{}", last.transcript());
}
