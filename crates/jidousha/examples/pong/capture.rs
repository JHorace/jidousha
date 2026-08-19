//! The picture: the frame the check already recorded, rendered on a GPU and
//! written out as a PNG.
//!
//! `verify.rs` asserts on what was *submitted*; this renders one of those
//! frames for real and leaves something a person can look at, which is the one
//! question no assertion in this file's sibling can reach — whether it looks
//! like Pong.
//!
//! There is nothing to replay and nothing to restructure: a `FrameRecord`
//! carries the finished `FramePlan`, with the depth sort and the batching
//! already done, and a renderer built for the purpose will execute it.
//!
//! A machine with no GPU is not a failure. Every runner this project has is
//! headless and some have no graphics stack at all: the run says it skipped and
//! stays green. Every *other* handshake error is a fault, because reporting one
//! of those as "no GPU here" files a real problem as a property of the hardware
//! on every machine for ever.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    FrameRecord, RenderBackend, RenderError, WgpuBackend, create_builtin_textures, encode_png,
};

use crate::checks::{Checks, fail};

/// How big the captured picture is.
///
/// The **same 16:9 shape** as the recorder's viewport. The projection was
/// computed from that viewport and is baked into every plan, so a capture of
/// another shape stretches the picture while every assertion goes on passing —
/// none of them look at pixels.
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

/// Render `frame` and write it out. Returns the line the summary prints.
pub(super) fn capture_a_frame(checks: &mut Checks, frame: &FrameRecord) -> String {
    // The recorder's viewport and this must be the same shape, and it is worth
    // asserting rather than remembering.
    let recorded = crate::WINDOW;
    checks.require(
        (CAPTURE_SIZE.aspect() - recorded.aspect()).abs() < 1e-4,
        "the capture is not the shape the frame was planned for",
        format!(
            "the recorder's viewport is {}x{} and the capture is {}x{}; the projection is baked \
             into the plan, so a capture of another shape is a stretched picture that every \
             assertion still passes",
            recorded.width, recorded.height, CAPTURE_SIZE.width, CAPTURE_SIZE.height
        ),
    );

    let mut gpu = WgpuBackend::offscreen(CAPTURE_SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
            // No adapter is a fact about the machine, not a failure.
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

    // The built-in textures, in the order the recorder created them, so the ids
    // inside the plan mean the same thing to this renderer. This game loads no
    // assets, so there is nothing else to upload and the table it returns is
    // not needed.
    let _ = create_builtin_textures(&mut gpu);
    if let Err(error) = gpu.render(&frame.plan) {
        checks.require(
            false,
            "the GPU refused a plan the recorder had already accepted",
            one_line(&error.to_string()),
        );
        return format!(
            "skipped, the frame would not render ({})",
            one_line(&error.to_string())
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
    // `tools/verify` takes the first line whose text starts with `capture:` and
    // contains ` written to `, so this wording is load-bearing.
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
