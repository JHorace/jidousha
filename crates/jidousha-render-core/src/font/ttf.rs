//! A parsed TTF face: its metrics in line units, and the atlas it rasterizes to.
//!
//! Key types: `TtfFace`, `FontError`, `atlas_texels`, `cell_px`, `cell_origin`.
//! Depends on: `ab_glyph`, `jidousha-core`. Must never depend on:
//! `jidousha-assets` — a face is built from bytes somebody else fetched, so the
//! store's contract stays the store's (ADR-0042).
//! INVARIANT: every number a draw needs is a pure function of the face and the
//! raster size. The atlas layout is computed the same way in two places — here,
//! when the texels are made, and in `mod.rs`, when a quad's corners and UVs are
//! worked out — and they agree because both call the functions below rather
//! than each doing the arithmetic. Nothing about a drawn glyph depends on
//! whether its atlas has been uploaded yet, which is what lets text lay out
//! correctly on the frame before the GPU has seen the face.

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use jidousha_core::Rect;
use jidousha_core::math::Vec2;

/// The printable ASCII range this crate draws, space through `~`.
const ASCII: core::ops::RangeInclusive<u32> = 0x20..=0x7E;
/// The Latin-1 supplement, no-break space through `ÿ`.
///
/// 0x80–0x9F are C1 control codes and are not characters anybody sets, so the
/// covered range starts at the no-break space (ADR-0042).
const LATIN1: core::ops::RangeInclusive<u32> = 0xA0..=0xFF;

/// How many cells the ASCII range takes.
const ASCII_CELLS: u32 = 0x7E - 0x20 + 1;
/// How many cells the Latin-1 supplement takes.
const LATIN1_CELLS: u32 = 0xFF - 0xA0 + 1;

/// Which cell the fallback box lives in — after every covered character.
pub(crate) const FALLBACK_CELL: u32 = ASCII_CELLS + LATIN1_CELLS;

/// How many cells an atlas has: every covered character, plus the fallback.
pub(crate) const CELLS: u32 = FALLBACK_CELL + 1;

/// Cells across an atlas.
///
/// Sixteen, like the built-in font's, so the two atlases read the same way when
/// somebody dumps one to a PNG to see what went wrong.
pub(crate) const COLUMNS: u32 = 16;
/// Cells down an atlas.
pub(crate) const ROWS: u32 = CELLS.div_ceil(COLUMNS);

/// The transparent border around every glyph's ink, in texels.
///
/// The built-in font's border does two jobs and so does this one: it is what
/// makes nearest sampling safe at any scale, because a fragment landing a hair
/// outside a glyph finds clear border rather than its neighbour's ink.
pub(crate) const PAD: u32 = 1;

/// The smallest line a face is rasterized at, in texels.
///
/// Below about six texels a proportional face is mush whatever the rasterizer
/// does, and an atlas that small costs nothing to keep. Clamping rather than
/// refusing, because the alternative is text that vanishes at a size a game
/// picked for a reason.
pub(crate) const MIN_PX: u32 = 6;

/// The largest line a face is rasterized at, in texels.
///
/// Sixty-four keeps a 16×12 grid of cells inside the 2048×2048 texture the
/// WebGL2 envelope guarantees (renderer.md §8) with room to spare, and text
/// larger than this scales the 64-texel atlas up — which is the same bargain
/// every sprite in the engine makes.
pub(crate) const MAX_PX: u32 = 64;

/// The size the face's ink extents are measured at, in texels.
///
/// Big enough that rounding one texel either way changes the measured extent by
/// well under a percent, which is all this number is for.
const REF_PX: f32 = 256.0;

/// What went wrong turning bytes into a face.
///
/// One case, because there is one thing that can go wrong: the bytes are not a
/// font this crate can read. Everything else — the file being missing, the load
/// still being in flight — belongs to the asset store and is answered there
/// (assets.md §1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontError {
    /// What the face was going to be called.
    pub(crate) name: String,
    /// How many bytes were handed over.
    pub(crate) bytes: usize,
}

impl core::fmt::Display for FontError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "[jidousha] the font \"{}\" could not be parsed\n  \
             specifics: {} byte(s) were handed to the renderer and none of them parsed as a \
             TrueType or OpenType face\n  \
             likely cause: the bytes are not a font file — a truncated download, an HTML error \
             page saved under a .ttf name, or a .woff2, which this engine does not read\n  \
             fix: check the file at the path the asset store loaded, and commit a .ttf or .otf",
            self.name, self.bytes
        )
    }
}

