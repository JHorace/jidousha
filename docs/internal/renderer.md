# Renderer basics — design and contracts

Status: **living doc for `jidousha-render-core` and `jidousha-render-wgpu`; R0
and R1 implemented, R2–R4 still design.** Sections carry `Implemented (R0)` and
`Implemented (R1)` notes where code exists; everything else is design ahead of
the code.
Same conventions as the core doc: **CONTRACT** items are binding and tested.

Inherits: backend seam + WebGL2 envelope (ADR-0003), `DrawCtx` and Draw-phase
typing (ADR-0008), math (ADR-0009), coordinates/color/ordering
(`docs/conventions.md`), error taxonomy (core §9).

In scope (v1): sprites, 2D camera, debug primitives (rect/line/circle), minimal
bitmap text, the backend interface, headless verification. Out of scope
(deferred, each needs a design note or ADR): custom shaders/materials,
render-to-texture, particles, lighting, post-processing, atlas packing tools, 3D.

Debug primitives and text are in scope *for v1* deliberately: agent-generated
prototypes need score text and hitbox visualization on day one, and the
verification story (§9) leans on them.

---

## 1. Architecture

Four layers, top to bottom; each only talks to the next:

```
game Draw systems          fn(&mut DrawCtx) — public API, world-space, submissions
jidousha-render-core       sort → batch → FramePlan; camera; fonts; NO gpu types
backend trait (seam)       ~6 methods, executes a FramePlan it never inspects twice
jidousha-render-wgpu       wgpu implementation (later: -ash for native)
```

CONTRACT: everything above the seam is backend-agnostic and runs headless (the
null backend, §9). All cleverness (sorting, batching, culling if ever) lives in
render-core so backends stay dumb executors — that's what keeps the ash port and
the WebGL2 fallback cheap.

## 2. Submission model

Drawing is **immediate-mode from the game's point of view**: Draw systems submit
world-space draw ops every frame via `DrawCtx`; nothing is retained across frames
at the API level. One mechanism for everything drawn.

```rust
fn draw_game(ctx: &mut DrawCtx) {
    for (_, t, s) in ctx.world.query::<(&Transform, &Sprite)>() {
        ctx.sprite(t, s);
    }
    ctx.rect(bounds, Color::rgba(1.0, 0.0, 0.0, 0.3), layers::DEBUG);
    ctx.text(Vec2::new(-4.0, 3.0), "score: 12", TextStyle::default());
}
```

- `ctx.sprite(&Transform, &Sprite)` — the workhorse. Takes the same component
  types entities carry, so the canonical query-and-submit loop is trivial.
- `ctx.rect / line / circle / text` — primitives and text; same submission
  stream, same ordering rules, no separate "debug renderer" mode.
- The common case is prepackaged: **`jidousha::systems::draw_sprites`** is an
  engine-provided Draw system doing exactly the loop above for all
  `(Transform, Sprite)` entities. Games register it explicitly
  (`app.add_system(Draw, jidousha::systems::draw_sprites)`) — explicit
  registration keeps the schedule readable, and "provided system you register"
  is still one mechanism, not a second path.
- Exact public signatures are finalized in the API pass; shapes above are the
  design intent.

Implemented (R0):

- `ctx.sprite(&Transform, &Sprite)` is real, and arrives through the `Submit`
  extension trait rather than as an inherent method. The sketch above quietly
  assumed `DrawCtx`, `Transform`, and `Sprite` could all see each other; they
  cannot — `DrawCtx` lives in core (ADR-0008) and a sprite names a texture asset
  that core is forbidden to depend on. **ADR-0015** resolves it: core owns the
  sink and an opaque `TextureId`, render-core owns everything that knows what a
  texture is. Games get `Submit` from the prelude and never meet the seam.
- Everything drawn is a `Quad` — four world-space corners, four UVs, a tint, a
  texture, a depth. Sprites expand into one; rectangles, lines, circles, and
  glyphs will expand into more of the same (R3), so sorting and batching never
  grow a second case.
- `draw_sprites` is here and registered explicitly, as designed.
- `rect`, `line`, `circle`, and `text` are **not** here. R0's bullet in §11
  listed transcript tests for text glyph expansion, but the expansion itself —
  and the embedded font it needs — is R3's, and testing an expansion that does
  not exist is not a thing that can be done. §11 is corrected below.

