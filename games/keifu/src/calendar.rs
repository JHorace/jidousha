//! The year and season counter, and the Door countdown (SPEC §2.2).
//!
//! The calendar holds a 0-based year index and a month that is the season. The
//! game is only ever in SUMMER or WINTER. The final day is 25 years after the
//! start, so `years_until_door` is 25 in year 1's summer and 0 in the last summer.

use crate::constants::{DOOR_YEARS, MONTHS_PER_YEAR, SUMMER, WINTER};

/// Where the run is in time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calendar {
    /// 0-based year index.
    pub year_index: i32,
    /// The calendar month, 0..4; only SUMMER (1) and WINTER (3) are visited.
    pub month: usize,
}

impl Calendar {
    /// The start of a run: year index 0, summer.
    pub fn start() -> Self {
        Self {
            year_index: 0,
            month: SUMMER,
        }
    }

    /// `current_year` = year index + 1, so the first summer is year 1.
    pub fn current_year(self) -> i32 {
        self.year_index + 1
    }

    /// `final.year - today.year`: 25 in year 1, 0 in the last summer.
    pub fn years_until_door(self) -> i32 {
        DOOR_YEARS - self.year_index
    }

    /// The Door stands open once the countdown reaches 0.
    pub fn door_stands_open(self) -> bool {
        self.years_until_door() <= 0
    }

    /// Whether this is the 26th summer, the Door's.
    pub fn is_last_summer(self) -> bool {
        self.month == SUMMER && self.door_stands_open()
    }

    /// Whether it is winter: the hearth, and the turning that follows it.
    pub fn is_winter(self) -> bool {
        self.month == WINTER
    }

    /// Begin winter: same year, month WINTER.
    pub fn begin_winter(&mut self) {
        self.month = WINTER;
    }

    /// Begin summer: month SUMMER, next year.
    pub fn begin_summer(&mut self) {
        self.month = SUMMER;
        self.year_index += 1;
    }

    /// The season's index into `lore.seasons`.
    pub fn season(self) -> usize {
        debug_assert!(self.month < MONTHS_PER_YEAR);
        self.month
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_summer_is_year_one_with_twenty_five_years_until_the_door() {
        let calendar = Calendar::start();
        assert_eq!(calendar.current_year(), 1);
        assert_eq!(calendar.season(), 1);
        assert_eq!(calendar.years_until_door(), 25);
        assert!(!calendar.door_stands_open());
    }

    #[test]
    fn the_twenty_sixth_summer_is_the_last_and_the_door_stands_open() {
        let mut calendar = Calendar::start();
        for _ in 0..25 {
            calendar.begin_winter();
            assert_eq!(calendar.season(), 3);
            calendar.begin_summer();
        }
        assert_eq!(calendar.current_year(), 26);
        assert_eq!(calendar.years_until_door(), 0);
        assert!(calendar.is_last_summer());
    }

    #[test]
    fn winter_keeps_the_year_and_summer_advances_it() {
        let mut calendar = Calendar::start();
        calendar.begin_winter();
        assert_eq!(calendar.current_year(), 1);
        assert_eq!(calendar.years_until_door(), 25);
        calendar.begin_summer();
        assert_eq!(calendar.current_year(), 2);
        assert_eq!(calendar.years_until_door(), 24);
    }
}
