# Renderer basics — design and contracts

Status: **living doc for `jidousha-render-core` and `jidousha-render-wgpu`;
R0–R4 implemented — the v1 renderer is complete.** Sections carry `Implemented (RN)` notes
where code exists; everything else is design ahead of the code.
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

Implemented (R2):

- `create_builtin_textures(backend)` uploads the white texel and the
  placeholder and returns the table naming them. The ids come back inside the
  table rather than being assumed to be 0 and 1 — an assumption that is true
  only for as long as nothing else is created first, and the driver was making
  it.
- The placeholder is 16×16 with 4-texel checks: magenta against black, four
  checks across. Two large checks can pass for art at a glance and sixteen small
  ones turn to mush when scaled down; four reads as "this is wrong" at every
  size. The texels are a constant in render-core, which is what makes the
  bit-identical CONTRACT above true by construction rather than by discipline —
  and `NullBackend` now records the texels it was handed, so a test can check it.
- `upload_ready_textures(assets, backend, table)` is the once-a-frame loop that
  moves everything newly loaded onto the GPU. The texels are **moved** out of the
  asset store rather than borrowed (ADR-0016).
- **Sampling is nearest-neighbour**, with clamped addressing and no mipmaps.
  Prototype 2D art is pixel art far more often than not, and linear filtering
  turns it to mush at every scale but 1:1 — "why is my sprite blurry" is the
  first complaint a 2D engine earns. It also gives R4's golden images far less
  room for drivers to disagree. Revisit when a game wants smooth scaling, and the
  shape of that change is a per-texture choice rather than a different global
  default.

## 6. Text (minimal, v1)

- One embedded monospace bitmap font (a compiled-in atlas; zero asset
  dependencies, works before any asset loads, works headless).
- `ctx.text(pos, &str, TextStyle { size, color, depth })` expands to glyph
  quads through the standard sprite path — no separate pipeline. (This sketch
  previously said `{ size_world_units, color, layer, z }`, which predates
  public-api.md §3 making `Depth` the one depth argument every immediate
  primitive takes. Corrected at R3, in favour of the higher-precedence doc.)
- Explicit non-goals for v1: TTF rendering, shaping, wrapping, non-ASCII beyond
  Latin-1. Real typography is a future subsystem; this is for scores, debug
  readouts, and prototype UI.

Implemented (R3):

- **The font is 5×7, and the source is the picture.** `font.rs` holds one line
  per character — the character itself, then seven rows of five — and nothing
  generates it from anything else. To change a glyph you change its shape. The
  character is written out at the start of each line and checked against its
  ASCII code, so a deleted line is a loud failure rather than a font silently
  shifted by one.
- Printable ASCII, 32 through 126, plus a **fallback box** for everything else.
  A character the font does not have draws the box rather than nothing, for the
  same reason a missing texture draws the placeholder: "half my text is
  missing" should be a picture, not a mystery.
- Cells are 7×9 — the glyph plus a one-texel transparent border on every side.
  The border does two jobs: it is the letter spacing, and it is what makes
  nearest sampling safe at any scale, because a fragment landing a hair outside
  a glyph finds its neighbour's border instead of its neighbour's ink.
- `TextStyle::size` is the height of one **line**, in world units, so text
  scales with the camera like everything else. `TextStyle::width_of` measures a
  string exactly (monospace, no kerning) — without it a game cannot centre a
  score, and guessing is what makes prototype UI look wrong.
- `\n` starts a new line. Wrapping remains the non-goal; an explicit line break
  is three lines of code and is what a multi-line debug readout needs.
- The atlas is registered in the `TextureTable` like any loaded texture, under
  an id reserved below `1 << 32`. Asset ids pack a generation of at least one
  into their high half, so the whole low range belongs to the renderer's own
  textures and can never collide.

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

Implemented (R2): the pipeline is one WGSL shader, one vertex format (position,
uv, color — eight floats, 32 bytes), one uniform buffer holding the view-proj
matrix, one growable vertex buffer, and one `draw` call per batch, in plan
order. Three things worth naming:

- **Vertices are packed by hand**, field by field, rather than cast with
  `bytemuck`. The layout is then stated once in the same order the
  `VertexAttribute` list declares it, so the two can be read against each other
  — and a stride mismatch is the kind of bug that draws *something*, just not
  what was asked for. It also costs no dependency and no `repr(C)` promise on a
  type belonging to another crate.
- **No face culling.** A negative `Transform::scale` mirrors a sprite, which
  reverses its winding; with culling on, a game that flipped a character by
  scaling it by −1 would watch it disappear.
- **Colors are linearized on the CPU**, in one function, used by both the clear
  color and every vertex. The engine's `Color` is sRGB-encoded (conventions) and
  the surface is an `-srgb` format, so the conversion has to happen somewhere;
  doing it in WGSL would put the same curve in two languages, which must agree
  and cannot be compared. Interpolating linear colors across a triangle is also
  the more correct of the two. R1's clear skipped the conversion entirely, which
  is why a grey window looked washed out — that is fixed here.

## 8. WebGL2 envelope, concretely (ADR-0003 §4)

Hard limits all rendering must respect until the envelope ADR is revisited:

- No compute shaders, no storage buffers, no instanced-only paths.
- Texture max 2048×2048 (safe floor across old mobile/WebGL2 devices;
  doctor-checkable). Power-of-two not required (WebGL2 core).
- Uniform data fits one small UBO (view-proj + little else).
- sRGB: framebuffer encoding differences between WebGPU/WebGL2 are the wgpu
  backend's problem to normalize; CONTRACT: the same FramePlan produces
  visually identical output on both paths (golden-image tolerance, §9).

Implemented (web harness): the web path is **WebGL2 only** for now — see §9's
note on what forced that. The CONTRACT above is therefore untested rather than
false: there is currently one web path, not two, so nothing yet compares them.
It becomes testable when WebGPU is enabled on the web, and R4's golden images
are what would compare them.

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

**Implemented (R4):**

- **`WgpuBackend::offscreen(size)`** is a second constructor, not a mode: it
  makes a backend with a texture instead of a surface. Everything after the
  target is created is the same code the window runs — same pipeline, same
  shader, same uploads — which is what makes a capture evidence about the
  backend rather than about a second rendering path.
- **A windowed backend refuses to capture**, naming `offscreen` in the refusal.
  A presented surface texture is gone, and keeping a readable copy would mean a
  full-screen blit on every frame of every game to serve a feature only tests
  use. `DELIBERATE:` at the site.
- **`capture()` blocks**, on a buffer map, and is native-only by construction —
  the web has no equivalent wait. It is the only place in the engine that
  blocks, and a game never calls it. On the web the check that a frame reached
  the screen is `tools/serve-web --check`, which asks from outside.
- **The row padding is stripped in one place.** wgpu wants every row of a
  texture-to-buffer copy 256-byte aligned; forgetting to unpad produces an image
  of exactly the right length, skewed diagonally. `padded_row_bytes` is a named
  function with tests that run on every target, including wasm where nothing
  calls it.
- **`compare` / `Tolerance` / `diff_image`** live in render-core, not in the
  wgpu crate: they compare two `RawImage`s and do not care which backend made
  either, so the ash port reuses every reference unchanged. `Tolerance` makes
  every way of passing a number stated at the callsite — `CLOSE_ENOUGH` is 2
  levels per channel and 0.5% of pixels, `EXACT` is nothing at all.
- **Failures leave evidence.** A mismatch writes `<name>-actual.png` and
  `<name>-diff.png` into `target/verify/golden/`, the diff painting differing
  pixels magenta, and CI uploads them. A golden failure that leaves nothing to
  look at makes the reader re-run it by hand to find out what happened.
- **References are blessed explicitly** (`JIDOUSHA_BLESS=1`), never written on
  first run. A reference that appears when the file is missing turns every
  unexplained change into a new reference, which is the one way this tier can
  assert nothing at all.
