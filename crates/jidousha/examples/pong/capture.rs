//! The captured frame: the plan this run already recorded, drawn on a GPU.
//!
//! `verify.rs` asserts on what was *submitted*; this renders it for real and
//! leaves a picture behind — the half a person can look at, and the half that
//! would catch a backend drawing nothing at all.
//!
//! **It does not play the game a second time.** `prototype_kit` replays its
//! whole session through an offscreen backend because its `play` is handed the
//! backend to render through; this game's is handed a `FrameRecorder` and never
//! names a backend, so that shape does not transfer without dragging the
//! controller and every check along with it. It does not have to:
//! `FrameRecord::plan` is the `FramePlan` the recorder's null backend was given,
//! `RenderBackend::render` takes a plan, and a plan is finished work — world
//! space is already gone, the sort and the batching already happened, and what
//! is left is vertices and texture ids. So the last frame of the match is
//! replayable on any backend that has the textures it names, and building those
//! is one call (see `capture_a_frame` for why the ids line up, which is checked
//! rather than assumed).
//!
//! A machine with no GPU is not a failure. Some runners have no graphics stack
//! at all; the run says so and the rest of the verification stands, exactly as
//! the golden tests do (renderer.md §9).

use std::path::{Path, PathBuf};

use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FramePlan, PhysicalSize, RenderBackend, WgpuBackend,
    create_builtin_textures, encode_png,
};

use crate::checks::{Checks, VIEWPORT};

/// How big the captured artifact is.
///
/// Small enough to be cheap to write every run, big enough to see what the game
/// looks like — and the **same 16:9 shape** the recorder used.
///
/// That is not a preference, it is the one thing a replayed plan cannot survive
/// getting wrong. `FramePlan::view_projection` was computed from `VIEWPORT`
/// before this file ever saw it and nothing here can recompute it; render it
/// into a target of another aspect and the picture stretches, silently, with
/// every assertion in `verify.rs` still passing because none of them are
/// looking at pixels. So the ratio is checked by the compiler rather than
/// promised in prose, by cross-multiplication so no float rounds.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(480, 270);
const _: () = assert!(
    CAPTURE_SIZE.width * VIEWPORT.height == VIEWPORT.width * CAPTURE_SIZE.height,
    "the capture is a different shape from the viewport the frame plan was projected for, \
     so the picture would be stretched and nothing would say so"
);

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
/// summary into three lines of somebody else's paragraph with no way to tell
/// which fact they belong to. Every word is kept — a machine with no GPU is
/// precisely where the detail is worth having.
fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Where the captured frame is written. `target/verify/` is what CI uploads.
fn artifact_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join("pong.png")
}

/// Draw `plan` on a GPU and write it out as a PNG; say what happened either way.
///
/// `plan` is the last frame of the played match, exactly as the recorder's null
/// backend received it, and `font` is the recorder's font atlas id.
///
/// **Why the texture ids in a recorded plan mean anything on a second backend.**
/// A plan names textures by `BackendTextureId`, which is whatever backend
/// created them counting upwards; a fresh `WgpuBackend` counts from zero again.
/// They agree here because both counters start empty and both are filled by the
/// same call: `FrameRecorder::new` runs `create_builtin_textures` before
/// anything else, this runs it before anything else, and that function creates
/// white, then the placeholder, then the font atlas, in that fixed order. This
/// game loads no assets — every shape is white-with-a-colour and every string is
/// the atlas — so nothing else is ever created on either side and nothing shifts
/// the numbering.
///
/// That is an argument, so it is checked. The font is the *last* built-in, so
/// two dense sequences from empty backends agreeing on it agree on all three;
/// and no batch may name an id past it, which is what "this game loads no
/// assets" has to mean if the replay is to be honest. A debug build would panic
/// inside the backend on an id it does not have, so a mismatch cannot pass
/// quietly — but it would panic with the renderer's message about an engine bug,
/// which is the wrong diagnosis for what would actually be wrong here.
pub(crate) fn capture_a_frame(
    checks: &mut Checks,
    plan: &FramePlan,
    font: BackendTextureId,
) -> String {
    let mut gpu = WgpuBackend::offscreen(CAPTURE_SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
            Err(error) => {
                return format!(
                    "skipped, no GPU on this machine ({})",
                    one_line(&error.to_string())
                );
            }
        }
    }
    if !gpu.is_ready() {
        return "skipped, the GPU handshake never finished".to_owned();
    }

    // After the handshake, so the texels go to a device that exists rather than
    // waiting in the backend for one. The table it hands back is thrown away:
    // the plan resolved its texture ids at plan time, and this call is here for
    // the uploads and for the one id it lets us compare.
    let table = create_builtin_textures(&mut gpu);
    checks.require(
        table.resolve(FONT_TEXTURE) == font,
        "the recorded plan's texture ids do not mean the same thing on a second backend",
        format!(
            "the font atlas is {:?} on the wgpu backend and {font:?} on the recorder's; the \
             plan names textures by the id whichever backend created them, so replaying it \
             here would draw the wrong texels — or none, if the id does not exist",
            table.resolve(FONT_TEXTURE),
        ),
    );
    let strays: Vec<BackendTextureId> = plan
        .batches
        .iter()
        .map(|batch| batch.texture)
        .filter(|id| id.0 > font.0)
        .collect();
    checks.require(
        strays.is_empty(),
        "the recorded frame draws a texture this game never loaded",
        format!(
            "{strays:?} come after the last built-in ({font:?}); every shape here is a colour \
             on the white texel and every string is the font atlas, so a fourth texture means \
             the replay below is missing one"
        ),
    );

    if let Err(error) = gpu.render(plan) {
        // A reading, not a reason to stop: this game collects its failures and
        // reports them together (checks.rs), so a capture that could not be
        // taken is one more line rather than the only line.
        checks.require(
            false,
            "the GPU would not draw the frame the run recorded",
            format!("rendering the last frame's plan offscreen failed: {error}"),
        );
        return format!(
            "skipped, the frame would not render ({})",
            one_line(&error.to_string())
        );
    }
    let image = match gpu.capture() {
        Ok(image) => image,
        Err(error) => {
            checks.require(
                false,
                "the GPU drew the frame and then would not hand it back",
                format!("an offscreen backend can always read its own target: {error}"),
            );
            return format!(
                "skipped, the frame would not read back ({})",
                one_line(&error.to_string())
            );
        }
    };

    let path = artifact_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(error) = std::fs::write(&path, encode_png(&image)) {
        checks.require(
            false,
            "the captured frame could not be written",
            format!("tried to write {}: {error}", path.display()),
        );
        return format!("skipped, {} could not be written ({error})", path.display());
    }
    let shown = std::fs::canonicalize(&path).unwrap_or(path);
    format!(
        "{}x{} written to {}",
        image.size.width,
        image.size.height,
        shown.display()
    )
}
