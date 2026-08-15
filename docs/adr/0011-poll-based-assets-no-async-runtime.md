# ADR-0011: Asset API is poll-based — no async runtime, no async/await in public API

Status: accepted · 2026-08-15

## Context

Asset loading is inherently asynchronous on the web (fetch) and best off-thread on
native (ADR-0005 made the API async-by-design). The idiomatic-Rust reflex is
`async fn load(...)` plus an executor (tokio/smol) or `wasm-bindgen-futures`.
The question is whether async Rust belongs in a game engine's public surface.

## Decision

The asset API is **synchronous-looking and poll-based**:

```rust
let tex: TextureHandle = assets.load("player.png");   // returns immediately, always
// ... frames pass; the engine completes loads in the background ...
assets.status(tex)      // Loading | Ready | Failed — checkable, rarely needed
```

- `load` never blocks and never fails at call time. Handles are usable
  immediately (renderer draws the placeholder until ready — renderer §5).
- Internally: native uses a plain loader thread + channel; web uses browser
  fetch callbacks. Both feed a completion queue that the engine drains at one
  deterministic commit point per frame (assets doc §4). **No async runtime
  dependency, no `async fn` anywhere in the public API, no `.await` in game
  code, ever.**

## Rationale

- **Async Rust is the single largest ergonomic hazard we could hand a game
  agent**: colored functions, `Send`/`'static` bound errors, executor choice,
  and the worst diagnostics in the language — all to express "it'll be ready in
  a few frames," which a game loop expresses naturally by *being a loop*.
- Games poll; frames are the native async primitive of the medium. The
  placeholder policy means the common case needs no status check at all.
- Dependency budget: tokio-class runtimes are the heaviest common dependency in
  the ecosystem, bought here for nothing we need.
- Determinism: a hand-owned completion queue drained at a fixed point is easy
  to make replay-deterministic (assets doc §4); executor scheduling is not.

## Consequences

- The internal loader machinery is modestly more manual (a thread, a channel, a
  queue) — ~100 lines we own completely, in exchange for zero async surface.
- A future subsystem wanting true async (networking, someday) must make its own
  case in its own ADR; this ADR is not a precedent against it internally, only
  a wall around the public API and the dependency tree.
- DELIBERATE tags at the loader implementation point here — a future agent
  proposing "modernize to async/tokio" must read this first.

## Alternatives rejected

- **`async fn` public API + executor**: idiomatic-looking, and wrong for every
  reason above.
- **Blocking loads**: dead on arrival — no synchronous fs exists on web
  (ADR-0005), and a frozen first frame on native is silent-failure-adjacent.
