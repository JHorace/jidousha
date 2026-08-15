# Asset loading basics — design and contracts

Status: **design draft, pre-implementation.** Becomes the living internal doc for
`jidousha-assets`. **CONTRACT** items are binding and tested.

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

## 5. Internals: the platform seam and threading

```
jidousha-assets
  AssetStore        handles, states, CPU-side data, completion queue, commit()
  ByteSource trait  fn request(&mut self, path) -> RequestId  +  completion drain
jidousha-platform   provides the ByteSource impls:
  native            one loader thread, std::sync::mpsc; fs read + png decode off-thread
  web               fetch via wasm-bindgen; png decode on main thread at commit
```

- The trait seam mirrors ADR-0003's discipline: `jidousha-assets` never touches
  fs, fetch, or wasm-bindgen; platform crates own I/O. A third impl —
  `MemorySource` (preloaded `HashMap<path, bytes>` with scripted completion
  ticks) — is the test/verify workhorse and ships in `jidousha-assets` itself.
- Native decode happens on the loader thread (PNG decode is the slow part);
  web decodes at the commit point (main thread — acceptable at prototype scale;
  PERF-revisit with evidence, options exist: workers, `createImageBitmap` — the
  latter forbidden by the identical-texels CONTRACT unless proven bit-exact).
- GPU upload: at commit, newly-Ready textures are handed to render-core, which
  calls `backend.create_texture` at the next frame start. CPU-side pixels are
  then dropped (assets keep only metadata; `capture()`/goldens read from GPU).

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
- Golden transcript tests cover: placeholder → real texture swap at the
  scripted tick; Failed → placeholder + single error; unload → debug panic on
  use (a `should_panic` test locking the message).

## 8. Milestones

Sequenced against renderer milestones (renderer needs textures at R2):

- **A0 — store + states + MemorySource.** Handles, statuses, commit point,
  scripted-readiness testing, `all_ready`, unload semantics + panics. No I/O,
  no GPU; runs everywhere incl. wasm CI. Exit: state-machine property tests
  green; readiness-replay test (same script → same per-tick statuses) green.
- **A1 — native loader.** Loader thread + mpsc, fs ByteSource, `png` decode
  (dep delta recorded), case-strict check, limits, §6 error set.
  Exit: `examples/sprites.rs` loads real files; error-message snapshot tests.
- **A2 — web loader.** fetch ByteSource, decode-at-commit, HTTP error mapping.
  Exit: sprites example loads over HTTP in the browser; wasm CI covers the
  non-fetch logic, manual check covers the rest (ADR-0005 proxy note).
- **A3 — CI + verify integration.** `tools/check-assets`, doctor checks,
  `verify` wired to MemorySource scripting. Exit: a deliberately broken path in
  an example fails CI with the §6 message.

## 9. Deferred (tracked, not designed)

Audio assets (with the audio system) · serde/structured data · hot reload
(agents restart cheaply; humans may want it later) · atlas packing · bundles /
single-file packs for distribution · streaming · compressed textures ·
`createImageBitmap` off-main-thread decode (needs bit-exactness proof).
