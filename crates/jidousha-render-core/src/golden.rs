//! Comparing a captured frame against a reference picture (renderer.md §9,
//! tier 2).
//!
//! Key types: `Tolerance`, `Comparison`, `compare`, `diff_image`, `encode_png`,
//! `decode_png`.
//! Depends on: `backend`, `jidousha-assets`. Must never depend on: `wgpu` — this
//! compares two `RawImage`s and does not care which backend made either, which
//! is what lets the ash port reuse every golden reference unchanged.
//! INVARIANT: comparison is **tolerant by construction and never by accident**.
//! Every way of passing is a number stated in a [`Tolerance`], so "this test is
//! looser than it looks" is visible at the callsite rather than buried here.
//!
//! Why tolerance at all: GPU rasterization differs slightly between drivers —
//! edge pixels land on different sides of a boundary, and filtering rounds
//! differently. Exact match across machines is a flake factory. The *exact*
//! tier is the submission transcript (§9 tier 1), which has no pixels to
//! disagree about; this tier exists to keep the backend honest, and a backend
//! that drew the right picture one pixel differently is honest.

use core::fmt;

use jidousha_assets::{AssetError, TextureData};

use crate::backend::RawImage;
use jidousha_core::PhysicalSize;

/// How different two pictures may be and still count as the same picture.
///
/// Both numbers have to be exceeded for a comparison to fail, and they measure
/// different failures: `per_channel` catches "the whole image is slightly
/// wrong" (a colour-space change, a mis-set blend mode), `differing_fraction`
/// catches "a few pixels are completely wrong" (an edge landing one pixel over)
/// without letting a genuinely different picture through.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    /// How far one channel of one pixel may be from the reference, 0–255.
    ///
    /// A pixel within this on every channel is not counted as differing at all.
    pub per_channel: u8,
    /// What fraction of pixels may differ by more than `per_channel`, 0.0–1.0.
    pub differing_fraction: f32,
}

impl Tolerance {
    /// What a golden test should want unless it has a reason otherwise.
    ///
    /// Two levels of colour is below anything an eye or a screenshot diff will
    /// show, and covers the rounding a driver does in filtering. Half a percent
    /// of pixels covers the outline of a shape landing one pixel over on a
    /// different rasterizer, and is far under what any real change costs — a
    /// sprite that moved, a colour that changed, or a batch that stopped
    /// drawing all move percentages, not fractions of one.
    pub const CLOSE_ENOUGH: Self = Self {
        per_channel: 2,
        differing_fraction: 0.005,
    };

    /// Nothing may differ at all.
    ///
    /// For comparing two captures from the *same* machine — the stability
    /// check, where any difference is a real one.
    pub const EXACT: Self = Self {
        per_channel: 0,
        differing_fraction: 0.0,
    };
}

/// What comparing two pictures found.
#[derive(Clone, Debug, PartialEq)]
pub struct Comparison {
    /// Whether the two count as the same picture under the tolerance used.
    pub matched: bool,
    /// Pixels differing by more than the tolerance allows per channel.
    pub differing: usize,
    /// Pixels compared. Zero when the sizes disagree.
    pub total: usize,
    /// The largest single-channel difference anywhere.
    pub worst: u8,
    /// Where `worst` was found, in pixels from the top-left.
    pub worst_at: Option<(u32, u32)>,
    /// Set when the two are not even the same shape, which no tolerance covers.
    pub size_mismatch: Option<(PhysicalSize, PhysicalSize)>,
}

impl Comparison {
    /// The fraction of compared pixels that differed, 0.0 when nothing was
    /// compared.
    #[must_use]
    pub fn differing_fraction(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        self.differing as f32 / self.total as f32
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some((expected, actual)) = self.size_mismatch {
            return write!(
                formatter,
                "the reference is {}x{} and the capture is {}x{}",
                expected.width, expected.height, actual.width, actual.height
            );
        }
        let percent = f64::from(self.differing_fraction()) * 100.0;
        write!(
            formatter,
            "{} of {} pixels differ ({percent:.3}%), worst channel delta {}",
            self.differing, self.total, self.worst
        )?;
        if let Some((x, y)) = self.worst_at {
            write!(formatter, " at ({x}, {y})")?;
        }
        Ok(())
    }
}

