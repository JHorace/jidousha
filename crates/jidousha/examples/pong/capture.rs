//! The picture: the frame the check already recorded, rendered on a GPU and
//! written out as a PNG.
//!
//! No second session and no game to re-run. A `FrameRecord` carries the
//! finished `FramePlan` — the depth sort and the batching already done — and a
//! renderer built for the purpose executes it. This game loads no assets, so
//! the built-in textures are the whole table and the ids inside the plan mean
//! the same thing here because both counters started empty and both were
//! filled by the same call.
//!
//! A machine with no GPU is a fact about the machine rather than a failure, and
//! every other handshake error is a fault: calling one of those "no GPU here"
//! files a real problem as a property of the hardware, on every machine, for
//! ever.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};

use crate::checks::{Checks, fail};

/// How big the captured picture is.
///
/// The **same 16:9 shape** the recorder's viewport has. The projection was
/// computed from that viewport and is baked into every plan; nothing
/// downstream can recompute it, so a capture of another shape stretches the
/// picture while every assertion goes on passing.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(480, 270);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// An engine message flattened onto one line.
///
/// `RenderError`'s `Display` is the four-part shape, which is right on its own
/// and wrong inside a `--verify` summary, where the convention is a verdict
/// line and then one indented line per fact.
fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render `frame` on a GPU and write it out, returning the line `tools/verify`
/// reads.
pub(crate) fn capture_a_frame(
    checks: &mut Checks,
    frame: &FrameRecord,
    font: BackendTextureId,
) -> String {
    let mut gpu = WgpuBackend::offscreen(CAPTURE_SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
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

    // The built-in textures, in the order the recorder created them. A game of
    // shapes and text needs nothing else, and the table this returns is unused.
    let textures = create_builtin_textures(&mut gpu);
    // Checked rather than assumed. If the font is not where the recorder put
    // it, every other id in the plan is wrong too and the picture is of
    // something else.
    checks.require(
        textures.resolve(FONT_TEXTURE) == font,
        "the replay's texture ids do not mean what the recorded plan means",
        format!(
            "the recorder put the font on {font:?} and this backend put it on {:?}",
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
    checks.require(
        image.size.width * CAPTURE_SIZE.height == image.size.height * CAPTURE_SIZE.width,
        "the capture is not the aspect the plan was projected for",
        format!(
            "{}x{} against a recorder viewport of {}x{}; nothing downstream can recompute the \
             projection, so a capture of another shape is a stretched picture that every \
             assertion still passes",
            image.size.width, image.size.height, CAPTURE_SIZE.width, CAPTURE_SIZE.height,
        ),
    );

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
    // `tools/verify` takes the first line starting `capture:` that contains
    // " written to " and puts what follows into its report. Worded differently
    // the run still passes and the report says no picture was taken.
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
