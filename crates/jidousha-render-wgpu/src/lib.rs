//! The wgpu backend: the only crate in the engine that knows what a GPU is.
//!
//! Key types: `WgpuBackend`.
//! Depends on: `wgpu`, `jidousha-render-core`, `jidousha-core`. Must never be
//! depended on by: anything except the composition root that picks a backend.
//! INVARIANT (ADR-0003, CONTRACT): no `wgpu` type appears in this crate's
//! public API. `WgpuBackend` is opaque; everything it takes and returns is an
//! engine type from `jidousha-render-core`.
//! INVARIANT: this crate makes no decisions. It executes a `FramePlan` — it
//! does not sort, batch, cull, or reorder — because two backends that both
//! decide things are two backends that disagree (renderer.md §1).
//!
//! Built so far (`docs/internal/renderer.md` §11): R1 — surface, clear, present
//! and resize; R2 — the sprite pipeline, texture upload, and one draw call per
//! batch; R4 — the offscreen target and `capture`, which is what golden images
//! and `tools/verify`'s frame artifact are taken through.

mod capture;
mod color;
mod init;
mod pipeline;

use jidousha_render_core::{
    BackendTextureId, FramePlan, PhysicalSize, RawImage, RenderBackend, RenderError, TextureDesc,
};

use crate::capture::read_back;
use crate::color::linear;
use crate::init::{Gpu, Pending, Target, configure, offscreen_texture};
use crate::pipeline::SpritePipeline;

/// A renderer backed by wgpu.
///
/// Created before the GPU is ready and usable immediately, in the same shape
/// assets take (ADR-0011): asking is synchronous, arriving is not. Frames
/// submitted before the device lands are dropped rather than queued — a frame
/// is a picture of a moment, and showing a stale one later is worse than
/// showing none.
pub struct WgpuBackend {
    state: State,
    /// Textures the engine asked for, indexed by [`BackendTextureId`].
    textures: Vec<Option<Slot>>,
    /// This frame's vertex bytes, kept between frames so the allocation is not
    /// made sixty times a second.
    scratch: Vec<u8>,
}

enum State {
    /// Waiting for an adapter and a device.
    Starting(Box<Pending>),
    /// Ready to draw.
    Running(Box<Live>),
    /// The machine cannot provide a GPU; the error is reported once per frame
    /// asked, not stored, because `render` is where a caller can act on it.
    Failed(RenderError),
}

/// A GPU with a pipeline on it.
struct Live {
    gpu: Gpu,
    pipeline: SpritePipeline,
}

/// One texture the engine asked for.
enum Slot {
    /// Uploaded, with the bind group the pipeline draws it through.
    Ready { bind_group: wgpu::BindGroup },
    /// Asked for before the device arrived, and uploaded the moment it does.
    ///
    /// DELIBERATE: the texels are held rather than dropped, so
    /// [`create_texture`](RenderBackend::create_texture) means "this texture
    /// will be on the GPU" with no timing rider attached. Art usually finishes
    /// loading before the GPU handshake does — a small PNG off a warm disk
    /// beats an adapter and device negotiation — so this is the common path
    /// during startup rather than a corner. The alternative, making every
    /// caller ask whether the device has arrived first, is an unwritten rule
    /// that would be forgotten exactly once.
    Waiting { desc: TextureDesc, texels: Vec<u8> },
}

/// Which backends to let wgpu choose from.
///
/// DELIBERATE (web): WebGL2 only, not `Backends::all()`. Found by
/// `tools/serve-web --check`, which is the reason that tool exists: with all
/// backends enabled, wgpu asks the browser for WebGPU first, and on a browser
/// that has `navigator.gpu` but yields no adapter — Chromium under a software
/// rasterizer, and every browser without WebGPU support — the request fails and
/// **nothing falls back to GL**. The page loads, the engine runs, and the canvas
/// stays blank. Forcing GL renders correctly on the same machine.
///
/// This costs nothing today: the device is already asked for
/// `downlevel_webgl2_defaults` limits (ADR-0003 §4, renderer.md §8), so WebGPU
/// would be handed the same envelope WebGL2 gives. Revisit when there is a
/// reason to want WebGPU on the web — a compute path, or a limit the envelope
/// raises — and at that point the right shape is to try WebGPU and fall back,
/// which needs the window kept so a second surface can be made.
#[cfg(target_arch = "wasm32")]
fn instance_descriptor() -> wgpu::InstanceDescriptor {
    wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    }
}

