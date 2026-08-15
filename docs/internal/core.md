# Engine core — design and contracts

Status: **living internal doc**, part spec and part record. Implemented through
M1 (entities and single-table storage); everything from M2 on is still the spec
the implementing agent builds against. Sections describing shipped code say what
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

## 5. Queries

```rust
for (e, pos, vel) in world.query::<(&mut Position, &Velocity)>() { ... }
```

- Tuple of `&T` / `&mut T` component accesses, plus minimal filters:
  `With<T>`, `Without<T>`. Entity is always available as the first yield.
- Queries take `&World`; column-level borrow flags (RefCell-style, per component
  type) enforce aliasing rules at runtime. Sequential single-threaded systems
  make conflicts rare — they arise only from nested/overlapping queries — and
  when they occur the panic message names both queries, the component type, and
  the running system (§8 format).
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

**Engine RNG** — a seeded PCG-class `Rng` resource created from `GameConfig::seed`.
CONTRACT: `jidousha-core` and game simulation code use only this. `rand::thread_rng`
and OS entropy are banned in simulation paths (doctor/lint-checked).

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
  trig, clippy-enforced (ADR-0009). `jidousha-core::math` lands in M3.
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
  exercising dead-entity paths. Storage is one table with an absent-slot
  `Option` per row, swap-removed on despawn: the archetype graph is M2's job,
  and M1 exists to pin the observable semantics first.
- **M2 — archetypes + queries.** Archetype graph, entity moves on insert/remove,
  tuple queries + `With`/`Without`, borrow flags with the §9 message format.
  Iteration-determinism tests (same op script twice → identical iteration
  transcripts). Exit: model + determinism tests green.
- **M3 — resources, commands, schedule, time.** Phases, registration-order
  execution, command buffers with end-of-system application, `Time` +
  accumulator loop (driven manually), seeded `Rng`. **Replay test: random system
  soup + scripted inputs, run twice, world state hash identical — the ADR-level
  determinism contract becomes a regression test here.** Exit: replay green.
- **M4 — app + headless.** `GameConfig`, `run`/`headless` with the shared loop,
  `schedule_debug`, panic hook with system names. Typed phases + `DrawCtx` with
  the read-only world view (ADR-0008); submission sink stubbed until the
  renderer lands. Compile-fail tests (`trybuild`) locking in the
  `on_unimplemented` error text for `&mut` access in Draw. First example:
  `examples/headless_sim.rs` (pure simulation, asserts on state, no window).
  Exit: example runs in CI on all targets incl. wasm.
- **M5 — platform crate.** winit wrapper, window, event pump → `InputSnapshot`
  scaffold (full input system is its own doc), real-time loop driver feeding the
  accumulator. `examples/window_blank.rs` opens a window natively and on web
  (manual check; headless CI proxy per ADR-0005). Exit: blank window on all
  three targets; core still has zero platform deps.

Renderer design doc picks up from M5.
