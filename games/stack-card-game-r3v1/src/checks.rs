//! The instrument: an accumulator for failed checks, the named-check summary
//! lines, and the frame readers every check is spelled with.
//!
//! Nobody running `--verify` can watch the duel, so these messages are the only
//! instrument there is: a check reports the numbers it judged, and a failed
//! check does not stop the run.
//!
//! Key items: `Checks`, `fail`, `rows_found`, `panel_has`.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord};
use jidousha::ui::{Floors, Panel, judge_frame, judge_panel};

use crate::screen::{Art, Flat};

/// Every failed check, kept rather than exited on, and one summary line per
/// named check that passed.
#[derive(Default)]
pub(crate) struct Checks {
    problems: Vec<(String, String)>,
    pub(crate) summary: Vec<String>,
}

impl Checks {
    pub(crate) fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if !ok {
            self.problems.push((what.to_owned(), specifics));
        }
    }

    /// A summary line for the verdict block.
    pub(crate) fn note(&mut self, line: String) {
        self.summary.push(line);
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
        "run `cargo run -p stack_card_game_r3v1` and play the duel, then compare \
         with the assertion above",
    )
}

/// The readability floors: 12-unit text, everything inside the 960 x 540 table.
pub(crate) const FLOORS: Floors = Floors {
    min_text: 12.0,
    chrome: Rect {
        min: Vec2::ZERO,
        max: Vec2::new(960.0, 540.0),
    },
    world: Rect {
        min: Vec2::ZERO,
        max: Vec2::new(960.0, 540.0),
    },
};

/// Judge one panel against the floors and against the frame it was drawn
/// into; every breach becomes a failed check named for `screen`.
pub(crate) fn judge(
    checks: &mut Checks,
    screen: &str,
    panel: &Panel<Art>,
    frame: &FrameRecord,
    font: BackendTextureId,
    view: Rect,
) {
    let controls = vec![("PASS (space)".to_owned(), crate::screen::PASS_BUTTON)];
    for breach in judge_panel(panel, &FLOORS, &controls, &[]) {
        checks.require(
            false,
            "a floor was breached",
            format!("{screen}: {} - {}", breach.what, breach.detail),
        );
    }
    for breach in judge_frame(panel, frame, font, &Flat, view) {
        checks.require(
            false,
            "a row the screen says is not on the frame",
            format!("{screen}: {} - {}", breach.what, breach.detail),
        );
    }
    for text in panel.all_strings() {
        checks.require(
            text.chars().all(|c| (' '..='~').contains(&c)),
            "a string the font cannot draw",
            format!("{screen}: {text:?} has a character outside space..tilde"),
        );
    }
}

/// Whether the panel has a row reading exactly `text`.
pub(crate) fn panel_has(panel: &Panel<Art>, text: &str) -> bool {
    panel.all_strings().any(|row| row == text)
}

/// The closest any quad comes to the edge of `view` (negative: off screen).
pub(crate) fn clearance(frame: &FrameRecord, view: Rect) -> f32 {
    frame
        .quads()
        .into_iter()
        .map(|quad| {
            let bounds = quad.bounds();
            let gap = (bounds.min - view.min).min(view.max - bounds.max);
            gap.x.min(gap.y)
        })
        .fold(f32::MAX, f32::min)
}

/// Every quad inside `view`, reported against `screen`.
pub(crate) fn on_screen(checks: &mut Checks, screen: &str, frame: &FrameRecord, view: Rect) {
    let off: Vec<Rect> = frame
        .quads()
        .into_iter()
        .map(|quad| quad.bounds())
        .filter(|bounds| !view.contains_rect(*bounds))
        .collect();
    checks.require(
        off.is_empty(),
        "something was drawn off screen",
        format!(
            "{screen}: {} quads outside {view:?}, first {:?}",
            off.len(),
            off.first()
        ),
    );
}
