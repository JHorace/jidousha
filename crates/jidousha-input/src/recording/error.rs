//! Why a recording could not be read.
//!
//! Key types: `RecordingError`.
//! Depends on: `codec`, `jidousha-core`.
//! INVARIANT (input.md §7, core.md §9): every variant's message names what
//! happened, the likely cause, and the fix. A file that came from outside is an
//! environmental failure, so this is a `Result`, not a panic.

use core::fmt;

use jidousha_core::message;

use crate::codec::DecodeError;

use super::VERSION;

/// Why a recording could not be read.
///
/// Environmental, not a contract violation: the file came from outside, and a
/// `try_`-class `Result` is what the taxonomy asks for (input.md §7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordingError {
    /// The bytes do not begin with a recording header.
    NotARecording,
    /// A recording, from a version this build does not read.
    Version {
        /// The version the file claims.
        found: u16,
    },
    /// A complete record inside it did not decode.
    Snapshot(DecodeError),
    /// The timeline runs backwards.
    OutOfOrder {
        /// The tick that came second.
        found: u64,
        /// The tick it came after.
        after: u64,
    },
}

impl fmt::Display for RecordingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (what, specifics, cause, fix) = match self {
            RecordingError::NotARecording => (
                "this is not a jidousha recording".to_owned(),
                "the file does not start with the recording header".to_owned(),
                "the wrong file was passed, or it is empty",
                "check the path; a recording is written by the engine and starts with `JDRC`",
            ),
            RecordingError::Version { found } => (
                format!("this recording is version {found}"),
                format!("this build reads version {VERSION}"),
                "the file was made by a different version of the engine",
                "re-record the session with this build; recordings are not converted between \
                 versions (input.md §5)",
            ),
            RecordingError::Snapshot(error) => (
                "a tick inside the recording did not decode".to_owned(),
                error.to_string(),
                "the file is corrupt in the middle rather than cut short at the end",
                "re-record the session — a recording that is merely incomplete replays up to \
                 where it stops, but one that is wrong in the middle cannot",
            ),
            RecordingError::OutOfOrder { found, after } => (
                format!("the recording's timeline runs backwards: tick {found} after {after}"),
                "records are in tick order and each tick appears once".to_owned(),
                "the file was assembled from two sessions, or edited",
                "re-record the session",
            ),
        };
        formatter.write_str(&message(&what, &specifics, cause, fix))
    }
}

impl core::error::Error for RecordingError {}
