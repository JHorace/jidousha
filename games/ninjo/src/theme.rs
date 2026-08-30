//! The palette and the type scale — UI.md §2's colour roles, once (UI.md §2).
//!
//! Every colour ninjo draws is named here and nowhere else, in the roles the
//! specification gives them rather than in the places they happen to be used:
//! `EMBER` is "a refusal, a death, a cost", not "the colour of the party
//! strip's third line". A role that appears twice on screen is one constant
//! read twice, which is what makes "changing a signifier is a UI.md edit"
//! (§2) a thing a reader can check.
//!
//! **Sizes are in reference pixels**, because this game's world unit *is* a
//! reference pixel (see `layout.rs`). So `SMALL` being 12 is literally UI.md
//! §7's floor — "no text smaller than the equivalent of 12px at reference
//! scale" — and `floors.rs` asserts it rather than trusting this comment.

use jidousha::prelude::*;

/// Hex, the way the mockup states every colour, at full opacity.
///
/// A `const fn` so the palette below is constants rather than a lazy table,
/// and so a colour that does not parse is a compile error rather than a
/// surprise on screen. `Color::rgb` takes 0.0-1.0 (there are no 0-255
/// constructors in v1, by convention), and this is the one place that
/// conversion happens.
const fn hex(value: u32) -> Color {
    Color::rgb(
        ((value >> 16) & 0xff) as f32 / 255.0,
        ((value >> 8) & 0xff) as f32 / 255.0,
        (value & 0xff) as f32 / 255.0,
    )
}

/// Behind everything, including the letterbox the scaling contract leaves.
pub const VOID: Color = hex(0x0d0b14);
/// The ground the board sits on.
pub const GROUND: Color = hex(0x14121d);
/// A panel's fill.
pub const PANEL: Color = hex(0x1e1b2b);
/// A bar's fill - the top bar and the party strip.
pub const BAR: Color = hex(0x1a1726);
/// The party strip, a shade under the bar.
pub const STRIP: Color = hex(0x17141f);
/// An ordinary border.
pub const BORDER: Color = hex(0x363050);
/// Parchment: ordinary text.
pub const INK: Color = hex(0xe8ddc4);
/// Dim text: descriptions, arithmetic, everything secondary.
pub const DIM: Color = hex(0x8d84a0);
/// Dimmer still: the lock line and the return hint.
pub const FAINT: Color = hex(0x55506b);
/// Selection, payout, and the player's interests. **Not a general accent.**
pub const GOLD: Color = hex(0xe0b34a);
/// The shadow under a live button.
pub const GOLD_DEEP: Color = hex(0xa97e2f);
/// Refusal, blocking, death, betrayal, and every cost.
pub const EMBER: Color = hex(0xd4553a);
/// Regard, and a member who is in.
pub const REGARD: Color = hex(0x4fae8f);
/// A dead button's fill.
pub const BUTTON_DEAD: Color = hex(0x3a3450);
/// A ghost button's fill.
pub const GHOST: Color = hex(0x262238);

// ── the map's terrain fills (grid.rs is the reader; one colour per kind) ──

/// A road tile.
pub const ROAD: Color = hex(0x8a7a55);
/// A plains tile.
pub const PLAINS: Color = hex(0x2e3b2a);
/// A forest tile.
pub const FOREST: Color = hex(0x1d2f22);
/// A rough tile.
pub const ROUGH: Color = hex(0x3a3244);
/// A water tile.
pub const WATER: Color = hex(0x1b2b46);
/// A peak tile.
pub const PEAK: Color = hex(0x4a4553);

/// The scrim the log drawer and the resolution takeover lay over the board.
pub const SCRIM: Color = Color::rgba(0.055, 0.051, 0.09, 0.96);
/// The tuning drawer's border: gold, held back, so the one drawer that changes
/// what the simulation reads says which drawer it is (UI.md §12).
pub const TUNE_EDGE: Color = Color::rgba(0.878, 0.702, 0.290, 0.55);

/// Draw bands. Named once so no bare `layer: 1` appears at a call site.
pub mod layers {
    /// The terrain tiles — the world's floor.
    pub const TERRAIN: i16 = -8;
    /// Location markers, over the terrain.
    pub const MARKER: i16 = -7;
    /// Party tokens, over the markers.
    pub const TOKEN: i16 = -6;
    /// Location labels — text that pans with the map.
    pub const MAP_TEXT: i16 = -5;
    /// Bars, panels and cards.
    pub const PANEL: i16 = -3;
    /// A card's inner fill and its selection ring.
    pub const CARD: i16 = -2;
    /// Portraits, icons and button faces.
    pub const PIECE: i16 = -1;
    /// Every glyph on the board.
    pub const TEXT: i16 = 1;
    /// The log drawer and the resolution takeover, over the board entirely.
    pub const OVERLAY: i16 = 4;
    /// Glyphs and art belonging to an overlay.
    pub const OVERLAY_TEXT: i16 = 6;
}

/// A panel heading, a quest name, a button's label.
pub const HEAD: f32 = 13.0;
/// Body text: everything a player reads a sentence of.
pub const BODY: f32 = 13.0;
/// The smallest text ninjo draws — UI.md §7's floor, exactly.
pub const SMALL: f32 = 12.0;

/// The floor itself, so the assertion and the scale read the same constant.
pub const MIN_TEXT: f32 = 12.0;
/// The smallest a clickable target may be, on either axis (UI.md §7).
pub const MIN_TARGET: f32 = 32.0;

/// A style at `size` in `color`, on the board's text band.
pub fn text(size: f32, color: Color) -> TextStyle {
    TextStyle {
        face: Face::BUILT_IN,
        size,
        color,
        depth: Depth::layer(layers::TEXT),
    }
}
