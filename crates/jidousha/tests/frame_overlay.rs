//! The frame-pacing overlay, in pixels: what it covers, and what it leaves alone.
//!
//! The native overlay is off unless `JIDOUSHA_FRAMETIME` asks for it
//! (frame-pacing.md §6), and "off by default" is a claim about a *picture*
//! rather than about a boolean. So this renders one scene twice through a real
//! GPU — once as the game submitted it and once with the readout over it — and
//! asks the two captures the two questions that matter: the overlay is visible
//! where it draws, and nothing under it moved.
//!
//! It also writes both captures to `target/verify/`, so the pair a change to
//! this area has to be justified with is a command anyone can re-run rather
//! than a screenshot somebody took once.
//!
//! **No adapter is not a failure**, exactly as in `backend_agnostic.rs` and for
//! the same reason: every runner this project has is headless and some have no
//! graphics stack at all (renderer.md §9). What is not allowed is skipping in
//! silence, so it prints why.

use jidousha::prelude::*;
use jidousha_render_core::{
    RenderBackend, RenderError, TextureTable, create_builtin_textures, encode_png,
    overlay::draw_readout, plan_frame,
};
use jidousha_render_wgpu::WgpuBackend;

/// How big the captures are. Small, but big enough that the readout's five-by-
/// seven glyphs are more than one pixel each.
const SIZE: PhysicalSize = PhysicalSize::new(1280, 720);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// What the panel says on a captured frame.
///
/// A fixed string rather than a live reading: this test is about where the
/// overlay lands and what it covers, and a capture whose text changed with the
/// machine's frame rate would be a picture nobody could compare.
const READOUT: &str = "jidousha frame pacing: JIDOUSHA_FRAMETIME=1\n\
                       present   ~60.0 fps - median 16.67ms, mean 16.67ms\n\
                       spread    16.41ms .. 17.02ms over 240 frames\n\
                       pacing    vsync - the display sets the rate\n\
                       ticks/fr  0:0 (0%)  1:240 (100%)  2:0 (0%)  3+:0 (0%)";

/// Where the pair is written, for a change to this area to be argued from.
///
/// Beside every other capture this project takes (`tools/verify`'s frame
/// artifacts, tooling.md §3). Resolved from the manifest directory rather than
/// from the working directory, because cargo runs an integration test with the
/// *crate* as its cwd and a relative path would leave a second `target/` inside
/// `crates/jidousha/`.
fn artifacts() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target/verify")
}

/// A scene with something recognisable in every corner and across the middle.
fn draw_the_scene(ctx: &mut DrawCtx) {
    ctx.rect(
        Rect::from_center_size(Vec2::ZERO, Vec2::new(24.0, 4.0)),
        Color::rgba(0.15, 0.35, 0.75, 1.0),
        Depth::layer(0),
    );
    for corner in [
        Vec2::new(-14.0, -7.0),
        Vec2::new(14.0, -7.0),
        Vec2::new(-14.0, 7.0),
        Vec2::new(14.0, 7.0),
    ] {
        ctx.circle(
            corner,
            2.0,
            Color::rgba(0.9, 0.7, 0.2, 1.0),
            Depth::layer(1),
        );
    }
    ctx.text(
        Vec2::new(-6.0, -1.0),
        "the game's own picture",
        TextStyle {
            size: 1.4,
            ..TextStyle::default()
        },
    );
}

/// The camera both captures are taken through.
fn set_the_camera(world: &mut World) {
    world.insert_resource(Camera {
        height: 20.0,
        clear_color: Color::rgba(0.06, 0.06, 0.09, 1.0),
        viewport: SIZE,
        ..Camera::default()
    });
}

/// A backend with a GPU behind it, or `None` on a machine that has none.
fn offscreen() -> Option<WgpuBackend> {
    let mut backend = WgpuBackend::offscreen(SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match backend.poll() {
            Ok(()) if backend.is_ready() => return Some(backend),
            Ok(()) => {}
            Err(error @ RenderError::NoAdapter { .. }) => {
                println!("no GPU on this machine, so the pixel half is skipped: {error}");
                return None;
            }
            Err(error) => {
                panic!("the GPU handshake failed, and not for want of an adapter: {error}")
            }
        }
    }
    println!("the GPU handshake never finished, so the pixel half is skipped");
    None
}

/// Render the scene once, with the overlay or without it, and read it back.
fn capture(backend: &mut WgpuBackend, textures: &TextureTable, with_overlay: bool) -> Vec<u8> {
    let mut simulation = headless(GameConfig::default(), |app| {
        app.add_system(Startup, set_the_camera);
        app.add_system(Draw, draw_the_scene);
    });
    simulation.tick();
    let camera = *simulation.world().resource::<Camera>();

    // The same composition the driver does: the game's submissions, then the
    // readout appended to a copy the world never sees (driver/frame.rs).
    let mut quads = simulation.draw().quads().to_vec();
    if with_overlay {
        draw_readout(&camera, READOUT, &mut quads);
    }
    let plan = plan_frame(&camera, &quads, textures);
    if let Err(error) = backend.render(&plan) {
        panic!("the offscreen target could not be drawn into: {error}");
    }
    let image = match backend.capture() {
        Ok(image) => image,
        Err(error) => panic!("an offscreen backend could not read itself back: {error}"),
    };
    let png = encode_png(&image);
    let name = if with_overlay {
        "overlay-on.png"
    } else {
        "overlay-off.png"
    };
    let directory = artifacts();
    let path = directory.join(name);
    if std::fs::create_dir_all(&directory).is_ok() && std::fs::write(&path, &png).is_ok() {
        println!("capture: {}", path.display());
    }
    image.rgba
}

#[test]
fn the_overlay_covers_its_own_corner_and_changes_no_pixel_outside_it() {
    // The whole of "off by default" and "presentation only", as pixels. If the
    // two captures differed anywhere but the corner the panel occupies, the
    // instrument would be changing the thing it is there to measure.
    let Some(mut backend) = offscreen() else {
        return;
    };
    let textures = create_builtin_textures(&mut backend);
    let plain = capture(&mut backend, &textures, false);
    let with_overlay = capture(&mut backend, &textures, true);
    assert_eq!(plain.len(), with_overlay.len());

    // Where the two pictures differ, in pixels rather than in bytes — the
    // question is "which part of the frame moved", and a byte index answers a
    // different one.
    let width = SIZE.width as usize;
    let mut differing = 0_usize;
    let mut worst = (0_usize, 0_usize);
    for (index, (was, now)) in plain
        .chunks_exact(4)
        .zip(with_overlay.chunks_exact(4))
        .enumerate()
    {
        if was != now {
            differing += 1;
            worst = worst.max((index / width, index % width));
        }
    }

    assert!(differing > 0, "the overlay was asked for and drew nothing");
    // A panel of five lines at `READOUT_LINES_ON_SCREEN` is a real share of the
    // frame; a handful of stray pixels would pass the check above while the
    // readout was in fact off screen or scaled to nothing.
    let share = differing as f32 / (plain.len() / 4) as f32;
    assert!(
        share > 0.01,
        "only {share} of the frame changed, which is not a readable panel"
    );
    // …and every one of those pixels is in the corner the panel is pinned to.
    // This is the presentation-only claim: the game's picture is where it was.
    assert!(
        worst.0 < SIZE.height as usize / 2 && worst.1 < width / 2,
        "the overlay changed a pixel at row {}, column {} — outside the corner it draws in",
        worst.0,
        worst.1
    );
}
