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
/// Most of the width of the board, because the wage and the who are the
/// **board's** controls and not the row's: a stepper inside a row would be a
/// control inside a control, which the overlap floor refuses, and a row cut
/// short to make room for one would leave no room for the verdict's reason —
/// which is the sentence wave 1.2 exists to put on screen.
///
/// **Fifty-two narrower since the candidate picker landed**, which is
/// [`board_who`]'s target and the gap beside it — and the verdict's reason
/// came out of it **twenty-eight pixels wider** rather than fifty-two
/// narrower, because the fit moved up to the row's first line where the
/// duration had left room (`job::FIT`).
/// A row's two lines are what the work is and how well it suits somebody over
/// what kind it is and what they say about it, and the fit belongs to the
/// first of those readings anyway.
pub fn board_row(index: usize) -> Rect {
    Rect::from_min_size(board_row_origin(index), Vec2::new(468.0, 32.0))
}

/// Job row `index`'s **`who?` button** — the candidate list for *that job*
/// (the candidate-picker session).
///
/// A target of its own between the row and its `?`, for the reason the `?` is
/// one: the row *is* the posting, and a control inside a control is what the
/// overlap floor refuses. It is what makes the board self-sufficient — a
/// posting can be aimed at somebody without reaching the map sprite the board
/// is covering (`FINDINGS.md` G-026).
pub fn board_who(index: usize) -> Rect {
    Rect::from_min_size(
        board_row_origin(index) + Vec2::new(476.0, 0.0),
        Vec2::new(44.0, 36.0),
    )
}

/// Job row `index`'s **why button** — the arithmetic behind the verdict the
/// row is showing, one tap deeper (the legibility session).
///
/// A target of its own at the end of the row rather than the verdict cell
/// made clickable, because the row *is* the posting and a control inside a
/// control is what the overlap floor refuses — the same separation the roster
/// row makes between its name box and its chips. It and [`board_who`] stand in
/// the ninety-six pixels between the row's end and the board's.
pub fn board_why(index: usize) -> Rect {
    Rect::from_min_size(
        board_row_origin(index) + Vec2::new(528.0, 0.0),
        Vec2::splat(36.0),
    )
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
/// **what the work is, what it pays, and how the selected character fits it**
/// over **what kind of work it is and who has it or what they say about it**.
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
    /// **The selected character's fit for this work**, at the end of the first
    /// line — beside the pot and the duration, because how well the work pays
    /// and how well it suits somebody are one reading of one offer.
    ///
    /// It stood on the second line until the candidate picker took the `who?`
    /// target out of the row's width; the duration had left room here, so the
    /// verdict's reason came out of the move twenty-eight pixels wider rather
    /// than fifty-two narrower.
    pub const FIT: Vec2 = Vec2::new(342.0, 3.0);
    /// How wide that may run.
    pub const FIT_W: f32 = 60.0;
    /// The task-type chip's icon.
    pub const TASK_ICON: Vec2 = Vec2::new(8.0, 14.0);
    /// And its word.
    pub const TASK_NAME: Vec2 = Vec2::new(28.0, 17.0);
    /// How wide that may run — the widest task id is five characters, and the
    /// six pixels this gave up went to the verdict's reason beside it.
    pub const TASK_W: f32 = 50.0;
    /// **What the row has to say about itself**: what has become of it, or —
    /// where it is open and somebody is selected — what the scorer says they
    /// would do about a posting here, and why (wave 1.2).
    ///
    /// One cell, because the two readings never both apply: a row that is
    /// somebody's is not a row anybody can be asked for, and a verdict is
    /// strictly more than the word `open` it replaces. The widest cell on the
    /// row, because a verdict without its reason is a number about a person.
    pub const SAYS: Vec2 = Vec2::new(86.0, 17.0);
    /// How wide that may run — the same 380 the candidate row's own verdict
    /// cell has, so one sentence about one offer is cut in the same place on
    /// both surfaces rather than in two
    /// (`verify::the_picker_names_a_person` measures the widest against both).
    pub const SAYS_W: f32 = 380.0;
}

// ── the candidate picker: who a job could be posted to (UI.md §3c) ─────────

/// How many candidate rows the picker has room for.
///
/// Ten, which is the whole cast (`people::roster`) — **everyone appears**,
/// including the people who are out, because posting to somebody who is out
/// is legal and travels. `layout_floors` asserts the registry is not larger
/// than this, because a candidate with no row is a person the board cannot
/// name, which is the whole defect this surface closes.
pub const PICKER_ROWS: usize = 10;

