//! Units that live in types rather than in comments (conventions, practices §1.3).
//!
//! Key types: `Seconds`.
//! Depends on: nothing. Must never depend on: anything — these are the
//! vocabulary every other module speaks in.
//! INVARIANT: a duration in a public API is `Seconds`, never a bare `f32` and
//! never milliseconds (conventions §Time).

use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

/// A duration, in seconds.
///
/// ```
/// use jidousha_core::Seconds;
///
/// let frame = Seconds(1.0 / 60.0);
/// let two_frames = frame + frame;
/// assert!(two_frames > frame);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Seconds(pub f32);

impl Seconds {
    /// No time at all.
    pub const ZERO: Seconds = Seconds(0.0);

    /// The underlying value, for arithmetic the newtype does not cover.
    ///
    /// Prefer the operators where they suffice: the point of the newtype is
    /// that seconds do not silently become milliseconds.
    #[must_use]
    pub fn as_f32(self) -> f32 {
        self.0
    }
}

impl fmt::Display for Seconds {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}s", self.0)
    }
}

impl Add for Seconds {
    type Output = Seconds;

    fn add(self, other: Seconds) -> Seconds {
        Seconds(self.0 + other.0)
    }
}

impl AddAssign for Seconds {
    fn add_assign(&mut self, other: Seconds) {
        self.0 += other.0;
    }
}

impl Sub for Seconds {
    type Output = Seconds;

    fn sub(self, other: Seconds) -> Seconds {
        Seconds(self.0 - other.0)
    }
}

impl SubAssign for Seconds {
    fn sub_assign(&mut self, other: Seconds) {
        self.0 -= other.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_add_and_subtract_as_durations() {
        let mut total = Seconds(0.5);
        total += Seconds(0.25);
        assert_eq!(total, Seconds(0.75));
        total -= Seconds(0.25);
        assert_eq!(total, Seconds(0.5));
    }

    #[test]
    fn seconds_display_carries_its_unit() {
        assert_eq!(Seconds(0.25).to_string(), "0.25s");
    }
}
