//! wgpu implementation of the render backend trait.
//!
//! Key types: none yet — lands in R1 (`docs/internal/renderer.md` §11).
//! Depends on: `jidousha-render-core`, `wgpu`. Must never be depended on by:
//! `jidousha-core`, `jidousha-render-core`, `jidousha` (facade selects a backend
//! at the platform layer).
//! INVARIANT: the ONLY crate that may depend on `wgpu`; no `wgpu` type appears in
//! its public API (ADR-0003).
