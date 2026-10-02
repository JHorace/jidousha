//! The turning screen (SPEC §18.1) — W8 SCAFFOLD: only the winter page.
//!
//! "What the winter did": "The winter of year N", then the winter's lines in the order
//! they were written, or "A quiet winter. The house kept to the fire." A page longer
//! than the panel continues on further leaves ("..., continued"). "Go on" turns the
//! leaf; the last reads "Summer comes", which brings the next summer; "Skip ahead"
//! brings it at once (with no death page to decide, nothing holds it back). Pointing at
//! a line opens the sheet of the hero it names first, beside the page in the dock. W8
//! adds the death, birth, coming-of-age, arrival and year pages, and the heir choice.

use jidousha::prelude::*;

use crate::content::Content;
use crate::house::House;
use crate::passage::Passage;
use crate::screen::{MIN_TEXT, PAD, Page, Target, UiState, ink, layers, wrap};
use crate::summer::{button, lay_out_top_bar};
use crate::telling_view::{PANEL, about, text_area};
use crate::text::fmt;
use crate::words::W;

/// The navigation strip along the panel's foot (`telling_view::text_area` stops above it).
const NAV_H: f32 = 48.0;
/// The pitch of 14 px and 16 px type.
const PITCH: f32 = 17.0;
const TITLE_PITCH: f32 = 20.0;
/// Air between lines, and below the page's heading and title.
const LINE_GAP: f32 = 6.0;
const PART_GAP: f32 = 10.0;
/// A navigation button.
const NAV_BUTTON: Vec2 = Vec2::new(36.0, 34.0);
const NAV_WIDE: f32 = 140.0;

/// The winter page's lines: what the winter wrote, or the quiet winter's line.
pub fn winter_lines(content: &Content, passage: &Passage) -> Vec<String> {
    if passage.winter.is_empty() {
        vec![content.words[W::TurningQuietWinter].to_owned()]
    } else {
        passage.winter.clone()
    }
}

/// "The winter of year N".
pub fn winter_title(content: &Content, passage: &Passage) -> String {
    fmt(
        &content.words[W::TurningWinterTitle],
        &[&passage.year.to_string()],
    )
}

fn height(text: &str) -> f32 {
    wrap(text, text_area().size().x, MIN_TEXT).len() as f32 * PITCH
}

/// The winter page's leaves: ranges of its lines, each under the page's heading (the
/// first also under its title).
pub fn leaves(content: &Content, passage: &Passage) -> Vec<std::ops::Range<usize>> {
    let lines = winter_lines(content, passage);
    let room = text_area().size().y;
    let mut out = Vec::new();
    let mut at = 0;
    while at < lines.len() || out.is_empty() {
        let mut used = PITCH
            + PART_GAP
            + if out.is_empty() {
                TITLE_PITCH + PART_GAP
            } else {
                0.0
            };
        let start = at;
        while at < lines.len() && used + height(&lines[at]) <= room {
            used += height(&lines[at]) + LINE_GAP;
            at += 1;
        }
        // INVARIANT: every leaf holds a line, or the page could never end.
        assert!(
            at > start,
            "[keifu] a winter line is taller than a leaf: {:?}\n  likely cause: a very \
             long line of content\n  fix: give the turning's panel more height",
            lines.get(at)
        );
        out.push(start..at);
    }
    out
}

/// Lay the turning out: the bar, the leaf on screen, its navigation, and the dock.
pub fn lay_out(page: &mut Page, content: &Content, house: &House, passage: &Passage, ui: &UiState) {
    let words = &content.words;
    lay_out_top_bar(page, content, house);
    page.shape(PANEL, ink::PANEL, layers::PANEL);
    let leaves = leaves(content, passage);
    let current = ui.leaf.min(leaves.len() - 1);
    let lines = winter_lines(content, passage);
    let area = text_area();
    let heading = if current == 0 {
        words[W::TurningPageWinter].to_owned()
    } else {
        fmt(&words[W::TurningContinued], &[&words[W::TurningPageWinter]])
    };
    let mut y = text(page, &heading, area.min.y, MIN_TEXT, ink::HEADING, None) + PART_GAP;
    if current == 0 {
        y = text(
            page,
            &winter_title(content, passage),
            y,
            16.0,
            ink::BODY,
            None,
        ) + PART_GAP;
    }
    for line in &lines[leaves[current].clone()] {
        y = text(page, line, y, MIN_TEXT, ink::BODY, Some(house)) + LINE_GAP;
    }
    lay_out_nav(page, content, leaves.len(), current);
    crate::dock::lay_out(page, content, house, ui);
}

/// One logical line, wrapped into the text area from `y`; pointing at it points at the
/// hero it names first, when `house` is given. Returns the y below it.
fn text(
    page: &mut Page,
    line: &str,
    y: f32,
    size: f32,
    color: Color,
    house: Option<&House>,
) -> f32 {
    let area = text_area();
    let pitch = if size > MIN_TEXT { TITLE_PITCH } else { PITCH };
    let pieces = wrap(line, area.size().x, size);
    for (index, piece) in pieces.iter().enumerate() {
        let at = Vec2::new(area.min.x, y + index as f32 * pitch);
        if index == 0 {
            page.text(layers::TEXT, at, piece.clone(), size, color, PANEL);
        } else {
            page.continue_text(layers::TEXT, at, piece.clone(), size, color, PANEL);
        }
    }
    let bottom = y + pieces.len() as f32 * pitch;
    if let Some(id) = house.and_then(|house| about(&house.heroes, line)) {
        let rect = Rect::from_min_size(
            Vec2::new(area.min.x, y),
            Vec2::new(area.size().x, bottom - y),
        );
        page.targets.push((rect, Target::Hero(id)));
    }
    bottom
}

/// A numbered button per leaf, "Skip ahead", and "Go on" — "Summer comes" on the last.
fn lay_out_nav(page: &mut Page, content: &Content, count: usize, current: usize) {
    let words = &content.words;
    let top = PANEL.max.y - NAV_H + (NAV_H - NAV_BUTTON.y) * 0.5;
    for leaf in 0..count {
        let rect = Rect::from_min_size(
            Vec2::new(PANEL.min.x + PAD + leaf as f32 * (NAV_BUTTON.x + 6.0), top),
            NAV_BUTTON,
        );
        button(
            page,
            rect,
            &(leaf + 1).to_string(),
            Target::Leaf(leaf),
            layers::PANEL,
        );
        if leaf == current {
            let mark = Rect::from_min_size(
                Vec2::new(rect.min.x, rect.max.y + 2.0),
                Vec2::new(NAV_BUTTON.x, 3.0),
            );
            page.shape(mark, ink::GOLD, layers::MARK);
        }
    }
    let next = if current + 1 < count {
        W::TurningNext
    } else {
        W::TurningLast
    };
    let go_on = Rect::from_min_size(
        Vec2::new(PANEL.max.x - PAD - NAV_WIDE, top),
        Vec2::new(NAV_WIDE, NAV_BUTTON.y),
    );
    button(page, go_on, &words[next], Target::GoOn, layers::PANEL);
    let skip = Rect::from_min_size(
        Vec2::new(go_on.min.x - 12.0 - NAV_WIDE, top),
        Vec2::new(NAV_WIDE, NAV_BUTTON.y),
    );
    button(
        page,
        skip,
        &words[W::TurningSkip],
        Target::Skip,
        layers::PANEL,
    );
}
