//! Vectors, angles, and the engine's own trigonometry.
//!
//! Key types: `Radians`, `Vec2`, `Vec3`, `Mat4`; `sin_cos`, `atan2`, `rotate`.
//! Depends on: `glam` (scalar-math). Must never depend on: the standard
//! library's trigonometry — see below.
//! INVARIANT: every function here is built from IEEE add, subtract, multiply,
//! divide and round, which are bit-exact on every platform Rust targets. The
//! same angle gives the same sine on glibc, MSVC, and wasm, which
//! `f32::sin` does not promise (ADR-0009).

pub use glam::{Vec2, Vec3, Vec4};

use crate::resource::Resource;

/// An angle, in radians.
///
/// Degrees appear nowhere in the engine; `Radians::from_degrees` exists for
/// humans typing a number they can picture.
///
/// ```
/// use jidousha_core::math::Radians;
///
/// let quarter_turn = Radians::from_degrees(90.0);
/// assert!((quarter_turn.as_f32() - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Radians(pub f32);

impl Radians {
    /// No rotation.
    pub const ZERO: Radians = Radians(0.0);

    /// A full turn.
    pub const TAU: Radians = Radians(core::f32::consts::TAU);

    /// Convert from degrees, for humans.
    #[must_use]
    pub fn from_degrees(degrees: f32) -> Self {
        Radians(degrees * (core::f32::consts::PI / 180.0))
    }

    /// The angle in degrees, for humans and for debug output.
    #[must_use]
    pub fn to_degrees(self) -> f32 {
        self.0 * (180.0 / core::f32::consts::PI)
    }

    /// The underlying value.
    #[must_use]
    pub fn as_f32(self) -> f32 {
        self.0
    }
}

impl Resource for Radians {}

/// π/2 in f64 — the quadrant the polynomials are written for.
const FRAC_PI_2: f64 = core::f64::consts::FRAC_PI_2;
/// 2/π, for finding which quadrant an angle falls in.
const FRAC_2_PI: f64 = core::f64::consts::FRAC_2_PI;

/// The sine and cosine of `angle`, both at once.
///
/// Deterministic by construction: polynomial evaluation in `f64` over IEEE
/// arithmetic, rounded once to `f32`. Accurate to about 1e-7 — far inside the
/// 1e-6 the engine promises, and identical on every platform (ADR-0009).
///
/// ```
/// use jidousha_core::math::{Radians, sin_cos};
///
/// let (sine, cosine) = sin_cos(Radians(0.0));
/// assert_eq!((sine, cosine), (0.0, 1.0));
///
/// // Same input, same bits, every run and every platform.
/// assert_eq!(sin_cos(Radians(1.234)), sin_cos(Radians(1.234)));
/// ```
#[must_use]
pub fn sin_cos(angle: Radians) -> (f32, f32) {
    let (sine, cosine) = sin_cos_f64(f64::from(angle.0));
    (sine as f32, cosine as f32)
}

/// The angle of the vector `(x, y)`, measured from the +X axis.
///
/// Positive rotation is clockwise on screen, because +Y points down
/// (conventions, ADR-0010). The result is in `-π..=π`; `atan2(0, 0)` is zero.
///
/// ```
/// use jidousha_core::math::{Radians, atan2};
///
/// // Straight down the +Y axis is a quarter turn clockwise.
/// let angle = atan2(1.0, 0.0);
/// assert!((angle.as_f32() - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
/// ```
#[must_use]
pub fn atan2(y: f32, x: f32) -> Radians {
    Radians(atan2_f64(f64::from(y), f64::from(x)) as f32)
}

/// Turn `vector` by `angle`.
///
/// Built on [`sin_cos`], so it inherits the same determinism. This is the
/// engine's replacement for glam's angle constructors, which route through the
/// platform's libm (ADR-0009).
///
/// ```
/// use jidousha_core::math::{Radians, Vec2, rotate};
///
/// let turned = rotate(Vec2::new(1.0, 0.0), Radians::from_degrees(90.0));
/// assert!((turned.x - 0.0).abs() < 1e-6);
/// assert!((turned.y - 1.0).abs() < 1e-6);
/// ```
#[must_use]
pub fn rotate(vector: Vec2, angle: Radians) -> Vec2 {
    let (sine, cosine) = sin_cos(angle);
    Vec2::new(
        vector.x * cosine - vector.y * sine,
        vector.x * sine + vector.y * cosine,
    )
}

