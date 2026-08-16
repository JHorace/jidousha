//! Input snapshots: the complete, recordable input truth for one Update tick.
//!
//! Key types: `Input`, `InputSnapshot`, `Key`, `PointerState`, `InputEvent`,
//! `SnapshotBuilder`, `InputScript`.
//! Depends on: `jidousha-core`. Must never be depended on by: `jidousha-core`.
//! INVARIANT: snapshots are plain data — simulation reads input only through them,
//! never through platform events; pointers are modeled as pointers, not "the mouse"
//! (core.md §7, ADR-0005).
//!
//! Built so far (`docs/internal/input.md` §8): I0 — the types, the edge rules,
//! the snapshot codec, and [`InputScript`]. The winit translation tables (I1)
//! and the recording stream (I2) land next.

mod builder;
mod codec;
mod key;
mod pointer;
mod script;
mod snapshot;

pub use builder::{InputEvent, SnapshotBuilder};
pub use codec::DecodeError;
pub use key::Key;
pub use pointer::{PointerButton, PointerId, PointerState};
pub use script::InputScript;
pub use snapshot::{Input, InputSnapshot};
