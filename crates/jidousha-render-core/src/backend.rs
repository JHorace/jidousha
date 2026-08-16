//! The seam: what a render backend must do, stated in engine types only.
//!
//! Key types: `RenderBackend`, `BackendTextureId`, `TextureDesc`, `RawImage`,
//! `PhysicalSize`, `RenderError`.
//! Depends on: `jidousha-core`, `plan`. Must never depend on: `wgpu`, `ash`, or
//! any graphics API (ADR-0003, CONTRACT).
//! INVARIANT: five methods, and none of them takes a graphics type. Backends
//! are dumb executors — sorting, batching, and every other decision happens
//! above this line, which is what keeps the ash port and the WebGL2 fallback
//! cheap (renderer.md §1, §7).

use core::fmt;

use jidousha_core::message;

use crate::plan::FramePlan;

/// A texture as the backend knows it.
///
/// Meaningless above the seam: render-core stores these and hands them back in
/// a [`FramePlan`], and only the backend knows what one points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackendTextureId(pub u32);

/// The size of a surface or a texture, in physical pixels.
///
/// Physical, not logical: DPI scaling is the platform's business, and the
/// renderer works in the pixels it is actually given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalSize {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl PhysicalSize {
    /// A size in pixels.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Width divided by height, or 1.0 for a degenerate surface.
    ///
    /// A minimized window reports zero height, and a camera that divided by it
    /// would put NaN into every vertex of the frame.
    #[must_use]
    pub fn aspect(self) -> f32 {
        if self.width == 0 || self.height == 0 {
            return 1.0;
        }
        self.width as f32 / self.height as f32
    }
}

/// What a texture is made of.
///
/// v1 is RGBA8 sRGB and nothing else (renderer.md §3), so this carries only the
/// size; the format is implied and the same everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureDesc {
    /// Size in texels.
    pub size: PhysicalSize,
}

/// Pixels read back off the GPU, for golden-image tests (R4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawImage {
    /// Size in pixels.
    pub size: PhysicalSize,
    /// RGBA8, row-major, top row first.
    pub rgba: Vec<u8>,
}

/// What can go wrong in a backend.
///
/// Environmental: no adapter, a lost device, a surface that vanished. Contract
/// violations — an unknown texture, a NaN transform — are panics per core §9,
/// not values (renderer.md §10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// The surface could not be acquired this frame.
    SurfaceLost {
        /// What the backend said.
        detail: String,
    },
    /// The device is gone; v1 does not recreate it.
    DeviceLost {
        /// What the backend said.
        detail: String,
    },
    /// The backend cannot do something the plan asked for.
    Unsupported {
        /// What was asked.
        detail: String,
    },
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (what, detail, cause, fix) = match self {
            RenderError::SurfaceLost { detail } => (
                "the render surface was lost",
                detail,
                "the window was resized, minimized, or moved between displays mid-frame",
                "skip the frame and try the next one; a surface usually comes back by itself",
            ),
            RenderError::DeviceLost { detail } => (
                "the graphics device was lost",
                detail,
                "a driver reset, a GPU hang, or the machine went to sleep",
                "restart the program — v1 does not recreate the device (renderer.md §10)",
            ),
            RenderError::Unsupported { detail } => (
                "the backend cannot render this frame",
                detail,
                "the frame asked for something outside the WebGL2 envelope (renderer.md §8)",
                "check the texture sizes and the batch count against the envelope",
            ),
        };
        formatter.write_str(&message(what, detail, cause, fix))
    }
}

impl core::error::Error for RenderError {}

/// What every render backend implements.
///
/// Five methods. Growth beyond about eight is a design smell to resist: every
/// method here is one more thing the ash port and the WebGL2 path must both
/// get right (renderer.md §7).
pub trait RenderBackend {
    /// Upload a texture and name it.
    ///
    /// `texels` is RGBA8, row-major, `desc.size.width * desc.size.height * 4`
    /// bytes long.
    fn create_texture(&mut self, desc: &TextureDesc, texels: &[u8]) -> BackendTextureId;

    /// Release a texture. Drawing with the id afterwards is a contract
    /// violation, not an error value.
    fn destroy_texture(&mut self, id: BackendTextureId);

    /// The surface changed size.
    fn resize_surface(&mut self, size: PhysicalSize);

    /// Draw one frame.
    ///
    /// CONTRACT: the backend executes the plan as given. It does not reorder
    /// batches, merge them, or decide what is visible — those decisions were
    /// made above the seam, and a backend that second-guesses them makes two
    /// backends disagree.
    ///
    /// # Errors
    ///
    /// If the surface or device is unavailable.
    fn render(&mut self, plan: &FramePlan) -> Result<(), RenderError>;

    /// Read the last rendered frame back as pixels, for golden-image tests.
    ///
    /// # Errors
    ///
    /// If the backend cannot read back, or has nothing to read.
    fn capture(&mut self) -> Result<RawImage, RenderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_degenerate_surface_has_a_usable_aspect() {
        // A minimized window reports zero height. Dividing by it would put NaN
        // into every vertex of the frame, which is a far worse outcome than a
        // frame drawn at the wrong shape and never seen.
        assert_eq!(PhysicalSize::new(0, 0).aspect(), 1.0);
        assert_eq!(PhysicalSize::new(800, 0).aspect(), 1.0);
        assert_eq!(PhysicalSize::new(1600, 900).aspect(), 16.0 / 9.0);
    }

    #[test]
    fn a_render_error_reads_like_every_other_engine_error() {
        let error = RenderError::DeviceLost {
            detail: "adapter reported a reset".to_owned(),
        };
        let text = error.to_string();
        assert!(
            text.starts_with("[jidousha] the graphics device was lost"),
            "{text}"
        );
        assert!(text.contains("likely cause:"), "{text}");
        assert!(text.contains("fix:"), "{text}");
    }
}
