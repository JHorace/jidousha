//! The summer screen: the top bar, the household, the yard, the board (W4,
//! `board_view.rs`), and the sheet of whatever is pointed at (SPEC §5.4, §19.1).
//!
//! The board fills the right half; the hero sheet and the quest sheet are raised
//! over it, and the help line stands in whatever the board leaves empty.

use jidousha::prelude::*;

use crate::art::{Figure, figure_named, hero_figure};
use crate::board::Slot;
use crate::board_view::{draw_hand, draw_quest_sheet, lay_out_board, sheet_raised};
use crate::constants::{DREAD_LIMIT, ROSTER_SEATS, YARD_SPOTS};
use crate::content::Content;
use crate::family::top_bar;
use crate::hero::{Fate, Hero, HeroId};
use crate::house::House;
use crate::ids::Phase;
use crate::screen::{MIN_TEXT, PAD, PAGE_H, PAGE_W, Page, Target, UiState, ink, layers, wrap};
use crate::sheet::{Ink, hero_sheet};
use crate::words::W;

/// The top bar's height.
pub const TOP_H: f32 = 76.0;
/// Where the household and yard column starts.
pub const LEFT_X: f32 = 24.0;
/// A hero card.
pub const CARD: Vec2 = Vec2::new(148.0, 84.0);
/// The gap between cards.
pub const CARD_GAP: f32 = 8.0;
/// Cards per row (CONSTANTS §14: 12 seats, 3 columns).
pub const COLUMNS: usize = 3;
/// Where the roster's first card sits.
pub const ROSTER_TOP: f32 = 112.0;
/// The sheet panel.
pub const SHEET: Rect = Rect {
    min: Vec2::new(500.0, 88.0),
    max: Vec2::new(PAGE_W - 24.0, PAGE_H - 8.0),
};
/// "The family" button.
pub const FAMILY_BUTTON: Rect = Rect {
    min: Vec2::new(PAGE_W - 184.0, 18.0),
    max: Vec2::new(PAGE_W - 24.0, 54.0),
};
/// The sheet's type size, and its line pitch.
const SHEET_SIZE: f32 = MIN_TEXT;
const SHEET_PITCH: f32 = 17.0;
/// The heirloom's sprite on the sheet: 16 px art at 2x.
const HEIRLOOM_FIGURE: f32 = 32.0;

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

/// Where the yard's label and first card sit: below the roster's four rows.
pub fn yard_top() -> f32 {
    let rows = ROSTER_SEATS.div_ceil(COLUMNS) as f32;
    ROSTER_TOP + rows * (CARD.y + CARD_GAP) + 26.0
}

/// The whole screen rectangle.
pub fn screen_rect() -> Rect {
    Rect::from_min_size(Vec2::ZERO, Vec2::new(PAGE_W, PAGE_H))
}

