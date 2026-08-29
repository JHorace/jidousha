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
// `Submissions` is here because `HeadlessSim::draw` returns one, and a return
// type named in a signature and defined nowhere is the one kind of gap a reader
// cannot work around (e0-findings.md F-017). A game reaches it as
// `sim.draw().quads()` and never writes the name.
pub use jidousha_core::{
    App, Draw, GameConfig, HeadlessSim, Startup, Submissions, Update, headless,
};
pub use jidousha_platform::{RunError, asset_source, run};

// --- ECS --------------------------------------------------------------------
pub use jidousha_core::{
    Bundle, Commands, Component, DrawCtx, Entity, Resource, With, Without, World, WorldView,
};

// --- Math and primitives ----------------------------------------------------
// Re-exported as a module so a reader can browse it, and *every name in it* is
// also in the prelude. A game therefore writes `use jidousha::prelude::*;` and
// nothing else; `use jidousha::math::sin_cos;` beside that glob imports the same
// item a second time, which is what two worked examples were doing when E0 run 4
// concluded there was no rule (e0-findings.md F-045).
pub use jidousha_core::math;
pub use jidousha_core::{
    Color, Depth, EntityDeadError, Quad, Rect, Rng, Seconds, TextureId, Time, message,
};

// --- Render -----------------------------------------------------------------
pub use jidousha_core::Transform;
pub use jidousha_render_core::{
    Camera, Face, FontError, Fonts, PhysicalSize, Sprite, Submit, TextExtents, TextStyle,
    draw_sprites,
};

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
        Commands, Component, Depth, Draw, DrawCtx, Entity, EntityDeadError, Face, FontError, Fonts,
        GameConfig, HeadlessSim, Input, Key, MemorySource, PhysicalSize, PointerButton, PointerId,
        PointerState, Quad, Rect, Resource, Rng, RunError, Seconds, Sprite, Startup, Submissions,
        Submit, TextExtents, TextStyle, TextureHandle, TextureId, Time, Transform, Update, With,
        Without, World, WorldView, asset_source, draw_sprites, headless, message, run,
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
/// use jidousha::testing::{FrameRecorder, InputScript};
///
/// fn move_it(world: &mut World) {
///     let Some(input) = world.find_resource::<Input>() else { return };
///     let step = f32::from(input.held(Key::D)) - f32::from(input.held(Key::A));
///     for (_, transform) in world.query_mut::<&mut Transform>() {
///         transform.pos.x += step;
///     }
/// }
///
/// fn draw_it(ctx: &mut DrawCtx) {
///     for (_, transform) in ctx.world.query::<&Transform>() {
///         let box_of_it = Rect::from_center_size(transform.pos, Vec2::ONE);
///         ctx.rect(box_of_it, Color::WHITE, Depth::layer(0));
///     }
/// }
///
/// let mut sim = headless(GameConfig::default(), |app| {
///     app.add_system(Update, move_it);
///     app.add_system(Draw, draw_it);
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
///
/// // And what was drawn, which is the other half: one recorder, one `draw`.
/// let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));
/// let frame = recorder.draw(&mut sim);
/// assert_eq!(frame.quads().len(), 1);
/// assert_eq!(frame.quads()[0].bounds().center().x, 5.0);
/// ```
pub mod testing {
    // Scripting what a load returns, and when. `decode_png` turns real bytes
    // into the `TextureData` a `MemorySource` hands back, so a test can bake a
    // picture into its binary and script the tick it arrives on.
    pub use jidousha_assets::{MemorySource, ReplaySource, TextureData, decode_png};
    // `DecodeError` is named by `InputSnapshot::try_decode` and carried inside
    // `RecordingError::Snapshot`, so it was already in the reference twice
    // without an entry of its own (e0-findings.md F-017).
    //
    // `SnapshotBuilder` and `InputEvent` are the closed-loop half of scripted
    // input: `InputScript` answers "what happens on tick N of a plan fixed in
    // advance", and a check that has to *see the game* before deciding what to
    // press has no such plan. They are the driver's own edge rules, so a
    // controller written this way is exercising the path a real keyboard takes
    // rather than a second one (ADR-0019).
    pub use jidousha_input::{
        AssetReady, DecodeError, Input, InputEvent, InputScript, InputSnapshot, Recording,
        RecordingError, SnapshotBuilder, TickRecord,
    };
    // `encode_png` here takes a captured frame (`RawImage`), which is what a
    // golden image or a `tools/verify` artifact is written from. Its inverse
    // for that type lives in render-core and has no caller outside the golden
    // tests, so it is not part of this surface until something needs it.
    // `FrameRecorder` is the way to ask what a game drew: it owns the backend,
    // the texture table and the plan, so a `--verify` mode says `draw(&mut sim)`
    // once instead of writing the driver's five steps out and then rebuilding
    // the texture table against a throwaway backend to learn which id the font
    // got (e0-findings.md F-010). It is now the *only* way in: the hand-driven
    // pieces — `NullBackend`, `plan_frame`, and the golden-image comparison
    // vocabulary `compare`/`Comparison`/`Tolerance`/`diff_image` — left this
    // surface with ADR-0028. They were here because `prototype_kit` drove a
    // backend by hand to buy one claim about the engine, that claim is now a
    // test, and nothing a *game* writes ever named them: the testing document
    // never mentioned one of the six, and `check-api-coverage` skips this
    // module, so they were exported and taught nowhere for four milestones.
    // The engine's own crates still have them; they are simply not a game's.
    // `create_builtin_textures`, `upload_ready_textures` and `TextureTable` do
    // stay, because a capture replays a recorded plan and a plan names texture
    // ids — a game with art has to create the built-ins and upload its own art
    // in the same order before the ids mean the same thing
    // (`examples/prototype_kit/capture.rs`).
    // `Batch` and `QuadVertex` are here for the same reason `Submissions` is
    // (e0-findings.md F-017): `FramePlan::batches` is a `Vec<Batch>` and
    // `Batch::vertices` a `Vec<QuadVertex>`, so both were named by a field a
    // check reads and defined nowhere. A check that counts glyphs off a plan —
    // `plan.batches.iter().filter(|batch| batch.texture == font)` — needs to
    // know a batch has a texture and a `quad_count`; `FrameRecorder` is still
    // the shorter road and the one a game should take.
    // `RenderError` is F-017's shape a third time — named by every
    // `RenderBackend` signature here and defined nowhere in this surface — and
    // it is exported for a reason a check can act on rather than only for
    // completeness (e0-findings.md F-067). A capture path has to tell **no
    // adapter on this machine** from a fault: the first is a fact about the
    // runner and the run stays green, the second is a real problem and
    // reporting it as "no GPU here" files an engine bug as a property of the
    // hardware. The distinction is `RenderError::NoAdapter`, and matching on it
    // needs the type. `examples/prototype_kit/capture.rs` is the worked case.
    // `find_bounds` is the fold every check that measures a drawn thing was
    // writing out by hand: a circle is sixteen wedges and a string is one quad
    // per character, so "how big is the thing that was drawn" is never a quad
    // anybody drew (e0-findings.md F-116, ADR-0032).
    pub use jidousha_render_core::{
        BackendTextureId, Batch, DrawnQuad, FONT_TEXTURE, FramePlan, FrameRecord, FrameRecorder,
        PhysicalSize, QuadVertex, RawImage, RenderBackend, RenderError, TextureTable,
        create_builtin_textures, encode_png, find_bounds, upload_ready_textures,
        upload_text_atlases,
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
