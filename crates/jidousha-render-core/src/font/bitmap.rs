//! The built-in font's own atlas: cells, regions, and texels.
//!
//! Key types: `ATLAS_W`, `ATLAS_H`, `atlas_texels`, `region`, `CELL_ASPECT`.
//! Depends on: `glyphs`, `jidousha-core`. Must never depend on: `ab_glyph` —
//! this face is compiled in and has no outlines to rasterize, which is the
//! whole reason it works before any asset exists (renderer.md §6).
//! INVARIANT: every number here is derived from `glyphs.rs`'s picture table, so
//! the art is still the only place a glyph's shape is stated.

use jidousha_core::Rect;
use jidousha_core::math::Vec2;

use super::glyphs::{CELL_H, CELL_W, COLUMNS, ROWS, cell_index, ink};

/// The atlas is this wide, in texels.
pub(crate) const ATLAS_W: u32 = COLUMNS * CELL_W;
/// The atlas is this tall, in texels.
pub(crate) const ATLAS_H: u32 = ROWS * CELL_H;

/// How wide one cell is as a fraction of a line — the monospace advance.
///
/// A cell is 7 texels across and 9 down, and [`TextStyle::size`] is the line,
/// so every character advances seven ninths of it.
///
/// [`TextStyle::size`]: super::TextStyle::size
pub(crate) const CELL_ASPECT: f32 = CELL_W as f32 / CELL_H as f32;

/// Which part of the atlas a character samples, in normalized coordinates.
pub(crate) fn region(character: char) -> Rect {
    let index = cell_index(character);
    let column = index % COLUMNS;
    let row = index / COLUMNS;
    let min = Vec2::new(
        (column * CELL_W) as f32 / ATLAS_W as f32,
        (row * CELL_H) as f32 / ATLAS_H as f32,
    );
    let size = Vec2::new(
        CELL_W as f32 / ATLAS_W as f32,
        CELL_H as f32 / ATLAS_H as f32,
    );
    Rect::from_min_size(min, size)
}

/// The whole atlas, RGBA8, row-major.
///
/// White everywhere, with alpha carrying the shape: a transparent texel is
/// white-and-invisible rather than black-and-invisible, so nothing dark can
/// bleed into a glyph's edge if the sampler ever stops being nearest.
pub(crate) fn atlas_texels() -> Vec<u8> {
    let mut texels = vec![0u8; (ATLAS_W * ATLAS_H * 4) as usize];
    for (index, texel) in texels.chunks_exact_mut(4).enumerate() {
        texel[0] = 255;
        texel[1] = 255;
        texel[2] = 255;
        let index = index as u32;
        texel[3] = if ink(index % ATLAS_W, index / ATLAS_W) {
            255
        } else {
            0
        };
    }
    texels
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A character as a picture, read back out of the atlas.
    ///
    /// The font is data, and data with no way to look at it is data that drifts.
    /// This is how a test — or an agent wondering why the score looks wrong —
    /// asks what a glyph actually is.
    fn picture(character: char) -> String {
        let region = region(character);
        let left = (region.min.x * ATLAS_W as f32).round() as u32;
        let top = (region.min.y * ATLAS_H as f32).round() as u32;
        let mut out = String::new();
        for y in 0..CELL_H {
            for x in 0..CELL_W {
                out.push(if ink(left + x, top + y) { '#' } else { '.' });
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn a_glyph_reads_back_out_of_the_atlas_as_it_was_drawn() {
        // 'A' end to end: the table, the cell arithmetic, and the border.
        assert_eq!(
            picture('A'),
            ".......\n\
             ...#...\n\
             ..#.#..\n\
             .#...#.\n\
             .#...#.\n\
             .#####.\n\
             .#...#.\n\
             .#...#.\n\
             .......\n"
        );
    }

    #[test]
    fn a_character_the_font_does_not_have_draws_the_fallback_box() {
        // Not nothing. A missing glyph that drew nothing would make "my text is
        // half there" a mystery (renderer.md §5's reasoning, applied to text).
        assert_eq!(region('\u{2603}'), region('\u{4e2d}'));
        assert!(
            picture('\u{2603}').contains('#'),
            "the fallback is visible:\n{}",
            picture('\u{2603}')
        );
        assert_ne!(region('A'), region('\u{2603}'));
    }

    #[test]
    fn the_atlas_is_white_with_the_shape_in_its_alpha() {
        let texels = atlas_texels();
        assert_eq!(texels.len(), (ATLAS_W * ATLAS_H * 4) as usize);
        // Every texel is white; only alpha varies. A transparent texel that was
        // black would darken a glyph's edge under any filter but nearest.
        assert!(
            texels.chunks_exact(4).all(|t| t[0..3] == [255, 255, 255]),
            "every texel is white"
        );
        assert!(texels.chunks_exact(4).any(|t| t[3] == 255), "some ink");
        assert!(texels.chunks_exact(4).any(|t| t[3] == 0), "some space");
    }

    #[test]
    fn the_atlas_alpha_is_the_glyph_shape_and_not_its_negative() {
        // "Some opaque and some transparent" is true of an inverted atlas too,
        // and an inverted font draws solid blocks with letter-shaped holes —
        // which is legible enough at a glance to survive a screenshot. Every
        // texel is compared to the art it came from instead.
        let texels = atlas_texels();
        for y in 0..ATLAS_H {
            for x in 0..ATLAS_W {
                let alpha = texels[((y * ATLAS_W + x) * 4 + 3) as usize];
                let expected = if ink(x, y) { 255 } else { 0 };
                assert_eq!(alpha, expected, "texel ({x}, {y})");
            }
        }
    }
}