/// **The candidate list a job row's `who?` opens.**
///
/// It stands where the board stands and is taller than it: ten rows of two
/// lines each do not fit the board's rectangle, and the space below it is the
/// breakdown band's, which is why the two are never up together (UI.md §3c —
/// the picker and the band are the same tap-deeper move on the same row, and
/// there is one of it).
///
/// **It replaces the board rather than sitting beside it**: the left of the
/// screen is one column, the character panel has the other, and a surface
/// drawn under another is a row nobody can read lying across a control
/// somebody can click. So while the picker is up the board draws nothing at
/// all and the picker's own header carries what the player still needs — the
/// job's name and the wage its verdicts were read at.
pub fn picker_panel() -> Rect {
    Rect::from_min_size(Vec2::new(16.0, 112.0), Vec2::new(576.0, 424.0))
}

/// Its title row: which job the list is for.
pub fn picker_title() -> Vec2 {
    Vec2::new(28.0, 122.0)
}

/// **And the wage the answers below were read at, on a line of its own.**
///
/// Two header lines rather than one, because one clipped: the job's name and
/// the wage together run past the fit chip on the longest job this game
/// authors, and what a clip takes off is the tail — which was the wage, and
/// the wage appears nowhere else while the picker is covering the board's
/// footer. `layout_floors` asserts both lines fit at the longest name the
/// scenario holds.
pub fn picker_wage() -> Vec2 {
    Vec2::new(28.0, 140.0)
}

/// How wide either header line may run before it is clipped — up to the fit
/// chip, which is the board's own and stands in the same place on both
/// surfaces.
pub const PICKER_HEAD_W: f32 = 360.0;

/// Candidate row `index` — **the choosing target**, which sets the one
/// selection and nothing else (UI.md §3b).
///
/// The whole width of the picker: the portrait inside it is a picture and not
/// a target of its own, because the row already answers the only question
/// this surface asks and a second target inside it would be a control inside
/// a control.
pub fn picker_row(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(24.0, 160.0 + index as f32 * 34.0),
        Vec2::new(544.0, 32.0),
    )
}

/// The picker's footer hint, under the rows — a wrapped block, because the
/// fit chip's explanation is a sentence and it is the same sentence the board
/// prints (`asks::fit_means`).
pub fn picker_hint() -> Vec2 {
    Vec2::new(28.0, 502.0)
}

/// How wide it may run before it wraps — the picker's own width, since
/// nothing else stands in that band.
pub const PICKER_HINT_W: f32 = 552.0;

/// The columns inside a candidate row, as offsets from its top-left — two
/// lines: **who they are, how they fit this work, and where they are** over
/// **what they would say about the offer, and why**.
pub mod cand {
    use jidousha::prelude::Vec2;

    /// The portrait's inset. Drawn at `sheet::PORTRAIT_SCALE`, which fills
    /// the row's height exactly.
    pub const PORTRAIT: Vec2 = Vec2::new(0.0, 0.0);
    /// Where the name starts — clear of the portrait.
    pub const NAME: Vec2 = Vec2::new(38.0, 3.0);
    /// How wide that may run.
    pub const NAME_W: f32 = 96.0;
    /// Their fit for this job's kind of work — the column the list is sorted
    /// on.
    pub const FIT: Vec2 = Vec2::new(140.0, 3.0);
    /// How wide that may run.
    pub const FIT_W: f32 = 56.0;
    /// **Where they are** — at home, or out with where and when the work
    /// they are on is done.
    pub const WHERE: Vec2 = Vec2::new(202.0, 3.0);
    /// How wide that may run.
    pub const WHERE_W: f32 = 340.0;
    /// **The journey from wherever they stand** — its own cell rather than
    /// the tail of the whereabouts, because a clip takes the tail and the
    /// journey is what a posting to somebody who is out costs.
    ///
    /// At the **head** of the second line rather than the end of it: a
    /// fixed-width column before a ragged one reads as two columns, where a
    /// ragged one before a fixed one runs its longest row into the next cell
    /// — which is what the first photograph of this surface showed.
    pub const TRAVEL: Vec2 = Vec2::new(38.0, 17.0);
    /// How wide that may run.
    pub const TRAVEL_W: f32 = 116.0;
    /// **What the scorer says they would do about this offer**, and why.
    ///
    /// The row's second line up to the travel cell, and **wide enough for the
    /// widest verdict line the scorer produces** —
    /// `verify::the_picker_names_a_person` measures that over a played world
    /// and asserts it here, because the reason on a refusal is the rival's own
    /// sentence and which rival wins is what a played world has.
    pub const SAYS: Vec2 = Vec2::new(162.0, 17.0);
    /// How wide that may run.
    pub const SAYS_W: f32 = 380.0;
}

