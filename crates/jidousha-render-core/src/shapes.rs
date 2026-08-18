//! Rectangles, lines and circles, expanded into the one thing the engine draws.
//!
//! Key types: `rect_quad`, `line_quad`, `circle_quads`.
//! Depends on: `jidousha-core`.
//! INVARIANT: everything here produces `Quad`s sampling [`TextureId::WHITE`], so
//! a debug rectangle goes through the same sort, the same batch, and the same
//! pipeline as a sprite. There is no debug renderer and no second path
//! (renderer.md §2) — which is also why a shape and a sprite at the same depth
//! interleave correctly instead of one class always winning.

use jidousha_core::math::{Radians, Vec2, sin_cos};
use jidousha_core::{Color, Depth, Quad, Rect, TextureId};

/// How many straight edges a circle is made of.
///
/// DELIBERATE: fixed, not scaled by radius. A radius-dependent count would make
/// the transcript — and every golden image — change when a circle grows by a
/// pixel, and the whole verification story rests on identical submissions
/// producing identical output (renderer.md §2, §9). Thirty-two is smooth enough
/// that a ball in a prototype reads as round at any size a prototype uses.
/// Revisit with a game that needs a smoother one, and the shape of that change
/// is an argument, not a different constant.
///
/// The resulting **sixteen quads** (two segments each) are now a published number:
/// `Submit::circle`'s summary says so, Concepts states the per-verb quad budget,
/// and *Testing your game* has an assertion that relies on the fan being
/// inscribed and sharing the centre (ADR-0020). Moving this constant is therefore
/// a documentation change and needs a superseding ADR, not just a smoother circle.
const CIRCLE_SEGMENTS: u32 = 32;

/// Untextured quads sample the white texel, so any coordinate does.
///
/// Zero rather than the unit rectangle: it is one texel, every coordinate names
/// it, and a transcript reads better without four different numbers that all
/// mean the same thing.
const FLAT: [Vec2; 4] = [Vec2::ZERO; 4];

/// The quad for a filled, axis-aligned rectangle.
///
/// Corners wind top-left, top-right, bottom-right, bottom-left, as every quad
/// in the engine does. Y is down, so `min` is the top-left (ADR-0010).
pub(crate) fn rect_quad(rect: Rect, color: Color, depth: Depth) -> Quad {
    Quad {
        corners: [
            rect.min,
            Vec2::new(rect.max.x, rect.min.y),
            rect.max,
            Vec2::new(rect.min.x, rect.max.y),
        ],
        uvs: FLAT,
        tint: color,
        texture: TextureId::WHITE,
        depth,
    }
}

/// The quad for a thick line segment: a rectangle rotated onto the segment.
///
/// A zero-length segment draws a square dot of `thickness` rather than nothing.
/// Nothing would be a silent no-op, and "my line vanished when the two points
/// met" is a bug report nobody enjoys writing — one that is *visible* is the
/// engine saying what happened (core.md §9's no-silent-failure rule).
pub(crate) fn line_quad(from: Vec2, to: Vec2, thickness: f32, color: Color, depth: Depth) -> Quad {
    let along = to - from;
    let length = along.length();
    // Below this the direction is noise rather than a direction. Pointing along
    // +X is arbitrary and deliberate: a dot has no orientation to get wrong.
    let direction = if length > 1e-6 {
        along / length
    } else {
        Vec2::X
    };
    // `perp` is (-y, x): a quarter turn, which on a Y-down screen is clockwise
    // (ADR-0010). Which side is which does not matter here — the offset is
    // applied both ways — but the winding does, and this order gives the same
    // top-left-first cycle every other quad has.
    let across = Vec2::new(-direction.y, direction.x) * (thickness * 0.5);
    let (start, end) = if length > 1e-6 {
        (from, to)
    } else {
        (
            from - direction * (thickness * 0.5),
            from + direction * (thickness * 0.5),
        )
    };
    Quad {
        corners: [start - across, end - across, end + across, start + across],
        uvs: FLAT,
        tint: color,
        texture: TextureId::WHITE,
        depth,
    }
}

