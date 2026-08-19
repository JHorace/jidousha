//! The instrument: somewhere to put a failed check, and the float comparisons
//! every reading is spelled with.
//!
//! Two rules, both of which cost a cycle each to learn the hard way. A check
//! reports the numbers it judged rather than the conclusion it reached —
//! "no one won" says nothing, "no one won: 3-2, longest rally 14 touches, top
//! ball speed 25.6 u/s" says the ball is too slow for the field. And a failed
//! check does not stop the run: nobody watching a `--verify` mode can look at
//! the game, so an instrument that halts at the first bad reading costs a whole
//! cycle per fault.

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
    pub(crate) fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if !ok {
            self.problems.push((what.to_owned(), specifics));
        }
    }

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
/// Not the same thing as a failed check: a ball in the wrong place is one
/// fault among several worth reporting together, a ball that does not exist
/// leaves nothing after it to measure. Only the second kind belongs here.
pub(crate) fn fail(what: &str, specifics: &str) -> ! {
    eprintln!("{}", complaint(what, specifics));
    std::process::exit(1);
}

fn complaint(what: &str, specifics: &str) -> String {
    message(
        what,
        specifics,
        "the game changed, the controller in verify.rs changed, or the engine did",
        "run `cargo run -p jidousha --example pong` and watch it, then compare with the \
         numbers above; the capture in target/verify/pong.png is the same frame this run \
         judged",
    )
}

/// `a > b`, and false when either is NaN.
///
/// Written out rather than as `!(a <= b)`, because negating a float comparison
/// silently means something else: a NaN that crept into a position satisfies
/// every `!(a <= b)` and would pass this whole verification.
pub(crate) fn greater(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(Ordering::Greater))
}

/// Within `slack`, and false when either is NaN.
pub(crate) fn within(a: f32, b: f32, slack: f32) -> bool {
    greater(slack, (a - b).abs())
}

/// Within a thousandth, and false when either is NaN.
pub(crate) fn near(a: f32, b: f32) -> bool {
    within(a, b, 0.001)
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
