//! The instrument: an accumulator for failed checks, kept rather than exited
//! on, each reported in the engine's four-part shape with the numbers it
//! judged.

use std::cmp::Ordering;
use std::process::ExitCode;

use jidousha::prelude::*;

/// Every failed check, kept rather than exited on.
#[derive(Default)]
pub(crate) struct Checks {
    problems: Vec<(String, String)>,
    passed: usize,
}

impl Checks {
    pub(crate) fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if ok {
            self.passed += 1;
        } else {
            self.problems.push((what.to_string(), specifics));
        }
    }

    /// How many checks held, and how many did not.
    pub(crate) fn counts(&self) -> (usize, usize) {
        (self.passed, self.problems.len())
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

/// One problem, in the engine's four-part message shape.
fn complaint(what: &str, specifics: &str) -> String {
    message(
        what,
        specifics,
        "the game changed, or the engine did",
        "run `cargo run -p call_of_cthulhu_r3v2 -- --verify` and read the transcript, then \
         compare with the assertion above",
    )
}

/// `a > b`, and false when either is NaN.
pub(crate) fn greater(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(Ordering::Greater))
}

/// Within `tolerance`, and false when either is NaN.
pub(crate) fn near(a: f32, b: f32, tolerance: f32) -> bool {
    greater(tolerance, (a - b).abs())
}
