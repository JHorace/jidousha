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

/// How wide a drawer handle is, and how far apart they sit.
///
/// **Seventy-two since wave 1.2**, where four handles of eighty fitted and
/// five do not: the ledger is the fifth drawer, and the row of them still has
/// to start right of the treasury. Seventy-two holds `ROSTER`, the longest
/// label, with room, and it is twice the target floor.
const HANDLE_W: f32 = 72.0;
const HANDLE_PITCH: f32 = 80.0;
const HANDLE_X: f32 = 528.0;

/// Drawer handle `index` in the status bar's row of them.
fn handle(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(HANDLE_X + index as f32 * HANDLE_PITCH, 2.0),
        Vec2::new(HANDLE_W, 32.0),
    )
}

/// The tuning drawer's handle, at the head of the row.
pub fn tune_button() -> Rect {
    handle(0)
}

/// The roster drawer's handle — every character in one list (wave 1.1's
/// clarity slice). The `r` key opens the same drawer.
pub fn roster_button() -> Rect {
    handle(1)
}

/// **The postings ledger's handle** (wave 1.2): every posting the player has
/// made, and the standing rates that price them.
pub fn ledger_button() -> Rect {
    handle(2)
}

/// The feed drawer's handle.
pub fn feed_button() -> Rect {
    handle(3)
}

/// The auto-pause config drawer's handle, at the end of the row.
pub fn modes_button() -> Rect {
    handle(4)
}

// ── the meters band: the aggregates for the glance (GDD §3) ────────────────

/// The band the meter chips sit in, under the status bar.
pub fn meters_band() -> Rect {
    Rect::from_min_size(Vec2::new(0.0, 36.0), Vec2::new(DESIGN_W, 40.0))
}

/// Meter chip `index` — click it for the faces behind the count.
pub fn meter_chip(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(16.0 + index as f32 * 158.0, 40.0),
        Vec2::new(150.0, 32.0),
    )
}

/// The rows inside a meter chip, from its top-left.
pub mod mchip {
    /// The icon's inset.
    pub const ICON: f32 = 8.0;
    /// The label's left.
    pub const LABEL_X: f32 = 30.0;
    /// The label's top.
    pub const LABEL_TOP: f32 = 10.0;
}

/// The pause banner, under the meters — why the world stopped itself.
pub fn banner_at() -> Vec2 {
    Vec2::new(16.0, 82.0)
}

/// The toast row, under the banner — a bounced order's arithmetic lands here.
pub fn toast_at() -> Vec2 {
    Vec2::new(16.0, 98.0)
}

// ── the faces list: what a meter chip opens into ───────────────────────────

/// How many faces the list has room for.
pub const FACE_ROWS: usize = 4;

/// The panel a drilled meter chip opens.
pub fn faces_panel() -> Rect {
    Rect::from_min_size(Vec2::new(16.0, 116.0), Vec2::new(300.0, 184.0))
}

/// Its title row.
pub fn faces_title() -> Vec2 {
    Vec2::new(28.0, 124.0)
}

/// Face row `index` — click a face for that character's panel.
pub fn faces_row(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(24.0, 140.0 + index as f32 * 36.0),
        Vec2::new(284.0, 32.0),
    )
}

// ── the site panel: one site's job board (UI.md §3c) ───────────────────────

/// How many job rows the board has room for.
///
/// Six, which is what a site is authored with (`sim::authored_sites`) — and
/// `verify` asserts no site holds more, because a job with no row is a job
/// nobody can be sent to now that the row *is* the order.
pub const BOARD_ROWS: usize = 6;

/// The panel a tapped site marker opens.
///
/// Left of the character panel and clear of it, because the two are up
/// together through the whole of a dispatch: the board says what the work is
/// and the panel says who is being sent.
pub fn board_panel() -> Rect {
    Rect::from_min_size(Vec2::new(16.0, 112.0), Vec2::new(576.0, 328.0))
}

/// The board's title row: which site, and how much of it is open.
pub fn board_title() -> Vec2 {
    Vec2::new(28.0, 122.0)
}

