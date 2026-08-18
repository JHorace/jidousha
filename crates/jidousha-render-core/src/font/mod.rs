//! The engine's own font: five texels by seven, compiled in, no assets.
//!
//! Key types: `TextStyle`; `atlas_texels`, `layout`, `glyph_quad`.
//! Depends on: `jidousha-core`, `glyphs`. Must never depend on:
//! `jidousha-assets` — the whole point of an embedded font is that it works
//! before anything has loaded, and on a machine where nothing ever will
//! (renderer.md §6).
//! INVARIANT: glyphs expand into the same `Quad`s a sprite does, sampling an
//! atlas like any other texture. There is no text pipeline, no text shader, and
//! no second sort — which is why text obeys layers and z exactly like art does.

mod glyphs;

use jidousha_core::math::Vec2;
use jidousha_core::{Color, Depth, Quad, Rect, TextureId};

use self::glyphs::{CELL_H, CELL_W, COLUMNS, ROWS, cell_index, ink};

/// The atlas is this wide, in texels.
pub(crate) const ATLAS_W: u32 = COLUMNS * CELL_W;
/// The atlas is this tall, in texels.
pub(crate) const ATLAS_H: u32 = ROWS * CELL_H;

/// The id every glyph quad samples.
///
/// Reserved rather than allocated: `TextureHandle::texture_id` packs a
/// generation of at least one into the high half, so every id below `1 << 32`
/// is free for the renderer's own textures and can never collide with an asset.
/// [`TextureId::WHITE`] is zero; this is one.
///
/// Public so a verification can ask **"is there text on screen?"** — resolve it
/// through the frame's `TextureTable` and the quads sampling that texture are
/// the glyphs (renderer.md §9). It is not a second way to draw text: `ctx.text`
/// remains the only one, and this names what that produced.
pub const FONT_TEXTURE: TextureId = TextureId::from_bits(1);

/// How a line of text is drawn — monospace over the ninety-five printable ASCII
/// characters, space through `~`, every one of them advancing 7/9 of `size`,
/// with anything outside that range drawn as a visible box rather than skipped.
/// Every glyph's quad is exactly `size` tall and `size * 7 / 9` wide and is laid
/// out from its **top-left** corner, so a line placed at `at` occupies
/// `at.y ..= at.y + size` vertically and `width_of(text)` horizontally, and an
/// N-line block is `N * size` tall. There is no `height_of`, because that is the
/// whole of it: the vertical extent of text is `size` times the number of lines.
///
/// Inside that box the ink is the middle seven ninths, with one ninth of clear
/// border above and below — which is what `size` "including the gap" means, and
/// why consecutive lines a full `size` apart do not touch.
///
/// ```
/// # use jidousha_render_core::TextStyle;
/// # use jidousha_core::Color;
/// let style = TextStyle {
///     size: 1.5,                 // one line is 1.5 world units tall
///     color: Color::WHITE,
///     ..TextStyle::default()
/// };
/// # let _ = style;
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// One line's height in world units — a glyph quad, top to bottom.
    ///
    /// World units rather than points or texels, so text scales with the camera
    /// like everything else and a game never thinks about pixels
    /// (conventions).
    pub size: f32,
    /// Multiplied into the glyphs, which are white.
    pub color: Color,
    /// Where in the draw order, same as every other immediate primitive.
    pub depth: Depth,
}

impl Default for TextStyle {
    /// White, one unit tall, in the middle of the draw order.
    ///
    /// DELIBERATE: a `Default` that means something (ADR-0012) — "some readable
    /// text, here" is what a debug readout wants and what a prototype starts
    /// with.
    fn default() -> Self {
        Self {
            size: 1.0,
            color: Color::WHITE,
            depth: Depth::default(),
        }
    }
}

impl TextStyle {
    /// In world units — its widest line only, so a block centres crooked.
    ///
    /// Monospace with no kerning, so this is exact rather than an estimate.
    /// A game centers a score with it, or draws a panel behind a readout;
    /// without it the only way to know is to guess.
    ///
    /// Every character advances the same `size * 7 / 9`, so a layout can be
    /// reasoned about before it is run: an N-character line is `N * 7 / 9 *
    /// size` wide, and whether it fits is arithmetic rather than a thing to
    /// discover from a transcript.
    ///
    /// `\n` starts a new line, so a multi-line string laid out at one position
    /// is a block, and this is the width of that block. Centering by it is the
    /// documented idiom and it is completely silent: nothing warns that the
    /// result is wider than the camera can see, so a banner overruns the screen
    /// with every assertion still passing. `Camera::visible_bounds` is what
    /// tells you, and *Testing your game* has the check.
    ///
    /// **Centering a multi-line block by this centers only its longest line.**
    /// The block is laid out from one top-left corner, so subtracting half of
    /// the widest line puts that line in the middle and hangs every shorter one
    /// off to the left of centre — a two-line banner of uneven lengths draws
    /// visibly crooked, on screen, at the right size, with the bounds check and
    /// the glyph count both passing. This is a different failure from the
    /// overrun above and nothing catches it. Centre each line by its own width,
    /// with one `Submit::text` call per line (e0-findings.md F-060).
    #[must_use]
    pub fn width_of(&self, text: &str) -> f32 {
        let longest = text
            .split('\n')
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        longest as f32 * self.advance()
    }

