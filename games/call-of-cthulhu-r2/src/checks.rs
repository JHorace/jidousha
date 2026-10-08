//! The instrument: an accumulator for failed checks.
//!
//! A check reports the numbers it judged rather than the conclusion it
//! reached, and a failed check does not stop the run — an instrument that
//! halts at the first bad reading costs a whole cycle per fault.

use std::process::ExitCode;

use jidousha::prelude::*;

/// Every failed check, kept rather than exited on.
#[derive(Default)]
pub struct Checks {
    problems: Vec<(String, String)>,
    passed: usize,
}

impl Checks {
    pub fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if ok {
            self.passed += 1;
        } else {
            self.problems.push((what.to_owned(), specifics));
        }
    }

    pub fn passed(&self) -> usize {
        self.passed
    }

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

fn complaint(what: &str, specifics: &str) -> String {
    message(
        what,
        specifics,
        "the game's rules or screens changed, or the engine did",
        "run `cargo run -p call_of_cthulhu_r2 -- --verify` and read the summary beside the \
         failing check",
    )
}
