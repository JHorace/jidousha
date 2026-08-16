//! sRGB in, linear out — the conversion the conventions promise happens
//! "inside the render backend, invisibly".
//!
//! Key types: `linear`.
//! Depends on: `jidousha-core`. Must never depend on: `wgpu` — this is
//! arithmetic, and keeping it separate is what lets it be tested on a machine
//! with no GPU.
//! INVARIANT: one implementation, used by both the clear color and every vertex
//! color. The obvious alternative — converting vertex colors in WGSL — would
//! put the same curve in two languages, and two curves that must agree and
//! cannot be compared is exactly the shape of a bug that ships.

use jidousha_core::Color;

/// A color as the GPU wants it: linear RGB, straight alpha.
///
/// The engine's `Color` is sRGB-encoded, because that is what a person means by
/// "half grey" (conventions). Blending, interpolation across a triangle, and
/// the surface's own `-srgb` encoding all assume linear, so the conversion has
/// to happen somewhere; here is the somewhere.
///
/// PERF: a `powf` per color channel, six vertices per quad. Measured against
/// nothing so far — at prototype scale a frame is hundreds of quads, not
/// hundreds of thousands. If it ever matters, the fix is a small lookup table
/// rather than moving the curve into the shader.
///
/// DELIBERATE: `powf` is a libm call, which ADR-0009 bans from the determinism
/// path. This is not on it — the result goes to the GPU and never back into
/// simulation, and nothing recorded or replayed passes through here. Golden
/// images (R4) compare with a tolerance that a last-bit difference between
/// platforms sits far inside.
#[must_use]
pub(crate) fn linear(color: Color) -> [f32; 4] {
    [
        linear_channel(color.r),
        linear_channel(color.g),
        linear_channel(color.b),
        // Alpha is not sRGB-encoded. It is a coverage fraction, not a
        // brightness, and putting it through the curve would make everything
        // half-transparent the wrong amount.
        color.a,
    ]
}

/// One channel of the sRGB transfer function, inverted.
///
/// The piecewise definition from the sRGB specification, not the `2.2` gamma
/// approximation: the linear segment near black is where the approximation is
/// most wrong, and near black is where banding shows.
fn linear_channel(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_and_white_are_fixed_points() {
        // Whatever the curve does in between, the ends must not move, or every
        // clear color and every untinted sprite would be subtly off.
        assert_eq!(linear(Color::BLACK), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(linear(Color::WHITE), [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn mid_grey_darkens_the_way_srgb_says() {
        // The number this whole module exists for: 0.5 in sRGB is about 0.214
        // in linear light, not 0.5. Passing it through unconverted is what R1's
        // clear did, and it is why a grey window looked washed out.
        let [red, ..] = linear(Color::rgb(0.5, 0.5, 0.5));
        assert!((red - 0.2140).abs() < 1e-3, "{red}");
    }

    #[test]
    fn alpha_passes_through_untouched() {
        // Alpha is coverage, not brightness. Half-transparent means half.
        let [.., alpha] = linear(Color::rgba(1.0, 1.0, 1.0, 0.5));
        assert_eq!(alpha, 0.5);
    }

    #[test]
    fn the_curve_never_goes_backwards() {
        // Monotonic, including across the join between the two pieces at
        // 0.04045 — a discontinuity there would show as a band in any gradient.
        let mut previous = -1.0;
        for step in 0..=1000 {
            let value = linear_channel(step as f32 / 1000.0);
            assert!(value > previous, "not increasing at {step}");
            previous = value;
        }
    }

    #[test]
    fn the_two_pieces_meet() {
        let below = linear_channel(0.04045);
        let above = linear_channel(0.040450002);
        assert!((below - above).abs() < 1e-6, "{below} then {above}");
    }
}
