//! Sort, batch, and hand the backend something it can execute without thinking.
//!
//! Key types: `FramePlan`, `Batch`, `QuadVertex`, `TextureTable`.
//! Depends on: `jidousha-core`, `backend`, `camera`.
//! INVARIANT: this is where every decision is made. Backends execute a plan and
//! never revisit it, which is what keeps the ash port and the WebGL2 fallback
//! cheap (renderer.md §1).
//! CONTRACT: identical submission streams produce identical plans — same order,
//! same batches, same vertices, bit for bit. The transcript tests depend on it,
//! and so does every golden image later (renderer.md §2, §9).

use std::collections::BTreeMap;

use jidousha_core::math::{Mat4, Vec2};
use jidousha_core::{Color, Quad, TextureId};

use crate::backend::BackendTextureId;
use crate::camera::Camera;

/// One vertex of an expanded quad.
///
/// World-space position, texture coordinate, and color, exactly as the GPU
/// wants them. The view-projection matrix is the only transform left for the
/// backend to apply (renderer.md §7).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadVertex {
    /// Where, in world space.
    pub position: Vec2,
    /// Where to sample, normalized.
    pub uv: Vec2,
    /// The tint, multiplied into the sample.
    pub color: Color,
}

/// A run of quads sharing one texture, drawn in one call.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    /// What every quad in this batch samples.
    pub texture: BackendTextureId,
    /// Six vertices per quad: two triangles, wound the same way every time.
    pub vertices: Vec<QuadVertex>,
}

impl Batch {
    /// How many quads this batch draws.
    #[must_use]
    pub fn quad_count(&self) -> usize {
        self.vertices.len() / VERTICES_PER_QUAD
    }
}

/// Six vertices per quad — two triangles, no index buffer.
///
/// PERF: an index buffer would save a third of the vertex bandwidth. It is not
/// here because v1 has no evidence that vertex bandwidth is the constraint at
/// prototype scale, and one buffer is one fewer thing for two backends to agree
/// about (renderer.md §7).
const VERTICES_PER_QUAD: usize = 6;

/// Everything the backend needs to draw one frame.
///
/// Plain data, engine types only. A backend that inspects this and decides to
/// do something else has broken the contract that makes two backends agree.
#[derive(Clone, Debug, PartialEq)]
pub struct FramePlan {
    /// What to fill the screen with first.
    pub clear_color: Color,
    /// World space to clip space.
    pub view_projection: Mat4,
    /// What to draw, in the order to draw it.
    pub batches: Vec<Batch>,
}

impl FramePlan {
    /// How many quads the whole frame draws.
    #[must_use]
    pub fn quad_count(&self) -> usize {
        self.batches.iter().map(Batch::quad_count).sum()
    }
}

/// Which backend texture each [`TextureId`] currently maps to.
///
/// This is where the not-ready policy lives (renderer.md §5): an id nobody has
/// registered draws the **placeholder**. That covers both cases without asking
/// anyone — a texture still loading was never registered, and a texture that
/// failed never will be. Nothing here needs to know which.
pub struct TextureTable {
    white: BackendTextureId,
    placeholder: BackendTextureId,
    uploaded: BTreeMap<TextureId, BackendTextureId>,
}

impl TextureTable {
    /// A table with the two built-in textures the renderer always has.
    ///
    /// `white` is a 1×1 opaque white texel, so a shape carrying only a color
    /// goes through the same pipeline as a sprite. `placeholder` is the
    /// checkered magenta every not-ready texture draws as.
    #[must_use]
    pub fn new(white: BackendTextureId, placeholder: BackendTextureId) -> Self {
        Self {
            white,
            placeholder,
            uploaded: BTreeMap::new(),
        }
    }

    /// Record that `id`'s texels are on the GPU as `backend`.
    pub fn register(&mut self, id: TextureId, backend: BackendTextureId) {
        self.uploaded.insert(id, backend);
    }

    /// Forget `id`, so it draws the placeholder again.
    pub fn forget(&mut self, id: TextureId) {
        self.uploaded.remove(&id);
    }

    /// Whether `id` has real texels behind it.
    #[must_use]
    pub fn is_ready(&self, id: TextureId) -> bool {
        id == TextureId::WHITE || self.uploaded.contains_key(&id)
    }

    /// What to actually sample for `id`.
    ///
    /// Never fails: an unknown id is not an error, it is a texture that has not
    /// arrived. Drawing the placeholder is loud, deterministic, and non-fatal,
    /// which is what a game's first frames need while its art is in flight
    /// (ADR-0011, renderer.md §5).
    #[must_use]
    pub fn resolve(&self, id: TextureId) -> BackendTextureId {
        if id == TextureId::WHITE {
            return self.white;
        }
        self.uploaded.get(&id).copied().unwrap_or(self.placeholder)
    }

