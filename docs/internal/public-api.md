# Public API — the `jidousha` facade and `docs/api/`

Status: **design draft, pre-implementation.** Owns: what game agents see.
**CONTRACT** items binding as elsewhere. The facade crate is built last
(milestone F0, after subsystems exist), but every subsystem PR must conform to
this inventory — additions to it go through this doc first.

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
`run` lands with the platform crate (M5); it is the same loop with a different
driver (core.md §8).

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

Rough count: ~45 types/functions. CONTRACT: the v1 prototype substrate
("agent Pong/asteroids/breakout") must be expressible with this list alone —
that's exactly what acceptance milestone E0 tests (implementation plan).

## 3. Design notes

- `Depth { layer, z }` (with `Depth::default()` = (0, 0.0) and
  `Depth::layer(i16)`) is the uniform depth argument for immediate primitives;
  sprites carry `layer` in `Sprite` and `z` in `Transform` because they're
  entity data. This asymmetry is DELIBERATE: components are the entity-driven
  path, `Depth` is the immediate path; merging them made both worse.
- `jidousha::testing` (InputScript, transcript assertion helpers) is public but
  documented in its own short section of `docs/api/` — game agents use it in
  their games' tests, which agents should be writing too.
- `RunError` is the §9-taxonomy environmental class (no GPU adapter, etc.);
  everything else at the API surface panics per taxonomy with agent-grade
  messages.

## 4. `docs/api/` — the generated game-agent surface

- **One file**, `docs/api/jidousha-api.md`, generated by `tools/gen-api-doc`
  from the facade crate's rustdoc (mechanism (impl); likely rustdoc JSON).
  CI fails when stale (practices §2.3) or when over **25k tokens** (counted in
  CI; the budget is the point — it must fit comfortably in a game-writing
  agent's context alongside the game itself).
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
- CONTRACT: `docs/api/` never mentions internal crates, the backend seam,
  archetype storage, or any implementation vocabulary. Quality bar (practices
  §2.3): a fresh agent with only this file + `examples/` ships a working
  prototype. E0 is the test.

## 5. Examples as API fixtures

`examples/` is part of the public surface (practices §5.1): `headless_sim`,
`window_clear`, `sprites`, `prototype_kit`, `input_echo`, `homing`,
`spawn_and_reap`, plus `quickstart` (the docs/api embed). CONTRACT: every §2 item appears in at least
one example; `tools/check-api-coverage` (grep-level) enforces in CI.

**Before F0**, the facade does not exist, so an example has no `jidousha` crate
to depend on. Examples written in the meantime live beside the crate they
exercise (`crates/<crate>/examples/`) and name that internal crate — `homing`,
added with ADR-0013's refinements, is the first. This is a knowing, temporary
breach of the "games depend on `jidousha` only" contract above: at F0 these
examples move to the root `examples/` directory, are rewritten against the
facade, and the CI grep that enforces the contract lands with them. `tools/test`
already runs every workspace example, wherever it lives.