/// Sine and cosine in f64, with the argument reduced into one quadrant.
fn sin_cos_f64(x: f64) -> (f64, f64) {
    // Which quadrant, and how far into it. `round` is an exact IEEE operation,
    // so the split is identical everywhere.
    let quadrant = (x * FRAC_2_PI).round();
    let remainder = x - quadrant * FRAC_PI_2;
    let (sine, cosine) = (sin_poly(remainder), cos_poly(remainder));
    // Saturating cast, then a wrap into 0..4: huge angles lose precision in the
    // reduction above, but never correctness of the quadrant arithmetic.
    match (quadrant as i64).rem_euclid(4) {
        0 => (sine, cosine),
        1 => (cosine, -sine),
        2 => (-sine, -cosine),
        _ => (-cosine, sine),
    }
}

/// Sine on `[-π/4, π/4]`, by Taylor series through x^11.
///
/// The next term would contribute less than 1e-17 at the edge of the interval,
/// which f64 cannot represent next to the leading term.
fn sin_poly(x: f64) -> f64 {
    let square = x * x;
    x * (1.0
        + square
            * (-1.0 / 6.0
                + square * (1.0 / 120.0 + square * (-1.0 / 5040.0 + square * (1.0 / 362_880.0)))))
}

/// Cosine on `[-π/4, π/4]`, by Taylor series through x^10.
fn cos_poly(x: f64) -> f64 {
    let square = x * x;
    1.0 + square
        * (-0.5 + square * (1.0 / 24.0 + square * (-1.0 / 720.0 + square * (1.0 / 40320.0))))
}

/// `atan2` in f64.
fn atan2_f64(y: f64, x: f64) -> f64 {
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }
    if x.abs() >= y.abs() {
        let base = atan_poly(y / x);
        if x < 0.0 {
            if y < 0.0 {
                base - core::f64::consts::PI
            } else {
                base + core::f64::consts::PI
            }
        } else {
            base
        }
    } else {
        // Beyond the diagonal, atan(y/x) = π/2 - atan(x/y), which keeps the
        // polynomial's argument inside the interval it is accurate on.
        let base = FRAC_PI_2 - atan_poly(x / y);
        if y < 0.0 {
            base - core::f64::consts::PI
        } else {
            base
        }
    }
}

/// `tan(π/8)`: beyond this the series below is reduced once more.
const TAN_PI_8: f64 = 0.414_213_562_373_095_1;

/// Arctangent for `|t| <= 1`.
fn atan_poly(t: f64) -> f64 {
    if t.abs() > TAN_PI_8 {
        // atan(t) = π/4 + atan((t-1)/(t+1)), which shrinks the argument to
        // within ±tan(π/8) — where the series below converges quickly.
        let sign = if t < 0.0 { -1.0 } else { 1.0 };
        let magnitude = t.abs();
        let reduced = (magnitude - 1.0) / (magnitude + 1.0);
        return sign * (core::f64::consts::FRAC_PI_4 + atan_series(reduced));
    }
    atan_series(t)
}

