//! The captured frames: the screenshots a person looks at (giri's capture
//! path, re-aimed at the map).
//!
//! Eight pictures. The mid-travel map and the feed are each taken at the
//! reference surface and at a narrow one (the narrow set exists to catch
//! scaling regressions, which are invisible to every assertion that is not
//! about pixels). The rest are reference only: the settlement before anything
//! has been dispatched — the cast standing at their homes, named — the
//! auto-pause config panel, one character's own panel with the selection ring
//! on their figure, and the tuning drawer (a dev surface whose rows are the
//! smallest type in the game).
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
use crate::restart::DrawerRun;
use crate::sweep::Conducted;
use crate::{sprites, verify};

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// One picture that is wanted.
struct Wanted {
    name: String,
    surface: PhysicalSize,
    frame: FrameRecord,
    font: BackendTextureId,
}

/// How big a picture of `surface` is written at: reference surfaces are
/// halved to a 960x540 file, the narrow set is captured whole. The ratio is
/// asserted, not remembered (jidousha-capture.md).
fn capture_size(surface: PhysicalSize) -> PhysicalSize {
    if surface.width > crate::layout::REFERENCE.width
        || surface.height > crate::layout::REFERENCE.height
    {
        PhysicalSize::new(surface.width / 2, surface.height / 2)
    } else {
        surface
    }
}

/// Render every wanted frame and write the PNGs.
pub fn capture_screens(
    checks: &mut Checks,
    reference: &Conducted,
    narrow: &Conducted,
    drawer: &DrawerRun,
) -> String {
    let mut wanted: Vec<Wanted> = Vec::new();
    // The reference-only set: pictures of *what is on screen* rather than of
    // how the chrome scales, which the map and feed pairs already cover.
    for name in ["settlement", "modes", "person"] {
        if let Some(shot) = reference.photo(name) {
            wanted.push(Wanted {
                name: format!("{name}-reference"),
                surface: verify::HEADLESS_VIEWPORT,
                frame: shot.frame.clone(),
                font: reference.font,
            });
        } else {
            checks.require(
                false,
                "a reference-only capture was never photographed",
                format!("the {name} photo is missing from the reference run"),
            );
        }
    }
    for name in ["map", "feed"] {
        if let Some(shot) = reference.photo(name) {
            wanted.push(Wanted {
                name: format!("{name}-reference"),
                surface: verify::HEADLESS_VIEWPORT,
                frame: shot.frame.clone(),
                font: reference.font,
            });
        } else {
            checks.require(
                false,
                "a reference capture was never photographed",
                format!("the {name} photo is missing from the reference run"),
            );
        }
        if let Some(shot) = narrow.photo(name) {
            wanted.push(Wanted {
                name: format!("{name}-narrow"),
                surface: verify::NARROW_VIEWPORT,
                frame: shot.frame.clone(),
                font: narrow.font,
            });
        } else {
            checks.require(
                false,
                "a narrow capture was never photographed",
                format!("the {name} photo is missing from the narrow run"),
            );
        }
    }
    if let Some(shot) = &drawer.shot {
        wanted.push(Wanted {
            name: "tuning-reference".to_owned(),
            surface: verify::HEADLESS_VIEWPORT,
            frame: shot.frame.clone(),
            font: drawer.font,
        });
    }

    let Some(first) = wanted.first() else {
        return "capture: skipped, no frame was recorded".to_owned();
    };
    let mut gpu = WgpuBackend::offscreen(capture_size(first.surface));
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
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
    // ninjo's own art, uploaded the way the driver uploads it, so the
    // plan's sprite ids name textures this backend has.
    let mut assets = sprites::store();
    let _gallery = sprites::Gallery::load(&mut assets);
    for failure in sprites::settle(&mut assets) {
        checks.require(
            false,
            "ninjo's art did not all resolve for the capture",
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

/// Where a captured frame is written.
fn artifact_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join(format!("ninjo-{name}.png"))
}