/// The travel line under it — the selected character's own journey to here.
pub fn board_travel() -> Vec2 {
    Vec2::new(28.0, 140.0)
}

/// How wide either header row may run before it is clipped — up to the fit
/// chip beside them.
pub const BOARD_HEAD_W: f32 = 360.0;

/// The board's close button.
pub fn board_close() -> Rect {
    Rect::from_min_size(Vec2::new(556.0, 120.0), Vec2::new(36.0, 32.0))
}

/// Job row `index` — **the posting target** (wave 1.2). Tapping an open row
/// posts it to the selected character at the standing rate.
///
/// The whole width of the board, because the wage and the who are the
/// **board's** controls and not the row's: a stepper inside a row would be a
/// control inside a control, which the overlap floor refuses, and a row cut
/// short to make room for one would leave no room for the verdict's reason —
/// which is the sentence this wave exists to put on screen.
pub fn board_row(index: usize) -> Rect {
    Rect::from_min_size(board_row_origin(index), Vec2::new(564.0, 32.0))
}

fn board_row_origin(index: usize) -> Vec2 {
    Vec2::new(24.0, 164.0 + index as f32 * 38.0)
}

/// The board's **wage down** button — what the next tap would offer.
///
/// One wage control for the board rather than one per row: the wage is what
/// you are offering *now*, the row you tap is what you are offering it for,
/// and six steppers would be six controls inside six targets.
pub fn board_wage_down() -> Rect {
    Rect::from_min_size(Vec2::new(388.0, 396.0), Vec2::splat(32.0))
}

/// Where the wage a tap would offer is drawn, between its two steppers.
pub fn board_wage_value() -> Rect {
    Rect::from_min_size(Vec2::new(420.0, 396.0), Vec2::new(40.0, 32.0))
}

/// The board's **wage up** button.
pub fn board_wage_up() -> Rect {
    Rect::from_min_size(Vec2::new(460.0, 396.0), Vec2::splat(32.0))
}

/// The board's **who** toggle: post to the selected character, or to anyone.
///
/// The targeted/open choice, made before the tap and readable while it is
/// being made — where a per-row OPEN button would put the choice inside the
/// same gesture that commits it.
pub fn board_to() -> Rect {
    Rect::from_min_size(Vec2::new(500.0, 396.0), Vec2::new(88.0, 32.0))
}

/// The board's **fit chip** — what the fit column means, and what it does not
/// mean yet (wave 1.2's clarity rider), tapped like any other chip.
///
/// In the header band, right of the travel line and left of the close: the
/// footer is the hint's and the wage's, and the rows are targets — a chip
/// among them would overlap one or lie across the other, and both are
/// floors.
pub fn board_fit_chip() -> Rect {
    Rect::from_min_size(Vec2::new(400.0, 116.0), Vec2::new(124.0, 32.0))
}

/// The footer hint, under the rows — a wrapped block since wave 1.2, because
/// the fit chip's explanation is a sentence and not a label.
pub fn board_hint() -> Vec2 {
    Vec2::new(28.0, 398.0)
}

/// How wide it may run before it wraps — narrower than the panel, because
/// the wage and the who stand at the end of the same band.
pub const BOARD_HINT_W: f32 = 352.0;

/// The columns inside a job row, as offsets from its top-left — two lines:
/// **what the work is and what it pays** over **what kind it is, who has it,
/// and how the selected character fits it**.
pub mod job {
    use jidousha::prelude::Vec2;

