# Asset loading basics — design and contracts

Status: **living doc for `jidousha-assets`; A0, A1 and A2 implemented, plus the
I2 replay seam; A3 still design.**
Sections carry `Implemented (AN)` notes where code exists; everything else is
design ahead of the code. **CONTRACT** items are binding and tested.

Inherits: async-by-design + no-wall-clock (ADR-0005), poll-based API / no async
runtime (ADR-0011), placeholder policy and TextureHandle consumption
(renderer §5), WebGL2 texture limits (renderer §8), error taxonomy (core §9),
determinism contract (core §7).

In scope (v1): texture loading (PNG), raw byte loading, handle/state model,
deterministic readiness, the platform byte-source seam, path policy.
Out of scope (deferred): audio (no audio system yet), structured data loading
(serde — defer until a game needs it), asset streaming, hot reload, atlas
packing, asset bundles/packs, compression.

---

## 1. Concepts

**Handles** are copyable opaque IDs, same shape philosophy as `Entity`:

```rust
pub struct TextureHandle(AssetId);   // AssetId = index + generation
pub struct BytesHandle(AssetId);
```

**States**: `Loading → Ready | Failed`, plus destroyed-by-`unload`.

```rust
pub enum AssetStatus { Loading, Ready, Failed }
```

- `assets.load_texture("player.png") -> TextureHandle` — returns immediately,
  never blocks, never errors at call time (ADR-0011).
- `assets.status(handle) -> AssetStatus`; `assets.all_ready() -> bool`
  (true when every load requested so far has resolved — the one-line
  "loading gate" for games that want one).
- `assets.unload(handle)` — frees CPU and GPU memory. Using a handle after
  `unload` is a **contract violation** (debug panic, §9 message) — distinct
  from `Failed`, which is environmental (renders placeholder + one structured
  error; renderer §5). Rationale: unload-then-use is a bug the agent wrote;
  a missing file is a fact about the world.
- v1 lifetime policy: assets live until `unload` or program exit. **No
  refcounting, no automatic drop** — deterministic, simple, and prototypes
  never care. Revisit only with evidence of memory pressure.

The `Assets` API is a resource on the world (accessed like any other:
`world.resource_mut::<Assets>()`), so systems load assets without new plumbing.

Implemented (A0):

- `Assets::new(source)` is the one constructor (ADR-0012); the source is the §5
  seam, so a test store and a shipped store differ in one argument.
- Handles are generational exactly as `Entity` is, and print as
  `TextureHandle(3 v2)`. `unload` bumps the slot's generation, so a handle used
  afterwards is *detected*, never silently pointing at whatever took its place.
  Slots are reused LIFO, which makes handle allocation a pure function of the
  operation history — the same script hands out the same handles every run.
- `AssetHandle` is sealed: `TextureHandle` and `BytesHandle` are the whole set,
  and the slot-lookup half of it lives in a private supertrait, so game code can
  ask a handle its kind and cannot reach a slot index at all.
- The two kinds have separate tables. Mixing them up is a compile error rather
  than a lookup that quietly finds the wrong thing.

## 2. Paths

- One **asset root**: `assets/` beside the game manifest, configurable in
  `GameConfig`. All paths are relative to it: `load_texture("player.png")`,
  `load_texture("levels/1/bg.png")`. Forward slashes only, on every platform.
- Web serves the same directory over HTTP (same-origin, relative URL); native
  reads the filesystem. Identical path strings work identically. CONTRACT.
- **Case-strict everywhere, including Windows.** The native loader verifies the
  on-disk name matches the requested path byte-for-byte (directory-listing
  check) and fails with a §9 error if only the case differs. Rationale: the
  classic breakage is "works on my Windows machine, 404s on the web server" —
  we make the strict platform's behavior universal so it's caught on first run
  anywhere.
- **Convention: asset paths are string literals at the load site** — not
  computed, not concatenated (interpolating a *directory* of numbered levels is
  the sanctioned exception, flagged with a comment). This makes every asset
  reference greppable, which enables the CI check in §7 — a mistyped path
  becomes a CI failure, not a runtime placeholder hunt.