/// A filled circle, as a fan of quads around its center.
///
/// Each quad is the center and three points on the rim, so it covers two
/// segments as two triangles — half as many quads as a triangle fan would need,
/// and every one of them convex, which is what keeps `FrameRecord::covering`
/// able to answer "is this point on the ball?" exactly (renderer.md §9).
///
/// # Panics
///
/// In debug builds, if `radius` is negative — a circle turned inside out is a
/// bug in whatever computed it, not a shape to draw.
pub(crate) fn circle_quads(
    center: Vec2,
    radius: f32,
    color: Color,
    depth: Depth,
    out: &mut Vec<Quad>,
) {
    debug_assert!(
        radius >= 0.0,
        "[jidousha] a circle was asked for with a negative radius: {radius}\n  \
         likely cause: a radius computed by subtraction, or a scale that went below zero\n  \
         fix: clamp it at the source; the renderer cannot draw a circle inside out \
         (renderer.md §10)"
    );
    // A circle of no size covers nothing. That is an answer, not a failure —
    // unlike a negative radius, which the assertion above catches. Written this
    // way round because a NaN radius is neither, and must draw nothing rather
    // than thirty-two wedges of NaN that `plan_frame` would then refuse to sort.
    if !matches!(radius.partial_cmp(&0.0), Some(core::cmp::Ordering::Greater)) {
        return;
    }
    let rim = |step: u32| {
        let turn = Radians(core::f32::consts::TAU * step as f32 / CIRCLE_SEGMENTS as f32);
        // The engine's own trigonometry: bit-identical on every platform, which
        // is what lets a transcript of a circle be compared at all (ADR-0009).
        let (sine, cosine) = sin_cos(turn);
        center + Vec2::new(cosine, sine) * radius
    };
    for pair in 0..CIRCLE_SEGMENTS / 2 {
        let first = pair * 2;
        out.push(Quad {
            corners: [center, rim(first), rim(first + 1), rim(first + 2)],
            uvs: FLAT,
            tint: color,
            texture: TextureId::WHITE,
            depth,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rectangle_covers_exactly_what_it_was_given() {
        let quad = rect_quad(
            Rect::from_min_size(Vec2::new(1.0, 2.0), Vec2::new(4.0, 3.0)),
            Color::RED,
            Depth::default(),
        );
        // All four, in order. Checking only the opposite pair leaves the other
        // two free to swap, which is a quad that still *renders* the same
        // rectangle — culling is off — while quietly breaking the winding every
        // other quad in the engine keeps. Mutation testing found exactly that.
        assert_eq!(quad.corners[0], Vec2::new(1.0, 2.0), "top-left");
        assert_eq!(quad.corners[1], Vec2::new(5.0, 2.0), "top-right");
        assert_eq!(quad.corners[2], Vec2::new(5.0, 5.0), "bottom-right");
        assert_eq!(quad.corners[3], Vec2::new(1.0, 5.0), "bottom-left");
        assert_eq!(quad.texture, TextureId::WHITE);
        assert_eq!(quad.tint, Color::RED);
    }

    #[test]
    fn a_horizontal_line_is_as_thick_as_it_was_asked_to_be() {
        let quad = line_quad(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            2.0,
            Color::WHITE,
            Depth::default(),
        );
        let ys: Vec<f32> = quad.corners.iter().map(|corner| corner.y).collect();
        // Top-left first, and top is the *smaller* Y (ADR-0010).
        assert_eq!(ys, vec![-1.0, -1.0, 1.0, 1.0], "one unit either side");
        let xs: Vec<f32> = quad.corners.iter().map(|corner| corner.x).collect();
        assert_eq!(xs, vec![0.0, 10.0, 10.0, 0.0]);
    }

    #[test]
    fn a_diagonal_line_keeps_its_thickness() {
        // The failure this rules out: offsetting by the un-normalized direction,
        // which makes a long line fat and a short one thin.
        let quad = line_quad(
            Vec2::ZERO,
            Vec2::new(30.0, 40.0),
            2.0,
            Color::WHITE,
            Depth::default(),
        );
        let width = (quad.corners[3] - quad.corners[0]).length();
        assert!((width - 2.0).abs() < 1e-5, "{width}");
    }

    #[test]
    fn a_line_of_no_length_still_draws_something() {
        // Silence here would be a game losing a marker the moment two points
        // met, with nothing on screen to say why.
        let quad = line_quad(
            Vec2::new(3.0, 3.0),
            Vec2::new(3.0, 3.0),
            2.0,
            Color::WHITE,
            Depth::default(),
        );
        let area = (quad.corners[1] - quad.corners[0]).length()
            * (quad.corners[3] - quad.corners[0]).length();
        assert!((area - 4.0).abs() < 1e-5, "a 2x2 dot, not nothing: {area}");
    }

    #[test]
    fn a_circle_is_made_of_convex_quads_that_meet_at_the_center() {
        let mut quads = Vec::new();
        circle_quads(
            Vec2::new(5.0, 5.0),
            2.0,
            Color::BLUE,
            Depth::default(),
            &mut quads,
        );
        assert_eq!(quads.len() as u32, CIRCLE_SEGMENTS / 2);
        for quad in &quads {
            assert_eq!(
                quad.corners[0],
                Vec2::new(5.0, 5.0),
                "a fan from the center"
            );
            for corner in &quad.corners[1..] {
                let distance = (*corner - Vec2::new(5.0, 5.0)).length();
                assert!((distance - 2.0).abs() < 1e-4, "{distance} from the center");
            }
        }
    }

    #[test]
    fn a_circle_closes() {
        // The last quad's far corner must be the first quad's near one, or the
        // circle has a wedge missing that nothing else would notice.
        let mut quads = Vec::new();
        circle_quads(Vec2::ZERO, 1.0, Color::WHITE, Depth::default(), &mut quads);
        let first = quads[0].corners[1];
        let last = quads[quads.len() - 1].corners[3];
        assert!((first - last).length() < 1e-5, "{first:?} then {last:?}");
    }

    #[test]
    fn a_circle_of_no_size_draws_nothing() {
        let mut quads = Vec::new();
        circle_quads(Vec2::ZERO, 0.0, Color::WHITE, Depth::default(), &mut quads);
        assert!(quads.is_empty());
    }

    #[test]
    fn every_shape_is_untextured_and_carries_its_depth() {
        let depth = Depth { layer: 3, z: 1.5 };
        let mut quads = vec![
            rect_quad(Rect::UNIT, Color::WHITE, depth),
            line_quad(Vec2::ZERO, Vec2::X, 1.0, Color::WHITE, depth),
        ];
        circle_quads(Vec2::ZERO, 1.0, Color::WHITE, depth, &mut quads);
        for quad in &quads {
            assert_eq!(quad.texture, TextureId::WHITE);
            assert_eq!(quad.depth, depth);
        }
    }
}
