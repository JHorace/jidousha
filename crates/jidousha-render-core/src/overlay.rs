//! A diagnostic readout drawn over the frame, in the corner, at a readable size.
//!
//! Key types: `draw_readout`, `READOUT_LINES_ON_SCREEN`.
//! Depends on: `camera`, `font`, `shapes`, `jidousha-core`.
//! Must never be depended on by: anything a game can reach. This is the
//! platform driver's instrument (frame-pacing.md §6) and it is not re-exported
//! by the facade — the one way a *game* draws text is `ctx.text`, and this is
//! not a second one (renderer.md §6).
//! INVARIANT: the same glyph layout `ctx.text` uses, through the same `layout`
//! and `glyph_quad`, sampling the same built-in atlas. Nothing here is a second
//! text path; what it adds is a corner to pin the block to and a backdrop to
//! make it legible over a scene.
//! INVARIANT: presentation-only, and never called unless a switch outside the
//! engine asked for it. Nothing simulation can observe changes when this draws:
//! the quads are appended after the Draw phase has finished, so the world never
//! sees them and a recorded transcript never contains them.

use jidousha_core::math::Vec2;
use jidousha_core::{Color, Depth, Quad, Rect};

use crate::camera::Camera;
use crate::font::{TextStyle, glyph_quad, layout};
use crate::shapes::rect_quad;

/// How many of the readout's lines would fill the window from top to bottom.
///
/// The overlay is sized as a **fraction of the window** rather than in world
/// units or in device pixels, and each of the three candidates fails somewhere
/// the other two do not:
///
/// - world units would make the readout's size a property of the game's camera,
///   so the same overlay would be unreadable in a game that looks at a hundred
///   units and enormous in one that looks at five;
/// - a fixed device-pixel size is legible on a 1080p monitor and a smear on a
///   4K one, where the same eleven pixels are a third of the height.
///
/// Fifty-four lines to a window is around thirteen pixels at 720p and forty at
/// 2160p — the readout occupies the same visual fraction of any screen, which
/// is what a screenshot in a bug report needs it to do.
pub const READOUT_LINES_ON_SCREEN: f32 = 54.0;

/// How far the block sits from the top-left corner, in lines.
///
/// Measured in lines rather than in pixels for the same reason the size is: a
/// margin that stayed constant while the text scaled would crowd the corner on
/// a big display and float in the middle of nowhere on a small one.
const MARGIN_LINES: f32 = 0.6;

/// How much dark ground is left around the text, in lines.
const PADDING_LINES: f32 = 0.4;

/// What the backdrop is filled with.
///
/// Nearly opaque, because the readout has to be legible over a white sprite as
/// well as over an empty scene, and translucent at all because covering the
/// corner of the frame outright would hide the thing being diagnosed.
const BACKDROP: Color = Color::rgba(0.0, 0.0, 0.0, 0.9);

/// What the text is drawn in.
///
/// Green on near-black, which is the palette the web overlay's panel already
/// uses (web-publish.md §2) — a playtester comparing a native screenshot with a
/// browser one should not have to work out that they are the same instrument.
const INK: Color = Color::rgba(0.62, 1.0, 0.68, 1.0);

/// The layer the readout draws on.
///
/// The top of the band range, so nothing a game can put on screen is over it:
/// an overlay a game's own UI could hide would fail exactly when the game was
/// busy, which is when it is worth reading.
const OVERLAY_LAYER: i16 = i16::MAX;