    /// The job's name.
    pub const NAME: Vec2 = Vec2::new(8.0, 3.0);
    /// How wide that may run.
    pub const NAME_W: f32 = 180.0;
    /// The pot.
    pub const POT: Vec2 = Vec2::new(196.0, 3.0);
    /// How wide that may run.
    pub const POT_W: f32 = 50.0;
    /// How long the work takes.
    pub const DURATION: Vec2 = Vec2::new(254.0, 3.0);
    /// How wide that may run.
    pub const DURATION_W: f32 = 80.0;
    /// The task-type chip's icon.
    pub const TASK_ICON: Vec2 = Vec2::new(8.0, 14.0);
    /// And its word.
    pub const TASK_NAME: Vec2 = Vec2::new(28.0, 17.0);
    /// How wide that may run.
    pub const TASK_W: f32 = 56.0;
    /// The selected character's fit for this work.
    pub const FIT: Vec2 = Vec2::new(92.0, 17.0);
    /// How wide that may run.
    pub const FIT_W: f32 = 60.0;
    /// **What the row has to say about itself**: what has become of it, or —
    /// where it is open and somebody is selected — what the scorer says they
    /// would do about a posting here, and why (wave 1.2).
    ///
    /// One cell, because the two readings never both apply: a row that is
    /// somebody's is not a row anybody can be asked for, and a verdict is
    /// strictly more than the word `open` it replaces. The widest cell on the
    /// row, because a verdict without its reason is a number about a person.
    pub const SAYS: Vec2 = Vec2::new(160.0, 17.0);
    /// How wide that may run.
    pub const SAYS_W: f32 = 396.0;
}

// ── the character panel ────────────────────────────────────────────────────

/// The panel a selected character opens.
pub fn person_panel() -> Rect {
    Rect::from_min_size(Vec2::new(600.0, 116.0), Vec2::new(344.0, 300.0))
}

/// Its close button.
pub fn person_close() -> Rect {
    Rect::from_min_size(Vec2::new(900.0, 122.0), Vec2::new(36.0, 32.0))
}

/// How many trait chips a sheet has room for across the panel.
///
/// Three, which is `CAST.md` §3.3's authoring norm and the width the panel
/// has; the registry's coverage check is what keeps the cast inside it.
pub const SHEET_CHIPS: usize = 3;

/// Trait chip `slot` on the character panel — **a click target**, because a
/// chip anywhere it appears opens its one-line explanation.
pub fn sheet_chip(slot: usize) -> Rect {
    Rect::from_min_size(
        person_panel().min + Vec2::new(12.0 + slot as f32 * 106.0, 70.0),
        Vec2::new(102.0, 32.0),
    )
}

/// The rows inside the character panel, as offsets from the panel's top-left.
pub mod sheet {
    use jidousha::prelude::Vec2;

    /// The portrait's inset.
    pub const PORTRAIT: Vec2 = Vec2::new(12.0, 12.0);
    /// Its scale (16 texels at 2 = 32 units).
    pub const PORTRAIT_SCALE: f32 = 2.0;
    /// Where the name sits.
    pub const NAME: Vec2 = Vec2::new(52.0, 18.0);
    /// The traits heading.
    pub const TRAITS: Vec2 = Vec2::new(12.0, 52.0);
    /// The wallet's coin.
    pub const WALLET_ICON: Vec2 = Vec2::new(12.0, 110.0);
    /// And its number.
    pub const WALLET_TEXT: Vec2 = Vec2::new(34.0, 112.0);
    /// The desperation flame.
    pub const NEED_ICON: Vec2 = Vec2::new(12.0, 132.0);
    /// And its number.
    pub const NEED_TEXT: Vec2 = Vec2::new(34.0, 134.0);
    /// The source line, wrapped.
    pub const SOURCE: Vec2 = Vec2::new(12.0, 156.0);
    /// What they are doing and why, wrapped.
    pub const DOING: Vec2 = Vec2::new(12.0, 198.0);
    /// Where they live.
    pub const HOME: Vec2 = Vec2::new(12.0, 240.0);
    /// The chip explanation, when a chip has been tapped.
    pub const EXPLAIN: Vec2 = Vec2::new(12.0, 258.0);
    /// How wide a wrapped row may run inside the panel.
    pub const PROSE_W: f32 = 320.0;
}

// ── party strip ────────────────────────────────────────────────────────────

/// The always-present strip at the bottom: one chip per party, which since
/// wave 1.1 is one chip per person.
pub fn party_strip() -> Rect {
    Rect::from_min_size(Vec2::new(0.0, 440.0), Vec2::new(DESIGN_W, DESIGN_H - 440.0))
}

/// The strip's label.
pub fn party_label() -> Vec2 {
    Vec2::new(16.0, 443.0)
}

