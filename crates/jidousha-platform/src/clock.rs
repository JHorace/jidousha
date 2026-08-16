//! The only wall clock in the engine.
//!
//! Key types: `FrameClock`.
//! Depends on: `web-time`, `jidousha-core`. Must never be depended on by:
//! anything above the platform seam.
//! INVARIANT: real time enters the engine here and nowhere else. Everything
//! above works in ticks and `Seconds` derived from the fixed timestep, so the
//! same inputs replay identically on a fast machine and a slow one
//! (ADR-0005, core.md §7).
//!
//! DELIBERATE: `web_time::Instant`, not `std::time::Instant`. The std type
//! compiles on wasm and then panics at run time, which is the worst of both —
//! it would pass CI's `cargo check` and fail in a browser. `web-time` is
//! `std::time` on native and `performance.now()` on the web, and it arrives
//! with winit rather than as a dependency of its own (ADR-0005's "time shim at
//! the platform boundary", made concrete).

use jidousha_core::Seconds;
use web_time::Instant;

/// How much real time one frame may contribute, before the loop stops trying
/// to catch up.
///
/// The same ceiling `Simulation` applies internally; applied here too so the
/// clock never even reports a spiral-of-death-sized frame. A machine that
/// stalls for ten seconds resumes ten seconds behind, which is visible and
/// recoverable, rather than running six hundred ticks at once.
const MAX_FRAME: Seconds = Seconds(0.25);

/// Turns the passage of real time into frame durations.
pub struct FrameClock {
    last: Instant,
}

impl FrameClock {
    /// Start the clock now.
    ///
    /// DELIBERATE: no `Default` impl, despite `clippy::new_without_default`
    /// (see ADR-0012) — one way to do everything, and `new` is that way.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            last: Instant::now(),
        }
    }

    /// How long since the last call, clamped.
    ///
    /// The clamp is what a driver wants: a frame that took a quarter second is
    /// reported as a quarter second whether the machine hitched for that long
    /// or the window sat behind a screensaver for an hour.
    pub fn frame(&mut self) -> Seconds {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        Seconds(elapsed.min(MAX_FRAME.as_f32()))
    }

    /// Forget the time that passed, without spending it.
    ///
    /// Called when the window comes back from being hidden or unfocused: the
    /// gap was real time, but it was not gameplay, and feeding it to the
    /// accumulator would make the game lurch on the frame it returns.
    pub fn skip(&mut self) {
        self.last = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_never_longer_than_the_ceiling() {
        // The clamp, checked without waiting: a clock whose last mark is far in
        // the past reports the ceiling, not the gap.
        let mut clock = FrameClock::new();
        clock.last = Instant::now() - core::time::Duration::from_secs(60);
        assert_eq!(clock.frame(), MAX_FRAME);
    }

    #[test]
    fn a_frame_is_never_negative() {
        let mut clock = FrameClock::new();
        assert!(clock.frame().as_f32() >= 0.0);
    }

    #[test]
    fn skipping_forgets_the_gap() {
        let mut clock = FrameClock::new();
        clock.last = Instant::now() - core::time::Duration::from_secs(60);
        clock.skip();
        assert!(
            clock.frame().as_f32() < MAX_FRAME.as_f32(),
            "the hour behind a screensaver is not gameplay"
        );
    }
}
