//! Judging what a photographed screen drew: every row and icon of the
//! screen's content, found on the recorded frame — one layout, two readers.
//!
//! The content is this game's (`screens::content`, over its lens, grid,
//! clock and tuning); the finding is the kit's (`jidousha::ui::judge_frame`,
//! ADR-0046). The chrome lives in UI space and is placed by `UiMap` at draw
//! time, so the judge transforms each expected row through the same mapping
//! the frame was drawn with before looking for its glyphs; map-space rows are
//! looked for where they are.

use crate::camera::UiMap;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::screens;
use crate::sweep::{Conducted, Shot};

/// Every row and icon of the shot's content, found on its frame.
pub fn judge_chrome(checks: &mut Checks, run: &Conducted, shot: &Shot, what: &str) {
    let tuning = Tuning::SHIPPED;
    // **The camera the frame was drawn with**, not the one the run opened at:
    // since the legibility session the map's words are a function of the zoom
    // and the chrome's positions are a function of the mapping, so a reader
    // that assumed the default camera would look for every row in the wrong
    // box on a photograph taken anywhere else (UI.md §4).
    let map = UiMap::for_camera(&shot.camera);
    let view = shot.camera.visible_bounds();
    let grid = crate::grid::grid();
    let panel = screens::content(
        &shot.flow,
        &crate::lens::Lens::on(&shot.sim),
        &grid,
        &shot.clock,
        &tuning,
        screens::reading(&shot.clock, &tuning, screens::TICK),
        &shot.camera,
    );
    for breach in jidousha::ui::judge_frame(&panel, &shot.frame, run.font, &map, view) {
        checks.require(false, breach.what, format!("{what}: {}", breach.detail));
    }
}