/// Every backend this build has, which is what native wants: Vulkan, DX12, or
/// GL, whichever the machine offers.
#[cfg(not(target_arch = "wasm32"))]
fn instance_descriptor() -> wgpu::InstanceDescriptor {
    wgpu::InstanceDescriptor::new_without_display_handle()
}

impl WgpuBackend {
    /// Ask for a GPU that can draw to `window`.
    ///
    /// `window` is anything with a window and display handle — in practice the
    /// platform crate's window. It is taken by value and kept alive for as long
    /// as the surface is.
    ///
    /// # Errors
    ///
    /// If a surface cannot be created for the window at all. Adapter and device
    /// failures arrive later, through [`is_ready`](WgpuBackend::is_ready) and
    /// `render`, because they are asked for asynchronously.
    pub fn new<W>(window: W, size: PhysicalSize) -> Result<Self, RenderError>
    where
        W: wgpu::DisplayAndWindowHandle + 'static,
    {
        let instance = wgpu::Instance::new(instance_descriptor());
        let surface =
            instance
                .create_surface(window)
                .map_err(|error| RenderError::SurfaceLost {
                    detail: format!("the window could not provide a drawing surface: {error}"),
                })?;
        // The instance is not kept: wgpu refcounts it behind the surface, and a
        // field nobody reads is a field that will drift.
        Ok(Self {
            state: State::Starting(Box::new(Pending::new(&instance, surface, size))),
            textures: Vec::new(),
            scratch: Vec::new(),
        })
    }

    /// Ask for a GPU that draws into a texture nobody sees.
    ///
    /// No window and no surface, so this works on a machine with no display —
    /// which is every CI runner the project has, and the reason golden images
    /// are a tier that can actually run (renderer.md §9). Everything after the
    /// target is created is the same code the window uses: the same pipeline,
    /// the same shader, the same uploads.
    ///
    /// Unlike [`new`](WgpuBackend::new) this cannot fail up front — there is no
    /// surface to fail to create. An absent adapter arrives later, through
    /// [`poll`](WgpuBackend::poll) and `render`, like every other GPU failure.
    #[must_use]
    pub fn offscreen(size: PhysicalSize) -> Self {
        let instance = wgpu::Instance::new(instance_descriptor());
        Self {
            state: State::Starting(Box::new(Pending::offscreen(&instance, size))),
            textures: Vec::new(),
            scratch: Vec::new(),
        }
    }

    /// Whether the GPU has arrived.
    ///
    /// A driver can draw before this is true; nothing will appear, which is
    /// correct — there is nowhere to put it yet.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self.state, State::Running(_))
    }

    /// Move the GPU handshake along, if it is still going.
    ///
    /// Called once per frame by the driver. Doing this here rather than
    /// blocking in `new` is what keeps the first frames of a program responsive
    /// on every platform, and is the only design that works on the web at all.
    pub fn poll(&mut self) -> Result<(), RenderError> {
        let State::Starting(pending) = &mut self.state else {
            return Ok(());
        };
        match pending.poll() {
            Ok(None) => Ok(()),
            Ok(Some(gpu)) => {
                // The pipeline is built for the surface's format and kept for
                // the life of the backend. `configure` picks that format from
                // the surface and the adapter, neither of which changes, so a
                // resize cannot invalidate it — and if that ever stopped being
                // true, wgpu's validation would say so rather than draw wrong.
                let pipeline = SpritePipeline::new(&gpu.device, gpu.target.format());
                let live = Live { gpu, pipeline };
                self.state = State::Running(Box::new(live));
                self.upload_waiting();
                Ok(())
            }
            Err(error) => {
                self.state = State::Failed(error.clone());
                Err(error)
            }
        }
    }

    /// Upload every texture that was asked for before the device existed.
    ///
    /// Runs once, on the frame the GPU arrives. The texels are dropped as each
    /// one lands, so the wait costs memory only while it lasts.
    fn upload_waiting(&mut self) {
        let State::Running(live) = &mut self.state else {
            return;
        };
        for slot in &mut self.textures {
            if !matches!(slot, Some(Slot::Waiting { .. })) {
                continue;
            }
            // Taken out rather than read in place: the upload replaces the slot,
            // and the texels have to stop borrowing from it first.
            let Some(Slot::Waiting { desc, texels }) = slot.take() else {
                continue;
            };
            *slot = Some(upload(live, &desc, &texels));
        }
    }
}

