//! The instrument: an accumulator for failed checks.
//!
//! Nobody running `--verify` can look at the game, so these messages are the
//! only instrument there is. Two rules follow, and both cost a cycle to learn:
//! a check reports the numbers it judged rather than the conclusion it reached,
//! and a failed check does not stop the run — an instrument that halts at the
//! first bad reading costs a whole cycle per fault (e0-findings.md F-061).

use std::process::ExitCode;

use jidousha::prelude::*;

/// Every failed check, kept rather than exited on.
///
/// Nobody running `--verify` can look at the game, so the run is the only
/// instrument there is — and an instrument that stops at the first bad reading
/// costs a whole cycle per fault. Each entry prints in the engine's four-part
/// shape and reports the numbers it judged rather than the conclusion it
/// reached, which are the same two rules for the same reason.
#[derive(Default)]
pub(crate) struct Checks {
    problems: Vec<(String, String)>,
}

impl Checks {
    pub(crate) fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if !ok {
            self.problems.push((what.to_string(), specifics));
        }
    }

    /// Whether every check so far has held.
    pub(crate) fn passed(&self) -> bool {
        self.problems.is_empty()
    }

    /// Print everything that failed, and say whether anything did.
    pub(crate) fn verdict(&self) -> ExitCode {
        if self.problems.is_empty() {
            return ExitCode::SUCCESS;
        }
        for (what, specifics) in &self.problems {
            eprintln!("{}", complaint(what, specifics));
        }
        ExitCode::FAILURE
    }
}

/// Stop the run, for a reading that makes every later one meaningless.
///
/// Not the same thing as a failed check, and the distinction is the whole of
/// why `Checks` exists: a paddle that is *in the wrong place* is one fault among
/// several worth reporting together, while a paddle that is *gone* leaves
/// nothing after it to measure. Only the second kind belongs here.
pub(crate) fn fail(what: &str, specifics: &str) -> ! {
    eprintln!("{}", complaint(what, specifics));
    std::process::exit(1);
}

/// One problem, in the engine's four-part message shape.
fn complaint(what: &str, specifics: &str) -> String {
    message(
        what,
        specifics,
        "the game changed, or the engine did",
        "run `cargo run -p call_of_cthulhu_r3v1` and play it, then \
         compare with the assertion above",
    )
}
