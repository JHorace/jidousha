//! The board on the summer screen: up to four quest cards in a 2x2 grid in the
//! right panel, each with its seats as tiles along its foot; the quest sheet and
//! its history panel; and the hero in hand.
//!
//! The right panel holds the board, and a sheet is raised over it while a hero
//! or a quest is pointed at and nothing is in hand (the original's sheets are
//! pop-ups too). The board's targets stay live under a raised sheet, so the
//! pointer that raised it keeps it raised; a drag lowers it, so the card a hero
//! is held over is always on screen.

use jidousha::prelude::*;

use crate::art::hero_figure;
use crate::board::Slot;
use crate::constants::{DANGER_LIMIT, QUEST_COUNT};
use crate::content::Content;
use crate::hero::HeroId;
use crate::house::House;
use crate::quest_card::{CardReading, preview_seats, read_card};
use crate::quest_sheet::{Ink, quest_sheet};
use crate::screen::{MIN_TEXT, PAD, Page, Target, UiState, ink, layers, wrap};
use crate::summer::{SHEET, figure_tint};
use crate::words::W;

/// The gap between cards.
const GAP: f32 = 8.0;
/// A seat tile: a figure over a name.
pub const TILE: Vec2 = Vec2::new(84.0, 60.0);
/// The type's line pitch.
const PITCH: f32 = 17.0;
/// The figure on a tile, and in hand: 16 px art at 2x and 3x.
const TILE_FIGURE: f32 = 32.0;
const HAND_FIGURE: f32 = 48.0;

/// Where board slot `slot` sits: a 2x2 grid filling the right panel.
pub fn quest_rect(slot: usize) -> Rect {
    let size = Vec2::new((SHEET.size().x - GAP) * 0.5, (SHEET.size().y - GAP) * 0.5);
    let at = Vec2::new((slot % 2) as f32, (slot / 2) as f32) * (size + Vec2::splat(GAP));
    Rect::from_min_size(SHEET.min + at, size)
}

/// Where seat `seat` of the card at `card` sits: a row along its foot.
pub fn seat_rect(card: Rect, seat: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(
            card.min.x + PAD + seat as f32 * (TILE.x + GAP),
            card.max.y - PAD - TILE.y,
        ),
        TILE,
    )
}

/// What a drag hands the card reading: who is in hand, and where they would land.
pub fn hand(house: &House, ui: &UiState) -> Option<(HeroId, Option<Slot>)> {
    let drag = ui.drag?;
    Some((drag.hero, house.landing(drag.hero, drag.over)))
}

/// Whether a sheet is raised over the board.
pub fn sheet_raised(ui: &UiState) -> bool {
    ui.drag.is_none() && (ui.pointing.is_some() || ui.pointing_quest.is_some())
}

/// Lay the board out: its targets always, its cards unless a sheet covers them.
pub fn lay_out_board(page: &mut Page, content: &Content, house: &House, ui: &UiState) {
    let draw = !sheet_raised(ui);
    let hand = hand(house, ui);
    let watched = ui.drag.map(|d| d.hero).or(ui.pointing);
    for quest in 0..house.board.len() {
        let card = quest_rect(quest);
        let seats = preview_seats(house, quest, hand);
        let party: Vec<HeroId> = seats.iter().flatten().copied().collect();
        for (seat, shown) in seats.iter().enumerate() {
            let slot = Slot::Quest { quest, seat };
            let rect = seat_rect(card, seat);
            match *shown {
                Some(id) => {
                    if draw {
                        tile(page, content, house, id, rect);
                    }
                    match house.hero_in(slot) {
                        Some(seated) if ui.drag.map(|d| d.hero) != Some(seated) => {
                            page.targets.push((rect, Target::Hero(seated)))
                        }
                        _ => page.targets.push((rect, Target::Seat(slot))),
                    }
                }
                None => {
                    if draw {
                        page.shape(rect, ink::PAGE, layers::MARK);
                    }
                    page.targets.push((rect, Target::Seat(slot)));
                }
            }
        }
        if draw {
            let landing_here = matches!(
                hand.and_then(|h| h.1),
                Some(Slot::Quest { quest: q, .. }) if q == quest
            );
            let hot = ui.pointing_quest == Some(quest) || landing_here;
            let reading = read_card(content, house, quest, &party, watched);
            draw_card(page, &reading, card, hot);
        }
        page.targets.push((card, Target::Quest(quest)));
    }
    if draw && house.board.len() < QUEST_COUNT {
        let free = Rect {
            min: quest_rect(house.board.len()).min,
            max: SHEET.max,
        };
        let free = if house.board.len() % 2 == 1 {
            // An odd board leaves a card's width beside the last card; start below it.
            Rect {
                min: quest_rect(house.board.len() + 1).min,
                ..free
            }
        } else {
            free
        };
        let area = Rect {
            min: free.min + Vec2::splat(PAD),
            max: free.max - Vec2::splat(PAD),
        };
        paragraph(page, &content.words[W::SummerHelp], area, ink::NOTE, free);
    }
}

