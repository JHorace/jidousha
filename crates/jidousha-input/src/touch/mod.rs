//! Fingers: the bounded list a snapshot carries, and the slots it comes from.
//!
//! Key types: `FingerId`, `TouchId`, `TouchPhase`, `Touch`, `TouchList`.
//! `TouchTracker` — which finger is in which slot — is in `tracker`.
//! Depends on: `jidousha-core` for `Vec2`. Must never depend on: `winit` or
//! `web-sys` — a platform's finger identifier arrives as an opaque
//! [`FingerId`] and is turned into a slot here, above the seam, where it is
//! testable on a machine with no touchscreen (ADR-0004, input.md §3a).
//! INVARIANT: the list is a fixed structure of at most [`MAX_TOUCHES`], never a
//! heap allocation that grows with how many fingers a table can fit. A snapshot
//! is written to disk sixty times a second; its size is part of the format.
//! INVARIANT: touches are canonical — sorted by slot, no duplicates — for the
//! same reason the key list is: two snapshots meaning the same input *are*
//! equal, and equal snapshots encode to equal bytes.

use core::fmt;

use jidousha_core::math::Vec2;

/// How many touches a snapshot carries.
///
/// Four, because that is what a game reads and what a recording pays for: two
/// thumbs and two more fingers covers every interaction a 2D game has asked
/// for, and the fifth finger of a hand flat on a tablet is noise the format
/// would carry forever. A touch beyond the fourth is dropped at the builder —
/// a documented boundary, the same shape as a key the `Key` enum does not name
/// (input.md §3a).
pub const MAX_TOUCHES: usize = 4;

/// A platform's name for one finger.
///
/// Opaque on purpose: winit counts fingers in `u64`s and a browser in `i32`s,
/// and neither number means anything to a game. It exists so the engine can
/// tell one finger from another between events, and it never reaches a
/// snapshot — what a snapshot carries is the [`TouchId`] slot this is mapped
/// to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FingerId(u64);

impl FingerId {
    /// The finger a platform's identifier names.
    ///
    /// The only constructor: the platform layer calls it, and a check driving
    /// synthetic touches calls it with numbers of its own choosing. Any two
    /// distinct values are two distinct fingers and nothing more is promised.
    #[must_use]
    pub fn from_platform(id: u64) -> FingerId {
        FingerId(id)
    }
}

impl fmt::Display for FingerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "finger {}", self.0)
    }
}

/// Which of the snapshot's touch slots a touch occupies.
///
/// Stable for the life of one touch: the slot a finger is given when it lands
/// is the slot it reports in until it lifts, so a game can follow one finger
/// across ticks by its id. Reused afterwards — slot 0 is whatever is in slot 0
/// now, not "the first finger of the session".
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TouchId(u8);

impl TouchId {
    /// Which slot, `0..`[`MAX_TOUCHES`].
    #[must_use]
    pub fn slot(self) -> u8 {
        self.0
    }

    /// The slot a wire value names, or `None` if this build has no such slot.
    pub(crate) fn find_by_slot(slot: u8) -> Option<TouchId> {
        (usize::from(slot) < MAX_TOUCHES).then_some(TouchId(slot))
    }
}

impl fmt::Display for TouchId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "touch {}", self.0)
    }
}

/// What happened to a touch on this tick.
///
/// DELIBERATE: there is no fifth phase for "down and unchanged". A finger
/// resting still reports [`TouchPhase::Moved`], because the distinction
/// between a finger that moved a pixel and one that moved none is one no game
/// has asked for, and a phase nobody reads is a phase in every recording
/// forever (input.md §3a).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TouchPhase {
    /// The finger landed this tick.
    Began,
    /// The finger is down — moved, or simply still there.
    Moved,
    /// The finger lifted this tick.
    Ended,
    /// The system took the touch away this tick: a notification shade, a
    /// gesture the browser claimed, the window losing focus. Not a lift, and
    /// worth telling apart — a cancelled drag should be undone, not committed.
    Cancelled,
}

impl TouchPhase {
    /// Every phase, in declaration order.
    pub const ALL: &'static [TouchPhase] = &[
        TouchPhase::Began,
        TouchPhase::Moved,
        TouchPhase::Ended,
        TouchPhase::Cancelled,
    ];

    /// Whether this phase ends the touch: it is the last tick the finger
    /// appears on.
    #[must_use]
    pub fn is_final(self) -> bool {
        matches!(self, TouchPhase::Ended | TouchPhase::Cancelled)
    }

    /// This phase's wire code, as written into recordings.
    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            TouchPhase::Began => 1,
            TouchPhase::Moved => 2,
            TouchPhase::Ended => 3,
            TouchPhase::Cancelled => 4,
        }
    }

    /// The phase a wire code names, or `None` if this build has never heard of
    /// it.
    #[must_use]
    pub fn find_by_code(code: u8) -> Option<TouchPhase> {
        match code {
            1 => Some(TouchPhase::Began),
            2 => Some(TouchPhase::Moved),
            3 => Some(TouchPhase::Ended),
            4 => Some(TouchPhase::Cancelled),
            _ => None,
        }
    }

    /// The variant's name, for messages.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            TouchPhase::Began => "Began",
            TouchPhase::Moved => "Moved",
            TouchPhase::Ended => "Ended",
            TouchPhase::Cancelled => "Cancelled",
        }
    }
}

