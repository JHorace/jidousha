//! The instrument: an accumulator for failed checks, and the float comparisons
//! every reading in this example is spelled with.
//!
//! Nobody running `--verify` can look at the game, so these messages are the
//! only instrument there is. Two rules follow, and both cost a cycle to learn:
//! a check reports the numbers it judged rather than the conclusion it reached,
//! and a failed check does not stop the run — an instrument that halts at the
//! first bad reading costs a whole cycle per fault (e0-findings.md F-061).

use std::cmp::Ordering;
use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::FrameRecord;

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
        "run `cargo run -p jidousha --example prototype_kit` and watch it, then \
         compare with the assertion above",
    )
}

/// `a > b`, and false when either is NaN.
///
/// Spelled out rather than written `!(a > b)` because the negation of a
/// float comparison silently means something else — a NaN that crept into a
/// position would satisfy every plain `<=` check and pass this verification
/// (the same reason `circle_quads` spells its radius test out).
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
