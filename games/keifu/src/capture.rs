//! The pictures: recorded frames rendered on a GPU and written out as PNGs.
//!
//! The path is `examples/prototype_kit/capture.rs`'s, less the art half: this game
//! draws only shapes and text, so `create_builtin_textures` is the whole texture
//! table (docs/api/jidousha-capture.md). Three pictures: the W0+W1 oracle screen
//! (top bar and Garrick's sheet), a child's sheet, and the family screen.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    FONT_TEXTURE, FrameRecord, FrameRecorder, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};

use crate::checks::Checks;
use crate::screen::{Target, WINDOW};
use crate::verify::{hero_named, point_at, session};

/// The recorder's 16:9 shape, at full size so the sheet's small type reads.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(1280, 720);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Capture the three pictures; returns the summary lines, the first one being the
/// `capture:` line `tools/verify` reads.
pub fn capture_all(
    checks: &mut Checks,
    recorder: &mut FrameRecorder,
    garrick: &FrameRecord,
) -> Vec<String> {
    checks.require(
        CAPTURE_SIZE.width * WINDOW.height == CAPTURE_SIZE.height * WINDOW.width,
        "the capture is not the recorder's shape",
        format!("{CAPTURE_SIZE:?} against {WINDOW:?}"),
    );
    let mut sim = session(crate::verify::SEEDS[0]);
    let wren = hero_named(&sim, "Wren");
    point_at(&mut sim, Target::Hero(wren), false);
    let child = recorder.draw(&mut sim);
    point_at(&mut sim, Target::OpenFamily, true);
    let elsbeth = hero_named(&sim, "Maren");
    point_at(&mut sim, Target::Hero(elsbeth), false);
    let family = recorder.draw(&mut sim);
    let font = recorder.font_texture();
    let mut lines = Vec::new();
    for (index, (frame, file)) in [
        (garrick, "keifu.png"),
        (&child, "keifu-child.png"),
        (&family, "keifu-family.png"),
    ]
    .into_iter()
    .enumerate()
    {
        let said = capture(checks, frame, font, file);
        lines.push(if index == 0 {
            format!("capture: {said}")
        } else {
            format!("also captured: {said}")
        });
    }
    lines
}

fn capture(
    checks: &mut Checks,
    frame: &FrameRecord,
    font: jidousha::testing::BackendTextureId,
    file: &str,
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
        return format!("{file} not written: the GPU refused the plan");
    }
    let Ok(image) = gpu.capture() else {
        checks.require(
            false,
            "the GPU rendered and would not hand the frame back",
            file.to_owned(),
        );
        return format!("{file} not written: no image came back");
    };
    let path = artifact_path(file);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&path, encode_png(&image)).is_err() {
        checks.require(
            false,
            "a captured frame could not be written",
            path.display().to_string(),
        );
        return format!("{file} not written");
    }
    let shown = std::fs::canonicalize(&path).unwrap_or(path);
    format!(
        "{}x{} written to {}",
        image.size.width,
        image.size.height,
        shown.display()
    )
}

fn artifact_path(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join(file)
}
