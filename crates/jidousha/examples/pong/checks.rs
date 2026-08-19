//! The instrument: an accumulator for failed checks, and the float comparisons
//! every reading in this example is spelled with.
//!
//! Nobody running `--verify` can look at the game, so these messages are the
//! only instrument there is. Two rules follow: a check reports the numbers it
//! judged rather than the conclusion it reached, and a failed check does not
//! stop the run, because an instrument that halts at the first bad reading
//! costs a whole cycle per fault.

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
    /// Record a reading. `what` is the fault; `specifics` are the numbers it
    /// judged, which is the half that turns a failure into a diagnosis.
    pub(crate) fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if !ok {
            self.problems.push((what.to_owned(), specifics));
        }
    }

    /// How many readings were bad, for the summary line.
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
/// Not the same thing as a failed check: a ball in the wrong place is one fault
/// among several worth reporting together, while a ball that is *gone* leaves
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
        "the game changed, or the controller in verify.rs did",
        "run `cargo run -p jidousha --example pong` and watch it, then compare with the \
         assertion above",
    )
}

/// `a > b`, and false when either is NaN.
///
/// Spelled out rather than as the negation of `<=`, because a NaN that crept
/// into a position satisfies every plain `<=` and would pass this whole run.
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

/// The union of every quad covering `at` that fits inside a box of `size`
/// centred there, or `None` if nothing does.
///
/// This is how a *circle* is checked. `ctx.circle` submits sixteen wedges, not
/// one square, so nothing the size of the ball is drawn anywhere; what is true
/// is that every wedge has the centre as a corner and fits inside the disc's
/// bounding box, so their union is exactly that box.
pub(crate) fn disc_union(frame: &FrameRecord, at: Vec2, size: f32) -> Option<Rect> {
    let box_of_it = Rect::from_center_size(at, Vec2::splat(size));
    let mut union: Option<Rect> = None;
    for quad in frame.covering(at) {
        let drawn = quad.bounds();
        // Written out rather than as `Rect::contains`, which is half-open and
        // would throw away the one wedge reaching the far edge.
        let inside = drawn.min.x >= box_of_it.min.x - 1e-3
            && drawn.min.y >= box_of_it.min.y - 1e-3
            && drawn.max.x <= box_of_it.max.x + 1e-3
            && drawn.max.y <= box_of_it.max.y + 1e-3;
        if !inside {
            continue;
        }
        union = Some(match union {
            None => drawn,
            Some(so_far) => Rect {
                min: so_far.min.min(drawn.min),
                max: so_far.max.max(drawn.max),
            },
        });
    }
    union
}