/// A seat tile: the hero's figure over their name.
fn tile(page: &mut Page, content: &Content, house: &House, id: HeroId, rect: Rect) {
    let hero = &house.heroes[id];
    page.shape(rect, ink::HOT, layers::MARK);
    let figure = Rect::from_min_size(
        Vec2::new(rect.center().x - TILE_FIGURE * 0.5, rect.min.y + 4.0),
        Vec2::splat(TILE_FIGURE),
    );
    page.figure(
        figure,
        hero_figure(content, hero),
        figure_tint(hero),
        layers::TEXT,
    );
    let style = TextStyle {
        size: MIN_TEXT,
        ..TextStyle::default()
    };
    let at = Vec2::new(
        rect.center().x - style.width_of(&hero.name) * 0.5,
        rect.max.y - MIN_TEXT - 6.0,
    );
    page.text(
        layers::TEXT,
        at,
        hero.name.clone(),
        MIN_TEXT,
        ink::BODY,
        rect,
    );
}

/// Text at `at`, right-aligned to `right`.
fn right_aligned(
    page: &mut Page,
    text: &str,
    right: f32,
    y: f32,
    size: f32,
    color: Color,
    panel: Rect,
) {
    let style = TextStyle {
        size,
        ..TextStyle::default()
    };
    let at = Vec2::new(right - style.width_of(text), y);
    page.text(layers::TEXT, at, text, size, color, panel);
}

/// `text` wrapped into `area`, one logical line; returns the y below it.
fn paragraph(page: &mut Page, text: &str, area: Rect, color: Color, panel: Rect) -> f32 {
    let mut y = area.min.y;
    for (index, piece) in wrap(text, area.size().x, MIN_TEXT).into_iter().enumerate() {
        let at = Vec2::new(area.min.x, y);
        if index == 0 {
            page.text(layers::TEXT, at, piece, MIN_TEXT, color, panel);
        } else {
            page.continue_text(layers::TEXT, at, piece, MIN_TEXT, color, panel);
        }
        y += PITCH;
    }
    y
}

/// The outcome colours, disaster to triumph.
const BANDS: [Color; 4] = [
    ink::WARN,
    Color::rgb(0.86, 0.62, 0.30),
    Color::rgb(0.45, 0.72, 0.45),
    ink::GOLD,
];

