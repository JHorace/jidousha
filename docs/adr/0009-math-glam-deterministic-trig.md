# ADR-0009: Math — glam, with engine-owned deterministic trig

Status: accepted · 2026-08-15

## Context

Core §10 deferred the math-crate choice to the renderer design. Criteria:
dependency budget (practices §5.8), training-data familiarity, and — hardest —
the cross-platform determinism contract (core §7): same seed + inputs must replay
identically on glibc, MSVC, and wasm. Basic IEEE ops (add/mul/div/sqrt) are
bit-exact everywhere; **libm-backed transcendentals (`sin`, `cos`, `atan2`, …)
are not** — each platform's libm rounds differently.

## Decision

1. **glam** for vectors/matrices, with the `scalar-math` feature enabled
   (sidelines SIMD-path variation across x86/ARM/wasm; revisit only with a
   benchmark showing it matters). Wrapped by engine newtypes where conventions
   demand them (`Radians`, `Seconds`); raw `Vec2`/`Vec3`/`Mat4` are used
   directly — they're conventions-compliant already and agent-familiar.
   Verify at adoption: `cargo tree` delta should be ~zero crates with default
   features; record the measurement per §5.8.
2. **Engine-owned deterministic trig**: `jidousha-core::math` provides
   `sin_cos(Radians)`, `atan2`, and friends as deterministic pure-Rust
   implementations (polynomial approximations; bit-identical everywhere by
   construction, accuracy target ~1e-6 — ample for gameplay).
3. **Std float trig is banned engine-wide** via clippy `disallowed-methods`
   (`f32::sin`, `f32::cos`, `f32::tan`, `f32::atan2`, `f32::sin_cos`, and f64
   equivalents) in all jidousha crates. glam constructors that internally call
   libm trig (`Mat2::from_angle`-class) are likewise disallowed; the engine
   provides equivalents built on its own `sin_cos`.

## Rationale

- glam: zero-dep, pure Rust, the de-facto standard in Rust gamedev training
  data — agents write it fluently. Writing our own vector types buys nothing
  and costs familiarity.
- Trig is the one genuine determinism hole, and it can't be fixed by crate
  choice — any crate calling libm inherits the problem. Owning ~100 lines of
  polynomial math closes it permanently.
- Mechanical enforcement (clippy) over prose: an agent reaching for `.sin()`
  gets an immediate lint naming the replacement.

## Consequences

- `jidousha-core::math` is in scope for milestone M3 (needed by the time any
  system rotates anything); property-tested against std trig for accuracy and
  against itself for cross-platform bit-equality (wasm CI runs the same vectors).
- Renderer and all other engine crates use engine trig too — one rule, no
  carve-outs; camera rotation matrices route through `sin_cos`.
- 3D headroom: glam covers 3D types already; the decision doesn't need revisiting
  for ADR-0001's eventual-3D.

## Alternatives rejected

- **Own math crate**: determinism-equivalent but forfeits agent familiarity for
  hundreds of lines of undifferentiated work.
- **glam default (SIMD) + fixup later**: add/mul are IEEE-exact so it would
  *probably* replay identically, but "probably" is not a contract; scalar-math
  makes it boring. Revisit with data if math ever profiles hot.
- **Fixed-point simulation math**: bulletproof determinism, catastrophic
  ergonomics for agents and for the entire f32-based ecosystem boundary.