**Ordering.** Per conventions: stable sort by (`layer: i16`, `z: f32`,
submission order). CONTRACT: identical submission streams produce identical
sorted order and identical batches — the submission transcript (§9) is
deterministic and diffable.

**Transparency.** v1 renders everything alpha-blended, back-to-front by the sort
key, no depth buffer — the painter's algorithm, correct for 2D and maximally
simple. PERF-revisit only with evidence.

Implemented (R0): the sort is by (`layer`, `z`, submission index) with the index
compared explicitly rather than relying on the sort being stable — the tie-break
is a CONTRACT, and a contract should not rest on which algorithm the standard
library happens to use. Batching merges only *neighbouring* quads that share a
texture: reordering to merge more would be exactly the cleverness the painter's
algorithm forbids, so a game that interleaves two textures pays a batch each,
and can see that it does.

## 3. Data model

```rust
pub struct Transform {            // component, jidousha-core
    pub pos: Vec2,                // world space, Y-down (ADR-0010)
    pub z: f32,                   // draw depth within layer; higher = on top
    pub rot: Radians,             // clockwise on screen (ADR-0010)
    pub scale: Vec2,              // 1.0 = natural size
}

pub struct Sprite {               // component, jidousha-render-core
    pub texture: TextureHandle,   // from jidousha-assets
    pub region: Option<Rect>,     // atlas sub-rect in texels; None = whole texture
    pub size: Vec2,               // world-unit size of the quad
    pub anchor: Vec2,             // (0,0)=center .. (±0.5,±0.5)=edges; default center
    pub tint: Color,              // multiplied; default WHITE
    pub flip_x: bool, pub flip_y: bool,
    pub layer: i16,
}
```

- `Transform` lives in core (shared with physics-ish gameplay), `Sprite` in
  render-core. Both are plain data with `Default`.

Implemented (R0): `Transform` has a `Default` — the origin, unrotated, natural
size — and `Sprite` does **not**, because there is no meaningful default texture
(ADR-0012: a `Default` must mean something). `Sprite::new(handle)` is the one
constructor, and `..Sprite::new(handle)` is how a game states the rest. `region`
is in normalized 0..1 coordinates rather than texels, since the CONTRACT below
forbids reading texture dimensions — a texel rectangle would require exactly the
lookup that is banned.
- 3D headroom (ADR-0001): `Transform.pos` staying `Vec2`+`z` is a DELIBERATE 2D
  ergonomic choice; the eventual 3D transform is a separate future type + ADR,
  not a hidden third component on this one.
- `size` is in world units, decoupled from texture resolution — swapping art
  never changes gameplay geometry (and keeps sim independent of asset pixel
  data, which matters for determinism: **CONTRACT: nothing in simulation may
  read texture dimensions**; sizes are explicit data).

## 4. Camera

A `Camera` **resource** (one camera in v1; multiple cameras/render-to-texture are
deferred together):

```rust
pub struct Camera {
    pub center: Vec2,             // world position at screen center
    pub height: f32,              // world units spanned vertically; zoom = change height
    pub clear_color: Color,
}
```

- Width follows from height × aspect. **Default windowing behavior: fixed world
  height, width expands with aspect ratio** — prototypes look right on any
  screen without letterboxing logic. (Letterbox mode: deferred until a game
  needs it.)
- `camera.world_to_screen(Vec2) -> Vec2` / `screen_to_world(Vec2) -> Vec2` are
  the ONLY sanctioned space conversions (conventions). The input system uses
  them to deliver pointer positions in world space; games mostly never touch
  screen space at all.
- Draw systems read the camera; render-core builds one view-proj matrix per
  frame from it (rotation via engine `sin_cos`, ADR-0009).

Implemented (R0):

- `Camera` carries a fourth field the sketch does not have: `viewport`, the
  surface size in pixels. Without it `world_to_screen(Vec2) -> Vec2` cannot be
  written — the aspect ratio has to come from somewhere. The driver maintains it
  (the platform crate will write it on resize); the default 1280×720 gives a
  headless run a definite aspect, which is what keeps a transcript identical
  between a test and a windowed run of the same size.
