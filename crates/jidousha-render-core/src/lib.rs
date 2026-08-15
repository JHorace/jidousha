//! Draw submissions, sorting, and the backend trait every render backend implements.
//!
//! Key types: none yet — lands in R0 (`docs/internal/renderer.md` §11).
//! Depends on: `jidousha-core`. Must never be depended on by: `jidousha-core`.
//! INVARIANT: contains no backend-specific types; the `wgpu`→`ash` swap must be
//! invisible from here (ADR-0003).
