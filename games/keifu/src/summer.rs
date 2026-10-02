//! The summer screen: the top bar, the household, the yard, the board (W4,
//! `board_view.rs`), and the sheet dock (`dock.rs`) with the open sheet in it (SPEC
//! §5.4, §19.1).
//!
//! Four regions that never overlap, left to right: the household and the yard; the
//! board, its four quest cards in a 2x2 grid; and the sheet dock, the full height of
//! the screen at its right edge. The top bar runs over the first two. Nothing is
//! raised over anything, so with a sheet open — and mid-drag, with the held hero's
//! sheet in the dock — the roster, every card's live preview and the hand all show.

use jidousha::prelude::*;

use crate::art::hero_figure;
use crate::board::Slot;
use crate::board_view::{draw_hand, lay_out_board};
use crate::constants::{DREAD_LIMIT, ROSTER_SEATS, YARD_SPOTS};
use crate::content::Content;
use crate::family::top_bar;
use crate::hero::{Fate, Hero, HeroId};
use crate::house::House;
use crate::ids::Phase;
use crate::screen::{MIN_TEXT, PAGE_H, PAGE_W, Page, Target, UiState, ink, layers};
use crate::words::W;

/// The top bar's height.
pub const TOP_H: f32 = 58.0;
/// Where the household and yard column starts.
pub const LEFT_X: f32 = 16.0;
/// A hero card: the figure, the name under it, the dread pips under that.
pub const CARD: Vec2 = Vec2::new(84.0, 84.0);
/// The gap between cards.
pub const CARD_GAP: f32 = 4.0;
/// Cards per row (CONSTANTS §14: 12 seats, 3 columns).
pub const COLUMNS: usize = 3;
/// Where the roster's first card sits: under the bar and the household's label.
pub const ROSTER_TOP: f32 = TOP_H + 26.0;
/// The gap between the three regions.
const GUTTER: f32 = 12.0;
/// The sheet dock, down the right edge, the screen's full height (`dock.rs`).
pub const SHEET: Rect = Rect {
    min: Vec2::new(PAGE_W - 344.0, 0.0),
    max: Vec2::new(PAGE_W, PAGE_H),
};
/// The top bar: over the household and the board, short of the dock.
pub const TOP_BAR: Rect = Rect {
    min: Vec2::ZERO,
    max: Vec2::new(SHEET.min.x - 8.0, TOP_H),
};
/// The board: right of the household, left of the dock, under the bar.
pub const BOARD: Rect = Rect {
    min: Vec2::new(
        LEFT_X + COLUMNS as f32 * (CARD.x + CARD_GAP) - CARD_GAP + GUTTER,
        TOP_H + 8.0,
    ),
    max: Vec2::new(SHEET.min.x - GUTTER, PAGE_H - 8.0),
};
/// "The family" button, at the bar's right end.
pub const FAMILY_BUTTON: Rect = Rect {
    min: Vec2::new(TOP_BAR.max.x - 152.0, 12.0),
    max: Vec2::new(TOP_BAR.max.x - 12.0, 46.0),
};

/// Where card `slot` of a grid starting at `top` sits.
pub fn card_rect(top: f32, slot: usize) -> Rect {
    let column = (slot % COLUMNS) as f32;
    let row = (slot / COLUMNS) as f32;
    Rect::from_min_size(
        Vec2::new(
            LEFT_X + column * (CARD.x + CARD_GAP),
            top + row * (CARD.y + CARD_GAP),
        ),
        CARD,
    )
}

/// Where the yard's first card sits: below the roster's four rows and its label.
pub fn yard_top() -> f32 {
    let rows = ROSTER_SEATS.div_ceil(COLUMNS) as f32;
    ROSTER_TOP + rows * (CARD.y + CARD_GAP) - CARD_GAP + 26.0
}

/// The whole screen rectangle.
pub fn screen_rect() -> Rect {
    Rect::from_min_size(Vec2::ZERO, Vec2::new(PAGE_W, PAGE_H))
}