// ── the work list: the person-side mirror of the candidate picker ─────────

/// How many rows of open work the list has room for.
///
/// **Ten, and the cap is declared rather than discovered.** The settlement
/// authors four sites of six jobs, so a character can have twenty-four jobs
/// open to them and the column holds ten of two lines each — the same ten
/// rows, at the same pitch, in the same rectangle as the candidate picker
/// this surface mirrors. The ten it shows are the ten the sort puts first
/// (fit descending, ties in site then authored order), and the footer says
/// how many it is not showing, because a list that silently stopped at ten
/// would be a list the player could not trust.
pub const WORK_ROWS: usize = 10;

/// **The list of the work open to the selected character** (UI.md §3f).
///
/// It stands where the board and the candidate picker stand, for the reason
/// the picker does: the left of the screen is one column, the character panel
/// has the other, and a surface drawn under another is a row nobody can read
/// lying across a control somebody can click. Tapping a row opens that site's
/// board in this same rectangle.
pub fn worklist_panel() -> Rect {
    picker_panel()
}

/// Its title row: whose work this is.
pub fn worklist_title() -> Vec2 {
    picker_title()
}

/// The line under it: how much work there is, at what wage, in what order.
pub fn worklist_note() -> Vec2 {
    picker_wage()
}

/// How wide either header line may run before it is clipped — up to the fit
/// chip, which is the board's own and stands in the same place on all three
/// surfaces.
pub const WORKLIST_HEAD_W: f32 = PICKER_HEAD_W;

/// Work row `index` — **the navigating target**: it opens that site's board
/// with the job in view, and posts nothing (UI.md §3f).
pub fn worklist_row(index: usize) -> Rect {
    picker_row(index)
}

/// The footer hint, under the rows.
pub fn worklist_hint() -> Vec2 {
    picker_hint()
}

/// How wide it may run before it wraps.
pub const WORKLIST_HINT_W: f32 = PICKER_HINT_W;

/// The columns inside a work row, as offsets from its top-left — two lines:
/// **what the work is, what it pays, how long it takes and how well it suits
/// them** over **what the journey costs and what they would say about it**.
pub mod work {
    use jidousha::prelude::Vec2;

    /// The task-type chip's icon — the same picture on a job row and on a
    /// person, because it is the same word.
    pub const TASK_ICON: Vec2 = Vec2::new(0.0, 0.0);
    /// The job's name.
    ///
    /// **Wide enough for the longest job the scenario authors**, not for the
    /// longest one it happened to have: `floors::layout_floors` measures every
    /// authored name against this and every other cell on the row, because a
    /// clip on this surface takes the tail of the one word that says which
    /// job a tap is about.
    pub const NAME: Vec2 = Vec2::new(26.0, 3.0);
    /// How wide that may run.
    pub const NAME_W: f32 = 164.0;
    /// What it pays.
    pub const POT: Vec2 = Vec2::new(196.0, 3.0);
    /// How wide that may run.
    pub const POT_W: f32 = 48.0;
    /// How long it takes.
    pub const DURATION: Vec2 = Vec2::new(250.0, 3.0);
    /// How wide that may run.
    pub const DURATION_W: f32 = 70.0;
    /// Their fit for this kind of work — the column the list is sorted on.
    pub const FIT: Vec2 = Vec2::new(326.0, 3.0);
    /// How wide that may run.
    pub const FIT_W: f32 = 56.0;
    /// **Which site it stands at** — the thing a list across every board has
    /// to say that a single board never did.
    pub const WHERE: Vec2 = Vec2::new(388.0, 3.0);
    /// How wide that may run.
    pub const WHERE_W: f32 = 150.0;
    /// **The walk from wherever they stand**, at the head of the second line
    /// for the reason a candidate row puts it there: a fixed-width column
    /// before a ragged one reads as two columns.
    pub const TRAVEL: Vec2 = Vec2::new(26.0, 17.0);
    /// How wide that may run.
    pub const TRAVEL_W: f32 = 128.0;
    /// **What the scorer says they would do about this job at its standing
    /// rate**, and why — the same sentence, from the same read, that the
    /// board's own row and a candidate row carry.
    pub const SAYS: Vec2 = Vec2::new(162.0, 17.0);
    /// How wide that may run.
    pub const SAYS_W: f32 = 380.0;
}

// ── the character panel ────────────────────────────────────────────────────

