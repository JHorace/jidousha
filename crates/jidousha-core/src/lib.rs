//! Deterministic simulation core: entities, components, queries, schedule, time, app model.
//!
//! Key types: none yet — M0 is the scaffold milestone (`docs/internal/core.md` §11).
//! Depends on: nothing. Must never be depended on by nothing; must never depend on
//! any other jidousha crate, and never on `winit`/`wgpu` (core.md §1 CONTRACT).
//! INVARIANT: compiles on every target including `wasm32-unknown-unknown` with zero
//! `cfg` branches in simulation logic, and observes no wall clock (ADR-0005).
