//! What a game says about text: which face, how big, what colour — and how big
//! the result is.
//!
//! Key types: `Face`, `TextStyle`, `TextExtents`.
//! Depends on: `jidousha-core`, `bitmap`, `store`, `ttf`.
//! INVARIANT: measuring needs nothing but the style. A `Face` carries its own
//! metrics, so `TextStyle::measure` is a method on a `Copy` value with no store
//! in reach — which is what lets a game's layout module, which has no world and
//! no borrow of anything, measure the text it is about to place (ADR-0042).

use jidousha_core::math::Vec2;
use jidousha_core::{Color, Depth, Rect, TextureId};

use super::ttf::TtfFace;
use super::{FONT_TEXTURE, bitmap, store, ttf};

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
pub struct Face(pub(super) Kind);

/// What a [`Face`] actually is.
#[derive(Clone, Copy, Debug)]
pub(super) enum Kind {
    /// The compiled-in five-by-seven bitmap.
    BuiltIn,
    /// A parsed TTF face, kept for the life of the program (see
    /// [`Fonts`](super::Fonts)).
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
    pub(super) fn advance(self, character: char) -> f32 {
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
    pub(super) fn cell(self, character: char, size: f32) -> (Rect, Rect, TextureId) {
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
        // A size of zero, a negative one, or a NaN: no width holds any number
        // of characters, and the promise this makes is that the count fits.
        if !advance.is_finite() || advance <= 0.0 {
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
