//! The instrument: an accumulator for failed checks.
//!
//! Nobody running `--verify` watches the game, so a check reports the numbers
//! it judged, and a failed check does not stop the run.

use std::process::ExitCode;

use jidousha::prelude::*;

/// Every failed check, kept rather than exited on.
#[derive(Default)]
pub(crate) struct Checks {
    problems: Vec<(String, String)>,
    /// How many checks were asked, failed or not — printed in the summary.
    pub(crate) asked: usize,
}

impl Checks {
    pub(crate) fn require(&mut self, ok: bool, what: &str, specifics: String) {
        self.asked += 1;
        if !ok {
            self.problems.push((what.to_string(), specifics));
        }
    }

    pub(crate) fn failed(&self) -> usize {
        self.problems.len()
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
pub(crate) fn fail(what: &str, specifics: &str) -> ! {
    eprintln!("{}", complaint(what, specifics));
    std::process::exit(1);
}

fn complaint(what: &str, specifics: &str) -> String {
    message(
        what,
        specifics,
        "the game changed, or the engine did",
        "run `cargo run -p stack_card_game_r2` and play to the state above, then compare \
         with the assertion",
    )
}
