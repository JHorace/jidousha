# Public API — the `jidousha` facade and `docs/api/`

Status: **implemented at F0.** The facade crate exists, `docs/api/` is generated
from it, and `tools/check-api-coverage` enforces both of §1's contracts. Owns:
what game agents see. **CONTRACT** items binding as elsewhere. Additions to the
inventory still go through this doc first — the difference is that a change here
and a change to the crate now fail CI when they disagree.

## 1. Principles

- **The facade is a curation, not a re-export dump.** `jidousha` re-exports the
  curated surface from internal crates. If a symbol isn't in the inventory
  below (or added here by a reviewed change), it is not public. `#[doc(hidden)]`
  escape hatches are banned — hidden public API is how surfaces rot.
- **CONTRACT: games depend on `jidousha` only.** Examples and generated games
  never name an internal crate. CI-checked (grep examples' `Cargo.toml`s).
- **One import**: `use jidousha::prelude::*;` brings in everything below.
  Sub-modules exist for humans browsing docs, but the prelude is the way.
- **Budgeted surface.** The full API must document into ~25k tokens (§4). CI
  counts; growth beyond budget forces a curation conversation, not a bigger doc.
- Every public item carries a doc comment with a one-line summary and a
  compilable example (doctest). `#![deny(missing_docs)]` + doctests in CI.

## 2. The inventory (v1)

Signatures here are final unless marked (impl) — implementation may adjust
(impl)-marked details with a doc update; changing a final signature is a
reviewed API change.

**App & lifecycle**
```rust
jidousha::run(config: GameConfig, setup: impl FnOnce(&mut App)) -> Result<(), RunError>
jidousha::headless(config: GameConfig, setup: impl FnOnce(&mut App)) -> HeadlessSim
App::add_system(phase, system)            // Startup|Update: fn(&mut World); Draw: fn(&mut DrawCtx)
GameConfig { title, seed, fixed_dt, asset_root, window_size, camera_height }  // + Default
HeadlessSim::tick(&mut self, TickInput)   // TickInput = InputSnapshot + asset readiness script
HeadlessSim::world(&self) / world_mut(&mut self)
Startup, Update, Draw                     // phase types
```

Built in M4, in `jidousha-core` until the facade re-exports them: `App`,
`headless`, `HeadlessSim`, `GameConfig`, and the three phase types. Two
differences from the signatures above, both waiting on milestones rather than
decisions: `HeadlessSim::tick` takes no argument yet (`TickInput` needs input
and assets to exist), and `GameConfig` carries `title`, `seed` and `fixed_dt`
only — the asset, window and camera fields arrive with their subsystems, and
`..GameConfig::default()` means adding them disturbs nothing already written.

**`window_size` landed at E0** (e0-findings.md F-013), five milestones after the
subsystem it was waiting for. M5 built the window and did not bring the field
with it, and nothing noticed, because "arrives with its subsystem" was written
in two documents and attached to no milestone's checklist. The E0 game wanted a
16:9 window for a 34×19 field, could not tell whether the field existed, and
settled for a comment admitting a narrow window crops the playfield — its own
author calling that "a gameplay decision made by ignorance".

This moved `PhysicalSize` from `jidousha-render-core` to `jidousha-core`, with a
re-export so no call site changed. `GameConfig` lives in core and core depends
on no other jidousha crate, so the type had to be on the near side of that
seam — the same reasoning ADR-0015 applies to the draw vocabulary, applied to
pixels. On the web the field is ignored: the canvas is sized by the page, and a
canvas that disagreed with its CSS would be drawn at one size and shown at
another.

`camera_height` remains unlanded, and is recorded here rather than dropped.
`run` lands with the platform crate (M5); it is the same loop with a different
driver (core.md §8).

Built in M5: `run(config, setup) -> Result<(), RunError>` in `jidousha-platform`,
with the signature above. Two additions the inventory does not list, both for
drivers rather than games: `jidousha_core::build`, the construction both drivers
share, and a per-tick callback on `Simulation::advance` that lets a driver set
the input resource per tick without core naming an `InputSnapshot`. `RunError`
is the §9 environmental class the inventory already anticipated; its commonest
variant tells a headless caller to use `headless` instead.

**ECS (core doc §2–6)**
```rust
Entity
World::{spawn, despawn, is_alive, insert, remove}
World::{component, component_mut, find_component, find_component_mut}   // ::<T>(e)
World::{resource, resource_mut, find_resource, insert_resource, remove_resource}
World::query::<Q>()                       // Q: tuples of &T/&mut T, With<T>, Without<T>
World::commands() -> Commands             // spawn/despawn/insert/remove, deferred
DrawCtx { world: &WorldView, ... }        // WorldView: read-only query/component/resource
derive(Component)
```

Built in M4: `Radians`, `math::{sin_cos, atan2, rotate}`, and the `Vec2`/`Vec3`
re-exports, all in `jidousha_core::math`.

**Math & primitives (ADR-0009, conventions)**
```rust
Vec2, Vec3, Mat4 (glam re-exports)        // the blessed subset; no wildcard glam re-export
Radians (+ from_degrees), Seconds, Rect, Color, Depth { layer: i16, z: f32 }
math::{sin_cos, atan2, ...}               // deterministic trig (impl: exact fn list)
Rng                                       // seeded; resource
Time { tick, fixed_dt, elapsed, alpha }
```

**Render (renderer doc)**
```rust
Transform { pos, z, rot, scale }          // component
Sprite { texture, region, size, anchor, tint, flip_x, flip_y, layer }  // component
Camera { center, height, clear_color }    // resource
Camera::{world_to_screen, screen_to_world}
DrawCtx::{sprite(&Transform, &Sprite), rect(Rect, Color, Depth),
          line(Vec2, Vec2, f32, Color, Depth), circle(Vec2, f32, Color, Depth),
          text(Vec2, &str, TextStyle)}
TextStyle { size, color, depth }          // + Default
systems::draw_sprites                     // provided Draw system
```

Built in R0, split across `jidousha-core` and `jidousha-render-core` until the
facade re-exports them. `Transform`, `Color`, `Rect` and `Depth` are in core —
they are the vocabulary `DrawCtx`'s sink speaks, and core cannot depend on the
renderer (ADR-0015). `Sprite`, `Camera` (with `world_to_screen`/`screen_to_world`)
and `draw_sprites` are in render-core.

Two additions to the inventory above. `DrawCtx::submit(Quad)` is the sink itself,
and `Quad` with it: games do not call it — `ctx.sprite(...)` does — but it is
public because render-core has to reach it from outside core. And `TextureId`,
the opaque id that lets core name a texture it cannot see; `TextureHandle::texture_id()`
mints one.

One shape change: `ctx.sprite(...)` and the rest arrive through the `Submit`
extension trait rather than as inherent methods on `DrawCtx`, for the same
reason. **The prelude must carry `Submit`** (F0) or `ctx.sprite(...)` will not
resolve in game code — that is the one place this seam can leak, and the F0
checklist should treat it as load-bearing rather than incidental.

`Camera` carries a fourth field the sketch above omits: `viewport`, the surface
size in pixels, without which `world_to_screen` has no aspect ratio to work
from. The driver maintains it.

Built in R1: nothing new for games. `Camera.clear_color` and `Camera.viewport`
start meaning something — the first is what a window is filled with, the second
is maintained by the driver on resize — and `jidousha_render_wgpu::WgpuBackend`
exists but is named only by the composition root. A game never mentions a
backend, which is the point of ADR-0003.

Built in R2: again nothing new for games, which is the interesting part — every
item the §2 inventory lists for sprites was already here, and R2 made them draw.
`Sprite`, `Transform`, `Camera` and `draw_sprites` are unchanged, and
`examples/sprites.rs` is written entirely out of them. Two additions, both for
drivers: `create_builtin_textures` and `upload_ready_textures` in render-core,
which the platform crate calls once a frame. A game never names either.

Built in R3: `DrawCtx::{rect, line, circle, text}` — all four on the `Submit`
trait beside `sprite` — and `TextStyle { size, color, depth }` with its
`Default`. `Depth` is now what every immediate primitive takes, as §3 designed.

One addition to the inventory: **`TextStyle::width_of(&str)`**. Without it a
game cannot centre a score, because the metrics are the engine's and a handle to
them is the only way to ask. It is exact rather than an estimate — the font is
monospace with no kerning — and multi-line text reports its widest line.

`TextStyle::size` is the height of one **line** in world units, not a point
size and not a cap height: text scales with the camera like everything else, and
world units are the only unit a game ever states (conventions).

**Assets (assets doc)**
```rust
Assets::{load_texture, load_bytes, status, all_ready, unload}   // resource
TextureHandle, BytesHandle, AssetStatus
```

Built in A0, in `jidousha-assets` until the facade re-exports it. Four additions
to the list above, none of them a change to it: `Assets::new(source)` (the one
constructor, ADR-0012), `bytes_of` (without it `load_bytes` returns a handle to
nothing anyone can read), `path_of` (a handle is opaque, so a game that wants to
log *which* asset failed has no other way to name it), and `commit(tick)` with
its `AssetFailure` — the driver-facing half of assets.md §4, which the platform
crate will call and games will not.

The `ByteSource` seam (`ByteSource`, `MemorySource`, `Completion`, `RequestId`)
is public because the platform crates implement it from outside. `MemorySource`
is the only part of it a game agent has reason to name, and only in tests.

Built in A1: `FileSource` in `jidousha-platform`, which a game names once when
building its `Assets`. Everything else A1 added is seam vocabulary that games do
not touch — `Payload`, `TextureData`, `AssetError`, `decode_png`,
`MAX_TEXTURE_SIZE`, `RequestId::from_bits` — plus `Assets::texture_of`, which is
for the renderer. CONTRACT: **simulation must not read `texture_of`**, because
nothing in a game's logic may depend on texture dimensions (renderer.md §3).
`AssetFailure` swapped its `reason: String` for a typed `error: AssetError`,
which is what lets each failure class say something specific; `message()` is
unchanged.

Built in A2: `jidousha_platform::asset_source(root)` — **the one thing a game
calls to get a source**, `FileSource` on native and `WebSource` on the web. It
is an addition to the inventory, and it earns its place by removing a `cfg` from
every game that loads anything: choosing a source per target is the platform
crate's job, and a game that did it itself would get it wrong the first time it
was ported. `AssetError` gained `Http` and `Unreachable`, which are web-only
failure classes §6 already anticipated.

Built in R2: `Assets::take_uploads` and `TextureUpload`, the renderer's side of
the store. Driver-facing, like `commit` — a game never calls either. The change
a game could notice is that `texture_of` returns `None` once the renderer has
taken the texels (ADR-0016), which matters only to code that reads texels, and
simulation is forbidden from doing that anyway.

Removed in I2: `ByteSource::outstanding`. Nothing called it — `all_ready` walks
the store's entries — so it was three implementations maintaining a counter for
nobody, and a second way to ask a question that already had one. A game could not
have been calling it; the seam is implemented from outside by the platform
crates, and they lose a field each.

Built in I2: `ReplaySource`, `Resolution`, `Assets::resolved()` and
`RequestId::bits()` — the recording seam. All four are for whoever is writing or
replaying a recording, which today is the driver and a test; a game touches none
of them. `ReplaySource` is the one a test agent has reason to name, and it names
it the same way it already names `MemorySource`: as the source it hands to
`Assets::new`.

**Input (input doc)**
```rust
Input::{held, just_pressed, just_released, pointer, pointers, window_focused}  // resource
Key, PointerButton, PointerState, InputSnapshot
InputScript                               // testing/verify; in jidousha::testing
```

Built in I0, in `jidousha-input` until the facade re-exports it. The list above
is exactly what a game touches, and it is unchanged. Around it sit the pieces the
*driver* needs, which games do not: `Input::new` and `InputSnapshot::new`,
`InputSnapshot`'s accessors (`held_keys`, `pressed_keys`, `released_keys`,
`pointers`, `window_focused`) and its `encode`/`try_decode` with `DecodeError`,
`PointerId`, and the `InputEvent`/`SnapshotBuilder` pair the platform crate will
feed. `InputScript` also carries `last_tick`, so a test can drive a script
without restating its length.

