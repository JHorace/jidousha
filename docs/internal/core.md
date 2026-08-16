# Engine core — design and contracts

Status: **living internal doc**, part spec and part record. Implemented through
M4 (the engine core, end to end: ECS, schedule, time, app lifecycle, math);
M5 and the subsystem milestones are still the spec the implementing agent
builds against. Sections describing shipped code say what
the code does — where the two ever disagree, the code is the bug. Contracts
marked **CONTRACT** are binding — tests must encode them, and changing one
requires an ADR.

Covers: workspace layout, ECS (entities, components, storage, queries, resources,
commands), schedule, time and the main loop, app lifecycle, error taxonomy, and
implementation milestones. Does NOT cover: renderer, assets, input internals —
those get their own docs; only their boundaries with the core appear here.

Decisions inherited: Rust (ADR-0001), full ECS (ADR-0002), custom implementation
(ADR-0006), callback-shaped lifecycle and web constraints (ADR-0004, ADR-0005).

---

## 1. Workspace layout

Cargo workspace; crate boundaries are the enforcement mechanism for the isolation
rules (a crate that lacks the `wgpu` dependency cannot leak `wgpu` types).

```
crates/
  jidousha-core         ECS, schedule, time, app model. No I/O, no platform deps.
                        Compiles on every target incl. wasm with zero cfg branches.
  jidousha-platform     winit wrapper: window, event pump, loop driver.
                        ONLY crate with a winit dependency.
  jidousha-render-core  Renderer front end + backend interface (trait).
  jidousha-render-wgpu  wgpu backend. ONLY crate with a wgpu dependency.
  jidousha-assets       Poll-based async asset loading (docs/internal/assets.md).
  jidousha-input        Engine input types + per-tick snapshot building.
  jidousha              Facade: the public API games see. Re-exports the curated
                        surface; `docs/api/` is generated from THIS crate only.
examples/               canonical programs (public-API only)
tools/                  doctor, test, verify, gen-api-doc, checks
```

CONTRACT: `jidousha-core` has no dependency on any other jidousha crate and no
platform-conditional simulation logic. Everything below in this doc lives in
`jidousha-core` unless stated otherwise.

---

## 2. Entities

```rust
pub struct Entity { index: u32, generation: NonZeroU32 }
```

- Copyable opaque ID. No pointers, no lifetimes. The only way game code refers to
  a thing in the world.
- **Generational**: despawning bumps the slot's generation; stale `Entity` values
  are detectably dead, never silently reused.
- CONTRACT (determinism): allocation is a pure function of operation history.
  Free slots are reused LIFO from a free list. No randomness, no hashing of
  addresses, no platform variation.
- Debug format: `Entity(index vGen)`, e.g. `Entity(17 v3)` — greppable in logs
  and error messages.

Liveness API (per the verb conventions):

- `world.is_alive(e) -> bool`
- Operations on dead entities follow the error taxonomy (§8): structural ops
  (`despawn`, `insert`, `remove`) on a dead entity are contract violations —
  loud panic in debug with the entity, its generation, and when it died if known;
  `try_*` variants exist for the rare legitimately-racy gameplay cases.
- Implemented (M1): the panic and the `try_*` `Result` carry the *same* text —
  one message, two deliveries. "When it died" is reported as the generation the
  slot now holds, since there is no clock to name a tick until M3.
- CONTRACT (M1): `remove::<T>` states an end state — the entity has no `T`
  afterwards — so it is idempotent; removing a component the entity never had is
  not a failure. Only the entity being dead is. Likewise `insert::<T>` replaces
  any `T` already present.
- `world.entity_count()` reports how many entities are alive.

## 3. Components

- Plain Rust structs/enums. `derive(Component)` implements the marker trait only —
  the derive generates **no new public symbols** (greppability rule).
  Implemented (M1): the trait, with `impl Component for Foo {}` written by hand.
  The derive needs a proc-macro crate, which is not in the layout (§1) and would
  be the workspace's first dependencies; it lands with the facade (F0), where the
  public ergonomics are settled. Because the derive expands to exactly the manual
  impl, nothing written against the trait today changes when it arrives.
- Bounds: `'static + Send + Sync`. (Single-threaded today; these bounds are free
  now and unlockable later — adding them retroactively breaks every game.)
- No component registration step: types are registered lazily on first use.
  Registration order therefore MUST NOT affect observable behavior (see §4).