/// One quest card.
fn draw_card(page: &mut Page, card: &CardReading, rect: Rect, hot: bool) {
    page.shape(rect, if hot { ink::HOT } else { ink::PANEL }, layers::PANEL);
    let x = rect.min.x + PAD;
    let right = rect.max.x - PAD;
    let half = x + (right - x) * 0.5;
    let top = rect.min.y;
    page.text(
        layers::TEXT,
        Vec2::new(x, top + 8.0),
        card.place.clone(),
        MIN_TEXT,
        ink::NOTE,
        rect,
    );
    // Danger: the word, then a pip per point out of four.
    let pips_x = right - DANGER_LIMIT as f32 * 14.0 + 4.0;
    right_aligned(
        page,
        &card.danger_word,
        pips_x - 8.0,
        top + 8.0,
        MIN_TEXT,
        ink::NOTE,
        rect,
    );
    for pip in 0..DANGER_LIMIT {
        let color = if pip < card.danger {
            ink::WARN
        } else {
            ink::PIP_EMPTY
        };
        let at = Vec2::new(pips_x + pip as f32 * 14.0, top + 10.0);
        page.shape(
            Rect::from_min_size(at, Vec2::splat(10.0)),
            color,
            layers::MARK,
        );
    }
    page.text(
        layers::TEXT,
        Vec2::new(x, top + 25.0),
        card.title.clone(),
        18.0,
        ink::BODY,
        rect,
    );
    let mut tag_x = x;
    for (tag, feared) in &card.tags {
        let color = if *feared { ink::WARN } else { ink::NOTE };
        page.text(
            layers::TEXT,
            Vec2::new(tag_x, top + 48.0),
            tag.clone(),
            MIN_TEXT,
            color,
            rect,
        );
        tag_x += TextStyle {
            size: MIN_TEXT,
            ..TextStyle::default()
        }
        .width_of(tag)
            + 14.0;
    }
    page.text(
        layers::TEXT,
        Vec2::new(x, top + 66.0),
        card.needs.clone(),
        16.0,
        ink::HEADING,
        rect,
    );
    if let Some(bring) = &card.you_bring {
        right_aligned(page, bring, right, top + 66.0, 16.0, ink::BODY, rect);
    }
    match (&card.odds, &card.idle) {
        (Some(odds), _) => {
            let mut bar_x = x;
            for (band, ways) in card.forecast.ways.iter().enumerate() {
                if *ways == 0 {
                    continue;
                }
                let width = (right - x) * *ways as f32 / 36.0;
                let bar = Rect::from_min_size(Vec2::new(bar_x, top + 88.0), Vec2::new(width, 8.0));
                page.shape(bar, BANDS[band], layers::MARK);
                bar_x += width;
            }
            for (index, text) in odds.iter().enumerate() {
                let at = Vec2::new(
                    if index % 2 == 0 { x } else { half },
                    top + 100.0 + (index / 2) as f32 * PITCH,
                );
                page.text(layers::TEXT, at, text.clone(), MIN_TEXT, ink::BODY, rect);
            }
        }
        (None, Some(idle)) => {
            let area =
                Rect::from_min_size(Vec2::new(x, top + 92.0), Vec2::new(right - x, 2.0 * PITCH));
            paragraph(page, idle, area, ink::NOTE, rect);
        }
        (None, None) => {}
    }
    page.text(
        layers::TEXT,
        Vec2::new(x, top + 136.0),
        card.renown.clone(),
        MIN_TEXT,
        ink::NOTE,
        rect,
    );
    page.text(
        layers::TEXT,
        Vec2::new(half, top + 136.0),
        card.unanswered.clone(),
        MIN_TEXT,
        ink::NOTE,
        rect,
    );
    let mut y = top + 155.0;
    for (text, color) in [(&card.dream, ink::GOLD), (&card.warning, ink::WARN)] {
        if let Some(text) = text {
            let area = Rect::from_min_size(Vec2::new(x, y), Vec2::new(right - x, 0.0));
            y = paragraph(page, text, area, color, rect);
        }
    }
}