Note for F0: `InputScript` belongs behind `jidousha::testing` per the list above,
and the facade does not exist yet. Until it does it lives beside the types it
builds, and moving it is a re-export, not a rewrite.

Built in I1: nothing new. The translation from winit's vocabulary to the
engine's is entirely inside `jidousha-platform` and no type crosses out of it
(ADR-0004) — a game sees the same `Input` it saw at I0, now with real keys and a
real pointer behind it. `examples/input_echo.rs` is written from the list above
plus one thing not on it: `Input::snapshot()`, for the unusual case of wanting
*every* key that is down rather than asking about one. A readout is what that is
for; a game asks `held(Key::W)`.

Built in I2: `Recording`, `TickRecord`, `AssetReady` and `RecordingError` — the
stream around the snapshots. Driver-and-tooling vocabulary, like `Assets::commit`:
a game writes a recording only in the sense that the driver writes one for it.
The list a game touches is still the four lines above, and I2 did not change
them — `PointerState` in particular still has no `world` field, which ADR-0017
now settles rather than defers.

Built in R4 (verification vocabulary, not game vocabulary): `WgpuBackend::offscreen`
— which a game never calls, because a game has a window — plus render-core's
`Tolerance`, `Comparison`, `compare`, `diff_image`, `encode_png`, `decode_png`,
and `FONT_TEXTURE`. The last is the only one worth a second look: it is public so
a verification can ask *"is there text on screen?"* by resolving it through the
frame's `TextureTable` and finding the quads that sample it. It is not a second
way to draw text — `ctx.text` remains the only one — it names what that
produced. `jidousha_assets::encode_png` joins `decode_png` for the same reason:
writing a captured frame out is what a golden reference and a `tools/verify`
artifact are.

