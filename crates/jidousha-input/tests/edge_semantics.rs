//! The I0 exit criterion: every event produces exactly one edge, on exactly
//! one tick, under thousands of random event streams (input.md §8).
//!
//! Two checks over the same runs. The first compares every tick against a
//! naive model. The second counts edges across a whole run and asserts the
//! total matches what the stream asked for — a model can agree tick by tick
//! and still lose an edge at a frame boundary, and this is the check that
//! would notice.

mod support;

use std::collections::BTreeMap;

use jidousha_input::{
    InputEvent, InputSnapshot, Key, MAX_TOUCHES, PointerButton, PointerId, SnapshotBuilder,
    TouchPhase,
};
use support::{ALPHABET, Expected, Reference, Step, generate};

/// How many independent streams to run, and how long each one is.
const STREAMS: u64 = 2000;
const STREAM_LENGTH: usize = 150;

/// Everything one run produced, for the checks that need the whole thing.
struct Run {
    /// Every snapshot the driver saw, in order.
    snapshots: Vec<InputSnapshot>,
    /// Press edges seen per key, across the run.
    presses: BTreeMap<Key, usize>,
    /// Release edges seen per key, across the run.
    releases: BTreeMap<Key, usize>,
}

/// Drive the builder and the model in lockstep, comparing every tick.
fn run(steps: &[Step]) -> Result<Run, String> {
    let mut builder = SnapshotBuilder::new();
    let mut model = Reference::new();
    let mut out = Run {
        snapshots: Vec::new(),
        presses: BTreeMap::new(),
        releases: BTreeMap::new(),
    };

    for (index, step) in steps.iter().enumerate() {
        match *step {
            Step::Event(event) => {
                builder.record(event);
                model.record(event);
            }
            Step::Frame { ticks } => {
                for tick in 0..ticks {
                    let (snapshot, want) = if tick == 0 {
                        (builder.first_tick_snapshot(), model.first_tick())
                    } else {
                        (builder.catch_up_snapshot(), model.catch_up_tick())
                    };
                    let got = Expected::of(&snapshot);
                    if got != want {
                        return Err(format!(
                            "step {index}, tick {tick} of the frame:\n  got  {got:?}\n  want {want:?}"
                        ));
                    }
                    for &key in snapshot.pressed_keys() {
                        *out.presses.entry(key).or_default() += 1;
                    }
                    for &key in snapshot.released_keys() {
                        *out.releases.entry(key).or_default() += 1;
                    }
                    out.snapshots.push(snapshot);
                }
            }
        }
    }
    Ok(out)
}

/// Count the edges the stream *asked* for, by grouping events into the windows
/// between emitted first ticks.
///
/// This is deliberately a different shape of computation from the builder's:
/// the builder inserts edges incrementally as events arrive, while this slices
/// the stream into windows and asks how many windows mention each key. One
/// press edge per key per window is the whole contract, stated as arithmetic.
fn expected_edge_counts(steps: &[Step]) -> (BTreeMap<Key, usize>, BTreeMap<Key, usize>) {
    let mut presses: BTreeMap<Key, usize> = BTreeMap::new();
    let mut releases: BTreeMap<Key, usize> = BTreeMap::new();

    let mut window_pressed: Vec<Key> = Vec::new();
    let mut window_released: Vec<Key> = Vec::new();
    // Physical state, needed only to know what a focus loss releases.
    let mut down: Vec<Key> = Vec::new();

    for step in steps {
        match *step {
            Step::Event(InputEvent::KeyPressed(key)) => {
                push_once(&mut window_pressed, key);
                push_once(&mut down, key);
            }
            Step::Event(InputEvent::KeyReleased(key)) => {
                push_once(&mut window_released, key);
                down.retain(|held| *held != key);
            }
            Step::Event(InputEvent::FocusLost) => {
                for key in core::mem::take(&mut down) {
                    push_once(&mut window_released, key);
                }
            }
            Step::Event(_) => {}
            Step::Frame { ticks } => {
                if ticks == 0 {
                    // No tick ran, so this window is still open: the events
                    // carry over rather than being spent or lost.
                    continue;
                }
                for key in window_pressed.drain(..) {
                    *presses.entry(key).or_default() += 1;
                }
                for key in window_released.drain(..) {
                    *releases.entry(key).or_default() += 1;
                }
            }
        }
    }
    // Anything after the last frame boundary was never observed by a tick, and
    // is correctly absent from both counts.
    (presses, releases)
}

fn push_once(list: &mut Vec<Key>, key: Key) {
    if !list.contains(&key) {
        list.push(key);
    }
}

#[test]
fn every_tick_matches_the_reference_model_under_random_event_streams() {
    for seed in 0..STREAMS {
        let steps = generate(seed, STREAM_LENGTH);
        if let Err(mismatch) = run(&steps) {
            panic!("seed {seed}: {mismatch}");
        }
    }
}

#[test]
fn every_event_produces_exactly_one_edge_across_the_whole_run() {
    for seed in 0..STREAMS {
        let steps = generate(seed, STREAM_LENGTH);
        let Ok(actual) = run(&steps) else {
            // The model test above reports mismatches properly.
            continue;
        };
        let (presses, releases) = expected_edge_counts(&steps);
        assert_eq!(
            actual.presses, presses,
            "seed {seed}: press edges do not match the stream"
        );
        assert_eq!(
            actual.releases, releases,
            "seed {seed}: release edges do not match the stream"
        );
    }
}

