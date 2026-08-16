//! The backend that draws nothing and records everything.
//!
//! Key types: `NullBackend`, `FrameRecord`, `DrawnQuad`.
//! Depends on: `backend`, `plan`, `jidousha-core`.
//! INVARIANT: this is the primary verification tier, not a stub. Transcripts
//! answer "what was drawn, where, in what order, in how many batches" on every
//! target with no GPU — which is how an agent asks visual questions without
//! rendering a pixel (renderer.md §9).

use core::fmt::Write as _;

use jidousha_core::math::Vec2;
use jidousha_core::{Color, PhysicalSize, Rect};

use crate::backend::{BackendTextureId, RawImage, RenderBackend, RenderError, TextureDesc};
use crate::plan::{Batch, FramePlan};

/// One quad, read back out of a recorded frame.
///
/// The plan stores triangles because that is what a GPU wants; this is the same
/// data in the shape a question is asked in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawnQuad {
    /// Which batch it came from, so batching is observable.
    pub batch: usize,
    /// What it sampled.
    pub texture: BackendTextureId,
    /// The four corners, in world space, in draw order.
    pub corners: [Vec2; 4],
    /// The tint.
    pub tint: Color,
}

impl DrawnQuad {
    /// The axis-aligned box around the quad.
    ///
    /// For an unrotated sprite this is the sprite; for a rotated one it is the
    /// box it sweeps, which is usually what a coarse "is it on screen" question
    /// wants.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        let mut min = self.corners[0];
        let mut max = self.corners[0];
        for corner in &self.corners[1..] {
            min = min.min(*corner);
            max = max.max(*corner);
        }
        Rect { min, max }
    }

    /// Whether `world` is inside the quad itself, rotation included.
    ///
    /// Exact rather than approximate: a rotated sprite's bounding box claims
    /// corners the sprite does not cover, and "is the cursor on the ship" is
    /// precisely the question that would get the wrong answer.
    #[must_use]
    pub fn contains(&self, world: Vec2) -> bool {
        // A convex quad contains a point when the point is on the same side of
        // every edge. Zero counts as inside, so an edge or a degenerate quad
        // still answers sensibly.
        let mut positive = false;
        let mut negative = false;
        for index in 0..4 {
            let from = self.corners[index];
            let to = self.corners[(index + 1) % 4];
            let cross = (to - from).perp_dot(world - from);
            if cross > 0.0 {
                positive = true;
            }
            if cross < 0.0 {
                negative = true;
            }
        }
        !(positive && negative)
    }
}

/// One frame, as it was submitted to the backend.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameRecord {
    /// The plan exactly as the backend received it.
    pub plan: FramePlan,
}

impl FrameRecord {
    /// Every quad drawn this frame, in draw order.
    #[must_use]
    pub fn quads(&self) -> Vec<DrawnQuad> {
        let mut quads = Vec::new();
        for (index, batch) in self.plan.batches.iter().enumerate() {
            append_batch_quads(&mut quads, index, batch);
        }
        quads
    }

    /// Every quad covering `world`, front to back — the last one drawn first.
    ///
    /// This is "what is at this point?", which with the camera's
    /// `screen_to_world` is also "what did the player just click on?".
    #[must_use]
    pub fn covering(&self, world: Vec2) -> Vec<DrawnQuad> {
        let mut hits: Vec<DrawnQuad> = self
            .quads()
            .into_iter()
            .filter(|quad| quad.contains(world))
            .collect();
        hits.reverse();
        hits
    }

    /// How many quads were drawn.
    #[must_use]
    pub fn quad_count(&self) -> usize {
        self.plan.quad_count()
    }

    /// The frame as text: deterministic, diffable, and readable in a failure
    /// message.
    ///
    /// This is the snapshot format the transcript tests assert on. Floats are
    /// printed to three decimals — enough to see a sprite move by a pixel, few
    /// enough that the last bit of a rotation does not rewrite the file.
    #[must_use]
    pub fn transcript(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "clear {}", format_color(self.plan.clear_color));
        for (index, batch) in self.plan.batches.iter().enumerate() {
            let _ = writeln!(
                out,
                "batch {index}: texture {} ({} quads)",
                batch.texture.0,
                batch.quad_count()
            );
            let mut quads = Vec::new();
            append_batch_quads(&mut quads, index, batch);
            for quad in quads {
                let bounds = quad.bounds();
                let _ = writeln!(
                    out,
                    "  quad {} {} tint {}",
                    format_vec(bounds.min),
                    format_vec(bounds.max),
                    format_color(quad.tint)
                );
            }
        }
        out
    }
}

