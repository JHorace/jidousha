# ADR-0005: Platform targets — Linux, Windows, Web; Android later

Status: accepted · 2026-08-15

## Context

The engine's product is agent-generated prototypes. Prototypes are only useful if
people can play them; the web is the lowest-friction distribution channel there is —
a playtest link that runs on any hardware. That makes web a first-class target despite
its constraints, not a port.

## Decision

Tier 1 (CI-gated from the first commit): **Linux, Windows, Web (wasm32)**.
Future (not yet built, not to be precluded): **Android**. Not targeted: macOS/iOS
for now (wgpu/winit keep the door open at near-zero cost).

**Web is in CI from day one.** `cargo check --target wasm32-unknown-unknown` (and a
headless wasm test where feasible) gates every merge. Web support rots in weeks if
not continuously built; retrofitting it is the single most expensive porting job in
engine development.

## Web constraints that shape the core (not the port)

1. **No owned main loop.** The browser drives frames via callbacks. The application
   lifecycle API is callback-shaped everywhere (fits winit, ADR-0004). Desktop is
   "web-shaped," not vice versa.
2. **Asset loading is async.** No synchronous file reads exist on web. The asset API
   is async-by-design on all platforms: request → handle immediately → loaded later
   (states: loading/ready/failed, poll- or callback-observable). Desktop just
   resolves faster. This lands in the asset system design.
3. **No wall clock, no threads (by default).** Simulation time comes from the
   engine's frame clock only — `std::time::Instant` is banned in engine code
   (web-compatible time shim at the platform boundary; clippy/CI-checked).
   Core stays single-threaded until a deliberate ADR adds threading within wasm's
   constraints (atomics + COOP/COEP headers).
4. **Determinism (§5.6) must hold cross-platform**: same seed + same inputs → same
   simulation on native and web. f32 math discipline; no platform-conditional
   simulation logic.
5. **Rendering envelope**: WebGL2-compatible feature set for now (ADR-0003 §4).

## Consequences

- `tools/doctor` checks the wasm toolchain (target installed, wasm-bindgen present).
- `tools/verify` needs a headless-browser or wasm-runtime path eventually; native
  headless is the near-term proxy, with the gap documented.
- A "build for web + serve locally" script is part of basic tooling — the playtest
  loop is the point of web support.
- Android later mostly follows from winit + wgpu; the main future costs are input
  (touch) and asset packaging. Touch shows up in the input design as a
  don't-preclude concern (e.g. pointer events not hardcoded to mouse).

## Alternatives rejected

- **Native-only first, web later**: cheapest now, catastrophic retrofit; loses the
  playtest-anywhere property the engine exists for.
- **Web-only**: loses native perf headroom and the ash future (ADR-0003).
