//! Text: the engine's own five-by-seven font, and any TTF face a game loads.
//!
//! Key types: `TextStyle`, `Face`, `TextExtents`, `Fonts`; `layout`,
//! `glyph_quad`.
//! Depends on: `jidousha-core`, `ab_glyph`, `bitmap`, `glyphs`, `store`, `ttf`.
//! Must never depend on: `jidousha-assets` — the built-in font has to work
//! before anything has loaded, and on a machine where nothing ever will
//! (renderer.md §6), and a TTF face is built from bytes the caller already
//! holds rather than from a store this crate reaches into (ADR-0042).
//! INVARIANT: glyphs expand into the same `Quad`s a sprite does, sampling an
//! atlas like any other texture. There is no text pipeline, no text shader, and
//! no second sort — which is why text obeys layers and z exactly like art does,
//! and why a TTF face costs the renderer one more texture and nothing else.

mod bitmap;
mod glyphs;
mod raster;
mod store;
mod style;
mod ttf;

use jidousha_core::math::Vec2;
use jidousha_core::{Quad, Rect, TextureId};

pub use store::{Fonts, upload_text_atlases};
pub use style::{Face, TextExtents, TextStyle};
pub use ttf::FontError;

/// The id every built-in glyph quad samples.
///
/// Reserved rather than allocated: `TextureHandle::texture_id` packs a
/// generation of at least one into the high half, so every id below `1 << 32`
/// is free for the renderer's own textures and can never collide with an asset.
/// [`TextureId::WHITE`] is zero; this is one, and a loaded face's atlases take
/// the ids above it (ADR-0042).
///
/// Public so a verification can ask **"is there text on screen?"** — resolve it
/// through the frame's `TextureTable` and the quads sampling that texture are
/// the glyphs (renderer.md §9). It is not a second way to draw text: `ctx.text`
/// remains the only one, and this names what that produced.
///
/// It names the **built-in** font only. A game drawing in a loaded face asks
/// [`Face::atlas_texture`] which id that face's text lands on, because a
/// proportional face has one atlas per size rather than one in total.
pub const FONT_TEXTURE: TextureId = TextureId::from_bits(1);

/// One glyph's quad, ready to submit.
pub(crate) struct Glyph {
    /// Where the cell sits, in world space.
    pub(crate) rect: Rect,
    /// Which cell of the atlas to sample.
    pub(crate) region: Rect,
    /// Which atlas — the built-in font's, or one of a loaded face's sizes.
    pub(crate) texture: TextureId,
}

/// Lay `text` out from `origin`, which is the top-left of the first line.
///
/// Y is down (ADR-0010), so successive lines are at increasing Y. `\n` starts a
/// new line; nothing else wraps, which is the documented non-goal — a prototype
/// that needs paragraphs needs a different subsystem (renderer.md §6).
pub(crate) fn layout(origin: Vec2, text: &str, style: &TextStyle) -> Vec<Glyph> {
    let mut glyphs = Vec::with_capacity(text.len());
    let mut pen = origin;
    for character in text.chars() {
        if character == '\n' {
            pen = Vec2::new(origin.x, pen.y + style.size);
            continue;
        }
        let (cell, region, texture) = style.face.cell(character, style.size);
        glyphs.push(Glyph {
            rect: Rect::from_min_size(pen + cell.min, cell.size()),
            region,
            texture,
        });
        pen.x += style.face.advance(character) * style.size;
    }
    glyphs
}

/// The quad for one laid-out glyph.
pub(crate) fn glyph_quad(glyph: &Glyph, style: &TextStyle) -> Quad {
    let (min, max) = (glyph.rect.min, glyph.rect.max);
    Quad {
        corners: [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)],
        uvs: [
            glyph.region.min,
            Vec2::new(glyph.region.max.x, glyph.region.min.y),
            glyph.region.max,
            Vec2::new(glyph.region.min.x, glyph.region.max.y),
        ],
        tint: style.color,
        texture: glyph.texture,
        depth: style.depth,
    }
}