/// Draw `readout` over the frame, pinned to the top-left corner.
///
/// The quads are appended to `quads` in backdrop-then-text order and carry the
/// topmost layer, so the sort in [`plan_frame`](crate::plan_frame) puts them
/// last however they arrived. `\n` starts a new line and nothing wraps, exactly
/// as `ctx.text` does — a readout wider than the window runs off it, and the
/// caller is the one that knows how long its own lines are.
///
/// `camera` is the frame's own camera, which is what makes the readout land in
/// the corner of the *window* rather than at some world position: a corner is a
/// screen fact, and [`Camera::screen_to_world`] is the one sanctioned way to
/// turn one into a place to draw (conventions).
///
/// A camera with no viewport — zero pixels either way, which is what a
/// minimized window reports — draws nothing rather than dividing by zero.
///
/// ```
/// # use jidousha_render_core::{Camera, overlay::draw_readout};
/// # use jidousha_core::Quad;
/// let camera = Camera::default();
/// let mut quads: Vec<Quad> = Vec::new();
/// draw_readout(&camera, "frame  16.7ms\nfps    60.0", &mut quads);
/// // One backdrop, then one quad per character of both lines.
/// assert_eq!(quads.len(), 1 + "frame  16.7ms".len() + "fps    60.0".len());
/// ```
pub fn draw_readout(camera: &Camera, readout: &str, quads: &mut Vec<Quad>) {
    if camera.viewport.width == 0 || camera.viewport.height == 0 {
        return;
    }
    let line = camera.height / READOUT_LINES_ON_SCREEN;
    let style = TextStyle {
        size: line,
        color: INK,
        depth: Depth {
            layer: OVERLAY_LAYER,
            z: 1.0,
        },
        ..TextStyle::default()
    };

    // The top-left of the *window*, then in by a margin — so the block is in
    // the corner whatever the camera is looking at.
    let corner = camera.screen_to_world(Vec2::ZERO);
    let origin = corner + Vec2::splat(line * (MARGIN_LINES + PADDING_LINES));

    let extents = style.measure(readout);
    let padding = Vec2::splat(line * PADDING_LINES);
    quads.push(rect_quad(
        Rect {
            min: origin - padding,
            max: origin + extents.size + padding,
        },
        BACKDROP,
        Depth {
            layer: OVERLAY_LAYER,
            z: 0.0,
        },
    ));

    for glyph in layout(origin, readout, &style) {
        quads.push(glyph_quad(&glyph, &style));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::FONT_TEXTURE;
    use jidousha_core::TextureId;

    /// A camera describing a plain 800×600 window.
    fn window() -> Camera {
        Camera {
            viewport: jidousha_core::PhysicalSize::new(800, 600),
            ..Camera::default()
        }
    }

    #[test]
    fn a_readout_is_a_backdrop_and_one_quad_per_character() {
        let mut quads = Vec::new();
        draw_readout(&window(), "ab\ncd", &mut quads);
        assert_eq!(quads.len(), 5, "one backdrop and four glyphs");
        assert_eq!(quads[0].texture, TextureId::WHITE, "the backdrop");
        assert!(
            quads[1..].iter().all(|quad| quad.texture == FONT_TEXTURE),
            "the rest are glyphs of the built-in font"
        );
    }

    #[test]
    fn the_readout_draws_over_everything_a_game_can_put_on_screen() {
        // The failure this rules out: a game's own HUD, drawn on the highest
        // layer it thought to use, painting over the instrument. `Depth::layer`
        // takes an `i16`, so the top of that range is a place nothing else can
        // reach — and the backdrop has to sit under its own text.
        let mut quads = Vec::new();
        draw_readout(&window(), "x", &mut quads);
        assert!(
            quads.iter().all(|quad| quad.depth.layer == i16::MAX),
            "nothing a game draws can be above this"
        );
        assert!(
            quads[0].depth.z < quads[1].depth.z,
            "the backdrop is under its own text, not over it"
        );
    }

    #[test]
    fn the_backdrop_covers_every_glyph_it_is_behind() {
        // A backdrop measured from the wrong extents is the bug that makes a
        // readout unreadable over a bright scene while every other assertion
        // here still passes: the text is drawn, in the right place, over
        // nothing. Lines of different lengths, because the widest one is what
        // the panel has to be sized from (`TextStyle::measure`).
        let mut quads = Vec::new();
        draw_readout(&window(), "a much longer line\nshort", &mut quads);
        let backdrop = quads[0].corners;
        let (min, max) = (backdrop[0], backdrop[2]);
        for glyph in &quads[1..] {
            for corner in glyph.corners {
                assert!(
                    corner.x >= min.x && corner.x <= max.x,
                    "a glyph hangs off the backdrop horizontally: {corner:?} in {min:?}..{max:?}"
                );
                assert!(
                    corner.y >= min.y && corner.y <= max.y,
                    "a glyph hangs off the backdrop vertically: {corner:?} in {min:?}..{max:?}"
                );
            }
        }
    }

    #[test]
    fn the_readout_lands_in_the_corner_of_the_window_whatever_the_camera_looks_at() {
        // The reading that makes this an *overlay* rather than something drawn
        // in the world: two cameras looking at different places, at different
        // zooms, put the block at the same place on screen.
        let corner_on_screen = |camera: &Camera| {
            let mut quads = Vec::new();
            draw_readout(camera, "readout", &mut quads);
            camera.world_to_screen(quads[0].corners[0])
        };

        let near = Camera {
            center: Vec2::new(-400.0, 250.0),
            height: 3.0,
            ..window()
        };
        let far = Camera {
            center: Vec2::new(1000.0, -80.0),
            height: 240.0,
            ..window()
        };
        let (a, b) = (corner_on_screen(&near), corner_on_screen(&far));
        assert!(
            (a.x - b.x).abs() < 0.5 && (a.y - b.y).abs() < 0.5,
            "the same corner: {a:?} vs {b:?}"
        );
        assert!(a.x > 0.0 && a.y > 0.0, "inset from the corner, not on it");
    }

    #[test]
    fn the_readout_takes_the_same_share_of_a_small_window_and_a_large_one() {
        // Why the size is a fraction of the window rather than a pixel count:
        // the same screenshot at 720p and at 2160p has to be equally readable,
        // which means the block covers the same fraction of both.
        let share = |height: u32| {
            let camera = Camera {
                viewport: jidousha_core::PhysicalSize::new(height * 16 / 9, height),
                ..Camera::default()
            };
            let mut quads = Vec::new();
            draw_readout(&camera, "present   vsync", &mut quads);
            let backdrop = quads[0].corners;
            let pixels = camera.world_to_screen(backdrop[2]) - camera.world_to_screen(backdrop[0]);
            pixels.y / camera.viewport.height as f32
        };
        let small = share(720);
        let large = share(2160);
        assert!(
            (small - large).abs() < 1e-3,
            "the readout is {small} of a 720p window and {large} of a 2160p one"
        );
    }

    #[test]
    fn a_minimized_window_draws_no_readout_rather_than_dividing_by_zero() {
        let camera = Camera {
            viewport: jidousha_core::PhysicalSize::new(0, 0),
            ..Camera::default()
        };
        let mut quads = Vec::new();
        draw_readout(&camera, "readout", &mut quads);
        assert!(quads.is_empty());
    }

    #[test]
    fn an_empty_readout_still_draws_nothing_but_its_own_backdrop() {
        // Not a special case in the code, and worth pinning: the caller decides
        // whether there is anything to say, and a caller with nothing to say
        // must not leave a stray panel on screen for the next reader to wonder
        // about.
        let mut quads = Vec::new();
        draw_readout(&window(), "", &mut quads);
        assert_eq!(quads.len(), 1, "the backdrop, and no glyphs");
    }
}