/// Compare a capture against a reference.
///
/// Two pictures of different sizes never match: no per-pixel tolerance can
/// describe that difference, and comparing the overlap would quietly pass a
/// capture taken at the wrong resolution.
#[must_use]
pub fn compare(expected: &RawImage, actual: &RawImage, tolerance: Tolerance) -> Comparison {
    if expected.size != actual.size {
        return Comparison {
            matched: false,
            differing: 0,
            total: 0,
            worst: 0,
            worst_at: None,
            size_mismatch: Some((expected.size, actual.size)),
        };
    }

    let mut differing = 0usize;
    let mut worst = 0u8;
    let mut worst_at = None;
    let width = expected.size.width.max(1);
    for (index, (left, right)) in expected
        .rgba
        .chunks_exact(4)
        .zip(actual.rgba.chunks_exact(4))
        .enumerate()
    {
        let mut pixel_worst = 0u8;
        for channel in 0..4 {
            pixel_worst = pixel_worst.max(left[channel].abs_diff(right[channel]));
        }
        if pixel_worst > worst {
            worst = pixel_worst;
            let index = index as u32;
            worst_at = Some((index % width, index / width));
        }
        if pixel_worst > tolerance.per_channel {
            differing += 1;
        }
    }

    let total = expected.rgba.len() / 4;
    let fraction = if total == 0 {
        0.0
    } else {
        differing as f32 / total as f32
    };
    Comparison {
        matched: fraction <= tolerance.differing_fraction,
        differing,
        total,
        worst,
        worst_at,
        size_mismatch: None,
    }
}

/// A picture of where two captures disagree, for a human or an agent to look at.
///
/// Differing pixels are drawn magenta and matching ones are darkened, so the
/// disagreement is the only thing in the image that is bright — the same reason
/// the missing-texture placeholder is magenta (renderer.md §5). Returns `None`
/// when the two are not the same size, because there is no image to draw.
#[must_use]
pub fn diff_image(
    expected: &RawImage,
    actual: &RawImage,
    tolerance: Tolerance,
) -> Option<RawImage> {
    if expected.size != actual.size {
        return None;
    }
    let mut rgba = Vec::with_capacity(expected.rgba.len());
    for (left, right) in expected
        .rgba
        .chunks_exact(4)
        .zip(actual.rgba.chunks_exact(4))
    {
        let worst = (0..4)
            .map(|channel| left[channel].abs_diff(right[channel]))
            .max()
            .unwrap_or(0);
        if worst > tolerance.per_channel {
            rgba.extend_from_slice(&[255, 0, 255, 255]);
        } else {
            rgba.extend_from_slice(&[left[0] / 4, left[1] / 4, left[2] / 4, 255]);
        }
    }
    Some(RawImage {
        size: expected.size,
        rgba,
    })
}

/// A captured frame as PNG bytes.
///
/// # Panics
///
/// If the image's texels do not match its size, or it has no pixels — see
/// [`jidousha_assets::encode_png`].
#[must_use]
pub fn encode_png(image: &RawImage) -> Vec<u8> {
    jidousha_assets::encode_png(&TextureData {
        width: image.size.width,
        height: image.size.height,
        rgba: image.rgba.clone(),
    })
}

