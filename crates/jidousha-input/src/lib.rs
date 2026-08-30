//! Input snapshots: the complete, recordable input truth for one Update tick.
//!
//! Key types: `Input`, `InputSnapshot`, `Key`, `PointerState`, `Touch`,
//! `InputEvent`, `SnapshotBuilder`, `InputScript`.
//! Depends on: `jidousha-core`. Must never be depended on by: `jidousha-core`.
//! INVARIANT: snapshots are plain data — simulation reads input only through them,
//! never through platform events; pointers are modeled as pointers, not "the mouse"
//! (core.md §7, ADR-0005).
//!
//! Built so far (`docs/internal/input.md` §8): I0 — the types, the edge rules,
//! the snapshot codec and [`InputScript`]; I1 — the platform translation; I2 —
//! the recording stream. I3 adds touch: a bounded list of at most
//! [`MAX_TOUCHES`] fingers, and the mirror that puts the first of them on the
//! primary pointer so a game written for a mouse is playable with a thumb
//! (input.md §3a, ADR-0043).

mod builder;
mod codec;
mod key;
mod pointer;
mod recording;
mod script;
mod snapshot;
mod touch;

pub use builder::{InputEvent, SnapshotBuilder};
pub use codec::DecodeError;
pub use key::Key;
pub use pointer::{PointerButton, PointerId, PointerState};
pub use recording::{AssetReady, Recording, RecordingError, TickRecord};
pub use script::InputScript;
pub use snapshot::{Input, InputSnapshot};
pub use touch::{FingerId, MAX_TOUCHES, Touch, TouchId, TouchPhase};
