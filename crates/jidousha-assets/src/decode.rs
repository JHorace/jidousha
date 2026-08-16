//! PNG decoding — one implementation, called from whichever thread has the
//! bytes.
//!
//! Key functions: `decode_png`.
//! Depends on: `png`, `payload`.
//! INVARIANT (assets.md §3, CONTRACT): every platform decodes through this
//! function. Never the browser's image decoder, never a second path — texel
//! data has to be bit-identical everywhere or the golden-image tests
//! (renderer.md §9) compare two different pictures.
//!
//! DELIBERATE: this lives in `jidousha-assets` rather than in the platform
//! crates that read the files, which looks like a layering inversion and is
//! not. assets.md §5 wants native decoding to happen off the frame, on the
//! loader thread; §3 wants one code path everywhere. Both hold if the *code*
//! lives here and the *call* happens wherever the bytes landed — the native
//! source calls this from its loader thread, and the web source will call it at
//! its commit point. What would break §3 is each platform bringing its own
//! decoder, which is exactly what putting this in the platform crates invites.

use crate::payload::{AssetError, MAX_TEXTURE_SIZE, TextureData};

/// Decode a PNG into RGBA8 texels.
///
/// # Errors
///
/// If the bytes are not a readable PNG, or the image is larger than the
/// envelope allows on either axis (renderer.md §8).
pub fn decode_png(bytes: &[u8]) -> Result<TextureData, AssetError> {
    // png 0.18 wants `Read + Seek`, which a slice is not by itself.
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|error| AssetError::Decode {
        detail: error.to_string(),
    })?;

    // Checked before decoding rather than after: a 16000×16000 PNG would
    // otherwise allocate a gigabyte on its way to being rejected.
    let info = reader.info();
    let (width, height) = (info.width, info.height);
    if width > MAX_TEXTURE_SIZE || height > MAX_TEXTURE_SIZE {
        return Err(AssetError::TooLarge { width, height });
    }

    let mut buffer = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let frame = reader
        .next_frame(&mut buffer)
        .map_err(|error| AssetError::Decode {
            detail: error.to_string(),
        })?;
    buffer.truncate(frame.buffer_size());

    let rgba = to_rgba8(&buffer, frame.color_type, frame.bit_depth, width, height)?;
    Ok(TextureData {
        width,
        height,
        rgba,
    })
}

