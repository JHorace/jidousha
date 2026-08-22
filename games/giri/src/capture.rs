//! The captured frame: one frame the check already recorded, rendered on a GPU
//! and written out as a PNG.
//!
//! `verify.rs` asserts on what was *submitted*; this renders one of those
//! frames for real and leaves a picture behind - the half a person can look at,
//! and the half that would catch a frame every assertion was happy with.
//!
//! **giri has no art, so this is the short path.** `create_builtin_textures` is
//! the whole texture table for a game of quads and text: the flat white every
//! untextured quad is tinted out of, the magenta placeholder, and the font
//! atlas. There is no `Assets` store to rebuild and no upload to replay - the
//! id check after it still applies, because a plan whose ids drifted renders
//! the wrong texture into a picture nothing else looks at.
//!
//! A machine with no GPU is not a failure: every runner this project has is
//! headless and some have no graphics stack at all.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};

use crate::checks::{Checks, fail};

/// How big the captured artifact is - the **same 16:9 shape** the window uses.
///
/// A capture at another aspect stretches the picture while every assertion goes
/// on passing, because none of them look at pixels.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(480, 270);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// An engine message flattened onto one line, for a one-line summary.
fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render the recorded frame on a GPU and write it out as a PNG.
pub fn capture_a_frame(checks: &mut Checks, frame: &FrameRecord, font: BackendTextureId) -> String {
    let mut gpu = WgpuBackend::offscreen(CAPTURE_SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
            // `NoAdapter` is a fact about the machine, and every other error is
            // a fault: reporting one of those as "no GPU here" files a real
            // problem as a property of the hardware, for ever.
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

    let textures = create_builtin_textures(&mut gpu);
    // Checked rather than assumed: both counters started empty and were filled
    // by the same call, so the font has to land on the id the recorder
    // reported. If it does not, every id in the plan is wrong and the picture
    // is of something else.
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
        .join("giri.png")
}