fn append_batch_quads(out: &mut Vec<DrawnQuad>, batch_index: usize, batch: &Batch) {
    // Six vertices per quad, wound (0,1,2),(0,2,3) — so the fourth corner is
    // the last vertex of the second triangle.
    for chunk in batch.vertices.chunks_exact(6) {
        out.push(DrawnQuad {
            batch: batch_index,
            texture: batch.texture,
            corners: [
                chunk[0].position,
                chunk[1].position,
                chunk[2].position,
                chunk[5].position,
            ],
            tint: chunk[0].color,
        });
    }
}

fn format_vec(value: Vec2) -> String {
    format!("({:.3}, {:.3})", value.x, value.y)
}

fn format_color(color: Color) -> String {
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        channel(color.r),
        channel(color.g),
        channel(color.b),
        channel(color.a)
    )
}

/// A float channel as a byte, clamped — a tint outside 0..1 is a game's
/// business, and the transcript should still be readable.
fn channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// A backend that records frames instead of drawing them.
///
/// Every target can run this, including wasm CI, and it is the tier that keeps
/// render-core honest — golden images (R4) keep the *GPU* backend honest, which
/// is a different job (renderer.md §9).
pub struct NullBackend {
    frames: Vec<FrameRecord>,
    /// Every texture created, with the texels it was given.
    ///
    /// The texels are kept, not counted: renderer.md §5's CONTRACT is that the
    /// placeholder is *bit-identical* across backends, and a backend that
    /// discarded what it was handed could not be asked whether that is true.
    textures: Vec<(TextureDesc, Vec<u8>)>,
    destroyed: Vec<BackendTextureId>,
    surface: PhysicalSize,
}

impl NullBackend {
    /// A backend with nothing recorded and a 1280×720 surface.
    ///
    /// DELIBERATE: no `Default` impl, despite `clippy::new_without_default`
    /// (see ADR-0012) — one way to do everything, and `new` is that way.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            textures: Vec::new(),
            destroyed: Vec::new(),
            surface: PhysicalSize::new(1280, 720),
        }
    }

    /// Every frame recorded so far, oldest first.
    #[must_use]
    pub fn frames(&self) -> &[FrameRecord] {
        &self.frames
    }

    /// The most recent frame, if anything has been drawn.
    #[must_use]
    pub fn last_frame(&self) -> Option<&FrameRecord> {
        self.frames.last()
    }

    /// Forget every recorded frame, for a test that wants a fresh window.
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Every recorded frame as one transcript.
    #[must_use]
    pub fn transcript(&self) -> String {
        let mut out = String::new();
        for (index, frame) in self.frames.iter().enumerate() {
            let _ = writeln!(out, "frame {index}:");
            for line in frame.transcript().lines() {
                let _ = writeln!(out, "  {line}");
            }
        }
        out
    }

    /// The surface size the backend was last told about.
    #[must_use]
    pub fn surface(&self) -> PhysicalSize {
        self.surface
    }

    /// How many textures have been created, including destroyed ones.
    #[must_use]
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// What `id` was uploaded with: its description and its texels.
    ///
    /// `None` for an id this backend never issued. A destroyed id still answers
    /// — the record is of what was uploaded, which is a question about the past.
    #[must_use]
    pub fn uploaded(&self, id: BackendTextureId) -> Option<(TextureDesc, &[u8])> {
        self.textures
            .get(id.0 as usize)
            .map(|(desc, texels)| (*desc, texels.as_slice()))
    }

    /// Textures that were destroyed, in the order they were.
    #[must_use]
    pub fn destroyed(&self) -> &[BackendTextureId] {
        &self.destroyed
    }
}

impl RenderBackend for NullBackend {
    fn create_texture(&mut self, desc: &TextureDesc, texels: &[u8]) -> BackendTextureId {
        self.textures.push((*desc, texels.to_vec()));
        // Ids count up and are never reused: a test that draws with a stale id
        // should see the stale id, not something that quietly still works.
        BackendTextureId(u32::try_from(self.textures.len() - 1).unwrap_or(u32::MAX))
    }

    fn destroy_texture(&mut self, id: BackendTextureId) {
        self.destroyed.push(id);
    }

    fn resize_surface(&mut self, size: PhysicalSize) {
        self.surface = size;
    }