#[test]
fn a_press_is_always_accompanied_by_a_held_bit() {
    // The invariant a game would notice first: just_pressed(k) && !held(k) is
    // a state that must not exist.
    for seed in 0..STREAMS {
        let Ok(run) = run(&generate(seed, STREAM_LENGTH)) else {
            continue;
        };
        for snapshot in &run.snapshots {
            for key in snapshot.pressed_keys() {
                assert!(
                    snapshot.held_keys().contains(key),
                    "seed {seed}: {key} was pressed without being held"
                );
            }
        }
    }
}

#[test]
fn catch_up_ticks_never_carry_an_edge() {
    // Stated directly rather than through the model, because this is the rule
    // a driver is most likely to break by calling the wrong method.
    let mut builder = SnapshotBuilder::new();
    builder.record(InputEvent::KeyPressed(Key::Space));
    let _ = builder.first_tick_snapshot();
    for _ in 0..5 {
        let snapshot = builder.catch_up_snapshot();
        assert!(snapshot.pressed_keys().is_empty());
        assert!(snapshot.released_keys().is_empty());
        assert_eq!(snapshot.held_keys(), [Key::Space], "still down, though");
    }
}

#[test]
fn a_catch_up_snapshot_carries_no_edges_even_with_events_pending() {
    // The guarantee is unconditional — it does not depend on the driver having
    // called `first_tick_snapshot` first. Found by mutation testing: with the
    // normal ordering the edges are already spent, so a `catch_up_snapshot`
    // that leaked them looked identical from every other test in this file.
    let mut builder = SnapshotBuilder::new();
    builder.record(InputEvent::KeyPressed(Key::Space));
    builder.record(InputEvent::Scrolled {
        id: PointerId::PRIMARY,
        lines: 3.0,
    });

    let snapshot = builder.catch_up_snapshot();
    assert!(snapshot.pressed_keys().is_empty(), "no press edge");
    assert!(snapshot.released_keys().is_empty(), "no release edge");
    assert_eq!(snapshot.pointers()[0].scroll, 0.0, "no scroll");
    assert_eq!(
        snapshot.held_keys(),
        [Key::Space],
        "the state behind the edge is still true, though"
    );
}

#[test]
fn the_generated_streams_reach_the_states_that_make_this_worth_running() {
    // Guards the generator, not the builder. Each of these is a case the
    // comparison above is worthless without, and none is guaranteed by
    // construction.
    let mut taps = 0;
    let mut catch_up_ticks = 0;
    let mut empty_frames = 0;
    let mut focus_losses = 0;
    let mut multi_key_ticks = 0;
    let mut touch_begins = 0;
    let mut touch_ends = 0;
    let mut touch_cancels = 0;
    let mut full_glass = 0;
    let mut mirrored_presses = 0;

    for seed in 0..STREAMS {
        let steps = generate(seed, STREAM_LENGTH);
        for step in &steps {
            match step {
                Step::Frame { ticks: 0 } => empty_frames += 1,
                Step::Frame { ticks } => catch_up_ticks += ticks.saturating_sub(1),
                Step::Event(InputEvent::FocusLost) => focus_losses += 1,
                Step::Event(_) => {}
            }
        }
        let Ok(run) = run(&steps) else { continue };
        for snapshot in &run.snapshots {
            for key in snapshot.pressed_keys() {
                if snapshot.released_keys().contains(key) {
                    taps += 1;
                }
            }
            if snapshot.held_keys().len() > 1 {
                multi_key_ticks += 1;
            }
            for touch in snapshot.touches() {
                match touch.phase {
                    TouchPhase::Began => touch_begins += 1,
                    TouchPhase::Ended => touch_ends += 1,
                    TouchPhase::Cancelled => touch_cancels += 1,
                    TouchPhase::Moved => {}
                }
            }
            if snapshot.touches().len() == MAX_TOUCHES {
                full_glass += 1;
            }
            // The mirror, observed from the outside: a tick that begins a touch
            // and presses the primary button is the whole promise of §3a.
            if snapshot.pointers()[0].just_pressed(PointerButton::Primary)
                && snapshot
                    .touches()
                    .iter()
                    .any(|touch| touch.phase == TouchPhase::Began)
            {
                mirrored_presses += 1;
            }
        }
    }

    assert!(taps > 0, "no key was ever tapped inside a single frame");
    assert!(catch_up_ticks > 0, "no frame ever ran a catch-up tick");
    assert!(empty_frames > 0, "no frame ever ran zero ticks");
    assert!(focus_losses > 0, "focus was never lost");
    assert!(multi_key_ticks > 0, "two keys were never held at once");
    assert!(touch_begins > 0, "no finger ever landed");
    assert!(touch_ends > 0, "no finger ever lifted");
    assert!(touch_cancels > 0, "no touch was ever cancelled");
    assert!(full_glass > 0, "the four-touch bound was never reached");
    assert!(
        mirrored_presses > 0,
        "no touch ever mirrored onto the cursor"
    );
    assert!(
        ALPHABET.len() > 1,
        "the alphabet must have room for collisions"
    );
}
