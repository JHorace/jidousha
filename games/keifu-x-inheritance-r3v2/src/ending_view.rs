//! The Ending's verdict page (SPEC §23, `scene/scenes/ending.jai`): after the Door, its
//! title and text by the locks that gave and one line per member of the party; once the
//! house closed, the closed title and verdict and "It was year Y, with the Door still R
//! years off."; then "N lived under this roof." with the house tally. Two controls: "The
//! family" — the tree, every hero remembered by an epitaph, with "The verdict" to come
//! back (`tree.rs`) — and "Begin another house".
//!
//! There is no score.

use jidousha::prelude::*;

use crate::content::Content;
use crate::ending::{Ending, Verdict, lived_here};
use crate::house::House;
use crate::screen::{MIN_TEXT, PAD, PAGE_H, PAGE_W, Page, Target, ink, layers, wrap};
use crate::summer::button;
use crate::text::fmt;
use crate::words::W;

/// The verdict's panel, centred.
pub const PANEL: Rect = Rect {
    min: Vec2::new(PAGE_W * 0.5 - 400.0, 64.0),
    max: Vec2::new(PAGE_W * 0.5 + 400.0, PAGE_H - 64.0),
};
/// "The family".
pub const FAMILY_BUTTON: Rect = Rect {
    min: Vec2::new(PAGE_W * 0.5 - 236.0, PANEL.max.y - 58.0),
    max: Vec2::new(PAGE_W * 0.5 - 16.0, PANEL.max.y - 22.0),
};
/// "Begin another house".
pub const AGAIN_BUTTON: Rect = Rect {
    min: Vec2::new(PAGE_W * 0.5 + 16.0, PANEL.max.y - 58.0),
    max: Vec2::new(PAGE_W * 0.5 + 236.0, PANEL.max.y - 22.0),
};
/// The title's type and pitch; the body's pitch; the air between paragraphs.
const TITLE: f32 = 22.0;
const TITLE_PITCH: f32 = 30.0;
const BODY: f32 = 16.0;
const BODY_PITCH: f32 = 20.0;
const PITCH: f32 = 18.0;
const GAP: f32 = 12.0;

/// How a paragraph of the verdict is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// The verdict's title.
    Title,
    /// The verdict's text.
    Body,
    /// A party's line, the closed house's year, the tally.
    Note,
}

/// The verdict page's paragraphs, top to bottom: the title, then the body.
pub fn paragraphs(content: &Content, house: &House, ending: &Ending) -> Vec<(String, Style)> {
    let mut out = vec![(ending.title(content).to_owned(), Style::Title)];
    match &ending.verdict {
        Verdict::Door { locks, lines } => {
            out.push((content.door.verdicts[*locks].clone(), Style::Body));
            out.extend(lines.iter().map(|line| (line.clone(), Style::Note)));
        }
        Verdict::Closed { year, years_off } => {
            out.push((content.door.closed_verdict.clone(), Style::Body));
            out.push((
                fmt(
                    &content.words[W::EndingClosedWhen],
                    &[&year.to_string(), &years_off.to_string()],
                ),
                Style::Note,
            ));
        }
    }
    out.push((lived_here(content, house), Style::Note));
    out
}

/// Lay the verdict page out.
pub fn lay_out(page: &mut Page, content: &Content, house: &House, ending: &Ending) {
    page.shape(PANEL, ink::PANEL, layers::PANEL);
    let width = PANEL.size().x - 2.0 * PAD * 2.0;
    let x = PANEL.min.x + PAD * 2.0;
    let mut y = PANEL.min.y + PAD * 2.0;
    for (index, (text, style)) in paragraphs(content, house, ending).into_iter().enumerate() {
        let (size, pitch, color) = match style {
            Style::Title => (TITLE, TITLE_PITCH, ink::HEADING),
            Style::Body => (BODY, BODY_PITCH, ink::BODY),
            Style::Note => (MIN_TEXT, PITCH, ink::NOTE),
        };
        for (piece_index, piece) in wrap(&text, width, size).into_iter().enumerate() {
            let at = Vec2::new(x, y);
            if piece_index == 0 {
                page.text(layers::TEXT, at, piece, size, color, PANEL);
            } else {
                page.continue_text(layers::TEXT, at, piece, size, color, PANEL);
            }
            y += pitch;
        }
        y += if index == 0 { 4.0 } else { GAP };
    }
    button(
        page,
        FAMILY_BUTTON,
        &content.words[W::EndingTree],
        Target::OpenFamily,
        layers::PANEL,
    );
    button(
        page,
        AGAIN_BUTTON,
        &content.words[W::EndingAgain],
        Target::BeginAgain,
        layers::PANEL,
    );
}
