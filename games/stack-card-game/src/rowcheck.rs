//! Reading the stack panel back: the rows a player reads, as strings, and the
//! glyphs that were really drawn for them.

use crate::checks::Checks;
use crate::ui::{Mark, Screen};
use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord};

/// One stack row as it should read (shipped literals, top of the stack first).
#[derive(Debug)]
pub struct Expect {
    left: &'static str,
    right: &'static str,
    mark: Mark,
}

impl Expect {
    pub fn row(left: &'static str, right: &'static str, mark: Mark) -> Expect {
        Expect { left, right, mark }
    }
}

/// The panel must show exactly these rows, in this order.
pub fn check_rows(checks: &mut Checks, what: &str, screen: &Screen, want: &[Expect]) {
    let got: Vec<(&str, &str, Mark)> = screen
        .stack_rows
        .iter()
        .map(|row| (row.left.as_str(), row.right.as_str(), row.mark))
        .collect();
    let same = got.len() == want.len()
        && got
            .iter()
            .zip(want)
            .all(|(g, w)| g.0 == w.left && g.1 == w.right && g.2 == w.mark);
    checks.require(
        same,
        &format!("the stack panel does not show what {what} needs"),
        format!("panel rows {got:?}; expected {want:?}"),
    );
}

/// How many font glyph quads were drawn wholly inside `rect`.
pub fn glyphs_in(frame: &FrameRecord, rect: Rect, font: BackendTextureId) -> usize {
    frame
        .quads()
        .iter()
        .filter(|quad| quad.texture == font && rect.contains_rect(quad.bounds()))
        .count()
}
