//! The recorded timeline: what arrived on every tick, in bytes.
//!
//! Key types: `Recording`. `TickRecord` and `AssetReady` are in `record`;
//! `RecordingError` is in `error`.
//! Depends on: `codec`, `snapshot`, `jidousha-core`. Must never depend on:
//! `jidousha-assets` — see [`AssetReady`] for how readiness gets in here
//! without it.
//! INVARIANT (input.md §5, CONTRACT): a versioned header, records that are
//! self-delimiting so a writer can append one per tick, and bytes that are the
//! same on every platform. A recording made on a Windows machine last month
//! replays on a Linux machine today.
//! INVARIANT: strict inside a record, tolerant at the tail. A record whose
//! bytes are present and wrong is an error; a record that simply runs out is
//! the end of the file, because a session that crashed is exactly the session
//! worth replaying.

use jidousha_core::{Seconds, message};

mod error;
mod record;

pub use error::RecordingError;
pub use record::{AssetReady, TickRecord};

use record::{decode_record, take};

/// Marks a recording, so a file that is not one says so rather than decoding
/// into nonsense.
const MAGIC: [u8; 4] = *b"JDRC";

/// The format version. Bump when the layout changes; old files are refused with
/// a message rather than misread.
const VERSION: u16 = 1;

/// A whole session: what it was seeded with, and what happened on every tick.
///
/// ```
/// use jidousha_input::{InputSnapshot, Recording, TickRecord};
/// use jidousha_core::Seconds;
///
/// let mut recording = Recording::new(42, Seconds(1.0 / 60.0));
/// recording.push(TickRecord {
///     tick: 1,
///     input: InputSnapshot::new(),
///     readiness: Vec::new(),
/// });
///
/// let bytes = recording.encode();
/// let read_back = Recording::try_decode(&bytes).expect("it was just written");
/// assert_eq!(read_back.seed(), 42);
/// assert_eq!(read_back.ticks().len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Recording {
    seed: u64,
    fixed_dt: Seconds,
    ticks: Vec<TickRecord>,
}

impl Recording {
    /// An empty recording of a run with this seed and timestep.
    ///
    /// Both are in the header because simulation state is a function of seed,
    /// systems and inputs (core.md §7) — a recording that did not carry the
    /// seed would replay a different game and look like a determinism bug.
    #[must_use]
    pub fn new(seed: u64, fixed_dt: Seconds) -> Self {
        Self {
            seed,
            fixed_dt,
            ticks: Vec::new(),
        }
    }

    /// What the run was seeded with.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// How long one tick was.
    #[must_use]
    pub fn fixed_dt(&self) -> Seconds {
        self.fixed_dt
    }

    /// Every tick recorded, in order.
    #[must_use]
    pub fn ticks(&self) -> &[TickRecord] {
        &self.ticks
    }

    /// Add a tick to the end.
    ///
    /// # Panics
    ///
    /// If `record.tick` is not after the last one. The timeline is the whole
    /// point of the file; one that runs backwards is a bug in whatever is
    /// writing it, not a file to tolerate (core.md §9).
    pub fn push(&mut self, record: TickRecord) {
        if let Some(last) = self.ticks.last()
            && record.tick <= last.tick
        {
            panic!(
                "{}",
                message(
                    &format!(
                        "a recording went backwards: tick {} after tick {}",
                        record.tick, last.tick
                    ),
                    "records are appended in tick order and each tick appears once",
                    "the recorder was given a stale tick, or the same tick twice",
                    "record once per tick, with the tick the simulation is on",
                )
            );
        }
        self.ticks.push(record);
    }