## 3. Formats and limits

- **v1 decodes PNG only**, via the `png` crate (small, pure Rust; record the
  `cargo tree` delta per §5.8 — expect a handful of crates). No `image`
  mega-crate: one format, one small dependency, one way to ship art.
  JPEG/others: deferred until someone actually needs them.
- CONTRACT: decoding uses the same `png` crate code path on every platform —
  never the browser's image decoder — so texel data is bit-identical
  everywhere. Golden-image tests (renderer §9) depend on this.
- Decoded textures are RGBA8, sRGB (conventions). Limits enforced at decode
  time with §9 errors: max 2048×2048 (renderer §8 envelope; the error message
  names the file, its size, and the limit).
- `load_bytes` hands back raw `Vec<u8>` for anything else a game invents.

## 4. Determinism: when "ready" happens

The subtle problem this section exists for: load *completion timing* is
environmental (disk speed, network, cache). If simulation can observe readiness
at arbitrary moments, the same game diverges between machines and between runs —
silently breaking replay (core §7).

Design:

- **Single commit point.** Completed loads (arriving on the loader channel /
  fetch callbacks) are held in a queue and applied — statuses flipped to
  Ready/Failed — at exactly one point in the frame: **before the first Update
  tick of the frame**. Between commit points, statuses are frozen; a mid-tick
  query never sees a transition. CONTRACT.
