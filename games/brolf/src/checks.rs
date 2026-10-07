//! The instrument: a collector for failed checks and the reads every gate is spelled with.
//!
//! Key types: `Checks`. Key functions: `fail`, `near`, `font_quads_in`, `dots_at_radius`,
//! `disc_at`.
//! Depends on: the prelude and the testing surface. Never depended on outside the game.
//! INVARIANT: a failed check does not stop the run, and reports the numbers it judged.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord, find_bounds};

/// Every failed check, kept rather than exited on.
#[derive(Default)]
pub struct Checks {
    problems: Vec<(String, String)>,
}

impl Checks {
    /// Record a failure unless `ok`.
    pub fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if !ok {
            self.problems.push((what.to_owned(), specifics));
        }
    }

    /// How many checks have failed.
    pub fn failures(&self) -> usize {
        self.problems.len()
    }

    /// Print everything that failed in the engine's four-part shape.
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

/// One problem, in the engine's four-part message shape.
fn complaint(what: &str, specifics: &str) -> String {
    message(
        what,
        specifics,
        "the game changed, or the engine did",
        "run `cargo run -p brolf` and watch it, then compare with the assertion above",
    )
}

/// Within `tolerance`, and false when either is NaN.
pub fn within(a: f32, b: f32, tolerance: f32) -> bool {
    matches!(
        (a - b).abs().partial_cmp(&tolerance),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    )
}

/// How many font quads lie wholly in the horizontal strip `y0..=y1`.
pub fn font_quads_in(frame: &FrameRecord, font: BackendTextureId, y0: f32, y1: f32) -> usize {
    frame
        .quads()
        .iter()
        .filter(|quad| quad.texture == font)
        .filter(|quad| {
            let b = quad.bounds();
            b.min.y >= y0 - 1e-3 && b.max.y <= y1 + 1e-3
        })
        .count()
}

/// How many square dots of `side` and `color` have their centre `radius` (within
/// `tolerance`) from `center`.
pub fn dots_at_radius(
    frame: &FrameRecord,
    color: Color,
    side: f32,
    center: Vec2,
    radius: f32,
    tolerance: f32,
) -> usize {
    frame
        .quads()
        .iter()
        .filter(|quad| quad.tint == color)
        .filter(|quad| {
            let size = quad.bounds().size();
            within(size.x, side, 0.01) && within(size.y, side, 0.01)
        })
        .filter(|quad| {
            within(
                (quad.bounds().center() - center).length(),
                radius,
                tolerance,
            )
        })
        .count()
}

/// The box round the disc of `radius` drawn at `center`: what covers the centre and
/// fits in the disc's own bounding box.
pub fn disc_at(frame: &FrameRecord, center: Vec2, radius: f32) -> Option<Vec2> {
    let bound = Rect::from_center_size(center, Vec2::splat(radius * 2.0));
    find_bounds(frame.covering(center).into_iter().filter(|quad| {
        let drawn = quad.bounds();
        drawn.min.x >= bound.min.x - 1e-3
            && drawn.min.y >= bound.min.y - 1e-3
            && drawn.max.x <= bound.max.x + 1e-3
            && drawn.max.y <= bound.max.y + 1e-3
    }))
    .map(|rect| rect.size())
}

/// The box round the font quads of height `height` lying in the strip `y0..=y1`.
pub fn font_bounds_in(
    frame: &FrameRecord,
    font: BackendTextureId,
    height: f32,
    y0: f32,
    y1: f32,
) -> Option<Rect> {
    find_bounds(
        frame
            .quads()
            .into_iter()
            .filter(|quad| quad.texture == font)
            .filter(|quad| {
                let b = quad.bounds();
                within(b.size().y, height, 1e-3) && b.min.y >= y0 - 1e-3 && b.max.y <= y1 + 1e-3
            }),
    )
}