- Zero-sized components are supported and idiomatic as tags (`Frozen`, `Player`).

CONTRACT: components hold data only. No methods with game logic beyond cheap
accessors/constructors; logic lives in systems. (Convention-enforced; called out
in review and the `add-subsystem` skill.)

## 4. Storage and iteration order

Archetype-based storage: entities with the same component set share an archetype;
components are stored in dense parallel columns.

The determinism contract is deliberately precise about what is and is not promised:

- CONTRACT: iteration order is a **deterministic function of world operation
  history**. Same seed + same operation sequence → same iteration order, on every
  platform, every run.
- Iteration order is NOT sorted by entity ID, NOT spawn order after removals
  (structural removal may swap-remove within a column), and NOT part of the public
  API's stability promise between engine versions. Game code needing an order
  sorts explicitly.
- Rationale: replay/golden-state verification needs reproducibility, not
  prettiness. Swap-remove keeps structural ops O(1) and is still fully
  deterministic given identical history.
- CONTRACT: archetype visit order for a query is deterministic (archetype
  creation order), and lazy type registration (§3) must not perturb any
  observable order.

Forbidden in `jidousha-core`: `HashMap`/`HashSet` iteration in any code path that
feeds observable state or ordering. Use `Vec`, index maps with stable order, or
sorted iteration. (Grep-checkable; part of `tools/check-*` lints.)

Implemented (M2), for the reader who needs to know how the promises are kept:

- An archetype's identity is its **sorted** `Vec<TypeId>`, so inserting the same
  components in different orders lands both entities in the same archetype.
  Sorting is by `TypeId`, whose ordering is not stable across compilations —
  that is fine, because nothing observable depends on *column* order.
- Archetypes live in a `Vec` in creation order and are found by linear scan of
  that `Vec`. No hash map is involved anywhere in the lookup, which is what
  makes the visit-order contract hold rather than merely happen to hold.
- Adding or removing a component moves the entity: its row is appended to the
  target archetype, every shared component is moved value-by-value, and the
  vacated row is swap-removed. The entity swapped into the hole is the only
  other one whose location changes, and the world repairs it before returning.
- `World` keeps a `Vec<Option<Location>>` from entity slot to (archetype, row).
  Entity handles never change; rows do, constantly.

## 5. Queries

```rust
for (e, pos, vel) in world.query_mut::<(&mut Position, &Velocity)>() { ... }
for (e, pos) in world.query::<&Position>() { ... }
```

- Tuple of `&T` / `&mut T` component accesses, plus minimal filters:
  `With<T>`, `Without<T>`. Entity is always available as the first yield. A
  filter yields `()`, so it holds a position in the item tuple:
  `for (e, pos, _) in world.query::<(&Position, With<Player>)>()`.
- **Reading takes `&World` (`query`), writing takes `&mut World` (`query_mut`)**
  — ADR-0013, which supersedes this section's original "queries take `&World`
  plus runtime borrow flags". That pairing was unsound next to bare-reference
  point access below: a `&mut T` yielded from a `&World` query and a `&T` from
  `component()` could alias with nothing to stop them. Overlapping access is now
  a compile error rather than a runtime panic, and `jidousha-core` needs no
  `unsafe`. `&mut T` in a read-only `query` is rejected by an
  `on_unimplemented` message in the §9 style.
- The one aliasing case the type system cannot see — a query naming the same
  component twice, `(&mut Position, &Position)` — panics with the §9 format,
  naming the component.
- Sharp edge (ADR-0013): a mutable query borrows the whole world, so game code
  cannot point-read another entity while iterating. See the pattern below.

### Reading other entities while mutating: the read-pass/write-pass pattern

This is the one workaround for ADR-0013's sharp edge, and it is canonical: any
system where entity A's write depends on entity B's data is written this way.
The tempting version does not compile, which is the point — the error arrives
at `cargo check`, not in a playtest:

```rust
for (_, position, target) in world.query_mut::<(&mut Position, &Target)>() {
    let goal = world.component::<Position>(target.0);   // ✗ world is exclusively borrowed
}
```

Split it into a read pass that collects and a write pass that consumes:

```rust
let mut goals: Vec<(Entity, Position)> = Vec::new();
for (missile, _, target) in world.query::<(&Position, &Target)>() {
    if let Some(goal) = world.find_component::<Position>(target.0) {
        goals.push((missile, *goal));       // read pass: world stays readable
    }
}
for (missile, goal) in goals {              // write pass: consumes what was read
    let position = world.component_mut::<Position>(missile);
    position.x += (goal.x - position.x).signum();
}
```

Working code: `crates/jidousha-core/examples/homing.rs` (homing missiles), which
runs in CI and asserts its own results.

- The `Vec` is the whole pattern. **No helper API exists, and none should be
  added**: one way to do everything (practices §5.3) applies to patterns as much
  as to functions, and a `collect_then_write` helper would be a second spelling
  of `collect()` that hides where the allocation happens.
- The read pass is also the natural place to filter, so the write pass usually
  touches fewer entities than a nested version would have.
- From M3 there is a second legitimate form: record the work as **commands**
  (§6) during the read pass and let them apply at the end of the system. Use
  that when the work is structural (spawn/despawn/insert/remove); use the
  collect form when it is a plain component write.
- Determinism is unaffected either way: both passes iterate in the same
  archetype-and-row order (§4), and the `Vec` preserves it.
- Point access: `world.component::<T>(e) -> &T` (panics with agent-grade message
  if absent/dead), `world.find_component::<T>(e) -> Option<&T>`, plus `_mut`
  variants. Per conventions: `get`-class infallible, `find`-class Option.

DELIBERATE: no Bevy-style system parameters (`fn sys(q: Query<...>, r: Res<...>)`)
— see ADR-0007. Systems are plain `fn(&mut World)`; queries are constructed
inline. Rationale recorded in the ADR: parameter extraction exists to enable
parallel scheduling we deliberately don't have (ADR-0002), its trait machinery
produces the worst error messages in the Rust game ecosystem (the anti-goal of
this engine), and plain functions are fully greppable with zero macro magic.

## 6. Resources and commands

**Resources** — typed singletons for world-global state (`Time`, `Rng`, input
snapshot, game state structs):

- `world.resource::<T>() -> &T` / `resource_mut` (panic-class),
  `find_resource` → Option, `insert_resource`, `remove_resource`.
- Same borrow-flag discipline as component columns.

**Commands** — deferred structural mutation. Direct structural ops on `&mut World`
(spawn/despawn/insert/remove) exist for setup code, but during a query iteration
the world is logically borrowed; systems record structural changes instead:

```rust
let mut cmd = world.commands();
cmd.spawn((Position::ZERO, Sprite::new(tex)));
cmd.despawn(e);
// applied automatically when the system returns
```

- CONTRACT: commands apply **at the end of the recording system, in recording
  order**, before the next system runs. One rule, no configuration. The next
  system always sees the previous system's structural changes.
- Command application is itself part of operation history (§4 determinism).

Implemented (M3):

- `world.commands()` takes `&World` and hands back a recorder over a buffer the
  world owns behind a `RefCell`. That interior mutability is what lets a system
  record while a read-only query holds the world — it guards the buffer only,
  never component or resource data, so it cannot be used to reach around
  ADR-0013. Taking a second recorder while one is alive panics.
- The schedule calls the world's flush after **every** system, which is what
  makes the "applied when the system returns" contract true rather than
  aspirational. A command that records more commands (a spawn applying its
  bundle) is still applied in the same flush, in order.
- CONTRACT: a command naming an entity that is no longer alive is a **no-op**,
  not a failure. Deferral exists because the world moves between recording and
  application: another system may legitimately have despawned the entity first,
  and the command's intent is then already satisfied or moot. This is the same
  reasoning that makes `remove` idempotent (§2).
- `commands.spawn(bundle)` returns nothing: the handle is allocated at
  application time. Give the entity what it needs through the bundle; a system
  that must hold the handle spawns directly on `&mut World`. Bundles are tuples
  of components, so one component is `(Frozen,)`.
- Determinism note learned from the mutation checks: the replay test does *not*
  catch a reordered command buffer, because reversal is still deterministic.
  Replay proves repeatability; the recording-order CONTRACT needs its own test,
  and has one (`tests/commands.rs`).

**Engine RNG** — a seeded PCG-class `Rng` resource created from `GameConfig::seed`.
CONTRACT: `jidousha-core` and game simulation code use only this. `rand::thread_rng`
and OS entropy are banned in simulation paths (doctor/lint-checked).

