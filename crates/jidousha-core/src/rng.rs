//! The engine's only source of randomness: a seeded PCG32 generator.
//!
//! Key types: `Rng`.
//! Depends on: `resource`. Must never depend on: OS entropy, the system clock,
//! or the `rand` crate.
//! INVARIANT: every value is a pure function of the seed and the number of
//! draws taken. Same seed, same sequence, on every platform and every run —
//! integer arithmetic only, so there is no float rounding to disagree about
//! (core.md §6 CONTRACT).

use crate::resource::Resource;

/// PCG32's multiplier, from the reference implementation.
const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

/// The simulation's random number generator, held as a world resource.
///
/// CONTRACT: simulation code draws randomness from here and nowhere else.
/// `rand::thread_rng`, `SystemTime`, and any other ambient entropy would make
/// a run unrepeatable, which breaks replay, verification, and every golden
/// test the engine's testing story rests on (core.md §6).
///
/// ```
/// use jidousha_core::Rng;
///
/// let mut rng = Rng::from_seed(42);
/// let first: Vec<u32> = (0..4).map(|_| rng.below(100)).collect();
///
/// // The same seed replays the same numbers, always.
/// let mut again = Rng::from_seed(42);
/// let second: Vec<u32> = (0..4).map(|_| again.below(100)).collect();
/// assert_eq!(first, second);
/// ```
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
    /// Odd by construction, which is what keeps the period full.
    increment: u64,
}

impl Resource for Rng {}

impl Rng {
    /// Create a generator from a seed.
    ///
    /// Every seed gives a different sequence; the same seed always gives the
    /// same one.
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        // The reference PCG32 seeding routine: an odd increment, then two
        // steps to mix the seed through the state.
        let mut rng = Self {
            state: 0,
            increment: (seed << 1) | 1,
        };
        rng.step();
        rng.state = rng.state.wrapping_add(seed);
        rng.step();
        rng
    }

    /// The next value in the sequence.
    pub fn next_u32(&mut self) -> u32 {
        let previous = self.state;
        self.step();
        // PCG32 XSH-RR: xorshift the high bits down, then rotate by the top
        // five bits of the state the output was drawn from.
        let xorshifted = (((previous >> 18) ^ previous) >> 27) as u32;
        let rotation = (previous >> 59) as u32;
        xorshifted.rotate_right(rotation)
    }

    /// A value in `0..limit`, with every value equally likely.
    ///
    /// # Panics
    ///
    /// If `limit` is zero — there is no value to return, and silently
    /// answering zero would hide the mistake.
    pub fn below(&mut self, limit: u32) -> u32 {
        assert!(
            limit > 0,
            "[jidousha] Rng::below(0) has no value to return\n  \
             the range 0..0 is empty\n  \
             likely cause: a count that can be zero was passed without checking\n  \
             fix: guard the call with `if limit > 0`, or use next_u32 if any value will do"
        );
        // Rejection sampling: modulo alone would favour the low values when
        // `limit` does not divide the u32 range. Deterministic either way, but
        // a biased die is a bug a game would eventually notice.
        let zone = u32::MAX - (u32::MAX % limit) - (limit - 1);
        loop {
            let draw = self.next_u32();
            if draw <= zone {
                return draw % limit;
            }
        }
    }

    /// A value in `0.0..1.0`.
    ///
    /// **Half-open, and which end is which matters.** `0.0` is drawn; `1.0`
    /// never is, because the top 24 bits give at most `2^24 - 1` twenty-fourths.
    /// A roll used as a probability against `<`, as a divisor, or scaled into an
    /// index each break at a different end, so the range is stated rather than
    /// left to the `..` (E0 run 10 assumed it and was right).
    ///
    /// Built from the top 24 bits, so every value is exactly representable and
    /// the result is bit-identical on every platform.
    pub fn next_f32(&mut self) -> f32 {
        const SCALE: f32 = 1.0 / (1u32 << 24) as f32;
        (self.next_u32() >> 8) as f32 * SCALE
    }

    fn step(&mut self) {
        self.state = self
            .state
            .wrapping_mul(MULTIPLIER)
            .wrapping_add(self.increment);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_replays_the_same_sequence() {
        let draws = |seed| -> Vec<u32> {
            let mut rng = Rng::from_seed(seed);
            (0..32).map(|_| rng.next_u32()).collect()
        };
        assert_eq!(draws(7), draws(7));
    }

    #[test]
    fn different_seeds_give_different_sequences() {
        let draws = |seed| -> Vec<u32> {
            let mut rng = Rng::from_seed(seed);
            (0..32).map(|_| rng.next_u32()).collect()
        };
        assert_ne!(draws(7), draws(8));
    }

    #[test]
    fn below_stays_inside_the_range() {
        let mut rng = Rng::from_seed(1);
        for _ in 0..1000 {
            assert!(rng.below(6) < 6);
        }
    }

    #[test]
    fn below_one_is_always_zero() {
        let mut rng = Rng::from_seed(1);
        assert_eq!(rng.below(1), 0);
    }

    #[test]
    fn next_f32_draws_zero_and_never_draws_one() {
        // The range is half-open and the documentation now says which end is
        // which; this is what holds it to that (e0-findings.md F-133).
        let mut rng = Rng::from_seed(11);
        let mut lowest = f32::MAX;
        let mut highest = f32::MIN;
        for _ in 0..100_000 {
            let roll = rng.next_f32();
            assert!((0.0..1.0).contains(&roll), "{roll} is outside 0.0..1.0");
            lowest = lowest.min(roll);
            highest = highest.max(roll);
        }
        // The largest representable draw, which is what "never 1.0" means here.
        assert!(highest <= 1.0 - 1.0 / (1u32 << 24) as f32, "{highest}");
        assert!(lowest < 0.001, "{lowest} — the low end should be reachable");
    }

    #[test]
    fn below_reaches_every_value_in_its_range() {
        let mut rng = Rng::from_seed(3);
        let mut seen = [false; 6];
        for _ in 0..1000 {
            seen[rng.below(6) as usize] = true;
        }
        assert!(seen.iter().all(|hit| *hit), "{seen:?}");
    }

    #[test]
    fn next_f32_stays_in_the_unit_interval() {
        let mut rng = Rng::from_seed(9);
        for _ in 0..1000 {
            let value = rng.next_f32();
            assert!((0.0..1.0).contains(&value), "{value}");
        }
    }

    #[test]
    #[should_panic(expected = "has no value to return")]
    fn below_zero_panics_rather_than_inventing_a_value() {
        let mut rng = Rng::from_seed(1);
        let _ = rng.below(0);
    }
}
