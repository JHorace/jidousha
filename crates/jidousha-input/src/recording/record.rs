//! One tick of the timeline, and its bytes.
//!
//! Key types: `TickRecord`, `AssetReady`.
//! Depends on: `codec`, `snapshot`, and its parent for `RecordingError`. Must
//! never depend on: `jidousha-assets` — see [`AssetReady`] for how readiness
//! gets in here without it.
//! INVARIANT (input.md §5, CONTRACT): a record is self-delimiting, which is what
//! makes the file append-only — a writer flushes one per tick and never rewrites
//! what it already wrote.
//! INVARIANT: running out of bytes mid-record is `Ok(None)`, not an error. The
//! position is rewound so the caller sees an untouched cursor, because a
//! half-read record must leave no trace of having been attempted.

use crate::snapshot::InputSnapshot;

use super::RecordingError;

/// An asset that resolved on some tick.
///
/// **The request id, not the handle.** `jidousha-input` cannot name a
/// `TextureHandle` — it does not depend on the assets crate, and should not —
/// so what is recorded is the number the store already uses to route a
/// completion. That number is deterministic: requests are numbered in load
/// order, and a replay runs the same game code and therefore asks for the same
/// things in the same order (assets.md §5). The same trick ADR-0015 uses for
/// textures, applied to a different seam.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetReady {
    /// Which request resolved, as [`RequestId`](crate::RequestIdBits) numbers
    /// them.
    pub request: u64,
    /// Whether it arrived. `false` is a load that failed — recorded because a
    /// game can observe the difference (assets.md §4).
    pub arrived: bool,
}

/// One tick of the timeline.
#[derive(Clone, Debug, PartialEq)]
pub struct TickRecord {
    /// Which tick this is. Records are in order and ticks never go backwards.
    pub tick: u64,
    /// What input the tick saw.
    pub input: InputSnapshot,
    /// Which assets committed on it, in the order the store applied them.
    pub readiness: Vec<AssetReady>,
}

impl TickRecord {
    /// This record's bytes, self-delimiting.
    ///
    /// Self-delimiting is what makes the append-only CONTRACT work: a writer
    /// flushes one of these per tick and never rewrites what it has already
    /// written, so a process that dies mid-session leaves a file that is valid
    /// up to the last whole tick.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let snapshot = self.input.encode();
        let mut out = Vec::with_capacity(snapshot.len() + 16 + self.readiness.len() * 9);
        out.extend_from_slice(&self.tick.to_le_bytes());
        // A length prefix rather than a terminator: the snapshot codec owns its
        // own bytes and this format does not get to know what is in them.
        out.extend_from_slice(&(snapshot.len() as u32).to_le_bytes());
        out.extend_from_slice(&snapshot);
        out.extend_from_slice(&(self.readiness.len() as u16).to_le_bytes());
        for entry in &self.readiness {
            out.extend_from_slice(&entry.request.to_le_bytes());
            out.push(u8::from(entry.arrived));
        }
        out
    }
}

/// One record, or `None` if the bytes run out before it is complete.
pub(super) fn decode_record(
    bytes: &[u8],
    at: &mut usize,
) -> Result<Option<TickRecord>, RecordingError> {
    let start = *at;
    let Some(tick) = take(bytes, at, 8).and_then(|slice| slice.try_into().ok()) else {
        *at = start;
        return Ok(None);
    };
    let Some(length) = take(bytes, at, 4).and_then(|slice| slice.try_into().ok()) else {
        *at = start;
        return Ok(None);
    };
    let length = u32::from_le_bytes(length) as usize;
    let Some(snapshot) = take(bytes, at, length) else {
        *at = start;
        return Ok(None);
    };
    // Present and complete, so a decode failure here is corruption rather than
    // truncation — and the snapshot codec is the strict one (ADR-0014).
    let input = InputSnapshot::try_decode(snapshot).map_err(RecordingError::Snapshot)?;

    let Some(count) = take(bytes, at, 2).and_then(|slice| slice.try_into().ok()) else {
        *at = start;
        return Ok(None);
    };
    let count = u16::from_le_bytes(count) as usize;
    let mut readiness = Vec::with_capacity(count);
    for _ in 0..count {
        let Some(request) = take(bytes, at, 8).and_then(|slice| slice.try_into().ok()) else {
            *at = start;
            return Ok(None);
        };
        let Some(arrived) = take(bytes, at, 1) else {
            *at = start;
            return Ok(None);
        };
        readiness.push(AssetReady {
            request: u64::from_le_bytes(request),
            arrived: arrived[0] != 0,
        });
    }
    Ok(Some(TickRecord {
        tick: u64::from_le_bytes(tick),
        input,
        readiness,
    }))
}

/// `count` bytes from `at`, advancing it, or `None` if there are not that many.
pub(super) fn take<'a>(bytes: &'a [u8], at: &mut usize, count: usize) -> Option<&'a [u8]> {
    let end = at.checked_add(count)?;
    let slice = bytes.get(*at..end)?;
    *at = end;
    Some(slice)
}
