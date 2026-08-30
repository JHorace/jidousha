//! What the backend reports about itself: bytes held, and GPU time per frame.
//!
//! The accounting seam (`RenderBackend::stats`) and the optional
//! `TIMESTAMP_QUERY` feature behind it, checked on a real device. Two claims
//! matter more than the numbers:
//!
//! - **device creation never narrows.** The feature is asked for as an
//!   intersection with what the adapter already offers, so a machine without
//!   timestamps creates its device exactly as it did before and answers
//!   `gpu n/a` (renderer.md §12a);
//! - **the byte totals are the backend's own running totals**, so they move
//!   when a texture is created and move back when it is destroyed.
//!
//! **No adapter is not a failure**, exactly as in `golden.rs` and for the same
//! reason: every runner this project has is headless and some have no graphics
//! stack at all (renderer.md §9). What is not allowed is skipping in silence,
//! so it prints why.

use jidousha_render_core::{
    Camera, FramePlan, PhysicalSize, RenderBackend, RenderError, TextureDesc, plan_frame,
};
use jidousha_render_wgpu::WgpuBackend;

/// How big the offscreen target is. Small: nothing here looks at a pixel.
const SIZE: PhysicalSize = PhysicalSize::new(64, 64);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// How many frames to draw before asking for a GPU reading.
///
/// The resolve is mapped asynchronously and read on a later frame, so a reading
/// is a frame or two old by design (timing.rs). Twenty is far more than enough
/// and still finishes instantly.
const FRAMES: usize = 20;

/// A backend with a GPU behind it, or `None` on a machine that has none.
fn offscreen() -> Option<WgpuBackend> {
    let mut backend = WgpuBackend::offscreen(SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match backend.poll() {
            Ok(()) if backend.is_ready() => return Some(backend),
            Ok(()) => {}
            Err(error @ RenderError::NoAdapter { .. }) => {
                println!("no GPU on this machine, so this file is skipped: {error}");
                return None;
            }
            Err(error) => {
                panic!("the GPU handshake failed, and not for want of an adapter: {error}")
            }
        }
    }
    println!("the GPU handshake never finished, so this file is skipped");
    None
}

/// A texture of `side` square, filled with opaque white.
fn texture(backend: &mut WgpuBackend, side: u32) -> jidousha_render_core::BackendTextureId {
    let desc = TextureDesc {
        size: PhysicalSize::new(side, side),
    };
    backend.create_texture(&desc, &vec![255; (side * side * 4) as usize])
}

/// A plan that draws one quad through the built-in white texel.
fn one_quad(backend: &mut WgpuBackend) -> FramePlan {
    let white = texture(backend, 1);
    let placeholder = texture(backend, 1);
    let mut table = jidousha_render_core::TextureTable::new(white, placeholder);
    table.register(jidousha_core::TextureId::WHITE, white);
    let camera = Camera {
        viewport: SIZE,
        ..Camera::default()
    };
    let quads = vec![jidousha_core::Quad {
        corners: [
            jidousha_core::math::Vec2::new(-1.0, -1.0),
            jidousha_core::math::Vec2::new(1.0, -1.0),
            jidousha_core::math::Vec2::new(1.0, 1.0),
            jidousha_core::math::Vec2::new(-1.0, 1.0),
        ],
        uvs: [
            jidousha_core::math::Vec2::ZERO,
            jidousha_core::math::Vec2::X,
            jidousha_core::math::Vec2::ONE,
            jidousha_core::math::Vec2::Y,
        ],
        texture: jidousha_core::TextureId::WHITE,
        tint: jidousha_core::Color::WHITE,
        depth: jidousha_core::Depth::layer(0),
    }];
    plan_frame(&camera, &quads, &table)
}

#[test]
fn a_device_is_created_whether_or_not_this_machine_offers_timestamp_queries() {
    // The claim that makes the optional feature safe to ask for at all: this
    // machine either has timestamps or has not, and either way the handshake
    // completes and frames draw. A `required_features` that named the feature
    // unconditionally would turn every machine without it into "no GPU".
    let Some(mut backend) = offscreen() else {
        return;
    };
    assert!(backend.is_ready(), "the device arrived");
    let plan = one_quad(&mut backend);
    for _ in 0..FRAMES {
        let Ok(()) = backend.render(&plan) else {
            panic!("an offscreen target is always available once the device is");
        };
    }
    // And the reading is either a number or an honest absence — never a zero
    // standing in for one (timing.rs).
    match backend.stats().gpu_frame {
        Some(gpu) => {
            println!(
                "this device offers timestamps: {:.3}ms",
                gpu.as_f32() * 1000.0
            );
            assert!(
                gpu.as_f32() > 0.0 && gpu.as_f32() < 1.0,
                "a 64x64 clear took {}s, which is not a frame time",
                gpu.as_f32()
            );
        }
        None => println!("this device offers no timestamp queries, so the panel says `gpu n/a`"),
    }
}

#[test]
fn the_accounting_moves_when_a_texture_is_created_and_moves_back_when_it_is_destroyed() {
    // The actionable memory tier, on the backend that actually holds the bytes.
    // RGBA8 and nothing else (renderer.md §3), so a 64-square texture is
    // exactly 16KiB — a shipped literal rather than the expression the code
    // under test uses, because a check that recomputed the formula would agree
    // with a wrong formula.
    let Some(mut backend) = offscreen() else {
        return;
    };
    let before = backend.stats().texture_bytes;
    let id = texture(&mut backend, 64);
    assert_eq!(
        backend.stats().texture_bytes - before,
        16_384,
        "a 64x64 RGBA8 texture is 16KiB"
    );
    backend.destroy_texture(id);
    assert_eq!(
        backend.stats().texture_bytes,
        before,
        "the bytes came back when the texture went"
    );
    backend.destroy_texture(id);
    assert_eq!(
        backend.stats().texture_bytes,
        before,
        "a second destroy of the same id cannot drive the total below zero"
    );
}

#[test]
fn the_buffers_a_frame_needs_are_accounted_for_once_there_is_a_device() {
    // The other half of the renderer's tier: the vertex buffer grows with the
    // busiest frame the run has had, and a total that never moved would be a
    // panel that could not see a Draw system accumulating quads.
    let Some(mut backend) = offscreen() else {
        return;
    };
    let plan = one_quad(&mut backend);
    let Ok(()) = backend.render(&plan) else {
        panic!("an offscreen target is always available once the device is");
    };
    assert!(
        backend.stats().buffer_bytes > 0,
        "a pipeline with a vertex buffer and a camera uniform holds no bytes"
    );
}