/// Widen whatever the file held into straight RGBA8.
///
/// PNG carries greyscale, palette, and colour with or without alpha, at several
/// bit depths. The engine has one texture format (renderer.md §3), so the
/// widening happens once, here, rather than as a special case in every backend.
fn to_rgba8(
    buffer: &[u8],
    color: png::ColorType,
    depth: png::BitDepth,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, AssetError> {
    if depth != png::BitDepth::Eight {
        // `Transformations::normalize_to_color8` above would handle this, but
        // saying so plainly beats a silently wrong picture.
        return Err(AssetError::Decode {
            detail: format!("{depth:?} bit depth is not supported; re-export as 8-bit"),
        });
    }
    let pixels = (width as usize) * (height as usize);
    let mut rgba = Vec::with_capacity(pixels * 4);
    match color {
        png::ColorType::Rgba => return Ok(buffer.to_vec()),
        png::ColorType::Rgb => {
            for chunk in buffer.chunks_exact(3) {
                rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
        }
        png::ColorType::Grayscale => {
            for &value in buffer {
                rgba.extend_from_slice(&[value, value, value, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for chunk in buffer.chunks_exact(2) {
                rgba.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
        }
        png::ColorType::Indexed => {
            return Err(AssetError::Decode {
                detail: "indexed colour was not expanded by the decoder".to_owned(),
            });
        }
    }
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a PNG the decoder can be pointed at.
    ///
    /// Encoding here rather than checking binaries into the repository: a test
    /// that states the pixels it expects to get back is readable, and a
    /// checked-in file is not.
    fn png_bytes(width: u32, height: u32, color: png::ColorType, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, width, height);
            encoder.set_color(color);
            encoder.set_depth(png::BitDepth::Eight);
            let Ok(mut writer) = encoder.write_header() else {
                panic!("the test's own encoder should accept this header");
            };
            let Ok(()) = writer.write_image_data(data) else {
                panic!("the test's own encoder should accept this data");
            };
        }
        out
    }

    #[test]
    fn an_rgba_png_decodes_to_its_texels() {
        let texels = [255, 0, 0, 255, 0, 255, 0, 128];
        let decoded = decode_png(&png_bytes(2, 1, png::ColorType::Rgba, &texels));
        assert_eq!(
            decoded,
            Ok(TextureData {
                width: 2,
                height: 1,
                rgba: texels.to_vec(),
            })
        );
    }

    #[test]
    fn an_rgb_png_gains_an_opaque_alpha_channel() {
        let decoded = decode_png(&png_bytes(2, 1, png::ColorType::Rgb, &[1, 2, 3, 4, 5, 6]));
        assert_eq!(
            decoded.map(|texture| texture.rgba),
            Ok(vec![1, 2, 3, 255, 4, 5, 6, 255])
        );
    }

    #[test]
    fn a_greyscale_png_widens_to_the_engines_one_format() {
        // One texture format means the widening happens here rather than as a
        // special case in every backend (renderer.md §3).
        let decoded = decode_png(&png_bytes(2, 1, png::ColorType::Grayscale, &[10, 200]));
        assert_eq!(
            decoded.map(|texture| texture.rgba),
            Ok(vec![10, 10, 10, 255, 200, 200, 200, 255])
        );
    }

    #[test]
    fn greyscale_with_alpha_keeps_its_alpha() {
        let decoded = decode_png(&png_bytes(1, 1, png::ColorType::GrayscaleAlpha, &[90, 40]));
        assert_eq!(
            decoded.map(|texture| texture.rgba),
            Ok(vec![90, 90, 90, 40])
        );
    }

    #[test]
    fn bytes_that_are_not_a_png_are_refused_with_a_reason() {
        let error = decode_png(b"this is not a png at all");
        assert!(matches!(error, Err(AssetError::Decode { .. })), "{error:?}");
    }

    #[test]
    fn a_truncated_png_is_refused() {
        let mut bytes = png_bytes(4, 4, png::ColorType::Rgba, &[0; 64]);
        bytes.truncate(bytes.len() / 2);
        assert!(matches!(decode_png(&bytes), Err(AssetError::Decode { .. })));
    }

    #[test]
    fn an_image_past_the_envelope_is_refused_by_its_size() {
        // Checked from the header, before decoding: rejecting after allocating
        // for a 16000-pixel-wide image is a memory spike on the way to an
        // error message.
        let over = MAX_TEXTURE_SIZE + 1;
        let bytes = png_bytes(
            over,
            1,
            png::ColorType::Grayscale,
            &vec![0u8; over as usize],
        );
        assert_eq!(
            decode_png(&bytes),
            Err(AssetError::TooLarge {
                width: over,
                height: 1,
            })
        );
    }

    #[test]
    fn an_image_exactly_at_the_limit_is_accepted() {
        let bytes = png_bytes(
            MAX_TEXTURE_SIZE,
            1,
            png::ColorType::Grayscale,
            &vec![7u8; MAX_TEXTURE_SIZE as usize],
        );
        let decoded = decode_png(&bytes);
        assert_eq!(
            decoded.map(|texture| (texture.width, texture.height)),
            Ok((MAX_TEXTURE_SIZE, 1))
        );
    }

    #[test]
    fn decoding_is_deterministic() {
        // The CONTRACT the golden-image tests rest on: the same bytes give the
        // same texels, every time and everywhere (assets.md §3).
        let bytes = png_bytes(3, 2, png::ColorType::Rgb, &[9; 18]);
        assert_eq!(decode_png(&bytes), decode_png(&bytes));
    }
}
