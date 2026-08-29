//! Touch through the snapshot, one transcript at a time (input.md §3a, ADR-0043).
//!
//! The property tests next door check touch against a naive model over
//! thousands of random streams; this file states the rules a person would want
//! to read, as sequences short enough to follow: a finger lands, moves and
//! lifts; the first one drives the cursor and the second one does not; a fifth
//! finger is dropped; a window that loses focus cancels what is on the glass.
//!
//! Everything here goes through `SnapshotBuilder`, which is the path a real
//! touchscreen takes — there is no second way to put a touch into a snapshot,
//! and a test that had one would be checking a road no player drives (ADR-0019).

use jidousha_core::math::Vec2;
use jidousha_input::{
    FingerId, Input, InputEvent, InputSnapshot, MAX_TOUCHES, PointerButton, SnapshotBuilder, Touch,
    TouchPhase,
};

/// A finger, named the way a platform would name it.
fn finger(id: u64) -> FingerId {
    FingerId::from_platform(id)
}

/// One touch event, for the builder.
fn touched(id: u64, phase: TouchPhase, at: (f32, f32)) -> InputEvent {
    InputEvent::Touched {
        finger: finger(id),
        phase,
        screen: Vec2::new(at.0, at.1),
    }
}

/// What a snapshot's touches look like, as `(slot, phase, position)`.
fn touches(snapshot: &InputSnapshot) -> Vec<(u8, TouchPhase, Vec2)> {
    snapshot
        .touches()
        .iter()
        .map(|touch: &Touch| (touch.id.slot(), touch.phase, touch.screen))
        .collect()
}

/// Whether the primary pointer is pressed, and where it is.
fn cursor(snapshot: &InputSnapshot) -> (Vec2, bool, bool, bool) {
    let input = Input::new(snapshot.clone());
    let pointer = input.pointer();
    (
        pointer.screen,
        pointer.held(PointerButton::Primary),
        pointer.just_pressed(PointerButton::Primary),
        pointer.just_released(PointerButton::Primary),
    )
}

#[test]
fn a_finger_lands_moves_and_lifts_across_three_ticks() {
    let mut builder = SnapshotBuilder::new();

    builder.record(touched(7, TouchPhase::Began, (100.0, 200.0)));
    let landed = builder.first_tick_snapshot();
    assert_eq!(
        touches(&landed),
        vec![(0, TouchPhase::Began, Vec2::new(100.0, 200.0))]
    );

    builder.record(touched(7, TouchPhase::Moved, (140.0, 205.0)));
    let dragged = builder.first_tick_snapshot();
    assert_eq!(
        touches(&dragged),
        vec![(0, TouchPhase::Moved, Vec2::new(140.0, 205.0))],
        "the same slot, because a slot belongs to a finger for its whole life"
    );

    builder.record(touched(7, TouchPhase::Ended, (150.0, 210.0)));
    let lifted = builder.first_tick_snapshot();
    assert_eq!(
        touches(&lifted),
        vec![(0, TouchPhase::Ended, Vec2::new(150.0, 210.0))]
    );

    let after = builder.first_tick_snapshot();
    assert!(
        after.touches().is_empty(),
        "a lifted finger is gone the tick after it lifts, not the tick it lifts"
    );
}

#[test]
fn a_finger_that_is_simply_resting_still_reports_every_tick() {
    // The reason `Moved` doubles as "down and unchanged": a game asking what is
    // on the glass has to be told on every tick, not only when something
    // changes, or it would have to keep its own copy of the state the snapshot
    // is supposed to be.
    let mut builder = SnapshotBuilder::new();
    builder.record(touched(1, TouchPhase::Began, (10.0, 10.0)));
    let _ = builder.first_tick_snapshot();

    for _ in 0..5 {
        let quiet = builder.first_tick_snapshot();
        assert_eq!(
            touches(&quiet),
            vec![(0, TouchPhase::Moved, Vec2::new(10.0, 10.0))]
        );
    }
}