/// Lay the summer screen out.
pub fn lay_out(page: &mut Page, content: &Content, house: &House, ui: &UiState) {
    let words = &content.words;
    let top = TOP_BAR;
    page.shape(top, ink::PANEL, layers::PANEL);
    let [year, season, renown, door, door_tags] = top_bar(content, house);
    let style = TextStyle {
        size: 16.0,
        ..TextStyle::default()
    };
    let mut x = LEFT_X;
    for (text, color) in [
        (year, ink::BODY),
        (season, ink::BODY),
        (renown.clone(), renown_ink(house)),
    ] {
        let width = style.width_of(&text);
        page.text(layers::TEXT, Vec2::new(x, 6.0), text, 16.0, color, top);
        x += width + 28.0;
    }
    page.text(
        layers::TEXT,
        Vec2::new(LEFT_X, 25.0),
        door,
        MIN_TEXT,
        ink::NOTE,
        top,
    );
    page.text(
        layers::TEXT,
        Vec2::new(LEFT_X, 41.0),
        door_tags,
        MIN_TEXT,
        ink::NOTE,
        top,
    );
    button(
        page,
        FAMILY_BUTTON,
        &words[W::FamilyOpen],
        Target::OpenFamily,
        layers::PANEL,
    );

    let screen = screen_rect();
    page.text(
        layers::TEXT,
        Vec2::new(LEFT_X, ROSTER_TOP - 20.0),
        &words[W::SummerHousehold],
        MIN_TEXT,
        ink::HEADING,
        screen,
    );
    let in_hand = ui.drag.map(|drag| drag.hero);
    for slot in 0..ROSTER_SEATS {
        let rect = card_rect(ROSTER_TOP, slot);
        match house.roster[slot] {
            // The hero in hand has left their seat until they are released.
            Some(id) if in_hand != Some(id) => hero_card(
                page,
                content,
                &house.heroes,
                id,
                rect,
                ui.pointing == Some(id),
            ),
            _ => {
                page.shape(rect, ink::PANEL, layers::PANEL);
                page.targets.push((rect, Target::Seat(Slot::Roster(slot))));
            }
        }
    }
    let yard = yard_top();
    page.text(
        layers::TEXT,
        Vec2::new(LEFT_X, yard - 20.0),
        &words[W::SummerYard],
        MIN_TEXT,
        ink::HEADING,
        screen,
    );
    let children = house.yard();
    if children.is_empty() {
        page.text(
            layers::TEXT,
            Vec2::new(LEFT_X, yard),
            &words[W::SummerNoChildren],
            MIN_TEXT,
            ink::NOTE,
            screen,
        );
    }
    for (slot, id) in children.iter().enumerate().take(YARD_SPOTS) {
        let rect = card_rect(yard, slot);
        hero_card(
            page,
            content,
            &house.heroes,
            *id,
            rect,
            ui.pointing == Some(*id),
        );
    }

    lay_out_board(page, content, house, ui);
    crate::dock::lay_out(page, content, house, ui);
    draw_hand(page, content, house, ui);
}

/// Renown is drawn red at 4 or below (CONSTANTS §14).
fn renown_ink(house: &House) -> Color {
    if house.renown <= 4 {
        ink::WARN
    } else {
        ink::BODY
    }
}

/// A labelled button.
pub fn button(page: &mut Page, rect: Rect, label: &str, target: Target, layer: i16) {
    page.shape(rect, ink::HOT, layer);
    let style = TextStyle {
        size: MIN_TEXT,
        ..TextStyle::default()
    };
    let at = Vec2::new(
        rect.center().x - style.width_of(label) * 0.5,
        rect.center().y - MIN_TEXT * 0.5,
    );
    page.text(layer + 2, at, label, MIN_TEXT, ink::BODY, rect);
    page.targets.push((rect, target));
}

/// The tint the original puts on a hero's sprite (SPEC §5.4): dead grey, wounded red,
/// elder grey; otherwise none. (Session 1's stand-in rectangle took its vocation's
/// aptitude colour in the "otherwise" case; a sprite carries its own colours.)
pub fn figure_tint(hero: &Hero) -> Color {
    if hero.fate == Fate::Dead || hero.phase() == Phase::Elder {
        ink::GONE
    } else if hero.wounded {
        ink::WARN
    } else {
        Color::WHITE
    }
}

/// A hero's sprite on a card: 16 px art at 3x; a child's at 2x, standing on the
/// same ground line, so a child reads smaller than the grown.
const FIGURE: f32 = 48.0;
const CHILD_FIGURE: f32 = 32.0;

/// A hero card: the figure with the age beside it (and a gold pip if settled), the
/// name under it, and a pip per point of dread under that.
fn hero_card(
    page: &mut Page,
    content: &Content,
    heroes: &[Hero],
    id: HeroId,
    rect: Rect,
    hot: bool,
) {
    let hero = &heroes[id];
    page.shape(rect, if hot { ink::HOT } else { ink::PANEL }, layers::PANEL);
    let size = if hero.phase() == Phase::Child {
        CHILD_FIGURE
    } else {
        FIGURE
    };
    let ground = rect.min + Vec2::new(4.0 + FIGURE * 0.5, 4.0 + FIGURE);
    let figure = Rect::from_min_size(ground - Vec2::new(size * 0.5, size), Vec2::splat(size));
    page.figure(
        figure,
        hero_figure(content, hero),
        figure_tint(hero),
        layers::MARK,
    );
    let beside = rect.min.x + 8.0 + FIGURE;
    page.text(
        layers::TEXT,
        Vec2::new(beside, rect.min.y + 8.0),
        hero.age.to_string(),
        MIN_TEXT,
        ink::NOTE,
        rect,
    );
    if hero.settled {
        let at = Vec2::new(beside + 2.0, rect.min.y + 30.0);
        page.shape(
            Rect::from_min_size(at, Vec2::splat(10.0)),
            ink::GOLD,
            layers::MARK,
        );
    }
    let style = TextStyle {
        size: MIN_TEXT,
        ..TextStyle::default()
    };
    let name_x = rect.center().x - style.width_of(&hero.name) * 0.5;
    page.text(
        layers::TEXT,
        Vec2::new(name_x, rect.min.y + 8.0 + FIGURE),
        hero.name.clone(),
        MIN_TEXT,
        ink::BODY,
        rect,
    );
    let pips_y = rect.min.y + 8.0 + FIGURE + MIN_TEXT + 3.0;
    for pip in 0..hero.fear.dread.min(DREAD_LIMIT) {
        let at = Vec2::new(rect.min.x + 6.0 + pip as f32 * 14.0, pips_y);
        page.shape(
            Rect::from_min_size(at, Vec2::splat(10.0)),
            ink::DREAD,
            layers::MARK,
        );
    }
    page.targets.push((rect, Target::Hero(id)));
}