const PCHIP_X: f32 = 16.0;
const PCHIP_Y: f32 = 460.0;
const PCHIP_W: f32 = 176.0;
const PCHIP_H: f32 = 36.0;
const PCHIP_GAP: f32 = 12.0;
/// How many party chips a strip row holds before the next one starts.
pub const PARTY_COLUMNS: usize = 5;

/// Party chip `index` — click to select an idle party for dispatch.
///
/// Two rows of five, because the founding band is ten and a chip that fits a
/// name and a status is 176 reference pixels wide: fifty of them in one row
/// would be a strip nobody could read, and a narrower chip would lose the
/// status line the strip exists for.
pub fn party_chip(index: usize) -> Rect {
    let column = index % PARTY_COLUMNS;
    let row = index / PARTY_COLUMNS;
    Rect::from_min_size(
        Vec2::new(
            PCHIP_X + column as f32 * (PCHIP_W + PCHIP_GAP),
            PCHIP_Y + row as f32 * (PCHIP_H + 4.0),
        ),
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
    pub const NAME_X: f32 = 38.0;
    /// The name's top.
    pub const NAME_TOP: f32 = 4.0;
    /// The status line's top.
    pub const STATUS_TOP: f32 = 20.0;
}

// ── the roster drawer: every character in one list ─────────────────────────

/// The roster drawer. The feed's rectangle: three drawers, one shape, never
/// two at once.
pub fn roster_panel() -> Rect {
    feed_panel()
}

/// Its title row.
pub fn roster_title() -> Vec2 {
    Vec2::new(28.0, 50.0)
}

/// The explanation row: what the last tapped trait chip means.
pub fn roster_explain() -> Vec2 {
    Vec2::new(28.0, 72.0)
}

/// How wide that row may run before it is clipped.
pub const ROSTER_EXPLAIN_W: f32 = 900.0;

/// How many roster rows the drawer has room for.
pub const ROSTER_ROWS: usize = 10;

fn roster_row_origin(index: usize) -> Vec2 {
    // Thirty-four apart, so the tenth row ends above the party strip's label:
    // the strip is drawn under every drawer, and a row of text lying across a
    // control it is not the label of is what the floors refuse.
    Vec2::new(20.0, 100.0 + index as f32 * 34.0)
}

/// Roster row `index`'s **open button** — the portrait and the name. Clicking
/// it opens that character's own panel.
///
/// Only the left of the row, because the trait chips beside it are click
/// targets of their own and a control inside a control is exactly what the
/// overlap floor refuses.
pub fn roster_open(index: usize) -> Rect {
    Rect::from_min_size(roster_row_origin(index), Vec2::new(100.0, 32.0))
}

/// Trait chip `slot` on roster row `index` — a target, like every other place
/// a chip appears.
pub fn roster_chip(index: usize, slot: usize) -> Rect {
    Rect::from_min_size(
        roster_row_origin(index) + Vec2::new(124.0 + slot as f32 * 108.0, 0.0),
        Vec2::new(104.0, 32.0),
    )
}

/// Where the wallet, the desperation and the activity sit on a roster row —
/// text, and to the right of every control on it.
pub mod rrow {
    use jidousha::prelude::Vec2;

    /// The portrait's inset from the open button. Drawn at scale 2, which
    /// fills the row's height exactly — the engine samples nearest, and a
    /// fractional scale puts a wobble in pixel art (UI.md §1.4).
    pub const PORTRAIT: Vec2 = Vec2::new(0.0, 0.0);
    /// Where the name starts.
    pub const NAME: Vec2 = Vec2::new(38.0, 10.0);
    /// A chip's icon, from the chip's own top-left.
    pub const CHIP_ICON: Vec2 = Vec2::new(4.0, 8.0);
    /// And its name.
    pub const CHIP_NAME: Vec2 = Vec2::new(26.0, 10.0);
    /// The wallet's number.
    pub const WALLET: Vec2 = Vec2::new(456.0, 10.0);
    /// The desperation.
    pub const NEED: Vec2 = Vec2::new(524.0, 10.0);
    /// What they are doing, and why.
    pub const DOING: Vec2 = Vec2::new(600.0, 10.0);
    /// How wide that may run.
    pub const DOING_W: f32 = 330.0;
}

// ── the feed drawer: the event log, as a view ──────────────────────────────

/// The feed drawer, over the map and under the party strip. The config drawer
/// uses the same rectangle: two drawers, one shape, never both open.
pub fn feed_panel() -> Rect {
    Rect::from_min_size(Vec2::new(0.0, 36.0), Vec2::new(DESIGN_W, 448.0))
}

/// The drawer's own title row.
pub fn feed_title() -> Vec2 {
    Vec2::new(28.0, 50.0)
}

/// The reason row, under the title and across the whole drawer — where an
/// auto-pause says why, beside the entry that caused it.
pub fn feed_reason() -> Vec2 {
    Vec2::new(28.0, 76.0)
}

/// How wide the reason may run before it is clipped.
pub const FEED_REASON_W: f32 = 908.0;

/// The show-ignored toggle, for auditing what the config is hiding.
pub fn feed_ignored_toggle() -> Rect {
    Rect::from_min_size(Vec2::new(760.0, 42.0), Vec2::new(180.0, 32.0))
}

/// How many feed rows the drawer has room for. The same number as the shipped
/// `feed_cap`, so the view and the drawer bound the feed at one place.
pub const FEED_ROWS: usize = 10;

/// Feed row `index` — click it to look at where it happened.
pub fn feed_row(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(20.0, 94.0 + index as f32 * 34.0),
        Vec2::new(920.0, 32.0),
    )
}

