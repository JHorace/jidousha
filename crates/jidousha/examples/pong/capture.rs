//! The picture: the frame the check already recorded, rendered on a GPU and
//! written out as a PNG.
//!
//! `verify.rs` asserts on what was *submitted*; this renders one of those
//! frames for real and leaves something a person can look at, which is the half
//! no assertion in that file reaches — whether it looks like Pong.
//!
//! There is no second session and no game to re-run: a `FrameRecord` carries
//! the finished `FramePlan`, with the depth sort and the batching already done,
//! and a renderer built for the purpose executes it. This game loads no art at
//! all, so the built-in textures are the whole table and the ids inside the
//! plan mean the same thing here as they did in the recorder — both counters
//! start empty and are filled by the same call.
//!
//! A machine with no GPU is not a failure. Every runner is headless and some
//! have no graphics stack; the run says it skipped and stays green. Every
//! *other* handshake error is a fault, and reporting one of those as "no GPU
//! here" files a real problem as a property of the hardware, for ever.

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};
use std::path::{Path, PathBuf};

use crate::checks::{Checks, fail};

/// How big the captured artifact is.
///
/// The **same 16:9 shape** the recorder drew at. The projection was computed
/// from that viewport and is baked into every plan; nothing downstream can
/// recompute it, so a capture of another shape stretches the picture while
/// every assertion goes on passing, because none of them look at pixels.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(480, 270);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// An engine message flattened onto one line.
///
/// `RenderError`'s `Display` is the four-part shape, which is right when it is
/// the only thing on the screen and wrong inside a `--verify` summary, where
/// the convention is one indented line per fact.
fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render the recorded frame on a GPU and write it out as a PNG.
///
/// The line it returns is printed as `capture: ...`, and `tools/verify` lifts
/// the path out of it by looking for a line starting `capture:` that contains
/// ` written to `.
pub(super) fn capture_a_frame(
    checks: &mut Checks,
    frame: &FrameRecord,
    font: BackendTextureId,
) -> String {
    // Asserted rather than remembered: the plan's projection came from the
    // recorder's viewport, so a capture of another shape is a picture of a
    // different game.
    let recorder_aspect = crate::verify::viewport().aspect();
    if !(CAPTURE_SIZE.aspect() - recorder_aspect).abs().lt(&0.001) {
        checks.require(
            false,
            "the capture is not the shape the frame was planned for",
            format!(
                "the recorder drew at {recorder_aspect:.4} and this captures at {:.4}; the \
                 projection is baked into the plan and nothing downstream can recompute it",
                CAPTURE_SIZE.aspect(),
            ),
        );
    }

    let mut gpu = WgpuBackend::offscreen(CAPTURE_SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
            // A fact about the machine, not a failure.
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

    // The built-ins are the whole table for a game of shapes and text. Checked
    // rather than assumed: if the font did not land on the id the recorder
    // reported, every other id in the plan is wrong too and the picture is of
    // something else.
    let textures = create_builtin_textures(&mut gpu);
    checks.require(
        textures.resolve(FONT_TEXTURE) == font,
        "the replay's texture ids do not mean what the recorded plan means",
        format!(
            "the recorder put the font on {font:?} and this backend put it on {:?}; the \
             plan names ids, so a mismatch means the picture samples the wrong textures",
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
