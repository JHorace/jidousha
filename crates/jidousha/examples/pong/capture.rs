//! The picture: the frame the check already judged, rendered for real and
//! written out as a PNG.
//!
//! No assertion in `verify.rs` can see whether the game *looks* like Pong —
//! whether the clear colour is right, whether the court reads as a court, or
//! whether the paddles are the colours they were meant to be. This is the half
//! a person or an agent can open.
//!
//! It replays the recorded frame's plan rather than running the session again.
//! The plan is the finished frame with the depth sort and the batching already
//! done, and a renderer built for the purpose will execute it — so there is no
//! second run to keep in step with the first, and no way for the picture to be
//! of a different game than the one that was checked.
//!
//! A machine with no GPU is a fact about the machine, not a failure: the run
//! says it skipped and stays green. Every *other* handshake error is a fault,
//! because filing one of those as "no GPU here" files a real problem as a
//! property of the hardware, on every machine, forever.

use std::path::{Path, PathBuf};

use jidousha::testing::{
    FrameRecord, PhysicalSize, RenderBackend, RenderError, WgpuBackend, create_builtin_textures,
    encode_png,
};

use crate::VIEWPORT;
use crate::checks::{Checks, fail};

/// How big the captured picture is.
///
/// The **same shape** as the recorder's viewport. The projection was computed
/// from that viewport and is baked into the plan; nothing downstream can
/// recompute it, so a capture of another shape stretches the picture while
/// every assertion in `verify.rs` goes on passing, because none of them look at
/// pixels.
const CAPTURE: PhysicalSize = PhysicalSize::new(640, 360);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// An engine message flattened onto one line.
///
/// `RenderError`'s `Display` is the four-part shape, which is right on its own
/// and wrong inside a `--verify` summary — that is a verdict line and then one
/// indented line per fact, and a four-line value turns the summary into
/// somebody else's paragraph. Every word is kept: a machine with no GPU is
/// exactly where the detail earns its place.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render `frame` and write it out. The returned string is the summary line.
pub(super) fn capture_a_frame(checks: &mut Checks, frame: &FrameRecord) -> String {
    // Asserted rather than remembered: the plan's projection came from the
    // recorder's viewport, so a capture at another aspect is a picture of a
    // different framing.
    let (want, got) = (VIEWPORT.aspect(), CAPTURE.aspect());
    if (want - got).abs() > 1e-3 {
        checks.require(
            false,
            "the capture is not the shape the frame was planned at",
            format!(
                "the recorder's viewport is {}x{} ({want:.4}) and the capture is {}x{} \
                 ({got:.4}); the projection is baked into the plan and nothing downstream \
                 can recompute it",
                VIEWPORT.width, VIEWPORT.height, CAPTURE.width, CAPTURE.height
            ),
        );
    }

    let mut gpu = WgpuBackend::offscreen(CAPTURE);
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

    // The built-in textures, in the order the recorder created them, so the
    // ids inside the plan mean the same thing to this renderer. This game
    // loads no art at all, so there is nothing else to upload — every shape is
    // a colour and every string is the built-in font.
    let _ = create_builtin_textures(&mut gpu);
    if let Err(error) = gpu.render(&frame.plan) {
        checks.require(
            false,
            "the renderer refused a plan the recorder had already accepted",
            one_line(&error.to_string()),
        );
        return format!(
            "skipped, the frame would not render ({})",
            one_line(&error.to_string())
        );
    }
    let Ok(image) = gpu.capture() else {
        fail(
            "the GPU drew the frame and then would not hand it back",
            "an offscreen renderer can always read its own target",
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
    // `tools/verify` takes the first line starting with `capture:` that
    // contains " written to " and lifts what follows into its report, so the
    // wording of this line is load-bearing.
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
        .join("pong.png")
}