    fn render(&mut self, plan: &FramePlan) -> Result<(), RenderError> {
        self.frames.push(FrameRecord { plan: plan.clone() });
        Ok(())
    }

    fn capture(&mut self) -> Result<RawImage, RenderError> {
        // Honest refusal rather than a blank image: a golden-image test that
        // silently passed against a backend with no pixels would be worse than
        // one that could not run at all (renderer.md §9).
        Err(RenderError::Unsupported {
            detail: "the null backend records frames and has no pixels to read back".to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::QuadVertex;

    fn square(min: Vec2, max: Vec2) -> Vec<QuadVertex> {
        let corners = [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];
        [0, 1, 2, 0, 2, 3]
            .into_iter()
            .map(|index| QuadVertex {
                position: corners[index],
                uv: Vec2::ZERO,
                color: Color::WHITE,
            })
            .collect()
    }

    fn frame_with(vertices: Vec<QuadVertex>) -> FrameRecord {
        FrameRecord {
            plan: FramePlan {
                clear_color: Color::BLACK,
                view_projection: jidousha_core::math::Mat4::IDENTITY,
                batches: vec![Batch {
                    texture: BackendTextureId(0),
                    vertices,
                }],
            },
        }
    }

    #[test]
    fn a_recorded_frame_reads_back_as_quads() {
        let frame = frame_with(square(Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0)));
        let quads = frame.quads();
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].bounds().min, Vec2::new(-1.0, -1.0));
        assert_eq!(quads[0].bounds().max, Vec2::new(1.0, 1.0));
    }

    #[test]
    fn a_point_inside_a_quad_is_found() {
        let frame = frame_with(square(Vec2::ZERO, Vec2::new(2.0, 2.0)));
        assert_eq!(frame.covering(Vec2::new(1.0, 1.0)).len(), 1);
        assert!(frame.covering(Vec2::new(3.0, 1.0)).is_empty());
    }

    #[test]
    fn containment_follows_the_rotation_not_the_bounding_box() {
        // A diamond: its bounding box claims the corners, the quad does not.
        // "Is the cursor on the ship" is exactly this question.
        let corners = [
            Vec2::new(0.0, -1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(-1.0, 0.0),
        ];
        let vertices: Vec<QuadVertex> = [0, 1, 2, 0, 2, 3]
            .into_iter()
            .map(|index| QuadVertex {
                position: corners[index],
                uv: Vec2::ZERO,
                color: Color::WHITE,
            })
            .collect();
        let frame = frame_with(vertices);
        assert!(frame.covering(Vec2::ZERO).len() == 1, "the middle");
        assert!(
            frame.covering(Vec2::new(0.9, 0.9)).is_empty(),
            "inside the box, outside the diamond"
        );
    }

    #[test]
    fn overlapping_quads_come_back_front_first() {
        let mut vertices = square(Vec2::ZERO, Vec2::new(2.0, 2.0));
        let mut front = square(Vec2::ZERO, Vec2::new(2.0, 2.0));
        for vertex in &mut front {
            vertex.color = Color::RED;
        }
        vertices.extend(front);
        let frame = frame_with(vertices);
        let hits = frame.covering(Vec2::new(1.0, 1.0));
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].tint, Color::RED, "the last drawn is on top");
    }

    #[test]
    fn the_transcript_is_stable_text() {
        let frame = frame_with(square(Vec2::new(-1.5, -0.5), Vec2::new(1.5, 0.5)));
        assert_eq!(
            frame.transcript(),
            "clear #000000ff\n\
             batch 0: texture 0 (1 quads)\n  \
             quad (-1.500, -0.500) (1.500, 0.500) tint #ffffffff\n"
        );
    }

    #[test]
    fn the_null_backend_refuses_to_pretend_it_has_pixels() {
        let mut backend = NullBackend::new();
        assert!(matches!(
            backend.capture(),
            Err(RenderError::Unsupported { .. })
        ));
    }

    #[test]
    fn textures_get_distinct_ids_that_are_never_reused() {
        let mut backend = NullBackend::new();
        let desc = TextureDesc {
            size: PhysicalSize::new(1, 1),
        };
        let first = backend.create_texture(&desc, &[255, 255, 255, 255]);
        backend.destroy_texture(first);
        let second = backend.create_texture(&desc, &[255, 255, 255, 255]);
        assert_ne!(first, second, "a destroyed id must not come back");
        assert_eq!(backend.destroyed(), [first]);
    }
}