impl fmt::Display for TouchPhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// One finger, for one tick.
///
/// DELIBERATE: no force, no radius, no world position. Force is reported by
/// two of the four platforms and normalized differently by both; a world
/// position is what a camera makes of a screen one and is derived at the point
/// of use (ADR-0017, and the same argument). What is here is what every
/// platform agrees on and every game needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Touch {
    /// Which slot, stable for the life of this touch.
    pub id: TouchId,
    /// What happened to it this tick.
    pub phase: TouchPhase,
    /// Where it is, in pixels from the surface's top-left — the same space
    /// [`PointerState::screen`](crate::PointerState::screen) is in, so
    /// `camera.screen_to_world` converts either one (input.md §3).
    pub screen: Vec2,
}

/// The touches of one tick: at most [`MAX_TOUCHES`], in slot order.
///
/// A fixed array and a count rather than a `Vec`. The bound is the contract —
/// a snapshot is a small value that gets written to disk every tick — and a
/// list that could not overflow is a list nothing has to check.
///
/// `PartialEq` and `Debug` are written rather than derived, and both read only
/// `entries[..len]`. A derived `PartialEq` would compare the padding as well,
/// which would make two lists meaning the same touches unequal the moment
/// anything ever left a used slot behind — and "two snapshots meaning the same
/// input *are* equal" is this crate's INVARIANT, not a coincidence to maintain
/// by hand.
#[derive(Clone, Copy)]
pub(crate) struct TouchList {
    entries: [Touch; MAX_TOUCHES],
    len: u8,
}

impl PartialEq for TouchList {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl fmt::Debug for TouchList {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.as_slice()).finish()
    }
}

/// What an unused slot holds. Never observable: only `entries[..len]` is ever
/// read, and the fill exists because an array has to be made of something.
const UNUSED: Touch = Touch {
    id: TouchId(0),
    phase: TouchPhase::Cancelled,
    screen: Vec2::ZERO,
};

impl TouchList {
    /// A tick with no fingers on the glass.
    pub(crate) const fn new() -> Self {
        Self {
            entries: [UNUSED; MAX_TOUCHES],
            len: 0,
        }
    }

    /// Add a touch after the ones already here.
    ///
    /// Returns `false` if the list is full, which is how the decoder refuses a
    /// file claiming more fingers than the format has room for. Callers that
    /// build a list from slots in order cannot hit it: there are exactly
    /// [`MAX_TOUCHES`] slots.
    pub(crate) fn push(&mut self, touch: Touch) -> bool {
        let at = usize::from(self.len);
        if at >= MAX_TOUCHES {
            return false;
        }
        self.entries[at] = touch;
        self.len += 1;
        true
    }

    /// Every touch this tick, in slot order.
    pub(crate) fn as_slice(&self) -> &[Touch] {
        &self.entries[..usize::from(self.len)]
    }
}

mod tracker;

pub(crate) use tracker::TouchTracker;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_phase_round_trips_through_its_wire_code() {
        for &phase in TouchPhase::ALL {
            assert_eq!(TouchPhase::find_by_code(phase.code()), Some(phase));
        }
    }

    #[test]
    fn zero_is_not_a_phase_code() {
        // Zero is what corrupt or truncated bytes hold, so it must not decode
        // to a real phase.
        assert_eq!(TouchPhase::find_by_code(0), None);
    }

    #[test]
    fn only_the_slots_the_format_has_are_slots() {
        for slot in 0..MAX_TOUCHES {
            let slot = u8::try_from(slot).expect("four fits in a byte");
            assert_eq!(TouchId::find_by_slot(slot).map(TouchId::slot), Some(slot));
        }
        assert_eq!(TouchId::find_by_slot(4), None);
        assert_eq!(TouchId::find_by_slot(255), None);
    }

    #[test]
    fn a_touch_list_holds_four_and_refuses_the_fifth() {
        // The bound is the contract: a snapshot is a fixed structure, and the
        // decoder leans on this to refuse a file claiming more.
        let mut list = TouchList::new();
        for slot in 0..5u8 {
            let pushed = list.push(Touch {
                id: TouchId(slot.min(3)),
                phase: TouchPhase::Moved,
                screen: Vec2::ZERO,
            });
            assert_eq!(pushed, usize::from(slot) < MAX_TOUCHES, "slot {slot}");
        }
        assert_eq!(list.as_slice().len(), MAX_TOUCHES);
    }

    #[test]
    fn two_lists_of_the_same_touches_are_equal_whatever_is_behind_them() {
        // The padding is not the value. Written as a test because the derived
        // `PartialEq` this replaces was correct only for as long as nothing
        // ever shortened a list.
        let touch = Touch {
            id: TouchId(0),
            phase: TouchPhase::Began,
            screen: Vec2::new(1.0, 2.0),
        };
        let mut one = TouchList::new();
        one.push(touch);
        let mut two = TouchList::new();
        two.push(touch);
        two.entries[1] = Touch {
            id: TouchId(3),
            phase: TouchPhase::Ended,
            screen: Vec2::new(9.0, 9.0),
        };
        assert_eq!(one, two, "only the touches count");
        assert_eq!(
            format!("{one:?}"),
            format!("{two:?}"),
            "and only they print"
        );
    }

    #[test]
    fn phases_print_as_the_thing_a_person_would_call_them() {
        assert_eq!(TouchPhase::Began.to_string(), "Began");
        assert_eq!(TouchId(2).to_string(), "touch 2");
        assert_eq!(FingerId::from_platform(9).to_string(), "finger 9");
    }

    #[test]
    fn only_ended_and_cancelled_end_a_touch() {
        let ends: Vec<TouchPhase> = TouchPhase::ALL
            .iter()
            .copied()
            .filter(|phase| phase.is_final())
            .collect();
        assert_eq!(ends, vec![TouchPhase::Ended, TouchPhase::Cancelled]);
    }
}
