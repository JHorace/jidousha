//! Judging what a photographed screen drew: every row and icon of the
//! screen's content, found on the recorded frame — one layout, two readers.
//!
//! The chrome lives in UI space and is placed by `UiMap` at draw time, so
//! the judge transforms each expected row through the same mapping the frame
//! was drawn with before looking for its glyphs; map-space rows are looked
//! for where they are.

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord};

use crate::camera::UiMap;
use crate::checks::{Checks, near};
use crate::constants::Tuning;
use crate::sweep::{Conducted, Shot};
use crate::{screens, ui, verify};

/// How many of a row's glyphs were drawn, counted inside the row's own
/// world-space box.
pub fn glyph_run(
    frame: &FrameRecord,
    font: BackendTextureId,
    at: Vec2,
    size: f32,
    width: f32,
) -> usize {
    frame
        .quads()
        .iter()
        .filter(|quad| {
            quad.texture == font
                && near(quad.bounds().min.y, at.y)
                && quad.bounds().min.x >= at.x - 0.5
                && quad.bounds().max.x <= at.x + width + 0.5 + size * 0.01
        })
        .count()
}

/// Every row and icon of the shot's content, found on its frame.
pub fn judge_chrome(checks: &mut Checks, run: &Conducted, shot: &Shot, what: &str) {
    let tuning = Tuning::SHIPPED;
    let map = UiMap::for_camera(&verify::run_camera(verify::HEADLESS_VIEWPORT));
    let view = verify::run_camera(verify::HEADLESS_VIEWPORT).visible_bounds();
    let panel = screens::content(
        &shot.flow,
        &crate::lens::Lens::on(&shot.sim),
        &shot.clock,
        &tuning,
    );
    let style_width = |text: &str, size: f32| {
        TextStyle {
            face: Face::BUILT_IN,
            size,
            ..TextStyle::default()
        }
        .width_of(text)
    };
    for run_text in &panel.runs {
        let at = map.to_world(run_text.at);
        let size = run_text.size * map.scale;
        let width = style_width(&run_text.text, size);
        let drawn = glyph_run(&shot.frame, run.font, at, size, width);
        checks.require(
            drawn == run_text.text.chars().count(),
            "a row of the chrome is not drawn as the string it is",
            format!(
                "{what}: {:?} at ({:.1}, {:.1}) is {} characters and {drawn} glyphs landed in \
                 its box",
                run_text.text,
                at.x,
                at.y,
                run_text.text.chars().count(),
            ),
        );
    }
    for run_text in &panel.world_runs {
        // Map-space rows are culled to the camera, so only the visible ones
        // owe the frame their glyphs.
        if !run_text.bounds().overlaps(view) {
            continue;
        }
        let width = style_width(&run_text.text, run_text.size);
        let drawn = glyph_run(&shot.frame, run.font, run_text.at, run_text.size, width);
        checks.require(
            drawn == run_text.text.chars().count(),
            "a map label is not drawn as the string it is",
            format!(
                "{what}: {:?} at ({:.1}, {:.1}) is {} characters and {drawn} glyphs landed in \
                 its box",
                run_text.text,
                run_text.at.x,
                run_text.at.y,
                run_text.text.chars().count(),
            ),
        );
    }
    for icon in &panel.icons {
        let at = map.to_world(icon.at);
        judge_icon(checks, run, shot, what, icon, at);
    }
    for icon in &panel.world_icons {
        if !icon.bounds().overlaps(view) {
            continue;
        }
        judge_icon(checks, run, shot, what, icon, icon.at);
    }
}

fn judge_icon(
    checks: &mut Checks,
    run: &Conducted,
    shot: &Shot,
    what: &str,
    icon: &ui::IconRun,
    at: Vec2,
) {
    let covered = shot.frame.quads().iter().any(|quad| {
        quad.texture != run.font
            && near(quad.bounds().min.x, at.x)
            && near(quad.bounds().min.y, at.y)
    });
    checks.require(
        covered,
        "an icon the screen says it draws is not on the frame",
        format!(
            "{what}: {:?} at ({:.1}, {:.1}) has no quad",
            icon.art, at.x, at.y
        ),
    );
}
