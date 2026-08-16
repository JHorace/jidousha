//! Golden images: the tier that keeps the *backend* honest (renderer.md §9).
//!
//! Transcripts keep render-core honest — they say what was submitted, exactly,
//! with no pixels to disagree about. Nothing above the seam can tell whether
//! wgpu drew any of it. This renders a fixed plan into an offscreen texture,
//! reads the pixels back, and compares them against a reference checked in
//! beside this file.
//!
//! **No adapter is not a failure.** Every runner this project has is headless,
//! and some have no graphics stack at all; a test that failed there would be
//! reporting on the machine rather than on the code. The tests below say so
//! loudly and return, and `tools/doctor` reports whether this tier can run at
//! all — a silently skipped test is the thing to avoid, not a skipped one.
//!
//! Reference images are blessed with `JIDOUSHA_BLESS=1 cargo test -p
//! jidousha-render-wgpu --test golden`, and the diff of the resulting PNG is
//! the review. Failures write `<name>-actual.png` and `<name>-diff.png` into
//! `target/verify/golden/` — the diff paints differing pixels magenta, so what
//! moved is the only bright thing in it.

use std::path::{Path, PathBuf};

use jidousha_core::Color;
use jidousha_core::math::{Mat4, Vec2};
use jidousha_render_core::{
    BackendTextureId, Batch, Camera, FramePlan, PhysicalSize, QuadVertex, RawImage, RenderBackend,
    Tolerance, compare, decode_png, diff_image, encode_png,
};
use jidousha_render_wgpu::WgpuBackend;

/// The size every golden image is taken at.
///
/// Small on purpose: a reference is a file in the repository, and 160×120 is
/// enough to see a sprite in the wrong place while staying under 10 KB.
const SIZE: PhysicalSize = PhysicalSize::new(160, 120);

/// How many polls to give the GPU handshake before calling it absent.
///
/// The backend is poll-based by design (ADR-0011, `init.rs`), and a test has no
/// frame loop to do the asking. On a machine with a working adapter this
/// resolves within a handful; anything needing thousands is a design problem
/// rather than a slow machine.
const HANDSHAKE_POLLS: usize = 10_000;

/// A backend with a GPU behind it, or `None` on a machine that has none.
fn offscreen() -> Option<WgpuBackend> {
    let mut backend = WgpuBackend::offscreen(SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match backend.poll() {
            Ok(()) if backend.is_ready() => return Some(backend),
            Ok(()) => {}
            Err(error) => {
                println!("no GPU on this machine, so the golden tier cannot run: {error}");
                return None;
            }
        }
    }
    println!("the GPU handshake never finished, so the golden tier cannot run");
    None
}

/// A quad, wound the way every quad in the engine is (ADR-0010, R3).
fn quad(min: Vec2, max: Vec2, color: Color, uv: [Vec2; 4]) -> Vec<QuadVertex> {
    let corners = [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];
    [0usize, 1, 2, 0, 2, 3]
        .into_iter()
        .map(|index| QuadVertex {
            position: corners[index],
            uv: uv[index],
            color,
        })
        .collect()
}

/// The whole-quad UVs, for a batch that samples one texture across each quad.
fn full_uv() -> [Vec2; 4] {
    [
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
    ]
}

/// A four-texel texture, one colour per corner.
///
/// Four texels rather than one, so the test can tell "the texture was sampled"
/// from "a flat colour was drawn": a single-texel texture looks identical to a
/// vertex colour, and would let a broken sampler pass.
fn corners_texture(backend: &mut WgpuBackend) -> BackendTextureId {
    #[rustfmt::skip]
    let texels: [u8; 16] = [
        255, 0, 0, 255,      0, 255, 0, 255,
        0, 0, 255, 255,      255, 255, 0, 255,
    ];
    backend.create_texture(
        &jidousha_render_core::TextureDesc {
            size: PhysicalSize::new(2, 2),
        },
        &texels,
    )
}

/// One white texel, for quads that carry only a colour.
fn white_texture(backend: &mut WgpuBackend) -> BackendTextureId {
    backend.create_texture(
        &jidousha_render_core::TextureDesc {
            size: PhysicalSize::new(1, 1),
        },
        &[255, 255, 255, 255],
    )
}