#[test]
fn the_first_finger_down_drives_the_cursor_and_presses_the_primary_button() {
    // The mirror rule, which is the whole reason a game written for a mouse is
    // playable with a thumb (input.md §3a).
    let mut builder = SnapshotBuilder::new();

    builder.record(touched(3, TouchPhase::Began, (320.0, 240.0)));
    let (at, held, pressed, released) = cursor(&builder.first_tick_snapshot());
    assert_eq!(at, Vec2::new(320.0, 240.0), "the cursor went to the finger");
    assert!(pressed, "and the primary button went down with it");
    assert!(held);
    assert!(!released);

    builder.record(touched(3, TouchPhase::Moved, (400.0, 300.0)));
    let (at, held, pressed, _) = cursor(&builder.first_tick_snapshot());
    assert_eq!(at, Vec2::new(400.0, 300.0), "a drag drags the cursor");
    assert!(held, "still down");
    assert!(
        !pressed,
        "and the press edge was spent on the tick it landed"
    );

    builder.record(touched(3, TouchPhase::Ended, (410.0, 305.0)));
    let (at, held, _, released) = cursor(&builder.first_tick_snapshot());
    assert_eq!(at, Vec2::new(410.0, 305.0));
    assert!(released, "lifting the finger releases the button");
    assert!(!held);
}

#[test]
fn a_second_finger_never_takes_the_cursor_from_the_first() {
    // The determinism the rule buys: whichever order two fingers are reported
    // in, the cursor is on the one that landed first, and stays there.
    let mut builder = SnapshotBuilder::new();
    builder.record(touched(1, TouchPhase::Began, (100.0, 100.0)));
    builder.record(touched(2, TouchPhase::Began, (700.0, 500.0)));
    let both = builder.first_tick_snapshot();

    assert_eq!(
        touches(&both),
        vec![
            (0, TouchPhase::Began, Vec2::new(100.0, 100.0)),
            (1, TouchPhase::Began, Vec2::new(700.0, 500.0)),
        ],
        "both fingers are in the snapshot, in slot order"
    );
    let (at, _, pressed, _) = cursor(&both);
    assert_eq!(at, Vec2::new(100.0, 100.0), "the first one to land");
    assert!(pressed, "one press, not two");

    // And moving the second one does not drag the cursor either.
    builder.record(touched(2, TouchPhase::Moved, (10.0, 10.0)));
    let (at, _, _, _) = cursor(&builder.first_tick_snapshot());
    assert_eq!(at, Vec2::new(100.0, 100.0));
}

#[test]
fn the_cursor_is_not_handed_to_a_finger_that_is_already_down() {
    // The other half of "first active touch wins": when the mirrored finger
    // lifts, the button releases. The finger still on the glass is *not*
    // promoted — a cursor that teleported to a thumb the player had been
    // resting there is a click nobody made.
    let mut builder = SnapshotBuilder::new();
    builder.record(touched(1, TouchPhase::Began, (100.0, 100.0)));
    builder.record(touched(2, TouchPhase::Began, (700.0, 500.0)));
    let _ = builder.first_tick_snapshot();

    builder.record(touched(1, TouchPhase::Ended, (100.0, 100.0)));
    let (at, held, pressed, released) = cursor(&builder.first_tick_snapshot());
    assert!(released, "the mirrored finger lifted");
    assert!(!held);
    assert!(!pressed);
    assert_eq!(at, Vec2::new(100.0, 100.0), "and it did not jump");

    // A *new* finger may take the mirror, because nothing holds it now.
    builder.record(touched(3, TouchPhase::Began, (640.0, 480.0)));
    let (at, _, pressed, _) = cursor(&builder.first_tick_snapshot());
    assert!(pressed);
    assert_eq!(at, Vec2::new(640.0, 480.0));
}

