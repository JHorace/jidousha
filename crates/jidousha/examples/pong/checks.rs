//! The instrument: an accumulator for failed checks, and the float comparisons
//! every reading in this run is spelled with.
//!
//! Nobody running `--verify` can look at the game, so these messages are the
//! only instrument there is. Two rules follow: a check reports the numbers it
//! judged rather than the conclusion it reached, and a failed check does not
//! stop the run.

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
    /// Record `what` as a problem unless `ok`, with the numbers that were
    /// judged.
    pub(crate) fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if !ok {
            self.problems.push((what.to_owned(), specifics));
        }
    }

    /// How many checks have failed so far.
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
/// Not the same thing as a failed check: a paddle in the wrong place is one
/// fault among several worth reporting together, while a paddle that is *gone*
/// leaves nothing after it to measure.
pub(crate) fn fail(what: &str, specifics: &str) -> ! {
    eprintln!("{}", complaint(what, specifics));
    std::process::exit(1);
}

/// One problem, in the engine's four-part message shape.
fn complaint(what: &str, specifics: &str) -> String {
    message(
        what,
        specifics,
        "the game's numbers changed, or the engine did",
        "run `cargo run -p jidousha --example pong` and watch it, then compare with the \
         assertion above",
    )
}

/// `a > b`, and false when either is NaN.
///
/// Spelled out rather than written `!(a <= b)`, because the negation of a float
/// comparison silently means something else: a NaN that crept into a position
/// satisfies every plain `<=` check and would pass this whole verification.
pub(crate) fn greater(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(Ordering::Greater))
}

/// Within a thousandth, and false when either is NaN.
pub(crate) fn near(a: f32, b: f32) -> bool {
    greater(0.001, (a - b).abs())
}

/// Within `slack`, and false when either is NaN.
pub(crate) fn within(a: f32, b: f32, slack: f32) -> bool {
    greater(slack, (a - b).abs())
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

/// The union of every quad covering `at` that fits inside `box_of_it`.
///
/// `ctx.circle` submits sixteen wedges and nothing the size of the disc is
/// drawn anywhere, so "a quad the size of the thing" is the wrong question for
/// a ball. All sixteen share the centre as a corner and all sixteen fit inside
/// the circle's bounding box, so the union of the ones covering the centre is
/// exactly `2r` square. The box filter is what keeps the centre-line dash
/// behind the ball out of the answer.
pub(crate) fn disc_span(frame: &FrameRecord, at: Vec2, radius: f32) -> Option<Rect> {
    let box_of_it = Rect::from_center_size(at, Vec2::splat(radius * 2.0));
    let mut union: Option<Rect> = None;
    for quad in frame.covering(at) {
        let drawn = quad.bounds();
        // Written out rather than as `Rect::contains`, which is half-open and
        // would throw away the wedge reaching the far edge.
        let inside = greater(drawn.min.x, box_of_it.min.x - 0.001)
            && greater(drawn.min.y, box_of_it.min.y - 0.001)
            && greater(box_of_it.max.x + 0.001, drawn.max.x)
            && greater(box_of_it.max.y + 0.001, drawn.max.y);
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