/// The scene every golden image in this file is of.
///
/// Fixed geometry and fixed colours: a golden reference is only worth having if
/// the picture is a function of the code and nothing else. Two batches, so the
/// reference also covers batch order — the textured quad is drawn second and
/// overlaps the flat one.
fn scene(view_projection: Mat4, flat: BackendTextureId, art: BackendTextureId) -> FramePlan {
    FramePlan {
        clear_color: Color::rgb(0.06, 0.08, 0.12),
        view_projection,
        batches: vec![
            Batch {
                texture: flat,
                vertices: quad(
                    Vec2::new(-8.0, -4.0),
                    Vec2::new(2.0, 3.0),
                    Color::rgb(0.9, 0.35, 0.2),
                    full_uv(),
                ),
            },
            Batch {
                texture: art,
                vertices: quad(
                    Vec2::new(-2.0, -1.0),
                    Vec2::new(7.0, 6.0),
                    Color::WHITE,
                    full_uv(),
                ),
            },
        ],
    }
}

/// A camera the size of the capture, so the picture fills it.
fn view_projection() -> Mat4 {
    Camera {
        height: 20.0,
        viewport: SIZE,
        ..Camera::default()
    }
    .view_projection()
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
}

fn artifact_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join("golden")
}

/// Compare `captured` against the reference called `name`, or bless it.
///
/// Blessing is opt-in through an environment variable rather than a fallback
/// when the file is missing: a reference that writes itself on first run would
/// turn every unexplained change into a new reference, which is the one way a
/// golden test can assert nothing at all.
fn check_against_reference(name: &str, captured: &RawImage) {
    let reference = golden_dir().join(format!("{name}.png"));
    if std::env::var_os("JIDOUSHA_BLESS").is_some() {
        let _ = std::fs::create_dir_all(golden_dir());
        let Ok(()) = std::fs::write(&reference, encode_png(captured)) else {
            panic!("could not write {}", reference.display());
        };
        println!("blessed {}", reference.display());
        return;
    }

    let Ok(bytes) = std::fs::read(&reference) else {
        panic!(
            "no reference image at {}\n  \
             likely cause: it has not been blessed yet, on any machine with a GPU\n  \
             fix: JIDOUSHA_BLESS=1 cargo test -p jidousha-render-wgpu --test golden, then \
             review the PNG in the diff before committing it",
            reference.display()
        );
    };
    let Ok(expected) = decode_png(&bytes) else {
        panic!("{} is not a readable PNG", reference.display());
    };

    let result = compare(&expected, captured, Tolerance::CLOSE_ENOUGH);
    if result.matched {
        return;
    }
    // Write what was actually drawn and where it differs, before failing: a
    // golden failure that leaves nothing to look at makes the reader re-run
    // the test by hand to find out what happened.
    let _ = std::fs::create_dir_all(artifact_dir());
    let actual_path = artifact_dir().join(format!("{name}-actual.png"));
    let _ = std::fs::write(&actual_path, encode_png(captured));
    let mut diff_path = String::from("(sizes differ, so there is no diff to draw)");
    if let Some(diff) = diff_image(&expected, captured, Tolerance::CLOSE_ENOUGH) {
        let path = artifact_dir().join(format!("{name}-diff.png"));
        let _ = std::fs::write(&path, encode_png(&diff));
        diff_path = path.display().to_string();
    }
    panic!(
        "the captured frame does not match {name}\n  {result}\n  \
         captured: {}\n  diff:     {diff_path}\n  \
         likely cause: the pipeline, the shader, or the colour conversion changed\n  \
         fix: look at the diff; if the change is intended, re-bless with JIDOUSHA_BLESS=1",
        actual_path.display(),
    );
}

