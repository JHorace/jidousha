//! The Jidousha engine facade — the entire public surface a game may use.
//!
//! Key modules: [`prelude`] (one import, everything a game needs), [`math`],
//! [`testing`] (headless verification vocabulary).
//! Depends on: every other jidousha crate. Must never be depended on by: any of
//! them.
//! INVARIANT (public-api.md §1, CONTRACT): `docs/api/` is generated from THIS
//! crate only, and games depend on `jidousha` and nothing else. Anything not
//! re-exported here is not public API. `#[doc(hidden)]` is banned — a hidden
//! public API is how a surface rots.
//! INVARIANT: this is a **curation, not a re-export dump**. Every internal crate
//! exports more than this: driver plumbing, backend seams, codec internals. What
//! reaches this file is what a game has a reason to name.
//!
//! ```
//! use jidousha::prelude::*;
//!
//! #[derive(Clone, Copy)]
//! struct Velocity(Vec2);
//! impl Component for Velocity {}
//!
//! fn drift(world: &mut World) {
//!     let step = world.resource::<Time>().fixed_dt.as_f32();
//!     for (_, transform, velocity) in world.query_mut::<(&mut Transform, &Velocity)>() {
//!         transform.pos += velocity.0 * step;
//!     }
//! }
//!
//! let mut sim = headless(GameConfig::default(), |app| {
//!     app.add_system(Update, drift);
//! });
//! let entity = sim.world_mut().spawn();
//! sim.world_mut().insert(entity, Transform::at(Vec2::ZERO));
//! sim.world_mut().insert(entity, Velocity(Vec2::new(3.0, 0.0)));
//!
//! for _ in 0..60 {
//!     sim.tick();
//! }
//! let moved = sim.world().component::<Transform>(entity).pos;
//! assert!(moved.x > 2.9 && moved.x < 3.1, "a second at three units a second");
//! ```

// --- App and lifecycle ------------------------------------------------------
pub use jidousha_core::{App, Draw, GameConfig, HeadlessSim, Startup, Update, headless};
pub use jidousha_platform::{RunError, asset_source, run};

// --- ECS --------------------------------------------------------------------
pub use jidousha_core::{
    Bundle, Commands, Component, DrawCtx, Entity, Resource, With, Without, World, WorldView,
};

// --- Math and primitives ----------------------------------------------------
pub use jidousha_core::math;
pub use jidousha_core::{
    Color, Depth, EntityDeadError, Quad, Rect, Rng, Seconds, TextureId, Time, message,
};

// --- Render -----------------------------------------------------------------
pub use jidousha_core::Transform;
pub use jidousha_render_core::{Camera, PhysicalSize, Sprite, Submit, TextStyle, draw_sprites};

// --- Assets -----------------------------------------------------------------
pub use jidousha_assets::{
    AssetError, AssetFailure, AssetStatus, Assets, BytesHandle, MemorySource, TextureHandle,
};

// --- Input ------------------------------------------------------------------
pub use jidousha_input::{Input, Key, PointerButton, PointerId, PointerState};

/// One import, and a game has everything.
///
/// `use jidousha::prelude::*;` is the way. The modules above exist for a human
/// browsing the documentation; nothing is only reachable through them.
///
/// [`Submit`] is in here and is load-bearing: it is the trait that carries
/// `ctx.sprite(...)`, `ctx.rect(...)`, `ctx.line(...)`, `ctx.circle(...)` and
/// `ctx.text(...)`, and without it in scope none of them resolve. The drawing
/// vocabulary lives on a trait rather than on `DrawCtx` so that the renderer can
/// add to it without core knowing what a sprite is (public-api.md §2).
pub mod prelude {
    pub use crate::math::{Radians, Vec2, Vec3, atan2, rotate, sin_cos};
    pub use crate::{
        App, AssetError, AssetFailure, AssetStatus, Assets, Bundle, BytesHandle, Camera, Color,
        Commands, Component, Depth, Draw, DrawCtx, Entity, EntityDeadError, GameConfig,
        HeadlessSim, Input, Key, MemorySource, PhysicalSize, PointerButton, PointerId,
        PointerState, Quad, Rect, Resource, Rng, RunError, Seconds, Sprite, Startup, Submit,
        TextStyle, TextureHandle, TextureId, Time, Transform, Update, With, Without, World,
        WorldView, asset_source, draw_sprites, headless, message, run,
    };
}

/// Verifying a game without opening a window (renderer.md §9, input.md §5).
///
/// The engine's thesis in one module: script the input, run N headless ticks,
/// and assert on world state *and* on what was drawn. Everything here is for a
/// test or a `--verify` mode; a game that shipped against it would be shipping
/// its test harness.
///
/// ```
/// use jidousha::prelude::*;
/// use jidousha::testing::{InputScript, NullBackend, create_builtin_textures, plan_frame};
///
/// fn move_it(world: &mut World) {
///     let Some(input) = world.find_resource::<Input>() else { return };
///     let step = f32::from(input.held(Key::D)) - f32::from(input.held(Key::A));
///     for (_, transform) in world.query_mut::<&mut Transform>() {
///         transform.pos.x += step;
///     }
/// }
///
/// let mut sim = headless(GameConfig::default(), |app| {
///     app.add_system(Update, move_it);
/// });
/// let entity = sim.world_mut().spawn();
/// sim.world_mut().insert(entity, Transform::at(Vec2::ZERO));
///
/// let script = InputScript::new().hold(Key::D, 1..6);
/// for tick in 1..=5 {
///     sim.world_mut().insert_resource(Input::new(script.snapshot_at(tick)));
///     sim.tick();
/// }
/// assert_eq!(sim.world().component::<Transform>(entity).pos.x, 5.0);
/// ```
pub mod testing {
    // Scripting what a load returns, and when. `decode_png` turns real bytes
    // into the `TextureData` a `MemorySource` hands back, so a test can bake a
    // picture into its binary and script the tick it arrives on.
    pub use jidousha_assets::{MemorySource, ReplaySource, TextureData, decode_png};
    pub use jidousha_input::{
        AssetReady, Input, InputScript, InputSnapshot, Recording, RecordingError, TickRecord,
    };
    // `encode_png` here takes a captured frame (`RawImage`), which is what a
    // golden image or a `tools/verify` artifact is written from. Its inverse
    // for that type lives in render-core and has no caller outside the golden
    // tests, so it is not part of this surface until something needs it.
    pub use jidousha_render_core::{
        BackendTextureId, Comparison, DrawnQuad, FONT_TEXTURE, FramePlan, FrameRecord, NullBackend,
        PhysicalSize, RawImage, RenderBackend, TextureTable, Tolerance, compare,
        create_builtin_textures, diff_image, encode_png, plan_frame, upload_ready_textures,
    };

    /// The renderer a golden image comes from.
    ///
    /// The one item in this whole surface that names a backend, and it is here
    /// because a picture has to be drawn by something: `WgpuBackend::offscreen`
    /// renders into a texture with no window anywhere, and `capture` reads the
    /// pixels back (renderer.md §9).
    ///
    /// This does not breach ADR-0003. What that decision forbids is `wgpu`
    /// *types* escaping the backend crate, and none do — `WgpuBackend` is
    /// opaque and every argument and return value it has is an engine type. A
    /// game still never names it; it appears in `testing` and nowhere else.
    pub use jidousha_render_wgpu::WgpuBackend;
}
