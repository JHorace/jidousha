//! The seam: what a render backend must do, stated in engine types only.
//!
//! Key types: `RenderBackend`, `BackendTextureId`, `TextureDesc`, `RawImage`,
//! `RenderError`, `Presentation`.
//! Depends on: `jidousha-core`, `plan`. Must never depend on: `wgpu`, `ash`, or
//! any graphics API (ADR-0003, CONTRACT).
//! INVARIANT: six methods, and none of them takes a graphics type. Backends
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

/// How a backend's finished frames reach the display.
///
/// The **pacing** fact, and the only one above the seam: a frame loop that
/// draws as fast as it can is correct on a swap chain that waits for the
/// display and is a runaway on one that does not. The driver reads this once a
/// frame and decides whether the display is pacing the loop or the loop has to
/// pace itself (frame-pacing.md §6).
///
/// Engine words, not a graphics API's: these are the three things a swap chain
/// can do with a finished frame, and every backend wgpu has — and ash will —
/// expresses its own modes in exactly these terms (ADR-0003).
///
/// It is also the line the native overlay prints, which is why the variants are
/// named rather than folded into a `bool`: "capped at 60fps because this
/// surface will not vsync" and "presenting at the display's own rate" are
/// different readings, and a playtester's report has to be able to say which.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presentation {
    /// Nothing is reaching a display: the GPU has not arrived yet, or this
    /// backend draws into a texture nobody sees.
    ///
    /// Not a pacing failure — there is nothing to pace. A driver in this state
    /// is either a few frames into startup, polling for a device it needs
    /// promptly, or headless, where the loop belongs to a test.
    Offscreen,
    /// Every present waits for the display's next refresh.
    ///
    /// The display sets the pace. A loop that added a cap of its own would beat
    /// against the refresh and drop frames on a rhythm nobody asked for, which
    /// is why [`needs_a_cap`](Presentation::needs_a_cap) says no here.
    Vsync,
    /// The newest finished frame replaces whatever was queued for the display.
    ///
    /// Never tears, never waits: frames are drawn as fast as the machine will
    /// draw them and most of them are thrown away. Smooth, and unbounded — a
    /// paused 2D game will hold a core and a GPU at whatever they can manage.
    Mailbox,
    /// Frames go to the display the moment they are finished, tearing if that
    /// lands mid-scan.
    ///
    /// Unbounded for the same reason [`Mailbox`](Presentation::Mailbox) is.
    Immediate,
}

impl Presentation {
    /// Whether the frame loop has to cap itself, because nothing else will.
    ///
    /// The **one** question the driver asks of this type. `Offscreen` answers
    /// no deliberately: a startup that has not got a device yet needs to poll
    /// for one promptly, and a headless run has no display to pace against.
    #[must_use]
    pub fn needs_a_cap(self) -> bool {
        matches!(self, Presentation::Mailbox | Presentation::Immediate)
    }
}

impl fmt::Display for Presentation {
    /// What the overlay's pacing line calls this, in the vocabulary a bug
    /// report can be searched for.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Presentation::Offscreen => "no surface yet",
            Presentation::Vsync => "vsync",
            Presentation::Mailbox => "mailbox",
            Presentation::Immediate => "immediate",
        })
    }
}

/// What every render backend implements.
///
/// Six methods. Growth beyond about eight is a design smell to resist: every
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

    /// How this backend's frames reach the display, as of now.
    ///
    /// Asked once a frame rather than once at startup: a backend answers
    /// [`Presentation::Offscreen`] until its device arrives, and the answer
    /// afterwards is a property of the surface it ended up with rather than of
    /// the one it asked for. The driver paces the loop on it (frame-pacing.md
    /// §6).
    ///
    /// CONTRACT: a report, never a request. Nothing above the seam may set the
    /// present mode — that is the backend's negotiation with the machine, and a
    /// caller that could override it would be deciding for a driver it cannot
    /// see.
    fn presentation(&self) -> Presentation;
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
    fn only_the_modes_that_never_wait_ask_the_loop_for_a_cap() {
        // The whole pacing decision, and the failure it exists to prevent: a
        // surface that presents without waiting leaves nothing bounding the
        // frame rate, so the loop bounds itself (frame-pacing.md §6). The two
        // that must answer *no* are as load-bearing as the two that answer yes
        // — a cap on top of vsync beats against the refresh, and a cap during
        // startup slows the device handshake down.
        assert!(Presentation::Mailbox.needs_a_cap());
        assert!(Presentation::Immediate.needs_a_cap());
        assert!(!Presentation::Vsync.needs_a_cap());
        assert!(!Presentation::Offscreen.needs_a_cap());
    }

    #[test]
    fn every_presentation_prints_a_name_a_bug_report_can_be_searched_for() {
        // This string goes on the overlay's pacing line and into whatever a
        // playtester pastes back. Two modes sharing a name would make the one
        // reading that matters — "is this machine vsynced or not" —
        // unanswerable from the report.
        let names: Vec<String> = [
            Presentation::Offscreen,
            Presentation::Vsync,
            Presentation::Mailbox,
            Presentation::Immediate,
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        for (index, name) in names.iter().enumerate() {
            assert!(!name.is_empty());
            assert!(!names[index + 1..].contains(name), "two modes print {name}");
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
