//! The forecast (SPEC §6): the 36 dice pairs, the four bands, and the percentages
//! the card and sheet print.
//!
//! `outcome` is the one banding of a margin. The forecast enumerates every pair of
//! dice through it, and W6's roll will throw two dice and ask the same `margin` and
//! `outcome` — so the odds the player reads are counted from the numbers the roll
//! uses, not from a second copy of CONSTANTS §3's table. The table itself is
//! asserted against this module, entry by entry, as shipped literals (`w4.rs`).

use crate::constants::{DICE_MIDPOINT, DICE_SIDES, SETBACK_MARGIN, TRIUMPH_MARGIN};
use crate::ids::Outcome;

/// Every pair of dice (CONSTANTS §3 `DICE_OUTCOMES`).
pub const DICE_OUTCOMES: i32 = DICE_SIDES * DICE_SIDES;

/// `margin = power + d1 + d2 - 7 - demand` (CONSTANTS §3).
pub fn margin(power: i32, d1: i32, d2: i32, demand: i32) -> i32 {
    power + d1 + d2 - DICE_MIDPOINT - demand
}

/// The band a margin falls in: TRIUMPH >= 4, SUCCESS 0..3, SETBACK -1..-4,
/// DISASTER <= -5 (CONSTANTS §3).
pub fn outcome(margin: i32) -> Outcome {
    if margin >= TRIUMPH_MARGIN {
        Outcome::Triumph
    } else if margin >= 0 {
        Outcome::Success
    } else if margin >= -SETBACK_MARGIN {
        Outcome::Setback
    } else {
        Outcome::Disaster
    }
}

/// How many of the 36 dice pairs fall in each band, by `Outcome` (disaster, setback,
/// success, triumph). An empty party has all chances 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Forecast {
    /// Pairs per outcome, indexed by `Outcome::index`.
    pub ways: [i32; 4],
}

impl Forecast {
    /// The chance of exactly `outcome`, in pairs of 36.
    pub fn ways(&self, outcome: Outcome) -> i32 {
        self.ways[outcome.index()]
    }
}

/// The forecast for `power` against `demand`; all zeros when nobody is seated.
pub fn forecast(power: i32, demand: i32, anyone: bool) -> Forecast {
    let mut out = Forecast::default();
    if !anyone {
        return out;
    }
    for d1 in 1..=DICE_SIDES {
        for d2 in 1..=DICE_SIDES {
            out.ways[outcome(margin(power, d1, d2, demand)).index()] += 1;
        }
    }
    out
}

/// `percent(x) = int(x * 100 + 0.5)` for x = `ways`/36 (CONSTANTS §3).
///
/// In integers: k/36 * 100 is never exactly half way between two whole numbers
/// (that would need 50k to be an odd multiple of 9), so float rounding in the
/// original cannot differ from this.
pub fn percent(ways: i32) -> i32 {
    (ways * 100 + DICE_OUTCOMES / 2) / DICE_OUTCOMES
}

/// The card's four percentages: "Succeed" is success or better (CONSTANTS §3's
/// "as shown" column), "triumph" triumph alone, "Setback" setback or worse and
/// "disaster" disaster alone (SPEC-GAPS KG-25).
pub fn card_percentages(forecast: &Forecast) -> [i32; 4] {
    let [disaster, setback, success, triumph] = forecast.ways;
    [
        percent(success + triumph),
        percent(triumph),
        percent(setback + disaster),
        percent(disaster),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bands_meet_at_four_zero_minus_one_and_minus_five() {
        assert_eq!(outcome(4), Outcome::Triumph);
        assert_eq!(outcome(3), Outcome::Success);
        assert_eq!(outcome(0), Outcome::Success);
        assert_eq!(outcome(-1), Outcome::Setback);
        assert_eq!(outcome(-4), Outcome::Setback);
        assert_eq!(outcome(-5), Outcome::Disaster);
    }

    #[test]
    fn the_forecast_reproduces_constants_three_at_every_margin() {
        // CONSTANTS §3, "Outcome odds by power minus demand": disaster, setback,
        // success, triumph, in 36ths, and the success-or-better percentage shown.
        let table: [(i32, [i32; 4], i32); 19] = [
            (-9, [35, 1, 0, 0], 0),
            (-8, [33, 3, 0, 0], 0),
            (-7, [30, 6, 0, 0], 0),
            (-6, [26, 10, 0, 0], 0),
            (-5, [21, 14, 1, 0], 3),
            (-4, [15, 18, 3, 0], 8),
            (-3, [10, 20, 6, 0], 17),
            (-2, [6, 20, 10, 0], 28),
            (-1, [3, 18, 14, 1], 42),
            (0, [1, 14, 18, 3], 58),
            (1, [0, 10, 20, 6], 72),
            (2, [0, 6, 20, 10], 83),
            (3, [0, 3, 18, 15], 92),
            (4, [0, 1, 14, 21], 97),
            (5, [0, 0, 10, 26], 100),
            (6, [0, 0, 6, 30], 100),
            (7, [0, 0, 3, 33], 100),
            (8, [0, 0, 1, 35], 100),
            (9, [0, 0, 0, 36], 100),
        ];
        for (gap, ways, shown) in table {
            let f = forecast(20 + gap, 20, true);
            assert_eq!(f.ways, ways, "power - demand {gap}");
            assert_eq!(card_percentages(&f)[0], shown, "power - demand {gap}");
        }
        assert_eq!(forecast(0, 10, true).ways, [36, 0, 0, 0], "<= -10");
        assert_eq!(forecast(30, 10, true).ways, [0, 0, 0, 36], ">= +9");
    }

    #[test]
    fn an_empty_party_forecasts_nothing() {
        assert_eq!(forecast(30, 0, false).ways, [0; 4]);
    }

    #[test]
    fn integer_percent_is_the_originals_float_rounding_for_every_k() {
        for k in 0..=36 {
            let float = ((k as f64 / 36.0) * 100.0 + 0.5) as i32;
            assert_eq!(percent(k), float, "k = {k}");
        }
    }
}