Built in F0: the facade exists, and the count above is now a measured 50 rather
than an estimate — `tools/check-api-coverage` reads it off the crate. Five items
joined the inventory during the build, each because something could not be
written without it: `PhysicalSize` (`Camera.viewport` is a public field of a
game-facing type, so its type is game-facing), `message` (a game writing its own
§9 errors needs the helper), `EntityDeadError` in the prelude rather than only at
the root, `MemorySource` (a test names it to script a load), and `asset_source`.

The **`Submit` warning above was correct and load-bearing**: the prelude carries
it, with a doc comment saying why, because `ctx.sprite(...)` does not resolve
without it.

`jidousha::testing` is the second module, and the one place a backend is named —
`WgpuBackend`, because a golden image has to be drawn by something. That is not a
breach of ADR-0003, which forbids `wgpu` *types* escaping; none do.

Added after E0 run 3: `Batch` and `QuadVertex` (e0-findings.md F-036). Same rule
as `Submissions` under F-017 — `FramePlan::batches` is a public field of an
exported type, so its element type is game-facing whether it was curated in or
not, and `Batch::vertices` drags `QuadVertex` in behind it. Exporting one and
not the other would move the hole rather than close it. This is the **second**
run to find this class by hand; the generator gate F-017 deferred is now overdue,
and the remaining candidates are listed there.

