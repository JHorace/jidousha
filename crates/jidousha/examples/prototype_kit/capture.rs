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

use jidousha::prelude::*;
use jidousha::testing::{RenderBackend, RenderError, WgpuBackend, encode_png};
use std::path::{Path, PathBuf};

use crate::checks::{Checks, fail};
use crate::verify::play;

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

/// An engine message flattened onto one line.
///
/// `RenderError`'s `Display` is the four-part shape, which is right when it is
/// the only thing on the screen and wrong inside a `--verify` summary: the
/// convention there is a verdict line and then one indented line per fact
/// (`tools/verify` prints exactly that block), and a four-line value turns the
/// summary into three lines of somebody else's paragraph. Every word is kept —
/// a machine with no GPU is precisely where the detail is worth having.
fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

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
pub(super) fn capture_a_frame(checks: &mut Checks, expected_track: &[f32]) -> String {
    let mut gpu = WgpuBackend::offscreen(CAPTURE_SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
            // Two different things, and only the first is a fact about the
            // machine. `NoAdapter` means there is no GPU here, which every
            // headless runner reports and which the transcript tier does not
            // need — the run stays green and says it skipped (renderer.md §9).
            // Anything else is a fault, and calling one of those "no GPU on
            // this machine" files an engine bug as a property of the hardware.
            Err(error @ RenderError::NoAdapter { .. }) => {
                return format!(
                    "skipped, no GPU on this machine ({})",
                    one_line(&error.to_string())
                );
            }
            Err(error) => {
                checks.require(
                    false,
                    "the GPU handshake failed, and not because the machine has no GPU",
                    format!(
                        "an adapter was found and the handshake still did not finish: {}",
                        one_line(&error.to_string())
                    ),
                );
                return format!(
                    "skipped, the GPU handshake failed ({})",
                    one_line(&error.to_string())
                );
            }
        }
    }
    if !gpu.is_ready() {
        return "skipped, the GPU handshake never finished".to_owned();
    }

    let run = play(&mut gpu, CAPTURE_SIZE);
    // A reading, not a reason to stop: the capture below is still worth taking,
    // and a run that reports this alongside whatever else went wrong is more
    // use than one that reports it alone.
    checks.require(
        run.paddle_track == expected_track,
        "the same game did different things on two backends",
        format!(
            "the paddle ended at {:?} through the GPU and {:?} through the null backend; \
             everything above the backend seam is backend-agnostic (renderer.md §1), so a \
             world that depends on which backend drew it is a layering bug",
            run.paddle_track.last(),
            expected_track.last(),
        ),
    );

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
