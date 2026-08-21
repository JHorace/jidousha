//! The picture: one recorded frame, rendered on a GPU and written out as a PNG.
//!
//! `verify.rs` asserts on what was *submitted*; this renders one of those frames
//! for real and leaves something a person can look at. It is the only instrument
//! in the whole surface that answers "does it look like the game", and nothing
//! takes it for you — `tools/verify` renders nothing and reads one line out of
//! what this run printed, so a game with no capture path passes every check and
//! simply cannot be looked at.
//!
//! There is no second session and no game to re-run: a `FrameRecord` carries the
//! finished `FramePlan`, with the depth sort and the batching already done, and
//! a renderer built for the purpose executes it.
//!
//! **Pong is a game of pure shapes and text, so this is the short path.**
//! `examples/prototype_kit/capture.rs` is the worked version and it loads art;
//! the lines this file leaves out are exactly that half — the `Assets` store,
//! the `load_texture`, the `commit` and the `upload_ready_textures`.
//! `create_builtin_textures` *is* the whole texture table here: the flat white
//! every untextured quad is tinted out of, the placeholder, and the font atlas
//! registered under `FONT_TEXTURE` by that same call. Those are the only ids a
//! plan built from `ctx.rect` and `ctx.text` can name. The id check that follows
//! them is not one of the omissions.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png,
};

/// How big the captured picture is.
///
/// The **same 16:9 shape** the recorder draws at. A capture at another aspect
/// stretches the picture while every assertion in `verify.rs` goes on passing,
/// because not one of them looks at a pixel — and nothing downstream can
/// recompute the right shape, because the plan carries world coordinates and a
/// view-projection rather than a framing.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(480, 270);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// An engine message flattened onto one line.
///
/// `RenderError`'s `Display` is the engine's four-part shape, which is right on
/// its own and wrong inside a `--verify` summary, where the convention is one
/// indented line per fact. Every word is kept: a machine with no GPU is exactly
/// where the detail is worth having.
fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render `frame` and write it out. `Ok` is the text of the `capture:` line.
///
/// A machine with no GPU is not a failure — every runner here is headless and
/// some have no graphics stack at all — so that case reports itself and keeps
/// the run green. Every *other* handshake error is a fault: reporting one of
/// those as "no GPU here" files a real problem as a property of the hardware, on
/// every machine, for ever.
pub(crate) fn write_the_picture(
    frame: &FrameRecord,
    font: BackendTextureId,
) -> Result<String, String> {
    let mut gpu = WgpuBackend::offscreen(CAPTURE_SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
            Err(error @ RenderError::NoAdapter { .. }) => {
                return Ok(format!(
                    "skipped, no GPU on this machine ({})",
                    one_line(&error.to_string())
                ));
            }
            Err(error) => {
                return Err(format!(
                    "an adapter was found and the handshake still did not finish, which is a \
                     fault rather than a fact about the machine: {}",
                    one_line(&error.to_string())
                ));
            }
        }
    }
    if !gpu.is_ready() {
        return Ok(format!(
            "skipped, the GPU handshake never finished in {HANDSHAKE_POLLS} polls"
        ));
    }

    // The built-in textures, in the order the recorder created them, so the ids
    // inside the plan mean the same thing here.
    let textures = create_builtin_textures(&mut gpu);
    // Checked rather than assumed, and it is one line. Both tables started empty
    // and both were filled by the same call, so the font has to land on the id
    // the recorder reported. If it does not, every other id in the plan is wrong
    // too and the PNG is a picture of something else — which every check in
    // `verify.rs` would be perfectly happy with.
    let here = textures.resolve(FONT_TEXTURE);
    if here != font {
        return Err(format!(
            "the replay's texture ids do not mean what the recorded plan means: the recorder \
             put the font on {font:?} and this backend put it on {here:?}, and the plan names \
             ids"
        ));
    }

    if let Err(error) = gpu.render(&frame.plan) {
        return Err(format!(
            "the GPU refused a plan the recorder had already accepted: {}",
            one_line(&error.to_string())
        ));
    }
    let image = match gpu.capture() {
        Ok(image) => image,
        Err(error) => {
            return Err(format!(
                "the GPU rendered the frame and then would not hand it back, though an \
                 offscreen backend can always read its own target: {}",
                one_line(&error.to_string())
            ));
        }
    };
    if (image.size.aspect() - CAPTURE_SIZE.aspect()).abs() > 1e-4 {
        return Err(format!(
            "the capture came back {}x{}, an aspect of {:.4} against the {:.4} it was asked \
             for — a picture of a different framing, which no assertion looks at",
            image.size.width,
            image.size.height,
            image.size.aspect(),
            CAPTURE_SIZE.aspect()
        ));
    }

    let path = artifact_path();
    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return Err(format!("could not make {}", parent.display()));
    }
    if std::fs::write(&path, encode_png(&image)).is_err() {
        return Err(format!("could not write {}", path.display()));
    }
    let shown = std::fs::canonicalize(&path).unwrap_or(path);
    // Worded exactly as `tools/verify` parses it: the first line whose text
    // starts with `capture:` and contains ` written to `. Any other wording and
    // the run passes while the report says no picture was taken.
    Ok(format!(
        "{}x{} written to {}",
        image.size.width,
        image.size.height,
        shown.display()
    ))
}

/// Where the picture is written.
fn artifact_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join("pong.png")
}
