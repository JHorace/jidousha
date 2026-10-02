//! W10 SCAFFOLD: the house closed (SPEC §2.1, §23). The Ending is W10's; until it lands,
//! a house whose renown was spent when its telling was left shows the closed verdict —
//! `door.closed_title`, `door.closed_verdict` and "It was year Y, with the Door still
//! R years off." — and "Begin another house", so a closed house is not a dead end.
//! W10 replaces this file whole (the family tree, the epitaphs, the tally).

use jidousha::prelude::*;

use crate::content::Content;
use crate::house::House;
use crate::screen::{MIN_TEXT, PAD, PAGE_H, PAGE_W, Page, Target, ink, layers, wrap};
use crate::summer::button;
use crate::text::fmt;
use crate::words::W;

/// The verdict's panel, centred.
pub const PANEL: Rect = Rect {
    min: Vec2::new(PAGE_W * 0.5 - 360.0, PAGE_H * 0.5 - 200.0),
    max: Vec2::new(PAGE_W * 0.5 + 360.0, PAGE_H * 0.5 + 200.0),
};
/// "Begin another house".
pub const AGAIN_BUTTON: Rect = Rect {
    min: Vec2::new(PAGE_W * 0.5 - 110.0, PANEL.max.y - 60.0),
    max: Vec2::new(PAGE_W * 0.5 + 110.0, PANEL.max.y - 24.0),
};

/// Lay the closed house's verdict out.
pub fn lay_out(page: &mut Page, content: &Content, house: &House) {
    page.shape(PANEL, ink::PANEL, layers::PANEL);
    let width = PANEL.size().x - 2.0 * PAD;
    let mut y = PANEL.min.y + PAD;
    let [title, verdict] = &content.door_closed;
    page.text(
        layers::TEXT,
        Vec2::new(PANEL.min.x + PAD, y),
        title.clone(),
        16.0,
        ink::HEADING,
        PANEL,
    );
    y += 30.0;
    let when = fmt(
        &content.words[W::EndingClosedWhen],
        &[
            &house.calendar.current_year().to_string(),
            &house.calendar.years_until_door().to_string(),
        ],
    );
    for paragraph in [verdict.as_str(), when.as_str()] {
        for (index, piece) in wrap(paragraph, width, MIN_TEXT).into_iter().enumerate() {
            let at = Vec2::new(PANEL.min.x + PAD, y);
            if index == 0 {
                page.text(layers::TEXT, at, piece, MIN_TEXT, ink::BODY, PANEL);
            } else {
                page.continue_text(layers::TEXT, at, piece, MIN_TEXT, ink::BODY, PANEL);
            }
            y += 17.0;
        }
        y += 10.0;
    }
    button(
        page,
        AGAIN_BUTTON,
        &content.words[W::EndingAgain],
        Target::BeginAgain,
        layers::PANEL,
    );
}
