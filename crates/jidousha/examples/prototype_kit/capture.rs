//! The captured frame: the frame the check already recorded, rendered on a GPU
//! and written out as a PNG.
//!
//! R4's artifact. `verify.rs` asserts on what was *submitted*; this renders one
//! of those frames for real and leaves a picture behind, which is the half a
//! person can look at and the half that would catch a backend drawing nothing
//! at all.
//!
//! There is no second session and no game to re-run: a `FrameRecord` carries
//! the finished `FramePlan`, with the depth sort and the batching already done,
//! and a renderer built for the purpose executes it.
//!
//! **This game loads art, which is the case the short path does not cover on its
//! own.** The plan names a texture id, and an id only means anything to a
//! backend that created its textures in the same order — so the replay creates
//! the built-ins and then uploads the same art, and checks the ids agree rather
//! than assuming they do.
//!
//! A machine with no GPU is not a failure. Every runner this project has is
//! headless and some have no graphics stack at all; the run says so and the
//! rest of the verification stands, exactly as the golden tests do
//! (renderer.md §9).
//!
//! **This file is load-bearing documentation, not only an example.**
//! `docs/api/jidousha-capture.md` names the four things about capture that are
//! its own — the `capture:` line `tools/verify` parses, the aspect ratio, the
//! texture-id check, and the no-GPU case — and sends the reader here for the
//! path itself (ADR-0034). It used to carry the path as well, the two copies
//! drifted, and the document's was the wrong one for six runs (e0-findings.md
//! F-134). So the reasoning at each step below is what a game author reads
//! instead of a second transcription: keep it written out, and if this path
//! changes, the document's four bullets are what to check against it.

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png, upload_ready_textures,
};
use std::path::{Path, PathBuf};

use crate::checks::{Checks, fail};
use crate::verify::store;

/// How big the captured artifact is.
///
/// Small enough to be cheap to write every run, big enough to see what the game
/// looks like — and the **same 16:9 shape** the window uses. A capture at
/// another aspect is a picture of a different framing: the field is twenty
/// world units tall whatever the window is, so a 4:3 capture crops the left
/// paddle out entirely and the artifact stops being a picture of the game.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(480, 270);

/// How many polls to give the GPU handshake before calling it absent.
///
/// The backend is poll-based by design (ADR-0011); a verify run has no frame
/// loop, so it does the asking itself.
const HANDSHAKE_POLLS: usize = 10_000;

/// An engine message flattened onto one line.
///
/// `RenderError`'s `Display` is the four-part shape, which is right when it is
/// the only thing on the screen and wrong inside a `--verify` summary: the
/// convention there is a verdict line and then one indented line per fact
/// (`tools/verify` prints exactly that block), and a four-line value turns the
/// summary into three lines of somebody else's paragraph. Every word is kept —
/// a machine with no GPU is precisely where the detail is worth having.
fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render the recorded frame on a GPU and write it out as a PNG.
///
/// A machine with no GPU is not a failure. Every runner this project has is
/// headless and some have no graphics stack at all; the run says so and the
/// rest of the verification stands, exactly as the golden tests do.
pub(super) fn capture_a_frame(
    checks: &mut Checks,
    frame: &FrameRecord,
    font: BackendTextureId,
) -> String {
    let mut gpu = WgpuBackend::offscreen(CAPTURE_SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
            // Two different things, and only the first is a fact about the
            // machine. `NoAdapter` means there is no GPU here, which every
            // headless runner reports and which the transcript tier does not
            // need — the run stays green and says it skipped (renderer.md §9).
            // Anything else is a fault, and calling one of those "no GPU on
            // this machine" files an engine bug as a property of the hardware.
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

    // The built-in textures first, in the order the recorder created them, so
    // the ids inside the plan mean the same thing here — then the same art,
    // uploaded the same way, because this game has some and the plan names it.
    let mut textures = create_builtin_textures(&mut gpu);
    let mut assets = store();
    // The same file the game asks for, requested the same way: a store only
    // has texels for something that was *loaded*, so a replay that skips this
    // uploads nothing and the plan names a texture the GPU does not have.
    let _ = assets.load_texture("sprites/hero.png");
    // Past the tick the store is scripted to resolve on, so the load has
    // something to hand over.
    let _ = assets.commit(crate::verify::TICKS);
    upload_ready_textures(&mut assets, &mut gpu, &mut textures);

    // Checked rather than assumed, which is the whole load-bearing step: both
    // counters started empty and both were filled by the same calls in the same
    // order, so the font has to land on the id the recorder reported. If it
    // does not, every other id in the plan is wrong too and the picture is of
    // something else.
    checks.require(
        textures.resolve(FONT_TEXTURE) == font,
        "the replay's texture ids do not mean what the recorded plan means",
        format!(
            "the recorder put the font on {font:?} and this backend put it on {:?}; the plan \
             names ids, so a mismatch means the picture samples the wrong textures",
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
        .join("prototype_kit.png")
}