Implemented (M3): PCG32 (XSH-RR), integer arithmetic only, so the sequence is
bit-identical everywhere. `next_u32`, `below(limit)` (rejection-sampled, so the
distribution is even), and `next_f32` built from the top 24 bits. Until M4's
`GameConfig` exists the seed is an argument to `Simulation::new`.

## 7. Schedule, time, and the loop

### Schedule

Three phases, fixed set for v1:

- **Startup** — runs once before the first tick.
- **Update** — the simulation. Runs on the fixed timestep, possibly 0..n times
  per rendered frame.
- **Draw** — runs once per rendered frame, after Update catches up. Reads world,
  produces draw submissions (renderer doc owns what those are). **Read-only by
  type (ADR-0008)**: Draw systems are `fn(&mut DrawCtx)`; `ctx.world` is a
  read-only view whose `query` accepts only `&T` access (trait-bound with a
  `#[diagnostic::on_unimplemented]` message in the §9 style), and `ctx.draw(...)`
  is the submission sink. Phases are types naming their system signature, so
  registering an Update-shaped fn in Draw is a compile error. The verification
  harness keeps a world-hash check across Draw as defense-in-depth against
  interior-mutability escapes.

```rust
app.add_system(Update, physics_system);   // appended; runs in registration order
```

- CONTRACT: within a phase, **observable behavior is that of sequential execution
  in registration order**. v1 implements this literally (single-threaded, no
  dependency solver). A future parallel scheduler (own ADR) may run provably
  non-conflicting systems concurrently but must preserve this observable
  ordering — game code written against v1 stays valid unmodified.
- Parallel-pivot headroom (kept warm, not built): components/resources are
  already `Send + Sync`; systems may communicate only through world state; and
  the borrow-flag machinery can record each system's observed access set in
  debug builds — empirical conflict graphs come free the day a parallel
  scheduler needs them, with no signature changes (ADR-0007).
- The full schedule is printable: `app.schedule_debug()` lists phases and system
  names (function-name-derived) in run order — one call answers "what runs when"
  for any debugging agent.

Implemented (M3):

- Phases are types (`Startup`, `Update`) with an associated `Context`, which is
  how Draw will refuse Update-shaped functions once it lands with `DrawCtx`
  (ADR-0008). Each phase owns its list; the lists are `Vec`s, so registration
  order *is* run order.
- `add_system` is generic over the function's own type, which is the only way to
  read its name: `type_name` on a fn item gives the path, and the last segment
  is what the listing shows. Registering a closure yields `{{closure}}`, which
  is self-punishing and therefore the whole enforcement.
- `schedule_debug` landed in M3 rather than M4 — it costs nothing once names
  are captured, and a schedule you cannot print is a schedule you cannot debug.
- Implemented (M4): `Draw` joins the phase set. `IntoSystem<P>` is where a
  phase's signature is enforced, and it carries the `on_unimplemented` text.
  A caveat worth knowing, found by the compile-fail harness: registering a
  Draw-shaped function in Update trips rustc's own signature mismatch (E0631)
  *before* that text fires, so what an agent sees there is rustc naming both
  signatures and mentioning `IntoSystem<Update>` — informative, but not our
  sentence. The `&mut T`-in-a-Draw-query case, which ADR-0008 predicts is the
  common mistake, does show the engine's own message.
- Implemented (M4): the world-hash check ADR-0008 asks for is a *structural*
  comparison across Draw (entity count, archetype count, live locations),
  asserted in debug builds. A component mutated through a `Cell` would slip
  past it; catching that needs hashing component bytes, which the engine cannot
  do for types it does not know. The type system remains the real enforcement,
  and this is the defense in depth behind it.

### Time and the fixed timestep

The `Time` resource is the ONLY clock simulation code may observe:

```rust
pub struct Time {
    pub tick: u64,          // Update ticks since startup — THE canonical timeline
    pub fixed_dt: Seconds,  // constant per run; default 1/60
    pub elapsed: Seconds,   // tick * fixed_dt
    pub alpha: f32,         // Draw-phase only: interpolation fraction [0,1)
}
```

Loop shape (standard accumulator):

```
frame:
  platform pumps events → input snapshot(s)         (jidousha-platform)
  accumulator += real frame time (clamped, max 0.25s)
  while accumulator >= fixed_dt:
      run Update phase once (tick += 1)             // input snapshot fixed per tick
      accumulator -= fixed_dt
  alpha = accumulator / fixed_dt
  run Draw phase once
```

