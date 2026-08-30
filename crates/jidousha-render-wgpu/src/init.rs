//! Getting a GPU without an async runtime.
//!
//! Key types: `Pending`, `Gpu`.
//! Depends on: `wgpu`, `jidousha-render-core`.
//! INVARIANT: no executor, no `block_on`, no runtime dependency. wgpu's adapter
//! and device requests are futures; this polls them from the frame loop the
//! engine already has, which is the same answer ADR-0011 gave for assets — a
//! game loop expresses "not yet, ask again" by being a loop.
//!
//! DELIBERATE: a ten-line poll rather than `pollster` or `wasm-bindgen-futures`
//! plus two code paths. Blocking is not available on the web at all, so a
//! native-blocks/web-spawns design would be two implementations of one thing,
//! only one of which could ever be tested here. Polling is one implementation
//! that is correct on both: on native these futures are ready almost at once,
//! and on the web the browser resolves the promise on its own schedule while
//! the frame loop keeps asking.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use jidousha_render_core::{PhysicalSize, Presentation, RenderError};

/// A GPU that is ready to draw.
pub(crate) struct Gpu {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) target: Target,
    /// Kept because reconfiguring the surface on resize needs it again.
    pub(crate) adapter: wgpu::Adapter,
}

/// Where a frame is drawn.
///
/// A window and an offscreen texture differ in three places — how the target is
/// created, how a frame is acquired, and whether it is presented — and in
/// nothing else. Everything between those points is one code path, which is the
/// whole reason golden images are worth taking: a capture goes through the same
/// pipeline, the same shader and the same uploads as the picture on screen
/// (renderer.md §9).
pub(crate) enum Target {
    /// A window's surface, presented every frame.
    Window {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    },
    /// A texture nobody sees, which can be read back.
    Offscreen {
        texture: wgpu::Texture,
        size: PhysicalSize,
    },
}

impl Target {
    /// The format a pipeline drawing into this must be built for.
    pub(crate) fn format(&self) -> wgpu::TextureFormat {
        match self {
            Target::Window { config, .. } => config.format,
            Target::Offscreen { texture, .. } => texture.format(),
        }
    }
}