/// Put texels on the GPU and make the bind group that draws them.
fn upload(live: &Live, desc: &TextureDesc, texels: &[u8]) -> Slot {
    let texture = live.gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("jidousha texture"),
        size: wgpu::Extent3d {
            width: desc.size.width,
            height: desc.size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // sRGB, so the GPU converts to linear light as it samples — the other
        // half of the conversion `color.rs` does for vertex colors.
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    live.gpu.queue.write_texture(
        texture.as_image_copy(),
        texels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(desc.size.width * 4),
            rows_per_image: Some(desc.size.height),
        },
        wgpu::Extent3d {
            width: desc.size.width,
            height: desc.size.height,
            depth_or_array_layers: 1,
        },
    );
    Slot::Ready {
        bind_group: live.pipeline.bind_texture(&live.gpu.device, &texture),
    }
}

impl RenderBackend for WgpuBackend {
    fn create_texture(&mut self, desc: &TextureDesc, texels: &[u8]) -> BackendTextureId {
        let id = BackendTextureId(u32::try_from(self.textures.len()).unwrap_or(u32::MAX));
        let slot = match &self.state {
            State::Running(live) => upload(live, desc, texels),
            // No device yet. Hold the texels and upload them when there is one;
            // the caller is told nothing, because from its side this texture is
            // on its way regardless.
            State::Starting(_) | State::Failed(_) => Slot::Waiting {
                desc: *desc,
                texels: texels.to_vec(),
            },
        };
        self.textures.push(Some(slot));
        id
    }

    fn destroy_texture(&mut self, id: BackendTextureId) {
        // Slots are never reused: an id handed back after being destroyed
        // should stay dead, not quietly name whatever was uploaded next.
        if let Some(slot) = self.textures.get_mut(id.0 as usize) {
            *slot = None;
        }
    }

    fn resize_surface(&mut self, size: PhysicalSize) {
        match &mut self.state {
            State::Running(live) => match &mut live.gpu.target {
                Target::Window { surface, config } => {
                    if let Ok(new_config) =
                        configure(surface, &live.gpu.adapter, &live.gpu.device, size)
                    {
                        *config = new_config;
                    }
                }
                // A new texture, because a texture cannot be resized. The old
                // one is dropped with the frame it held, which nothing has read
                // — a capture reads the frame it just rendered, and a resize
                // means there is a new frame coming.
                Target::Offscreen { texture, size: at } => {
                    *texture = offscreen_texture(&live.gpu.device, size);
                    *at = size;
                }
            },
            // Not ready yet: remember it, so the surface is configured at the
            // size the window has *now* rather than the size it had when the
            // program started. A window resized during startup is common —
            // a tiling compositor does it to every new window.
            State::Starting(pending) => pending.set_size(size),
            State::Failed(_) => {}
        }
    }

