//! Reading a rendered frame back off the GPU (renderer.md §9, tier 2).
//!
//! Key functions: `read_back`.
//! Depends on: `wgpu`, `jidousha-render-core`.
//! INVARIANT: this is the only place in the engine that blocks. It waits on a
//! buffer map, which is what reading pixels costs; a game never calls it, and
//! the frame loop's rule against blocking is intact because golden images are a
//! test tier rather than part of a frame.
//! INVARIANT: the row padding wgpu requires on a texture-to-buffer copy is
//! stripped here and nowhere else. A caller that saw padded rows would see an
//! image that is the right length and skewed diagonally.

use jidousha_render_core::{PhysicalSize, RawImage, RenderError};

/// How wide a row of a copy destination must be, in bytes.
///
/// wgpu requires every row of a texture-to-buffer copy to start on a 256-byte
/// boundary, so a capture buffer is wider than the picture and every row has
/// padding on the end. Forgetting to strip it produces an image that is skewed
/// diagonally and still the right length — which is why this is a named
/// function with a test rather than an expression inside the copy.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const COPY_ROW_ALIGNMENT: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// The padded stride, in bytes, for a row of `width` RGBA8 pixels.
///
/// Compiled on every target even though only the native readback calls it: it
/// is plain arithmetic, it is the part of the copy that is easy to get subtly
/// wrong, and a test that runs everywhere costs nothing to keep.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn padded_row_bytes(width: u32) -> u32 {
    let unpadded = width * 4;
    unpadded.div_ceil(COPY_ROW_ALIGNMENT) * COPY_ROW_ALIGNMENT
}

/// Copy a rendered texture off the GPU, one row at a time, unpadded.
///
/// Blocking, and native-only by construction: the wait is `Device::poll`, which
/// the web has no equivalent for. Golden images are a native tier (renderer.md
/// §9), and a game never calls this — the frame loop's rule against blocking is
/// intact.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_back(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: PhysicalSize,
) -> Result<RawImage, RenderError> {
    if size.width == 0 || size.height == 0 {
        return Err(RenderError::Unsupported {
            detail: format!(
                "a {}x{} target has no pixels to read back",
                size.width, size.height
            ),
        });
    }
    let padded = padded_row_bytes(size.width);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("jidousha capture"),
        size: u64::from(padded) * u64::from(size.height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("jidousha capture"),
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(size.height),
            },
        },
        wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        // A closed channel means the caller gave up, which cannot happen — it
        // is blocked on the receive below.
        let _ = sender.send(result);
    });
    if let Err(error) = device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    }) {
        return Err(RenderError::DeviceLost {
            detail: format!("the device stopped while reading a frame back: {error}"),
        });
    }
    match receiver.recv() {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            return Err(RenderError::DeviceLost {
                detail: format!("the captured frame could not be mapped: {error}"),
            });
        }
        Err(error) => {
            return Err(RenderError::DeviceLost {
                detail: format!("the capture never reported back: {error}"),
            });
        }
    }

    let unpadded = (size.width * 4) as usize;
    let mut rgba = Vec::with_capacity(unpadded * size.height as usize);
    {
        let Ok(mapped) = slice.get_mapped_range() else {
            return Err(RenderError::DeviceLost {
                detail: "the captured frame was mapped and then could not be read".to_owned(),
            });
        };
        for row in mapped.chunks_exact(padded as usize) {
            rgba.extend_from_slice(&row[..unpadded]);
        }
    }
    buffer.unmap();
    Ok(RawImage { size, rgba })
}

/// Reading pixels back needs a blocking wait, which the web does not have.
///
/// Golden images are a native tier (renderer.md §9) — on the web the check that
/// a frame reached the screen is `tools/serve-web --check`, which screenshots
/// the page from outside rather than asking the engine.
#[cfg(target_arch = "wasm32")]
pub(crate) fn read_back(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    _texture: &wgpu::Texture,
    _size: PhysicalSize,
) -> Result<RawImage, RenderError> {
    Err(RenderError::Unsupported {
        detail: "reading a frame back needs a blocking wait, which the web has no equivalent \
                 for; use tools/serve-web --check to verify the web target"
            .to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_narrower_than_the_alignment_is_padded_up_to_it() {
        // The common case for a small golden image, and the one where getting
        // it wrong produces a picture skewed diagonally rather than a crash.
        assert_eq!(padded_row_bytes(1), COPY_ROW_ALIGNMENT);
        assert_eq!(
            padded_row_bytes(16),
            COPY_ROW_ALIGNMENT,
            "64 bytes rounds up"
        );
    }

    #[test]
    fn a_row_that_is_already_aligned_is_not_padded_further() {
        // The off-by-one this rules out: rounding a multiple *up* to the next
        // boundary wastes a row's worth of bytes and shifts every row after it.
        assert_eq!(padded_row_bytes(64), 256, "64 pixels is exactly 256 bytes");
        assert_eq!(padded_row_bytes(128), 512);
    }

    #[test]
    fn a_row_one_pixel_past_a_boundary_takes_a_whole_extra_block() {
        assert_eq!(padded_row_bytes(65), 512);
    }

    #[test]
    fn every_padded_row_is_a_whole_number_of_blocks_and_holds_the_pixels() {
        // The two properties the copy actually depends on, over a range wide
        // enough to cover any capture the envelope allows.
        for width in [1u32, 2, 7, 63, 64, 65, 100, 255, 256, 1024, 1920] {
            let padded = padded_row_bytes(width);
            assert_eq!(padded % COPY_ROW_ALIGNMENT, 0, "width {width}");
            assert!(padded >= width * 4, "width {width}");
            assert!(
                padded < width * 4 + COPY_ROW_ALIGNMENT,
                "no wasted block at {width}"
            );
        }
    }
}
