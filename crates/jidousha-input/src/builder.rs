//! Where raw events become one tick's truth: the edge rules, in one place.
//!
//! Key types: `InputEvent`, `SnapshotBuilder`.
//! Depends on: `key`, `pointer`, `snapshot`. Must never depend on: `winit` —
//! the platform crate translates its events into [`InputEvent`] and nothing
//! else crosses (ADR-0004, input.md §6).
//! INVARIANT: edges are recorded data, never the difference between two ticks.
//! A tap that begins and ends between two frames still reports its press and
//! its release; diffing the held sets would lose it entirely (input.md §2).
//!
//! DELIBERATE: this lives here rather than in `jidousha-platform`, which
//! input.md §6 originally drew it into. The edge rules and the focus-loss
//! policy are pure logic and are CONTRACTs; putting them behind the winit seam
//! would make them untestable on wasm CI and testable on native only through a
//! window. What stays platform-side is exactly what needs a platform: the
//! translation tables and scroll normalization.

use jidousha_core::math::Vec2;

use crate::key::Key;
use crate::pointer::{PointerButton, PointerId, PointerState};
use crate::snapshot::{InputSnapshot, expect_finite, insert_sorted, pointer_mut, remove_sorted};

/// One thing that happened, in the engine's vocabulary.
///
/// The platform layer produces these from whatever its window system reports;
/// nothing else is an input event. Window resizes and close requests are not
/// here on purpose — they are lifecycle, not input (input.md §4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputEvent {
    /// A key went down.
    KeyPressed(Key),
    /// A key came up.
    KeyReleased(Key),
    /// A pointer moved to a position, in pixels from the window's top-left.
    PointerMoved {
        /// Which pointer moved.
        id: PointerId,
        /// Where it moved to.
        screen: Vec2,
    },
    /// A pointer button went down.
    ButtonPressed {
        /// Which pointer.
        id: PointerId,
        /// Which button.
        button: PointerButton,
    },
    /// A pointer button came up.
    ButtonReleased {
        /// Which pointer.
        id: PointerId,
        /// Which button.
        button: PointerButton,
    },
    /// The wheel turned, in lines — normalized by the platform layer.
    Scrolled {
        /// Which pointer.
        id: PointerId,
        /// How far, in lines. Positive is down.
        lines: f32,
    },
    /// The window lost focus.
    FocusLost,
    /// The window got focus back.
    FocusGained,
}

/// Accumulates events between frames and hands out one snapshot per tick.
///
/// The platform layer owns one of these for the life of the window. Everything
/// about *when* input is observable flows from the two methods below: the
/// frame's events land on its first tick, and the catch-up ticks behind it see
/// state without edges.
pub struct SnapshotBuilder {
    /// Keys down right now, as events have left them. Canonical.
    held: Vec<Key>,
    /// Keys that went down during this frame. Canonical.
    pressed: Vec<Key>,
    /// Keys that came up during this frame. Canonical.
    released: Vec<Key>,
    /// Every pointer, sorted by id, with this frame's edges on it.
    pointers: Vec<PointerState>,
    window_focused: bool,
}