impl core::error::Error for FontError {}

/// A face, parsed, with everything a layout needs precomputed.
///
/// **Metrics are in line units**: one unit is one line, which is exactly what
/// [`TextStyle::size`](super::TextStyle::size) means. A face therefore measures
/// the same at every size, and a size is one multiplication away at the point
/// of use.
pub(crate) struct TtfFace {
    /// Which face this is, among those created on one `Fonts`.
    pub(crate) id: u32,
    /// What the game called it — carried for error messages and `Debug`.
    pub(crate) name: String,
    /// The outlines.
    pub(crate) font: FontVec,
    /// Each covered cell's advance, in line units.
    advances: Vec<f32>,
    /// Whether the face actually has a glyph for each covered cell.
    covered: Vec<bool>,
    /// The widest advance of any covered character, in line units.
    max_advance: f32,
    /// Each cell's ink span across, in line units, measured from the pen.
    ///
    /// Cells are a uniform grid — one size for the whole atlas, so a cell's
    /// place in it is arithmetic — but a *quad* is cut down to the columns its
    /// own glyph actually inks. Without this every glyph would be drawn as wide
    /// as the widest one in the face, and a check asking whether a row of text
    /// fits inside a panel would measure an `i` as an `M`.
    ink_x: Vec<(f32, f32)>,
    /// The union of every covered glyph's ink, in line units, measured from the
    /// pen at the **top** of the line box.
    ink_min: Vec2,
    /// That union's size, in line units.
    ink_size: Vec2,
    /// How far the baseline sits below the top of the line box, in line units.
    pub(crate) ascent: f32,
}

impl core::fmt::Debug for TtfFace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // The outlines are megabytes and nothing reads them from a dump.
        f.debug_struct("Face")
            .field("name", &self.name)
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl TtfFace {
    /// Parse `bytes` and measure everything a layout will ask for.
    ///
    /// The measuring pass rasterizes every covered glyph once, at [`REF_PX`],
    /// to find the ink extents. PERF: a few milliseconds per face, paid once
    /// when the face is created and never on a frame.
    pub(crate) fn parse(id: u32, name: &str, bytes: &[u8]) -> Result<Self, FontError> {
        let Ok(font) = FontVec::try_from_vec(bytes.to_vec()) else {
            return Err(FontError {
                name: name.to_owned(),
                bytes: bytes.len(),
            });
        };
        let scaled = font.as_scaled(PxScale::from(REF_PX));
        // `PxScale::y` is the line box by definition — ascent minus descent —
        // so a face scaled to REF_PX has a REF_PX line, and dividing by it puts
        // every measurement into line units.
        let ascent = scaled.ascent() / REF_PX;

        let mut advances = Vec::with_capacity(CELLS as usize);
        let mut covered = Vec::with_capacity(CELLS as usize);
        let mut ink_x = Vec::with_capacity(CELLS as usize);
        let mut max_advance = 0.0_f32;
        let (mut min, mut max) = (Vec2::new(f32::MAX, f32::MAX), Vec2::new(f32::MIN, f32::MIN));
        for cell in 0..CELLS {
            let Some(character) = cell_character(cell) else {
                // The fallback cell, whose box is drawn rather than outlined,
                // and which therefore fills the cell it is given.
                advances.push(max_advance);
                covered.push(true);
                ink_x.push((0.0, max_advance));
                continue;
            };
            let id = font.glyph_id(character);
            // Glyph zero is `.notdef`: the face was asked for a character it
            // does not have. Recorded rather than guessed at, so a draw can
            // reach for the fallback box instead of an empty cell.
            let has = id.0 != 0;
            covered.push(has);
            let advance = scaled.h_advance(id) / REF_PX;
            advances.push(advance);
            // A character with no ink — a space — still gets a quad the width
            // of its advance, sampling the clear cell it was never drawn into.
            // One quad per character, whatever the character is: a game
            // counting the glyphs it drew counts the same number in either
            // face (e0-findings.md F-076).
            let mut span = (0.0, advance);
            if has {
                max_advance = max_advance.max(advance);
                if let Some(outline) = font.outline_glyph(id.with_scale(PxScale::from(REF_PX))) {
                    let bounds = outline.px_bounds();
                    // `px_bounds` is measured from the baseline with Y down, so
                    // adding the ascent moves it onto the line box's top.
                    min = min.min(Vec2::new(bounds.min.x, bounds.min.y) / REF_PX);
                    max = max.max(Vec2::new(bounds.max.x, bounds.max.y) / REF_PX);
                    span = (bounds.min.x / REF_PX, bounds.max.x / REF_PX);
                }
            }
            ink_x.push(span);
        }
        if min.x > max.x || min.y > max.y {
            // A face with no drawable glyph in the covered range. Nothing can
            // be laid out against it, so it is not a face this engine can use.
            return Err(FontError {
                name: name.to_owned(),
                bytes: bytes.len(),
            });
        }
        // The fallback box is drawn inside the ink extents, so it needs no room
        // of its own; its advance is the widest real one, so a run of unknown
        // characters spaces like text rather than piling up.
        let ink_min = Vec2::new(min.x, min.y + ascent);
        Ok(Self {
            id,
            name: name.to_owned(),
            font,
            advances,
            covered,
            max_advance,
            ink_x,
            ink_min,
            ink_size: max - min,
            ascent,
        })
    }

    /// The cell `character` samples — its own, or the fallback box.
    pub(crate) fn cell(&self, character: char) -> u32 {
        let Some(cell) = character_cell(character) else {
            return FALLBACK_CELL;
        };
        if self.covered.get(cell as usize).copied().unwrap_or(false) {
            cell
        } else {
            FALLBACK_CELL
        }
    }

    /// How far `character` moves the pen, in line units.
    pub(crate) fn advance(&self, character: char) -> f32 {
        let cell = self.cell(character);
        self.advances.get(cell as usize).copied().unwrap_or(0.0)
    }

    /// The widest advance any covered character has, in line units.
    pub(crate) fn max_advance(&self) -> f32 {
        self.max_advance
    }
}

