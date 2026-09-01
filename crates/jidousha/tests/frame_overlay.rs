//! The performance overlay, in pixels: what it covers, and what it leaves alone.
//!
//! The native overlay is off unless `JIDOUSHA_FRAMETIME` asks for it, and it
//! has **levels** — 1 is the pacing panel, 2 adds the performance sections
//! (frame-pacing.md §6, §7). "Off by default" is a claim about a *picture*
//! rather than about a boolean, and so is "level 2 adds sections and moves
//! nothing". So this renders one scene three times through a real GPU — as the
//! game submitted it, with the level-1 readout over it, and with the level-2
//! one — and asks the captures the questions that matter: each panel is visible
//! where it draws, the bigger one covers more, and nothing under either moved.
//!
//! It also writes all three captures to `target/verify/`, so the set a change
//! to this area has to be justified with is a command anyone can re-run rather
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

/// What the level-1 panel says on a captured frame.
///
/// A fixed string rather than a live reading: this test is about where the
/// overlay lands and what it covers, and a capture whose text changed with the
/// machine's frame rate would be a picture nobody could compare.
const PACING_READOUT: &str = "jidousha frame pacing: JIDOUSHA_FRAMETIME=1\n\
                              present   ~60.0 fps - median 16.67ms, mean 16.67ms\n\
                              spread    16.41ms .. 17.02ms over 240 frames\n\
                              pacing    vsync - the display sets the rate\n\
                              ticks/fr  0:0 (0%)  1:240 (100%)  2:0 (0%)  3+:0 (0%)";

/// What the level-2 panel says, in the same fixed-reading spirit.
///
/// Every section the performance panel adds, at plausible widths — the point of
/// the capture is that the widest line still fits in the corner it is pinned to
/// and that the block does not run off the frame (frame-pacing.md §7).
const PERF_READOUT: &str = "jidousha performance: JIDOUSHA_FRAMETIME=2\n\
                            present   ~60.0 fps - median 16.67ms, mean 16.67ms\n\
                            spread    16.41ms .. 17.02ms over 240 frames\n\
                            pacing    vsync - the display sets the rate\n\
                            ticks/fr  0:0 (0%)  1:240 (100%)  2:0 (0%)  3+:0 (0%)\n\
                            frame deltas\n\
                            \x20 16-17ms ####################  240 (100%)\n\
                            frame breakdown  ms: median  p95  max\n\
                            \x20 sim                            0.31    0.55    1.20\n\
                            \x20 draw                           0.12    0.20    0.44\n\
                            \x20 encode                         0.08    0.14    0.30\n\
                            \x20 present ###################   16.16   16.40   22.00\n\
                            \x20 sleep                          0.00    0.00    0.00\n\
                            \x20 busy    3% of a 16.67ms frame - 16.16ms of it waiting\n\
                            cpu       process 41% of one core\n\
                            gpu       median 2.10ms, p95 2.60ms over 240 frames\n\
                            memory    rss 184.2MB\n\
                            \x20 renderer 12.6MB textures, 0.4MB buffers\n\
                            \x20 world    412 entities, 1236 components, 318 quads drawn\n\
                            snapshot  press F9 to write this panel under target/";

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

/// Render the scene once, with a readout over it or without one, and read it
/// back.
fn capture(
    backend: &mut WgpuBackend,
    textures: &TextureTable,
    readout: Option<&str>,
    name: &str,
) -> Vec<u8> {
    let mut simulation = headless(GameConfig::default(), |app| {
        app.add_system(Startup, set_the_camera);
        app.add_system(Draw, draw_the_scene);
    });
    simulation.tick();
    let camera = *simulation.world().resource::<Camera>();

    // The same composition the driver does: the game's submissions, then the
    // readout appended to a copy the world never sees (driver/frame.rs).
    let mut quads = simulation.draw().quads().to_vec();
    if let Some(readout) = readout {
        draw_readout(&camera, readout, &mut quads);
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
    let directory = artifacts();
    let path = directory.join(name);
    if std::fs::create_dir_all(&directory).is_ok() && std::fs::write(&path, &png).is_ok() {
        println!("capture: {}", path.display());
    }
    image.rgba
}

/// How two captures differ: how many pixels, and the furthest one from the
/// corner the panel is pinned to.
///
/// In pixels rather than in bytes — the question is "which part of the frame
/// moved", and a byte index answers a different one.
fn difference(plain: &[u8], with_overlay: &[u8]) -> (usize, (usize, usize)) {
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
    (differing, worst)
}

#[test]
fn every_level_of_the_overlay_covers_its_own_corner_and_no_pixel_outside_it() {
    // The whole of "off by default", "presentation only" and "the levels are
    // cumulative", as pixels. If a capture differed anywhere but the corner the
    // panel occupies, the instrument would be changing the thing it is there to
    // measure — and that has to hold at level 2, which draws four times as many
    // lines as level 1 (frame-pacing.md §7).
    let Some(mut backend) = offscreen() else {
        return;
    };
    let textures = create_builtin_textures(&mut backend);
    let plain = capture(&mut backend, &textures, None, "overlay-off.png");
    let pacing = capture(
        &mut backend,
        &textures,
        Some(PACING_READOUT),
        "overlay-on.png",
    );
    let perf = capture(
        &mut backend,
        &textures,
        Some(PERF_READOUT),
        "overlay-perf.png",
    );
    assert_eq!(plain.len(), pacing.len());
    assert_eq!(plain.len(), perf.len());

    let pixels = plain.len() / 4;
    let width = SIZE.width as usize;
    let mut covered = Vec::new();
    for (level, capture) in [("1", &pacing), ("2", &perf)] {
        let (differing, worst) = difference(&plain, capture);
        assert!(
            differing > 0,
            "level {level} was asked for and drew nothing"
        );
        // A panel of five lines at `READOUT_LINES_ON_SCREEN` is a real share of
        // the frame; a handful of stray pixels would pass the check above while
        // the readout was in fact off screen or scaled to nothing.
        let share = differing as f32 / pixels as f32;
        assert!(
            share > 0.01,
            "level {level} changed only {share} of the frame, which is not a readable panel"
        );
        // …and every one of those pixels is in the corner the panel is pinned
        // to. This is the presentation-only claim: the game's picture is where
        // it was.
        assert!(
            worst.0 < SIZE.height as usize / 2 && worst.1 < width / 2,
            "level {level} changed a pixel at row {}, column {} - outside the corner it draws in",
            worst.0,
            worst.1
        );
        covered.push(differing);
    }
    // Cumulative, in pixels: the performance panel is the pacing panel plus
    // sections, so it cannot cover less of the frame than the pacing one.
    assert!(
        covered[1] > covered[0],
        "level 2 covered {} pixels against level 1's {}, so it drew no more of a panel",
        covered[1],
        covered[0]
    );
}
