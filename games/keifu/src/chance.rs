//! The randomness primitives of SPEC §22.1, on the engine's deterministic `Rng`.
//!
//! One generator for all gameplay — the world's `Rng` resource, seeded from the
//! run's recorded seed — and these are the only ways the game draws from it.
//! Every primitive is written from `u`, a uniform draw on [0, 1), exactly as the
//! spec states it, so the distributions are the original's.

use jidousha::prelude::Rng;

/// `u`: uniform on [0, 1).
fn unit(rng: &mut Rng) -> f32 {
    // INVARIANT: `next_f32` is documented as `0.0..1.0`; `between` clamps as well,
    // so a value at 1.0 could never index past the end.
    rng.next_f32()
}

/// `between(lo, hi)` = lo + floor(u * (hi - lo + 1)): a uniform integer in lo..=hi.
pub fn between(rng: &mut Rng, lo: i32, hi: i32) -> i32 {
    debug_assert!(lo <= hi, "between({lo}, {hi}) is an empty range");
    let span = (hi - lo + 1) as f32;
    (lo + (unit(rng) * span).floor() as i32).min(hi)
}

/// `chance(p)` = u < p.
pub fn chance(rng: &mut Rng, p: f32) -> bool {
    unit(rng) < p
}

/// `index(n)` = between(0, n - 1).
pub fn index(rng: &mut Rng, n: usize) -> usize {
    assert!(n > 0, "[keifu] index(0): there is nothing to pick from");
    between(rng, 0, n as i32 - 1) as usize
}

/// `fresh_index(n, prev)`: uniform over {0..n-1} without `prev`.
///
/// Two draws when the first hits `prev`, as the spec says: the second is uniform
/// over the n - 1 others, so each of them is reached with probability
/// 1/n + (1/n)(1/(n-1)) = 1/(n-1).
pub fn fresh_index(rng: &mut Rng, n: usize, prev: usize) -> usize {
    assert!(
        n > 1,
        "[keifu] fresh_index({n}, {prev}): no other index exists"
    );
    let first = index(rng, n);
    if first != prev {
        return first;
    }
    let second = index(rng, n - 1);
    if second >= prev { second + 1 } else { second }
}

/// `unclaimed_index(claimed)`: uniform over the unclaimed, else uniform over all.
pub fn unclaimed_index(rng: &mut Rng, claimed: &[bool]) -> usize {
    let open: Vec<usize> = (0..claimed.len()).filter(|&i| !claimed[i]).collect();
    if open.is_empty() {
        index(rng, claimed.len())
    } else {
        open[index(rng, open.len())]
    }
}

/// A bag drawn without replacement, refilled when empty (SPEC §17.1, §22.1).
#[derive(Clone, Debug, PartialEq)]
pub struct Bag<T: Clone> {
    full: Vec<T>,
    left: Vec<T>,
}

impl<T: Clone> Bag<T> {
    /// A full bag of `items`.
    pub fn new(items: Vec<T>) -> Self {
        Self {
            left: items.clone(),
            full: items,
        }
    }

    /// Draw one: a uniform index into what is left, removed. An empty bag refills first.
    pub fn draw(&mut self, rng: &mut Rng) -> T {
        if self.left.is_empty() {
            self.left = self.full.clone();
        }
        let at = index(rng, self.left.len());
        self.left.remove(at)
    }

    /// How many are left before the bag refills.
    pub fn left(&self) -> usize {
        self.left.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRAWS: usize = 60_000;

    #[test]
    fn between_is_uniform_over_its_closed_range() {
        let mut rng = Rng::from_seed(7);
        let mut counts = [0usize; 3];
        for _ in 0..DRAWS {
            let value = between(&mut rng, -1, 1);
            assert!((-1..=1).contains(&value), "{value} outside -1..=1");
            counts[(value + 1) as usize] += 1;
        }
        for count in counts {
            let share = count as f32 / DRAWS as f32;
            assert!(
                (share - 1.0 / 3.0).abs() < 0.01,
                "share {share:.4} of {counts:?}"
            );
        }
    }

    #[test]
    fn fresh_index_never_repeats_and_is_uniform_over_the_others() {
        let mut rng = Rng::from_seed(11);
        let mut counts = [0usize; 4];
        for _ in 0..DRAWS {
            let value = fresh_index(&mut rng, 4, 2);
            counts[value] += 1;
        }
        assert_eq!(counts[2], 0, "fresh_index returned the previous index");
        for (index, count) in counts.iter().enumerate().filter(|(i, _)| *i != 2) {
            let share = *count as f32 / DRAWS as f32;
            assert!(
                (share - 1.0 / 3.0).abs() < 0.01,
                "index {index} share {share:.4}"
            );
        }
    }

    #[test]
    fn unclaimed_index_picks_only_unclaimed_and_falls_back_to_all() {
        let mut rng = Rng::from_seed(3);
        for _ in 0..1000 {
            let value = unclaimed_index(&mut rng, &[true, false, true, false]);
            assert!(value == 1 || value == 3, "picked claimed index {value}");
        }
        let mut seen = [false; 3];
        for _ in 0..1000 {
            seen[unclaimed_index(&mut rng, &[true, true, true])] = true;
        }
        assert_eq!(
            seen, [true; 3],
            "an all-claimed pick is not uniform over all"
        );
    }

    #[test]
    fn a_bag_gives_every_item_once_before_it_refills() {
        let mut rng = Rng::from_seed(5);
        let mut bag = Bag::new(vec![1, 2, 3, 4, 5]);
        let mut first: Vec<i32> = (0..5).map(|_| bag.draw(&mut rng)).collect();
        first.sort_unstable();
        assert_eq!(first, [1, 2, 3, 4, 5]);
        assert_eq!(bag.left(), 0);
        let _ = bag.draw(&mut rng);
        assert_eq!(bag.left(), 4, "an empty bag did not refill before drawing");
    }

    #[test]
    fn chance_holds_its_probability() {
        let mut rng = Rng::from_seed(9);
        let hits = (0..DRAWS).filter(|_| chance(&mut rng, 0.15)).count();
        let share = hits as f32 / DRAWS as f32;
        assert!((share - 0.15).abs() < 0.01, "share {share:.4}");
    }
}