impl SnapshotBuilder {
    /// A builder with nothing pressed and the window focused.
    ///
    /// DELIBERATE: no `Default` impl, despite `clippy::new_without_default`
    /// (see ADR-0012) — one way to do everything, and `new` is that way.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            held: Vec::new(),
            pressed: Vec::new(),
            released: Vec::new(),
            pointers: vec![PointerState::new(PointerId::PRIMARY)],
            window_focused: true,
        }
    }

    /// Take note of one event.
    ///
    /// # Panics
    ///
    /// If a pointer moves to a non-finite position. A recording holding NaN
    /// cannot replay — NaN does not equal itself — so it is refused where it
    /// enters rather than where it later fails to compare.
    pub fn record(&mut self, event: InputEvent) {
        match event {
            InputEvent::KeyPressed(key) => {
                insert_sorted(&mut self.pressed, key);
                insert_sorted(&mut self.held, key);
            }
            InputEvent::KeyReleased(key) => {
                insert_sorted(&mut self.released, key);
                remove_sorted(&mut self.held, &key);
            }
            InputEvent::PointerMoved { id, screen } => {
                expect_finite(screen, id);
                pointer_mut(&mut self.pointers, id).screen = screen;
            }
            InputEvent::ButtonPressed { id, button } => {
                let pointer = pointer_mut(&mut self.pointers, id);
                insert_sorted(&mut pointer.pressed, button);
                insert_sorted(&mut pointer.held, button);
            }
            InputEvent::ButtonReleased { id, button } => {
                let pointer = pointer_mut(&mut self.pointers, id);
                insert_sorted(&mut pointer.released, button);
                remove_sorted(&mut pointer.held, &button);
            }
            InputEvent::Scrolled { id, lines } => {
                pointer_mut(&mut self.pointers, id).scroll += lines;
            }
            InputEvent::FocusLost => self.lose_focus(),
            InputEvent::FocusGained => self.window_focused = true,
        }
    }

    /// The snapshot for the first Update tick of this frame, consuming the
    /// frame's edges.
    ///
    /// CONTRACT: each physical event produces exactly one edge, on exactly one
    /// tick (input.md §2). Calling this twice without recording anything in
    /// between yields the edges once and then never again.
    pub fn first_tick_snapshot(&mut self) -> InputSnapshot {
        // A key tapped inside this frame is *held* for this one tick, even
        // though it is also released on it. Without that, a game could see
        // just_pressed(k) without held(k), which no game expects.
        let mut held = self.held.clone();
        for &key in &self.pressed {
            insert_sorted(&mut held, key);
        }
        let mut pointers = self.pointers.clone();
        for pointer in &mut pointers {
            for &button in &pointer.pressed {
                insert_sorted(&mut pointer.held, button);
            }
        }

        let snapshot = InputSnapshot {
            held,
            pressed: core::mem::take(&mut self.pressed),
            released: core::mem::take(&mut self.released),
            pointers,
            window_focused: self.window_focused,
        };

        // The builder keeps the state; only the edges and the scroll were the
        // frame's to spend.
        for pointer in &mut self.pointers {
            pointer.pressed.clear();
            pointer.released.clear();
            pointer.scroll = 0.0;
        }
        snapshot
    }

    /// The snapshot for a second or later Update tick of the same frame.
    ///
    /// CONTRACT: no edges, no scroll — only what the frame's events left
    /// standing. Three catch-up ticks must not fire one jump three times.
    #[must_use]
    pub fn catch_up_snapshot(&self) -> InputSnapshot {
        InputSnapshot {
            held: self.held.clone(),
            pressed: Vec::new(),
            released: Vec::new(),
            pointers: self
                .pointers
                .iter()
                .map(|pointer| PointerState {
                    id: pointer.id,
                    screen: pointer.screen,
                    scroll: 0.0,
                    held: pointer.held.clone(),
                    pressed: Vec::new(),
                    released: Vec::new(),
                })
                .collect(),
            window_focused: self.window_focused,
        }
    }

    /// Synthesize a release for everything still down, and mark the window
    /// unfocused.
    ///
    /// CONTRACT: the stuck-key-after-alt-tab bug is designed out here
    /// (input.md §4). The synthesized releases are recorded exactly like real
    /// ones, because replay does not care why a key came up.
    fn lose_focus(&mut self) {
        self.window_focused = false;
        for key in core::mem::take(&mut self.held) {
            insert_sorted(&mut self.released, key);
        }
        for pointer in &mut self.pointers {
            for button in core::mem::take(&mut pointer.held) {
                insert_sorted(&mut pointer.released, button);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builder_with(events: &[InputEvent]) -> SnapshotBuilder {
        let mut builder = SnapshotBuilder::new();
        for &event in events {
            builder.record(event);
        }
        builder
    }

    #[test]
    fn a_tap_inside_one_frame_reports_press_held_and_release_on_the_same_tick() {
        let mut builder = builder_with(&[
            InputEvent::KeyPressed(Key::Space),
            InputEvent::KeyReleased(Key::Space),
        ]);
        let snapshot = builder.first_tick_snapshot();
        assert_eq!(snapshot.pressed_keys(), [Key::Space]);
        assert_eq!(snapshot.released_keys(), [Key::Space]);
        assert_eq!(snapshot.held_keys(), [Key::Space], "held for its one tick");
    }

    #[test]
    fn a_tap_is_over_by_the_next_tick() {
        let mut builder = builder_with(&[
            InputEvent::KeyPressed(Key::Space),
            InputEvent::KeyReleased(Key::Space),
        ]);
        let _ = builder.first_tick_snapshot();
        let next = builder.catch_up_snapshot();
        assert!(next.held_keys().is_empty());
        assert!(next.pressed_keys().is_empty());
    }

    #[test]
    fn edges_are_spent_once_and_the_held_state_survives() {
        let mut builder = builder_with(&[InputEvent::KeyPressed(Key::D)]);
        let first = builder.first_tick_snapshot();
        assert_eq!(first.pressed_keys(), [Key::D]);

        let second = builder.first_tick_snapshot();
        assert!(second.pressed_keys().is_empty(), "the edge was spent");
        assert_eq!(second.held_keys(), [Key::D], "the key is still down");
    }

    #[test]
    fn a_catch_up_tick_keeps_the_held_state_and_drops_the_edges() {
        let mut builder = builder_with(&[
            InputEvent::KeyPressed(Key::D),
            InputEvent::Scrolled {
                id: PointerId::PRIMARY,
                lines: 3.0,
            },
        ]);
        let first = builder.first_tick_snapshot();
        assert_eq!(first.pointers()[0].scroll, 3.0);

        let catch_up = builder.catch_up_snapshot();
        assert_eq!(catch_up.held_keys(), [Key::D]);
        assert!(catch_up.pressed_keys().is_empty());
        assert_eq!(
            catch_up.pointers()[0].scroll,
            0.0,
            "scroll is not re-applied"
        );
    }

    #[test]
    fn scroll_within_a_frame_accumulates_and_then_resets() {
        let mut builder = builder_with(&[
            InputEvent::Scrolled {
                id: PointerId::PRIMARY,
                lines: 1.5,
            },
            InputEvent::Scrolled {
                id: PointerId::PRIMARY,
                lines: -0.5,
            },
        ]);
        assert_eq!(builder.first_tick_snapshot().pointers()[0].scroll, 1.0);
        assert_eq!(builder.first_tick_snapshot().pointers()[0].scroll, 0.0);
    }

    #[test]
    fn losing_focus_releases_everything_that_was_down() {
        // The alt-tab bug, designed out: without this, the player comes back to
        // a character still running left.
        let mut builder = builder_with(&[
            InputEvent::KeyPressed(Key::A),
            InputEvent::KeyPressed(Key::W),
            InputEvent::ButtonPressed {
                id: PointerId::PRIMARY,
                button: PointerButton::Primary,
            },
        ]);
        let _ = builder.first_tick_snapshot();
        builder.record(InputEvent::FocusLost);

        let snapshot = builder.first_tick_snapshot();
        assert_eq!(snapshot.released_keys(), [Key::A, Key::W]);
        assert!(snapshot.held_keys().is_empty());
        assert!(
            snapshot.pointers()[0].just_released(PointerButton::Primary),
            "the button too"
        );
        assert!(!snapshot.window_focused());
    }

    #[test]
    fn focus_returns_without_re_pressing_anything() {
        let mut builder = builder_with(&[InputEvent::KeyPressed(Key::A), InputEvent::FocusLost]);
        let _ = builder.first_tick_snapshot();
        builder.record(InputEvent::FocusGained);
        let snapshot = builder.first_tick_snapshot();
        assert!(snapshot.window_focused());
        assert!(
            snapshot.held_keys().is_empty(),
            "the player is not still holding a key we released for them"
        );
    }

    #[test]
    fn a_pointer_carries_its_position_between_frames() {
        let mut builder = builder_with(&[InputEvent::PointerMoved {
            id: PointerId::PRIMARY,
            screen: Vec2::new(400.0, 300.0),
        }]);
        let _ = builder.first_tick_snapshot();
        let next = builder.first_tick_snapshot();
        assert_eq!(next.pointers()[0].screen, Vec2::new(400.0, 300.0));
    }

    #[test]
    fn a_second_press_of_a_held_key_adds_no_second_edge() {
        // The documented boundary: a tick is the resolution of the recorded
        // timeline, and one tick cannot express two presses of one key. At
        // 60 Hz this is a double-tap inside 16 ms.
        let mut builder = builder_with(&[
            InputEvent::KeyPressed(Key::Space),
            InputEvent::KeyReleased(Key::Space),
            InputEvent::KeyPressed(Key::Space),
        ]);
        let snapshot = builder.first_tick_snapshot();
        assert_eq!(snapshot.pressed_keys(), [Key::Space]);
        assert_eq!(snapshot.released_keys(), [Key::Space]);
    }
}