/// The columns inside a feed row, as offsets from its top-left — the entry's
/// anatomy: world timestamp, class chip, place tag, and the text under them.
pub mod entry {
    use jidousha::prelude::Vec2;

    /// The world timestamp.
    pub const STAMP: Vec2 = Vec2::new(8.0, 4.0);
    /// The class chip's icon.
    pub const CHIP_ICON: Vec2 = Vec2::new(90.0, 2.0);
    /// The class chip's name.
    pub const CHIP_NAME: Vec2 = Vec2::new(112.0, 4.0);
    /// The place tag.
    pub const PLACE: Vec2 = Vec2::new(252.0, 4.0);
    /// The event's own sentence, on the second line.
    pub const TEXT: Vec2 = Vec2::new(8.0, 18.0);
    /// How wide that sentence may run.
    pub const TEXT_W: f32 = 900.0;
}

/// How many notices the drawer's footer shows.
pub const NOTICE_ROWS: usize = 2;

/// The notices heading, under the feed.
pub fn notices_title() -> Vec2 {
    Vec2::new(28.0, 438.0)
}

/// Notice row `index`.
pub fn notice_row(index: usize) -> Vec2 {
    Vec2::new(28.0, 454.0 + index as f32 * 14.0)
}

// ── the postings ledger drawer (wave 1.2) ─────────────────────────────────

/// The ledger drawer: the same rectangle every other drawer has.
pub fn ledger_panel() -> Rect {
    feed_panel()
}

/// Its title row.
pub fn ledger_title() -> Vec2 {
    Vec2::new(28.0, 50.0)
}

/// The line under the title: what a posting is, and what withdrawing does.
pub fn ledger_note() -> Vec2 {
    Vec2::new(28.0, 72.0)
}

/// How wide that may run.
pub const LEDGER_NOTE_W: f32 = 700.0;

/// How many postings the ledger shows at once, newest first.
pub const LEDGER_ROWS: usize = 8;

fn ledger_row_origin(index: usize) -> Vec2 {
    Vec2::new(20.0, 94.0 + index as f32 * 38.0)
}

/// Ledger row `index` — the posting itself, two lines of it.
pub fn ledger_row(index: usize) -> Rect {
    Rect::from_min_size(ledger_row_origin(index), Vec2::new(560.0, 32.0))
}

/// How wide a ledger row's text may run.
pub const LEDGER_ROW_W: f32 = 550.0;

/// The **withdraw** button on ledger row `index` — the one way to take a
/// posting down.
pub fn ledger_withdraw(index: usize) -> Rect {
    Rect::from_min_size(
        ledger_row_origin(index) + Vec2::new(572.0, 0.0),
        Vec2::new(96.0, 32.0),
    )
}

