//! A recording made before touch existed still replays (input.md §5, ADR-0043).
//!
//! `fixtures/pre-touch-session.jdrc` was written by the engine as it stood at
//! the commit before this one — a real file from a real encoder, not this
//! test's idea of what version 1 looked like. That distinction is the whole
//! point: a fixture the new code generates would agree with the new code by
//! construction, and would keep agreeing with it while both drifted away from
//! the files on the owner's disk.
//!
//! What it proves is one sentence: **the touch list is additive.** Everything
//! before it decodes at either version, so a version 1 snapshot is a version 2
//! snapshot read to where it stops — and it means what it always meant, which
//! is that nobody was touching anything.

use jidousha_core::Seconds;
use jidousha_input::{Key, PointerButton, Recording};

/// The file, as it was written.
const PRE_TOUCH: &[u8] = include_bytes!("fixtures/pre-touch-session.jdrc");

#[test]
fn a_recording_written_before_touch_existed_still_decodes() {
    let Ok(recording) = Recording::try_decode(PRE_TOUCH) else {
        panic!("a version 1 recording must still be readable");
    };
    assert_eq!(recording.seed(), 0xB0A7, "its header, unchanged");
    assert_eq!(recording.fixed_dt(), Seconds(1.0 / 60.0));
    assert_eq!(recording.ticks().len(), 24);
}

#[test]
fn every_tick_of_it_replays_with_an_empty_touch_list() {
    let Ok(recording) = Recording::try_decode(PRE_TOUCH) else {
        panic!("readable");
    };
    for record in recording.ticks() {
        assert!(
            record.input.touches().is_empty(),
            "tick {}: a session recorded before touch existed had no fingers on the glass",
            record.tick
        );
    }
}

#[test]
fn what_the_old_session_actually_did_is_still_what_it_did() {
    // The negative control for the two tests above: a file that decoded into
    // twenty-four empty snapshots would pass both of them and mean nothing.
    // These are the events that were recorded, read back off the fixture.
    let Ok(recording) = Recording::try_decode(PRE_TOUCH) else {
        panic!("readable");
    };
    let tick = |number: u64| {
        let Some(record) = recording
            .ticks()
            .iter()
            .find(|record| record.tick == number)
        else {
            panic!("tick {number} is in the fixture");
        };
        &record.input
    };

    assert_eq!(tick(3).pressed_keys(), [Key::D], "the key it pressed");
    assert_eq!(tick(5).held_keys(), [Key::D], "and held");
    assert_eq!(tick(12).released_keys(), [Key::D]);
    assert!(tick(7).pointers()[0].just_pressed(PointerButton::Primary));
    assert!(tick(9).pointers()[0].just_released(PointerButton::Primary));
    assert_eq!(tick(15).pointers()[0].scroll, -2.0, "the wheel it turned");
    assert_eq!(tick(18).pressed_keys(), [Key::Space]);
    assert!(!tick(21).window_focused(), "the focus it lost");
    assert!(tick(22).window_focused(), "and got back");
    let Some(fourth) = recording.ticks().iter().find(|record| record.tick == 4) else {
        panic!("tick 4 is in the fixture");
    };
    assert_eq!(
        fourth.readiness.len(),
        1,
        "and the asset that arrived on tick 4"
    );

    // The pointer walked across the window, and every position survived.
    let positions: Vec<f32> = recording
        .ticks()
        .iter()
        .map(|record| record.input.pointers()[0].screen.x)
        .collect();
    assert_eq!(positions.first(), Some(&43.5));
    assert_eq!(positions.last(), Some(&124.0));
}

#[test]
fn replaying_it_through_this_engine_writes_a_recording_of_the_new_version() {
    // Documented, not silently broken: reading an old file is an upgrade. The
    // file that comes back out is version 2 and an older engine will refuse it
    // by number rather than misread it — which is why a recording worth
    // keeping is kept as the file it was written as (input.md §5).
    let Ok(recording) = Recording::try_decode(PRE_TOUCH) else {
        panic!("readable");
    };
    let written = recording.encode();
    assert_ne!(written, PRE_TOUCH, "written at the current version");

    let Ok(again) = Recording::try_decode(&written) else {
        panic!("what this build writes, this build reads");
    };
    assert_eq!(again, recording, "and it is the same session either way");
}