#[test]
fn a_tap_inside_one_frame_reports_its_landing_and_then_its_lift() {
    // The touch spelling of input.md §2's tap rule. One entry per touch per
    // tick means the two edges cannot share a tick, so the second waits — and
    // neither is lost, which is what a game keying a tap off `Ended` needs.
    let mut builder = SnapshotBuilder::new();
    builder.record(touched(4, TouchPhase::Began, (50.0, 60.0)));
    builder.record(touched(4, TouchPhase::Ended, (52.0, 61.0)));

    let landing = builder.first_tick_snapshot();
    assert_eq!(
        touches(&landing),
        vec![(0, TouchPhase::Began, Vec2::new(52.0, 61.0))],
        "at the position it ended at, which is where the tap was"
    );
    let (_, _, pressed, released) = cursor(&landing);
    assert!(pressed, "the mirrored press");
    assert!(
        released,
        "and its release, on the same tick — a tap is a click"
    );

    let lift = builder.first_tick_snapshot();
    assert_eq!(
        touches(&lift),
        vec![(0, TouchPhase::Ended, Vec2::new(52.0, 61.0))],
        "the lift was owed, not lost"
    );

    assert!(builder.first_tick_snapshot().touches().is_empty());
}

#[test]
fn a_fifth_finger_is_dropped_and_says_nothing() {
    // The documented bound: four is what the snapshot carries, and the fifth
    // finger is dropped the way an unmapped key is — a boundary, not a
    // failure (input.md §3a).
    let mut builder = SnapshotBuilder::new();
    for id in 0..6u64 {
        builder.record(touched(id, TouchPhase::Began, (id as f32, 0.0)));
    }
    let full = builder.first_tick_snapshot();
    assert_eq!(full.touches().len(), MAX_TOUCHES);
    assert_eq!(
        full.touches()
            .iter()
            .map(|touch| touch.screen.x)
            .collect::<Vec<f32>>(),
        vec![0.0, 1.0, 2.0, 3.0],
        "the first four, and the fifth and sixth were never heard from"
    );

    // And the fifth finger's later events do not resurrect it.
    builder.record(touched(5, TouchPhase::Moved, (999.0, 999.0)));
    builder.record(touched(5, TouchPhase::Ended, (999.0, 999.0)));
    let next = builder.first_tick_snapshot();
    assert_eq!(next.touches().len(), MAX_TOUCHES);
    assert!(next.touches().iter().all(|touch| touch.screen.x < 4.0));
}

#[test]
fn a_slot_is_reused_once_its_finger_is_gone() {
    let mut builder = SnapshotBuilder::new();
    builder.record(touched(1, TouchPhase::Began, (10.0, 0.0)));
    builder.record(touched(2, TouchPhase::Began, (20.0, 0.0)));
    let _ = builder.first_tick_snapshot();

    builder.record(touched(1, TouchPhase::Ended, (10.0, 0.0)));
    let _ = builder.first_tick_snapshot();

    builder.record(touched(3, TouchPhase::Began, (30.0, 0.0)));
    let reused = builder.first_tick_snapshot();
    assert_eq!(
        touches(&reused),
        vec![
            (0, TouchPhase::Began, Vec2::new(30.0, 0.0)),
            (1, TouchPhase::Moved, Vec2::new(20.0, 0.0)),
        ],
        "slot 0 is whoever is in slot 0 now, not the first finger of the session"
    );
}

#[test]
fn losing_focus_cancels_every_finger_and_releases_the_mirror() {
    // The touch half of input.md §4. Cancelled rather than ended: the fingers
    // may still be on the glass, and what the engine knows is that it stopped
    // being told about them.
    let mut builder = SnapshotBuilder::new();
    builder.record(touched(1, TouchPhase::Began, (100.0, 100.0)));
    builder.record(touched(2, TouchPhase::Began, (200.0, 200.0)));
    let _ = builder.first_tick_snapshot();

    builder.record(InputEvent::FocusLost);
    let lost = builder.first_tick_snapshot();
    assert_eq!(
        touches(&lost),
        vec![
            (0, TouchPhase::Cancelled, Vec2::new(100.0, 100.0)),
            (1, TouchPhase::Cancelled, Vec2::new(200.0, 200.0)),
        ]
    );
    let (_, held, _, released) = cursor(&lost);
    assert!(released, "the mirrored button came up with them");
    assert!(!held);
    assert!(!lost.window_focused());

    assert!(
        builder.first_tick_snapshot().touches().is_empty(),
        "and the glass is clear afterwards"
    );

    // The platform's own cancellations, arriving after ours, change nothing.
    builder.record(touched(1, TouchPhase::Cancelled, (100.0, 100.0)));
    assert!(builder.first_tick_snapshot().touches().is_empty());
}

