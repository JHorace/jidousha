//! The instrument: failed checks collected rather than exited on.
//!
//! Nobody running `--verify` can look at the game, so a check reports the
//! numbers and strings it judged, and a failed check does not stop the run.

use std::process::ExitCode;

use jidousha::prelude::*;

/// Every failed check, kept.
#[derive(Default)]
pub struct Checks {
    problems: Vec<(String, String)>,
    passed: usize,
}

impl Checks {
    /// Record `ok`; on failure keep what was wrong and the specifics it judged.
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

    /// Print every failure; SUCCESS only if there were none.
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

/// Stop the run, for a reading that leaves nothing after it to measure.
pub fn fail(what: &str, specifics: &str) -> ! {
    eprintln!("{}", complaint(what, specifics));
    std::process::exit(1);
}

fn complaint(what: &str, specifics: &str) -> String {
    message(
        what,
        specifics,
        "the game, its content, or the engine changed",
        "run `cargo run -p keifu_x_inheritance_r3v1` and point at the hero or screen named above, then \
         compare with spec/SPEC.md and spec/MODULES.md",
    )
}
