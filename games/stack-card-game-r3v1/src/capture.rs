//! The captured frame: the aiming frame of the `respond-window` check — three
//! items on the stack, the two legal Turn targets lit — rendered on a GPU and
//! written out as a PNG.
//!
//! A game of shapes and text: the built-in textures are the whole texture
//! table, so there is no store to replay (docs/api/jidousha-capture.md). A
//! machine with no GPU says so and the run stays green; any other handshake
//! error is a failed check.
//!
//! Key function: `capture_a_frame`.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};

use crate::WINDOW;
use crate::checks::{Checks, fail};

/// 960 x 540: the window's 16:9, so the picture is not stretched, and big
/// enough that the 12-unit rows can be read off it.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(960, 540);

/// Polls the GPU handshake gets before it is called absent.
const HANDSHAKE_POLLS: usize = 10_000;

fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render `frame` on a GPU and write it out; the `capture:` summary text.
pub(crate) fn capture_a_frame(
    checks: &mut Checks,
    frame: &FrameRecord,
    font: BackendTextureId,
) -> String {
    let (wide, tall) = (
        CAPTURE_SIZE.width * WINDOW.height,
        CAPTURE_SIZE.height * WINDOW.width,
    );
    checks.require(
        wide == tall,
        "the capture is not the window's shape",
        format!("{CAPTURE_SIZE:?} against a {WINDOW:?} window"),
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
            "the recorder put the font on {font:?} and this backend on {:?}",
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

fn artifact_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join("stack_card_game_r3v1.png")
}