/// The built-in font's atlas, for whoever is uploading it.
pub(crate) fn builtin_atlas() -> (u32, u32, Vec<u8>) {
    (bitmap::ATLAS_W, bitmap::ATLAS_H, bitmap::atlas_texels())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_advances_by_one_cell_per_character() {
        let style = TextStyle {
            size: 9.0,
            ..TextStyle::default()
        };
        let glyphs = layout(Vec2::ZERO, "abc", &style);
        assert_eq!(glyphs.len(), 3);
        // A cell is 7 wide and 9 tall, so at size 9 the advance is exactly 7.
        assert_eq!(glyphs[0].rect.min, Vec2::ZERO);
        assert_eq!(glyphs[1].rect.min, Vec2::new(7.0, 0.0));
        assert_eq!(glyphs[2].rect.min, Vec2::new(14.0, 0.0));
    }

    #[test]
    fn a_space_is_a_glyph_and_a_newline_is_not() {
        // A game asserting an exact glyph count on a line it drew needs to know
        // which characters cost a quad, and space is the one nobody can guess
        // (e0-findings.md F-076). It is one of the ninety-five printable ASCII
        // characters this font covers, with a blank cell of its own, so it is
        // laid out and advances like any other.
        let style = TextStyle {
            size: 9.0,
            ..TextStyle::default()
        };
        let glyphs = layout(Vec2::ZERO, "a b", &style);
        assert_eq!(glyphs.len(), 3, "the space is a glyph");
        assert_eq!(glyphs[2].rect.min, Vec2::new(14.0, 0.0), "and it advanced");
        assert_eq!(layout(Vec2::ZERO, "a\nb", &style).len(), 2);
    }

    #[test]
    fn a_newline_returns_to_the_left_and_moves_down() {
        // Down is +Y (ADR-0010), and a newline submits no glyph of its own.
        let style = TextStyle {
            size: 9.0,
            ..TextStyle::default()
        };
        let glyphs = layout(Vec2::new(5.0, 5.0), "a\nb", &style);
        assert_eq!(glyphs.len(), 2, "the newline is not a glyph");
        assert_eq!(glyphs[0].rect.min, Vec2::new(5.0, 5.0));
        assert_eq!(glyphs[1].rect.min, Vec2::new(5.0, 14.0));
    }

    #[test]
    fn the_measured_width_is_what_the_glyphs_actually_occupy() {
        // A game centers a score with this, so it has to agree with layout
        // rather than approximate it.
        let style = TextStyle {
            size: 9.0,
            ..TextStyle::default()
        };
        assert_eq!(style.width_of("abc"), 21.0);
        let glyphs = layout(Vec2::ZERO, "abc", &style);
        let last = glyph_quad(&glyphs[2], &style);
        assert_eq!(last.corners[1].x, style.width_of("abc"));
    }

    #[test]
    fn the_column_count_and_the_measured_width_are_the_same_ratio_read_two_ways() {
        // `width_of`'s doc states the advance as `size * 7 / 9` and
        // `columns_in` is that sentence read backwards, so a game never has to
        // rediscover the fraction (games/giri/FINDINGS.md G-003). This is the
        // assertion that keeps the two in step: a round trip through both, at
        // sizes and lengths that do not divide evenly.
        for size in [0.7_f32, 1.0, 9.0, 13.5] {
            let style = TextStyle {
                size,
                ..TextStyle::default()
            };
            for length in [1_usize, 2, 7, 40, 137] {
                let line = "x".repeat(length);
                let width = style.width_of(&line);
                assert_eq!(
                    style.columns_in(width),
                    length,
                    "size {size}: {length} characters measure {width} and must count back"
                );
                // And it never overruns: one more character than it reported
                // is wider than the column it was asked about.
                let fitted = style.columns_in(width);
                assert!(style.width_of(&"x".repeat(fitted + 1)) > width);
            }
        }
    }

    #[test]
    fn a_column_narrower_than_one_character_fits_nothing() {
        // The floor, at the only boundary a layout actually meets: a panel too
        // narrow for a single glyph reports zero rather than one, so a caller
        // slicing by the count draws nothing instead of overrunning.
        let style = TextStyle {
            size: 9.0,
            ..TextStyle::default()
        };
        assert_eq!(style.columns_in(6.9), 0);
        assert_eq!(style.columns_in(7.0), 1);
        assert_eq!(style.columns_in(0.0), 0);
        assert_eq!(style.columns_in(-10.0), 0);
    }

    #[test]
    fn a_multi_line_string_measures_its_widest_line() {
        let style = TextStyle {
            size: 9.0,
            ..TextStyle::default()
        };
        assert_eq!(style.width_of("a\nlonger\nx"), style.width_of("longer"));
        assert_eq!(style.measure("a\nlonger\nx").lines, 3);
        assert_eq!(style.measure("a\nlonger\nx").size.y, 27.0);
    }

    #[test]
    fn a_glyph_quad_samples_only_its_own_cell() {
        let style = TextStyle::default();
        let glyphs = layout(Vec2::ZERO, "A", &style);
        let quad = glyph_quad(&glyphs[0], &style);
        let region = bitmap::region('A');
        // All four corners, for the same reason the rectangle test checks all
        // four: the opposite pair alone would let the other two swap.
        assert_eq!(quad.uvs[0], region.min, "top-left");
        assert_eq!(quad.uvs[1], Vec2::new(region.max.x, region.min.y));
        assert_eq!(quad.uvs[2], region.max, "bottom-right");
        assert_eq!(quad.uvs[3], Vec2::new(region.min.x, region.max.y));
        assert_eq!(quad.texture, FONT_TEXTURE);
    }

    #[test]
    fn centering_a_block_by_its_width_leaves_the_short_line_left_of_centre() {
        // e0-findings.md F-060. `width_of` is the widest line and `layout` runs
        // from one top-left corner, so the documented centring idiom centres
        // the longest line and hangs the rest off to its left. The geometry is
        // correct, every existing assertion passes, and the picture is crooked
        // — so the trap is pinned here and named in `measure`'s own entry.
        let style = TextStyle::default();
        let block = "LONGEST LINE\nshort";
        let left = -style.width_of(block) / 2.0;
        let glyphs = layout(Vec2::new(left, 0.0), block, &style);

        let advance = bitmap::CELL_ASPECT * style.size;
        let long_end = left + 12.0 * advance;
        let short_end = left + 5.0 * advance;
        assert!(
            (left + long_end).abs() < 1e-5,
            "the longest line is the one that ends up centred"
        );
        assert!(
            short_end < -1e-3,
            "the short line ends left of centre: it runs to {short_end}"
        );
        // And both lines really do start at the same x, which is why.
        assert_eq!(glyphs[0].rect.min.x, glyphs[12].rect.min.x);
        assert!(
            glyphs[12].rect.min.y > glyphs[0].rect.min.y,
            "one line lower"
        );
    }

    #[test]
    fn the_font_id_can_never_be_an_asset_id() {
        // The reservation this whole scheme rests on: assets pack a generation
        // of at least one into the high half, so their ids start at 1 << 32.
        assert!(FONT_TEXTURE.bits() < 1 << 32);
        assert_ne!(FONT_TEXTURE, TextureId::WHITE);
    }

    #[test]
    fn fitting_a_string_to_a_width_never_overruns_it() {
        let style = TextStyle {
            size: 9.0,
            ..TextStyle::default()
        };
        let line = "the quick brown fox";
        for width in [0.0_f32, 6.9, 7.0, 30.0, 55.5, 1000.0] {
            let fitted = style.fits_in(line, width);
            let head: String = line.chars().take(fitted).collect();
            assert!(
                style.width_of(&head) <= width || fitted == 0,
                "{fitted} characters of {line:?} measure {} in {width}",
                style.width_of(&head)
            );
            if fitted < line.chars().count() {
                let more: String = line.chars().take(fitted + 1).collect();
                assert!(style.width_of(&more) > width, "and one more does not fit");
            }
        }
    }

    #[test]
    fn the_built_in_face_is_itself_and_nothing_else() {
        assert_eq!(Face::BUILT_IN, Face::BUILT_IN);
        assert_eq!(Face::BUILT_IN.name(), "built-in");
        assert_eq!(TextStyle::default().face, Face::BUILT_IN);
    }
}