- CONTRACT (the engine's central promise): **simulation state is a pure function
  of (seed, registered systems, per-tick input snapshots).** Native and web
  included — same seed and inputs replay to identical state everywhere. The
  verification harness's replay tests encode exactly this.
- Wall-clock types (`std::time::Instant`, `SystemTime`) are banned outside
  `jidousha-platform` (CI grep check). Real frame time enters the loop only as
  the accumulator input, at the platform boundary.
- f32 discipline for cross-platform determinism: no `fast-math`-style flags, no
  platform intrinsics in simulation, no `f32::sin`-class functions in engine
  simulation code where bit-exactness matters — the math crate decision (own vs
  `glam`, and its determinism guarantees) is called out as an open question in §10.

### Input boundary (interface only; input doc owns internals)

`jidousha-input` builds an `InputSnapshot` per Update tick from platform events.
CONTRACT: the snapshot is a plain-data value, the complete input truth for that
tick, and is recordable/replayable. Simulation reads input ONLY via the snapshot
resource — never via events or platform callbacks. This single choke point is what
makes replay work. Pointer input is modeled as pointers (not "the mouse") so touch
(Android, ADR-0005) does not force a redesign.

## 8. App lifecycle

Callback-shaped (web requirement, ADR-0004/0005 — desktop is web-shaped, not
vice versa):

```rust
fn main() {
    jidousha::run(GameConfig {
        title: "asteroids",
        seed: 42,
        ..GameConfig::default()
    }, |app| {
        app.add_system(Startup, spawn_level);
        app.add_system(Update, player_control);   // fn(&mut World)
        app.add_system(Update, physics);
        app.add_system(Draw, draw_sprites);       // fn(&mut DrawCtx) — ADR-0008
    });
}
```

- `run` never returns (winit/web semantics). All game setup happens in the
  closure; all game logic lives in systems.
- **Headless is a first-class mode, not a cfg hack**:

```rust
let mut sim = jidousha::headless(config, |app| { ...same closure... });
sim.tick(InputSnapshot::default());   // drive Update manually, one tick
sim.world();                          // inspect state
```

  `headless()` lives in `jidousha-core` (no platform/render deps), runs Startup +
  Update phases only, and is the substrate for `tools/verify`, replay tests, and
  every core integration test. CONTRACT: `run` and `headless` execute Startup and
  Update identically — one loop implementation, two drivers.

Implemented (M5): the CONTRACT is now structural. Both drivers call
`jidousha_core::build(config, setup)` to construct the simulation, and both run
ticks through `Simulation::advance`, which owns the accumulator. `run` lives in
`jidousha-platform` and adds only the two things a window brings: when a frame
happens, and how long it was. On the web `run` returns immediately and the
browser keeps calling back — the one `cfg` branch in the engine, in the crate
that exists to absorb exactly this difference.

## 9. Error taxonomy

Two classes, engine-wide (renderer/assets docs inherit this):

1. **Contract violations** (bugs in game or engine code): using a dead entity,
   missing component via `get`-class access, borrow conflicts, dead resource.
   → **Panic** with agent-grade message. Not `Result` — these are unrecoverable
   programming errors, and a panic with a great message is the fastest possible
   repair-loop signal.
2. **Environmental/expected failures** (missing asset file, bad data, backend
   loss): → `Result` with the same message discipline. Never a silent fallback.

Message format (CONTRACT — tested via UI-test-style snapshot tests):

```
[jidousha] <what happened>
  <specifics: entity/component/system names and values>
  likely cause: <the most common mistake producing this>
  fix: <the concrete change to make>
```

Example:

```
[jidousha] component access failed: Position not present on Entity(17 v3)
  in system: player_control (Update)
  likely cause: entity was spawned without Position, or a previous system removed it
  fix: use world.find_component::<Position>(e) if absence is expected here,
       or add Position at spawn in spawn_level
```

The running system's name is always included — the panic hook knows the schedule
position. Release builds keep the same messages (they're cheap; string formatting
only on the failure path).

## 10. Open questions (deferred, tracked here)

- ~~Math crate~~ — resolved: glam (`scalar-math`) + engine-owned deterministic
  trig, clippy-enforced (ADR-0009). Landed in M4 (moved from M3, which needed no
  vectors). `math::{sin_cos, atan2, rotate}` evaluate polynomials in `f64` over
  IEEE add/multiply/round only, so the same angle gives the same bits on every
  platform; a test locks those bits, and the accuracy tests compare against std
  trig — the one sanctioned use of it, since the ban exists because *platforms*
  disagree, not because std is wrong here. glam's measured cost: **1 crate, zero
  transitive** (practices §5.8), exactly as the ADR predicted.
- ~~Draw-phase immutability~~ — resolved: type-enforced via `DrawCtx` (ADR-0008).
- **Change detection / events / parallelism**: explicitly out of v1 (ADR-0006).
  Each returns only via its own ADR with a driving use case.

## 11. Implementation milestones

Each milestone = mergeable, tested, green CI (fmt, clippy `-D warnings`, tests,
wasm check). Ordered so every step has something verifiable. The implementing
agent works one milestone per session-ish; BLOCKED.md protocol applies throughout.

- **M0 — scaffold.** ✅ Done. Workspace + empty crates, CI pipeline,
  `tools/doctor`, `tools/test` (report file + timeouts + failure counter),
  CLAUDE.md size check. Tooling is documented in `docs/internal/tooling.md`.
- **M1 — entities + single-archetype storage.** ✅ Done. Entity allocator
  (generational, LIFO free list), one-archetype world:
  spawn/despawn/insert/remove/point access. **Property tests against a naive
  reference model** (`crates/jidousha-core/tests/support/`, a `Vec` of slots
  holding a `BTreeMap` of components, compared under 2000 random operation
  sequences) — this reference model is load-bearing for every later milestone.
  The generator is seeded and stdlib-only, so a failure names the seed and the
  shortest failing prefix; a second test guards that the sequences keep
  exercising dead-entity paths. Storage was one table with an absent-slot
  `Option` per row — replaced by real archetypes in M2, which the same model
  tests then held to the same semantics.
- **M2 — archetypes + queries.** ✅ Done. Archetypes keyed by their sorted
  component set, entity moves on insert/remove, tuple queries + `With`/`Without`.
  Aliasing is enforced by the borrow checker rather than runtime borrow flags —
  the original plan was unsound beside bare-reference point access (ADR-0013) —
  leaving one runtime check, for a component named twice in one query.
  Iteration-determinism tests (same op script twice → identical iteration
  transcripts) live in `tests/query.rs`; the reference model gained query
  comparison and a `query_mut` operation.
- **M3 — resources, commands, schedule, time.** ✅ Done. Phases with
  registration-order execution, command buffers applied after every system,
  `Time` + the accumulator loop, and the seeded `Rng`. The loop lives in
  `Simulation` (`src/simulation.rs`) — one implementation that M4's `run` and
  `headless` both wrap, per §8's CONTRACT. **The replay test** (`tests/replay.rs`)
  runs a system soup against a scripted input track for 200 ticks, twice,
  hashing the world after every tick; the hash covers iteration order, so an
  ordering change fails it too. Sibling tests guard that the run actually churns
  (spawns, freezes, reaps) and that seed and inputs each reach the simulation.
  `schedule_debug` landed here rather than in M4, since system names are
  captured at registration anyway.
- **M4 — app + headless.** ✅ Done. `GameConfig` (title, seed, fixed_dt; the
  asset/window/camera fields arrive with their subsystems), `App`, `headless` →
  `HeadlessSim`, `schedule_debug`, and the panic hook that names the running
  system — which also inserts §9's `in system:` line into every engine message.
  Typed phases with `IntoSystem<P>`: `Draw` takes `&mut DrawCtx`, whose
  `WorldView` has no method that mutates (ADR-0008). Compile-fail tests lock the
  error text, via `tools/check-compile-fail` rather than `trybuild` (27
  transitive dev-dependencies against a budget that prefers none; the mechanism
  is sixty lines). `jidousha-core::math` landed here: glam with `scalar-math`
  (+1 crate, zero transitive) and engine-owned polynomial trig, with std trig
  and glam's angle constructors clippy-banned.
  `examples/headless_sim.rs` runs a whole game with no window and asserts a
  bit-identical replay of itself; CI builds every example for wasm as well as
  running them natively.
  Two things the milestone named that are **not** here, with reasons:
  `run` (the windowed driver) belongs to the platform crate and lands in M5 —
  core has no window to drive; and the submission sink stays absent rather than
  stubbed, because the renderer owns its vocabulary and a placeholder would
  only have to be unlearned. What M4 delivers is the Draw *signature*, so no
  game's draw systems need rewriting when the sink arrives.
  The sink landed in **R0**, and the wait paid off: it is `DrawCtx::submit`
  taking a `Quad`, plus `Color`/`Rect`/`Depth`/`TextureId`/`Transform` as the
  vocabulary it needs. Those are rendering-shaped types in the ECS crate, which
  needed a decision of its own — core may depend on no other jidousha crate
  (§1, CONTRACT) and so cannot name a texture asset. **ADR-0015** records how
  that was resolved: core carries an opaque id, and every crate that knows what
  a texture *is* sits above it. No game's draw systems were rewritten.
- **M5 — platform crate.** ✅ winit wrapper, window, event pump → `InputSnapshot`
  scaffold (full input system is its own doc), real-time loop driver feeding the
  accumulator. `examples/window_blank.rs` opens a window natively and on web
  (manual check; headless CI proxy per ADR-0005). Exit: blank window on all
  three targets; core still has zero platform deps.

  `jidousha_platform::run(config, setup)` is the windowed driver, taking the
  same closure `headless` takes. Two small additions to core made that share one
  loop rather than two:

  - `jidousha_core::build(config, setup) -> Simulation` — the construction both
    drivers go through. `headless` is now a wrapper around it. The CONTRACT in
    §8 stops being a promise about the code and becomes a fact about it.
  - `Simulation::advance` gained a per-tick callback, `(&mut World, tick_index)`.
    The driver needs it to honor input.md §2: a frame's events belong to its
    first tick and the catch-up ticks behind it see no edges. Core cannot name
    an `InputSnapshot` (§1, CONTRACT), so the driver reaches in rather than core
    reaching out. The accumulator stays in one place, which was the point.

  **Verified, and not.** The engine side is tested: the driver's frame logic
  runs headless in unit tests (edges to the first tick only, edges surviving a
  frame that ran no ticks, focus loss releasing held keys, one draw per frame,
  no spiral of death), and both targets compile in CI. **The window itself is
  unverified.** This environment has no display — `run` correctly reports
  `no display to open a window on` and names `headless` as the thing to do
  instead — so "a blank window appears on Linux, Windows and the web" is still
  waiting on a human with a screen. That was always specified as a manual check
  (ADR-0005's headless CI proxy); it is called out here so nobody reads the tick
  above as more than it is.

  What the mutation checks said. Seven deliberate breakages, six caught. Three
  of the six were caught only after the tests were strengthened — the first pass
  asserted `submissions().is_empty()` for "did the frame draw", which is true
  whether it drew or not, and reached past winit's event vocabulary by recording
  on the builder directly, so the whole `WindowEvent` arm was unexercised. Both
  are now real: a Draw-phase counter, and the event handling extracted into a
  method that needs no `ActiveEventLoop` to call.

  The seventh is genuinely equivalent and is left standing: giving every tick
  `first_tick_snapshot()` instead of splitting on the tick index changes
  nothing, because that call spends the builder's edges and the second call
  returns what the catch-up snapshot would. The split stays in the code because
  it states the contract, and carries a `DELIBERATE:` tag saying it is currently
  unobservable — the opposite of I0's finding, where the contract turned out to
  be stronger than the test rather than weaker.

  A bug this milestone's tests caught, worth keeping: the first version took the
  frame's input snapshot *before* `advance` decided how many ticks to run, which
  spends the builder's edges. On a machine drawing faster than it ticks — every
  machine, most frames — a key pressed during a frame that ran no ticks was
  silently dropped. Taking the snapshot inside the per-tick callback fixes it,
  and `a_frame_too_short_for_a_tick_runs_none_and_keeps_the_edges` is the test
  that found it.

  Dependency delta (practices §5.8): `winit` 0.30, the first non-glam dependency
  and the one ADR-0004 already accepted. 1 → 183 external crates workspace-wide,
  70 in a native build's tree. Features are listed rather than inherited so the
  footprint changes only when someone decides it does; `wayland-csd-adwaita` is
  in the list deliberately, because without it a Wayland window has no title bar
  or close button, which would make the milestone's own manual check worse.
  `web-time` comes with it and is the wall-clock shim ADR-0005 asked for.

Renderer design doc picks up from M5.