    /// The header, without any records.
    ///
    /// A writer emits this once and then appends [`TickRecord::encode`] per
    /// tick; the result decodes identically to [`encode`](Recording::encode).
    #[must_use]
    pub fn header(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(18);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&self.seed.to_le_bytes());
        out.extend_from_slice(&self.fixed_dt.as_f32().to_le_bytes());
        out
    }

    /// The whole recording as bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = self.header();
        for record in &self.ticks {
            out.extend_from_slice(&record.encode());
        }
        out
    }

    /// Read a recording back.
    ///
    /// A trailing partial record is **not** an error: it is where the writer
    /// stopped, and a session that crashed is precisely the one worth
    /// replaying (input.md §5). A record whose bytes are all present and wrong
    /// *is* an error — the two are told apart by whether the bytes the length
    /// prefix promised actually exist.
    ///
    /// # Errors
    ///
    /// If the header is missing, is not a recording, is a version this build
    /// does not read, or if a complete record inside it does not decode.
    pub fn try_decode(bytes: &[u8]) -> Result<Recording, RecordingError> {
        let mut at = 0usize;
        let magic = take(bytes, &mut at, 4).ok_or(RecordingError::NotARecording)?;
        if magic != MAGIC {
            return Err(RecordingError::NotARecording);
        }
        let version = u16::from_le_bytes(
            take(bytes, &mut at, 2)
                .ok_or(RecordingError::NotARecording)?
                .try_into()
                .map_err(|_| RecordingError::NotARecording)?,
        );
        if version != VERSION {
            return Err(RecordingError::Version { found: version });
        }
        let seed = u64::from_le_bytes(
            take(bytes, &mut at, 8)
                .ok_or(RecordingError::NotARecording)?
                .try_into()
                .map_err(|_| RecordingError::NotARecording)?,
        );
        let fixed_dt = f32::from_le_bytes(
            take(bytes, &mut at, 4)
                .ok_or(RecordingError::NotARecording)?
                .try_into()
                .map_err(|_| RecordingError::NotARecording)?,
        );

        let mut recording = Recording::new(seed, Seconds(fixed_dt));
        while at < bytes.len() {
            match decode_record(bytes, &mut at)? {
                // Ran out mid-record: this is the end of the file, not a fault.
                None => break,
                Some(record) => {
                    if let Some(last) = recording.ticks.last()
                        && record.tick <= last.tick
                    {
                        return Err(RecordingError::OutOfOrder {
                            found: record.tick,
                            after: last.tick,
                        });
                    }
                    recording.ticks.push(record);
                }
            }
        }
        Ok(recording)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::InputSnapshot;
    use crate::{InputEvent, Key, SnapshotBuilder};

    /// A recording of `ticks` ticks, with a key held for the middle third.
    fn sample(ticks: u64) -> Recording {
        let mut recording = Recording::new(7, Seconds(1.0 / 60.0));
        let mut builder = SnapshotBuilder::new();
        for tick in 1..=ticks {
            if tick == ticks / 3 {
                builder.record(InputEvent::KeyPressed(Key::D));
            }
            if tick == 2 * ticks / 3 {
                builder.record(InputEvent::KeyReleased(Key::D));
            }
            recording.push(TickRecord {
                tick,
                input: builder.first_tick_snapshot(),
                readiness: if tick == 2 {
                    vec![
                        AssetReady {
                            request: 0,
                            arrived: true,
                        },
                        AssetReady {
                            request: 1,
                            arrived: false,
                        },
                    ]
                } else {
                    Vec::new()
                },
            });
        }
        recording
    }

    #[test]
    fn a_recording_round_trips_through_its_bytes() {
        let recording = sample(30);
        let bytes = recording.encode();
        let Ok(read_back) = Recording::try_decode(&bytes) else {
            panic!("what was just encoded must decode");
        };
        assert_eq!(read_back, recording);
    }

    #[test]
    fn the_bytes_round_trip_too() {
        // The other direction, which is what "byte-stable" means: two
        // recordings that mean the same thing are the same file, so a
        // recording can be diffed and checked in.
        let bytes = sample(12).encode();
        let Ok(read_back) = Recording::try_decode(&bytes) else {
            panic!("decodable");
        };
        assert_eq!(read_back.encode(), bytes);
    }

    #[test]
    fn the_header_and_the_records_concatenate_into_the_whole_file() {
        // The append-only CONTRACT, stated as an equation: a writer that emits
        // the header once and one record per tick produces exactly the file
        // `encode` produces. That is what lets it never rewrite what it has
        // already written.
        let recording = sample(9);
        let mut appended = recording.header();
        for record in recording.ticks() {
            appended.extend_from_slice(&record.encode());
        }
        assert_eq!(appended, recording.encode());
    }

    #[test]
    fn a_recording_cut_short_replays_up_to_where_it_stops() {
        // The reason the tail is tolerant: a session that crashed is precisely
        // the session worth replaying, and the file it left is valid up to the
        // last whole tick.
        let recording = sample(20);
        let whole = recording.encode();
        for cut in (recording.header().len()..whole.len()).step_by(3) {
            let Ok(partial) = Recording::try_decode(&whole[..cut]) else {
                panic!("a truncated recording is not an error");
            };
            assert!(
                partial.ticks().len() <= recording.ticks().len(),
                "never more than was written"
            );
            // Whatever survived is a prefix of the original, exactly.
            assert_eq!(partial.ticks(), &recording.ticks()[..partial.ticks().len()]);
        }
    }

    #[test]
    fn a_recording_corrupt_in_the_middle_is_an_error() {
        // The other side of the tolerance: bytes that are all present and wrong
        // are not a short file, and pretending otherwise would replay a session
        // that never happened.
        let mut bytes = sample(6).encode();
        // Land inside the first record's snapshot, past the header and the
        // tick and length fields.
        let target = 18 + 12 + 2;
        bytes[target] ^= 0xFF;
        assert!(matches!(
            Recording::try_decode(&bytes),
            Err(RecordingError::Snapshot(_)) | Err(RecordingError::OutOfOrder { .. })
        ));
    }

    #[test]
    fn something_that_is_not_a_recording_says_so() {
        assert_eq!(
            Recording::try_decode(b"not a recording at all"),
            Err(RecordingError::NotARecording)
        );
        assert_eq!(
            Recording::try_decode(&[]),
            Err(RecordingError::NotARecording)
        );
    }

    #[test]
    fn a_recording_from_another_version_is_refused_by_number() {
        let mut bytes = sample(3).encode();
        bytes[4] = 99;
        assert_eq!(
            Recording::try_decode(&bytes),
            Err(RecordingError::Version { found: 99 })
        );
    }

    #[test]
    fn the_seed_and_the_timestep_survive_the_trip() {
        // Without them a replay runs a different game and looks like a
        // determinism bug (core.md §7).
        let recording = Recording::new(0xDEAD_BEEF, Seconds(1.0 / 120.0));
        let Ok(read_back) = Recording::try_decode(&recording.encode()) else {
            panic!("decodable");
        };
        assert_eq!(read_back.seed(), 0xDEAD_BEEF);
        assert_eq!(read_back.fixed_dt(), Seconds(1.0 / 120.0));
    }

    #[test]
    fn asset_readiness_survives_the_trip_with_its_order() {
        let recording = sample(5);
        let Ok(read_back) = Recording::try_decode(&recording.encode()) else {
            panic!("decodable");
        };
        let readiness = &read_back.ticks()[1].readiness;
        assert_eq!(
            readiness,
            &[
                AssetReady {
                    request: 0,
                    arrived: true
                },
                AssetReady {
                    request: 1,
                    arrived: false
                },
            ],
            "order is the order the store applied them"
        );
    }

    #[test]
    #[should_panic(expected = "a recording went backwards")]
    fn recording_a_tick_twice_is_a_bug_in_the_recorder() {
        let mut recording = Recording::new(1, Seconds(1.0 / 60.0));
        for _ in 0..2 {
            recording.push(TickRecord {
                tick: 4,
                input: InputSnapshot::new(),
                readiness: Vec::new(),
            });
        }
    }

    #[test]
    fn a_file_whose_timeline_runs_backwards_is_refused() {
        // `push` panics, so this can only arrive from a file — one assembled
        // from two sessions, or edited.
        let mut recording = Recording::new(1, Seconds(1.0 / 60.0));
        recording.push(TickRecord {
            tick: 9,
            input: InputSnapshot::new(),
            readiness: Vec::new(),
        });
        let mut bytes = recording.header();
        bytes.extend_from_slice(
            &TickRecord {
                tick: 9,
                input: InputSnapshot::new(),
                readiness: Vec::new(),
            }
            .encode(),
        );
        bytes.extend_from_slice(
            &TickRecord {
                tick: 2,
                input: InputSnapshot::new(),
                readiness: Vec::new(),
            }
            .encode(),
        );
        assert_eq!(
            Recording::try_decode(&bytes),
            Err(RecordingError::OutOfOrder { found: 2, after: 9 })
        );
    }
}
