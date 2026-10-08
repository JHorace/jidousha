//! The captured frame: one recorded frame rendered on a GPU and written out as
//! a PNG. A game of text and rectangles, so the built-in textures are the
//! whole texture table (`docs/api/jidousha-capture.md`).

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};

use crate::checks::Checks;
use crate::screens::WINDOW;

/// The artifact's size: the recorder's 16:9, small.
pub const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(640, 360);

const HANDSHAKE_POLLS: usize = 10_000;

fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render `frame` and write it to `target/verify/<name>.png`; say what
/// happened, as the summary's `capture:` line wants it.
pub fn capture(
    checks: &mut Checks,
    frame: &FrameRecord,
    font: BackendTextureId,
    name: &str,
) -> String {
    let want = WINDOW.aspect();
    let got = CAPTURE_SIZE.aspect();
    checks.require(
        (want - got).abs() < 0.001,
        "the capture is not at the recorder's aspect, so the picture is stretched",
        format!("recorder {want:.4}, capture {got:.4}"),
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
    checks.require(
        textures.resolve(FONT_TEXTURE) == font,
        "the replay's texture ids do not mean what the recorded plan means",
        format!(
            "the recorder put the font on {font:?}, this backend on {:?}",
            textures.resolve(FONT_TEXTURE)
        ),
    );
    if let Err(error) = gpu.render(&frame.plan) {
        checks.require(
            false,
            "the GPU refused a recorded plan",
            one_line(&error.to_string()),
        );
        return "failed: the GPU refused the plan".to_owned();
    }
    let image = match gpu.capture() {
        Ok(image) => image,
        Err(error) => {
            checks.require(
                false,
                "the GPU would not hand the frame back",
                one_line(&error.to_string()),
            );
            return "failed: no image came back".to_owned();
        }
    };
    let path = artifact_path(name);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(error) = std::fs::write(&path, encode_png(&image)) {
        checks.require(
            false,
            "the captured frame could not be written",
            format!("{}: {error}", path.display()),
        );
        return "failed: not written".to_owned();
    }
    let shown = std::fs::canonicalize(&path).unwrap_or(path);
    format!(
        "{}x{} written to {}",
        image.size.width,
        image.size.height,
        shown.display()
    )
}

fn artifact_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join(format!("{name}.png"))
}
