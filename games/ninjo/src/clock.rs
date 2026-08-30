//! The world clock: integer world-minutes, speed as player input (DESIGN §4).
//!
//! The engine's fixed 60Hz timestep is untouched and ticks always run. The
//! game holds this clock as a resource and advances it each tick by the
//! current speed: every tick `accum` grows by the speed's per-tick
//! accumulation (a named drawer constant; 0 while paused), and every
//! `minute_ticks` accumulated carries one world-minute. Integer arithmetic
//! throughout — the clock never holds a float.
//!
//! **Pause is not a freeze of the program.** Ticks run, input is processed,
//! the camera moves; only this resource stops carrying minutes. Orders issued
//! while paused are ordinary recorded input that takes effect at the held
//! world-time — a property of the model, not a feature.
//!
//! **Speed is player input through the snapshot** (`flow.rs` reads the keys
//! and the chips); this file only holds the state and the arithmetic, so the
//! clock cannot be advanced by anything but the one `advance` system.
//!
//! The one float here is presentation: `reading` projects the clock onto
//! fractional minutes for the draw systems' between-tile interpolation, and
//! `remember` keeps the previous tick's projection so a token is drawn at
//! `previous.lerp(current, alpha)` (ADR-0041). Nothing reads it back into the
//! simulation.

use jidousha::prelude::*;

use crate::constants::Tuning;

/// The three running speeds. Pause is a separate flag, so resuming returns to
/// the speed the player last chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rate {
    /// 1x.
    X1,
    /// 2x.
    X2,
    /// 4x.
    X4,
}

impl Rate {
    /// The chip's label.
    pub fn label(self) -> &'static str {
        match self {
            Rate::X1 => "1x",
            Rate::X2 => "2x",
            Rate::X4 => "4x",
        }
    }

    /// This speed's per-tick accumulation, from the drawer.
    pub fn accumulation(self, tuning: &Tuning) -> i64 {
        match self {
            Rate::X1 => tuning.speed_1x,
            Rate::X2 => tuning.speed_2x,
            Rate::X4 => tuning.speed_4x,
        }
    }
}

/// The world clock, held as a resource.
#[derive(Clone, Copy, Debug)]
pub struct Clock {
    /// World-minutes since the scenario opened. The address every occurrence
    /// and every event carries.
    pub minutes: u64,
    /// Ticks-worth of accumulation toward the next minute, in `0..minute_ticks`.
    pub accum: i64,
    /// Whether the clock is held. The program never is.
    pub paused: bool,
    /// The speed the clock runs at while not paused.
    pub rate: Rate,
    /// The previous tick's [`reading`](Clock::reading) — presentation state
    /// for ADR-0041's `previous.lerp(current, alpha)`, written by
    /// [`remember`] and read by nothing in the simulation.
    pub previous_reading: f32,
}

impl Resource for Clock {}

impl Clock {
    /// The clock a scenario opens with: minute zero, paused at 1x — the player
    /// decides when the world starts moving (pause is consent, DESIGN §0).
    pub fn opening() -> Self {
        Self {
            minutes: 0,
            accum: 0,
            paused: true,
            rate: Rate::X1,
            previous_reading: 0.0,
        }
    }

    /// What this tick will add to `accum`.
    pub fn accumulation(&self, tuning: &Tuning) -> i64 {
        if self.paused {
            0
        } else {
            self.rate.accumulation(tuning)
        }
    }

    /// The clock as fractional minutes — presentation only, for the draw
    /// systems' between-tile interpolation. The simulation reads `minutes`.
    pub fn reading(&self, tuning: &Tuning) -> f32 {
        let ticks = tuning.minute_ticks.max(1) as f32;
        self.minutes as f32 + self.accum as f32 / ticks
    }
}

/// A world-minute as the clock readout and every log line print it:
/// `d1 06:40` — day, hour, minute, days counted from one.
pub fn stamp(minutes: u64) -> String {
    format!(
        "d{} {:02}:{:02}",
        minutes / 1440 + 1,
        (minutes % 1440) / 60,
        minutes % 60
    )
}

/// Keep the previous tick's fractional reading, for ADR-0041's idiom.
///
/// Registered before [`advance`], so `previous_reading` is where the clock
/// stood when the last frame's state was committed.
pub fn remember(world: &mut World) {
    let tuning = *world.resource::<Tuning>();
    let clock = world.resource_mut::<Clock>();
    clock.previous_reading = clock.reading(&tuning);
}

/// Advance the world clock by the current speed.
///
/// A tick that accumulates past several minute boundaries carries them all —
/// the scheduler (`sim::fire_due`, registered after this) fires everything
/// due in the crossed span, in world-time order.
pub fn advance(world: &mut World) {
    let tuning = *world.resource::<Tuning>();
    let step = {
        let clock = world.resource::<Clock>();
        clock.accumulation(&tuning)
    };
    let ticks = tuning.minute_ticks.max(1);
    let clock = world.resource_mut::<Clock>();
    clock.accum += step;
    while clock.accum >= ticks {
        clock.accum -= ticks;
        clock.minutes += 1;
    }
}