    fn render(&mut self, plan: &FramePlan) -> Result<(), RenderError> {
        self.poll()?;
        let Self {
            state,
            textures,
            scratch,
        } = self;
        let live = match state {
            State::Running(live) => live,
            State::Starting(_) => return Ok(()),
            State::Failed(error) => return Err(error.clone()),
        };

        // What to draw into, and what to hand back to the compositor after.
        // An offscreen target is always there; a surface has to be asked for,
        // and can say no in five different ways.
        let (view, present) = match &live.gpu.target {
            Target::Offscreen { texture, .. } => (
                texture.create_view(&wgpu::TextureViewDescriptor::default()),
                None,
            ),
            Target::Window { surface, config } => match surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => (
                    frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                    Some(frame),
                ),
                // Suboptimal still draws: the picture is correct, the swap chain
                // is merely no longer ideal. Reconfiguring next frame is enough,
                // and throwing this one away would flicker.
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    surface.configure(&live.gpu.device, config);
                    (
                        frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                        Some(frame),
                    )
                }
                // The window changed under us, or there is nothing to draw into.
                // Reconfiguring and skipping the frame is the standard recovery,
                // and one dropped frame is invisible at sixty a second.
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                    surface.configure(&live.gpu.device, config);
                    return Ok(());
                }
                // Occluded means nobody can see it; a timeout means the
                // compositor is busy. Both are reasons to skip, not to fail.
                wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                    return Ok(());
                }
                other => {
                    return Err(RenderError::SurfaceLost {
                        detail: format!("the surface could not be acquired: {other:?}"),
                    });
                }
            },
        };

        let ranges = live.pipeline.prepare(
            &live.gpu.device,
            &live.gpu.queue,
            plan.view_projection,
            &plan.batches,
            scratch,
        );

        let mut encoder = live
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("jidousha frame"),
            });
        {
            let clear = linear(plan.clear_color);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("jidousha sprites"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(clear[0]),
                            g: f64::from(clear[1]),
                            b: f64::from(clear[2]),
                            a: f64::from(clear[3]),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            live.pipeline.bind(&mut pass);
            // In plan order, one draw call each. No sorting, no merging, no
            // skipping: those decisions were made in render-core.
            for (batch, range) in plan.batches.iter().zip(ranges) {
                let Some(Some(Slot::Ready { bind_group })) = textures.get(batch.texture.0 as usize)
                else {
                    // An id the backend does not have. Either it was destroyed
                    // while still in a texture table, or it is still waiting for
                    // a device that this branch says has arrived — both are
                    // engine bugs rather than anything a game can cause, so the
                    // frame draws without it and says nothing per-frame.
                    debug_assert!(
                        false,
                        "[jidousha] a frame plan named backend texture {} which is not uploaded\n  \
                         likely cause: destroy_texture was called without TextureTable::forget\n  \
                         fix: report this with the reproduction — game code cannot cause it",
                        batch.texture.0
                    );
                    continue;
                };
                pass.set_bind_group(1, bind_group, &[]);
                pass.draw(range, 0..1);
            }
        }
        live.gpu.queue.submit(Some(encoder.finish()));
        if let Some(frame) = present {
            live.gpu.queue.present(frame);
        }
        Ok(())
    }

    fn capture(&mut self) -> Result<RawImage, RenderError> {
        let State::Running(live) = &mut self.state else {
            return Err(RenderError::Unsupported {
                detail: "there is no GPU yet, so there is no frame to read back".to_owned(),
            });
        };
        let Target::Offscreen { texture, size } = &live.gpu.target else {
            // DELIBERATE: a windowed backend refuses rather than reading its
            // surface back. A presented surface texture is gone, and keeping a
            // readable copy would mean a full-screen blit on every frame of
            // every game to serve a feature only tests use. `offscreen` is the
            // constructor that can answer this, and it renders through the same
            // pipeline — which is what makes the answer worth having
            // (renderer.md §9).
            return Err(RenderError::Unsupported {
                detail: "a windowed backend cannot read its surface back; build the backend \
                         with WgpuBackend::offscreen to capture frames"
                    .to_owned(),
            });
        };
        read_back(&live.gpu.device, &live.gpu.queue, texture, *size)
    }
}
