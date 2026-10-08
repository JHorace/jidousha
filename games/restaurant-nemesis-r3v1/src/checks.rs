//! The instrument: failed checks collected rather than exited on, and the
//! float comparisons every reading is spelled with.
//!
//! A check reports the numbers it judged, and a failed check does not stop
//! the run (`docs/api/jidousha-testing.md`, "Collect the failures").

use std::process::ExitCode;

use jidousha::prelude::*;

/// Every failed check, kept.
#[derive(Default)]
pub struct Checks {
    problems: Vec<(String, String)>,
    /// How many checks ran, for the summary.
    pub ran: usize,
}

impl Checks {
    /// Record one check.
    pub fn require(&mut self, ok: bool, what: &str, specifics: String) {
        self.ran += 1;
        if !ok {
            self.problems.push((what.to_string(), specifics));
        }
    }

    /// How many failed.
    pub fn failed(&self) -> usize {
        self.problems.len()
    }

    /// Print everything that failed, and say whether anything did.
    pub fn verdict(&self) -> ExitCode {
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
pub fn fail(what: &str, specifics: &str) -> ! {
    eprintln!("{}", complaint(what, specifics));
    std::process::exit(1);
}

fn complaint(what: &str, specifics: &str) -> String {
    message(
        what,
        specifics,
        "the game changed, or the engine did",
        "run `cargo run -p restaurant_nemesis_r3v1` and play it, then compare with the assertion above",
    )
}