/// The rows inside a ledger row, from its top-left.
pub mod ledger {
    use jidousha::prelude::Vec2;

    /// Who, what, how much, and until when.
    pub const HEAD: Vec2 = Vec2::new(8.0, 3.0);
    /// Who has heard it and who has answered it.
    pub const ANSWER: Vec2 = Vec2::new(8.0, 17.0);
}

/// The standing-rates band's title, at the top of its column.
pub fn rates_title() -> Vec2 {
    Vec2::new(RATES_X, 72.0)
}

/// Where the standing-rates band starts — right of the ledger's own rows and
/// their withdrawals.
const RATES_X: f32 = 704.0;

/// How wide the rates band's own prose may run.
pub const RATES_NOTE_W: f32 = 240.0;

/// The note under the rates, saying what they price.
pub fn rates_note() -> Vec2 {
    Vec2::new(RATES_X, 260.0)
}

fn rates_row_origin(index: usize) -> Vec2 {
    Vec2::new(RATES_X, 94.0 + index as f32 * 38.0)
}

/// Where standing-rate row `index`'s task type is named.
pub fn rates_name(index: usize) -> Vec2 {
    rates_row_origin(index) + Vec2::new(0.0, 10.0)
}

/// The **rate down** button on standing-rate row `index`.
pub fn rates_down(index: usize) -> Rect {
    Rect::from_min_size(
        rates_row_origin(index) + Vec2::new(76.0, 0.0),
        Vec2::splat(32.0),
    )
}

/// Where the rate itself is drawn, between its two steppers.
pub fn rates_value(index: usize) -> Rect {
    Rect::from_min_size(
        rates_row_origin(index) + Vec2::new(108.0, 0.0),
        Vec2::new(40.0, 32.0),
    )
}

/// The **rate up** button on standing-rate row `index`.
pub fn rates_up(index: usize) -> Rect {
    Rect::from_min_size(
        rates_row_origin(index) + Vec2::new(148.0, 0.0),
        Vec2::splat(32.0),
    )
}

/// The **standing-posting** button on standing-rate row `index` — post this
/// kind of work, to anyone, until withdrawn, at the rate beside it.
///
/// The closest thing to policy the player has by hand (the GDD's postings
/// section): a standing open posting for a task type at the standing rate,
/// made from the panel that sets the rate.
pub fn rates_post(index: usize) -> Rect {
    Rect::from_min_size(
        rates_row_origin(index) + Vec2::new(188.0, 0.0),
        Vec2::new(60.0, 32.0),
    )
}

/// The ledger's footer, under both bands.
pub fn ledger_footer() -> Vec2 {
    Vec2::new(28.0, 406.0)
}

// ── the auto-pause config drawer ───────────────────────────────────────────

/// Its title row.
pub fn modes_title() -> Vec2 {
    Vec2::new(28.0, 46.0)
}

/// The note under the title.
pub fn modes_note() -> Vec2 {
    Vec2::new(28.0, 64.0)
}

/// How wide the drawer's prose may run.
pub fn modes_prose_width() -> f32 {
    DESIGN_W - 56.0
}

/// How many config rows a column holds before the next one starts.
///
/// **Seven since wave 1.2**: the asks module's six classes take the table
/// from seven rows to thirteen, and thirteen rows of forty pixels is a
/// drawer twice the height of the screen. Two columns of seven is what fits,
/// and the radios narrow to eighty-four to make room for the second column —
/// still two and a half times the target floor.
pub const MODES_ROWS: usize = 7;
const MODES_COL_X: f32 = 20.0;
const MODES_COL_PITCH: f32 = 468.0;
const MODES_RADIO_W: f32 = 84.0;
const MODES_RADIO_PITCH: f32 = 88.0;
/// Where a row's radios start, from the row's own left — right of the widest
/// class id there is (`posting-withdrawn`, seventeen glyphs), because a radio
/// that started under the end of a name would put two rows' glyphs in one
/// box and the frame judge counts glyphs by box.
const MODES_RADIO_X: f32 = 200.0;