/// Arctangent by its Taylor series, for `|t| <= tan(π/8)`.
///
/// The last term kept contributes about 1e-11 at the edge of that interval.
fn atan_series(t: f64) -> f64 {
    let square = t * t;
    t * (1.0
        + square
            * (-1.0 / 3.0
                + square
                    * (1.0 / 5.0
                        + square
                            * (-1.0 / 7.0
                                + square
                                    * (1.0 / 9.0
                                        + square * (-1.0 / 11.0 + square * (1.0 / 13.0)))))))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference values come from the standard library's trigonometry, which
    /// the engine is banned from using anywhere else (ADR-0009): the ban exists
    /// because libm varies by platform, and a test comparing against it on one
    /// platform is exactly the right use for it.
    #[allow(clippy::disallowed_methods)]
    fn reference_sin_cos(x: f32) -> (f32, f32) {
        (x.sin(), x.cos())
    }

    #[allow(clippy::disallowed_methods)]
    fn reference_atan2(y: f32, x: f32) -> f32 {
        y.atan2(x)
    }

    #[test]
    fn sin_and_cos_match_the_standard_library_to_a_millionth() {
        let mut worst = 0.0f32;
        for step in -2000..=2000 {
            let angle = step as f32 * 0.01;
            let (sine, cosine) = sin_cos(Radians(angle));
            let (want_sine, want_cosine) = reference_sin_cos(angle);
            worst = worst.max((sine - want_sine).abs());
            worst = worst.max((cosine - want_cosine).abs());
        }
        assert!(worst < 1e-6, "worst error {worst}");
    }

    #[test]
    fn atan2_matches_the_standard_library_to_a_millionth() {
        let mut worst = 0.0f32;
        for y_step in -20..=20 {
            for x_step in -20..=20 {
                let (y, x) = (y_step as f32 * 0.5, x_step as f32 * 0.5);
                if x == 0.0 && y == 0.0 {
                    continue;
                }
                let got = atan2(y, x).as_f32();
                let want = reference_atan2(y, x);
                worst = worst.max((got - want).abs());
            }
        }
        assert!(worst < 1e-6, "worst error {worst}");
    }

    #[test]
    fn the_quadrants_come_out_where_they_should() {
        let quarter = core::f32::consts::FRAC_PI_2;
        for (angle, want) in [
            (0.0, (0.0, 1.0)),
            (quarter, (1.0, 0.0)),
            (2.0 * quarter, (0.0, -1.0)),
            (3.0 * quarter, (-1.0, 0.0)),
        ] {
            let (sine, cosine) = sin_cos(Radians(angle));
            assert!((sine - want.0).abs() < 1e-6, "sin({angle}) = {sine}");
            assert!((cosine - want.1).abs() < 1e-6, "cos({angle}) = {cosine}");
        }
    }

    #[test]
    fn large_angles_stay_bounded_and_finite() {
        for angle in [1e3, -1e3, 1e6, -1e6] {
            let (sine, cosine) = sin_cos(Radians(angle));
            assert!(sine.abs() <= 1.0 + 1e-6, "sin({angle}) = {sine}");
            assert!(cosine.abs() <= 1.0 + 1e-6, "cos({angle}) = {cosine}");
        }
    }

    #[test]
    fn the_trig_bit_pattern_is_locked_across_platforms() {
        // The determinism claim in ADR-0009 is "bit-identical everywhere by
        // construction". These are the bits x86-64 produces; wasm and MSVC must
        // agree, since every operation involved is IEEE-exact. A change here
        // means the arithmetic changed, which is a contract break, not a
        // tuning detail.
        let samples: Vec<(u32, u32)> = [0.0f32, 0.5, 1.0, 1.234, -2.5, 3.5, 100.0]
            .into_iter()
            .map(|angle| {
                let (sine, cosine) = sin_cos(Radians(angle));
                (sine.to_bits(), cosine.to_bits())
            })
            .collect();
        assert_eq!(
            samples,
            [
                (0x00000000, 0x3f800000),
                (0x3ef57744, 0x3f60a940),
                (0x3f576aa4, 0x3f0a5140),
                (0x3f719e12, 0x3ea932ba),
                (0xbf193578, 0xbf4d17c0),
                (0xbeb399dc, 0xbf6fbba0),
                (0xbf01a12e, 0x3f5cc0ee),
            ]
        );
    }

    #[test]
    fn rotating_by_a_full_turn_returns_the_vector() {
        let start = Vec2::new(3.0, -4.0);
        let turned = rotate(start, Radians::TAU);
        assert!((turned - start).length() < 1e-5, "{turned:?}");
    }

    #[test]
    fn rotation_turns_clockwise_on_screen() {
        // +Y is down, so a positive angle takes +X toward +Y (ADR-0010).
        let turned = rotate(Vec2::new(1.0, 0.0), Radians::from_degrees(90.0));
        assert!(turned.y > 0.9, "{turned:?}");
    }

    #[test]
    fn degrees_and_radians_round_trip() {
        for degrees in [0.0, 45.0, 90.0, 180.0, -270.0] {
            let angle = Radians::from_degrees(degrees);
            assert!((angle.to_degrees() - degrees).abs() < 1e-3, "{degrees}");
        }
    }

    #[test]
    fn atan2_of_the_origin_is_zero_rather_than_undefined() {
        assert_eq!(atan2(0.0, 0.0), Radians::ZERO);
    }
}
