//! The Jidousha engine facade — the entire public surface a game may use.
//!
//! Key types: none yet — the curated re-export set lands in F0
//! (`docs/internal/public-api.md`).
//! Depends on: every other jidousha crate. Must never be depended on by: any of them.
//! INVARIANT: `docs/api/` is generated from THIS crate only; anything not re-exported
//! here is not public API, and games never reach past it (agent-practices §2.4).