- **The reference comparison is Linux-only, deliberately.** A reference is a
  picture *some rasterizer* produced; CI blesses and compares on lavapipe, Mesa's
  CPU rasterizer, which is deterministic and identical across runners. A D3D or
  Metal device fills edge pixels differently enough that a tolerance loose
  enough to accept it would be loose enough to accept a real regression.
  Everything else in the file — the offscreen target, capture, unpadding, the
  clear colour, render-twice stability — runs everywhere.
- **No adapter is not a failure.** Every runner is headless and some have no
  graphics stack at all; the tests say so and pass, and `tools/doctor` reports
  whether the tier can run, so a skipped tier is a diagnosable fact rather than
  a silence. CI installs `mesa-vulkan-drivers` on Linux, which is what turns the
  skip into a run.

**Implemented (R4), `tools/verify` integration:** `examples/prototype_kit`'s
verify run now asserts text on screen (a glyph covers the middle of the score,
which is positioned by `TextStyle::width_of` — so the layout ran), the ball's
world position after a fixed number of ticks against a checked-in number, and
that a sprite-sized quad is drawn where the world puts it. It then replays the
*same* session through an offscreen `WgpuBackend`, asserts the world did the
same thing on both backends (§1's contract, checked rather than asserted), and
writes the last frame to `target/verify/prototype_kit.png`.

**Closed (web harness):** `tools/serve-web <example>` builds an example for
wasm, runs `wasm-bindgen`, writes the page from `tools/web/index.html`, and
serves it. `--check` additionally drives a headless Chromium at it, screenshots
the result, decodes the PNG, and asserts the canvas is not the page's own
background — so "it works on the web" means *the engine drew a frame in a
browser*, not "it compiled". Tooling notes and the version-skew trap are in
tooling.md.

**The harness found a real bug on its first run**, which is the argument for
having built it. With `Backends::all()`, wgpu asks the browser for WebGPU first;
on a browser that has `navigator.gpu` but yields no adapter — Chromium under a
software rasterizer, and **every browser without WebGPU support** — the request
fails and nothing falls back to GL. The page loads, the engine runs, the module
reports itself healthy, and the canvas stays blank. R1 shipped that way and
nobody could have noticed, because there was no way to load the page. The web
build now asks for `Backends::GL` explicitly, which costs nothing today (the
device already requests `downlevel_webgl2_defaults` limits) and is tagged
`DELIBERATE:` with the condition for revisiting it.

Worth keeping from that: the check's first version only asserted that the module
*started*, and the buggy page started perfectly. Checking pixels is what turned
it from a smoke test into a verification — the same lesson M5 learned when a
vacuous "did the frame draw" assertion let two mutants through.

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

  **Verified, both targets, by different means.** The isolation rule, the poll
  mechanism, and both target builds are checked in CI, and `tools/test` builds
  `window_clear` without running it. The environment this was built in has no
  display *and no GPU adapter at all* — the probe reports "no suitable graphics
  adapter found" — so neither could be seen from here. **A human has since
  confirmed the colored window natively**, and the web is confirmed
  automatically by `tools/serve-web --check`, which loads the page in a headless
  browser and checks that the canvas was actually painted.

  The web harness did not exist when R1 was written, which is why the WebGL2
  fallback bug it later found could ship unnoticed. §9 records both.
- **R2 — sprites end to end.** ✅ Texture upload, sprite pipeline (WGSL), batching,
  camera UBO, `draw_sprites`, placeholder texture. `examples/sprites.rs`
  (moving, rotating, tinted, atlas-region sprites).
  Exit: R0 transcripts unchanged; sprites visible on all targets.

  **Both exit criteria met, and the second one was actually checked.** The R0
  transcript tests pass unmodified — the seam held, and the sprite pipeline
  needed no change above it. `tools/serve-web sprites --check` drives the
  example in a headless browser and screenshots it: the four atlas tiles, the
  rotating sprite, the tinted half-transparent one, and the checkered magenta
  placeholder are all there, correctly oriented and correctly ordered. That is
  a stronger claim than R1 could make on the day, and it is the web harness
  paying for itself a second time.

  Native is unverified from here, as R1 was: this environment has no GPU adapter
  at all. The same code path is what the browser ran, and a human check is the
  remaining half.

  **What was wired, beyond the pipeline.** The driver now commits assets and
  uploads what became ready, *before* the frame's ticks — assets.md §4's
  commit-point CONTRACT, which had been designed since A0 and unimplemented
  since M5. The two built-in textures are created through
  `create_builtin_textures`, which returns the table naming them; the driver
  previously assumed they were ids 0 and 1, which was true only because nothing
  had been created yet.

  **Two bugs the writing caught**, both about the gap between asking and
  arriving — the same seam R1's two bugs were on:

  - `create_texture` before the device existed silently dropped the texels and
    handed back an id that would never sample anything. R1 could not hit it
    (nothing was uploaded); at R2 it is the *common* startup path, because a
    small PNG off a warm disk beats an adapter-and-device negotiation. The
    backend now holds those texels and uploads them the moment the device lands,
    so `create_texture` means what it says with no timing rider attached.
  - The clear color was never linearized, so every window since R1 has been
    lighter than the color it was given. Nobody could have noticed without a
    reference; the sprite pipeline supplies one, since a white quad over a white
    clear must match. Fixed in the one conversion function both now use.

  **What the mutation checks said.** Eighteen deliberate breakages, five of
  which escaped the first time. Three of those five were real gaps, and the
  fixes are the interesting part:

  - **The generation check on a queued upload was untested**, because the test
    that was supposed to cover it tested the wrong thing. `unload` removes the
    entry, so an unloaded id is skipped by the entry lookup and the generation
    never comes into it. The case it actually guards is the slot *coming back*:
    unload, load something else into the same index, and without the check the
    stale id finds the new entry, takes its texels, and registers them under the
    dead handle's name. That test exists now.
  - **The view-projection matrix was checked against the identity**, which is
    mostly zeroes — and a buffer that is mostly zeroes agrees with one that was
    never written. Truncating the pack to a single float passed. It is now
    checked against sixteen distinct values, plus a column-major ordering test,
    since a row-major pack would compile, run, and put every sprite somewhere
    else.
  - **Nothing exercised the driver's backend at all**, because no test had one:
    `Driver` held a `WgpuBackend`, which needs a window and a GPU. It now holds
    a `Box<dyn RenderBackend>` — this crate still *picks* wgpu, in `resumed`, and
    nothing else names it — so the tests install a `NullBackend` they keep a
    handle to. That closed the last untested third of `frame`, and it is what
    catches four of the eighteen.

  The two that still escape are **equivalent mutants**, and both are equivalent
  for a reason worth knowing:

  - *`take_uploads` cloning the queue instead of draining it.* The drain is
    guarded twice — the queue is emptied, and the texels are taken out of the
    slot — so removing one guard changes nothing a caller can see. What it does
    change is that the queue grows forever, which is a leak no assertion here
    can reach.
  - *Queueing every payload, bytes included.* Texture and bytes ids share a
    value space (both are index + generation, one table each), so a bytes
    completion queues an id that aliases a *texture* slot. It cannot do damage:
    the lookup is in the texture table and the payload match accepts only
    `Payload::Texture`, and a texture's data is present only after that texture
    queued itself. The guard states the intent; the safety comes from the two
    checks after it. That is closer to a real bug than is comfortable, which is
    why it is written down here.
- **R3 — primitives + text.** ✅ rect/line/circle expansion, embedded font,
  `examples/prototype_kit/` (sprites + shapes + score text — the "can an
  agent make Pong?" substrate is now complete). Exit: transcript tests for all
  primitive expansion; example runs everywhere.

  **Both criteria met.** `tests/primitives.rs` asserts on recorded frames for
  every primitive; `tools/serve-web prototype_kit --check` screenshots the
  example in a headless browser, and the field markings, the paddles, the
  hitbox outline, the score, the debug readout and the whole printable ASCII
  range are all there and legible. Native remains the human half, as at R1 and
  R2 — this environment has no GPU adapter.

  **A circle is a fan of quads, not a triangle fan.** Each quad is the centre
  and three points on the rim, so it covers two segments as two triangles —
  half the quads a fan would need, and every one convex, which is what keeps
  `FrameRecord::covering` able to answer "is the cursor on the ball?" exactly
  rather than by bounding box. The segment count is fixed at 32 rather than
  scaled by radius: a radius-dependent count would change the transcript, and
  every golden image, when a circle grows by a pixel.

  **Shapes and text are not a debug layer.** They expand into the same `Quad`
  and go through the same sort and batch, so a hitbox outline can be drawn
  *behind* a sprite by choosing a lower layer. Engines with a separate debug
  pass cannot do that, and every outline they draw sits on top whether that
  helps or not. `shapes_and_sprites_interleave_by_depth` is the test that says
  so.

  **A thing worth knowing about alpha.** Blending happens in linear light,
  because the surface is sRGB and that is where blending is physically right.
  The practical consequence is that a small alpha over a dark background reads
  much brighter than the number suggests — 6% white looked like a solid grey
  disc in `prototype_kit` before it was turned down to 1.5%. This is correct
  and is not going to change; it is recorded because it surprises everyone once.

  **What the mutation checks said.** Twenty-one deliberate breakages, twenty
  caught first time. The escape was **swapping a rectangle's other two corners**
  — which renders identically, because culling is off and the two triangles
  still cover the same rectangle, and which silently breaks the winding every
  other quad in the engine keeps. The test had checked only the opposite pair,
  which a swap does not move. Both it and the glyph-quad test now pin all four,
  because that invariant is what R4's golden images and any future culling will
  rest on.
- **R4 — capture + golden images + verify integration.** ✅ Offscreen
  `capture()` in the wgpu backend, tolerance-based golden tests in native CI,
  `tools/verify` wired: headless sim ticks + transcript assertions + optional
  frame capture.
  Exit: a `verify` run on `prototype_kit` asserts text on screen, sprite
  positions after N ticks, and produces a captured PNG artifact. ✅

  Delivered: `WgpuBackend::offscreen` and `capture.rs`, render-core's `golden.rs`,
  `jidousha-assets::encode_png`, `tests/golden.rs` with `tests/golden/sprite_scene.png`,
  and the example's `capture.rs`. §9 records the design decisions.

  **The environment was the hard part, and it is worth writing down.** A GPU-less
  machine cannot bless a reference, and the first probe here found zero adapters:
  no Vulkan ICD, no libEGL, nothing for wgpu to pick. `mesa-vulkan-drivers`
  provides lavapipe, a CPU rasterizer, and with it wgpu reports one adapter and
  the whole tier runs — deterministically, which is better for a reference than a
  real GPU would be. CI installs the same package. Everything below is a
  consequence of that: the Linux-only comparison, the loud skip, doctor's `gpu`
  check.

  **Two constructors, not a flag.** `new(window, size)` and `offscreen(size)`
  make different things rather than configuring one thing, so there is no state
  where a backend is "windowed but capturing". The refusal on the windowed path
  names the constructor that can answer, which is the whole cost of the split.

  **What the mutation checks said.** Twenty-two deliberate breakages, all
  twenty-two caught. The interesting ones were in the comparison rather than in
  the rendering: dropping alpha from the per-pixel comparison, comparing only the
  overlap of two differently-sized images, and returning `matched: true` on a
  size mismatch all die, because each has a test written against the specific
  wrong answer it produces. Moving the textured quad half a world unit — three
  pixels — moves 1.97% of the frame, against a 0.5% threshold, so the tolerance
  has room to absorb driver rounding without absorbing a regression.

## 12. Deferred (tracked, not designed)

Custom shaders/materials · render-to-texture & multiple cameras · particles ·
lighting/post · letterbox mode · TTF text · atlas packing tooling (assets doc
may claim it) · instancing/perf work (needs evidence) · 3D bridge (future ADR).
