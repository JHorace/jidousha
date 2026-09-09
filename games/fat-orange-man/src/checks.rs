//! The instrument: an accumulator for failed checks, and the float comparisons
//! every reading in `verify.rs` is spelled with.
//!
//! Copied in shape from `crates/jidousha/examples/prototype_kit/checks.rs`.
//! Nobody running `--verify` can look at the game, so a check reports the
//! numbers it judged rather than the conclusion it reached, and a failed check
//! is collected rather than exited on — an instrument that halts at the first
//! bad reading costs a whole cycle per fault.

use std::cmp::Ordering;
use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::FrameRecord;

/// Every failed check, kept rather than exited on.
#[derive(Default)]
pub struct Checks {
    problems: Vec<(String, String)>,
}

impl Checks {
    /// Record `what`/`specifics` if `ok` is false.
    pub fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if !ok {
            self.problems.push((what.to_string(), specifics));
        }
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

/// Stop the run, for a reading that makes every later one meaningless — a
/// missing resource, a frame never recorded. Not the same as a failed check.
pub fn fail(what: &str, specifics: &str) -> ! {
    eprintln!("{}", complaint(what, specifics));
    std::process::exit(1);
}

/// One problem, in the engine's four-part message shape.
fn complaint(what: &str, specifics: &str) -> String {
    message(
        what,
        specifics,
        "the game changed, or the engine did",
        "run `cargo run -p fat-orange-man` and watch it, then compare with the assertion above",
    )
}

/// `a > b`, and false when either is NaN. Spelled out rather than `!(a <= b)`,
/// because the negation of a float comparison silently admits NaN.
pub fn greater(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(Ordering::Greater))
}

/// Within a thousandth, and false when either is NaN.
pub fn near(a: f32, b: f32) -> bool {
    greater(0.001, (a - b).abs())
}

/// The sizes of every quad covering a point, for a message that says what it
/// found rather than only what it wanted.
pub fn sizes_covering(frame: &FrameRecord, at: Vec2) -> String {
    let sizes: Vec<String> = frame
        .covering(at)
        .into_iter()
        .map(|quad| {
            let size = quad.bounds().size();
            format!("{:.3}x{:.3}", size.x, size.y)
        })
        .collect();
    if sizes.is_empty() {
        return "nothing at all".to_owned();
    }
    sizes.join(", ")
}
