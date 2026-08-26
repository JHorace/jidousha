//! Every rectangle on screen, as free functions — one layout, three readers.
//!
//! **One world unit is one reference pixel.** UI.md's reference resolution is
//! 960x540 and every number it states — the 12px text floor, the 32x32 target
//! floor — is stated in those pixels, so giri lays the world out at that scale
//! and the specification's numbers are the code's numbers. The camera spans
//! `DESIGN_H` world units at reference size and `scaling.rs` grows that span
//! to letterbox anything else; nothing below changes when the window does,
//! which is what "the layout is fixed and the *view* scales" means.
//!
//! The functions are free and public because three readers want them: the draw
//! systems place things with them, `flow.rs` hit-tests the pointer against
//! them, and `floors.rs` asserts UI.md §7's target and overlap floors over
//! them. A rectangle only the draw system knows is a rectangle nothing can
//! check.
//!
//! Origin is the design rect's top-left, `+y` is down (engine convention), and
//! the numbers are the approved mockup's own CSS geometry.

use jidousha::prelude::*;

/// The reference surface UI.md §6 names, in pixels.
pub const REFERENCE: PhysicalSize = PhysicalSize::new(960, 540);
/// The design rect's width in world units — one per reference pixel.
pub const DESIGN_W: f32 = REFERENCE.width as f32;
/// And its height.
pub const DESIGN_H: f32 = REFERENCE.height as f32;

/// The whole design rect: what is always on screen, whatever the window does.
pub fn design() -> Rect {
    Rect::from_min_size(Vec2::ZERO, Vec2::new(DESIGN_W, DESIGN_H))
}

// ── top bar ────────────────────────────────────────────────────────────────

/// The status bar: title, round, player gold.
pub fn topbar() -> Rect {
    Rect::from_min_size(Vec2::ZERO, Vec2::new(DESIGN_W, 36.0))
}

// ── quest row ──────────────────────────────────────────────────────────────

/// How many quest cards the board has room for (UI.md §3: "up to 4").
pub const QUEST_SLOTS: usize = 4;
const QUEST_X: f32 = 24.0;
const QUEST_Y: f32 = 56.0;
const QUEST_W: f32 = 138.0;
const QUEST_H: f32 = 172.0;
const QUEST_GAP: f32 = 16.0;

/// Where quest card `index` is.
pub fn quest_card(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(QUEST_X + index as f32 * (QUEST_W + QUEST_GAP), QUEST_Y),
        Vec2::new(QUEST_W, QUEST_H),
    )
}

// ── info panel ─────────────────────────────────────────────────────────────

/// The fixed panel on the right. Never a cursor-following bubble (UI.md §3).
pub fn info_panel() -> Rect {
    Rect::from_min_size(Vec2::new(640.0, 56.0), Vec2::new(296.0, 276.0))
}

/// The panel's text column, inside its padding.
pub fn info_content() -> Rect {
    let panel = info_panel();
    Rect::from_min_size(
        panel.min + Vec2::new(14.0, 12.0),
        panel.size() - Vec2::new(28.0, 24.0),
    )
}

/// The release control, which exists only while a quest is taken.
pub fn release_button() -> Rect {
    let panel = info_panel();
    Rect::from_min_size(
        Vec2::new(panel.min.x + 14.0, panel.max.y - 48.0),
        Vec2::new(200.0, 34.0),
    )
}

/// Where the beat's dilemma is written: the board's own empty quarter, under
/// the quest row and left of the panel.
///
/// The mockup leaves this space blank because its four quests fill the row;
/// giri's beats offer one apiece, and the sentence that says what the beat is
/// *about* has nowhere else to be that survives a quest being taken.
pub fn dilemma() -> Rect {
    Rect::from_min_size(Vec2::new(24.0, 240.0), Vec2::new(600.0, 92.0))
}

/// The log drawer's handle, in the status bar.
///
/// The mockup hangs it off the board's right edge, where it lands on the party
/// strip; the status bar has room and is where a secondary, always-available
/// control belongs. Moving it also gave the beat's dilemma the whole lower-left
/// band, which `floors.rs` had just caught it running into.
pub fn log_button() -> Rect {
    Rect::from_min_size(Vec2::new(752.0, 2.0), Vec2::new(80.0, 32.0))
}

/// The drawer itself, over the board and under the party strip (UI.md §3).
pub fn log_panel() -> Rect {
    Rect::from_min_size(Vec2::new(0.0, 36.0), Vec2::new(DESIGN_W, 308.0))
}

// ── the tuning drawer (UI.md §12) ──────────────────────────────────────────

