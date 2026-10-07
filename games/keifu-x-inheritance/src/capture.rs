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
//! and W5's staged ghost: "Lay Garrick's ghost" on a year-2 board. Then W6's telling:
//! the stay-home Meanwhile, a story half typed, the played page whole, a forging (the
//! cradle-ring, with Garrick's sheet in the dock), and the closed house's verdict. Then
//! W7's: Odo held over Pip's bench with the "+1 Spirit" preview, the played winter's
//! hearth seated with every preview, the winter page, Pip's and Odo's sheets after it,
//! and Ysolde's road-book on her sheet beside the played winter's page. Then W8's turning
//! and grown house, and W9's: Elsbeth's epitaph on the family screen's remembrance panel,
//! and a questing death's and an old age's epitaph at the top of their death pages. Then
//! W10's (`w10_stages.rs`): the Door card empty, mid-drag and seated, its sheet and its
//! help; the prologue and a lock's page; each verdict; the family at the Ending, staged
//! and played; and another house begun.

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
        (&["Garrick"][..], "keifu-x-inheritance.png"),
        (&["Wren"][..], "keifu-x-inheritance-child.png"),
        (&["", "Maren"][..], "keifu-x-inheritance-family.png"),
        (&["w3", "Garrick"][..], "keifu-x-inheritance-w3-garrick.png"),
        (&["w4-drag"][..], "keifu-x-inheritance-w4-drag.png"),
        (&["w4-sheet"][..], "keifu-x-inheritance-w4-sheet.png"),
        (&["w4-worst"][..], "keifu-x-inheritance-w4-worst.png"),
        (
            &["w4-worst", "q0"][..],
            "keifu-x-inheritance-w4-worst-sheet.png",
        ),
        (&[][..], "keifu-x-inheritance-dock-idle.png"),
        (
            &["Garrick", "end"][..],
            "keifu-x-inheritance-dock-scrolled.png",
        ),
        (&["w5-ghost"][..], "keifu-x-inheritance-w5-ghost.png"),
        (&["w6-stay"][..], "keifu-x-inheritance-w6-stay-home.png"),
        (&["w6-typing"][..], "keifu-x-inheritance-w6-typing.png"),
        (&["w6-page"][..], "keifu-x-inheritance-w6-page.png"),
        (&["w6-ring"][..], "keifu-x-inheritance-w6-ring.png"),
        (&["w6-closed"][..], "keifu-x-inheritance-w6-closed.png"),
        (&["w7-drag"][..], "keifu-x-inheritance-w7-drag.png"),
        (&["w7-seated"][..], "keifu-x-inheritance-w7-hearth.png"),
        (&["w7-winter"][..], "keifu-x-inheritance-w7-winter.png"),
        (&["w7-winter", "Pip"][..], "keifu-x-inheritance-w7-pip.png"),
        (&["w7-winter", "Odo"][..], "keifu-x-inheritance-w7-odo.png"),
        (
            &["w7-played", "Ysolde"][..],
            "keifu-x-inheritance-w7-road-book.png",
        ),
        (
            &["w8-stirred", "heir:Maren"][..],
            "keifu-x-inheritance-w8-death.png",
        ),
        (
            &["w8-oracle", "choose:Maren"][..],
            "keifu-x-inheritance-w8-chosen.png",
        ),
        (&["w8-turned"][..], "keifu-x-inheritance-w8-winter.png"),
        (
            &["w8-turned", "kind:Death"][..],
            "keifu-x-inheritance-w8-death-decided.png",
        ),
        (
            &["w8-turned", "kind:Birth"][..],
            "keifu-x-inheritance-w8-birth.png",
        ),
        (
            &["w8-turned", "kind:ComingOfAge"][..],
            "keifu-x-inheritance-w8-coming-of-age.png",
        ),
        (
            &["w8-turned", "kind:Arrival"][..],
            "keifu-x-inheritance-w8-arrival.png",
        ),
        (
            &["w8-turned", "kind:Year"][..],
            "keifu-x-inheritance-w8-year.png",
        ),
        (&["w8-grown"][..], "keifu-x-inheritance-w8-grown.png"),
        (
            &["w8-grown", ""][..],
            "keifu-x-inheritance-w8-grown-family.png",
        ),
        (&["", "Elsbeth"][..], "keifu-x-inheritance-w9-elsbeth.png"),
        (
            &["w9-questing"][..],
            "keifu-x-inheritance-w9-questing-death.png",
        ),
        (&["w9-old-age"][..], "keifu-x-inheritance-w9-old-age.png"),
        (
            &["w10:door-empty"][..],
            "keifu-x-inheritance-w10-door-empty.png",
        ),
        (
            &["w10:door-help"][..],
            "keifu-x-inheritance-w10-door-help.png",
        ),
        (
            &["w10:door-drag"][..],
            "keifu-x-inheritance-w10-door-drag.png",
        ),
        (
            &["w10:door-card"][..],
            "keifu-x-inheritance-w10-door-card.png",
        ),
        (
            &["w10:door-sheet"][..],
            "keifu-x-inheritance-w10-door-sheet.png",
        ),
        (
            &["w10:prologue"][..],
            "keifu-x-inheritance-w10-prologue.png",
        ),
        (&["w10:lock"][..], "keifu-x-inheritance-w10-lock.png"),
        (
            &["w10:verdict-0"][..],
            "keifu-x-inheritance-w10-verdict-0.png",
        ),
        (
            &["w10:verdict-1"][..],
            "keifu-x-inheritance-w10-verdict-1.png",
        ),
        (
            &["w10:verdict-2"][..],
            "keifu-x-inheritance-w10-verdict-2.png",
        ),
        (
            &["w10:verdict-3"][..],
            "keifu-x-inheritance-w10-verdict-3.png",
        ),
        (&["w10:family"][..], "keifu-x-inheritance-w10-family.png"),
        (&["w10:played"][..], "keifu-x-inheritance-w10-played.png"),
        (
            &["w10:played-family"][..],
            "keifu-x-inheritance-w10-played-family.png",
        ),
        (&["w10:reset"][..], "keifu-x-inheritance-w10-reset.png"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut recorder = FrameRecorder::new(WINDOW);
        let mut sim = if stage.first() == Some(&"w8-grown") {
            crate::floors_w8::grown(crate::verify::SEEDS[0], 15)
        } else {
            session(crate::verify::SEEDS[0])
        };
        for name in stage {
            if *name == "w8-grown" {
                continue;
            } else if let Some(kind) = name.strip_prefix("kind:") {
                use crate::passage::PageKind;
                let kind = match kind {
                    "Death" => PageKind::Death,
                    "Birth" => PageKind::Birth,
                    "ComingOfAge" => PageKind::ComingOfAge,
                    "Arrival" => PageKind::Arrival,
                    _ => PageKind::Year,
                };
                crate::w8::turn_to_kind(&mut sim, kind);
                // A frame for the page to settle under the pointer at rest.
                crate::verify::point(&mut sim, jidousha::prelude::Vec2::new(4.0, 700.0), false);
                continue;
            } else if let Some(heir) = name.strip_prefix("heir:").or(name.strip_prefix("choose:")) {
                let id = hero_named(&sim, heir);
                let target = crate::w8::heir_labels(&crate::verify::page_of(&sim))
                    .into_iter()
                    .map(|(t, _)| t)
                    .find(|t| matches!(t, Target::Heir(_, Some(h)) if *h == id));
                if let Some(target) = target {
                    point_at(&mut sim, target, name.starts_with("choose:"));
                }
                continue;
            } else if *name == "w8-stirred" || *name == "w8-oracle" {
                crate::w8::stage_garricks_winter(&mut sim);
                if *name == "w8-stirred" {
                    let _ = crate::w8::stir(&mut sim, true);
                }
                point_at(&mut sim, Target::LetWinterPass, true);
                crate::w8::go_to_the_choice(&mut sim);
                continue;
            } else if *name == "w8-turned" {
                crate::w8::stage_turned_year(&mut sim);
                continue;
            } else if *name == "w9-questing" || *name == "w9-old-age" {
                if *name == "w9-questing" {
                    crate::w9::stage_questing_death(&mut sim);
                } else {
                    crate::w9::stage_old_age(&mut sim);
                }
                // A frame for the page to settle under the pointer at rest.
                crate::verify::point(&mut sim, jidousha::prelude::Vec2::new(4.0, 700.0), false);
                continue;
            }
            if let Some(w10) = name.strip_prefix("w10:") {
                crate::w10_stages::stage(&mut sim, w10);
            } else if name.is_empty() {
                point_at(&mut sim, Target::OpenFamily, true);
            } else if *name == "w3" {
                stage_w3(&mut sim);
            } else if *name == "w4-drag" {
                crate::w4::stage_mid_drag(&mut sim);
            } else if *name == "w4-worst" {
                crate::w4::stage_worst(&mut sim);
            } else if *name == "end" {
                crate::verify::scroll_dock(&mut sim, -100.0);
            } else if *name == "w6-stay" {
                crate::w6_stages::stage_stay_home(&mut sim);
            } else if *name == "w6-typing" {
                crate::w6_stages::stage_typing(&mut sim);
            } else if *name == "w6-page" {
                crate::w6_stages::stage_page(&mut sim);
            } else if *name == "w6-ring" {
                crate::w6_stages::stage_ring(&mut sim);
            } else if *name == "w6-closed" {
                crate::w6_stages::stage_closed(&mut sim);
            } else if *name == "w7-drag" {
                let _held = crate::w7::stage_mid_drag(&mut sim);
            } else if *name == "w7-seated" {
                crate::w7::stage_played_seating(&mut sim);
            } else if *name == "w7-winter" {
                crate::w7::stage_oracle_winter(&mut sim);
            } else if *name == "w7-played" {
                let _ = crate::w7::stage_played_winter(&mut sim);
                point_at(&mut sim, Target::Leaf(0), true);
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