- The projection matrix is written out by hand rather than calling glam's
  `orthographic_*` helpers, which are deprecated as of glam 0.33 and moving.
  Six divisions we own beat an upstream API whose depth convention could change
  under us — the same reasoning ADR-0009 applies to trigonometry.
- A zero-sized viewport (a minimized window) reports an aspect of 1.0 and
  `screen_to_world` returns the camera center, rather than dividing by zero.
  A wrong answer nobody can see beats a NaN that spreads into gameplay.

## 5. Textures and the asset boundary

- `TextureHandle` is issued by `jidousha-assets` (its design doc owns loading,
  formats, and lifetimes; renderer consumes handles and pixel data).
- **Not-ready policy (the no-silent-failure rule meets async assets):** drawing
  a sprite whose texture is still loading renders the built-in **checkered
  magenta placeholder** — loud, deterministic, non-fatal (assets are legitimately
  in flight during the first frames; panicking would make every game's startup a
  race). A *failed* texture also renders the placeholder and emits one
  structured §9 error (once per asset, not per frame). A magenta screen is an
  agent-visible, screenshot-visible, transcript-visible signal.
- CONTRACT: placeholder rendering is bit-identical across backends (it's a
  built-in embedded texture, not backend-generated).

Implemented (R0): the policy is one line of `TextureTable::resolve` — an id
nobody registered resolves to the placeholder. That covers both cases without
asking anyone: a texture still loading was never registered, and one that failed
never will be. Nothing in the draw path needs to know which, and no code path
exists that could be wrong about it. The single structured error for a *failed*
asset already comes from `Assets::commit` (A0), which reports each failure
exactly once. The embedded placeholder texels arrive with R2, where there is a
GPU to upload them to; R0's `TextureTable` takes the two built-in ids as
arguments so the policy is testable today.

## 6. Text (minimal, v1)

- One embedded monospace bitmap font (a compiled-in atlas; zero asset
  dependencies, works before any asset loads, works headless).
- `ctx.text(pos, &str, TextStyle { size_world_units, color, layer, z })` expands
  to glyph quads through the standard sprite path — no separate pipeline.
- Explicit non-goals for v1: TTF rendering, shaping, wrapping, non-ASCII beyond
  Latin-1. Real typography is a future subsystem; this is for scores, debug
  readouts, and prototype UI.

## 7. Backend interface (the seam)

Engine-defined, 2D-specific, deliberately narrow. Sketch (final shape may add a
method or two, but growth beyond ~8 methods is a design smell to resist):

```rust
pub trait RenderBackend {
    fn create_texture(&mut self, desc: &TextureDesc, texels: &[u8]) -> BackendTextureId;
    fn destroy_texture(&mut self, id: BackendTextureId);
    fn resize_surface(&mut self, size: PhysicalSize);
    fn render(&mut self, plan: &FramePlan) -> Result<(), RenderError>;
    fn capture(&mut self) -> Result<RawImage, RenderError>;   // offscreen readback, §9
}
```

- `FramePlan` (built by render-core, plain data): clear color, view-proj matrix,
  and an ordered list of `Batch { texture: BackendTextureId, vertices: &[QuadVertex] }`.
  Quads are pre-expanded on CPU into world-space vertex data; the GPU applies
  view-proj and samples. One pipeline ("sprite"), one vertex format, v1.
- CPU quad expansion + one dynamic vertex buffer per frame + one draw call per
  batch is the v1 strategy: WebGL2-safe (no instancing dependency), trivially
  portable to ash, and comfortably fast for prototype-scale scenes
  (PERF-revisit with evidence, not speculation).
- CONTRACT (ADR-0003): `wgpu` appears only in `jidousha-render-wgpu`; `FramePlan`
  and everything in it are engine types. Shader source (WGSL today, SPIR-V under
  ash) is a backend-internal detail — render-core requests the "sprite pipeline"
  by name, never by source.

## 8. WebGL2 envelope, concretely (ADR-0003 §4)

Hard limits all rendering must respect until the envelope ADR is revisited:

- No compute shaders, no storage buffers, no instanced-only paths.
- Texture max 2048×2048 (safe floor across old mobile/WebGL2 devices;
  doctor-checkable). Power-of-two not required (WebGL2 core).
