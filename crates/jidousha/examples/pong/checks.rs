//! The instrument: an accumulator for failed checks, and the float comparisons
//! every reading in this game's verification is spelled with.
//!
//! Nobody running `--verify` can look at the game, so these messages are the
//! only instrument there is. A check reports the numbers it judged rather than
//! the conclusion it reached, and a failed check does not stop the run.

use std::cmp::Ordering;
use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::FrameRecord;

/// Every failed check, kept rather than exited on.
#[derive(Default)]
pub(crate) struct Checks {
    problems: Vec<(String, String)>,
}

impl Checks {
    /// Record a reading, and what it looked at if it was wrong.
    pub(crate) fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if !ok {
            self.problems.push((what.to_string(), specifics));
        }
    }

    /// How many checks failed.
    pub(crate) fn failures(&self) -> usize {
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
///
/// A paddle in the wrong place is one fault among several worth reporting
/// together; a paddle that is *gone* leaves nothing after it to measure.
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
        "run `cargo run -p jidousha --example pong` and watch it, then compare with the \
         assertion above",
    )
}

/// `a > b`, and false when either is NaN.
///
/// Spelled out rather than as `!(a <= b)`, because the negation of a float
/// comparison quietly means something else: a NaN that crept into a position
/// satisfies every negated test and passes the whole verification.
pub(crate) fn greater(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(Ordering::Greater))
}

/// Within a thousandth, and false when either is NaN.
pub(crate) fn near(a: f32, b: f32) -> bool {
    greater(0.001, (a - b).abs())
}

/// The sizes of every quad covering a point, for a message that has to say what
/// it found rather than only what it wanted.
pub(crate) fn sizes_covering(frame: &FrameRecord, at: Vec2) -> String {
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
