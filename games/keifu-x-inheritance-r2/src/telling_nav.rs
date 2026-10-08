//! The telling's navigation strip (SPEC §8): a numbered button per leaf, "Skip ahead", and
//! "Go on" — which on the last leaf reads "Winter comes", "After the Door" after the Door,
//! or "The last of it" if the house has closed.

use jidousha::prelude::*;

use crate::content::Content;
use crate::house::House;
use crate::screen::{PAD, Page, Target, ink, layers};
use crate::summer::button;
use crate::telling::Telling;
use crate::telling_view::PANEL;
use crate::words::W;

/// The navigation strip along the panel's foot.
pub const NAV_H: f32 = 48.0;
/// A navigation button, and the two wide ones ("After the Door" is the longest label).
const NAV_BUTTON: Vec2 = Vec2::new(36.0, 34.0);
const NAV_WIDE: f32 = 164.0;

/// The navigation strip: a numbered button per leaf, "Skip ahead", and "Go on" — on the
/// last leaf "Winter comes", "After the Door" after the Door, or "The last of it" if the
/// house has closed (SPEC §8).
pub fn lay_out_nav(
    page: &mut Page,
    content: &Content,
    house: &House,
    telling: &Telling,
    count: usize,
    current: usize,
) {
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
        W::TellingNext
    } else if telling.door.is_some() {
        W::TellingAfterDoor
    } else if house.renown <= 0 {
        W::TellingClosed
    } else {
        W::TellingToWinter
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
        &words[W::TellingSkip],
        Target::Skip,
        layers::PANEL,
    );
}
