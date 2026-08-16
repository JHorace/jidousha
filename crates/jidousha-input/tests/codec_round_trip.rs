//! The other half of the I0 exit criterion: a snapshot survives being written
//! down (input.md §5, §8).
//!
//! A recording is only worth making if what comes back is what went in, to the
//! bit. These run the same random streams the edge tests do, so the snapshots
//! being round-tripped are ones the engine actually produces rather than ones
//! written to be easy.

mod support;

use jidousha_input::{InputSnapshot, SnapshotBuilder};
use support::{Step, generate};

/// Every snapshot a run produces, from streams varied enough to be interesting.
fn snapshots(seed: u64) -> Vec<InputSnapshot> {
    let mut builder = SnapshotBuilder::new();
    let mut out = Vec::new();
    for step in generate(seed, 150) {
        match step {
            Step::Event(event) => builder.record(event),
            Step::Frame { ticks } => {
                for tick in 0..ticks {
                    out.push(if tick == 0 {
                        builder.first_tick_snapshot()
                    } else {
                        builder.catch_up_snapshot()
                    });
                }
            }
        }
    }
    out
}

#[test]
fn every_snapshot_the_engine_produces_survives_the_round_trip() {
    let mut checked = 0;
    for seed in 0..500 {
        for snapshot in snapshots(seed) {
            let bytes = snapshot.encode();
            match InputSnapshot::try_decode(&bytes) {
                Ok(decoded) => assert_eq!(decoded, snapshot, "seed {seed}"),
                Err(error) => panic!("seed {seed}: {error}"),
            }
            checked += 1;
        }
    }
    assert!(checked > 1000, "only {checked} snapshots were checked");
}

#[test]
fn the_bytes_are_the_same_bytes_after_a_round_trip() {
    // Both directions. If two byte strings could decode to one snapshot, a
    // recording would not be comparable byte-for-byte across machines, and
    // "byte-stable across platforms" would be a hope rather than a property.
    for seed in 0..500 {
        for snapshot in snapshots(seed) {
            let bytes = snapshot.encode();
            let Ok(decoded) = InputSnapshot::try_decode(&bytes) else {
                panic!("seed {seed}: a snapshot the engine produced failed to decode");
            };
            assert_eq!(decoded.encode(), bytes, "seed {seed}");
        }
    }
}

#[test]
fn a_snapshot_never_encodes_to_the_same_bytes_as_a_different_one() {
    // Cheap injectivity check across a large, varied population: equal bytes
    // must mean equal snapshots, or replay could silently substitute one tick
    // for another.
    let mut seen: Vec<(Vec<u8>, InputSnapshot)> = Vec::new();
    for seed in 0..200 {
        for snapshot in snapshots(seed) {
            let bytes = snapshot.encode();
            if let Some((_, other)) = seen.iter().find(|(candidate, _)| *candidate == bytes) {
                assert_eq!(*other, snapshot, "two different snapshots encoded alike");
            } else {
                seen.push((bytes, snapshot));
            }
        }
    }
    assert!(
        seen.len() > 100,
        "the population was too small to mean much"
    );
}

#[test]
fn a_truncated_recording_fails_rather_than_decoding_into_something_plausible() {
    // A crashed session leaves a partial write, and that partial write is
    // precisely the repro worth keeping (input.md §5) — so it must fail loudly
    // at the cut, not decode into a tick that never happened.
    for seed in 0..50 {
        for snapshot in snapshots(seed) {
            let bytes = snapshot.encode();
            for length in 0..bytes.len() {
                assert!(
                    InputSnapshot::try_decode(&bytes[..length]).is_err(),
                    "seed {seed}: a {length}-byte prefix decoded"
                );
            }
        }
    }
}
