//! Scripted input: what a test writes instead of hands on a keyboard.
//!
//! Key types: `InputScript`.
//! Depends on: `key`, `pointer`, `snapshot`.
//! INVARIANT: a script is a pure function from tick to snapshot. No hidden
//! cursor, no "next" — [`InputScript::snapshot_at`] can be asked for tick 900
//! before tick 3 and answers the same either way, which is what lets a test
//! seek, replay, and bisect (input.md §5).
//!
//! This is the engine's thesis in one type: an agent scripts input, runs N
//! headless ticks, asserts on world state, and never opens a window.

use core::ops::Range;
use core::panic::Location;

use jidousha_core::math::Vec2;
use jidousha_core::message;

use crate::key::Key;
use crate::pointer::{PointerButton, PointerId, PointerState};
use crate::snapshot::{InputSnapshot, insert_sorted};

/// One instruction in a script, and the line that wrote it.
#[derive(Clone, Debug)]
enum Directive {
    /// A key down for a range of ticks.
    Hold {
        key: Key,
        ticks: Range<u64>,
        at: String,
    },
    /// A key tapped on one tick: pressed, held, and released on it.
    Press { key: Key, tick: u64, at: String },
    /// The pointer's position, from this tick until the next such directive.
    PointerAt { tick: u64, screen: Vec2, at: String },
    /// A button tapped on one tick.
    Click {
        button: PointerButton,
        tick: u64,
        at: String,
    },
}

impl Directive {
    /// How the directive reads back to whoever wrote it.
    fn describe(&self) -> String {
        match self {
            Directive::Hold { key, ticks, at } => {
                format!("hold({key}, {}..{}) at {at}", ticks.start, ticks.end)
            }
            Directive::Press { key, tick, at } => format!("press({key}, {tick}) at {at}"),
            Directive::PointerAt { tick, screen, at } => {
                format!("pointer_at({tick}, {screen:?}) at {at}")
            }
            Directive::Click { button, tick, at } => {
                format!("click({button}, {tick}) at {at}")
            }
        }
    }
}

/// A scripted input session, built once and read per tick.
///
/// ```
/// use jidousha_input::{InputScript, Key, PointerButton};
/// use jidousha_core::math::Vec2;
///
/// let script = InputScript::new()
///     .hold(Key::D, 10..120)
///     .press(Key::Space, 30)
///     .pointer_at(60, Vec2::new(400.0, 300.0))
///     .click(PointerButton::Primary, 61);
///
/// // Walking right from tick 10, still walking at 100.
/// assert!(script.snapshot_at(10).pressed_keys().contains(&Key::D));
/// assert!(script.snapshot_at(100).held_keys().contains(&Key::D));
/// assert!(script.snapshot_at(120).released_keys().contains(&Key::D));
///
/// // The jump is one tick: pressed, held, released, all at 30.
/// let jump = script.snapshot_at(30);
/// assert!(jump.pressed_keys().contains(&Key::Space));
/// assert!(jump.released_keys().contains(&Key::Space));
/// ```
pub struct InputScript {
    directives: Vec<Directive>,
}

