//! The captured frame: the day-3 nemesis card over the queue, rendered on a GPU
//! and written to `target/verify/restaurant_nemesis_r3v2.png` (DESIGN.md G10).
//!
//! Shapes and text only, so the built-in textures are the whole texture
//! table. No GPU is a skip that says so; any other handshake error fails.
//!
//! Key function: `capture_a_frame`.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    FONT_TEXTURE, FrameRecord, FrameRecorder, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};

use crate::WINDOW;
use crate::checks::Checks;

/// 960 x 540: the window's 16:9, large enough to read 12-unit rows.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(960, 540);
const HANDSHAKE_POLLS: usize = 10_000;

fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render `frame` and write it out; the text after `capture:`.
pub(crate) fn capture_a_frame(checks: &mut Checks, frame: &FrameRecord) -> String {
    checks.require(
        CAPTURE_SIZE.width * WINDOW.height == CAPTURE_SIZE.height * WINDOW.width,
        "the capture is not the window's shape",
        format!("{CAPTURE_SIZE:?} against {WINDOW:?}"),
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
                    one_line(&error.to_string()),
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
    let font = FrameRecorder::new(WINDOW).font_texture();
    checks.require(
        textures.resolve(FONT_TEXTURE) == font,
        "the replay's texture ids do not mean what the recorded plan means",
        format!(
            "recorder font {font:?}, backend font {:?}",
            textures.resolve(FONT_TEXTURE)
        ),
    );
    if let Err(error) = gpu.render(&frame.plan) {
        checks.require(
            false,
            "the GPU refused a recorded plan",
            one_line(&error.to_string()),
        );
        return "skipped, the GPU refused the plan".to_owned();
    }
    let Ok(image) = gpu.capture() else {
        checks.require(
            false,
            "the GPU rendered and would not hand the frame back",
            "an offscreen backend can always read its own target".to_owned(),
        );
        return "skipped, nothing to read back".to_owned();
    };
    let path = artifact_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&path, encode_png(&image)).is_err() {
        checks.require(
            false,
            "the capture could not be written",
            path.display().to_string(),
        );
        return "skipped, the file could not be written".to_owned();
    }
    let shown = std::fs::canonicalize(&path).unwrap_or(path);
    format!(
        "{}x{} written to {}",
        image.size.width,
        image.size.height,
        shown.display()
    )
}

fn artifact_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join("restaurant_nemesis_r3v2.png")
}
