//! Pointers, plural: what a mouse and a finger have in common.
//!
//! Key types: `PointerId`, `PointerButton`, `PointerState`.
//! Depends on: `jidousha-core` for `Vec2`. Must never depend on: the renderer —
//! screen→world conversion goes through the camera, and the camera is not this
//! crate's business (input.md §3, conventions).
//! INVARIANT: a snapshot always carries the primary pointer, so `pointer()`
//! never has to answer "there isn't one". On a keyboard-only machine it sits at
//! the origin with nothing pressed.

use core::fmt;

use jidousha_core::math::Vec2;

/// Which pointer: the mouse, or one finger of several.
///
/// DELIBERATE: pointers are plural from the start (ADR-0005). Android is a
/// target, and a `Mouse` type would have to be unlearned the day touch lands.
/// Until then there is exactly one pointer and it is [`PointerId::PRIMARY`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PointerId(pub(crate) u32);

impl PointerId {
    /// The mouse, or the first finger down.
    pub const PRIMARY: PointerId = PointerId(0);

    /// The nth touch point. Unused until a touch platform lands.
    ///
    /// The index is a `u16` and the id a `u32` so that every touch index has
    /// its own id: saturating at the top would quietly alias two fingers.
    #[must_use]
    pub fn touch(index: u16) -> PointerId {
        PointerId(u32::from(index) + 1)
    }

    /// The id as written into recordings.
    #[must_use]
    pub fn code(self) -> u32 {
        self.0
    }

    /// The pointer a wire code names.
    #[must_use]
    pub fn from_code(code: u32) -> PointerId {
        PointerId(code)
    }
}

impl fmt::Display for PointerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            0 => formatter.write_str("primary"),
            index => write!(formatter, "touch {}", index - 1),
        }
    }
}

/// A pointer button.
///
/// Named by role, not by side: `Primary` is the left button on a right-handed
/// mouse and the right button on a left-handed one, which is what a game means
/// when it says "click".
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PointerButton {
    /// Click, tap, fire.
    Primary,
    /// The context-menu button.
    Secondary,
    /// The scroll wheel, pressed.
    Middle,
}

impl PointerButton {
    /// Every button, in declaration order.
    pub const ALL: &'static [PointerButton] = &[
        PointerButton::Primary,
        PointerButton::Secondary,
        PointerButton::Middle,
    ];

    /// This button's wire code, as written into recordings.
    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            PointerButton::Primary => 1,
            PointerButton::Secondary => 2,
            PointerButton::Middle => 3,
        }
    }

    /// The button a wire code names, or `None` if this build has never heard
    /// of it.
    #[must_use]
    pub fn find_by_code(code: u8) -> Option<PointerButton> {
        match code {
            1 => Some(PointerButton::Primary),
            2 => Some(PointerButton::Secondary),
            3 => Some(PointerButton::Middle),
            _ => None,
        }
    }

    /// The variant's name, for messages.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            PointerButton::Primary => "Primary",
            PointerButton::Secondary => "Secondary",
            PointerButton::Middle => "Middle",
        }
    }
}

impl fmt::Display for PointerButton {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// One pointer, for one tick.
///
/// The button lists follow the same rule the keyboard does: edges are recorded
/// data, not the difference between two ticks (input.md §2).
///
/// DELIBERATE: there is no `world` position here (see ADR-0017). Everything in
/// this struct is what the platform said; a world position is what a camera
/// makes of it, and the camera can change during the tick. Write
/// `camera.screen_to_world(input.pointer().screen)` at the point you know which
/// camera you mean.
#[derive(Clone, Debug, PartialEq)]
pub struct PointerState {
    /// Which pointer this is.
    pub id: PointerId,
    /// Where it is, in pixels from the window's top-left — the same
    /// orientation as world space, differing in units and camera offset
    /// (conventions).
    pub screen: Vec2,
    /// Scroll for this tick, in lines. The platform layer normalizes line-mode
    /// and pixel-mode deltas to this before the snapshot exists, so whatever it
    /// produced is what replays (input.md §3).
    pub scroll: f32,
    /// Buttons down for this tick. Canonical: sorted, no duplicates.
    pub(crate) held: Vec<PointerButton>,
    /// Buttons that went down this tick.
    pub(crate) pressed: Vec<PointerButton>,
    /// Buttons that came up this tick.
    pub(crate) released: Vec<PointerButton>,
}

impl PointerState {
    /// A pointer at the origin, touching nothing.
    #[must_use]
    pub fn new(id: PointerId) -> Self {
        Self {
            id,
            screen: Vec2::ZERO,
            scroll: 0.0,
            held: Vec::new(),
            pressed: Vec::new(),
            released: Vec::new(),
        }
    }

    /// Whether `button` is down this tick.
    #[must_use]
    pub fn held(&self, button: PointerButton) -> bool {
        self.held.contains(&button)
    }

    /// Whether `button` went down this tick.
    #[must_use]
    pub fn just_pressed(&self, button: PointerButton) -> bool {
        self.pressed.contains(&button)
    }

    /// Whether `button` came up this tick.
    #[must_use]
    pub fn just_released(&self, button: PointerButton) -> bool {
        self.released.contains(&button)
    }

    /// Every button down this tick, sorted.
    #[must_use]
    pub fn held_buttons(&self) -> &[PointerButton] {
        &self.held
    }

    /// Every button that went down this tick, sorted.
    #[must_use]
    pub fn pressed_buttons(&self) -> &[PointerButton] {
        &self.pressed
    }

    /// Every button that came up this tick, sorted.
    #[must_use]
    pub fn released_buttons(&self) -> &[PointerButton] {
        &self.released
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_button_round_trips_through_its_wire_code() {
        for &button in PointerButton::ALL {
            assert_eq!(PointerButton::find_by_code(button.code()), Some(button));
        }
    }

    #[test]
    fn zero_is_not_a_button_code() {
        // Zero is the obvious value for corrupt or truncated data to hold, so
        // it must not decode to a real button.
        assert_eq!(PointerButton::find_by_code(0), None);
    }

    #[test]
    fn a_fresh_pointer_is_at_the_origin_touching_nothing() {
        let pointer = PointerState::new(PointerId::PRIMARY);
        assert_eq!(pointer.screen, Vec2::ZERO);
        assert_eq!(pointer.scroll, 0.0);
        for &button in PointerButton::ALL {
            assert!(!pointer.held(button));
            assert!(!pointer.just_pressed(button));
            assert!(!pointer.just_released(button));
        }
    }

    #[test]
    fn pointers_print_as_the_thing_a_person_would_call_them() {
        assert_eq!(PointerId::PRIMARY.to_string(), "primary");
        assert_eq!(PointerId::touch(0).to_string(), "touch 0");
        assert_eq!(PointerId::touch(3).to_string(), "touch 3");
    }
}
