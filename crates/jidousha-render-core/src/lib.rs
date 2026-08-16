//! Draw submissions, sorting, and the backend trait every render backend implements.
//!
//! Key types: `Camera`, `Sprite`, `Submit`, `FramePlan`, `TextureTable`,
//! `RenderBackend`, `NullBackend`.
//! Depends on: `jidousha-core`, `jidousha-assets`. Must never be depended on by:
//! `jidousha-core`.
//! INVARIANT: contains no backend-specific types; the `wgpu`→`ash` swap must be
//! invisible from here (ADR-0003).
//!
//! Built so far (`docs/internal/renderer.md` §11): R0 — sprite submission, the
//! camera, sort and batch into a `FramePlan`, and the null backend that records
//! frames instead of drawing them; R2 — the built-in textures and the upload
//! loop that hands loaded art to whichever backend is there. The debug
//! primitives and text (R3) and golden images (R4) land next.
//!
//! ```
//! use jidousha_core::{Draw, GameConfig, Transform, headless, math::Vec2};
//! use jidousha_render_core::{Camera, NullBackend, RenderBackend, Sprite, TextureTable,
//!     BackendTextureId, draw_sprites, plan_frame};
//! use jidousha_assets::{Assets, MemorySource};
//!
//! let mut source = MemorySource::new();
//! source.insert("ship.png", vec![0]);
//! let mut assets = Assets::new(source);
//! let ship = assets.load_texture("ship.png");
//!
//! let mut sim = headless(GameConfig::default(), |app| {
//!     app.add_system(Draw, draw_sprites);
//! });
//! sim.world_mut().insert_resource(Camera::default());
//! let entity = sim.world_mut().spawn();
//! sim.world_mut().insert(entity, Transform::at(Vec2::new(2.0, 0.0)));
//! sim.world_mut().insert(entity, Sprite::new(ship));
//! sim.tick();
//!
//! // Draw, plan, and record — no GPU anywhere in this.
//! let quads: Vec<_> = sim.draw().quads().to_vec();
//! let camera = *sim.world().resource::<Camera>();
//! let textures = TextureTable::new(BackendTextureId(0), BackendTextureId(1));
//! let mut backend = NullBackend::new();
//! backend.render(&plan_frame(&camera, &quads, &textures)).expect("the null backend never fails");
//!
//! let frame = backend.last_frame().expect("one frame was drawn");
//! assert_eq!(frame.quad_count(), 1);
//! assert_eq!(frame.covering(Vec2::new(2.0, 0.0)).len(), 1, "the ship is where we put it");
//! ```

mod backend;
mod camera;
mod null;
mod plan;
mod sprite;
mod submit;
mod textures;

pub use backend::{
    BackendTextureId, PhysicalSize, RawImage, RenderBackend, RenderError, TextureDesc,
};
pub use camera::Camera;
pub use null::{DrawnQuad, FrameRecord, NullBackend};
pub use plan::{Batch, FramePlan, QuadVertex, TextureTable, plan_frame};
pub use sprite::Sprite;
pub use submit::{Submit, draw_sprites};
pub use textures::{create_builtin_textures, upload_ready_textures};