**Changed after E0 run 4: two signatures, one addition, one refusal.** §2 says a
final signature changes only as a reviewed API change; this is that record, and
each has its ADR.

- **`Camera::visible_bounds() -> Rect`**, was `-> (Vec2, Vec2)` (ADR-0021,
  e0-findings.md F-042). The pair it returned was `Rect`'s two fields under
  `Rect`'s own documented meaning, and the tuple was never a decision anybody
  made — no ADR, no `DELIBERATE:` tag, and no crate-boundary reason, since `Rect`
  is in core and the camera is downstream. It cost six hand-written comparisons in
  the single assertion `docs/api/` pushes hardest, in three consecutive runs.
- **`Rect::contains_rect(self, other) -> bool`** joins the inventory, and is the
  load-bearing half of that change: returning a `Rect` without it would have saved
  one destructuring line, and the six lines were the comparison. **Closed on all
  four sides**, unlike `contains`, which is half-open because it partitions space —
  a quad flush against the camera's edge is on screen. Both doc comments name the
  other; the distinction is pinned by a test, because getting it wrong would trade
  one silent trap for another.
- **`FrameRecorder::draw() -> FrameRecord`**, was `-> &FrameRecord` (ADR-0023,
  F-040). The borrow made the composition *Testing your game* recommends — inspect
  the run's last frame, then build the screens it never reached — a compile error,
  and a run worked around it with a second recorder that silently redirected the
  transcript. `clear()` was considered and **declined**: the frame history is what
  a failing assertion reads backwards.
