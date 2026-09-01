//! The sprite pipeline: one shader, one vertex format, one draw call per batch.
//!
//! Key types: `SpritePipeline`.
//! Depends on: `wgpu`, `jidousha-render-core`, `color`.
//! INVARIANT: this executes a `FramePlan`, it does not interpret one. Batches
//! are drawn in the order they arrive, with the vertices they carry — the
//! sorting and merging happened above the seam, and a backend that redid any of
//! it would make two backends disagree (renderer.md §1, §7).
//! INVARIANT (ADR-0003 §4): inside the WebGL2 envelope. No instancing, no
//! storage buffers, one small uniform buffer, one dynamic vertex buffer.

use jidousha_core::math::Mat4;
use jidousha_render_core::{Batch, QuadVertex};

use crate::color::linear;

/// Bytes one vertex occupies: position, uv, color — eight floats.
const VERTEX_SIZE: u64 = 8 * 4;

/// How big the vertex buffer starts, in vertices.
///
/// A few hundred quads' worth. It grows by doubling when a frame needs more,
/// and never shrinks: a game's busiest frame is a fair guess at its next
/// busiest frame.
const INITIAL_VERTICES: u64 = 4096;

/// Everything the GPU needs to draw the engine's one kind of thing.
pub(crate) struct SpritePipeline {
    pipeline: wgpu::RenderPipeline,
    /// The view-projection matrix, re-written once a frame.
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    /// Kept because every texture needs a bind group made against it.
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    vertices: wgpu::Buffer,
    /// How many vertices `vertices` can hold.
    capacity: u64,
}

impl SpritePipeline {
    /// Compile the shader and build the pipeline for a surface of `format`.
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("jidousha sprite shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("jidousha camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("jidousha camera"),
            size: 4 * 4 * 4,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("jidousha camera"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("jidousha texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // DELIBERATE: nearest-neighbour filtering, not linear. Prototype 2D art
        // is pixel art far more often than not, and linear filtering turns it
        // to mush at every scale but 1:1 — "why is my sprite blurry" is the
        // single most common first complaint about a 2D engine. It also keeps
        // R4's golden images stable, since nearest sampling leaves far less
        // room for drivers to disagree. Revisit when a game wants smooth
        // scaling; the shape of that change is a per-texture choice, not a
        // different global default.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("jidousha sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("jidousha sprite layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("jidousha sprite pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: VERTEX_SIZE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 16,
                            shader_location: 2,
                        },
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // Straight alpha over what is already there — the painter's
                    // algorithm, which is what the sort key is for
                    // (renderer.md §2).
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                // DELIBERATE: no face culling. A negative `Transform::scale`
                // mirrors a sprite, which reverses its winding — with culling
                // on, a game that flipped a character by scaling it by -1 would
                // watch it vanish. Two triangles cost nothing to rasterize
                // either way.
                cull_mode: None,
                ..Default::default()
            },
            // No depth buffer: 2D transparency is back-to-front by the sort key
            // (renderer.md §2).
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            camera_buffer,
            camera_bind_group,
            texture_layout,
            sampler,
            vertices: new_vertex_buffer(device, INITIAL_VERTICES),
            capacity: INITIAL_VERTICES,
        }
    }

    /// A bind group naming one texture, for the batches that sample it.
    ///
    /// Made once when the texture is uploaded rather than once per frame: a
    /// bind group is a description of where things are, and where they are does
    /// not change.
    pub(crate) fn bind_texture(
        &self,
        device: &wgpu::Device,
        texture: &wgpu::Texture,
    ) -> wgpu::BindGroup {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("jidousha texture"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Put this frame's camera and vertices where the GPU can read them.
    ///
    /// `scratch` is the caller's buffer, reused between frames so packing does
    /// not allocate sixty times a second. Returns the vertex ranges to draw,
    /// one per batch, in plan order.
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        batches: &[Batch],
        scratch: &mut Vec<u8>,
    ) -> Vec<core::ops::Range<u32>> {
        queue.write_buffer(
            &self.camera_buffer,
            0,
            &pack_matrix(view_projection.to_cols_array()),
        );

        scratch.clear();
        let mut ranges = Vec::with_capacity(batches.len());
        let mut start = 0u32;
        for batch in batches {
            pack_vertices(&batch.vertices, scratch);
            let end = start + u32::try_from(batch.vertices.len()).unwrap_or(u32::MAX);
            ranges.push(start..end);
            start = end;
        }

        let needed = u64::from(start);
        if needed > self.capacity {
            // Doubling until it fits, so a game that grows steadily does not
            // reallocate every frame. The old buffer is dropped, and wgpu frees
            // it once the frames still using it have finished.
            let mut capacity = self.capacity.max(1);
            while capacity < needed {
                capacity *= 2;
            }
            self.vertices = new_vertex_buffer(device, capacity);
            self.capacity = capacity;
        }
        if !scratch.is_empty() {
            queue.write_buffer(&self.vertices, 0, scratch);
        }
        ranges
    }

    /// Set the pipeline up on a pass that is about to draw batches.
    /// How many bytes of buffer this pipeline is holding on the GPU.
    ///
    /// The vertex buffer, which grows with the busiest frame the run has had,
    /// and the camera uniform, which never does. Read once a frame by the
    /// performance panel's accounting and by nothing else (renderer.md §12a).
    pub(crate) fn buffer_bytes(&self) -> u64 {
        self.vertices.size() + self.camera_buffer.size()
    }

    pub(crate) fn bind(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
    }
}