/// The columns a cell's glyph inks at `px`, relative to the pen, in texels.
///
/// The one piece of arithmetic that has to agree in two places — here, where a
/// quad's corners and UVs come from, and in [`atlas_texels`], where the glyph is
/// plotted — so both call this rather than each rounding for itself. `floor` and
/// `ceil` outwards plus a border on each side, so the window can only ever be
/// wider than the ink, never narrower.
pub(crate) fn glyph_window(face: &TtfFace, cell: u32, px: u32) -> (f32, f32) {
    let Some(&(min, max)) = face.ink_x.get(cell as usize) else {
        return (0.0, 0.0);
    };
    if max <= min {
        return (0.0, 0.0);
    }
    let (cell_w, _) = cell_px(face, px);
    let origin = cell_origin(face, px).x;
    let scale = px as f32;
    let left = ((min * scale).floor() - PAD as f32).max(origin);
    let right = ((max * scale).ceil() + PAD as f32).min(origin + cell_w as f32);
    (left, (right - left).max(0.0))
}

/// Which cell a character belongs in, or `None` if it is outside Latin-1.
fn character_cell(character: char) -> Option<u32> {
    let code = character as u32;
    if ASCII.contains(&code) {
        Some(code - ASCII.start())
    } else if LATIN1.contains(&code) {
        Some(ASCII_CELLS + code - LATIN1.start())
    } else {
        None
    }
}

/// Which character a cell holds, or `None` for the fallback cell.
pub(crate) fn cell_character(cell: u32) -> Option<char> {
    let code = if cell < ASCII_CELLS {
        ASCII.start() + cell
    } else if cell < FALLBACK_CELL {
        LATIN1.start() + cell - ASCII_CELLS
    } else {
        return None;
    };
    char::from_u32(code)
}

/// The raster size a line of `size` world units is drawn from, in texels.
///
/// One texel per world unit, rounded, and clamped to the range an atlas is
/// worth keeping. DELIBERATE: this does **not** consult the camera (ADR-0042).
/// A camera-derived raster size would be sharper at every zoom and would make
/// the atlas a function of the window, which is environmental — the same
/// picture would come back at two resolutions on two machines, and a golden
/// image could not be compared.
pub(crate) fn raster_px(size: f32) -> u32 {
    if !size.is_finite() {
        return MIN_PX;
    }
    (size.round().max(0.0) as u32).clamp(MIN_PX, MAX_PX)
}