fn modes_row_origin(index: usize) -> Vec2 {
    let column = index / MODES_ROWS;
    let row = index % MODES_ROWS;
    Vec2::new(
        MODES_COL_X + column as f32 * MODES_COL_PITCH,
        108.0 + row as f32 * 40.0,
    )
}

/// The class chip's icon on config row `index`.
pub fn modes_icon(index: usize) -> Vec2 {
    modes_row_origin(index) + Vec2::new(0.0, 8.0)
}

/// The class's name on config row `index`.
pub fn modes_name(index: usize) -> Vec2 {
    modes_row_origin(index) + Vec2::new(24.0, 10.0)
}

/// The radio for mode `mode` on config row `index`.
pub fn modes_radio(index: usize, mode: usize) -> Rect {
    let origin = modes_row_origin(index);
    Rect::from_min_size(
        Vec2::new(
            origin.x + MODES_RADIO_X + mode as f32 * MODES_RADIO_PITCH,
            origin.y,
        ),
        Vec2::new(MODES_RADIO_W, 32.0),
    )
}

/// The drawer's footer: what a change to this panel is.
pub fn modes_footer() -> Vec2 {
    Vec2::new(28.0, 398.0)
}

// ── the tuning drawer (giri's geometry, at the module's constants) ────────

/// The tuning drawer: from under the status bar to the bottom of the screen.
pub fn tuner_panel() -> Rect {
    Rect::from_min_size(Vec2::new(0.0, 36.0), Vec2::new(DESIGN_W, DESIGN_H - 36.0))
}

/// How many stepper rows a column holds before the next one starts.
///
/// Twelve, which is what the drawer's height allows at the target floor, and
/// three columns of them is what wave 1.1's thirty-four constants need. The
/// stamp keeps the last two hundred pixels of the screen and the prose band
/// moved under it: the stamp is the one thing in the drawer that has to stay
/// legible while every other row is being moved.
pub const TUNER_ROWS: usize = 12;
const TUNER_COL_X: f32 = 28.0;
const TUNER_COL_PITCH: f32 = 240.0;
const TUNER_ROW_Y: f32 = 110.0;
const TUNER_ROW_PITCH: f32 = 34.0;
/// The steppers' - and + size: the smallest target in the game, exactly the
/// floor.
const TUNER_STEP: f32 = 32.0;
/// The width the longest constant name needs (`grudge_ceiling` and friends).
const TUNER_NAME_W: f32 = 136.0;
/// The gap the value sits in, between the two buttons.
const TUNER_VALUE_W: f32 = 32.0;

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

/// The prose band, under the stamp: the hint row first, then the note.
pub fn tuner_hint() -> Vec2 {
    Vec2::new(TUNER_STAMP_X, 350.0)
}

/// How wide the hint, the note and the stamp may run before they wrap.
pub fn tuner_prose_width() -> f32 {
    DESIGN_W - TUNER_STAMP_X - 16.0
}

/// Where the stamp column starts — right of the third stepper column, which
/// ends at 740.
const TUNER_STAMP_X: f32 = 756.0;

/// The stamp: the constants actually in effect, always visible while the
/// drawer is open.
pub fn tuner_stamp() -> Vec2 {
    Vec2::new(TUNER_STAMP_X, 110.0)
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

/// How big a character standing at their home tile is drawn, in world units —
/// the same weight as a site marker, because a person is at least as much of
/// a thing on the map as a hole in the ground is.
///
/// A click on a figure opens that character's panel (`flow.rs`), and 32 is the
/// target floor exactly — but it is a *world* rectangle rather than a chrome
/// one, so it is not in `floors::targets` and the overlap floor does not bind
/// it against the chrome.
pub const HOME: f32 = 32.0;

/// A character's rectangle, centred over their home tile.
pub fn home_rect(tile: crate::grid::Tile) -> Rect {
    Rect::from_center_size(tile.center(), Vec2::splat(HOME))
}

/// Where a character's name starts, under them.
pub fn home_label(tile: crate::grid::Tile, width: f32) -> Vec2 {
    tile.center() + Vec2::new(-width * 0.5, HOME * 0.5 + 2.0)
}
