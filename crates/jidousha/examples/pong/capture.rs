//! The captured frame: the one the check already recorded, rendered on a GPU
//! and written out as a PNG.
//!
//! `verify.rs` asserts on what was *submitted*; this renders one of those
//! frames for real and leaves a picture behind. It is the only instrument in
//! this surface that answers "does it look like the game", and the only one
//! that would catch a backend drawing nothing at all.
//!
//! There is no second session and no game to re-run: a `FrameRecord` carries
//! the finished `FramePlan`, with the depth sort and the batching already done,
//! and a renderer built for the purpose executes it.
//!
//! This game draws no art, so the only textures the plan can name are the
//! built-in ones — the flat white every shape samples and the font atlas every
//! glyph does. That makes the replay's texture table three lines rather than
//! the asset dance a game with art needs; what does not change is that the ids
//! are **checked** rather than assumed, because a plan whose ids drifted
//! renders the wrong texture into a picture every other check is happy with.
//!
//! A machine with no GPU is not a failure — every runner here is headless and
//! some have no graphics stack at all — but it is not silent either: the run
//! says it skipped, and every *other* handshake error is reported as the fault
//! it is rather than filed as a property of the hardware.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};

use crate::checks::{Checks, fail, near_within};

/// How big the captured picture is.
///
/// The **same 16:9 shape** the window and the recorder use. A capture at
/// another aspect is a picture of a different framing — the court is twenty
/// world units tall whatever the surface is, so a 4:3 capture crops both
/// paddles off the sides — and every assertion in `verify.rs` goes on passing,
/// because none of them look at pixels.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(480, 270);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// An engine message flattened onto one line.
///
/// `RenderError`'s `Display` is the four-part shape, which is right on its own
/// and wrong inside a `--verify` summary, where the convention is one indented
/// line per fact. Every word is kept: a machine with no GPU is precisely where
/// the detail is worth having.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render the recorded frame on a GPU and write it out as a PNG.
///
/// The returned string is what the run prints after `capture: `. The wording
/// matters: `tools/verify` takes the first line starting with `capture:` that
/// also contains ` written to `, so a differently worded success is a run that
/// passes while the report says no picture was taken.
pub(crate) fn capture_a_frame(
    checks: &mut Checks,
    frame: &FrameRecord,
    font: BackendTextureId,
) -> String {
    // Asserted rather than remembered: nothing downstream can recompute the
    // recorder's aspect from the picture, so a capture at the wrong shape
    // stretches the game silently.
    checks.require(
        near_within(CAPTURE_SIZE.aspect(), crate::WINDOW.aspect(), 0.001),
        "the capture is not the shape the game is drawn at",
        format!(
            "capturing {}x{} ({:.4}) from a game framed at {}x{} ({:.4}); the picture would \
             be stretched and no assertion over quads could see it",
            CAPTURE_SIZE.width,
            CAPTURE_SIZE.height,
            CAPTURE_SIZE.aspect(),
            crate::WINDOW.width,
            crate::WINDOW.height,
            crate::WINDOW.aspect(),
        ),
    );

    let mut gpu = WgpuBackend::offscreen(CAPTURE_SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
            // Two different things, and only the first is a fact about the
            // machine. Reporting any other error as "no GPU here" files a real
            // problem as a property of the hardware, on every machine, for ever.
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

    // The built-ins, created in the order the recorder created them, so the ids
    // inside the plan mean the same thing here. This game loads nothing, so
    // that is the whole texture table.
    let textures = create_builtin_textures(&mut gpu);
    // Checked rather than assumed: both tables started empty and both were
    // filled by the same call, so the font has to land on the id the recorder
    // reported. If it does not, every other id in the plan is wrong too and the
    // picture is of something else.
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
        .join("pong.png")
}
