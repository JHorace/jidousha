//! The captured frame: the same game, rendered on a GPU, written out as a PNG.
//!
//! R4's artifact. `verify.rs` asserts on what was *submitted*; this renders it
//! for real and leaves a picture behind, which is the half a person can look at
//! and the half that would catch a backend drawing nothing at all.
//!
//! A machine with no GPU is not a failure. Every runner this project has is
//! headless and some have no graphics stack at all; the run says so and the
//! rest of the verification stands, exactly as the golden tests do
//! (renderer.md §9).

use jidousha::testing::{PhysicalSize, RenderBackend, WgpuBackend, encode_png};
use std::path::{Path, PathBuf};

use crate::verify::{fail, play};

/// How big the captured artifact is.
///
/// Small enough to be cheap to write every run, big enough to see what the game
/// looks like — and the **same 16:9 shape** the window uses. A capture at
/// another aspect is a picture of a different framing: the field is twenty
/// world units tall whatever the window is, so a 4:3 capture crops the left
/// paddle out entirely and the artifact stops being a picture of the game.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(480, 270);

/// How many polls to give the GPU handshake before calling it absent.
///
/// The backend is poll-based by design (ADR-0011); a verify run has no frame
/// loop, so it does the asking itself.
const HANDSHAKE_POLLS: usize = 10_000;

/// Render the same session on a GPU and write the last frame out as a PNG.
///
/// A second run of the same scripted game, through the wgpu backend instead of
/// the null one. Two things come out of it: the artifact a person or an agent
/// can look at, and the assertion that the *world* did the same thing both
/// times — which is renderer.md §1's contract that everything above the seam is
/// backend-agnostic, checked rather than asserted.
///
/// A machine with no GPU is not a failure. Every runner this project has is
/// headless and some have no graphics stack at all; the run says so and the
/// rest of the verification stands, exactly as the golden tests do.
pub(super) fn capture_a_frame(expected_track: &[f32]) -> String {
    let mut gpu = WgpuBackend::offscreen(CAPTURE_SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
            Err(error) => return format!("skipped, no GPU on this machine ({error})"),
        }
    }
    if !gpu.is_ready() {
        return "skipped, the GPU handshake never finished".to_owned();
    }

    let run = play(&mut gpu, CAPTURE_SIZE);
    if run.paddle_track != expected_track {
        fail(
            "the same game did different things on two backends",
            "everything above the backend seam is backend-agnostic (renderer.md §1), so a \
             world that depends on which backend drew it is a layering bug",
        );
    }

    let Ok(image) = gpu.capture() else {
        fail(
            "the GPU rendered the session and then would not hand the frame back",
            "an offscreen backend can always read its own target",
        );
    };
    let path = artifact_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&path, encode_png(&image)).is_err() {
        fail(
            "the captured frame could not be written",
            &format!("tried to write {}", path.display()),
        );
    }
    let shown = std::fs::canonicalize(&path).unwrap_or(path);
    format!(
        "{}x{} written to {}",
        image.size.width,
        image.size.height,
        shown.display()
    )
}

/// Where the captured frame is written.
fn artifact_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join("prototype_kit.png")
}