- **Declined: `Rect::sweep` and `Rect::inflate`** (ADR-0022, F-041), reached for by
  three runs. A primitive would absorb about eight of the forty lines a Pong writes
  — the other thirty are the collision *response*, which is the game's model — and
  answering the first question while refusing the second is the start of a physics
  subsystem ADR-0001 scopes out. What landed instead is the boundary, stated in
  Concepts with the eight-line shape, the way `App::quit` is stated.

Rough count: ~46 types/functions. CONTRACT: the v1 prototype substrate
("agent Pong/asteroids/breakout") must be expressible with this list alone —
that's exactly what acceptance milestone E0 tests (implementation plan).

## 3. Design notes

- `Depth { layer, z }` (with `Depth::default()` = (0, 0.0) and
  `Depth::layer(i16)`) is the uniform depth argument for immediate primitives;
  sprites carry `layer` in `Sprite` and `z` in `Transform` because they're
  entity data. This asymmetry is DELIBERATE: components are the entity-driven
  path, `Depth` is the immediate path; merging them made both worse.

  Text is the third case and the same rule: `TextStyle` carries `depth`, so
  `ctx.text` takes no trailing `Depth` while the other four verbs do. E0 run 1
  read that as a wobble against "one way to do everything" (e0-findings.md
  F-014) — fairly, since nothing had written it down. ADR-0018 does now, and
  states the rule the three cases share: **depth travels with whatever else
  describes the thing's appearance**, which is a sprite's components, text's
  style, or — when there is nothing else — an argument.
- `jidousha::testing` (InputScript, transcript assertion helpers) is public but
  documented in its own short section of `docs/api/` — game agents use it in
  their games' tests, which agents should be writing too.
- `RunError` is the §9-taxonomy environmental class (no GPU adapter, etc.);
  everything else at the API surface panics per taxonomy with agent-grade
  messages.

  **A game does not return one to `main`.** The examples match on it and print
  `Display`, which is the `message(what, specifics, likely_cause, fix)` house
  style; `fn main() -> Result<(), RunError>` prints the `Debug` form, and E0
  run 2 shipped a game whose only user-facing failure output was a struct dump
  with a vendored winit path in it (e0-findings.md F-022). The Quickstart is
  what every game copies, so the Quickstart is where the shape is decided.
- **There is no way for a game to quit itself in v1**, and that is a decision
  rather than an omission. No `App::quit`, nothing on `World` or `Commands`:
  `run` is the program until the player closes the window. A quit path is a
  lifecycle question — what gets flushed, whether Draw runs once more, what a
  web build does when there is no window to close — and v1 has no game that
  needs it. E0 run 2 read the whole reference looking (F-027), so Concepts now
  states it, and `Key::Escape` is listed because games back out of menus with
  it. Revisit with the first game that has a menu.

## 4. `docs/api/` — the generated game-agent surface

