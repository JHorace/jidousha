//! Every UI rectangle on screen, as free functions — one layout, three
//! readers.
//!
//! **UI space is 960x540 reference pixels**, exactly as giri's design rect
//! was; the map camera pans and zooms underneath, and `camera::UiMap` is the
//! one conversion that places this layout inside whatever the camera shows.
//! At the default camera on a 16:9 surface one UI unit is one world unit is
//! one reference pixel, so UI.md §7's floors — the 12px text floor, the
//! 32x32 target floor — are stated in these numbers directly.
//!
//! The functions are free and public because three readers want them: the
//! draw systems place things with them, `flow.rs` hit-tests the pointer
//! against them, and `floors.rs` asserts the target and overlap floors over
//! them. A rectangle only the draw system knows is a rectangle nothing can
//! check.

use jidousha::prelude::*;

/// The reference surface, in pixels.
pub const REFERENCE: PhysicalSize = PhysicalSize::new(960, 540);
/// UI space's width — one unit per reference pixel.
pub const DESIGN_W: f32 = REFERENCE.width as f32;
/// And its height.
pub const DESIGN_H: f32 = REFERENCE.height as f32;

/// The whole UI rect.
pub fn design() -> Rect {
    Rect::from_min_size(Vec2::ZERO, Vec2::new(DESIGN_W, DESIGN_H))
}

// ── top bar: title, clock, speed chips, treasury, drawer handles ───────────

/// The status bar across the top.
pub fn topbar() -> Rect {
    Rect::from_min_size(Vec2::ZERO, Vec2::new(DESIGN_W, 36.0))
}

/// The title's top-left.
pub fn title_at() -> Vec2 {
    Vec2::new(10.0, 11.0)
}

/// The clock readout's top-left — always visible (DESIGN §4).
pub fn clock_at() -> Vec2 {
    Vec2::new(96.0, 11.0)
}

/// How many speed chips there are: PAUSE, 1x, 2x, 4x.
pub const CHIPS: usize = 4;

/// Speed chip `index` — a clickable target, so the 32x32 floor binds it.
pub fn speed_chip(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(206.0 + index as f32 * 52.0, 2.0),
        Vec2::new(48.0, 32.0),
    )
}

/// The treasury's coin icon (16x16 at scale 2) — the redundancy floor's icon.
pub fn treasury_icon_at() -> Vec2 {
    Vec2::new(452.0, 10.0)
}

/// The treasury's number, beside its coin.
pub fn treasury_text_at() -> Vec2 {
    Vec2::new(472.0, 11.0)
}

/// The log drawer's handle, in the status bar.
pub fn log_button() -> Rect {
    Rect::from_min_size(Vec2::new(752.0, 2.0), Vec2::new(80.0, 32.0))
}

/// The tuning drawer's handle, beside the log's.
pub fn tune_button() -> Rect {
    Rect::from_min_size(Vec2::new(656.0, 2.0), Vec2::new(80.0, 32.0))
}

/// The toast row, under the bar — a bounced order's arithmetic lands here.
pub fn toast_at() -> Vec2 {
    Vec2::new(16.0, 44.0)
}

// ── party strip ────────────────────────────────────────────────────────────

/// The always-present strip at the bottom: one chip per party.
pub fn party_strip() -> Rect {
    Rect::from_min_size(Vec2::new(0.0, 484.0), Vec2::new(DESIGN_W, DESIGN_H - 484.0))
}

/// The strip's label.
pub fn party_label() -> Vec2 {
    Vec2::new(16.0, 487.0)
}

const PCHIP_X: f32 = 16.0;
const PCHIP_Y: f32 = 500.0;
const PCHIP_W: f32 = 286.0;
const PCHIP_H: f32 = 36.0;
const PCHIP_GAP: f32 = 12.0;

/// Party chip `index` — click to select an idle party for dispatch.
pub fn party_chip(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(PCHIP_X + index as f32 * (PCHIP_W + PCHIP_GAP), PCHIP_Y),
        Vec2::new(PCHIP_W, PCHIP_H),
    )
}

/// The rows inside a party chip, measured from its top-left.
pub mod pchip {
    /// The portrait's inset.
    pub const PORTRAIT: f32 = 2.0;
    /// The portrait's scale (16 texels at 2 = 32).
    pub const PORTRAIT_SCALE: f32 = 2.0;
    /// Where the name starts.
    pub const NAME_X: f32 = 40.0;
    /// The name's top.
    pub const NAME_TOP: f32 = 4.0;
    /// The status line's top.
    pub const STATUS_TOP: f32 = 20.0;
}

// ── log drawer ─────────────────────────────────────────────────────────────

/// The log drawer, over the map and under the party strip.
pub fn log_panel() -> Rect {
    Rect::from_min_size(Vec2::new(0.0, 36.0), Vec2::new(DESIGN_W, 448.0))
}

/// Where log line `index` is drawn inside the drawer.
pub fn log_row(index: usize) -> Vec2 {
    Vec2::new(28.0, 76.0 + index as f32 * 20.0)
}

