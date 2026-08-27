//! The instrument: an accumulator for failed checks.
//!
//! Nobody running `--verify` can look at the game, so these messages are the
//! only instrument there is. Two rules follow: a check reports the numbers it
//! judged rather than the conclusion it reached, and a failed check does not
//! stop the run - an instrument that halts at the first bad reading costs a
//! whole cycle per fault.

use std::process::ExitCode;

use jidousha::prelude::*;

/// Every failed check, kept rather than exited on.
#[derive(Default)]
pub struct Checks {
    problems: Vec<(String, String)>,
}

impl Checks {
    /// Record a claim and the numbers behind it.
    pub fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if !ok {
            self.problems.push((what.to_string(), specifics));
        }
    }

    /// How many claims have failed.
    pub fn failures(&self) -> usize {
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
///
/// Not the same thing as a failed check: a character in the wrong place is one
/// fault among several worth reporting together, while a roster that is *gone*
/// leaves nothing after it to measure.
pub fn fail(what: &str, specifics: &str) -> ! {
    eprintln!("{}", complaint(what, specifics));
    std::process::exit(1);
}

/// One problem, in the engine's four-part message shape.
fn complaint(what: &str, specifics: &str) -> String {
    message(
        what,
        specifics,
        "the game changed, or a tuning constant did",
        "run `cargo run -p giri-rt` and play the scenario, then compare with the assertion above",
    )
}

/// An engine message flattened onto one line, for a one-line summary.
///
/// A §9 message is four lines by design, and a `Checks` entry is one: a
/// failure that carried its own line breaks would break the report's shape
/// around the one entry a reader most needs to find.
pub fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `a > b`, and false when either is NaN.
///
/// Spelled out rather than written `!(a <= b)`: the negation of a float
/// comparison silently means something else, and a NaN that crept into a
/// layout would satisfy every plain `<=` check and pass this verification.
pub fn greater(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(std::cmp::Ordering::Greater))
}

/// Within a thousandth, and false when either is NaN.
pub fn near(a: f32, b: f32) -> bool {
    greater(0.001, (a - b).abs())
}
