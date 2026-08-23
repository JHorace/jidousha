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
const PARTY_Y: f32 = 370.0;
const PCARD_W: f32 = 170.0;
const PCARD_H: f32 = 164.0;
const PCARD_GAP: f32 = 12.0;

/// The rows inside a party card, measured from its top edge.
///
/// Named rather than added up at each draw site, because `floors.rs` asserts
/// that the last of them ends inside the card and that the stat row's icons sit
/// beside their numbers — both of which are questions about these offsets.
pub mod pcard {
    /// The portrait, at three texels per texel.
    pub const PORTRAIT_TOP: f32 = 6.0;
    /// How big the portrait is drawn.
    pub const PORTRAIT_SCALE: f32 = 3.0;
    /// The name.
    pub const NAME_TOP: f32 = 58.0;
    /// The stat row: flame, eye, coin.
    pub const STATS_TOP: f32 = 76.0;
    /// The character's outgoing regard edges.
    pub const REGARD_TOP: f32 = 98.0;
    /// The status line UI.md §4 states the grammar of.
    pub const STATUS_TOP: f32 = 130.0;
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
    Vec2::new(PARTY_X, 352.0)
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