/// The drawer's handle, in the status bar beside the log's.
///
/// The mockup puts it beside the gold counter; here it sits with the other
/// drawer handle, because they are the same kind of control and the bar is
/// where this game's secondary, always-available controls live (§10's log-handle
/// amendment, applied to the second drawer).
pub fn tune_button() -> Rect {
    Rect::from_min_size(Vec2::new(656.0, 2.0), Vec2::new(80.0, 32.0))
}

/// The tuning drawer: the log drawer's rectangle, exactly.
///
/// **Not the info column the mockup uses**, and the reason is the readability
/// floor rather than taste: ten constants at UI.md §7's 32x32 target floor is
/// 320 reference pixels of steppers alone, and the info column is 276 tall.
/// Three columns of rows in a drawer the width of the board fit with room for
/// the note and the stamp; the info column fits six constants and a scrollbar.
/// A drawer is already this game's shape for "an overlay over the board" and
/// the gold border is what says which drawer this one is (UI.md §12).
pub fn tuner_panel() -> Rect {
    log_panel()
}

/// How many stepper rows a column holds before the next one starts.
///
/// Five since P1: thirteen constants over three columns. The pitch tightened
/// to keep five 32-tall steppers inside the drawer's band — the target floor
/// is untouched, the air between rows is what got spent.
pub const TUNER_ROWS: usize = 5;
const TUNER_COL_X: f32 = 28.0;
const TUNER_COL_PITCH: f32 = 312.0;
const TUNER_ROW_Y: f32 = 110.0;
const TUNER_ROW_PITCH: f32 = 34.0;
/// How wide a stepper's - and + are, on both axes. The smallest target in the
/// game, and exactly UI.md §7's floor.
const TUNER_STEP: f32 = 32.0;
/// How far into a row the - sits: the width the constant's longest name needs.
/// `desperation_floor` is seventeen characters, which is 158.7 units at
/// `theme::SMALL` - so 164 leaves the name clear of the button.
const TUNER_NAME_W: f32 = 164.0;
/// The gap the value sits in, between the two buttons.
const TUNER_VALUE_W: f32 = 36.0;

/// The top-left of stepper row `index`, laid out down a column and then across.
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

/// The whole of stepper row `index` — what a hover is tested against, so that
/// pointing anywhere along a row gives that constant's one-line meaning.
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

/// The commit verb — on the preset row, right of the presets, because the
/// stepper columns grew down into the band it used to sit in (UI.md §13).
pub fn tuner_apply() -> Rect {
    Rect::from_min_size(Vec2::new(824.0, 72.0), Vec2::new(120.0, 32.0))
}

/// The drawer's title.
pub fn tuner_title() -> Vec2 {
    Vec2::new(TUNER_COL_X, 50.0)
}

/// The prose band under the steppers: the hint row first, then the note.
///
/// One band with two tenants in order rather than two fixed slots, because the
/// hint is one line for a constant's meaning and three for a refused
/// `?constants=` — and a note pinned under the taller case would sit in a hole
/// on every other frame.
pub fn tuner_hint() -> Vec2 {
    Vec2::new(TUNER_COL_X, 282.0)
}

/// How wide the hint and the note may run before they wrap.
///
/// Stops short of the stamp's column: the third stepper column is the short
/// one, the stamp sits under it, and a prose row that ran the drawer's whole
/// width would run straight through the readout.
pub fn tuner_prose_width() -> f32 {
    652.0 - TUNER_COL_X - 16.0
}

/// The stamp: the constants actually in effect, always visible while the drawer
/// is open (UI.md §12). Under the third stepper column, which is short.
pub fn tuner_stamp() -> Vec2 {
    Vec2::new(652.0, 216.0)
}

/// Where log line `index` is drawn inside the drawer.
pub fn log_row(index: usize) -> Vec2 {
    Vec2::new(28.0, 76.0 + index as f32 * 20.0)
}

/// How many log rows the drawer has room for before it would run past its own
/// bottom edge. Older entries stay in the resource and scroll off the view.
pub const LOG_ROWS: usize = 12;

// ── party strip ────────────────────────────────────────────────────────────

/// The always-present strip (UI.md §4).
pub fn party_strip() -> Rect {
    Rect::from_min_size(Vec2::new(0.0, 344.0), Vec2::new(DESIGN_W, DESIGN_H - 344.0))
}

const PARTY_X: f32 = 16.0;
const PARTY_Y: f32 = 360.0;
const PCARD_W: f32 = 170.0;
const PCARD_H: f32 = 176.0;
const PCARD_GAP: f32 = 12.0;

