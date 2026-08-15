//! Input snapshots: the complete, recordable input truth for one Update tick.
//!
//! Key types: none yet — lands in I0 (`docs/internal/input.md` §8).
//! Depends on: `jidousha-core`. Must never be depended on by: `jidousha-core`.
//! INVARIANT: snapshots are plain data — simulation reads input only through them,
//! never through platform events; pointers are modeled as pointers, not "the mouse"
//! (core.md §7, ADR-0005).