/// The reference is compared on Linux only, and deliberately.
///
/// A reference image is a picture *some rasterizer* produced. CI blesses and
/// compares on lavapipe — Mesa's CPU rasterizer, which is deterministic and the
/// same on every runner — and a Direct3D or Metal device fills edge pixels
/// differently enough that a tolerance loose enough to accept it would be loose
/// enough to accept a real regression. Widening the tolerance until every
/// platform agrees is how a golden test stops asserting anything.
///
/// Every other test in this file runs everywhere, including on Windows: the
/// offscreen target, the capture path, the row unpadding, and the clear colour
/// are all checked on any machine with any adapter. What is Linux-only is the
/// comparison against a file.
#[cfg(target_os = "linux")]
#[test]
fn a_rendered_frame_matches_its_reference_image() {
    let Some(mut backend) = offscreen() else {
        return;
    };
    let flat = white_texture(&mut backend);
    let art = corners_texture(&mut backend);
    let plan = scene(view_projection(), flat, art);

    let Ok(()) = backend.render(&plan) else {
        panic!("an offscreen target is always available once the device is");
    };
    let Ok(captured) = backend.capture() else {
        panic!("an offscreen backend can read its own target back");
    };
    assert_eq!(captured.size, SIZE, "captured at the size asked for");
    assert_eq!(
        captured.rgba.len(),
        (SIZE.width * SIZE.height * 4) as usize,
        "the row padding was stripped"
    );
    check_against_reference("sprite_scene", &captured);
}

#[test]
fn the_same_plan_renders_the_same_pixels_twice() {
    // Exact, not tolerant: this compares one machine against itself, where any
    // difference is a real one. It is also what makes the reference above worth
    // blessing — a backend that drew slightly differently each frame would
    // produce a reference nobody could reproduce.
    let Some(mut backend) = offscreen() else {
        return;
    };
    let flat = white_texture(&mut backend);
    let art = corners_texture(&mut backend);
    let plan = scene(view_projection(), flat, art);

    let Ok(()) = backend.render(&plan) else {
        panic!("the device is ready");
    };
    let Ok(first) = backend.capture() else {
        panic!("an offscreen backend can read its own target back");
    };
    let Ok(()) = backend.render(&plan) else {
        panic!("the device is ready");
    };
    let Ok(second) = backend.capture() else {
        panic!("an offscreen backend can read its own target back");
    };
    let result = compare(&first, &second, Tolerance::EXACT);
    assert!(result.matched, "{result}");
}

#[test]
fn the_clear_color_reaches_the_corners() {
    // Independent of any reference file, and true from first principles: the
    // scene's quads do not reach the top-left corner, so that pixel is the
    // clear colour and nothing else. This is the assertion that still holds if
    // every reference image in the repository is wrong.
    let Some(mut backend) = offscreen() else {
        return;
    };
    let flat = white_texture(&mut backend);
    let art = corners_texture(&mut backend);
    let Ok(()) = backend.render(&scene(view_projection(), flat, art)) else {
        panic!("the device is ready");
    };
    let Ok(captured) = backend.capture() else {
        panic!("an offscreen backend can read its own target back");
    };

    // sRGB out, so the stored byte is the sRGB encoding of the linear value the
    // shader wrote — which is the number `Color::rgb` was given, since the
    // engine's colours are sRGB-encoded to begin with (conventions).
    let corner = &captured.rgba[0..4];
    let expect = [
        (0.06f32 * 255.0).round() as u8,
        (0.08f32 * 255.0).round() as u8,
        (0.12f32 * 255.0).round() as u8,
        255,
    ];
    for channel in 0..4 {
        assert!(
            corner[channel].abs_diff(expect[channel]) <= 2,
            "corner pixel {corner:?} is not the clear colour {expect:?}"
        );
    }
}

#[test]
fn a_backend_with_no_gpu_yet_refuses_to_capture_rather_than_inventing_pixels() {
    // Runs on every machine, adapter or not: a blank image handed back here
    // would let a golden test pass against nothing, which is the failure mode
    // the whole tier exists to avoid (renderer.md §9).
    let mut backend = WgpuBackend::offscreen(SIZE);
    let Err(error) = backend.capture() else {
        panic!("a backend that has not finished its handshake has no frame to give");
    };
    let text = error.to_string();
    assert!(text.starts_with("[jidousha] "), "{text}");
    assert!(text.contains("no GPU yet"), "{text}");
}