    /// The placeholder's backend id.
    #[must_use]
    pub fn placeholder(&self) -> BackendTextureId {
        self.placeholder
    }
}

/// Turn a frame's submissions into a plan.
///
/// Sorts by (`layer`, `z`, submission order) and merges neighbouring quads that
/// sample the same texture. The sort is stable, so quads at equal depth stay in
/// the order they were submitted — that tie-break is the CONTRACT that makes a
/// transcript reproducible (renderer.md §2).
///
/// # Panics
///
/// In debug builds, if a quad's depth or position is not finite. A NaN would
/// sort inconsistently and put a hole in the frame; it is a contract violation
/// in whatever produced it (renderer.md §10).
#[must_use]
pub fn plan_frame(camera: &Camera, quads: &[Quad], textures: &TextureTable) -> FramePlan {
    debug_assert!(
        quads.iter().all(quad_is_finite),
        "[jidousha] a submitted quad has a non-finite position or depth\n  \
         likely cause: a Transform with NaN in it, usually from dividing by zero or normalizing a \
         zero-length vector\n  \
         fix: find the system writing that Transform; the renderer cannot sort what it cannot \
         compare (renderer.md §10)"
    );

    // Sort indices rather than quads: the index is the submission order, and
    // carrying it makes the tie-break explicit rather than a property of which
    // sort algorithm the standard library happens to use.
    let mut order: Vec<usize> = (0..quads.len()).collect();
    order.sort_by(|&left, &right| {
        let (a, b) = (&quads[left], &quads[right]);
        a.depth
            .layer
            .cmp(&b.depth.layer)
            .then_with(|| a.depth.z.total_cmp(&b.depth.z))
            .then_with(|| left.cmp(&right))
    });

    let mut batches: Vec<Batch> = Vec::new();
    for index in order {
        let quad = &quads[index];
        let texture = textures.resolve(quad.texture);
        // A new batch only when the texture changes: the sort already put the
        // quads in draw order, and reordering to merge more would be exactly
        // the cleverness the painter's algorithm forbids.
        let batch = match batches.last_mut() {
            Some(last) if last.texture == texture => last,
            _ => {
                batches.push(Batch {
                    texture,
                    vertices: Vec::new(),
                });
                match batches.last_mut() {
                    Some(batch) => batch,
                    None => unreachable!("a batch was just pushed"),
                }
            }
        };
        append_quad(&mut batch.vertices, quad);
    }

    FramePlan {
        clear_color: camera.clear_color,
        view_projection: camera.view_projection(),
        batches,
    }
}

/// Cut a quad into two triangles, always the same way.
fn append_quad(vertices: &mut Vec<QuadVertex>, quad: &Quad) {
    // Corners wind top-left, top-right, bottom-right, bottom-left, so the
    // triangles are (0,1,2) and (0,2,3) — one diagonal, chosen once.
    for corner in [0, 1, 2, 0, 2, 3] {
        vertices.push(QuadVertex {
            position: quad.corners[corner],
            uv: quad.uvs[corner],
            color: quad.tint,
        });
    }
}

