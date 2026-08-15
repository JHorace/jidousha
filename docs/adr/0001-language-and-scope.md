# ADR-0001: Language and scope

Status: accepted · 2026-08-15

## Context

Jidousha is a game engine designed from the ground up for AI agents — both as its
developers and as its users (agents generating game prototypes). The dominant design
criterion is therefore the quality of the agent feedback loop: how much wrongness is
caught statically, how legible errors are, and how fast the iterate cycle runs. Raw
runtime performance is secondary at this stage.

## Decision

- **Language: Rust**, latest stable, pinned via `rust-toolchain.toml`.
- **Scope: 2D first, 3D eventually.** Nothing in the core (math types, transform
  hierarchy, renderer boundary) may structurally preclude 3D, but no 3D feature is
  built now.
- Initial subsystems: engine core (ECS + loop), renderer basics, asset loading basics,
  input, and the public API for these. Explicitly deferred: audio, advanced rendering,
  asset streaming.

## Rationale

Rust over C++: the compiler catches far more statically and its diagnostics are the
most machine-legible in mainstream use — the agent repairs against `cargo check` text
without running the game. One toolchain, one build system, one formatter, and a highly
consistent training corpus. C++'s larger corpus is offset by dialect fragmentation,
slow builds, UB producing *no* diagnostic (the pathological case for a repair loop),
and build-system fragility.

Known Rust risk: borrow-checker fights around game object graphs. Mitigated
structurally by ECS (ADR-0002) — data-oriented, handle-based design dissolves the
shared-mutability patterns agents get stuck on.

## Consequences

- Dependency policy: prefer pure-Rust crates; system-C-linking crates need
  justification in an ADR (shrinks the environment-failure surface,
  agent-practices §6.5).
- Math and transforms are written 2D-with-3D-headroom (e.g. Z exists in transforms
  even if 2D rendering mostly ignores it). Concretes land in the core design docs.

## Alternatives rejected

- **C++**: corpus advantage, but worst-in-class agent repair loop.
- **C#**: strong middle option (GC, fast builds, good diagnostics), but weaker
  web story and less static safety than Rust.
- **Scripting-first (Lua/TS host)**: fastest cycle but catches nothing statically;
  may reappear later as a game-layer option, not as the engine language.
