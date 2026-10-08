//! The captured frame: one frame the check already recorded, rendered on a GPU
//! and written out as a PNG — `crates/jidousha/examples/prototype_kit/capture.rs`'s
//! path, less its art half (a game of shapes and text has none).
//!
//! A machine with no GPU is not a failure: the run says it skipped and the rest
//! of the verification stands.

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};
use std::path::{Path, PathBuf};

use crate::checks::{Checks, fail};

/// How big the captured artifact is: the window's 16:9 at three quarters of
/// its size. Big enough that 12-unit text is a legible 9 pixels — at the
/// example's 480x270 every row of the stack panel was a smear — and the same
/// shape as the recorder, which `verify` asserts.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(960, 540);

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

/// Render the recorded frame on a GPU and write it out as a PNG.
///
/// A machine with no GPU is not a failure. Every runner this project has is
/// headless and some have no graphics stack at all; the run says so and the
/// rest of the verification stands, exactly as the golden tests do.
pub(crate) fn capture_a_frame(
    checks: &mut Checks,
    frame: &FrameRecord,
    font: BackendTextureId,
) -> String {
    let window = crate::screen::WINDOW;
    checks.require(
        CAPTURE_SIZE.width * window.height == CAPTURE_SIZE.height * window.width,
        "the capture is not the window's shape",
        format!(
            "capture {}x{}, window {}x{}",
            CAPTURE_SIZE.width, CAPTURE_SIZE.height, window.width, window.height
        ),
    );
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

    // The built-in textures first, in the order the recorder created them, so
    // the ids inside the plan mean the same thing here — then the same art,
    // uploaded the same way, because this game has some and the plan names it.
    // A game of shapes and text: the built-ins are the whole texture table,
    // so there is no store to replay and nothing else to upload.
    let textures = create_builtin_textures(&mut gpu);

    // Checked rather than assumed, which is the whole load-bearing step: both
    // counters started empty and both were filled by the same calls in the same
    // order, so the font has to land on the id the recorder reported. If it
    // does not, every other id in the plan is wrong too and the picture is of
    // something else.
    checks.require(
        textures.resolve(FONT_TEXTURE) == font,
        "the replay's texture ids do not mean what the recorded plan means",
        format!(
            "the recorder put the font on {font:?} and this backend put it on {:?}; the plan \
             names ids, so a mismatch means the picture samples the wrong textures",
            textures.resolve(FONT_TEXTURE)
        ),
    );

    if let Err(error) = gpu.render(&frame.plan) {
        fail(
            "the GPU refused a plan the recorder had already accepted",
            &one_line(&error.to_string()),
        );
    }
    let Ok(image) = gpu.capture() else {
        fail(
            "the GPU rendered the frame and then would not hand it back",
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
        .join("stack_card_game_r2.png")
}