/// The panel a selected character opens.
///
/// **Twenty-four pixels taller since the party strip retired**, and the rows
/// inside it closed up by the same: a trait chip's explanation now says what
/// its row does *and* what nothing yet does with it (`traits::explain`'s
/// dormancy clause), and that is a sentence rather than a phrase.
/// `floors::layout_floors` asserts the longest explanation the vocabulary can
/// produce still ends inside this rectangle, so the panel is sized by the data
/// rather than by a guess that was true when it was typed.
///
/// It stops at the band below it rather than filling the space the strip left,
/// because that band is the breakdown's and the breakdown wants the width more
/// than this panel wants the height (§3e).
pub fn person_panel() -> Rect {
    Rect::from_min_size(Vec2::new(600.0, 116.0), Vec2::new(344.0, 324.0))
}

/// Its close button.
pub fn person_close() -> Rect {
    Rect::from_min_size(Vec2::new(900.0, 122.0), Vec2::new(36.0, 32.0))
}

/// **The door onto the work open to this person** (UI.md §3f) — the chip in
/// the panel's own header, beside the name it is about and left of the close.
///
/// In the header because the panel has nowhere else: at its widest state — a
/// trait's longest explanation open under the flowed rows — the sheet ends
/// twelve reference pixels above the panel's foot, and a target is
/// thirty-two. The header had the room, and the count on it is the one thing
/// about the list that belongs on the sheet whether or not the list is up.
pub fn sheet_work() -> Rect {
    Rect::from_min_size(
        person_panel().min + Vec2::new(216.0, 6.0),
        Vec2::new(80.0, 32.0),
    )
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
    /// **Where the flowed rows start**: the source line, and everything under
    /// it.
    ///
    /// **Measured from here down, not placed.** The source, the activity
    /// line, the home row and a tapped chip's explanation all wrap, and the
    /// four used to sit at four typed offsets that were true at the lengths
    /// of the day: at the shipped cast the activity line wraps to two rows
    /// and its second row was drawn through the home row (`FINDINGS.md`
    /// G-029, the same shape as the tuning drawer's). Each block now starts
    /// where the one above it ended, and `floors::layout_floors` asserts the
    /// whole flow ends inside the panel at the longest each of them can be.
    pub const SOURCE: Vec2 = Vec2::new(12.0, 156.0);
    /// The gap between two flowed blocks.
    pub const FLOW_GAP: f32 = 4.0;
    /// **How many rows the three blocks above the explanation take at their
    /// longest**: the source line, the activity line and the home row.
    ///
    /// Five — two, two and one. It is a budget rather than a measurement
    /// because the activity line is written out of a running world and there
    /// is no static longest; what makes it an assertion instead of a hope is
    /// `floors::content_floors`, which rebuilds the character panel over every
    /// state it judges and requires every row of it to land inside the panel.
    pub const LEAD_ROWS: usize = 5;
    /// How wide a wrapped row may run inside the panel.
    pub const PROSE_W: f32 = 320.0;
    /// How wide the name may run before the work chip beside it.
    pub const NAME_W: f32 = 160.0;
}

// ── the breakdown band: the scorer's arithmetic, one tap deeper ───────

/// **The band the party strip left behind** (the legibility session): where a
/// verdict's arithmetic is shown when somebody asks for it, and nothing at
/// all the rest of the time.
///
/// **The width of the screen**, under everything the base screen can have up:
/// a term line is a value, a word and what produced it, and two of those side
/// by side is what makes ten of them readable at a glance. The board ends at
/// 440 and the character panel stops there with it, so this band lies under
/// both. It is not a drawer: it is up *with* the surface that opened it,
/// exactly as the character panel is up with the selection.
pub fn breakdown_panel() -> Rect {
    Rect::from_min_size(Vec2::new(16.0, 444.0), Vec2::new(928.0, 92.0))
}

/// Its title row — what was weighed, and what it came to.
pub fn breakdown_title() -> Vec2 {
    Vec2::new(28.0, 450.0)
}

/// How wide the title may run before it is clipped.
pub const BREAKDOWN_TITLE_W: f32 = 908.0;

/// How many cells the band holds: five rows of two columns.
///
/// Ten, which is one more than the widest sum this game can produce — seven
/// terms of an answered posting, its total, and the candidate that beat it.
/// `floors::layout_floors` asserts that headroom against the scorer's own
/// term count rather than against this comment.
pub const BREAKDOWN_CELLS: usize = 10;

/// How many cells a column holds — the band reads down, then across.
pub const BREAKDOWN_ROWS: usize = 5;

