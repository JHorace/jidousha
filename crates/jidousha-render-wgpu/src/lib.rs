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
//! and resize. The sprite pipeline and texture upload land at R2, which is when
//! the batches in a plan start being drawn rather than counted.

mod init;

use jidousha_render_core::{
    BackendTextureId, FramePlan, PhysicalSize, RawImage, RenderBackend, RenderError, TextureDesc,
};

use crate::init::{Gpu, Pending, configure};

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
    textures: Vec<Option<wgpu::Texture>>,
}

enum State {
    /// Waiting for an adapter and a device.
    Starting(Box<Pending>),
    /// Ready to draw.
    Running(Box<Gpu>),
    /// The machine cannot provide a GPU; the error is reported once per frame
    /// asked, not stored, because `render` is where a caller can act on it.
    Failed(RenderError),
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
        })
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
                self.state = State::Running(Box::new(gpu));
                Ok(())
            }
            Err(error) => {
                self.state = State::Failed(error.clone());
                Err(error)
            }
        }
    }
}

impl RenderBackend for WgpuBackend {
    fn create_texture(&mut self, desc: &TextureDesc, texels: &[u8]) -> BackendTextureId {
        let id = BackendTextureId(u32::try_from(self.textures.len()).unwrap_or(u32::MAX));
        let State::Running(gpu) = &self.state else {
            // Before the device exists there is nothing to upload to. The id is
            // still handed out and still valid: the engine's texture table maps
            // ids it does not have to the placeholder, so a sprite waiting on
            // this draws the placeholder rather than nothing (renderer.md §5).
            self.textures.push(None);
            return id;
        };
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("jidousha texture"),
            size: wgpu::Extent3d {
                width: desc.size.width,
                height: desc.size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.queue.write_texture(
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
        self.textures.push(Some(texture));
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
            State::Running(gpu) => {
                if let Ok(config) = configure(&gpu.surface, &gpu.adapter, &gpu.device, size) {
                    gpu.config = config;
                }
            }
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
        let gpu = match &mut self.state {
            State::Running(gpu) => gpu,
            State::Starting(_) => return Ok(()),
            State::Failed(error) => return Err(error.clone()),
        };

        let frame = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            // Suboptimal still draws: the picture is correct, the swap chain is
            // merely no longer ideal. Reconfiguring next frame is enough, and
            // throwing this one away would flicker.
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                gpu.surface.configure(&gpu.device, &gpu.config);
                frame
            }
            // The window changed under us, or there is nothing to draw into.
            // Reconfiguring and skipping the frame is the standard recovery,
            // and one dropped frame is invisible at sixty a second.
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                gpu.surface.configure(&gpu.device, &gpu.config);
                return Ok(());
            }
            // Occluded means nobody can see it; a timeout means the compositor
            // is busy. Both are reasons to skip, not to fail.
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                return Ok(());
            }
            other => {
                return Err(RenderError::SurfaceLost {
                    detail: format!("the surface could not be acquired: {other:?}"),
                });
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("jidousha frame"),
            });
        {
            // R1 draws the clear and nothing else. The plan's batches are
            // carried, sorted and batched by render-core already, and start
            // being drawn at R2 when there is a pipeline to draw them with.
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("jidousha clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(plan.clear_color.r),
                            g: f64::from(plan.clear_color.g),
                            b: f64::from(plan.clear_color.b),
                            a: f64::from(plan.clear_color.a),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        gpu.queue.submit(Some(encoder.finish()));
        gpu.queue.present(frame);
        Ok(())
    }

    fn capture(&mut self) -> Result<RawImage, RenderError> {
        // Offscreen readback lands at R4, with the golden-image tests that are
        // its only caller. Refusing is honest; a blank image would let a golden
        // test pass against nothing (renderer.md §9).
        Err(RenderError::Unsupported {
            detail: "capture lands with the golden-image tests at R4".to_owned(),
        })
    }
}