impl InputScript {
    /// An empty script: every tick reports the player doing nothing.
    ///
    /// DELIBERATE: no `Default` impl, despite `clippy::new_without_default`
    /// (see ADR-0012) — one way to do everything, and `new` is that way.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            directives: Vec::new(),
        }
    }

    /// Hold `key` down for `ticks`.
    ///
    /// The press edge lands on `ticks.start` and the release edge on
    /// `ticks.end` — the range is half-open, like every other range in Rust, so
    /// `10..120` means down on tick 119 and up on tick 120.
    ///
    /// # Panics
    ///
    /// If this contradicts an earlier directive for the same key.
    #[must_use]
    #[track_caller]
    pub fn hold(mut self, key: Key, ticks: Range<u64>) -> Self {
        self.add(Directive::Hold {
            key,
            ticks,
            at: Location::caller().to_string(),
        });
        self
    }

    /// Tap `key` on `tick`: pressed, held, and released, all on that one tick.
    ///
    /// # Panics
    ///
    /// If this contradicts an earlier directive for the same key.
    #[must_use]
    #[track_caller]
    pub fn press(mut self, key: Key, tick: u64) -> Self {
        self.add(Directive::Press {
            key,
            tick,
            at: Location::caller().to_string(),
        });
        self
    }

    /// Put the pointer at `screen` from `tick` onwards.
    ///
    /// # Panics
    ///
    /// If another directive already moves the pointer on this tick, or if the
    /// position is not finite.
    #[must_use]
    #[track_caller]
    pub fn pointer_at(mut self, tick: u64, screen: Vec2) -> Self {
        crate::snapshot::expect_finite(screen, &PointerId::PRIMARY);
        self.add(Directive::PointerAt {
            tick,
            screen,
            at: Location::caller().to_string(),
        });
        self
    }

    /// Tap `button` on `tick`.
    ///
    /// # Panics
    ///
    /// If another directive already clicks that button on this tick.
    #[must_use]
    #[track_caller]
    pub fn click(mut self, button: PointerButton, tick: u64) -> Self {
        self.add(Directive::Click {
            button,
            tick,
            at: Location::caller().to_string(),
        });
        self
    }

    /// What the player is doing on `tick`.
    #[must_use]
    pub fn snapshot_at(&self, tick: u64) -> InputSnapshot {
        let mut snapshot = InputSnapshot::new();
        let mut pointer = PointerState::new(PointerId::PRIMARY);
        // The last position set at or before this tick — the pointer stays
        // where it was put.
        let mut placed_at = None;

        for directive in &self.directives {
            match directive {
                Directive::Hold { key, ticks, .. } => {
                    if ticks.contains(&tick) {
                        insert_sorted(&mut snapshot.held, *key);
                    }
                    if tick == ticks.start {
                        insert_sorted(&mut snapshot.pressed, *key);
                    }
                    if tick == ticks.end {
                        insert_sorted(&mut snapshot.released, *key);
                    }
                }
                Directive::Press { key, tick: at, .. } if *at == tick => {
                    insert_sorted(&mut snapshot.held, *key);
                    insert_sorted(&mut snapshot.pressed, *key);
                    insert_sorted(&mut snapshot.released, *key);
                }
                Directive::PointerAt {
                    tick: at, screen, ..
                } if *at <= tick => {
                    if placed_at.is_none_or(|placed| *at >= placed) {
                        pointer.screen = *screen;
                        placed_at = Some(*at);
                    }
                }
                Directive::Click {
                    button, tick: at, ..
                } if *at == tick => {
                    insert_sorted(&mut pointer.held, *button);
                    insert_sorted(&mut pointer.pressed, *button);
                    insert_sorted(&mut pointer.released, *button);
                }
                _ => {}
            }
        }

        snapshot.pointers = vec![pointer];
        snapshot
    }

    /// The last tick any directive mentions, so a test can drive the whole
    /// script without restating its length.
    #[must_use]
    pub fn last_tick(&self) -> u64 {
        self.directives
            .iter()
            .map(|directive| match directive {
                Directive::Hold { ticks, .. } => ticks.end,
                Directive::Press { tick, .. }
                | Directive::PointerAt { tick, .. }
                | Directive::Click { tick, .. } => *tick,
            })
            .max()
            .unwrap_or(0)
    }

    /// Add a directive, refusing anything that contradicts what is there.
    #[track_caller]
    fn add(&mut self, directive: Directive) {
        if let Some(existing) = self
            .directives
            .iter()
            .find(|other| conflicts(other, &directive))
        {
            panic!(
                "{}",
                message(
                    "contradictory input script",
                    &format!(
                        "{}\n  conflicts with: {}",
                        directive.describe(),
                        existing.describe()
                    ),
                    "two directives say different things about the same key or button on the \
                     same tick, and a snapshot can only hold one answer",
                    "widen or move one of the two ranges so they do not overlap; a key held \
                     across a tick cannot also be tapped on it",
                )
            );
        }
        self.directives.push(directive);
    }
}