/// The format an offscreen target is drawn into.
///
/// sRGB, like the surface formats the window path prefers, so the GPU encodes
/// on write exactly as it does on screen and the bytes read back are the bytes
/// a PNG holds. An offscreen target in linear space would produce captures that
/// are correct and look nothing like the window.
pub(crate) const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Make the texture an offscreen backend draws into.
///
/// `COPY_SRC` is what separates this from any other render target: it is the
/// usage that makes the pixels readable afterwards, and it is why capture is an
/// offscreen-only operation rather than something a window could also do.
pub(crate) fn offscreen_texture(device: &wgpu::Device, size: PhysicalSize) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("jidousha offscreen target"),
        size: wgpu::Extent3d {
            width: size.width.max(1),
            height: size.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: OFFSCREEN_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// A GPU that has been asked for and has not arrived.
pub(crate) struct Pending {
    wants: Wants,
    size: PhysicalSize,
    step: Step,
}

/// What the pending GPU is being asked for.
enum Wants {
    /// A surface, already made, waiting to be configured.
    Window(Option<wgpu::Surface<'static>>),
    /// A texture, made once the device exists.
    Offscreen,
}

/// Where the handshake has got to.
enum Step {
    /// Waiting for an adapter.
    Adapter(BoxFuture<Result<wgpu::Adapter, wgpu::RequestAdapterError>>),
    /// Waiting for a device, holding the adapter that will make it.
    Device {
        adapter: wgpu::Adapter,
        future: BoxFuture<Result<(wgpu::Device, wgpu::Queue), wgpu::RequestDeviceError>>,
    },
}

type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

impl Pending {
    /// Ask for an adapter that can draw to `surface`.
    pub(crate) fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: PhysicalSize,
    ) -> Self {
        let future = instance.request_adapter(&wgpu::RequestAdapterOptions {
            // High performance, which on a multi-GPU machine means "the
            // discrete one" — and the discrete one is where the compositor
            // usually is. Presenting means handing the compositor a buffer it
            // can import, and a buffer exported by one vendor's driver and
            // imported by another's may simply be refused: on a laptop with an
            // NVIDIA GPU driving the display and an AMD integrated GPU, every
            // windowed example died with "importing the supplied dmabufs
            // failed" before this line said so.
            //
            // This asked for `LowPower` on the reasoning that a 2D prototype
            // does not need the discrete GPU and asking costs battery. True
            // about power, wrong about which GPU can show the result: it sorts
            // the integrated adapter to the front, which is worse than the
            // default `None` (no sorting at all). Battery is the right thing to
            // want and the wrong thing to buy with a window that never opens.
            //
            // See e0-findings.md F-011. The offscreen path below stays on
            // LowPower — it presents to nothing, so none of this applies.
            power_preference: wgpu::PowerPreference::HighPerformance,
            // Required when targeting WebGL2 — an adapter is only useful if it
            // can present to the surface we already made (ADR-0003 §4).
            //
            // Not the fix for the mismatch above: under Wayland this filter
            // excludes almost nothing, because presentation goes through buffer
            // sharing rather than direct scanout, so every renderable GPU
            // reports that it can present.
            compatible_surface: Some(&surface),
            ..Default::default()
        });
        Self {
            wants: Wants::Window(Some(surface)),
            size,
            step: Step::Adapter(Box::pin(future)),
        }
    }

    /// Ask for any adapter at all, to draw into a texture.
    ///
    /// No `compatible_surface`, because there is no surface: this is what lets
    /// a golden-image test and a `tools/verify` capture run on a machine with
    /// no display — including every CI runner the project has.
    pub(crate) fn offscreen(instance: &wgpu::Instance, size: PhysicalSize) -> Self {
        let future = instance.request_adapter(&wgpu::RequestAdapterOptions {
            // Deliberately different from the windowed path above, which asks
            // for HighPerformance. Nothing here is presented to a compositor, so
            // the cross-vendor import that forces that choice cannot arise, and
            // the cheapest adapter that can draw a test frame is the right one.
            // An unexplained difference between these two calls would read as an
            // oversight, which is the only reason this comment exists.
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            ..Default::default()
        });
        Self {
            wants: Wants::Offscreen,
            size,
            step: Step::Adapter(Box::pin(future)),
        }
    }

    /// The surface size to configure with when the device arrives.
    ///
    /// A window resized while the GPU is still coming would otherwise be
    /// configured at the size it had when the program started.
    pub(crate) fn set_size(&mut self, size: PhysicalSize) {
        self.size = size;
    }

    /// Move the handshake along by as much as it will go right now.
    ///
    /// Returns the GPU on the poll that completes it, `None` while it is still
    /// coming, and an error if the machine cannot provide one.
    pub(crate) fn poll(&mut self) -> Result<Option<Gpu>, RenderError> {
        loop {
            match &mut self.step {
                Step::Adapter(future) => {
                    let Some(result) = poll_once(future.as_mut()) else {
                        return Ok(None);
                    };
                    // `NoAdapter`, not `Unsupported`: there being no GPU on
                    // this machine is not the backend declining a frame, and
                    // the two want opposite advice (e0-findings.md F-067).
                    let adapter = result.map_err(|error| RenderError::NoAdapter {
                        // Just what wgpu said. The "no graphics adapter"
                        // prefix this used to carry was doing the variant's
                        // job back when the variant said something else.
                        detail: error.to_string(),
                    })?;
                    let future = adapter.request_device(&wgpu::DeviceDescriptor {
                        label: Some("jidousha device"),
                        // The WebGL2 envelope, asked for by name: whatever the
                        // adapter can do, clamped to what the weakest target
                        // supports, so a frame that works here works there
                        // (ADR-0003 §4, renderer.md §8).
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                            .using_resolution(adapter.limits()),
                        memory_hints: wgpu::MemoryHints::default(),
                        trace: wgpu::Trace::Off,
                        experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    });
                    self.step = Step::Device {
                        adapter,
                        future: Box::pin(future),
                    };
                }
                Step::Device { adapter, future } => {
                    // Cloning is a refcount bump; wgpu handles are Arc-backed.
                    let adapter = adapter.clone();
                    let Some(result) = poll_once(future.as_mut()) else {
                        return Ok(None);
                    };
                    let (device, queue) = result.map_err(|error| RenderError::Unsupported {
                        detail: format!("no graphics device: {error}"),
                    })?;
                    let target = match &mut self.wants {
                        Wants::Offscreen => Target::Offscreen {
                            texture: offscreen_texture(&device, self.size),
                            size: self.size,
                        },
                        Wants::Window(surface) => {
                            let Some(surface) = surface.take() else {
                                return Err(RenderError::Unsupported {
                                    detail: "the surface was already taken — poll called after \
                                             it completed"
                                        .to_owned(),
                                });
                            };
                            let config = configure(&surface, &adapter, &device, self.size)?;
                            Target::Window { surface, config }
                        }
                    };
                    return Ok(Some(Gpu {
                        device,
                        queue,
                        target,
                        adapter,
                    }));
                }
            }
        }
    }
}

/// The present mode this engine asks every windowed surface for.
///
/// **Vsync, explicitly.** wgpu's `get_default_config` takes the *first* mode the
/// surface reports, and what that is depends entirely on the backend: DX12 lists
/// `[Mailbox, Fifo, …]` and Vulkan hands back whatever order the driver's
/// `vkGetPhysicalDeviceSurfacePresentModesKHR` happened to use — commonly
/// `Immediate` first. Neither of those waits for the display, so the frame loop
/// ran at whatever the machine could manage: a paused 2D prototype holding a
/// core and a GPU at full tilt (frame-pacing.md §6). Fifo is the one mode every
/// backend offers and the one that waits.
///
/// The fallback when a surface does not offer it lives above the seam, in the
/// driver, because it is a decision about the *loop* rather than about the
/// surface — this crate reports what it got and never paces anything
/// (renderer.md §7).
const WANTED_PRESENT_MODE: wgpu::PresentMode = wgpu::PresentMode::Fifo;

/// What a configured surface's present mode means above the seam.
///
/// `FifoRelaxed` counts as vsync: it waits for the refresh like `Fifo` and only
/// tears when a frame has already missed one, so the display is still what
/// paces the loop. `AutoVsync` and `AutoNoVsync` are wgpu's "pick one for me"
/// aliases and are never what this crate configured, but a match has to be
/// total and guessing wrong in the safe direction means capping a loop that did
/// not need it rather than letting a runaway through.
pub(crate) fn presentation_of(mode: wgpu::PresentMode) -> Presentation {
    match mode {
        wgpu::PresentMode::Fifo | wgpu::PresentMode::FifoRelaxed | wgpu::PresentMode::AutoVsync => {
            Presentation::Vsync
        }
        wgpu::PresentMode::Mailbox => Presentation::Mailbox,
        wgpu::PresentMode::Immediate | wgpu::PresentMode::AutoNoVsync => Presentation::Immediate,
    }
}

/// Set the surface up for the size it is now, and return what was chosen.
///
/// The bulk of the configuration comes from wgpu's own default for this surface
/// and adapter: which alpha mode, how many frames of latency. Those are
/// questions about the machine, and upstream answers them better than a guess
/// here would — and answers them again when a new wgpu adds a field.
///
/// The present mode is the exception, and [`WANTED_PRESENT_MODE`] says why.
pub(crate) fn configure(
    surface: &wgpu::Surface<'static>,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    size: PhysicalSize,
) -> Result<wgpu::SurfaceConfiguration, RenderError> {
    // A zero-sized surface is invalid to configure, and a minimized window
    // reports one. A single pixel is the smallest lie that keeps the surface
    // configurable until the window comes back.
    let width = size.width.max(1);
    let height = size.height.max(1);
    let Some(mut config) = surface.get_default_config(adapter, width, height) else {
        return Err(RenderError::Unsupported {
            detail: "the adapter cannot present to this surface".to_owned(),
        });
    };
    config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;

    // sRGB where it is offered: the engine's colors are sRGB-encoded
    // (conventions), and letting the surface do the encoding is what keeps the
    // WebGPU and WebGL2 paths looking the same (renderer.md §8).
    let capabilities = surface.get_capabilities(adapter);
    if let Some(srgb) = capabilities
        .formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
    {
        config.format = srgb;
    }

    // Vsync where the surface offers it, which in practice is everywhere:
    // Vulkan requires FIFO of every conformant implementation, and wgpu's DX12,
    // Metal and GL backends all list it. Where it is somehow absent, whatever
    // the surface put first stands and `presentation` below reports it — the
    // driver reads that and caps the loop itself (frame-pacing.md §6).
    if capabilities.present_modes.contains(&WANTED_PRESENT_MODE) {
        config.present_mode = WANTED_PRESENT_MODE;
    }
    surface.configure(device, &config);
    Ok(config)
}

/// Poll a future once, with a waker that does nothing.
///
/// Nothing needs waking: the caller is a frame loop that will ask again next
/// frame. That is the whole executor.
fn poll_once<T>(future: Pin<&mut dyn Future<Output = T>>) -> Option<T> {
    let mut context = Context::from_waker(Waker::noop());
    match future.poll(&mut context) {
        Poll::Ready(value) => Some(value),
        Poll::Pending => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ready_future_resolves_on_the_first_poll() {
        let mut future = Box::pin(async { 7 });
        assert_eq!(poll_once(future.as_mut()), Some(7));
    }

    #[test]
    fn only_the_modes_that_wait_for_the_display_are_reported_as_vsync() {
        // The mapping the driver's cap hangs off. Getting `FifoRelaxed` wrong
        // would cap a loop the display is already pacing; getting `Mailbox`
        // wrong would leave the runaway this whole change is about
        // (frame-pacing.md §6). `AutoNoVsync` is grouped with `Immediate`
        // deliberately: the safe direction to guess is "cap it".
        for waits in [
            wgpu::PresentMode::Fifo,
            wgpu::PresentMode::FifoRelaxed,
            wgpu::PresentMode::AutoVsync,
        ] {
            assert_eq!(presentation_of(waits), Presentation::Vsync, "{waits:?}");
            assert!(!presentation_of(waits).needs_a_cap(), "{waits:?}");
        }
        for never_waits in [
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::AutoNoVsync,
        ] {
            assert!(
                presentation_of(never_waits).needs_a_cap(),
                "{never_waits:?} would leave the loop unbounded"
            );
        }
    }

    #[test]
    fn the_mode_this_engine_asks_for_is_the_one_that_waits() {
        // A one-line guard on the constant itself, because the whole fix is
        // that line and a change to it would otherwise only show up on a
        // machine with a display.
        assert_eq!(presentation_of(WANTED_PRESENT_MODE), Presentation::Vsync);
    }

    #[test]
    fn a_pending_future_reports_that_it_is_not_done() {
        // The case the frame loop relies on: no waker is registered, no thread
        // is parked, and asking again later is the only mechanism.
        let mut polls = 0;
        let mut future = Box::pin(core::future::poll_fn(|_| {
            polls += 1;
            if polls < 3 {
                Poll::Pending
            } else {
                Poll::Ready("arrived")
            }
        }));
        assert_eq!(poll_once(future.as_mut()), None);
        assert_eq!(poll_once(future.as_mut()), None);
        assert_eq!(poll_once(future.as_mut()), Some("arrived"));
    }
}

#[cfg(test)]
mod wgpu_tests {
    use super::*;

    #[test]
    fn wgpu_futures_resolve_under_a_no_op_waker() {
        // The load-bearing assumption of this whole module: wgpu's adapter
        // request is a future, the engine has no executor, and asking it
        // repeatedly with a waker that does nothing is enough to get an answer.
        //
        // Deliberately not an assertion about *finding* an adapter — this
        // machine may have no GPU at all, and `Err` is a fine answer. What is
        // being checked is that the future reaches an answer rather than
        // staying `Pending` forever, which is the failure that would hang every
        // game's first frames.
        let instance = wgpu::Instance::default();
        let mut future = Box::pin(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            ..Default::default()
        }));

        // A generous bound: on a real machine this resolves on the first poll
        // or two. Anything that needs thousands is a design problem, not a slow
        // machine.
        for _ in 0..10_000 {
            if let Some(result) = poll_once(future.as_mut()) {
                // Report which it was, so the log says what this machine has.
                match result {
                    Ok(adapter) => println!("adapter: {:?}", adapter.get_info().backend),
                    Err(error) => println!("no adapter here, which is a fine answer: {error}"),
                }
                return;
            }
        }
        panic!("wgpu's adapter future never resolved under polling");
    }
}
