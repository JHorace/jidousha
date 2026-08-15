//! Asset handles and poll-based loading — async by design on every platform.
//!
//! Key types: none yet — lands in A0 (`docs/internal/assets.md` §8).
//! Depends on: `jidousha-core`. Must never be depended on by: `jidousha-core`.
//! INVARIANT: no async runtime and no blocking file I/O in the tick path; readiness
//! becomes visible to simulation only at deterministic points (ADR-0011).
