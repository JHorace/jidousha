//! PNG encoding — the inverse of `decode`, and the same library.
//!
//! Key functions: `encode_png`.
//! Depends on: `png`, `payload`.
//! INVARIANT (assets.md §3): the encoder and the decoder are the same crate, so
//! anything written here reads back through [`decode_png`](crate::decode_png)
//! bit for bit. A second PNG implementation would be one more thing that has to
//! agree with the first, and golden images are exactly where a disagreement
//! would show up as a picture that is "nearly" right.
//!
//! DELIBERATE: encoding lives in the asset crate even though nothing *loads* an
//! encoded image. It is here because PNG is here (see `decode.rs` for why that
//! is), and splitting the format across two crates to satisfy the crate's name
//! would put the two halves of one file format where they could drift apart.
//! The callers are golden images and `tools/verify` artifacts, both above this
//! layer, both native.

use crate::payload::TextureData;

/// Encode RGBA8 texels as a PNG.
///
/// Written at 8-bit RGBA, no interlacing and no ancillary chunks, so the same
/// pixels produce the same bytes on every machine — a golden reference is a
/// file that gets diffed, and a timestamp inside it would make every run a
/// change.
///
/// # Panics
///
/// If `image.rgba` is not `width * height * 4` bytes, or either dimension is
/// zero. Both are contract violations rather than environmental failures
/// (core.md §9): the caller built the image, and a picture with no pixels is
/// not a picture.
#[must_use]
pub fn encode_png(image: &TextureData) -> Vec<u8> {
    let expected = (image.width as usize) * (image.height as usize) * 4;
    assert!(
        image.width > 0 && image.height > 0,
        "{}",
        jidousha_core::message(
            &format!("cannot encode a {}x{} image", image.width, image.height),
            "a PNG has at least one pixel on each axis",
            "an empty capture or a zero-sized render target was handed straight to the encoder",
            "check the size before encoding; a backend refuses to capture a zero-sized target",
        )
    );
    assert!(
        image.rgba.len() == expected,
        "{}",
        jidousha_core::message(
            &format!(
                "a {}x{} image carries {} bytes",
                image.width,
                image.height,
                image.rgba.len()
            ),
            &format!("RGBA8 at that size is {expected} bytes"),
            "the size and the texels came from different places",
            "build the image from one source of truth for its dimensions",
        )
    );

    let mut bytes = Vec::new();
    // The writer is a `Vec`, so the only `io::Error` this API can produce is
    // one a `Vec` cannot raise. The parameters are checked above, which leaves
    // nothing for a `Result` to carry that the asserts have not already said.
    {
        let mut encoder = png::Encoder::new(&mut bytes, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let Ok(mut writer) = encoder.write_header() else {
            unreachable!("a Vec writer cannot fail and the size is checked above")
        };
        let Ok(()) = writer.write_image_data(&image.rgba) else {
            unreachable!("a Vec writer cannot fail and the length is checked above")
        };
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_png;

    fn image(width: u32, height: u32, fill: impl Fn(u32, u32) -> [u8; 4]) -> TextureData {
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&fill(x, y));
            }
        }
        TextureData {
            width,
            height,
            rgba,
        }
    }

    #[test]
    fn an_encoded_image_decodes_to_the_pixels_it_was_given() {
        // The property the whole golden tier rests on: what is written is what
        // comes back, exactly. A tolerance covers GPU rasterization differences,
        // not a lossy round trip through the file.
        let original = image(7, 5, |x, y| [x as u8 * 30, y as u8 * 50, 17, 255]);
        let Ok(read_back) = decode_png(&encode_png(&original)) else {
            panic!("what was just written must read back");
        };
        assert_eq!(read_back, original);
    }

    #[test]
    fn alpha_survives_the_round_trip() {
        // RGB-with-alpha is the format the engine has (renderer.md §3), and an
        // encoder that quietly dropped the alpha channel would turn every
        // transparent golden pixel opaque without changing a single colour.
        let original = image(4, 4, |x, _| [255, 0, 0, x as u8 * 60]);
        let Ok(read_back) = decode_png(&encode_png(&original)) else {
            panic!("what was just written must read back");
        };
        assert_eq!(read_back.rgba[3], 0, "the first pixel is transparent");
        assert_eq!(read_back, original);
    }

    #[test]
    fn the_same_pixels_encode_to_the_same_bytes_twice() {
        // A golden reference is a file that gets diffed. A timestamp or any
        // other ambient value inside it would make every run look like a change.
        let original = image(9, 3, |x, y| [x as u8, y as u8, 200, 255]);
        assert_eq!(encode_png(&original), encode_png(&original));
    }

    #[test]
    fn a_single_pixel_is_a_legal_picture() {
        // The smallest thing a capture can produce, and the edge the size check
        // must not reject.
        let original = image(1, 1, |_, _| [1, 2, 3, 4]);
        let Ok(read_back) = decode_png(&encode_png(&original)) else {
            panic!("one pixel is a picture");
        };
        assert_eq!(read_back.rgba, vec![1, 2, 3, 4]);
    }

    #[test]
    #[should_panic(expected = "cannot encode a 0x4 image")]
    fn an_image_with_no_pixels_is_refused_by_name() {
        let _ = encode_png(&TextureData {
            width: 0,
            height: 4,
            rgba: Vec::new(),
        });
    }

    #[test]
    #[should_panic(expected = "carries 3 bytes")]
    fn texels_that_do_not_match_the_size_are_refused_by_name() {
        // The failure this catches is a caller that built the buffer and the
        // dimensions from two different places — which produces a picture that
        // encodes without complaint and is skewed diagonally.
        let _ = encode_png(&TextureData {
            width: 2,
            height: 2,
            rgba: vec![0, 0, 0],
        });
    }
}
