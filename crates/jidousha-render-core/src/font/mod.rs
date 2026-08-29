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
mod store;
mod ttf;

use jidousha_core::math::Vec2;
use jidousha_core::{Color, Depth, Quad, Rect, TextureId};

pub use store::{Fonts, upload_text_atlases};
pub use ttf::FontError;

use self::ttf::TtfFace;

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

/// Which typeface a style draws in.
///
/// [`Face::BUILT_IN`] is the engine's own five-by-seven bitmap: monospace,
/// compiled in, and available on the first frame of a program before anything
/// has loaded. Everything else comes from [`Fonts::try_create_face`] and is
/// proportional, real type, at whatever size the style asks for.
///
/// Copyable and cheap — a face is a name for outlines the [`Fonts`] store keeps
/// for the life of the program, so a style holding one costs a pointer and a
/// game can put styles in constants, clone them, and pass them by value exactly
/// as it did when there was one font.
#[derive(Clone, Copy, Debug)]
pub struct Face(Kind);

/// What a [`Face`] actually is.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// The compiled-in five-by-seven bitmap.
    BuiltIn,
    /// A parsed TTF face, kept for the life of the program (see [`Fonts`]).
    Ttf(&'static TtfFace),
}

impl Face {
    /// The engine's own font: monospace, five texels by seven, no assets.
    pub const BUILT_IN: Face = Face(Kind::BuiltIn);

    /// What this face is called — `"built-in"`, or the name it was created
    /// with.
    ///
    /// For error messages and for a readout that wants to say which face it is
    /// drawing in. Faces are compared by identity, not by name: two faces
    /// created from the same bytes are two faces.
    #[must_use]
    pub fn name(&self) -> &str {
        match self.0 {
            Kind::BuiltIn => "built-in",
            Kind::Ttf(face) => &face.name,
        }
    }

    /// Which texture this face's glyphs sample at a line height of `size`.
    ///
    /// The answer to *"which of these quads is my heading?"* for a loaded face,
    /// and the counterpart of [`FONT_TEXTURE`] for the built-in one: resolve it
    /// through the frame's `TextureTable` and every quad sampling it is a glyph
    /// of this face at this size (renderer.md §9). A face is rasterized once
    /// per size, so two styles of the same size share an id and two sizes do
    /// not.
    ///
    /// For [`Face::BUILT_IN`] this is [`FONT_TEXTURE`] whatever the size — one
    /// compiled-in atlas serves every size of the bitmap font.
    #[must_use]
    pub fn atlas_texture(&self, size: f32) -> TextureId {
        match self.0 {
            Kind::BuiltIn => FONT_TEXTURE,
            Kind::Ttf(face) => store::atlas_texture(face.id, ttf::raster_px(size)),
        }
    }

    /// How far `character` moves the pen, as a fraction of one line.
    fn advance(self, character: char) -> f32 {
        match self.0 {
            Kind::BuiltIn => bitmap::CELL_ASPECT,
            Kind::Ttf(face) => face.advance(character),
        }
    }

    /// The advance every character shares, if the face is monospace.
    ///
    /// `None` for a proportional face, where there is no such number — which is
    /// the whole reason [`TextStyle::measure`] exists. It is not only a fast
    /// path: a fixed advance is measured by *multiplication*, and a run of a
    /// hundred characters added up one at a time lands a rounding step below
    /// the same run multiplied, so `columns_in` would count one column short of
    /// what `width_of` had just measured (the round trip games/giri G-003 asked
    /// for).
    fn uniform_advance(self) -> Option<f32> {
        match self.0 {
            Kind::BuiltIn => Some(bitmap::CELL_ASPECT),
            Kind::Ttf(_) => None,
        }
    }

    /// The widest advance any character has, as a fraction of one line.
    fn max_advance(self) -> f32 {
        match self.0 {
            Kind::BuiltIn => bitmap::CELL_ASPECT,
            Kind::Ttf(face) => face.max_advance(),
        }
    }

    /// The atlas `character` samples, and the cell's rectangle relative to the
    /// pen, in world units, for a line `size` tall.
    fn cell(self, character: char, size: f32) -> (Rect, Rect, TextureId) {
        match self.0 {
            Kind::BuiltIn => (
                Rect::from_min_size(Vec2::ZERO, Vec2::new(bitmap::CELL_ASPECT * size, size)),
                bitmap::region(character),
                FONT_TEXTURE,
            ),
            Kind::Ttf(face) => {
                let px = ttf::raster_px(size);
                // The atlas is a picture of a line `px` texels tall; the style
                // asks for one `size` world units tall. Everything the cell
                // knows is in texels, so one multiplication puts it in the
                // world — the same bargain a sprite drawn at a scale makes.
                let per_texel = size / px as f32;
                let cell = face.cell(character);
                let (_, cell_h) = ttf::cell_px(face, px);
                let (window_x, window_w) = ttf::glyph_window(face, cell, px);
                // Down the line box, every glyph is the same height, so a run
                // of text has one top edge and one baseline. Across, each glyph
                // is only as wide as it inks.
                let origin = Vec2::new(window_x, ttf::cell_origin(face, px).y) * per_texel;
                let extent = Vec2::new(window_w, cell_h as f32) * per_texel;
                (
                    Rect::from_min_size(origin, extent),
                    ttf::region(face, cell, px),
                    store::atlas_texture(face.id, px),
                )
            }
        }
    }
}

