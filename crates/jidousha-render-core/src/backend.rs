//! The seam: what a render backend must do, stated in engine types only.
//!
//! Key types: `RenderBackend`, `BackendTextureId`, `TextureDesc`, `RawImage`,
//! `RenderError`.
//! Depends on: `jidousha-core`, `plan`. Must never depend on: `wgpu`, `ash`, or
//! any graphics API (ADR-0003, CONTRACT).
//! INVARIANT: five methods, and none of them takes a graphics type. Backends
//! are dumb executors — sorting, batching, and every other decision happens
//! above this line, which is what keeps the ash port and the WebGL2 fallback
//! cheap (renderer.md §1, §7).

use core::fmt;

use jidousha_core::PhysicalSize;

use jidousha_core::message;

use crate::plan::FramePlan;

/// A texture as the backend knows it.
///
/// Meaningless above the seam: render-core stores these and hands them back in
/// a [`FramePlan`], and only the backend knows what one points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackendTextureId(pub u32);

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
    /// The machine will not give the engine a GPU to draw with.
    ///
    /// The earliest failure there is — before a device, before a surface,
    /// before any frame exists — and the most common one this project
    /// produces, because every runner it has is headless and some have no
    /// graphics stack at all. **It is not a failure of the run.** Everything
    /// above the backend seam is backend-agnostic and the draw transcript is
    /// the tier that always runs, so a headless check reports this as a skip
    /// and goes on asserting (renderer.md §9).
    ///
    /// Distinct from [`Unsupported`](RenderError::Unsupported) because the two
    /// have nothing in common but the word "cannot": that one is the backend
    /// declining something it was asked to draw, and this one is there being
    /// no backend to ask. Folding the two together sent every reader without a
    /// GPU — which was every E0 run — to check their texture sizes against the
    /// WebGL2 envelope (e0-findings.md F-067).
    NoAdapter {
        /// What the backend said.
        detail: String,
    },
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
            RenderError::NoAdapter { detail } => (
                "there is no graphics adapter",
                detail,
                "the machine has no GPU, or no driver for one — a headless container or a CI \
                 runner usually has neither",
                "install a driver — on Linux `mesa-vulkan-drivers` supplies lavapipe, a \
                 software rasterizer — or treat this as a skip: a run that asserts on the draw \
                 transcript needs no adapter at all (renderer.md §9). `tools/doctor` reports \
                 which drivers are present",
            ),
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

    /// Every variant, so a new one cannot be added carrying a copy of its
    /// neighbour's diagnosis.
    fn one_of_each() -> Vec<RenderError> {
        let detail = "what the backend said".to_owned();
        vec![
            RenderError::NoAdapter {
                detail: detail.clone(),
            },
            RenderError::SurfaceLost {
                detail: detail.clone(),
            },
            RenderError::DeviceLost {
                detail: detail.clone(),
            },
            RenderError::Unsupported { detail },
        ]
    }

    #[test]
    fn a_missing_adapter_is_not_reported_as_a_frame_the_backend_cannot_draw() {
        // The message an agent on a machine with no GPU reads, and for five E0
        // runs it told them to check their texture sizes against the WebGL2
        // envelope — a subsystem with no bearing on there being no driver
        // (e0-findings.md F-067). It has to name the driver instead.
        let text = RenderError::NoAdapter {
            detail: "no graphics adapter: no suitable adapter found".to_owned(),
        }
        .to_string();
        assert!(
            text.starts_with("[jidousha] there is no graphics adapter"),
            "{text}"
        );
        assert!(
            text.contains("mesa-vulkan-drivers"),
            "the fix names the package that supplies one: {text}"
        );
        assert!(
            !text.contains("WebGL2"),
            "nothing here is about what a frame asked for: {text}"
        );
    }

    #[test]
    fn no_two_render_errors_offer_the_same_diagnosis() {
        // The bug F-067 records is one variant's cause and fix being wrong for
        // a situation folded into it. Distinct text per variant is not proof
        // each one is right, but a duplicate is proof one of them is wrong.
        let messages: Vec<String> = one_of_each()
            .iter()
            .map(|error| {
                let text = error.to_string();
                text.split("\n  ")
                    .filter(|line| line.starts_with("likely cause:") || line.starts_with("fix:"))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        for (index, message) in messages.iter().enumerate() {
            assert!(
                !message.is_empty(),
                "{:?} states a cause and a fix",
                one_of_each()[index]
            );
            assert!(
                !messages[index + 1..].contains(message),
                "two variants give the same diagnosis, so one of them is wrong: {message}"
            );
        }
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
