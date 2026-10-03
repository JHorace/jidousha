//! What every screen draws with: ninjo's bands and palette, bound onto the
//! engine's UI kit.
//!
//! **The substrate is `jidousha::ui`** — `Panel`, `TextRun`, `IconRun`,
//! `wrap`, `clipped`, the floors — the kit this game was the exemplar for
//! (ADR-0046). What is here is the game's half: which band a row or an icon
//! is born on, which face and colour a measurement is made in, and the draw
//! helpers that take the palette. Nothing below names a number of its own.
//!
//! **Nothing here decides anything.** The arithmetic on screen is `Preview`,
//! which `flow::refresh_preview` fills from the same `assess` and `admit` the
//! send verb and the click handler gate on. The UI cannot disagree with the
//! resolution because the UI does not compute (DESIGN invariant 2; ADR-0039).
//!
//! **Every screen hands back its content as data first, and draws it second.**
//! A `Panel` is every string and every icon a screen puts on the frame, with
//! its position — so `verify.rs` can assert a glyph run of exactly
//! `text.chars().count()` at each `at`, `contracts.rs` can check every string
//! is ASCII the font can draw, and `floors.rs` can check UI.md §7's floors
//! against the same values the frame was built from. Three readers, one layout.
//!
//! Text sizes and colours come from `theme.rs`; every rectangle from
//! `layout.rs`.

use jidousha::prelude::*;
use jidousha::ui::TextRun;

use crate::sprites::{Art, Gallery};
use crate::theme;

/// Everything one screen puts on the frame, as data, over ninjo's art.
pub type Panel = jidousha::ui::Panel<Art>;

/// One icon a screen draws, over ninjo's art.
pub type IconRun = jidousha::ui::IconRun<Art>;

/// A row on the board's text band.
pub fn row(at: Vec2, text: impl Into<String>, size: f32, color: Color) -> TextRun {
    TextRun::new(at, text, theme::text(size, color))
}

/// The same, on an overlay's.
pub fn over(at: Vec2, text: impl Into<String>, size: f32, color: Color) -> TextRun {
    TextRun::new(
        at,
        text,
        TextStyle {
            depth: Depth::layer(theme::layers::OVERLAY_TEXT),
            ..theme::text(size, color)
        },
    )
}

/// An icon on the board's piece band, untinted.
pub fn icon(at: Vec2, art: Art, scale: f32) -> IconRun {
    IconRun::new(at, art, scale, theme::layers::PIECE)
}

/// How many characters of `size` fit across `width` world units.
///
/// One call to the engine's own measurement (`TextStyle::columns_in`), so the
/// font's advance ratio appears nowhere in this game.
pub fn columns(width: f32, size: f32) -> usize {
    theme::text(size, theme::INK).columns_in(width)
}

/// Centre a label horizontally in `rect`, at its `top` — the kit's one
/// baseline rule, measured in this game's face.
pub fn centered(rect: Rect, text: &str, size: f32, top: f32) -> Vec2 {
    jidousha::ui::centered(rect, &theme::text(size, theme::INK), text, top)
}

/// Draw a panel's contents through the UI mapping, with the gallery's art:
/// map-space lists as they are, chrome through the mapping — one transform,
/// applied at the last moment, so every reader of the layout reads it
/// untransformed.
pub fn draw(ctx: &mut DrawCtx, panel: &Panel, map: &crate::camera::UiMap) {
    let gallery = ctx.world.resource::<Gallery>().clone();
    panel.draw(ctx, map, |icon, scale| {
        gallery.sprite(icon.art, scale, icon.layer, icon.tint)
    });
}

/// Fill a rectangle.
pub fn fill(ctx: &mut DrawCtx, rect: Rect, color: Color, layer: i16) {
    ctx.rect(rect, color, Depth::layer(layer));
}

/// Draw a border `thickness` wide, inside `rect`.
pub fn border(ctx: &mut DrawCtx, rect: Rect, color: Color, thickness: f32, layer: i16) {
    let depth = Depth::layer(layer);
    let size = rect.size();
    ctx.rect(
        Rect::from_min_size(rect.min, Vec2::new(size.x, thickness)),
        color,
        depth,
    );
    ctx.rect(
        Rect::from_min_size(
            Vec2::new(rect.min.x, rect.max.y - thickness),
            Vec2::new(size.x, thickness),
        ),
        color,
        depth,
    );
    ctx.rect(
        Rect::from_min_size(rect.min, Vec2::new(thickness, size.y)),
        color,
        depth,
    );
    ctx.rect(
        Rect::from_min_size(
            Vec2::new(rect.max.x - thickness, rect.min.y),
            Vec2::new(thickness, size.y),
        ),
        color,
        depth,
    );
}

/// A button: a face, and the pressed-looking shadow under it.
///
/// `live` is the whole difference between a button that will do something and
/// one that will not — colour *and* the shadow, because colour alone is one
/// channel and UI.md §1 asks for two.
pub fn button(ctx: &mut DrawCtx, rect: Rect, live: bool, layer: i16) {
    let (face, shadow) = if live {
        (theme::GOLD, theme::GOLD_DEEP)
    } else {
        (theme::BUTTON_DEAD, theme::GHOST)
    };
    fill(
        ctx,
        Rect::from_min_size(rect.min + Vec2::new(0.0, 3.0), rect.size()),
        shadow,
        layer,
    );
    fill(ctx, rect, face, layer + 1);
}