/// Whether a quad can be sorted and drawn at all.
fn quad_is_finite(quad: &Quad) -> bool {
    quad.depth.z.is_finite()
        && quad
            .corners
            .iter()
            .all(|corner| corner.x.is_finite() && corner.y.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jidousha_core::Depth;

    const WHITE: BackendTextureId = BackendTextureId(0);
    const PLACEHOLDER: BackendTextureId = BackendTextureId(1);

    fn table() -> TextureTable {
        TextureTable::new(WHITE, PLACEHOLDER)
    }

    fn quad_at(depth: Depth, texture: TextureId) -> Quad {
        Quad {
            corners: [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
            uvs: [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
            tint: Color::WHITE,
            texture,
            depth,
        }
    }

    #[test]
    fn an_unregistered_texture_draws_the_placeholder() {
        let table = table();
        assert_eq!(table.resolve(TextureId::from_bits(7)), PLACEHOLDER);
        assert!(!table.is_ready(TextureId::from_bits(7)));
    }

    #[test]
    fn a_registered_texture_draws_itself() {
        let mut table = table();
        let id = TextureId::from_bits(7);
        table.register(id, BackendTextureId(42));
        assert_eq!(table.resolve(id), BackendTextureId(42));
        assert!(table.is_ready(id));
    }

    #[test]
    fn a_forgotten_texture_goes_back_to_the_placeholder() {
        let mut table = table();
        let id = TextureId::from_bits(7);
        table.register(id, BackendTextureId(42));
        table.forget(id);
        assert_eq!(table.resolve(id), PLACEHOLDER);
    }

    #[test]
    fn the_white_texture_is_always_ready() {
        // Shapes carrying only a color must draw even before any asset loads.
        assert!(table().is_ready(TextureId::WHITE));
        assert_eq!(table().resolve(TextureId::WHITE), WHITE);
    }

    #[test]
    fn quads_sort_by_layer_then_z_then_submission_order() {
        let quads = vec![
            quad_at(Depth { layer: 1, z: 0.0 }, TextureId::WHITE),
            quad_at(Depth { layer: 0, z: 5.0 }, TextureId::WHITE),
            quad_at(Depth { layer: 0, z: 1.0 }, TextureId::WHITE),
        ];
        let plan = plan_frame(&Camera::default(), &quads, &table());
        // One batch, since they all share a texture; the vertices carry the
        // order. Layer 0 first, and within it z 1 before z 5.
        assert_eq!(plan.batches.len(), 1);
        assert_eq!(plan.quad_count(), 3);
    }

    #[test]
    fn equal_depths_keep_their_submission_order() {
        // The tie-break that makes transcripts reproducible: two quads at the
        // same depth draw in the order the game submitted them.
        let mut first = quad_at(Depth::default(), TextureId::WHITE);
        first.tint = Color::RED;
        let mut second = quad_at(Depth::default(), TextureId::WHITE);
        second.tint = Color::BLUE;
        let plan = plan_frame(&Camera::default(), &[first, second], &table());
        let colors: Vec<Color> = plan.batches[0]
            .vertices
            .iter()
            .map(|vertex| vertex.color)
            .collect();
        assert_eq!(colors[0], Color::RED);
        assert_eq!(colors[VERTICES_PER_QUAD], Color::BLUE);
    }

    #[test]
    fn neighbouring_quads_with_one_texture_share_a_batch() {
        let mut table = table();
        let id = TextureId::from_bits(3);
        table.register(id, BackendTextureId(9));
        let quads = vec![quad_at(Depth::default(), id), quad_at(Depth::default(), id)];
        let plan = plan_frame(&Camera::default(), &quads, &table);
        assert_eq!(plan.batches.len(), 1);
        assert_eq!(plan.batches[0].quad_count(), 2);
    }

    #[test]
    fn a_texture_change_starts_a_new_batch() {
        let mut table = table();
        let (a, b) = (TextureId::from_bits(3), TextureId::from_bits(4));
        table.register(a, BackendTextureId(9));
        table.register(b, BackendTextureId(10));
        let quads = vec![
            quad_at(Depth { layer: 0, z: 0.0 }, a),
            quad_at(Depth { layer: 0, z: 1.0 }, b),
            quad_at(Depth { layer: 0, z: 2.0 }, a),
        ];
        let plan = plan_frame(&Camera::default(), &quads, &table);
        assert_eq!(plan.batches.len(), 3, "depth order wins over batch merging");
    }

    #[test]
    fn two_not_ready_textures_batch_together_as_the_placeholder() {
        // They resolve to the same backend texture, so they merge — which is
        // both correct and the reason a loading screen is cheap to draw.
        let quads = vec![
            quad_at(Depth::default(), TextureId::from_bits(3)),
            quad_at(Depth::default(), TextureId::from_bits(4)),
        ];
        let plan = plan_frame(&Camera::default(), &quads, &table());
        assert_eq!(plan.batches.len(), 1);
        assert_eq!(plan.batches[0].texture, PLACEHOLDER);
    }

    #[test]
    fn a_quad_becomes_two_triangles_the_same_way_every_time() {
        let quad = quad_at(Depth::default(), TextureId::WHITE);
        let plan = plan_frame(&Camera::default(), &[quad], &table());
        let positions: Vec<Vec2> = plan.batches[0]
            .vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect();
        assert_eq!(
            positions,
            vec![
                Vec2::ZERO,
                Vec2::X,
                Vec2::ONE,
                Vec2::ZERO,
                Vec2::ONE,
                Vec2::Y
            ]
        );
    }

    #[test]
    fn an_empty_frame_is_a_plan_with_no_batches() {
        let plan = plan_frame(&Camera::default(), &[], &table());
        assert!(plan.batches.is_empty());
        assert_eq!(plan.quad_count(), 0);
    }

    #[test]
    fn the_clear_color_comes_from_the_camera() {
        let camera = Camera {
            clear_color: Color::rgb(0.1, 0.2, 0.3),
            ..Camera::default()
        };
        let plan = plan_frame(&camera, &[], &table());
        assert_eq!(plan.clear_color, Color::rgb(0.1, 0.2, 0.3));
    }
}