/// The quest sheet for board slot `quest`, raised in the right panel: the sheet's
/// lines down the left column (and on into the right), then the place's history
/// in a panel of its own.
pub fn draw_quest_sheet(page: &mut Page, content: &Content, house: &House, quest: usize) {
    let sheet = quest_sheet(content, house, quest, &house.party(quest));
    let column = (SHEET.size().x - 3.0 * PAD) * 0.5;
    let top = SHEET.min.y + PAD;
    let mut x = SHEET.min.x + PAD;
    let mut y = top;
    for line in &sheet.lines {
        let (size, color) = match line.ink {
            Ink::Title => (20.0, ink::BODY),
            Ink::Heading => (MIN_TEXT, ink::HEADING),
            Ink::Body => (MIN_TEXT, ink::BODY),
            Ink::Note => (MIN_TEXT, ink::NOTE),
        };
        let value_w = line.value.as_ref().map_or(0.0, |_| 48.0);
        let pieces = wrap(&line.text, column - value_w, size);
        let height = pieces.len() as f32 * PITCH + if size > MIN_TEXT { 8.0 } else { 0.0 };
        if y + height > SHEET.max.y - PAD && x < SHEET.center().x {
            x += column + PAD;
            y = top;
        }
        if line.ink == Ink::Heading && y > top {
            y += 6.0;
        }
        let first = y;
        let wrapped = pieces.len() > 1;
        for (index, piece) in pieces.into_iter().enumerate() {
            let at = Vec2::new(x, y);
            if index == 0 {
                page.text(layers::TEXT, at, piece, size, color, SHEET);
            } else {
                page.continue_text(layers::TEXT, at, piece, size, color, SHEET);
            }
            y += if size > MIN_TEXT { size + 8.0 } else { PITCH };
        }
        // A wrapped paragraph keeps a little air below it.
        if wrapped && line.ink == Ink::Body {
            y += 4.0;
        }
        // After the whole line, so a wrapped line stays one logical line.
        if let Some(value) = &line.value {
            right_aligned(page, value, x + column, first, MIN_TEXT, ink::BODY, SHEET);
        }
    }
    // The history panel: the right column, below whatever the sheet ran into it.
    let history_x = SHEET.min.x + PAD + column + PAD;
    let history_top = if x > SHEET.min.x + PAD { y + PAD } else { top };
    let lines: Vec<Vec<String>> = sheet
        .history
        .iter()
        .map(|text| wrap(text, column - 2.0 * PAD, MIN_TEXT))
        .collect();
    let rows: usize = lines.iter().map(Vec::len).sum();
    let panel = Rect::from_min_size(
        Vec2::new(history_x, history_top),
        Vec2::new(column, rows as f32 * PITCH + 2.0 * PAD),
    );
    page.shape(panel, ink::HOT, layers::MARK);
    let mut y = panel.min.y + PAD;
    for pieces in lines {
        for (index, piece) in pieces.into_iter().enumerate() {
            let at = Vec2::new(panel.min.x + PAD, y);
            if index == 0 {
                page.text(layers::TEXT, at, piece, MIN_TEXT, ink::BODY, panel);
            } else {
                page.continue_text(layers::TEXT, at, piece, MIN_TEXT, ink::BODY, panel);
            }
            y += PITCH;
        }
    }
}

/// The hero in hand, under the pointer, kept on screen.
pub fn draw_hand(page: &mut Page, content: &Content, house: &House, ui: &UiState) {
    let Some(drag) = ui.drag else {
        return;
    };
    let back = Vec2::splat(HAND_FIGURE + 8.0);
    let screen = crate::summer::screen_rect();
    let center = Vec2::new(
        drag.at
            .x
            .clamp(screen.min.x + back.x * 0.5, screen.max.x - back.x * 0.5),
        drag.at
            .y
            .clamp(screen.min.y + back.y * 0.5, screen.max.y - back.y * 0.5),
    );
    page.shape(Rect::from_center_size(center, back), ink::HOT, layers::HAND);
    let hero = &house.heroes[drag.hero];
    page.figure(
        Rect::from_center_size(center, Vec2::splat(HAND_FIGURE)),
        hero_figure(content, hero),
        figure_tint(hero),
        layers::HAND_MARK,
    );
}