/// A vertex buffer holding `vertices` vertices.
fn new_vertex_buffer(device: &wgpu::Device, vertices: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("jidousha vertices"),
        size: vertices * VERTEX_SIZE,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// Lay vertices out the way the pipeline's attributes say they are.
///
/// DELIBERATE: written out rather than casting the slice with `bytemuck`. The
/// layout is stated once, here, in the same order the `VertexAttribute` list
/// above declares — so the two can be read against each other — and it costs no
/// dependency and no `repr(C)` promise on a type that belongs to another crate
/// (practices §5.8).
fn pack_vertices(vertices: &[QuadVertex], out: &mut Vec<u8>) {
    for vertex in vertices {
        out.extend_from_slice(&vertex.position.x.to_ne_bytes());
        out.extend_from_slice(&vertex.position.y.to_ne_bytes());
        out.extend_from_slice(&vertex.uv.x.to_ne_bytes());
        out.extend_from_slice(&vertex.uv.y.to_ne_bytes());
        for channel in linear(vertex.color) {
            out.extend_from_slice(&channel.to_ne_bytes());
        }
    }
}

/// The view-projection matrix as the uniform buffer's sixteen floats.
fn pack_matrix(columns: [f32; 16]) -> [u8; 64] {
    let mut out = [0u8; 64];
    for (index, value) in columns.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&value.to_ne_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use jidousha_core::Color;
    use jidousha_core::math::Vec2;

    #[test]
    fn a_vertex_packs_into_the_stride_the_pipeline_declares() {
        // If these two ever disagree the GPU reads garbage, and it reads it
        // silently — a stride mismatch draws *something*, just not this.
        let mut bytes = Vec::new();
        pack_vertices(
            &[QuadVertex {
                position: Vec2::new(1.0, 2.0),
                uv: Vec2::new(0.25, 0.75),
                color: Color::WHITE,
            }],
            &mut bytes,
        );
        assert_eq!(bytes.len(), VERTEX_SIZE as usize);
    }

    #[test]
    fn the_packed_fields_land_at_the_offsets_the_attributes_name() {
        let mut bytes = Vec::new();
        pack_vertices(
            &[QuadVertex {
                position: Vec2::new(3.0, -4.0),
                uv: Vec2::new(0.5, 1.0),
                color: Color::rgba(1.0, 1.0, 1.0, 0.25),
            }],
            &mut bytes,
        );
        let float_at = |offset: usize| {
            let mut four = [0u8; 4];
            four.copy_from_slice(&bytes[offset..offset + 4]);
            f32::from_ne_bytes(four)
        };
        assert_eq!(float_at(0), 3.0, "position.x at offset 0");
        assert_eq!(float_at(4), -4.0);
        assert_eq!(float_at(8), 0.5, "uv at offset 8");
        assert_eq!(float_at(12), 1.0);
        assert_eq!(float_at(16), 1.0, "color at offset 16");
        assert_eq!(float_at(28), 0.25, "alpha last, unconverted");
    }

    #[test]
    fn vertex_colors_are_linearized_on_the_way_to_the_gpu() {
        // The shader multiplies and nothing else, so if this did not happen
        // here it would not happen at all — and a tinted sprite would be the
        // wrong brightness in a way nobody would think to look for.
        let mut bytes = Vec::new();
        pack_vertices(
            &[QuadVertex {
                position: Vec2::ZERO,
                uv: Vec2::ZERO,
                color: Color::rgb(0.5, 0.5, 0.5),
            }],
            &mut bytes,
        );
        let mut four = [0u8; 4];
        four.copy_from_slice(&bytes[16..20]);
        let red = f32::from_ne_bytes(four);
        assert!((red - 0.2140).abs() < 1e-3, "{red}");
    }

    #[test]
    fn the_matrix_packs_column_major_into_sixty_four_bytes() {
        // Sixteen distinct values, on purpose: the identity matrix is mostly
        // zeroes, and a buffer that is mostly zeroes agrees with one that was
        // never written. This is the mutation-caught version of that mistake.
        let columns: [f32; 16] = core::array::from_fn(|index| index as f32 + 1.0);
        let packed = pack_matrix(columns);
        assert_eq!(packed.len(), 64);
        for (index, expected) in columns.iter().enumerate() {
            let mut four = [0u8; 4];
            four.copy_from_slice(&packed[index * 4..index * 4 + 4]);
            assert_eq!(f32::from_ne_bytes(four), *expected, "float {index}");
        }
    }

    #[test]
    fn the_matrix_reaches_the_gpu_in_the_order_the_shader_reads_it() {
        // WGSL's `mat4x4<f32>` is column-major, and so is glam. A row-major
        // pack would compile, run, and put every sprite somewhere else.
        let mut translation = Mat4::IDENTITY;
        translation.w_axis.x = 7.0;
        let packed = pack_matrix(translation.to_cols_array());
        let mut four = [0u8; 4];
        // Column three, element zero — offset 12 floats in, not 3.
        four.copy_from_slice(&packed[12 * 4..12 * 4 + 4]);
        assert_eq!(f32::from_ne_bytes(four), 7.0);
    }
}
