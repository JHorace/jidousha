//! The captured frame: the frame `verify.rs` already recorded, rendered on a
//! GPU and written out as a PNG.
//!
//! `docs/api/jidousha-capture.md`: a game of pure shapes and text has no asset
//! half, so this is the short path — `create_builtin_textures` is the whole
//! texture table (white, placeholder, font), there is no store to replay and no
//! `upload_ready_textures` to call. Four things still hold: `tools/verify` reads
//! exactly one `capture: ... written to ...` line; the capture is at the
//! recorder's aspect; the font id is checked before the PNG is believed; and a
//! machine with no GPU passes while a broken one does not.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};

use crate::checks::{Checks, fail};

/// How big the captured artifact is — the **same 9:16 shape** as `game::WINDOW`
/// (540x960). A capture at another aspect would stretch the picture while every
/// assertion went on passing.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(360, 640);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// An engine message flattened onto one line, for a `--verify` summary.
fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render `frame` on a GPU and write it out as a PNG. Returns the summary line
/// (`verify.rs` prints it after `capture: `).
pub fn capture_a_frame(checks: &mut Checks, frame: &FrameRecord, font: BackendTextureId) -> String {
    // The aspect is a contract with the recorder's viewport, not a thing to
    // remember — assert it rather than trusting the two literals stay in step.
    checks.require(
        (CAPTURE_SIZE.aspect() - crate::WINDOW.aspect()).abs() < 1e-4,
        "the capture is not the shape of the game's window",
        format!(
            "capture {}x{} is aspect {:.4}; the window is {:.4}",
            CAPTURE_SIZE.width,
            CAPTURE_SIZE.height,
            CAPTURE_SIZE.aspect(),
            crate::WINDOW.aspect(),
        ),
    );

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

    // Pure shapes and text: the three built-ins are the whole table.
    let textures = create_builtin_textures(&mut gpu);
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
        .join("fat-orange-man.png")
}