/// Where cell `index` starts: down the first column, then down the second.
pub fn breakdown_cell(index: usize) -> Vec2 {
    let column = index / BREAKDOWN_ROWS;
    let row = index % BREAKDOWN_ROWS;
    Vec2::new(
        28.0 + column as f32 * 460.0,
        466.0 + row as f32 * (crate::theme::SMALL + 2.0),
    )
}

/// How wide one cell's line may run before it is clipped.
pub const BREAKDOWN_CELL_W: f32 = 448.0;

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

/// How wide a row of it may run before it wraps.
pub const ROSTER_EXPLAIN_W: f32 = 900.0;

/// How many rows the explanation band holds.
///
/// Two, which is what stands between the band's own top and the first roster
/// row — and what the dormancy clause needed: a one-row clip ate the half of
/// the sentence that says what nothing yet does with the trait, which is the
/// half the legibility session exists to print.
pub const ROSTER_EXPLAIN_ROWS: usize = 2;

/// How many roster rows the drawer has room for.
pub const ROSTER_ROWS: usize = 10;

fn roster_row_origin(index: usize) -> Vec2 {
    // Thirty-four apart, so the tenth row ends inside the drawer that holds
    // it. The spacing was set against the party strip's label, which the
    // legibility session retired; the rows kept their pitch because the
    // drawer's own rectangle is what binds them and that has not moved.
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

/// The feed drawer, over the map. The config drawer uses the same rectangle:
/// two drawers, one shape, never both open.
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
    Rect::from_min_size(feed_row_origin(index), Vec2::new(876.0, 32.0))
}

fn feed_row_origin(index: usize) -> Vec2 {
    Vec2::new(20.0, 94.0 + index as f32 * 34.0)
}

/// Feed row `index`'s **why button** — the arithmetic behind a decision that
/// has already been made (the legibility session).
///
/// The same gesture the job board's rows carry, in the same place on the row,
/// so "tap the row to act on it, tap the question mark to see the sum" is one
/// rule rather than two. It answers on the entries that carry a reckoning and
/// bounces on the ones that do not, because an entry that recorded no
/// decision has no arithmetic and saying so is cheaper than a dead target.
pub fn feed_why(index: usize) -> Rect {
    Rect::from_min_size(
        feed_row_origin(index) + Vec2::new(884.0, 0.0),
        Vec2::splat(36.0),
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
    pub const TEXT_W: f32 = 856.0;
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
/// is measured down from it (`tuning::prose_top`): the stamp is the one thing
/// in the drawer that has to stay legible while every other row is being
/// moved, so it keeps the top of that column and the prose follows it.
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

/// The gap between the stamp and the prose band that follows it down the
/// right column.
///
/// **There is no `tuner_hint()` any more, and that is the fix.** The band's
/// top was a constant 350 while the stamp flowed down from 124 at one row per
/// two constants, so the thirty-fifth constant put the stamp through the hint
/// and nothing said so (`FINDINGS.md` G-028). The band's top is now measured
/// from the stamp above it — `tuning::prose_top`, one function read by the
/// drawer and by the floor — and this is the only number left in it.
pub const TUNER_PROSE_GAP: f32 = 10.0;

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

/// How big a character is drawn on the map, in world units — **one size,
/// wherever they are standing**, the same weight as a site marker, because a
/// person is at least as much of a thing on the map as a hole in the ground
/// is.
///
/// It was two constants until the double-drawn cast (`FINDINGS.md` G-023):
/// one for a figure at a doorstep and one for a token on the road, for what
/// had become one picture of one person. A second name for a size is a second
/// size waiting to happen.
///
/// A click on a figure opens that character's panel (`flow.rs`), and 32 is the
/// target floor exactly — but it is a *world* rectangle rather than a chrome
/// one, so it is not in `floors::targets` and the overlap floor does not bind
/// it against the chrome.
pub const HOME: f32 = 32.0;

/// A character's rectangle, centred over a tile.
///
/// The base a doorstep figure is drawn at; `screens::where_drawn` is the one
/// answer to where a person actually stands, and this is what it centres.
pub fn home_rect(tile: crate::grid::Tile) -> Rect {
    Rect::from_center_size(tile.center(), Vec2::splat(HOME))
}

/// Where a character's name starts, under the figure it names.
///
/// Off the rectangle the figure was drawn in rather than off the tile it
/// belongs to, so a name cannot be left behind by a figure that moved.
pub fn figure_label(figure: Rect, width: f32) -> Vec2 {
    Vec2::new(figure.center().x - width * 0.5, figure.max.y + 2.0)
}