#[test]
fn a_catch_up_tick_shows_the_fingers_and_none_of_the_edges() {
    // input.md §2's catch-up rule, in touch: three catch-up ticks must not
    // report one landing three times.
    let mut builder = SnapshotBuilder::new();
    builder.record(touched(1, TouchPhase::Began, (33.0, 44.0)));
    let first = builder.first_tick_snapshot();
    assert_eq!(first.touches()[0].phase, TouchPhase::Began);

    for _ in 0..3 {
        let catch_up = builder.catch_up_snapshot();
        assert_eq!(
            touches(&catch_up),
            vec![(0, TouchPhase::Moved, Vec2::new(33.0, 44.0))],
            "still down, carrying no edge"
        );
        let (_, held, pressed, _) = cursor(&catch_up);
        assert!(held, "the mirrored button is still down");
        assert!(!pressed, "but its edge is not repeated");
    }
}

#[test]
fn a_catch_up_tick_never_reports_a_landing_or_a_lift() {
    // Unconditional, the way the keyboard's version is: it does not depend on
    // `first_tick_snapshot` having been called first. A catch-up tick that
    // leaked a phase would be an edge on a tick that is defined not to have
    // any, and the owed end would then be reported twice.
    let mut builder = SnapshotBuilder::new();
    builder.record(touched(1, TouchPhase::Began, (1.0, 2.0)));
    builder.record(touched(2, TouchPhase::Began, (3.0, 4.0)));
    builder.record(touched(2, TouchPhase::Ended, (3.0, 4.0)));

    let catch_up = builder.catch_up_snapshot();
    assert!(
        catch_up
            .touches()
            .iter()
            .all(|touch| touch.phase == TouchPhase::Moved),
        "no edges: {:?}",
        touches(&catch_up)
    );
    let (_, _, pressed, released) = cursor(&catch_up);
    assert!(!pressed);
    assert!(!released);
}

#[test]
fn a_touch_the_engine_is_not_following_is_ignored() {
    // Every one of these is something a platform can report and the engine has
    // nothing to say about: a finger that moves without landing, one that
    // lifts twice, one that lands twice.
    let mut builder = SnapshotBuilder::new();
    builder.record(touched(9, TouchPhase::Moved, (5.0, 5.0)));
    builder.record(touched(9, TouchPhase::Ended, (5.0, 5.0)));
    assert!(builder.first_tick_snapshot().touches().is_empty());
    let (at, _, pressed, released) = cursor(&builder.first_tick_snapshot());
    assert_eq!(at, Vec2::ZERO, "and the cursor never moved");
    assert!(!pressed);
    assert!(!released);

    builder.record(touched(1, TouchPhase::Began, (10.0, 10.0)));
    builder.record(touched(1, TouchPhase::Began, (900.0, 900.0)));
    let doubled = builder.first_tick_snapshot();
    assert_eq!(
        touches(&doubled),
        vec![(0, TouchPhase::Began, Vec2::new(10.0, 10.0))],
        "one finger, one slot, at the position it actually landed"
    );

    builder.record(touched(1, TouchPhase::Ended, (10.0, 10.0)));
    builder.record(touched(1, TouchPhase::Ended, (10.0, 10.0)));
    let lifted = builder.first_tick_snapshot();
    assert_eq!(lifted.touches().len(), 1, "one lift, not two");
    assert!(builder.first_tick_snapshot().touches().is_empty());
}

#[test]
fn a_touch_at_a_non_finite_position_is_refused_where_it_enters() {
    // The same rule the pointer has: a recording holding NaN cannot replay,
    // because NaN does not equal itself.
    let mut builder = SnapshotBuilder::new();
    let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        builder.record(touched(1, TouchPhase::Began, (f32::NAN, 0.0)));
    }));
    let Err(payload) = refused else {
        panic!("a NaN touch position was accepted");
    };
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .unwrap_or("");
    assert!(message.contains("non-finite position"), "{message}");
    assert!(message.contains("finger 1"), "names the finger: {message}");
}