/// Lay the summer screen out.
pub fn lay_out(page: &mut Page, content: &Content, house: &House, ui: &UiState) {
    let words = &content.words;
    let top = Rect::from_min_size(Vec2::ZERO, Vec2::new(PAGE_W, TOP_H));
    page.shape(top, ink::PANEL, layers::PANEL);
    let [year, season, renown, door, door_tags] = top_bar(content, house);
    let style = TextStyle {
        size: 18.0,
        ..TextStyle::default()
    };
    let mut x = LEFT_X;
    for (text, color) in [
        (year, ink::BODY),
        (season, ink::BODY),
        (renown.clone(), renown_ink(house)),
    ] {
        let width = style.width_of(&text);
        page.text(layers::TEXT, Vec2::new(x, 10.0), text, 18.0, color, top);
        x += width + 32.0;
    }
    page.text(
        layers::TEXT,
        Vec2::new(LEFT_X, 34.0),
        door,
        MIN_TEXT,
        ink::NOTE,
        top,
    );
    page.text(
        layers::TEXT,
        Vec2::new(LEFT_X, 52.0),
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
        Vec2::new(LEFT_X, 90.0),
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
        Vec2::new(LEFT_X, yard - 22.0),
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

    // The board fills the right panel; a sheet is raised over it while a hero or a
    // quest is pointed at and nothing is in hand.
    if !ui.family_open && sheet_raised(ui) {
        page.shape(SHEET, ink::PANEL, layers::PANEL);
        match (ui.pointing, ui.pointing_quest) {
            (Some(id), _) => sheet(page, content, &house.heroes, id),
            (None, Some(quest)) => draw_quest_sheet(page, content, house, quest),
            (None, None) => {}
        }
    }
    lay_out_board(page, content, house, ui);
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

/// A hero card: figure, name, age, a pip per point of dread, a gold pip if settled.
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
    let ground = rect.min + Vec2::new(4.0 + FIGURE * 0.5, 18.0 + FIGURE);
    let figure = Rect::from_min_size(ground - Vec2::new(size * 0.5, size), Vec2::splat(size));
    page.figure(
        figure,
        hero_figure(content, hero),
        figure_tint(hero),
        layers::MARK,
    );
    let text_x = rect.min.x + 58.0;
    page.text(
        layers::TEXT,
        Vec2::new(text_x, rect.min.y + 10.0),
        hero.name.clone(),
        16.0,
        ink::BODY,
        rect,
    );
    page.text(
        layers::TEXT,
        Vec2::new(text_x, rect.min.y + 34.0),
        hero.age.to_string(),
        MIN_TEXT,
        ink::NOTE,
        rect,
    );
    let pips_y = rect.min.y + 60.0;
    for pip in 0..hero.fear.dread.min(DREAD_LIMIT) {
        let at = Vec2::new(text_x + pip as f32 * 14.0, pips_y);
        page.shape(
            Rect::from_min_size(at, Vec2::splat(10.0)),
            ink::DREAD,
            layers::MARK,
        );
    }
    if hero.settled {
        let at = Vec2::new(rect.max.x - 22.0, pips_y);
        page.shape(
            Rect::from_min_size(at, Vec2::splat(10.0)),
            ink::GOLD,
            layers::MARK,
        );
    }
    page.targets.push((rect, Target::Hero(id)));
}

/// The hero sheet, in the sheet panel: two columns, DESTINY onward in the second.
fn sheet(page: &mut Page, content: &Content, heroes: &[Hero], id: HeroId) {
    let sheet = hero_sheet(content, heroes, id);
    let column = (SHEET.size().x - 3.0 * PAD) * 0.5;
    let top = SHEET.min.y + PAD;
    let mut x = SHEET.min.x + PAD;
    let mut y = top;
    for (index, line) in sheet.lines.iter().enumerate() {
        if index == sheet.second_column {
            x += column + PAD;
            y = top;
        }
        let (size, color) = match line.ink {
            Ink::Name => (20.0, ink::BODY),
            Ink::Heading => (SHEET_SIZE, ink::HEADING),
            Ink::Body | Ink::Current => (SHEET_SIZE, ink::BODY),
            Ink::Note | Ink::Upcoming => (SHEET_SIZE, ink::NOTE),
            Ink::Warning => (SHEET_SIZE, ink::WARN),
            Ink::Done | Ink::Gone => (SHEET_SIZE, ink::GONE),
        };
        if line.ink == Ink::Heading && y > top {
            y += 6.0;
        }
        // The heirloom's sprite, at 2x, at the right of its heading's row.
        if line.ink == Ink::Heading
            && line.text == content.words[W::SheetHeirloom]
            && let Some(heirloom) = &heroes[id].heirloom
        {
            let figure: Figure = match figure_named(&heirloom.sprite) {
                Some(figure) => figure,
                None => panic!(
                    "[keifu] the heirloom {} is drawn with {:?}, which no imported sprite \
                     plays\n  fix: import one (art/import_sprites.py) and add its role",
                    heirloom.name, heirloom.sprite
                ),
            };
            let at = Vec2::new(x + column - HEIRLOOM_FIGURE, y);
            page.figure(
                Rect::from_min_size(at, Vec2::splat(HEIRLOOM_FIGURE)),
                figure,
                Color::WHITE,
                layers::MARK,
            );
        }
        // Stage marks, so done / current / upcoming read without colour as well.
        let text = match line.ink {
            Ink::Done => format!("[x] {}", line.text),
            Ink::Current => format!("[>] {}", line.text),
            Ink::Upcoming => format!("[ ] {}", line.text),
            _ => line.text.clone(),
        };
        let pips = line.pips.map_or(0.0, |(_, of)| 12.0 + of as f32 * 14.0);
        for (piece_index, piece) in wrap(&text, column - pips, size).into_iter().enumerate() {
            let style = TextStyle {
                size,
                ..TextStyle::default()
            };
            let end = x + style.width_of(&piece);
            if piece_index == 0 {
                page.text(layers::TEXT, Vec2::new(x, y), piece, size, color, SHEET);
            } else {
                page.continue_text(layers::TEXT, Vec2::new(x, y), piece, size, color, SHEET);
            }
            if let Some((filled, of)) = line.pips {
                for pip in 0..of {
                    let at = Vec2::new(end + 12.0 + pip as f32 * 14.0, y + 2.0);
                    let color = if pip < filled {
                        ink::DREAD
                    } else {
                        ink::PIP_EMPTY
                    };
                    page.shape(
                        Rect::from_min_size(at, Vec2::splat(10.0)),
                        color,
                        layers::MARK,
                    );
                }
            }
            y += if size > SHEET_SIZE {
                size + 6.0
            } else {
                SHEET_PITCH
            };
        }
    }
}