- **Readiness is part of the recorded timeline.** The replay recording
  (core §7's input stream) records, per tick, which assets committed
  Ready/Failed that tick. Replay applies them at the same ticks, regardless of
  actual load speed during playback. Readiness enters simulation through the
  same deterministic choke point as input. CONTRACT — and the reason `verify`
  runs are reproducible even though disk and network are not.
- Headless mode (`core §8`): `sim.tick(...)` takes the same recorded-readiness
  data; tests can script "texture X becomes ready at tick 30" and assert how
  the game behaves while waiting — loading behavior becomes *testable*.
- Practical note for game agents (this goes in `docs/api/`): you may branch on
  `status()`/`all_ready()` freely — it's deterministic under replay. The common
  pattern needs neither: draw immediately, placeholders resolve themselves.

Implemented (A0):

- `assets.commit(tick) -> Vec<AssetFailure>` is the commit point. It is the only
  code path that writes a status: every reader — `status`, `bytes_of`,
  `all_ready` — is a pure lookup, so "statuses are frozen between commits" is
  structural rather than a rule someone has to remember.
- `commit` panics if `tick` is earlier than the last one. Readiness is part of
  the timeline, and a timeline that runs backwards is a bug in the driver, not a
  state to tolerate (§9's no-silent-failure rule). Committing the same tick
  twice is legal and changes nothing.
- **Wired into the frame loop at R2** (this bullet previously said it was not):
  the driver's `settle_assets` runs Startup, commits at the frame's tick, and
  uploads, all before `advance` runs the frame's ticks — which is
  "before the first Update tick of the frame", implemented. Startup comes first
  because that is where a game builds its store and asks for its art; without it
  the first frame would find no store to commit. A game with no `Assets`
  resource is skipped, which is what most first prototypes are.
- **Deferred to the replay recording**: the per-tick record of *which* assets
  committed is a change to core's input stream, and lands when that recording
  format does. A0 delivers the half that makes it possible — readiness moves
  only at a numbered tick — and `MemorySource`'s scripted ticks stand in for the
  recording meanwhile, which is what makes the exit tests replayable today.

Implemented (I2): the recording exists, and the CONTRACT above is now tested end
to end.

- `Assets::resolved()` reports what the last `commit` resolved — a
  `Resolution { request, arrived }` per asset, in request order, cleared at each
  commit. A recorder reads it after committing and writes it into that tick's
  record; it is deliberately request *ids* rather than handles, because the
  recording lives in `jidousha-input`, which cannot name this crate's types.
- `ReplaySource` (§7) applies the recording on the way back: it wraps whatever
  source the replay actually has and releases each completion on the tick the
  recording gives it, so a disk that is fast today cannot change a session that
  was recorded on a slow one.
- `jidousha-platform/tests/record_replay.rs` is the proof, and its
  negative control is the part that matters: the same replay with the readiness
  records thrown away must *not* match. Without that assertion the whole
  mechanism could be inert and every other test here would still pass.

## 5. Internals: the platform seam and threading

```
jidousha-assets
  Assets            handles, states, CPU-side data, completion queue, commit()
  ByteSource trait  fn request(&mut self, path) -> RequestId  +  completion drain
jidousha-platform   provides the ByteSource impls:
  native            one loader thread, std::sync::mpsc; fs read + png decode off-thread
  web               fetch via wasm-bindgen; png decode on main thread at commit
```

- The trait seam mirrors ADR-0003's discipline: `jidousha-assets` never touches
  fs, fetch, or wasm-bindgen; platform crates own I/O. A third impl —
  `MemorySource` (preloaded path → bytes map with scripted completion ticks) —
  is the test/verify workhorse and ships in `jidousha-assets` itself.

Implemented (A0):

- `ByteSource` is two methods: `request` and `drain_completed(tick)`.
  CONTRACT: `drain_completed` is called only from `commit`, returns each
  completion exactly once, and orders one poll's completions by request id.
  That last clause is not decoration — a source that drains in hash order
  replays differently on the second run, and the exit tests catch it.

  It was three until I2: an `outstanding()` count, documented as what
  `all_ready` consulted. `all_ready` never consulted it — it walks the store's
  own entries, which already covers everything anyone asked for, because a
  request cannot exist without an entry. I2's mutation pass is what found it:
  deleting the term from `ReplaySource`'s implementation changed no test,
  because nothing read it. Three sources were each maintaining a counter for
  nobody, and a second way to ask "is anything in flight" could only ever
  disagree with the first.
- `MemorySource` stores its content in a `BTreeMap`, not a `HashMap` as this
  section originally said: an ordered map is the cheapest way to keep iteration
  out of the nondeterminism budget entirely (core §7).
- `ByteSource: Send + Sync` is inherited, not chosen — `Assets` is a world
  resource and resources are `Send + Sync`. A1's loader holds an
  `mpsc::Receiver`, which is `Send` but not `Sync`, so it wraps it in a `Mutex`
  that is never contended: the store is touched from one thread only.
- Unloading an asset whose bytes are still in flight drops its route. The bytes
  arrive at a later commit and are discarded, rather than landing in a slot
  something else now owns — the failure mode that made this worth a test.
- Native decode happens on the loader thread (PNG decode is the slow part);
  web decodes at the commit point (main thread — acceptable at prototype scale;
  PERF-revisit with evidence, options exist: workers, `createImageBitmap` — the
  latter forbidden by the identical-texels CONTRACT unless proven bit-exact).

Implemented (A2): exactly that. `WebSource`'s fetch callback pushes raw bytes
onto a queue and `drain_completed` decodes them, so decoding happens at the
commit rather than at whatever moment the network chose — which is the same
reason the completions are sorted by request id before they are returned. The
one thing the network controls is *which* commit an asset lands on, and that is
already on the timeline.
- GPU upload: at commit, newly-Ready textures are handed to render-core, which
  calls `backend.create_texture` at the next frame start. CPU-side pixels are
  then dropped (assets keep only metadata; `capture()`/goldens read from GPU).

Implemented (R2): `Assets::take_uploads` is the hand-off, and it **moves** the
texels out rather than lending them, which is what makes "then dropped" above a
property of the code rather than a step someone has to remember (ADR-0016).
Three details the sketch does not have:

- The queue holds ids and waits. There is frequently no renderer — a window
  arrives several frames into a program, and a headless run never has one — so
  nothing may be handed over eagerly or lost while waiting.
- Ids are queued at `commit`, so upload order follows commit order, which is on
  the timeline. Backend texture ids are assigned in upload order, so this is
  what makes two runs of the same script produce the same ids.
- A texture unloaded between the commit that readied it and the upload is
  dropped, checked by the generation on the queued id.

The upload happens at the *same* frame's start rather than the next one: the
driver commits, uploads, then ticks and draws, so art that became ready this
frame is drawn this frame.

## 6. Errors (§9 taxonomy applied)

```
[jidousha] asset failed: "sprites/Player.png"
  requested by: load_texture at examples/sprites.rs:12 (recorded at load site)
  likely cause: file exists as "sprites/player.png" — case mismatch (loads are
  case-strict on every platform; see docs/conventions.md)
  fix: rename the file or the path so they match exactly
```

- Failure classes with distinct messages: not found, case mismatch (detected
  separately, message names the near-miss file), decode error (names the byte
  offset/chunk), over-limit dimensions, HTTP error on web (status code + URL).
- Each failure is reported **once** (at commit), not per frame; the placeholder
  does the per-frame signaling visually.
- `load_*` records the callsite (`#[track_caller]`) so errors point at the
  requesting line, not the loader internals.

Implemented (A0):

- `AssetFailure { path, kind, requested_at, reason }`, returned from `commit` and
  formatted by `.message()` in core's §9 shape. `requested_at` is the
  `#[track_caller]` location, so the message names the game's line.
- A0 has **one** failure class: whatever the source reported. The distinct
  classes above — not found, case mismatch, decode error, over-limit — are
  things only a real loader can tell apart, and land with A1 and its snapshot
  tests. The shape they will be reported in is fixed now.
- "Reported once" is enforced by construction: `commit` drains the failure list
  it returns, so a second commit returns nothing to report.
- `message()` is public, and so is core's `message()` helper it delegates to —
  the other engine crates format identically or the §9 promise is only true
  inside core.

## 7. Verification and CI hooks

- **Asset-reference check** (`tools/check-assets`, in CI): extract string
  literals from `load_texture`/`load_bytes` callsites across examples and game
  code; verify each file exists (byte-for-byte case) under the asset root.
  Broken references fail CI before anything runs. (Enabled by the
  literal-paths convention, §2.)
- `tools/doctor` checks: asset root exists and is readable; for web runs, that
  the dev server serves it.
- `tools/verify` uses `MemorySource` with scripted readiness ticks by default —
  zero filesystem dependence, fully reproducible; a flag switches to real I/O
  for integration smoke tests.

Implemented (I2): `tools/verify` exists, and `examples/prototype_kit/`'s
`--verify` mode is the first user of it — a `MemorySource` with one scripted
arrival tick, so the placeholder is on screen for a known number of frames and
the art for the rest. No flag for real I/O yet: nothing has asked for one, and a
verify run that reads the disk is a verify run that can fail for reasons that say
nothing about the game.

Implemented (I2): `ReplaySource` (`replay.rs`) is the other half of §4 — it
wraps any source and releases its completions on the ticks a recording says,
holding early ones back — and a held completion leaves its asset `Loading`, so a
game's `all_ready` gate stays shut exactly as long as it did on the day.
`unreleased()` says how many are still waiting, which is how a replay that
stopped short can say so rather than looking like a shorter session. `Assets::resolved()` reports what a commit
resolved, in request order, which is what a recorder writes down.
- Golden transcript tests cover: placeholder → real texture swap at the
  scripted tick; Failed → placeholder + single error; unload → debug panic on
  use (a `should_panic` test locking the message).

Implemented (A1): `tests/file_source.rs` covers the §6 error set against real
files in a temporary asset root — missing, case mismatch, not-a-PNG, oversized,
a directory asked for as a file — and each assertion is on the *sentence*, so a
message that stopped naming the near-miss file would fail. The temporary root is
why these tests can create `Hero.png` beside `hero.png`, which is not something
to check into a repository that people clone onto case-insensitive filesystems.

Implemented (A0): the transcript tests exist in `tests/asset_replay.rs`, minus
the placeholder half, which needs a renderer (R2). The `should_panic` tests
locking the unload message are in `tests/asset_ops.rs`.

Implemented (R2): the placeholder half is
`jidousha-render-core/tests/loading_frames.rs` — a sprite drawn from the first
frame, whose texture is scripted to arrive at a known tick, asserted on both
sides of it; a sprite whose texture never arrives, drawing the placeholder
forever; and two sprites that share the placeholder's batch while waiting and
stop sharing when their art lands. It lives with the renderer rather than here
because the placeholder is the renderer's policy; the scripting is this crate's.

Wrinkle for A3: `examples/loading_gate.rs` loads from a `MemorySource`, so its
paths deliberately do not exist on disk. `tools/check-assets` must skip loads
whose store is a `MemorySource` rather than report them as broken references —
otherwise the check's first act is to fail on a correct example.

## 8. Milestones

Sequenced against renderer milestones (renderer needs textures at R2):

- **A0 — store + states + MemorySource.** ✅ Handles, statuses, commit point,
  scripted-readiness testing, `all_ready`, unload semantics + panics. No I/O,
  no GPU; runs everywhere incl. wasm CI. Exit: state-machine property tests
  green; readiness-replay test (same script → same per-tick statuses) green.

  Delivered: `tests/asset_ops.rs` (the behavioural contracts), `asset_model.rs`
  (2000 random load/commit/unload sequences against a naive reference store),
  `asset_replay.rs` (every script replayed, plus the golden transcript §7 asks
  for). The §7 items that are *not* here are the ones needing a real loader:
  `tools/check-assets`, doctor's asset-root checks, and `verify` integration are
  A3, and the placeholder half of the transcripts needs a renderer (R2).

  What the mutation checks said. Eight deliberate breakages, all caught: keeping
  a route across `unload`, `all_ready` waiting on failures, reporting failures
  every commit, resolving at load instead of at commit, not bumping the
  generation, draining in path order, ignoring the tick, and draining in hash
  order. The reference-model test caught all eight on its own — it is the test
  worth keeping expensive. The replay test caught only the last one, and that is
  the point rather than a weakness: seven of the eight breakages are perfectly
  deterministic, and replay is blind to a bug it reproduces faithfully. This is
  the same lesson core §6 recorded about reordered command buffers. Replay
  proves repeatability and nothing else; correctness needs the model.
- **A1 — native loader.** ✅ Loader thread + mpsc, fs ByteSource, `png` decode
  (dep delta recorded), case-strict check, limits, §6 error set.
  Exit: ~~`examples/sprites.rs` loads real files~~; error-message snapshot tests.

  **Exit criterion corrected.** `examples/sprites.rs` belongs to R2, which comes
  after this in the order — A1 could not deliver an example that does not exist
  yet without also delivering the sprite pipeline. `examples/load_from_disk.rs`
  stands in its place and is arguably the better test of *this* milestone: it
  loads real files, asserts the decoded texels, and shows every failure message,
  with no window and no GPU anywhere in it.

  `FileSource` lives in `jidousha-platform` and runs one loader thread. The
  `Mutex` around the `mpsc::Receiver` that A0's `ByteSource` doc predicted is
  exactly what landed, for exactly the predicted reason.

  Three changes to A0's seam, all forced by "decode off the frame":

  - `request` takes an `AssetKind`, so a source knows whether to decode.
  - `Completion` carries a `Payload` — bytes, or decoded `TextureData` — rather
    than a `Vec<u8>`. A source returns finished work.
  - The failure type is `AssetError` rather than a `String`, which is what lets
    §6's classes each say something specific.

  **Where decoding lives, and why it looks wrong.** `decode_png` is in
  `jidousha-assets`, not in the platform crate that reads the files. §3 wants
  one code path on every platform; §5 wants native decoding off the frame. Both
  hold if the *code* lives with the format and the *call* happens wherever the
  bytes landed — the native source calls it from its loader thread, and A2's web
  source will call it at the commit point. What would break §3 is each platform
  bringing its own decoder, which is what putting it in the platform crates
  invites.

  What the mutation checks said. Ten deliberate breakages, nine caught at once.
  The survivor was reversing the order completions come back in — the §5
  CONTRACT that keeps replay stable — which nothing noticed because every test
  had at most one failure in flight. `failures_are_reported_in_the_order_they_were_asked_for`
  now asks for three broken files in an order that is neither alphabetical nor
  reversed, so neither sorting by name nor reversing passes by accident.

  Dependency delta (practices §5.8): `png` 0.18, plus `simd-adler32`. 251 → 258
  external crates workspace-wide, 10 in the assets crate's own tree — "a handful
  of crates", as §3 predicted. No `image` mega-crate.
- **A2 — web loader.** ✅ fetch ByteSource, decode-at-commit, HTTP error mapping.
  Exit: sprites example loads over HTTP in the browser; wasm CI covers the
  non-fetch logic, manual check covers the rest (ADR-0005 proxy note).

  **The prediction held exactly.** A1 wrote that A2 would delete the compiled-in
  PNGs from `examples/sprites.rs` and that "the rest of the example does not
  change". Both examples that had a `cfg`'d `art()` now have a one-line one, and
  nothing else in either moved. `tools/serve-web sprites --check` shows the
  atlas, the hero and the glow on the canvas, fetched over HTTP.

  **Games do not write the `cfg`.** `jidousha_platform::asset_source(root)` is
  the one call: `FileSource` on native, `WebSource` on the web. Both remain
  public — `tests/file_source.rs` and `examples/load_from_disk.rs` name
  `FileSource` deliberately, because they are about the *file* loader — but a
  game asking "which source should I use" is a game doing the platform crate's
  job, and getting it wrong the first time it is ported.

  **`spawn_local`, not hand-polled futures.** R1 polls wgpu's futures with a
  no-op waker specifically because that code is shared between native and web,
  and a blocks-here/spawns-there design would be two implementations of one
  thing. This file compiles for one target, so there is no second implementation
  to keep honest. ADR-0011 governs the engine's *API*, which is unchanged:
  nothing a fetch does is observable until `commit`.

  **What the browser can and cannot tell you.** Case mismatches are undetectable
  here — a web server just returns 404 — which is the whole reason the *native*
  loader is case-strict (§2): the bug is caught at home instead of after
  deploying. The `Http` message says so explicitly, since a 404 on a name that
  works locally is exactly that bug arriving late.

  **Dependency delta (practices §5.8): none.** `wasm-bindgen`,
  `wasm-bindgen-futures`, `js-sys` and `web-sys` are all already in the wasm
  tree by way of winit, so naming them adds features rather than crates —
  measured, 258 before and after.

  **A gap this closed that was not on the list.** `eprintln!` does nothing on
  `wasm32-unknown-unknown`: Rust's standard streams have nowhere to go in a
  browser. Every §9 message the driver printed — a missing asset, a skipped
  frame — was written into a void on the one target where a person is most
  likely to be debugging by reading. A2 is what made that reachable, because
  mistyping an asset path is the ordinary way a web build fails. Run-time
  reports now go through `report::problem`, which is `eprintln!` on native and
  `console.error` in a browser.

  What the mutation checks said. Nine deliberate breakages, seven caught by
  `cargo test`. Two of the seven were only caught after a fix: the URL join's
  tests had been written *inside* the wasm-only module, where they never
  compiled and never ran — so the join moved out from behind the `cfg`, which is
  where anything testable belongs. The two that `cargo test` cannot see are in
  the page's JavaScript; one of them is caught by `tools/serve-web --check`
  (verified by hand, with the mutation applied), and the other — treating a
  non-engine `console.error` as harmless — is caught by nothing, because no
  example can produce one.
- **A3 — CI + verify integration.** `tools/check-assets`, doctor checks,
  `verify` wired to MemorySource scripting. Exit: a deliberately broken path in
  an example fails CI with the §6 message.

## 9. Deferred (tracked, not designed)

Audio assets (with the audio system) · serde/structured data · hot reload
(agents restart cheaply; humans may want it later) · atlas packing · bundles /
single-file packs for distribution · streaming · compressed textures ·
`createImageBitmap` off-main-thread decode (needs bit-exactness proof).
