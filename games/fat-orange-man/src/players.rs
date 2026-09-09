//! Three players for the `--verify` run, the way `docs/api/jidousha-controllers.md`
//! asks — except this game has no opponent, so the three are not
//! win / lose / mediocre but **three tap cadences**:
//!
//! - `IDLE` never taps. It proves the world still moves without the player —
//!   the bots keep feeding, the global count climbs, the square grows — while
//!   the player's own count stays nailed to zero. That is the clicker's version
//!   of "prove the game can be lost".
//! - `HUMAN` taps about four times a second, which is what a thumb actually
//!   does. It is the run that says whether the loop is worth doing: the count
//!   climbs, the rank improves, nothing is dropped.
//! - `MASHER` taps every tick. It is the upper bound — no cooldown, no limit
//!   (GDD §4.1) — and it must climb strictly past `HUMAN` and past some bots.
//!
//! A tap is a real pointer press/release pair through `SnapshotBuilder`, so it
//! goes through the same edge rules a browser tap does.

/// Whether a player taps on a given tick. Ticks count from 1.
pub type TapPlan = fn(u64) -> bool;

/// One named cadence.
pub struct Cadence {
    /// What to call it in the report.
    pub name: &'static str,
    /// Its tap schedule.
    pub taps: TapPlan,
}

/// Never feeds him.
pub const IDLE: Cadence = Cadence {
    name: "idle",
    taps: |_tick| false,
};

/// About four taps a second at 60 ticks a second.
pub const HUMAN: Cadence = Cadence {
    name: "human",
    taps: |tick| tick % 15 == 0,
};

/// Every tick.
pub const MASHER: Cadence = Cadence {
    name: "masher",
    taps: |_tick| true,
};

/// The three numbers a clicker player reports about itself — the analogue of
/// the controllers document's returner / objective / aim triple.
#[derive(Clone, Copy, Debug)]
pub struct Report {
    /// Taps the plan issued over the run.
    pub taps_issued: u64,
    /// The player's own feed count at the end. Equal to `taps_issued` when the
    /// backend dropped nothing.
    pub personal_end: u64,
    /// The player's 1-based leaderboard rank at the end.
    pub rank_end: usize,
}
