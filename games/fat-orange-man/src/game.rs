//! The pure layer: the growth curve, the layout rectangles, the tap test, and
//! the leaderboard projection — every question a `--verify` check wants to put
//! to the game, written as a free function it can call without a `World`.
//!
//! `docs/api/jidousha-api.md` Concepts: *write the two decisions a check will
//! want as free functions, now, while they are free.* For this game those are
//! **how big the square is** (`square_size`) and **what a tap does** — which
//! resolves to *where the tap targets are* (`feed_button`, `is_feed_tap`) and
//! *what the leaderboard shows* (`project_board`). The draw systems and the
//! checks both read these, so a picture and an assertion cannot disagree.

use jidousha::prelude::*;

// --- the layout, in constants derived from the window --------------------

/// The window the game opens at: portrait 9:16, a phone held upright. Every
/// world extent below is stated against the aspect this implies, so a change
/// here moves the layout with it rather than leaving a hand-typed ratio stale
/// (Concepts, "A layout in constants").
pub const WINDOW: PhysicalSize = PhysicalSize::new(540, 960);

/// Half the world height the camera spans — the one number the layout picks.
pub const HALF_H: f32 = 12.0;

/// Half the world width: the height times the shape of the window. `new` and
/// `aspect` are both `const fn`, so nothing types the ratio.
pub const HALF_W: f32 = HALF_H * WINDOW.aspect();

/// The camera the window and every headless run share, so what is verified is
/// what a person sees. `viewport` is the driver's to stamp; a headless run
/// overrides it through `FrameRecorder`, so a check rebuilds this with its own.
pub fn camera() -> Camera {
    Camera {
        center: Vec2::ZERO,
        height: HALF_H * 2.0,
        clear_color: BACKGROUND,
        // Set explicitly rather than left to the default, so `handle_tap`'s
        // `screen_to_world` and a `--verify` script's `world_to_screen` agree on
        // native's first frame and on every headless tick — nothing stamps the
        // viewport under `headless` (jidousha-testing.md, the viewport trap).
        // `run` re-stamps this from the real window on native from frame two on.
        viewport: WINDOW,
    }
}

/// The visible rectangle at the shipped aspect — `camera().visible_bounds()`
/// stated in constants so a `const fn` layout can rest on it.
pub const VIEW_MIN: Vec2 = Vec2::new(-HALF_W, -HALF_H);
/// The bottom-right of the visible rectangle.
pub const VIEW_MAX: Vec2 = Vec2::new(HALF_W, HALF_H);

/// Draw bands, named once so no `z: 3.0` ever appears at a call site.
pub mod layers {
    /// The orange man.
    pub const MAN: i16 = 0;
    /// The leaderboard panel, over the man so his growth never hides it.
    pub const PANEL: i16 = 1;
    /// The feed button.
    pub const BUTTON: i16 = 2;
    /// All text, over everything.
    pub const TEXT: i16 = 3;
}

// --- colours -----------------------------------------------------------

/// What the screen is cleared to — dark, so the orange reads against it.
pub const BACKGROUND: Color = Color::rgb(0.06, 0.07, 0.10);
/// The orange man himself.
pub const ORANGE: Color = Color::rgb(1.0, 0.50, 0.12);
/// The feed button.
pub const BUTTON: Color = Color::rgb(1.0, 0.58, 0.16);
/// Text drawn on the button — dark, for contrast against the orange.
pub const BUTTON_TEXT: Color = Color::rgb(0.10, 0.06, 0.02);
/// The leaderboard panel's fill.
pub const PANEL: Color = Color::rgba(1.0, 1.0, 1.0, 0.05);
/// Ordinary readout text.
pub const TEXT: Color = Color::rgb(0.92, 0.94, 0.98);
/// The player's own row in the leaderboard, picked out from the rest.
pub const TEXT_YOU: Color = Color::rgb(1.0, 0.78, 0.35);

// --- the orange man's girth ------------------------------------------

/// The square's side at zero communal feeds, in world units.
pub const SQUARE_MIN: f32 = 1.5;

