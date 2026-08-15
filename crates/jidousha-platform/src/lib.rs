//! Window, event pump and the real-time loop driver — the engine's only platform seam.
//!
//! Key types: none yet — lands in M5 (`docs/internal/core.md` §11).
//! Depends on: `jidousha-core`, `winit`. Must never be depended on by: `jidousha-core`.
//! INVARIANT: the ONLY crate that may depend on `winit` and the ONLY crate that may
//! read a wall clock; neither may appear in its public API (ADR-0004, ADR-0005).