- **One file**, `docs/api/jidousha-api.md`, generated by `tools/gen-api-doc`
  from the facade crate's rustdoc (mechanism (impl); likely rustdoc JSON).
  CI fails when stale (practices §2.3) or when over **25k tokens** (counted in
  CI; the budget is the point — it must fit comfortably in a game-writing
  agent's context alongside the game itself).

  **Amended (ADR-0025): two files, split by what the reader is doing.**
  `jidousha-api.md` is how a game is written — Quickstart, Concepts, Reference,
  Conventions — with the 25k budget above, now at ~13.3k. `jidousha-testing.md`
  is how one is checked — *Testing your game* and the `jidousha::testing`
  reference — with its own 15k budget, at ~11.6k. The trigger was measuring the
  one file at ~24.1k and finding **46% of it was about verifying a game rather
  than writing one**, which is 46% of a game-writing agent's budget spent on a
  job it is not doing. Both budgets are enforced per document; growth past
  either is still a curation conversation, not a bigger number.

  **After seven E0 runs that conversation came due, and was had.**
  `jidousha-testing.md` reached ~14.7k of its 15k on run 7's fixes, because four
  consecutive runs' findings all landed in the same file. Two things brought it
  back to ~13.6k, and they are worth separating because only one of them scales.

  - **~767 tokens came out of the reference, structurally** (ADR-0028).
    `NullBackend`, `plan_frame`, `compare`, `Comparison`, `Tolerance` and
    `diff_image` were exported for a road only `prototype_kit` walked; the
    example stopped walking it and they left. That is a fifth of the reference,
    recovered by removing items rather than by compressing prose, and it does not
    come back.
  - **~300 tokens came out of the prose, on a stated principle.** Evidence in
    this document does two jobs and only one belongs here: *persuasion* ("this
    really cost someone") earns a clause, while *specification* — numbers a
    reader can lift as a recipe — is a hazard, because F-080 is a run following a
    measured anecdote out of its scope. So the document keeps the rule and the
    scope qualifier and sends the case history to `e0-findings.md`, which holds
    all of it already. That is deduplication rather than loss, but it is a
    one-time recovery: the prose is now near its floor.

  **The open half.** The prose grows with each run's findings and the reference
  no longer does, so the next squeeze is prose again and there is nothing left to
  compress. The `make-game` skill is the obvious other home and `e0-findings.md`
  §7 has declined it four times, on the grounds that two homes for one lesson is
  worse than one crowded home — and it cannot be the answer while E0's read list
  is `docs/api/` + `examples/`, since a skill is either invisible to the run or
  changes what E0 measures. Watch it as a **convergence** signal instead: the
  prose half is supposed to stop growing when E0 passes, so a document full again
  *and* an E0 that still has not passed is evidence about the acceptance bar
  rather than about tokens. Raising a budget stays the one answer ADR-0025
  forecloses.

  Implemented (impl): not rustdoc JSON — that needs a nightly toolchain and
  `rust-toolchain.toml` pins stable (ADR-0005) — but a text extractor over the
  crate sources, with tests. Blocks close on indentation rather than brace
  depth, which `cargo fmt` guarantees and string literals defeat.

  **The signature half of the bullet below went unimplemented until E0 run 1
  measured what it cost** (e0-findings.md F-001): the Reference shipped as ~90
  name-and-one-liner bullets, and the run reported it could not make a single
  call from the document. Fixed by extracting declarations — fields with types,
  enum variants, trait and inherent method signatures, associated consts,
  `Default` values — at ~12.9k tokens of the 25k budget. The gap survived
  because a thin entry is indistinguishable from a complete one to the agent
  reading it, so `completeness_failures` now fails the run when an exported item
  yields no declaration. A generator that can under-report silently will.

  ~~**Still unimplemented: the "tiny example" third of the bullet above.**~~
  **Built, once ADR-0025's split made room.** Entries now carry a signature, a
  one-liner and the item's own doctest, which costs ~2.1k rather than the ~5k
  estimated below — only *exported* items render one, and most of the 41 blocks
  in the crates hang off things this surface never names. The example is the
  crate's doctest, so it is code CI compiles, the same argument the Quickstart is
  embedded verbatim for.

  Three things are stripped on the way out, and the third was found by the
  vocabulary gate firing rather than by foresight: rustdoc's hidden `#` setup
  lines, visible `use jidousha_…` imports, and internal crate paths written out
  mid-expression. All three name crates a facade exists to hide, and the
  document's own second sentence is "everything here is reachable from one
  import" — so an example that kept them would contradict the page it sits on.
  What remains is prelude-only and callable, which is what F-001 asked for.

  The original note, kept because its reasoning is what got it built:
  Entries carried a signature and a one-liner and no example. Deferred until E0 run 2 says
  whether signatures alone are enough (e0-findings.md F-001, "Still open") — the
  document is at ~13.8k tokens of 25,000 and the ~39 doctests already in the
  crates would cost about 5k more, so budget is not the constraint. (It became
  the constraint at ~24.1k, and stopped being one again at ~13.3k after
  ADR-0025 — that estimate is roughly the headroom the split handed back.)
  Recorded
  here rather than left implicit: §4 and `gen-api-doc` disagreeing without
  either saying so is precisely what F-001 was, and a second silent disagreement
  in the same paragraph would be the same bug wearing the same hat.

  **Overdue, and now the top of the generator's queue: nothing checks whether the
  sentence the generator keeps is the sentence that mattered.** `first_sentence`
  takes the first sentence of a doc comment and `trailing` truncates a member's at
  68 characters — both correct, both what they were built to do, and between them
  they are the mechanism behind **four of E0 run 4's sixteen findings**
  (e0-findings.md F-039, F-043, F-048, and the wording half of F-050). In every
  case the doc comment was already right and the reference printed something that
  was not wrong, merely empty: `Submit::circle` explained its fixed segment count in
  its body and the reference printed "Fill a circle."; `Time::alpha` explained
  itself in four lines and the reference printed a clause cut mid-sentence.

  This is the fourth consecutive run whose findings are mostly "the document does
  not say what this *does*", which `e0-findings.md` §6 predicted would mean the
  sentences are not the problem. The shape of the check: for each rendered item,
  compare the summary against the rest of its doc comment and fail when the body
  states a fact the summary drops — a truncated member line, or a second sentence
  carrying a number, a count, or a "nothing consumes this". It cannot be exact and
  does not need to be; a warning listing the items whose bodies are much longer
  than their summaries would have caught all four, because all four were the
  longest bodies with the shortest summaries in their groups. **This now outranks
  the F-017/F-036 export gate**, which has cost three runs a lookup each; this one
  cost a run a debug cycle and cost this file a recorded falsehood (F-039).

  **Built — the exact half. Measured and declined — the fuzzy half.** The two
  halves of the paragraph above turned out to be very different propositions, and
  the numbers are recorded here so the next reader does not have to re-derive
  them.

  The *truncated member line* half was not a heuristic at all, and it was worse
  than described. `trailing` cut a member's summary at 68 characters and appended
  an ellipsis, justified by "the whole sentence stays on the item it belongs to;
  this is the reminder" — **and that premise was false.** A member has no entry of
  its own anywhere in this document, so its summary appears on that line and
  nowhere else; the cut tail was not kept somewhere, it was deleted. It was
  happening to 28 lines. `FrameRecord::quads` was losing "…not *submission*
  order", which is the whole of ADR-0024. `member_lines` now puts a long summary
  on its own wrapped line above the signature, `cut_summaries` fails the run on
  any line inside a code block that ends in an ellipsis, and the cost is a few
  lines of document.

  The *body states a fact the summary drops* half **is not a gate, deliberately.**
  Both shapes suggested above were measured against the real sources:

  - "bodies much longer than their summaries" flags **71 of the exported items**
    at a 3× ratio. That is most of the surface, and it is not a defect — a doc
    comment's body is *supposed* to be longer than its summary.
  - "a number in the body the summary drops" flags **67**, and inspection shows
    the signal is swamped by ADR numbers (`0008`, `0011`, `0015`) and section
    references (`§6`, `§9`) rather than quantities.

  Neither is worth shipping. A gate that fires on most of the surface trains its
  reader to ignore it, which is strictly worse than no gate — the same argument
  this document makes about a thin reference entry being indistinguishable from a
  complete one. If this returns, the way in is a *targeted* signal rather than a
  size comparison: the recurring shape is a body sentence that names a count of
  something the API produces (`circle`'s sixteen wedges, `Submissions`' six
  vertices per quad), and that is a question about the vocabulary of a handful of
  doc comments, not about their length.
- Fixed structure: **Quickstart** (one complete ~60-line game, compiling,
  CI-tested — it IS an example file, included verbatim) → **Concepts** (seven
  short paragraphs: world/systems/phases, determinism & the tick, drawing,
  assets & placeholders, input, coordinates, and **the read-pass/write-pass
  pattern** — reading other entities while mutating, per ADR-0013; content
  lands at F0, drawn from core.md §5 and the `homing` example, stated in game
  vocabulary with no mention of archetypes or borrows) → **Reference** (the §2 inventory,
  grouped as above, one entry per item: signature, one-liner, tiny example) →
  **Conventions digest** (auto-included from conventions.md) → **Testing your
  game** (headless + InputScript, brief).

  **Amended (ADR-0025).** That is the *game* document's structure, minus the
  last item: **Testing your game** is now the other document, which runs
  *Testing your game* → **Reference** (`jidousha::testing`). The game document
  keeps a pointer where each half used to be — one in the Reference group that
  held the testing signatures, one in the section that held the prose — because
  the single cost of a split surface is an agent not knowing the second file is
  there.
- CONTRACT: `docs/api/` never mentions internal crates, the backend seam,
  archetype storage, or any implementation vocabulary. Quality bar (practices
  §2.3): a fresh agent with only these files + `examples/` ships a working
  prototype. E0 is the test.

  **Amended (ADR-0025): the CONTRACT is per document, and tighter than it was.**
  The game document is checked entire, with no exemption of any kind. The
  testing document may use exactly three words the game document may not —
  `wgpu`, `RenderBackend`, `FramePlan` — because a picture has to be drawn by
  something and a capture recipe cannot be written without naming the renderer.
  Everything else in `FORBIDDEN` applies to both: an internal crate name or a
  pointer into `docs/internal/` is refused in either. This *replaces* a wider
  exemption — `gen-api-doc` used to cut the whole `jidousha::testing` reference
  block out of the check, so anything forbidden could sit inside it unnoticed.
  Three words in one document beats one whole section in another.

  The old carve-out also failed for prose rather than entries, which is what
  forced the question: F-066 needed a capture recipe in *Testing your game*,
  could not name a renderer there, and shipped words and a pointer instead. That
  recipe is now compiling code.

  **"A pointer into `docs/internal/` is refused in either" became true in F-070**,
  and was not before. The guard has two halves — `CITATION_RE`, which strips a
  parenthetical citation on the way out, and `FORBIDDEN`, which fails the build on
  anything written another way — and `e0-findings.md` fell between them: the
  pattern's filename class carried no digits, and the `FORBIDDEN` entry names
  `docs/internal`, the directory, rather than the file. Two citations of it had
  reached the game document. The class takes digits now and `e0-findings` is in
  `FORBIDDEN`. Worth recording as the shape rather than the instance: a guard
  written as *pattern plus deny-list* leaves a gap wherever the pattern's
  assumptions and the deny-list's spelling do not line up, and the gap is silent
  by construction.

## 5. Examples as API fixtures

`examples/` is part of the public surface (practices §5.1): `headless_sim`,
`window_clear`, `sprites`, `prototype_kit`, `input_echo`, `homing`,
`spawn_and_reap`, `scripted_player`, `load_from_disk`, `loading_gate`,
`what_was_drawn`, plus the two the documents embed verbatim — `quickstart` and
`vec2_tour`. CONTRACT: every §2 item appears in at least
one example; `tools/check-api-coverage` (grep-level) enforces in CI.

**The two embedded ones carry a reference's job and need a reference's care.**
`quickstart` is the game document's opening, and `vec2_tour` *is* the `Vec2`
entry — the generator has nothing to generate from for a foreign type, so F-018
made an example the entry instead. The consequence, which E0 run 6 paid for
(F-071): cargo checks that everything the file lists exists and can say nothing
about what it omits, so completeness there is curated by hand. An operation a game
reaches for and that file does not name is a bug in the file. Neither example may
be deleted or thinned without regenerating `docs/api/`, and the list above is
prose rather than a gate — `tools/test` discovers examples from cargo, so a new
one runs in CI whether or not anybody adds it here. This paragraph drifted for
five examples before run 6's triage caught it.

**Before F0**, the facade did not exist, so an example had no `jidousha` crate
to depend on. Examples written in the meantime lived beside the crate they
exercised and named that internal crate — a knowing, temporary breach of the
"games depend on `jidousha` only" contract above.

Implemented (F0): they moved, they were rewritten against the facade, and
`tools/check-api-coverage` enforces the contract. **They live in
`crates/jidousha/examples/` rather than at the repository root**, which is the
one correction to the plan above: cargo lets an example depend on the package it
sits in, so the facade's own examples get `jidousha` and nothing else for free,
where a root `examples/` directory would need the workspace root turned into a
package to host them. The intent — an example depends on the facade only — is
what the check tests, and it is met.

Two examples stayed behind, and neither is a game: `window_blank` is the driver
smoke test M5 added, and `what_was_drawn` is render-core's own transcript
fixture. The coverage check reads only the facade's examples, so an engine
fixture cannot accidentally satisfy it.
