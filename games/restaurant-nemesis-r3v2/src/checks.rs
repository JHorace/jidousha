//! The instrument: failed checks kept rather than exited on, summary lines,
//! and the one look every judged frame gets — floors, frame, glyph floor,
//! ASCII, bounds, clear colour (DESIGN.md G2, G3).
//!
//! Key items: `Checks`, `look`, `FLOORS`, `OVERLAYS`, `panel_has`.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord};
use jidousha::ui::{Floors, Panel, frame_text_floor, judge_frame, judge_panel};

use crate::players::Session;
use crate::screen::{Art, Flat};
use crate::{camera, palette};

/// Every failed check, and one summary line per thing worth saying.
#[derive(Default)]
pub(crate) struct Checks {
    problems: Vec<(String, String)>,
    pub(crate) summary: Vec<String>,
    /// How many checks ran, failed or not.
    pub(crate) count: usize,
    /// Frames judged.
    pub(crate) frames: usize,
    /// The closest any judged quad came to the edge.
    pub(crate) clearance: f32,
}

impl Checks {
    pub(crate) fn new() -> Self {
        Self {
            clearance: f32::MAX,
            ..Self::default()
        }
    }

    pub(crate) fn require(&mut self, ok: bool, what: &str, specifics: String) {
        self.count += 1;
        if !ok {
            self.problems.push((what.to_owned(), specifics));
        }
    }

    pub(crate) fn note(&mut self, line: String) {
        self.summary.push(line);
    }

    pub(crate) fn failed(&self) -> usize {
        self.problems.len()
    }

    pub(crate) fn verdict(&self) -> ExitCode {
        if self.problems.is_empty() {
            return ExitCode::SUCCESS;
        }
        for (what, specifics) in &self.problems {
            eprintln!(
                "{}",
                message(
                    what,
                    specifics,
                    "the game changed, or the engine did",
                    "run `cargo run -p restaurant_nemesis_r3v2` and play a day, then compare \
                     with the assertion above",
                )
            );
        }
        ExitCode::FAILURE
    }
}

/// Text no smaller than 12; everything inside the 960 x 540 room.
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

/// The one overlay: (its name, its exact title row).
pub(crate) const OVERLAYS: &[(&str, &str)] = &[("NEMESIS", "NEMESIS CARD")];

/// Whether some row of the panel contains `text`.
pub(crate) fn panel_has(panel: &Panel<Art>, text: &str) -> bool {
    panel.all_strings().any(|row| row.contains(text))
}

/// Draw what the session shows now and judge it as `name`; hands both back.
pub(crate) fn look(
    checks: &mut Checks,
    name: &str,
    session: &mut Session,
) -> (FrameRecord, Panel<Art>) {
    let (frame, panel) = session.frame();
    judge(checks, name, &panel, &frame, session.font());
    (frame, panel)
}

/// Every floor, the frame, the bounds and the clear colour, over one frame.
pub(crate) fn judge(
    checks: &mut Checks,
    name: &str,
    panel: &Panel<Art>,
    frame: &FrameRecord,
    font: BackendTextureId,
) {
    checks.frames += 1;
    let view = camera().visible_bounds();
    let mut breaches = judge_panel(panel, &FLOORS, &[], OVERLAYS);
    breaches.extend(judge_frame(panel, frame, font, &Flat, view));
    breaches.extend(frame_text_floor(frame, font, FLOORS.min_text));
    let listed: Vec<String> = breaches
        .iter()
        .map(|b| format!("{} - {}", b.what, b.detail))
        .collect();
    checks.require(
        listed.is_empty(),
        "a screen breaks its floors or does not show what it says",
        format!("{name}: {listed:?}"),
    );
    let odd: Vec<&str> = panel
        .all_strings()
        .filter(|row| !row.chars().all(|c| (' '..='~').contains(&c)))
        .collect();
    checks.require(
        odd.is_empty(),
        "a string the font cannot draw",
        format!("{name}: {odd:?}"),
    );
    let mut off = 0;
    for quad in frame.quads() {
        let bounds = quad.bounds();
        if !view.contains_rect(bounds) {
            off += 1;
        }
        let gap = (bounds.min - view.min).min(view.max - bounds.max);
        checks.clearance = checks.clearance.min(gap.x.min(gap.y));
    }
    checks.require(
        off == 0,
        "something was drawn off screen",
        format!("{name}: {off} quads outside {view:?}"),
    );
    let clear = frame.plan.clear_color;
    let brightest = clear.r.max(clear.g).max(clear.b);
    checks.require(
        clear == palette::INK && brightest < 0.25 && clear.a > 0.99,
        "the room is not dark enough for its text",
        format!("{name}: clear colour {clear:?}, brightest channel {brightest:.3}"),
    );
}
