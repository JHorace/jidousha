//! The pictures: recorded frames rendered on a GPU and written out as PNGs.
//!
//! The path is `examples/prototype_kit/capture.rs`'s, art half included: the
//! built-in textures first, then the same sprites loaded in the same order from
//! the same scripted store, so the plan's texture ids mean the same thing here
//! (docs/api/jidousha-capture.md). The pictures: the W0+W1 oracle screen (top
//! bar, the cast's cards and Garrick's sheet in the dock), a child's sheet, the
//! family, W3's Garrick — settled, blessed, and holding the blade he carried a dream
//! to — and W4's: Brannoc held over "Grave goods" beside Garrick, the card
//! previewing them both with his sheet in the dock; the quest sheet once both are
//! seated; the staged worst case and its sheet. Then the sheet dock's own: idle,
//! holding the help over year 1's full board, and Garrick's sheet scrolled to its end;
//! and W5's staged ghost: "Lay Garrick's ghost" on a year-2 board.

use std::path::{Path, PathBuf};

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FrameRecord, FrameRecorder, RenderBackend, RenderError,
    WgpuBackend, create_builtin_textures, encode_png, upload_ready_textures,
};

use crate::art::Art;
use crate::checks::Checks;
use crate::screen::{Target, WINDOW};
use crate::verify::{ART_ARRIVES, hero_named, point_at, session, store};

/// The recorder's 16:9 shape, at full size so the sheet's small type reads.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(1280, 720);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Capture the pictures; returns the summary lines, the first one being the
/// `capture:` line `tools/verify` reads.
///
/// Each picture is staged in its own session and recorded by its own fresh
/// recorder: a recorder uploads the art once per session it settles, so only a
/// recorder that saw one session holds the art at the ids a replay recreates.
pub fn capture_all(checks: &mut Checks) -> Vec<String> {
    checks.require(
        CAPTURE_SIZE.width * WINDOW.height == CAPTURE_SIZE.height * WINDOW.width,
        "the capture is not the recorder's shape",
        format!("{CAPTURE_SIZE:?} against {WINDOW:?}"),
    );
    let mut lines = Vec::new();
    for (index, (stage, file)) in [
        (&["Garrick"][..], "keifu.png"),
        (&["Wren"][..], "keifu-child.png"),
        (&["", "Maren"][..], "keifu-family.png"),
        (&["w3", "Garrick"][..], "keifu-w3-garrick.png"),
        (&["w4-drag"][..], "keifu-w4-drag.png"),
        (&["w4-sheet"][..], "keifu-w4-sheet.png"),
        (&["w4-worst"][..], "keifu-w4-worst.png"),
        (&["w4-worst", "q0"][..], "keifu-w4-worst-sheet.png"),
        (&[][..], "keifu-dock-idle.png"),
        (&["Garrick", "end"][..], "keifu-dock-scrolled.png"),
        (&["w5-ghost"][..], "keifu-w5-ghost.png"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut recorder = FrameRecorder::new(WINDOW);
        let mut sim = session(crate::verify::SEEDS[0]);
        for name in stage {
            if name.is_empty() {
                point_at(&mut sim, Target::OpenFamily, true);
            } else if *name == "w3" {
                stage_w3(&mut sim);
            } else if *name == "w4-drag" {
                crate::w4::stage_mid_drag(&mut sim);
            } else if *name == "w4-worst" {
                crate::w4::stage_worst(&mut sim);
            } else if *name == "end" {
                crate::verify::scroll_dock(&mut sim, -100.0);
            } else if *name == "w5-ghost" {
                crate::w5::stage_ghost_board(&mut sim);
            } else if *name == "q0" {
                point_at(&mut sim, Target::Quest(0), false);
            } else if *name == "w4-sheet" {
                let _ = crate::w4::seat_the_oracle(&mut sim);
                point_at(&mut sim, Target::Quest(0), false);
            } else {
                let id = hero_named(&sim, name);
                point_at(&mut sim, Target::Hero(id), false);
            }
        }
        let frame = crate::verify::frame(&mut recorder, &mut sim);
        // Where this recorder put each sprite, for the replay to match.
        let art = Art::load(&mut store());
        let ids: Vec<(TextureId, BackendTextureId)> = crate::art::ROLES
            .map(|(_, figure)| art.texture(figure).texture_id())
            .into_iter()
            .map(|id| (id, recorder.texture(id)))
            .collect();
        let said = capture(checks, &frame, recorder.font_texture(), &ids, file);
        lines.push(if index == 0 {
            format!("capture: {said}")
        } else {
            format!("also captured: {said}")
        });
    }
    lines
}

/// W3's staged sheets (`w3::stage_settled`) on the session's own house.
fn stage_w3(sim: &mut HeadlessSim) {
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => crate::checks::fail("the content did not load", &error.to_string()),
    };
    crate::w3::stage_settled(
        &content,
        sim.world_mut().resource_mut::<crate::house::House>(),
    );
}

fn capture(
    checks: &mut Checks,
    frame: &FrameRecord,
    font: BackendTextureId,
    art: &[(TextureId, BackendTextureId)],
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
    let mut textures = create_builtin_textures(&mut gpu);
    // The same sprites, asked for in the same order, resolved and uploaded: a store
    // only has texels for what was loaded, so a replay that skipped this would
    // render placeholders where the plan names the cast.
    let mut assets = store();
    let _ = Art::load(&mut assets);
    let _ = assets.commit(ART_ARRIVES);
    upload_ready_textures(&mut assets, &mut gpu, &mut textures);
    checks.require(
        textures.resolve(FONT_TEXTURE) == font
            && art
                .iter()
                .all(|(id, recorded)| textures.resolve(*id) == *recorded),
        "the replay's texture ids do not mean what the recorded plan means",
        format!(
            "recorder font {font:?}, backend font {:?}; sprites recorded {:?}, replayed {:?}",
            textures.resolve(FONT_TEXTURE),
            art.iter().map(|(_, b)| *b).collect::<Vec<_>>(),
            art.iter()
                .map(|(id, _)| textures.resolve(*id))
                .collect::<Vec<_>>()
        ),
    );
    if textures.resolve(FONT_TEXTURE) != font
        || art
            .iter()
            .any(|(id, recorded)| textures.resolve(*id) != *recorded)
    {
        return format!("{file} not written: the replay's texture ids drifted");
    }
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
