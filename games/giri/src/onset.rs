//! Heuristic-onset instrumentation: how long a beat took to assemble, and how
//! much of it was spent looking (DESIGN §11's open question, UI.md §12).
//!
//! DESIGN §11 says where heuristic play begins is unanswerable a priori and has
//! to be *located* from playtesting. §8a says the cheap instrument is "what
//! players inspect and how long assembly takes per beat". This is that
//! instrument and nothing more: two counters, one log line at SEND, and a
//! `println!` beside it.
//!
//! **Local, and that is a property rather than a promise.** Nothing here opens a
//! socket, writes a file, or names a host; the whole of it is a line in the
//! expedition log and the same line on stdout, which on the web is the browser
//! console. It also records whether or not the tuning drawer was ever opened —
//! the two features are neighbours because both serve playtesting, not because
//! either depends on the other.
//!
//! **Ticks, not clocks.** The duration is counted in Update ticks and converted
//! with `Time::fixed_dt`; no wall clock is read anywhere (DESIGN invariant 5,
//! and the engine's standing rule).

use jidousha::prelude::*;

use crate::checks::Checks;
use crate::verify::BeatRun;

/// Whether this process is a person playing, rather than `--verify` scripting.
///
/// The same test `main` makes, for the same reason it makes it. Only the
/// *printing* is gated: the log line is written either way, because it is the
/// record. A verify run builds forty-four headless sessions in one process and
/// printing from each would bury the run's own report under two hundred lines
/// about a pointer nobody was holding.
pub fn playing() -> bool {
    !std::env::args().any(|argument| argument == "--verify")
}

/// What one beat's assembly cost the player, in the two quantities §8a names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Onset {
    /// The tick of the first roster interaction of this beat — the moment
    /// assembly starts. `None` until the player first touches a party card.
    pub first_touch: Option<u64>,
    /// How many times the pointer arrived on a sheet: a job's card or a
    /// person's.
    ///
    /// **Arrivals, not ticks spent.** Every sheet in giri is always on screen
    /// (invariant 2: the game never hides), so there is no inspect verb to
    /// count; what a player does instead is move the pointer onto a card to
    /// read it against the party, which is the hover the info panel reacts to.
    /// One arrival is one look.
    pub looks: u32,
    /// The card the pointer was on last tick, so an arrival is counted once
    /// rather than once per tick.
    pub on: Option<Card>,
}

/// Which sheet the pointer is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Card {
    /// A job's card, by index in the beat's offers.
    Quest(usize),
    /// A person's card, by roster index.
    Person(usize),
}

impl Onset {
    /// Note where the pointer is this tick, counting an arrival if it moved on
    /// to a card it was not on.
    pub fn look(&mut self, at: Option<Card>) {
        if at.is_some() && at != self.on {
            self.looks += 1;
        }
        self.on = at;
    }

    /// Note that the player touched the roster, which is when assembly starts.
    pub fn touch(&mut self, tick: u64) {
        self.first_touch.get_or_insert(tick);
    }

    /// The line SEND appends to the run log.
    ///
    /// Says "never assembled" rather than 0 when the party went out without a
    /// roster click: zero ticks would read as instant assembly, and the two are
    /// different facts about a playtest.
    pub fn line(&self, tick: u64, fixed_dt: Seconds) -> String {
        match self.first_touch {
            Some(start) => {
                let ticks = tick.saturating_sub(start);
                format!(
                    "assembly {ticks} ticks ({:.1}s) - {} sheet looks",
                    ticks as f32 * fixed_dt.0,
                    self.looks
                )
            }
            None => format!(
                "assembly not started - the party went out unchanged - {} sheet looks",
                self.looks
            ),
        }
    }
}

/// The instrument, checked against a played beat.
///
/// **An instrument nobody reads is an instrument nobody notices has stopped.**
/// Nothing in the game depends on these two numbers, so the only thing that can
/// tell a working counter from a zero is a check that asks a scripted run — one
/// that provably touched the roster and moved the pointer over cards — for what
/// its own log says.
pub fn judge(checks: &mut Checks, run: &BeatRun) {
    let beat = run.index + 1;
    let Some(line) = run
        .report_flow
        .log
        .iter()
        .find(|line| line.contains("assembly "))
    else {
        checks.require(
            false,
            "a beat was sent and the run log recorded nothing about the assembly",
            format!(
                "beat {beat}'s log is {:?}; DESIGN §11's onset question is answered from this \
                 line and nothing else writes it",
                run.report_flow.log
            ),
        );
        return;
    };
    let number_before = |word: &str| {
        line.split_whitespace()
            .zip(line.split_whitespace().skip(1))
            .find(|(_, next)| next.starts_with(word))
            .and_then(|(value, _)| value.parse::<u64>().ok())
    };
    let ticks = number_before("ticks");
    let looks = number_before("sheet");
    checks.require(
        ticks.is_some_and(|ticks| ticks > 0) && looks.is_some_and(|looks| looks > 0),
        "the assembly instrument recorded nothing for a beat that was assembled",
        format!(
            "beat {beat} logged {line:?}; the scripted run clicks the roster and moves the \
             pointer across cards, so both numbers are positive or the counter is not counting"
        ),
    );
}