impl PartialEq for Face {
    /// By identity: the built-in font is itself, and a loaded face is the one
    /// the `Fonts` store handed back.
    fn eq(&self, other: &Self) -> bool {
        match (self.0, other.0) {
            (Kind::BuiltIn, Kind::BuiltIn) => true,
            (Kind::Ttf(left), Kind::Ttf(right)) => core::ptr::eq(left, right),
            _ => false,
        }
    }
}

/// How big a laid-out string is.
///
/// What [`TextStyle::measure`] answers, and the thing a layout is solved
/// against: the box the pen sweeps, from the top-left corner a draw would put
/// the text at. Ink can lean a hair outside it — that is what a proportional
/// face's overhang is — but nothing is laid out against ink.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextExtents {
    /// Width of the widest line, and height of the whole block, in world units.
    pub size: Vec2,
    /// How many lines it is — one more than the number of `\n`s.
    pub lines: usize,
}

/// How a line of text is drawn: in a [`Face`], at a size, in a color, at a
/// depth.
///
/// Every glyph's quad is laid out from the pen, which starts at the **top-left**
/// of the first line, and `size` is one line top to bottom — so a line placed at
/// `at` occupies `at.y ..= at.y + size` vertically and
/// [`measure`](TextStyle::measure)`.size.x` horizontally, and an N-line block is
/// `N * size` tall. There is no `height_of`, because that is the whole of it:
/// the vertical extent of text is `size` times the number of lines.
///
/// In the **built-in** face every character advances 7/9 of `size`, the ink is
/// the middle seven ninths of the cell, and anything outside printable ASCII
/// draws a visible box rather than being skipped. In a **loaded** face the
/// advance is the character's own, the coverage is ASCII plus Latin-1, and
/// anything outside it — or anything the face itself does not have — draws the
/// same kind of box (ADR-0042). Neither face ever refuses a character; text
/// with a stray codepoint in it draws, and the box is what says so.
///
/// ```
/// # use jidousha_render_core::TextStyle;
/// # use jidousha_core::Color;
/// let style = TextStyle {
///     size: 1.5,                 // one line is 1.5 world units tall
///     color: Color::WHITE,
///     ..TextStyle::default()     // the built-in face, in the middle of the order
/// };
/// # let _ = style;
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    /// Which typeface. [`Face::BUILT_IN`] unless a game loaded one.
    pub face: Face,
    /// One line's height in world units — a glyph quad, top to bottom.
    ///
    /// World units rather than points or texels, so text scales with the camera
    /// like everything else and a game never thinks about pixels
    /// (conventions).
    ///
    /// A loaded face is rasterized at one texel per world unit, rounded, so a
    /// game whose world units *are* reference pixels gets type at exactly the
    /// resolution it is drawn at. Sizes below 6 and above 64 are rasterized at
    /// those and scaled, like any other texture.
    pub size: f32,
    /// Multiplied into the glyphs, which are white.
    pub color: Color,
    /// Where in the draw order, same as every other immediate primitive.
    pub depth: Depth,
}

impl Default for TextStyle {
    /// The built-in face, white, one unit tall, in the middle of the draw order.
    ///
    /// DELIBERATE: a `Default` that means something (ADR-0012) — "some readable
    /// text, here" is what a debug readout wants and what a prototype starts
    /// with, and it needs no asset to exist.
    fn default() -> Self {
        Self {
            face: Face::BUILT_IN,
            size: 1.0,
            color: Color::WHITE,
            depth: Depth::default(),
        }
    }
}

impl TextStyle {
    /// How big `text` is when drawn in this style, in world units.
    ///
    /// The measurement API, and the one a proportional face makes necessary:
    /// with the built-in font a game could multiply a character count by an
    /// advance it read in a doc comment, and with real type there is no such
    /// number (ADR-0042). A game centres a heading with this, sizes a panel
    /// behind a readout with it, and checks a row against a readability floor
    /// with it.
    ///
    /// `\n` starts a new line, so a multi-line string laid out at one position
    /// is a block: `size.x` is the **widest line** and `size.y` is
    /// `lines * style.size`, the whole block.
    ///
    /// **Centering a multi-line block by `size.x` centers only its longest
    /// line.** The block is laid out from one top-left corner, so subtracting
    /// half of the widest line puts that line in the middle and hangs every
    /// shorter one off to the left of centre — a two-line banner of uneven
    /// lengths draws visibly crooked, on screen, at the right size, with the
    /// bounds check and the glyph count both passing. Centre each line by its
    /// own width, with one `Submit::text` call per line (e0-findings.md F-060).
    ///
    /// It is also completely silent about running off the screen: nothing warns
    /// that the result is wider than the camera can see, so a banner overruns
    /// with every assertion still passing. `Camera::visible_bounds` is what
    /// tells you, and *Testing your game* has the check.
    ///
    /// ```
    /// # use jidousha_render_core::TextStyle;
    /// let style = TextStyle { size: 9.0, ..TextStyle::default() };
    /// let extents = style.measure("two\nlines");
    /// assert_eq!(extents.lines, 2);
    /// assert_eq!(extents.size.y, 18.0);
    /// ```
    #[must_use]
    pub fn measure(&self, text: &str) -> TextExtents {
        let mut widest = 0.0_f32;
        let mut lines = 1;
        let mut run = Run::default();
        for character in text.chars() {
            if character == '\n' {
                widest = widest.max(self.width(run));
                run = Run::default();
                lines += 1;
                continue;
            }
            run.push(self.face.advance(character));
        }
        TextExtents {
            size: Vec2::new(widest.max(self.width(run)), lines as f32 * self.size),
            lines,
        }
    }

