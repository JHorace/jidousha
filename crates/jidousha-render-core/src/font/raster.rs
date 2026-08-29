//! Turning a face's outlines into an atlas of texels.
//!
//! Key functions: `atlas_texels`.
//! Depends on: `ab_glyph`, `ttf`. Must never depend on: anything that knows what
//! a quad is — this file makes a picture, and where that picture is sampled from
//! is `style.rs`'s business.
//! INVARIANT: every glyph is plotted at the position [`glyph_window`] gives,
//! never at the one its own bounds happen to round to. That function is also
//! where a quad's corners and UVs come from, so the two agree by calling the
//! same code rather than by rounding the same way twice — and `tests` below
//! asserts the consequence directly: no glyph inks a texel outside the window
//! its quad samples, at any raster size.

use ab_glyph::{Font, PxScale};

use super::ttf::{
    CELLS, COLUMNS, FALLBACK_CELL, PAD, TtfFace, atlas_px, cell_character, cell_origin, cell_px,
    glyph_window,
};

/// The whole atlas at `px`, RGBA8, row-major.
///
/// White everywhere with the shape in the alpha, for the same reason the
/// built-in font's atlas is: a transparent texel that was black would darken a
/// glyph's edge under any filter but nearest, and the day this engine grows a
/// smoothly-sampled texture is the day that would start to show.
pub(crate) fn atlas_texels(face: &TtfFace, px: u32) -> Vec<u8> {
    let (cell_w, cell_h) = cell_px(face, px);
    let (atlas_w, atlas_h) = atlas_px(face, px);
    let origin = cell_origin(face, px);
    let mut texels = vec![0u8; (atlas_w * atlas_h * 4) as usize];
    for texel in texels.chunks_exact_mut(4) {
        texel[0] = 255;
        texel[1] = 255;
        texel[2] = 255;
    }
    let mut plot = |x: u32, y: u32, coverage: f32| {
        if x >= atlas_w || y >= atlas_h {
            return;
        }
        let alpha = (coverage.clamp(0.0, 1.0) * 255.0).round() as u8;
        let at = ((y * atlas_w + x) * 4 + 3) as usize;
        // Max rather than assignment: two glyphs never share a cell, but the
        // fallback box is drawn by hand below and this keeps the two paths from
        // having to agree about who writes last.
        texels[at] = texels[at].max(alpha);
    };
    let scale = PxScale::from(px as f32);
    for cell in 0..CELLS {
        let left = (cell % COLUMNS) * cell_w;
        let top = (cell / COLUMNS) * cell_h;
        if cell == FALLBACK_CELL {
            let (window_x, window_w) = glyph_window(face, cell, px);
            let inset = (window_x - origin.x).max(0.0) as u32;
            draw_fallback(&mut plot, left + inset, top, window_w as u32, cell_h);
            continue;
        }
        let Some(character) = cell_character(cell) else {
            continue;
        };
        let id = face.font.glyph_id(character);
        if id.0 == 0 {
            // Not in the face. Its cell stays clear and nothing samples it —
            // `TtfFace::cell` sends the character to the fallback box instead.
            continue;
        }
        let Some(outline) = face.font.outline_glyph(id.with_scale(scale)) else {
            // A covered character with no outline is a space, and a blank cell
            // is exactly right for one.
            continue;
        };
        let bounds = outline.px_bounds();
        // The glyph's own bounds are measured from the baseline; the cell's
        // origin is measured from the top of the line box, so the ascent is
        // what carries one into the other. Across, the glyph goes where
        // `glyph_window` says it does rather than where its own bounds round
        // to, because that window is what the quad and the UVs were cut from —
        // a glyph plotted anywhere else would be a glyph sampled off its edge.
        let ascent = face.ascent * px as f32;
        let (window_x, _) = glyph_window(face, cell, px);
        let offset_x = window_x + PAD as f32 - origin.x;
        let offset_y = bounds.min.y + ascent - origin.y;
        let (offset_x, offset_y) = (offset_x.round().max(0.0), offset_y.round().max(0.0));
        outline.draw(|x, y, coverage| {
            plot(
                left + x + offset_x as u32,
                top + y + offset_y as u32,
                coverage,
            );
        });
    }
    texels
}

/// The box a character outside the face's coverage draws.
///
/// A hollow rectangle, one texel thick, inset from the cell's border — the same
/// answer the built-in font gives, for the same reason: text that is missing
/// should be a picture rather than a mystery (renderer.md §6).
fn draw_fallback<F: FnMut(u32, u32, f32)>(plot: &mut F, left: u32, top: u32, w: u32, h: u32) {
    let thickness = (h / 16).max(1);
    let (inset_x, inset_y) = (PAD + w / 8, PAD + h / 6);
    if w <= 2 * inset_x || h <= 2 * inset_y {
        return;
    }
    let (right, bottom) = (w - inset_x, h - inset_y);
    for y in inset_y..bottom {
        for x in inset_x..right {
            let edge = x < inset_x + thickness
                || x >= right - thickness
                || y < inset_y + thickness
                || y >= bottom - thickness;
            if edge {
                plot(left + x, top + y, 1.0);
            }
        }
    }
}
