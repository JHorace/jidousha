//! The captured frame: one frame the check already recorded, rendered on a GPU
//! and written out as a PNG.
//!
//! The half a person can look at. `verify.rs` asserts on what was *submitted*;
//! this executes one of those frames for real, which is the only thing that
//! would catch a backend drawing nothing at all — and the only thing that
//! answers "does it look like Pong", which no assertion in that file reaches.
//!
//! There is no second session and no game to re-run: a `FrameRecord` carries the
//! finished `FramePlan`, with the depth sort and the batching already done, and
//! a renderer built for the purpose executes it.
//!
//! This game loads no assets — every shape is a colour and every string is the
//! built-in font — so the built-in textures are the whole texture table, and the
//! ids inside the plan mean the same thing here because both counters started
//! empty and were filled by the same call. That is checked rather than assumed.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};

use crate::checks::{Checks, fail};

/// How big the captured artifact is.
///
/// The **same 16:9 shape** the recorder's viewport has. The projection was
/// computed from that viewport and is baked into the plan; nothing downstream
/// can recompute it, so a capture of another shape stretches the picture while
/// every assertion goes on passing, because none of them look at pixels.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(640, 360);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// An engine message flattened onto one line.
///
/// `RenderError`'s `Display` is the four-part shape, which is right on its own
/// and wrong inside a `--verify` summary, where the convention is one indented
/// line per fact.
fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render `frame` on a GPU and write it out, returning the line the summary
/// prints.
///
/// A machine with no GPU is a fact about the machine, not a failure: say the
/// capture was skipped and keep the run green. Every *other* handshake error is
/// a fault, and reporting one of those as "no GPU here" files a real problem as
/// a property of the hardware for ever.
pub(super) fn capture_a_frame(
    checks: &mut Checks,
    frame: &FrameRecord,
    font: BackendTextureId,
    name: &str,
) -> String {
    checks.require(
        CAPTURE_SIZE.aspect() == crate::WINDOW.aspect(),
        "the capture is not the shape the frame was planned for",
        format!(
            "capturing {}x{} from a plan projected for {}x{}; the projection is baked in and \
             nothing downstream can recompute it, so the picture would be stretched while \
             every other check passed",
            CAPTURE_SIZE.width,
            CAPTURE_SIZE.height,
            crate::WINDOW.width,
            crate::WINDOW.height,
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

    let textures = create_builtin_textures(&mut gpu);
    // The load-bearing line. The plan names texture ids, and an id only means
    // anything to a backend that created its textures in the same order. Without
    // this, a plan whose ids drifted renders the wrong texture into a PNG that
    // every other check in this run is happy with.
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
    let path = artifact_path(name);
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

/// Where a captured frame is written.
fn artifact_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join(format!("{name}.png"))
}
