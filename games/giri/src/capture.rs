//! The captured frames: one PNG per screen mode, at reference size and at a
//! narrow one (UI.md §8).
//!
//! `verify.rs` asserts on what was *submitted*; this renders those frames for
//! real and leaves pictures behind — the half a person can look at, and the
//! half that would catch a frame every assertion was happy with. **The narrow
//! set is why there are two.** A scaling regression is invisible to every
//! assertion in this game that is not about pixels, and UI.md §6's defect was
//! exactly that shape: the transcript went on being correct while the screen
//! went wrong.
//!
//! giri has art now, so this is the long path: the same `Assets` store the game
//! builds, the same loads, the same wait for the disk (`sprites::settle`), and
//! `upload_ready_textures` before the plan is replayed — a plan naming a texture
//! the backend does not have renders the wrong picture into a file nothing else
//! looks at.
//!
//! A machine with no GPU is not a failure: every runner this project has is
//! headless and some have no graphics stack at all.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png, upload_ready_textures,
};

use crate::checks::{Checks, fail, one_line};
use crate::constants::Tuning;
use crate::sprites;
use crate::verify::{self, BeatRun};

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// One picture that is wanted: its name, the surface it was drawn to, and the
/// frame it came from.
struct Wanted {
    name: String,
    surface: PhysicalSize,
    frame: FrameRecord,
    font: BackendTextureId,
}

/// How big a picture of `surface` is written at.
///
/// A surface larger than the reference resolution is halved, so the reference
/// set is a 960x540 file rather than a 1920x1080 one; a surface at or under it
/// is captured whole, because the narrow set exists to be *looked at* and
/// halving an already-small screen makes it unreadable for the one reader it
/// has.
///
/// **The ratio is asserted, not remembered** (jidousha-capture.md): a capture of
/// another shape stretches the picture while every assertion goes on passing,
/// because none of them look at pixels.
fn capture_size(surface: PhysicalSize) -> PhysicalSize {
    if surface.width > crate::layout::REFERENCE.width
        || surface.height > crate::layout::REFERENCE.height
    {
        PhysicalSize::new(surface.width / 2, surface.height / 2)
    } else {
        surface
    }
}

/// The beat each screen mode is photographed from.
///
/// Beat 2 is the one with a killing in it, so its takeover is the screen the
/// resolution mode exists to show; beat 3 is the one with a refusal, so its
/// board carries the door rule's arithmetic. A capture set of four identical
/// boards would be a capture set nobody learns anything from.
fn chosen(runs: &[BeatRun]) -> Vec<(&'static str, usize)> {
    let killing = runs
        .iter()
        .position(|run| run.after.members.iter().any(|member| !member.alive))
        .unwrap_or(0);
    let refusal = runs
        .iter()
        .position(|run| run.refusal.is_some())
        .unwrap_or(0);
    vec![
        ("board", refusal),
        ("staged", refusal),
        ("resolution", killing),
    ]
}

/// Render every screen mode at both sizes and write them out.
pub fn capture_screens(checks: &mut Checks, runs: &[BeatRun], tuning: Tuning) -> String {
    let mut wanted: Vec<Wanted> = Vec::new();
    for (mode, beat) in chosen(runs) {
        // Reference comes from the run already played; the narrow set is a
        // second scripted run at the narrow surface, because a frame recorded
        // at one viewport cannot be re-rendered at another - the world-space
        // geometry in the plan was produced by that run's camera.
        if let Some(run) = runs.get(beat)
            && let Some(frame) = pick(run, mode)
        {
            wanted.push(Wanted {
                name: format!("{mode}-reference"),
                surface: verify::HEADLESS_VIEWPORT,
                frame: frame.clone(),
                font: run.font,
            });
        }
        let narrow = verify::play_at(beat, tuning, true, verify::NARROW_VIEWPORT);
        if let Some(frame) = pick(&narrow, mode) {
            wanted.push(Wanted {
                name: format!("{mode}-narrow"),
                surface: verify::NARROW_VIEWPORT,
                frame: frame.clone(),
                font: narrow.font,
            });
        }
    }

    let Some(first) = wanted.first() else {
        return "capture: skipped, no frame was recorded".to_owned();
    };
    let mut gpu = WgpuBackend::offscreen(capture_size(first.surface));
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
            // `NoAdapter` is a fact about the machine, and every other error is
            // a fault: reporting one of those as "no GPU here" files a real
            // problem as a property of the hardware, for ever.
            Err(error @ RenderError::NoAdapter { .. }) => {
                return format!(
                    "capture: skipped, no GPU on this machine ({})",
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
                    "capture: skipped, the GPU handshake failed ({})",
                    one_line(&error.to_string())
                );
            }
        }
    }
    if !gpu.is_ready() {
        return "capture: skipped, the GPU handshake never finished".to_owned();
    }

    let mut textures = create_builtin_textures(&mut gpu);
    // giri's own art, uploaded the way the driver uploads it, so the plan's
    // sprite ids name textures this backend has.
    let mut assets = sprites::store();
    let _gallery = sprites::Gallery::load(&mut assets);
    for failure in sprites::settle(&mut assets) {
        checks.require(
            false,
            "giri's art did not all resolve for the capture",
            one_line(&failure.message()),
        );
    }
    upload_ready_textures(&mut assets, &mut gpu, &mut textures);
    checks.require(
        textures.resolve(FONT_TEXTURE) == first.font,
        "the replay's texture ids do not mean what the recorded plan means",
        format!(
            "the recorder put the font on {:?} and this backend put it on {:?}",
            first.font,
            textures.resolve(FONT_TEXTURE)
        ),
    );

    let mut written: Vec<String> = Vec::new();
    for shot in &wanted {
        let size = capture_size(shot.surface);
        checks.require(
            crate::checks::near(size.aspect(), shot.surface.aspect()),
            "a capture is not the shape of the surface it was drawn to",
            format!(
                "{} was drawn to {}x{} and captured at {}x{}; another shape stretches the \
                 picture while every assertion goes on passing",
                shot.name, shot.surface.width, shot.surface.height, size.width, size.height
            ),
        );
        gpu.resize_surface(size);
        if let Err(error) = gpu.render(&shot.frame.plan) {
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
        let path = artifact_path(&shot.name);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::write(&path, encode_png(&image)).is_err() {
            fail(
                "a captured frame could not be written",
                &format!("tried to write {}", path.display()),
            );
        }
        written.push(format!(
            "{} {}x{} {}",
            shot.name,
            image.size.width,
            image.size.height,
            std::fs::canonicalize(&path).unwrap_or(path).display()
        ));
    }

    // `tools/verify` reads exactly one line: the first whose text starts with
    // `capture:` and contains " written to ". The rest are for a person.
    let mut summary = String::new();
    if let Some(headline) = written.first() {
        let (name, rest) = headline.split_once(' ').unwrap_or(("", headline));
        let (size, path) = rest.split_once(' ').unwrap_or(("", rest));
        summary.push_str(&format!("  capture: {name} {size} written to {path}\n"));
    }
    summary.push_str(&format!("  {} screen captures in all:\n", written.len()));
    for line in written.iter().skip(1) {
        summary.push_str(&format!("    also: {line}\n"));
    }
    summary.trim_end().to_owned()
}

fn pick<'a>(run: &'a BeatRun, mode: &str) -> Option<&'a FrameRecord> {
    verify::screen_modes(run)
        .into_iter()
        .find(|(name, _)| *name == mode)
        .and_then(|(_, frame)| frame)
}

/// Where a captured frame is written.
fn artifact_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join(format!("giri-{name}.png"))
}