/// How big a quest icon is drawn, in reference pixels, at each of its three
/// sizes.
///
/// **Sizes rather than scales**, because the four quest icons are not all one
/// texel size: three are 8x8 and the vault is 16x16 (the owner's curation,
/// `art/kenney-manifest.json`). `Art::scale_across` turns each of these into that
/// art's own whole-number scale, so a row of them is even. Every number here is
/// a multiple of 16 for that reason — it has to divide by both.
pub mod quest_icon {
    /// On a quest card, in the board's row of four.
    pub const CARD: f32 = 64.0;
    /// In the detail panel beside the quest's name.
    pub const DETAIL: f32 = 48.0;
    /// On the resolution takeover's head.
    pub const TAKEOVER: f32 = 64.0;
}

/// The rows inside a party card, measured from its top edge.
///
/// Named rather than added up at each draw site, because `floors.rs` asserts
/// that the stat row's icons sit beside their numbers and `judge.rs` rebuilds
/// the card from the same offsets — both questions about these numbers.
///
/// **v2 card shape (interim, UI.md §13)**: the portrait moved to the left
/// edge and the name, stats and desperation source share the column beside
/// it, which is what bought the rows the sheet grew — trait chips, mark
/// lines and the verdict all live below the header block. The chip band
/// flows (one row per trait), the mark and regard rows follow it, and the
/// status line is pinned at the bottom.
pub mod pcard {
    /// The portrait's top, at the card's left edge.
    pub const PORTRAIT_TOP: f32 = 4.0;
    /// Its left inset.
    pub const PORTRAIT_LEFT: f32 = 6.0;
    /// How big the portrait is drawn.
    pub const PORTRAIT_SCALE: f32 = 3.0;
    /// Where the header column right of the portrait starts.
    pub const RIGHT_COL: f32 = 58.0;
    /// The name, in the header column.
    pub const NAME_TOP: f32 = 4.0;
    /// The stat row under it: flame, coin.
    pub const STATS_TOP: f32 = 22.0;
    /// The desperation source, under the stats.
    pub const SOURCE_TOP: f32 = 40.0;
    /// Where the trait chips start, full width.
    pub const TRAITS_TOP: f32 = 56.0;
    /// One chip row's pitch.
    pub const TRAIT_PITCH: f32 = 16.0;
    /// The status line UI.md §4 states the grammar of, pinned at the bottom.
    pub const STATUS_TOP: f32 = 136.0;
}

/// Where roster card `index` is.
pub fn party_card(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(PARTY_X + index as f32 * (PCARD_W + PCARD_GAP), PARTY_Y),
        Vec2::new(PCARD_W, PCARD_H),
    )
}

/// The strip's label.
pub fn party_label() -> Vec2 {
    Vec2::new(PARTY_X, 346.0)
}

/// The send verb. Hidden entirely until a quest is taken (UI.md §3).
pub fn send_button() -> Rect {
    Rect::from_min_size(Vec2::new(784.0, 486.0), Vec2::new(160.0, 40.0))
}

/// Where the stated reason a send is blocked is drawn — right-aligned, so this
/// is the *right* edge of the text and the caller measures back from it.
pub fn send_reason_right() -> Vec2 {
    Vec2::new(944.0, 462.0)
}

/// The second row of the dilemma band: the beat's lesson, or - while one is
/// up - the toast a bounced click raised.
///
/// One slot, two tenants, because they are the same kind of thing in the same
/// voice: a line under the dilemma saying what this beat is about. A toast that
/// had a band of its own would need a band nothing else could ever use, and the
/// board has no room to keep one empty.
pub fn beat_note() -> Vec2 {
    Vec2::new(dilemma().min.x, dilemma().min.y + 52.0)
}

/// How wide the note may run before it wraps.
pub fn beat_note_width() -> f32 {
    dilemma().size().x
}

// ── resolution takeover ────────────────────────────────────────────────────

/// The full-screen takeover, replacing the board entirely (UI.md §3).
pub fn takeover() -> Rect {
    design()
}

/// The quest's icon and name, at the top of the takeover.
pub fn takeover_head() -> Vec2 {
    Vec2::new(240.0, 34.0)
}

/// The column event cards and the drift ledger share.
pub fn takeover_column() -> Rect {
    Rect::from_min_size(Vec2::new(170.0, 132.0), Vec2::new(620.0, 300.0))
}

/// Where event card `index` starts, given the heights of the ones above it.
pub fn event_card(index: usize, tops: &[f32]) -> Rect {
    let column = takeover_column();
    let top = column.min.y + tops.iter().take(index).sum::<f32>();
    let height = tops.get(index).copied().unwrap_or(44.0);
    Rect::from_min_size(
        Vec2::new(column.min.x, top),
        Vec2::new(column.size().x, height),
    )
}

/// The hint that a click anywhere returns to the board.
pub fn takeover_hint() -> Vec2 {
    Vec2::new(300.0, DESIGN_H - 34.0)
}