/// Whether two directives disagree about the same tick.
fn conflicts(left: &Directive, right: &Directive) -> bool {
    match (left, right) {
        (
            Directive::Hold {
                key: a, ticks: x, ..
            },
            Directive::Hold {
                key: b, ticks: y, ..
            },
        ) => a == b && x.start < y.end && y.start < x.end,
        (
            Directive::Hold {
                key: a, ticks: x, ..
            },
            Directive::Press {
                key: b, tick: t, ..
            },
        )
        | (
            Directive::Press {
                key: b, tick: t, ..
            },
            Directive::Hold {
                key: a, ticks: x, ..
            },
            // A tap on the tick a hold releases is fine: both say "up from
            // here". A tap anywhere inside the hold is not.
        ) => a == b && x.contains(t),
        (
            Directive::Press {
                key: a, tick: x, ..
            },
            Directive::Press {
                key: b, tick: y, ..
            },
        ) => a == b && x == y,
        (
            Directive::PointerAt {
                tick: x,
                screen: left,
                ..
            },
            Directive::PointerAt {
                tick: y,
                screen: right,
                ..
            },
        ) => x == y && left != right,
        (
            Directive::Click {
                button: a, tick: x, ..
            },
            Directive::Click {
                button: b, tick: y, ..
            },
        ) => a == b && x == y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hold_presses_at_the_start_and_releases_at_the_end() {
        let script = InputScript::new().hold(Key::D, 10..15);
        assert!(script.snapshot_at(9).held_keys().is_empty());
        assert_eq!(script.snapshot_at(10).pressed_keys(), [Key::D]);
        assert_eq!(script.snapshot_at(12).held_keys(), [Key::D]);
        assert!(script.snapshot_at(12).pressed_keys().is_empty());
        assert_eq!(
            script.snapshot_at(14).held_keys(),
            [Key::D],
            "last held tick"
        );
        assert_eq!(script.snapshot_at(15).released_keys(), [Key::D]);
        assert!(script.snapshot_at(15).held_keys().is_empty());
    }

    #[test]
    fn a_press_is_a_tap_on_one_tick() {
        let script = InputScript::new().press(Key::Space, 30);
        let snapshot = script.snapshot_at(30);
        assert_eq!(snapshot.pressed_keys(), [Key::Space]);
        assert_eq!(snapshot.held_keys(), [Key::Space]);
        assert_eq!(snapshot.released_keys(), [Key::Space]);
        assert!(script.snapshot_at(31).held_keys().is_empty());
    }

    #[test]
    fn the_pointer_stays_where_it_was_put() {
        let script = InputScript::new()
            .pointer_at(10, Vec2::new(100.0, 50.0))
            .pointer_at(20, Vec2::new(200.0, 60.0));
        assert_eq!(script.snapshot_at(5).pointers()[0].screen, Vec2::ZERO);
        assert_eq!(
            script.snapshot_at(15).pointers()[0].screen,
            Vec2::new(100.0, 50.0)
        );
        assert_eq!(
            script.snapshot_at(999).pointers()[0].screen,
            Vec2::new(200.0, 60.0)
        );
    }

    #[test]
    fn a_click_is_a_tap_on_one_tick() {
        let script = InputScript::new().click(PointerButton::Primary, 61);
        let pointer = &script.snapshot_at(61).pointers()[0].clone();
        assert!(pointer.just_pressed(PointerButton::Primary));
        assert!(pointer.held(PointerButton::Primary));
        assert!(pointer.just_released(PointerButton::Primary));
        assert!(!script.snapshot_at(62).pointers()[0].held(PointerButton::Primary));
    }

    #[test]
    fn asking_for_ticks_out_of_order_gives_the_same_answers() {
        // A script is a function of the tick, not a cursor — which is what lets
        // a test seek and bisect.
        let script = InputScript::new().hold(Key::W, 5..9).press(Key::Space, 20);
        let forwards: Vec<InputSnapshot> = (0..25).map(|tick| script.snapshot_at(tick)).collect();
        let backwards: Vec<InputSnapshot> =
            (0..25).rev().map(|tick| script.snapshot_at(tick)).collect();
        assert_eq!(forwards, backwards.into_iter().rev().collect::<Vec<_>>());
    }

    #[test]
    fn the_last_tick_is_the_furthest_any_directive_reaches() {
        let script = InputScript::new()
            .hold(Key::D, 10..120)
            .press(Key::Space, 30);
        assert_eq!(script.last_tick(), 120);
        assert_eq!(InputScript::new().last_tick(), 0);
    }

    #[test]
    #[should_panic(expected = "contradictory input script")]
    fn a_tap_inside_a_hold_is_refused() {
        let _ = InputScript::new().hold(Key::D, 5..10).press(Key::D, 7);
    }

    #[test]
    #[should_panic(expected = "contradictory input script")]
    fn the_conflict_is_caught_whichever_order_it_is_written_in() {
        let _ = InputScript::new().press(Key::D, 7).hold(Key::D, 5..10);
    }

    #[test]
    #[should_panic(expected = "contradictory input script")]
    fn overlapping_holds_of_one_key_are_refused() {
        let _ = InputScript::new().hold(Key::D, 5..10).hold(Key::D, 8..12);
    }

    #[test]
    fn holds_that_only_touch_at_the_edge_are_fine() {
        // 5..10 releases on tick 10, and 10..15 presses on it. That is a real
        // thing a player does, so it must not be a conflict.
        let script = InputScript::new().hold(Key::D, 5..10).hold(Key::D, 10..15);
        let snapshot = script.snapshot_at(10);
        assert_eq!(snapshot.pressed_keys(), [Key::D]);
        assert_eq!(snapshot.released_keys(), [Key::D]);
    }

    #[test]
    fn different_keys_never_conflict() {
        let script = InputScript::new().hold(Key::D, 5..10).hold(Key::W, 5..10);
        assert_eq!(script.snapshot_at(7).held_keys(), [Key::D, Key::W]);
    }

    #[test]
    fn the_conflict_message_names_both_directives_and_their_lines() {
        let panic = std::panic::catch_unwind(|| {
            let _ = InputScript::new().hold(Key::D, 5..10).press(Key::D, 7);
        })
        .err();
        let text = panic
            .and_then(|payload| payload.downcast::<String>().ok())
            .map(|boxed| *boxed)
            .unwrap_or_default();
        assert!(text.contains("press(D, 7)"), "{text}");
        assert!(text.contains("hold(D, 5..10)"), "{text}");
        assert!(text.contains("script.rs:"), "both lines are named: {text}");
    }

    #[test]
    fn repeating_the_same_pointer_position_is_not_a_conflict() {
        // Saying the same thing twice is redundant, not contradictory.
        let script = InputScript::new()
            .pointer_at(10, Vec2::new(1.0, 2.0))
            .pointer_at(10, Vec2::new(1.0, 2.0));
        assert_eq!(
            script.snapshot_at(10).pointers()[0].screen,
            Vec2::new(1.0, 2.0)
        );
    }

    #[test]
    #[should_panic(expected = "contradictory input script")]
    fn two_positions_on_one_tick_are_refused() {
        let _ = InputScript::new()
            .pointer_at(10, Vec2::new(1.0, 2.0))
            .pointer_at(10, Vec2::new(3.0, 4.0));
    }
}