- Uniform data fits one small UBO (view-proj + little else).
- sRGB: framebuffer encoding differences between WebGPU/WebGL2 are the wgpu
  backend's problem to normalize; CONTRACT: the same FramePlan produces
  visually identical output on both paths (golden-image tolerance, §9).

## 9. Verification (the point of all this)

Two tiers, cheap one first:

**Implemented (R0):** tier 1 in full — `NullBackend` records every `FramePlan`,
`FrameRecord::covering(world)` answers "what is at this point?" with exact
rotated-quad containment rather than a bounding box, and `transcript()` renders
a frame as stable, diffable text. `tests/transcript.rs` covers ordering,
batching, the placeholder policy, and the camera round trip; `tests/plan_model.rs`
checks sort and batch against a naive reference under 2000 random streams.
`examples/what_was_drawn.rs` is the loop end to end. Tier 2 (golden images) needs
a GPU and lands with R4.

1. **Submission transcripts** (primary, all targets, no GPU): the null backend
   records every `FramePlan` as structured data. Tests and `tools/verify` assert
   on it: what was drawn, where, what order, what batches. Combined with camera
   math this answers "is entity E on screen?", "does sprite A overlap B
   visually?", "what's at world point P?" — **agent-answerable visual questions
   without rendering a pixel.** Golden-transcript tests are ordinary snapshot
   tests: deterministic, diffable text.
2. **Golden images** (secondary, native CI): the wgpu backend's `capture()`
   renders offscreen and reads back pixels; tests compare against checked-in
   references with a small tolerance (GPU rasterization varies slightly across
   drivers — exact-match is a flake factory; transcripts are the exact tier).
   Keeps the *backend* honest the way transcripts keep *render-core* honest.

**Gap, recorded rather than resolved (R1):** there is no web harness anywhere in
the repository — no `index.html`, no `wasm-bindgen` invocation, no serve step.
`cargo check --target wasm32-unknown-unknown` gates every merge and proves the
engine *compiles* for the web, which is what ADR-0005 asked for and is not
nothing; but nothing turns that into something a browser can load. Every
milestone from here whose exit criterion says "on all three targets" is
therefore only checkable on two. This is not assigned to a milestone. It is
small — a `tools/serve-web` and a page — and it should land before R2 makes the
claim a third time.

`tools/verify <example>` composes both with headless simulation (core §8):
run N ticks with scripted input, then assert on world state + transcript, and
optionally capture a frame for human/agent eyeballs. This tool is the game-agent
feedback loop and gets built incrementally from R0 (see also asset/input docs).

## 10. Errors (core §9 taxonomy applied)

- No adapter/device at startup → `Result` from `jidousha::run` with a §9 message
  (likely cause: missing drivers/headless env; fix: doctor hints, try
  `JIDOUSHA_BACKEND=gl` fallback).

Implemented (R1): the adapter and device are asked for asynchronously, so their
failures cannot come back from `run` — it has already returned to the event
loop by then. They surface from `render` instead, as `RenderError::Unsupported`
naming what wgpu said. A frame that cannot be drawn is reported and skipped
rather than fatal: a lost surface usually comes back, and quitting a game
because one frame failed is worse than missing one frame. `JIDOUSHA_BACKEND=gl`
does not exist yet; wgpu picks a backend itself, and an override is worth adding
when someone needs it rather than before.
- Device lost mid-run → v1: fatal with §9 message (recreation is deferred).
- Oversized texture, NaN transform, unknown handle → contract violations: debug
  panic naming the entity/system per core §9.

## 11. Milestones

Continue from core M5 (windowed blank via platform crate). Same rules: each is
mergeable, tested, green CI on all three targets.

