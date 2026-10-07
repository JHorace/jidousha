//! The instrument: failed checks collected rather than exited on, each reporting the
//! numbers it judged, and the float comparison every reading is spelled with.

use std::process::ExitCode;

use jidousha::prelude::*;

/// Every failed check, kept.
#[derive(Default)]
pub struct Checks {
    problems: Vec<(String, String)>,
    passed: usize,
}

impl Checks {
    /// Record `ok`; on failure keep what was wrong and the numbers it judged.
    pub fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if ok {
            self.passed += 1;
        } else {
            self.problems.push((what.to_owned(), specifics));
        }
    }

    /// How many checks passed and failed.
    pub fn counts(&self) -> (usize, usize) {
        (self.passed, self.problems.len())
    }

    /// Print every failure and say whether anything failed.
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
        "the game changed, or the rules did",
        "run `cargo run -p call-of-cthulhu` and play it, then compare with DESIGN.md",
    )
}