    /// How wide a run of characters is, in world units.
    fn width(&self, run: Run) -> f32 {
        match self.face.uniform_advance() {
            Some(advance) => run.characters as f32 * (advance * self.size),
            None => run.units * self.size,
        }
    }

    /// The width of the widest line, in world units.
    ///
    /// `measure(text).size.x`, and named because centring is what a game asks
    /// for far more often than it asks for a block's extents:
    /// `at.x - style.width_of(line) * 0.5` is the whole idiom, and spelling it
    /// `measure(line).size.x * 0.5` puts a field access in the middle of it.
    ///
    /// DELIBERATE: the one place this crate offers a second spelling of
    /// something (ADR-0042). It is not a second *mechanism* — it is
    /// [`measure`](TextStyle::measure) — and every caveat in that entry applies
    /// here unchanged, which is why the caveats live there.
    #[must_use]
    pub fn width_of(&self, text: &str) -> f32 {
        self.measure(text).size.x
    }

    /// How many characters of **any** string fit across `width` world units.
    ///
    /// The pessimistic count, measured with the face's widest character, so the
    /// answer holds whatever the string turns out to say: a line of
    /// `columns_in(w)` characters is never wider than `w`. In the built-in
    /// face, where every character is the same width, that is exact and is the
    /// inverse of [`width_of`](TextStyle::width_of); in a proportional face it
    /// is a floor, and [`fits_in`](TextStyle::fits_in) is the tight answer for
    /// a string you have in your hand.
    ///
    /// This is what a game laying a *generated* string into a fixed column
    /// needs before it generates one: `ctx.text` does not wrap, and `\n` is the
    /// only break, so "how many characters fit" is a question the game has to
    /// answer before it draws.
    ///
    /// Rounds **down**. A width narrower than one character is zero columns.
    ///
    /// ```
    /// # use jidousha_render_core::TextStyle;
    /// let style = TextStyle { size: 0.9, ..TextStyle::default() };
    /// let columns = style.columns_in(30.0);
    /// assert!(style.width_of(&"x".repeat(columns)) <= 30.0);
    /// ```
    #[must_use]
    pub fn columns_in(&self, width: f32) -> usize {
        let advance = self.face.max_advance() * self.size;
        if !(advance > 0.0) {
            return 0;
        }
        // `as` saturates: a negative width is zero columns rather than a wrap.
        (width / advance) as usize
    }

    /// How many leading characters of `text` fit across `width` world units.
    ///
    /// The line-fitting helper: the tight answer, measured character by
    /// character, for a string that already exists. Truncating at the count it
    /// returns gives a line that fits, and one character more does not.
    ///
    /// Stops at the first `\n` — a newline already ends the line, so nothing
    /// past it is competing for this width.
    ///
    /// ```
    /// # use jidousha_render_core::TextStyle;
    /// let style = TextStyle { size: 9.0, ..TextStyle::default() };
    /// let fits = style.fits_in("hello world", 30.0);
    /// let head: String = "hello world".chars().take(fits).collect();
    /// assert!(style.width_of(&head) <= 30.0);
    /// ```
    #[must_use]
    pub fn fits_in(&self, text: &str, width: f32) -> usize {
        let mut run = Run::default();
        for character in text.chars() {
            if character == '\n' {
                break;
            }
            let mut next = run;
            next.push(self.face.advance(character));
            // Measured the way `measure` measures, rather than by adding as we
            // go: the two have to agree about the last character that fits, or
            // a caller truncating at this count draws a line `width_of` then
            // says is too wide.
            if self.width(next) > width {
                break;
            }
            run = next;
        }
        run.characters
    }
}

/// A run of characters being measured: how many, and how wide in line units.
///
/// Both numbers, because a fixed-advance face measures by multiplying the count
/// and a proportional one by adding the widths, and [`TextStyle::width`] is the
/// one place that chooses (see [`Face::uniform_advance`]).
#[derive(Clone, Copy, Default)]
struct Run {
    /// How many characters, newlines excluded.
    characters: usize,
    /// Their total advance, in line units.
    units: f32,
}

impl Run {
    /// Add one character of `advance` line units.
    fn push(&mut self, advance: f32) {
        self.characters += 1;
        self.units += advance;
    }
}

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