/// How many log rows the drawer has room for. Older entries stay in the
/// resource and scroll off the view.
pub const LOG_ROWS: usize = 19;

/// The drawer's own title row.
pub fn log_title() -> Vec2 {
    Vec2::new(28.0, 50.0)
}

// ── the tuning drawer (giri's geometry, at eight constants) ────────────────

/// The tuning drawer: from under the status bar to the bottom of the screen.
pub fn tuner_panel() -> Rect {
    Rect::from_min_size(Vec2::new(0.0, 36.0), Vec2::new(DESIGN_W, DESIGN_H - 36.0))
}

/// How many stepper rows a column holds before the next one starts.
pub const TUNER_ROWS: usize = 8;
const TUNER_COL_X: f32 = 28.0;
const TUNER_COL_PITCH: f32 = 312.0;
const TUNER_ROW_Y: f32 = 110.0;
const TUNER_ROW_PITCH: f32 = 34.0;
/// The steppers' - and + size: the smallest target in the game, exactly the
/// floor.
const TUNER_STEP: f32 = 32.0;
/// The width the longest constant name needs (`minute_ticks` and friends).
const TUNER_NAME_W: f32 = 164.0;
/// The gap the value sits in, between the two buttons.
const TUNER_VALUE_W: f32 = 36.0;

fn tuner_row_origin(index: usize) -> Vec2 {
    let column = index / TUNER_ROWS;
    let row = index % TUNER_ROWS;
    Vec2::new(
        TUNER_COL_X + column as f32 * TUNER_COL_PITCH,
        TUNER_ROW_Y + row as f32 * TUNER_ROW_PITCH,
    )
}

/// Where stepper row `index`'s name is drawn.
pub fn tuner_name(index: usize) -> Vec2 {
    tuner_row_origin(index) + Vec2::new(0.0, 10.0)
}

/// The - of stepper row `index`.
pub fn tuner_minus(index: usize) -> Rect {
    Rect::from_min_size(
        tuner_row_origin(index) + Vec2::new(TUNER_NAME_W, 0.0),
        Vec2::splat(TUNER_STEP),
    )
}

/// The + of stepper row `index`.
pub fn tuner_plus(index: usize) -> Rect {
    Rect::from_min_size(
        tuner_row_origin(index) + Vec2::new(TUNER_NAME_W + TUNER_STEP + TUNER_VALUE_W, 0.0),
        Vec2::splat(TUNER_STEP),
    )
}

/// The gap between them, where the value is centred.
pub fn tuner_value(index: usize) -> Rect {
    Rect::from_min_size(
        tuner_row_origin(index) + Vec2::new(TUNER_NAME_W + TUNER_STEP, 0.0),
        Vec2::new(TUNER_VALUE_W, TUNER_STEP),
    )
}

/// The whole of stepper row `index` — what a hover is tested against.
pub fn tuner_row(index: usize) -> Rect {
    Rect::from_min_size(
        tuner_row_origin(index),
        Vec2::new(TUNER_NAME_W + 2.0 * TUNER_STEP + TUNER_VALUE_W, TUNER_STEP),
    )
}

/// The label in front of the preset row.
pub fn tuner_presets_label() -> Vec2 {
    Vec2::new(TUNER_COL_X, tuner_preset(0).min.y + 10.0)
}

/// Preset button `index`.
pub fn tuner_preset(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(112.0 + index as f32 * 128.0, 72.0),
        Vec2::new(120.0, 32.0),
    )
}

/// The commit verb, on the preset row.
pub fn tuner_apply() -> Rect {
    Rect::from_min_size(Vec2::new(824.0, 72.0), Vec2::new(120.0, 32.0))
}

/// The drawer's title.
pub fn tuner_title() -> Vec2 {
    Vec2::new(TUNER_COL_X, 50.0)
}

/// The prose band under the steppers: the hint row first, then the note.
pub fn tuner_hint() -> Vec2 {
    Vec2::new(TUNER_COL_X, 432.0)
}

/// How wide the hint and the note may run before they wrap.
pub fn tuner_prose_width() -> f32 {
    652.0 - TUNER_COL_X - 16.0
}

/// The stamp: the constants actually in effect, always visible while the
/// drawer is open. Beside the single stepper column, which is short.
pub fn tuner_stamp() -> Vec2 {
    Vec2::new(400.0, 110.0)
}

// ── the map's own geometry (world units, not UI units) ─────────────────────

/// How big a location marker's click target is, in world units — 32, so the
/// target floor holds at the reference camera, where a world unit is a
/// reference pixel.
pub const MARKER: f32 = 32.0;

/// A location marker's rectangle, centred over its tile.
pub fn marker_rect(tile: crate::grid::Tile) -> Rect {
    Rect::from_center_size(tile.center(), Vec2::splat(MARKER))
}

/// Where a location's label starts, under its marker.
pub fn marker_label(tile: crate::grid::Tile, width: f32) -> Vec2 {
    tile.center() + Vec2::new(-width * 0.5, MARKER * 0.5 + 2.0)
}

/// A party token's size, in world units (16 texels at scale 2).
pub const TOKEN: f32 = 32.0;