- **R0 — render-core, no GPU.** ✅ `DrawCtx` submission sink, sprite expansion,
  the camera, sort/batch into `FramePlan`, null backend, transcript snapshot
  tests (ordering, batching, placeholder policy). Runs on wasm CI too — no GPU
  needed. Exit: transcripts green everywhere; `verify` can assert "sprite at P".

  Corrected from the original bullet: **text glyph expansion moved to R3**,
  which is where the expansion and its embedded font already were. R0 could not
  test an expansion that does not exist yet.

  What the mutation checks said. Fourteen deliberate breakages, all caught:
  dropping the layer from the sort, dropping z, reversing the submission-order
  tie-break, batching regardless of texture, never batching, resolving a
  not-ready texture to white instead of the placeholder, winding the quad into
  overlapping triangles, flipping the anchor's sign, ignoring `flip_x`, dropping
  the transform's rotation, un-flipping the projection's Y, falling back to a
  bounding box for containment, keeping submissions across frames, and rotating
  before scaling. The reference-model test caught the sorting and batching ones;
  the transcript tests caught the geometry; and the two that only the unit tests
  caught — the winding and the Y flip — are the ones a transcript cannot see,
  because both produce a frame that is wrong only once a GPU rasterizes it.
  That is the honest limit of tier 1, and the reason tier 2 exists.
- **R1 — wgpu clear + present.** ✅ Surface init on Linux/Windows/web, clear color,
  resize handling. `examples/window_clear.rs`. Exit: colored window on all
  three targets (web manually verified; native CI headless-runs it).

  `WgpuBackend` implements render-core's `RenderBackend` and is opaque: no wgpu
  type appears in its public API, so ADR-0003's isolation rule holds by
  construction rather than by discipline. The platform crate is the composition
  root that picks it — the one place naming a concrete backend — and wires the
  frame path end to end: submissions → `plan_frame` → `backend.render`. R2 adds
  a pipeline and nothing else changes shape.

  **Getting a GPU without an async runtime.** wgpu's adapter and device requests
  are futures, and the engine has no executor. Rather than `pollster` on native
  and `wasm-bindgen-futures` on the web — two implementations of one thing, only
  one of which could ever be tested here — the backend polls those futures from
  the frame loop with a no-op waker, and reports "not ready yet" until they
  land. This is ADR-0011's answer applied to the GPU: a game loop expresses
  "ask again next frame" by being a loop. It is ten lines and no dependency, and
  a test polls a *real* wgpu adapter request to prove the mechanism reaches an
  answer rather than staying `Pending` forever — the failure that would hang
  every game's first frames.

  Two bugs the writing caught, both about the gap between asking and arriving: a
  window resized while the GPU is still coming had its resize dropped, so the
  surface configured at the startup size (a tiling compositor resizes every new
  window, so this was not hypothetical); and the adapter was consumed by the
  handshake and never kept, so `resize_surface` could never reconfigure at all.

  **Verified, and not.** The isolation rule, the poll mechanism, and both target
  builds are checked, and `tools/test` builds `window_clear` without running it.
  **The colored window itself is unverified** — this environment has no display
  *and no GPU adapter at all* (the probe reports "no suitable graphics adapter
  found"), so nothing here can see a pixel. Native and web both need a human
  with a screen. The web additionally has **no harness at all**: the wasm build
  compiles, but there is no `index.html`, no `wasm-bindgen` step, and nothing to
  serve — so "colored window on the web" is not merely unverified, it is
  currently unreachable. That gap is called out in §9 below and belongs to
  whoever picks it up; it was never assigned to a milestone.
- **R2 — sprites end to end.** Texture upload, sprite pipeline (WGSL), batching,
  camera UBO, `jidousha::systems::draw_sprites`, placeholder texture.
  `examples/sprites.rs` (moving, rotating, tinted, atlas-region sprites).
  Exit: R0 transcripts unchanged (render-core untouched proves the seam);
  sprites visible on all targets.
- **R3 — primitives + text.** rect/line/circle expansion, embedded font,
  `examples/prototype_kit.rs` (sprites + shapes + score text — the "can an
  agent make Pong?" substrate is now complete). Exit: transcript tests for all
  primitive expansion; example runs everywhere.
- **R4 — capture + golden images + verify integration.** Offscreen `capture()`
  in the wgpu backend, tolerance-based golden tests in native CI, `tools/verify`
  wired: headless sim ticks + transcript assertions + optional frame capture.
  Exit: a `verify` run on `prototype_kit` asserts text on screen, sprite
  positions after N ticks, and produces a captured PNG artifact.

## 12. Deferred (tracked, not designed)

Custom shaders/materials · render-to-texture & multiple cameras · particles ·
lighting/post · letterbox mode · TTF text · atlas packing tooling (assets doc
may claim it) · instancing/perf work (needs evidence) · 3D bridge (future ADR).
