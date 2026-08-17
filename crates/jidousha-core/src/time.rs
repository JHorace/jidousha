//! The simulation clock: the only clock simulation code may observe.
//!
//! Key types: `Time`.
//! Depends on: `resource`, `units`. Must never depend on: `std::time` —
//! wall-clock types are banned outside `jidousha-platform` (ADR-0005), and real
//! frame time enters the engine only as the argument to
//! [`Simulation::advance`](crate::Simulation::advance).
//! INVARIANT: `elapsed` is exactly `tick * fixed_dt`, and `fixed_dt` never
//! changes during a run — a tick is the same length at the start of a game as
//! an hour in (core.md §7).

use crate::resource::Resource;
use crate::units::Seconds;

/// How far the simulation has got, in ticks — held as a world resource.
///
/// The engine installs it before the first tick and keeps it current, so
/// `world.resource::<Time>()` always answers. A game never inserts one;
/// [`Time::new`] is how the engine builds it from `GameConfig::fixed_dt`.
///
/// `tick` is the canonical timeline: it counts Update phases, not frames, and
/// it advances by exactly one per Update however fast or slow the machine is.
/// Simulation code that wants "how long since X" stores a tick and subtracts.
///
/// ```
/// use jidousha_core::{Seconds, Simulation, Time};
///
/// let mut simulation = Simulation::new(7, Seconds(1.0 / 60.0));
/// simulation.tick();
/// simulation.tick();
///
/// let time = simulation.world().resource::<Time>();
/// assert_eq!(time.tick, 2);
/// assert_eq!(time.elapsed, Seconds(2.0 / 60.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Time {
    /// Update ticks since startup — the canonical timeline.
    pub tick: u64,
    /// The length of one tick. Constant for the whole run.
    pub fixed_dt: Seconds,
    /// `tick * fixed_dt`.
    ///
    /// Derived, and inexact for steps `f32` cannot represent — 1/60 among them.
    /// It is recomputed from `tick` every step rather than accumulated, which
    /// keeps the error from compounding (an hour of ticks: ~0.0002s out, versus
    /// ~2.8s if the steps were summed). Logic that must be exact keys off
    /// `tick`, which is why that is the canonical timeline.
    pub elapsed: Seconds,
    /// How far into the next tick the last rendered frame fell, in `0.0..1.0`.
    ///
    /// Draw-phase only, for interpolating between the last two simulation
    /// states. Update systems must ignore it: reading it there would make the
    /// simulation depend on frame timing, which is exactly what the fixed
    /// timestep exists to prevent.
    pub alpha: f32,
}

impl Resource for Time {}

impl Time {
    /// The clock at the start of a run, before the first tick.
    #[must_use]
    pub fn new(fixed_dt: Seconds) -> Self {
        Self {
            tick: 0,
            fixed_dt,
            elapsed: Seconds::ZERO,
            alpha: 0.0,
        }
    }

    /// Advance by exactly one tick.
    ///
    /// `elapsed` is recomputed from `tick` rather than accumulated, so a long
    /// run cannot drift the way repeated addition of a fractional `fixed_dt`
    /// would.
    pub(crate) fn advance(&mut self) {
        self.tick += 1;
        self.elapsed = Seconds(self.tick as f32 * self.fixed_dt.as_f32());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_clock_starts_before_the_first_tick() {
        let time = Time::new(Seconds(0.5));
        assert_eq!(time.tick, 0);
        assert_eq!(time.elapsed, Seconds::ZERO);
    }

    #[test]
    fn elapsed_is_the_tick_count_times_the_fixed_step() {
        let mut time = Time::new(Seconds(0.5));
        time.advance();
        time.advance();
        time.advance();
        assert_eq!(time.tick, 3);
        assert_eq!(time.elapsed, Seconds(1.5));
    }

    #[test]
    fn elapsed_beats_accumulation_over_a_long_run() {
        // A 1/60 step is not representable in f32, so neither answer is exact.
        // What matters is that the error stays put instead of compounding:
        // over an hour of ticks, summing drifts by ~2.8s while recomputing
        // from the tick count is out by ~0.0002s.
        let fixed_dt = Seconds(1.0 / 60.0);
        let mut time = Time::new(fixed_dt);
        let mut summed = 0.0f32;
        for _ in 0..216_000 {
            time.advance();
            summed += fixed_dt.as_f32();
        }
        let ideal = 3600.0;
        let recomputed_error = (time.elapsed.as_f32() - ideal).abs();
        let summed_error = (summed - ideal).abs();
        assert!(
            recomputed_error < summed_error,
            "recomputed {recomputed_error} should beat summed {summed_error}"
        );
        assert!(recomputed_error < 0.001, "{recomputed_error}");
    }

    #[test]
    fn the_tick_count_is_exact_however_long_the_run() {
        // `elapsed` is derived and inexact; `tick` is the canonical timeline
        // and stays exact, which is why game logic keys off it (core.md §7).
        let mut time = Time::new(Seconds(1.0 / 60.0));
        for _ in 0..216_000 {
            time.advance();
        }
        assert_eq!(time.tick, 216_000);
    }
}