    /// How far one character moves the pen, in world units.
    fn advance(&self) -> f32 {
        self.size * CELL_W as f32 / CELL_H as f32
    }
}

/// One glyph's quad, ready to submit.
pub(crate) struct Glyph {
    /// Where the cell's top-left corner sits, in world space.
    pub(crate) at: Vec2,
    /// Which cell of the atlas to sample.
    pub(crate) region: Rect,
}

/// Lay `text` out from `origin`, which is the top-left of the first cell.
///
/// Y is down (ADR-0010), so successive lines are at increasing Y. `\n` starts a
/// new line; nothing else wraps, which is the documented non-goal — a prototype
/// that needs paragraphs needs a different subsystem (renderer.md §6).
pub(crate) fn layout(origin: Vec2, text: &str, style: &TextStyle) -> Vec<Glyph> {
    let advance = style.advance();
    let mut glyphs = Vec::with_capacity(text.len());
    let mut pen = origin;
    for character in text.chars() {
        if character == '\n' {
            pen = Vec2::new(origin.x, pen.y + style.size);
            continue;
        }
        glyphs.push(Glyph {
            at: pen,
            region: glyph_region(character),
        });
        pen.x += advance;
    }
    glyphs
}

/// The quad for one laid-out glyph.
pub(crate) fn glyph_quad(glyph: &Glyph, style: &TextStyle) -> Quad {
    let size = Vec2::new(style.size * CELL_W as f32 / CELL_H as f32, style.size);
    let min = glyph.at;
    let max = min + size;
    Quad {
        corners: [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)],
        uvs: [
            glyph.region.min,
            Vec2::new(glyph.region.max.x, glyph.region.min.y),
            glyph.region.max,
            Vec2::new(glyph.region.min.x, glyph.region.max.y),
        ],
        tint: style.color,
        texture: FONT_TEXTURE,
        depth: style.depth,
    }
}

/// Which part of the atlas a character samples, in normalized coordinates.
fn glyph_region(character: char) -> Rect {
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
        let region = glyph_region(character);
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
        assert_eq!(glyph_region('\u{2603}'), glyph_region('\u{4e2d}'));
        assert!(
            picture('\u{2603}').contains('#'),
            "the fallback is visible:\n{}",
            picture('\u{2603}')
        );
        assert_ne!(glyph_region('A'), glyph_region('\u{2603}'));
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

    #[test]
    fn text_advances_by_one_cell_per_character() {
        let style = TextStyle {
            size: 9.0,
            ..TextStyle::default()
        };
        let glyphs = layout(Vec2::ZERO, "abc", &style);
        assert_eq!(glyphs.len(), 3);
        // A cell is 7 wide and 9 tall, so at size 9 the advance is exactly 7.
        assert_eq!(glyphs[0].at, Vec2::ZERO);
        assert_eq!(glyphs[1].at, Vec2::new(7.0, 0.0));
        assert_eq!(glyphs[2].at, Vec2::new(14.0, 0.0));
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
        assert_eq!(glyphs[0].at, Vec2::new(5.0, 5.0));
        assert_eq!(glyphs[1].at, Vec2::new(5.0, 14.0));
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
    fn a_multi_line_string_measures_its_widest_line() {
        let style = TextStyle {
            size: 9.0,
            ..TextStyle::default()
        };
        assert_eq!(style.width_of("a\nlonger\nx"), style.width_of("longer"));
    }

    #[test]
    fn a_glyph_quad_samples_only_its_own_cell() {
        let style = TextStyle::default();
        let glyphs = layout(Vec2::ZERO, "A", &style);
        let quad = glyph_quad(&glyphs[0], &style);
        let region = glyph_region('A');
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
        // — so the trap is pinned here and named in `width_of`'s own entry.
        let style = TextStyle::default();
        let block = "LONGEST LINE\nshort";
        let left = -style.width_of(block) / 2.0;
        let glyphs = layout(Vec2::new(left, 0.0), block, &style);

        let advance = style.advance();
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
        assert_eq!(glyphs[0].at.x, glyphs[12].at.x);
        assert!(glyphs[12].at.y > glyphs[0].at.y, "and one line lower");
    }

    #[test]
    fn the_font_id_can_never_be_an_asset_id() {
        // The reservation this whole scheme rests on: assets pack a generation
        // of at least one into the high half, so their ids start at 1 << 32.
        assert!(FONT_TEXTURE.bits() < 1 << 32);
        assert_ne!(FONT_TEXTURE, TextureId::WHITE);
    }
}