/// A reference picture read back from PNG bytes.
///
/// The same decoder every texture goes through, so a reference and the art it
/// is a picture of cannot disagree about what a PNG means (assets.md §3).
///
/// # Errors
///
/// If the bytes are not a readable PNG.
pub fn decode_png(bytes: &[u8]) -> Result<RawImage, AssetError> {
    let texture = jidousha_assets::decode_png(bytes)?;
    Ok(RawImage {
        size: PhysicalSize::new(texture.width, texture.height),
        rgba: texture.rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, color: [u8; 4]) -> RawImage {
        RawImage {
            size: PhysicalSize::new(width, height),
            rgba: color.repeat((width * height) as usize),
        }
    }

    /// The same image with one pixel changed by `delta` on the red channel.
    fn nudged(base: &RawImage, index: usize, delta: u8) -> RawImage {
        let mut image = base.clone();
        image.rgba[index * 4] = image.rgba[index * 4].wrapping_add(delta);
        image
    }

    #[test]
    fn a_picture_matches_itself_exactly() {
        let image = solid(4, 4, [10, 20, 30, 255]);
        let result = compare(&image, &image, Tolerance::EXACT);
        assert!(result.matched, "{result}");
        assert_eq!(result.differing, 0);
        assert_eq!(result.worst, 0);
        assert_eq!(result.total, 16, "every pixel was compared");
    }

    #[test]
    fn nothing_at_all_may_differ_under_the_exact_tolerance() {
        // What `EXACT` is for: comparing one machine against itself, where any
        // difference is a real one. A widened `EXACT` would let the
        // render-twice stability check pass against a backend that drew
        // slightly differently each frame — and that check is what makes a
        // blessed reference reproducible in the first place.
        let base = solid(4, 4, [10, 20, 30, 255]);
        let result = compare(&base, &nudged(&base, 0, 1), Tolerance::EXACT);
        assert!(!result.matched, "one level is a difference: {result}");
        assert_eq!(result.differing, 1);
        assert_eq!(Tolerance::EXACT.per_channel, 0);
        assert_eq!(Tolerance::EXACT.differing_fraction, 0.0);
    }

    #[test]
    fn a_difference_within_the_channel_tolerance_is_not_counted() {
        // The rounding one driver does and another does not.
        let base = solid(4, 4, [10, 20, 30, 255]);
        let result = compare(&base, &nudged(&base, 0, 2), Tolerance::CLOSE_ENOUGH);
        assert!(result.matched, "{result}");
        assert_eq!(result.differing, 0, "within tolerance is not a difference");
        assert_eq!(result.worst, 2, "but it is still reported");
    }

    #[test]
    fn a_difference_past_the_channel_tolerance_is_counted() {
        // One level past `per_channel`, which must land on the other side.
        let base = solid(4, 4, [10, 20, 30, 255]);
        let result = compare(&base, &nudged(&base, 0, 3), Tolerance::CLOSE_ENOUGH);
        assert_eq!(result.differing, 1);
        assert_eq!(result.worst, 3);
    }

    #[test]
    fn a_few_differing_pixels_still_match_and_many_do_not() {
        // The other half of the tolerance: an edge landing one pixel over is
        // fine, and a picture that is genuinely different is not. 1 in 400 is
        // under half a percent; 4 in 400 is over it.
        let base = solid(20, 20, [10, 20, 30, 255]);
        let mut few = base.clone();
        few.rgba[0] = 255;
        assert!(compare(&base, &few, Tolerance::CLOSE_ENOUGH).matched);

        let mut many = base.clone();
        for index in 0..4 {
            many.rgba[index * 4] = 255;
        }
        let result = compare(&base, &many, Tolerance::CLOSE_ENOUGH);
        assert!(!result.matched, "{result}");
        assert_eq!(result.differing, 4);
    }

    #[test]
    fn the_worst_pixel_is_reported_where_it_actually_is() {
        // A failure message that points at the wrong pixel sends the reader to
        // the wrong part of the picture, which is worse than saying nothing.
        let base = solid(8, 4, [0, 0, 0, 255]);
        // Index 11 in an 8-wide image is (3, 1).
        let result = compare(&base, &nudged(&base, 11, 200), Tolerance::EXACT);
        assert_eq!(result.worst, 200);
        assert_eq!(result.worst_at, Some((3, 1)));
        assert!(result.to_string().contains("at (3, 1)"), "{result}");
    }

    #[test]
    fn two_pictures_of_different_sizes_never_match() {
        // No per-pixel tolerance describes this, and comparing the overlap
        // would quietly pass a capture taken at the wrong resolution.
        let result = compare(
            &solid(4, 4, [0, 0, 0, 255]),
            &solid(4, 5, [0, 0, 0, 255]),
            Tolerance {
                per_channel: 255,
                differing_fraction: 1.0,
            },
        );
        assert!(!result.matched, "not even a total tolerance covers it");
        assert_eq!(
            result.size_mismatch,
            Some((PhysicalSize::new(4, 4), PhysicalSize::new(4, 5)))
        );
        assert!(result.to_string().contains("4x4"), "{result}");
        assert!(result.to_string().contains("4x5"), "{result}");
    }

    #[test]
    fn alpha_counts_as_a_channel() {
        // The engine draws with alpha (renderer.md §7). A comparison that
        // looked at RGB only would pass a frame whose blending stopped working
        // wherever the colours happened to agree.
        let base = solid(4, 4, [10, 20, 30, 255]);
        let mut transparent = base.clone();
        for pixel in transparent.rgba.chunks_exact_mut(4) {
            pixel[3] = 0;
        }
        let result = compare(&base, &transparent, Tolerance::CLOSE_ENOUGH);
        assert!(!result.matched, "{result}");
        assert_eq!(result.differing, 16);
    }

    #[test]
    fn the_diff_image_marks_exactly_the_pixels_that_differ() {
        let base = solid(2, 2, [40, 80, 120, 255]);
        let changed = nudged(&base, 3, 100);
        let Some(diff) = diff_image(&base, &changed, Tolerance::CLOSE_ENOUGH) else {
            panic!("same size, so there is a diff to draw");
        };
        assert_eq!(diff.size, base.size);
        assert_eq!(&diff.rgba[0..4], &[10, 20, 30, 255], "matching, darkened");
        assert_eq!(
            &diff.rgba[12..16],
            &[255, 0, 255, 255],
            "differing, magenta"
        );
    }

    #[test]
    fn there_is_no_diff_to_draw_between_two_different_shapes() {
        assert!(
            diff_image(
                &solid(4, 4, [0, 0, 0, 255]),
                &solid(5, 4, [0, 0, 0, 255]),
                Tolerance::CLOSE_ENOUGH,
            )
            .is_none()
        );
    }

    #[test]
    fn a_captured_frame_survives_the_trip_through_a_file() {
        // What a golden reference is: a capture written out and read back. If
        // this were lossy, every reference would be compared against something
        // slightly other than what produced it.
        let image = RawImage {
            size: PhysicalSize::new(3, 2),
            rgba: (0..24).collect(),
        };
        let Ok(read_back) = decode_png(&encode_png(&image)) else {
            panic!("what was just written must read back");
        };
        assert_eq!(read_back, image);
        assert!(compare(&image, &read_back, Tolerance::EXACT).matched);
    }
}
