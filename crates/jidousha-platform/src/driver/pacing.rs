//! How often a frame is allowed to start, when the display will not say.
//!
//! Key types: `Pacing`, `Schedule`, `FALLBACK_CAP_HZ`.
//! Depends on: `jidousha-core` (`Seconds`), `jidousha-render-core`
//! (`Presentation`). Must never depend on: `winit` — the decision is made here
//! and `mod.rs` is the one place it becomes a `ControlFlow` (ADR-0004), which
//! is what lets every case below be tested without a window.
//! INVARIANT: **presentation only**. Nothing here reaches the accumulator, the
//! fixed timestep, or `MAX_FRAME`; a capped loop runs the same ticks over the
//! same wall time as an uncapped one, drawn fewer times (core.md §7, ADR-0005).
//! A cap on the *simulation* would be a speed change, which is the one thing
//! this must never become (frame-pacing.md §6).

use core::time::Duration;

use jidousha_core::Seconds;
use jidousha_render_core::Presentation;

/// How many frames a second a native run draws when nothing else is pacing it.
///
/// **Sixty**, which is the default fixed timestep (`GameConfig::fixed_dt`) and
/// therefore the rate at which this engine's picture actually changes. Drawing
/// faster than the simulation ticks re-draws states the game has already shown:
/// with interpolation it buys smoothness on a display that can present it, and
/// on a surface that never waits for the display it buys nothing at all and
/// costs a whole core.
///
/// It is a **fallback**, not a policy: a vsynced surface — which is every
/// surface this engine can normally get — is paced by the display and ignores
/// this, so a 144Hz monitor still gets 144 frames a second. This is what a
/// surface that refuses to vsync falls back to, and the number is chosen so
/// that the fallback is never *slower* than the thing being drawn.
pub(crate) const FALLBACK_CAP_HZ: f32 = 60.0;

/// The same cap as a period, which is the form every comparison below wants.
const FALLBACK_CAP_PERIOD: Seconds = Seconds(1.0 / FALLBACK_CAP_HZ);

/// What the loop should do before the next frame.
///
/// Two cases, and the second is a **wait**, never a spin: the loop is handed a
/// duration and winit sleeps the thread on it. A cap implemented by looping
/// until a clock moved would burn exactly the core this exists to give back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Schedule {
    /// Ask for the next frame straight away.
    ///
    /// Either the display is pacing the loop — a vsynced present blocks on the
    /// swap chain, which is a wait the operating system does for us — or
    /// nothing is being presented at all and the loop is polling for a device
    /// it needs promptly.
    Now,
    /// Sleep this long, then ask.
    Wait(Duration),
}

/// What the last frame learned about how frames reach the display.
///
/// One value, re-read every frame rather than once at startup: the surface is
/// configured after the device arrives, is reconfigured on every resize, and a
/// backend that lost its device answers differently afterwards.
pub(crate) struct Pacing {
    presentation: Presentation,
}

impl Pacing {
    /// Before any frame has been drawn — nothing is presenting yet.
    pub(crate) fn new() -> Self {
        Self {
            presentation: Presentation::Offscreen,
        }
    }

    /// Record what the backend says about this frame's presentation.
    pub(crate) fn observe(&mut self, presentation: Presentation) {
        self.presentation = presentation;
    }

    /// What to do now that a frame has been drawn and `spent` of it has gone.
    ///
    /// `spent` is how long this frame took to produce
    /// ([`FrameClock::since_frame`](crate::FrameClock::since_frame)), so the
    /// wait is the remainder of the cap rather than a whole cap on top of the
    /// work — a loop that slept a full period after every frame would run at
    /// half the rate it was capped to.
    pub(crate) fn schedule(&self, spent: Seconds) -> Schedule {
        if !self.presentation.needs_a_cap() {
            return Schedule::Now;
        }
        let remaining = FALLBACK_CAP_PERIOD.as_f32() - spent.as_f32();
        // A frame that already overran the cap has nothing left to wait out,
        // and asking `Duration` for a negative span would panic.
        if remaining <= 0.0 {
            return Schedule::Now;
        }
        Schedule::Wait(Duration::from_secs_f32(remaining))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pacing(presentation: Presentation) -> Pacing {
        let mut pacing = Pacing::new();
        pacing.observe(presentation);
        pacing
    }

    #[test]
    fn a_vsynced_surface_is_asked_for_the_next_frame_immediately() {
        // The display is already the wait. A cap on top of it would beat
        // against the refresh — a 144Hz monitor capped at 60 does not draw 60
        // evenly spaced frames, it drops two out of every five on a rhythm
        // nobody asked for.
        assert_eq!(
            pacing(Presentation::Vsync).schedule(Seconds(0.001)),
            Schedule::Now
        );
    }

    #[test]
    fn a_surface_that_never_waits_is_made_to_wait_out_the_rest_of_the_cap() {
        // The runaway this whole module exists for: on a mailbox or immediate
        // swap chain nothing blocks, so a paused 2D game redraws as fast as the
        // machine can manage until something here says otherwise.
        for presentation in [Presentation::Mailbox, Presentation::Immediate] {
            let spent = Seconds(0.004);
            let Schedule::Wait(wait) = pacing(presentation).schedule(spent) else {
                panic!("{presentation} was left uncapped");
            };
            let total = spent.as_f32() + wait.as_secs_f32();
            assert!(
                (total - 1.0 / FALLBACK_CAP_HZ).abs() < 1e-6,
                "{presentation}: the frame plus its wait is {total}s, not one cap period"
            );
        }
    }

    #[test]
    fn a_frame_that_already_overran_the_cap_does_not_wait_at_all() {
        // And in particular does not try to wait a negative duration, which is
        // a panic rather than a small number.
        assert_eq!(
            pacing(Presentation::Immediate).schedule(Seconds(1.0)),
            Schedule::Now
        );
        assert_eq!(
            pacing(Presentation::Immediate).schedule(FALLBACK_CAP_PERIOD),
            Schedule::Now
        );
    }

    #[test]
    fn a_run_with_nothing_on_screen_yet_is_not_slowed_down_while_it_starts() {
        // `Offscreen` is the answer for a device that has not arrived, and the
        // frames that report it are the ones polling for it (renderer.md §10).
        // Capping those would make every windowed run take longer to show its
        // first picture, in exchange for nothing — no frames are reaching a
        // display to be paced.
        assert_eq!(Pacing::new().schedule(Seconds(0.0)), Schedule::Now);
        assert_eq!(
            pacing(Presentation::Offscreen).schedule(Seconds(0.0)),
            Schedule::Now
        );
    }

    #[test]
    fn the_fallback_cap_is_never_slower_than_the_simulation_it_draws() {
        // The failure mode a smaller number would produce: a cap under the tick
        // rate makes every rendered frame run two ticks, which is the "ball
        // jumps forward" signature this project already has a whole document
        // about (frame-pacing.md §2). The cap must bound the *waste*, never the
        // game.
        let fixed_dt = jidousha_core::GameConfig::default().fixed_dt.as_f32();
        assert!(
            FALLBACK_CAP_PERIOD.as_f32() <= fixed_dt + 1e-6,
            "a {FALLBACK_CAP_HZ}Hz cap draws less often than the {}Hz simulation ticks",
            1.0 / fixed_dt
        );
    }
}