/// One cell's size at `px`, in texels — ink plus the border, the same for
/// every character.
pub(crate) fn cell_px(face: &TtfFace, px: u32) -> (u32, u32) {
    let width = (face.ink_size.x * px as f32).ceil().max(1.0) as u32 + 2 * PAD;
    let height = (face.ink_size.y * px as f32).ceil().max(1.0) as u32 + 2 * PAD;
    (width, height)
}

/// Where a cell's top-left corner sits relative to the pen, in texels.
///
/// The pen is at the top of the line box, which is where `ctx.text` puts it.
pub(crate) fn cell_origin(face: &TtfFace, px: u32) -> Vec2 {
    Vec2::new(
        (face.ink_min.x * px as f32).floor() - PAD as f32,
        (face.ink_min.y * px as f32).floor() - PAD as f32,
    )
}

/// The whole atlas's size at `px`, in texels.
pub(crate) fn atlas_px(face: &TtfFace, px: u32) -> (u32, u32) {
    let (width, height) = cell_px(face, px);
    (width * COLUMNS, height * ROWS)
}

/// Which part of the atlas a cell's glyph samples, in normalized coordinates.
///
/// The cell's row is the whole cell's — a glyph is as tall as the line box
/// however short it is, so a run of text has one baseline and one top edge —
/// and its column span is [`glyph_window`], so an `i` samples an `i`'s worth of
/// atlas rather than a `W`'s worth of mostly-clear cell.
pub(crate) fn region(face: &TtfFace, cell: u32, px: u32) -> Rect {
    let (cell_w, cell_h) = cell_px(face, px);
    let (atlas_w, atlas_h) = atlas_px(face, px);
    let (window_x, window_w) = glyph_window(face, cell, px);
    let left = (cell % COLUMNS) * cell_w;
    let top = (cell / COLUMNS) * cell_h;
    let inset = window_x - cell_origin(face, px).x;
    Rect::from_min_size(
        Vec2::new(
            (left as f32 + inset) / atlas_w as f32,
            top as f32 / atlas_h as f32,
        ),
        Vec2::new(window_w / atlas_w as f32, cell_h as f32 / atlas_h as f32),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::raster::atlas_texels;

    /// The face every test here measures against.
    ///
    /// Compiled in rather than loaded, because this is a unit test of the
    /// arithmetic and a unit test that needs a working directory is a unit test
    /// that fails on somebody's machine. It is the same file `assets/fonts/`
    /// ships and `CREDITS.md` names.
    const FIRA: &[u8] = include_bytes!("../../../../assets/fonts/FiraSans-Regular.ttf");

    fn face() -> TtfFace {
        match TtfFace::parse(0, "Fira Sans", FIRA) {
            Ok(face) => face,
            Err(error) => panic!("{error}"),
        }
    }

    #[test]
    fn every_covered_glyph_is_rasterized_inside_the_window_its_quad_samples() {
        // The invariant the whole scheme rests on, and the one that cannot be
        // seen from outside: a quad's corners and its UVs are cut from
        // `glyph_window`, and the texels are plotted from the same function, so
        // ink outside that window is ink a glyph samples off its own edge —
        // which draws as a sliver of the letter next door, at the size where
        // rounding happened to go the other way, on one machine.
        let face = face();
        for px in [MIN_PX, 7, 11, 12, 23, 24, 47, MAX_PX] {
            let texels = atlas_texels(&face, px);
            let (cell_w, cell_h) = cell_px(&face, px);
            let (atlas_w, _) = atlas_px(&face, px);
            let origin = cell_origin(&face, px).x;
            for cell in 0..CELLS {
                let (window_x, window_w) = glyph_window(&face, cell, px);
                let inset = (window_x - origin) as u32;
                let (left, top) = ((cell % COLUMNS) * cell_w, (cell / COLUMNS) * cell_h);
                for y in 0..cell_h {
                    for x in 0..cell_w {
                        let inside = x >= inset && x < inset + window_w as u32;
                        if inside {
                            continue;
                        }
                        let alpha = texels[(((top + y) * atlas_w + left + x) * 4 + 3) as usize];
                        assert_eq!(
                            alpha,
                            0,
                            "at {px} texels, cell {cell} inks column {x}, outside its window \
                             {inset}..{}",
                            inset + window_w as u32
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_glyphs_window_never_leaves_the_cell_it_belongs_to() {
        // The other half of the same invariant, stated as arithmetic rather
        // than as texels: a window that ran past its cell would sample the
        // next character along even where nothing was drawn.
        let face = face();
        for px in [MIN_PX, 13, 24, MAX_PX] {
            let (cell_w, _) = cell_px(&face, px);
            let origin = cell_origin(&face, px).x;
            for cell in 0..CELLS {
                let (window_x, window_w) = glyph_window(&face, cell, px);
                assert!(window_x >= origin, "cell {cell} starts left of its cell");
                assert!(
                    window_x + window_w <= origin + cell_w as f32,
                    "cell {cell} runs past its cell at {px} texels"
                );
            }
        }
    }

    #[test]
    fn the_atlas_fits_inside_the_texture_size_every_backend_guarantees() {
        // renderer.md §8: 2048 is the WebGL2 envelope, and an atlas over it
        // would upload on a desktop and fail on the platform this engine
        // targets. `MAX_PX` is the number that keeps this true.
        let face = face();
        let (width, height) = atlas_px(&face, MAX_PX);
        assert!(width <= 2048 && height <= 2048, "{width}x{height}");
    }

    #[test]
    fn ascii_and_latin_1_are_covered_and_everything_else_is_the_box() {
        let face = face();
        for code in 0x20..=0x7E_u32 {
            let Some(character) = char::from_u32(code) else {
                continue;
            };
            assert_ne!(face.cell(character), FALLBACK_CELL, "{character:?}");
        }
        for code in 0xA0..=0xFF_u32 {
            let Some(character) = char::from_u32(code) else {
                continue;
            };
            assert_ne!(face.cell(character), FALLBACK_CELL, "{character:?}");
        }
        // Outside Latin-1, and outside the BMP: the box, not a panic and not a
        // skipped character (renderer.md §6).
        for character in [
            '\u{2603}',
            '\u{4e2d}',
            '\u{1F600}',
            '\u{7F}',
            '\u{80}',
            '\u{9F}',
        ] {
            assert_eq!(face.cell(character), FALLBACK_CELL, "{character:?}");
        }
    }

    #[test]
    fn the_fallback_box_is_visible_and_advances_like_a_character() {
        let face = face();
        let px = 24;
        let texels = atlas_texels(&face, px);
        let (cell_w, cell_h) = cell_px(&face, px);
        let (atlas_w, _) = atlas_px(&face, px);
        let (left, top) = (
            (FALLBACK_CELL % COLUMNS) * cell_w,
            (FALLBACK_CELL / COLUMNS) * cell_h,
        );
        let mut ink = 0;
        for y in 0..cell_h {
            for x in 0..cell_w {
                if texels[(((top + y) * atlas_w + left + x) * 4 + 3) as usize] > 0 {
                    ink += 1;
                }
            }
        }
        assert!(ink > 0, "the box a stray codepoint draws is visible");
        assert!(face.advance('\u{2603}') > 0.0, "and it moves the pen");
    }

    #[test]
    fn a_proportional_face_measures_proportionally() {
        let face = face();
        assert!(
            face.advance('i') < face.advance('W'),
            "an i is narrower than a W, which is the whole reason `measure` exists"
        );
        assert!(face.advance(' ') > 0.0, "and a space still advances");
        assert!(face.max_advance() >= face.advance('W'));
    }

    #[test]
    fn a_raster_size_is_clamped_rather_than_refused() {
        // A game is allowed to ask for text a tenth of a unit tall. It gets an
        // atlas it can read scaled down, rather than a panic or a zero-sized
        // texture.
        assert_eq!(raster_px(0.0), MIN_PX);
        assert_eq!(raster_px(-4.0), MIN_PX);
        assert_eq!(raster_px(f32::NAN), MIN_PX);
        assert_eq!(raster_px(f32::INFINITY), MIN_PX);
        assert_eq!(raster_px(1e9), MAX_PX);
        assert_eq!(raster_px(24.4), 24);
        assert_eq!(raster_px(24.5), 25);
    }
}