/// The largest side the square is *drawn* at, in world units.
///
/// The counter is uncapped — endless growth is the joke (GDD §3.7) — but the
/// picture cannot be, so the render clamps here. GDD §4.3 names this a
/// "visual/UX necessity, not a gameplay mechanic": the number keeps climbing,
/// the drawn square stops. Sized to leave a margin inside `HALF_W`, and to sit
/// clear of the readout above and the panel below, so the off-screen check
/// stays meaningful.
pub const SQUARE_MAX: f32 = 10.6;

/// How fast the square approaches `SQUARE_MAX`. `side = SQUARE_MIN + GROWTH *
/// sqrt(feeds)`, a decelerating curve (GDD §4.3): about 2,000 communal feeds to
/// reach half the screen, about 10,000 to fill it — slow enough that a session
/// sees the square move and fast enough that it moves within one.
pub const GROWTH: f32 = 0.091;

/// Where the square is centred.
pub const SQUARE_CENTER: Vec2 = Vec2::new(0.0, -4.2);

/// The square's drawn side, in world units, for a given **global** feed count.
///
/// Monotonically non-decreasing in `feeds` and clamped to `SQUARE_MAX`. This is
/// the growth curve; a check calls it directly rather than watching the square.
pub fn square_size(feeds: u64) -> f32 {
    // `sqrt` on a whole count, not trigonometry — no determinism ban applies,
    // and it is bit-identical for the range a session reaches.
    let side = SQUARE_MIN + GROWTH * (feeds as f32).sqrt();
    side.min(SQUARE_MAX)
}

/// The square's world-space bounds at a given global feed count.
pub fn square_bounds(feeds: u64) -> Rect {
    Rect::from_center_size(SQUARE_CENTER, Vec2::splat(square_size(feeds)))
}

// --- the tap targets --------------------------------------------------

/// The feed button: large, centred, bottom-anchored — single-thumb reach on a
/// phone (GDD §7). Stated once; the draw system and the checks both read it.
pub fn feed_button() -> Rect {
    Rect::from_center_size(Vec2::new(0.0, VIEW_MAX.y - 1.8), Vec2::new(11.0, 2.6))
}

/// The leaderboard panel: the band between the readout and the button. Drawn
/// over the square (a higher layer), so the square growing under it is fine.
pub fn leaderboard_panel() -> Rect {
    Rect {
        min: Vec2::new(-6.25, 2.2),
        max: Vec2::new(6.25, 8.4),
    }
}

/// Whether a world-space point feeds the orange man.
///
/// A tap on the button **or** on the man himself counts (GDD §4.1). The square
/// grows, so its current bounds are a parameter rather than a constant.
pub fn is_feed_tap(world: Vec2, square: Rect) -> bool {
    feed_button().contains(world) || square.contains(world)
}

// --- the leaderboard projection -------------------------------------

/// One feeder on the board.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The display name.
    pub name: String,
    /// Their feed count.
    pub count: u64,
    /// Whether this is the player.
    pub you: bool,
}

/// What the leaderboard draws: the top rows, and the player's own row when they
/// are not among them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    /// The top `n` feeders, count-descending.
    pub top: Vec<Entry>,
    /// The player's 1-based rank over the whole field.
    pub your_rank: usize,
    /// How many feeders there are in all.
    pub field: usize,
    /// The player's own entry, repeated here when `your_rank > top.len()` so the
    /// draw code has a pinned row without re-scanning; `None` when the player is
    /// already visible in `top`.
    pub your_row: Option<Entry>,
}

/// Rank `entries` and take the top `n`, with the player's standing worked out.
///
/// Sorts count-descending, breaking ties by name so the order is stable across
/// runs and machines (Concepts, "Iteration order is deterministic"). `entries`
/// is consumed because the caller rebuilds it every tick from the backend.
pub fn project_board(mut entries: Vec<Entry>, n: usize) -> Board {
    entries.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    let field = entries.len();
    let your_rank = entries
        .iter()
        .position(|entry| entry.you)
        .map(|index| index + 1)
        .unwrap_or(field + 1);
    let top: Vec<Entry> = entries.iter().take(n).cloned().collect();
    let your_row = if your_rank > top.len() {
        entries.into_iter().find(|entry| entry.you)
    } else {
        None
    };
    Board {
        top,
        your_rank,
        field,
        your_row,
    }
}
