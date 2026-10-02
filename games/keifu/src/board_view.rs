//! The board on the summer screen: up to four quest cards in a 2x2 grid, each with
//! its seats as tiles along its foot, and the hero in hand.
//!
//! The board has its own region (`summer::BOARD`) and the sheets have theirs (the
//! dock, `dock.rs`), so every card is drawn always — with a sheet open and mid-drag
//! alike, the card a hero is held over is on screen with its live preview.

use jidousha::prelude::*;

use crate::art::hero_figure;
use crate::board::Slot;
use crate::constants::DANGER_LIMIT;
use crate::content::Content;
use crate::hero::HeroId;
use crate::house::House;
use crate::quest_card::{CardReading, preview_seats, read_card};
use crate::screen::{MIN_TEXT, Page, Target, UiState, ink, layers, wrap};
use crate::summer::{BOARD, figure_tint};

/// The gap between cards.
const GAP: f32 = 8.0;
/// A card's inner margin.
const CARD_PAD: f32 = 10.0;
/// A seat tile: a figure over a name.
pub const TILE: Vec2 = Vec2::new(84.0, 60.0);
/// The type's line pitch.
const PITCH: f32 = 17.0;
/// The pitch of 16 px type.
const PITCH_16: f32 = 20.0;
/// The figure on a tile, and in hand: 16 px art at 2x and 3x.
const TILE_FIGURE: f32 = 32.0;
const HAND_FIGURE: f32 = 48.0;

/// Where board slot `slot` sits: a 2x2 grid filling the board.
pub fn quest_rect(slot: usize) -> Rect {
    let size = Vec2::new((BOARD.size().x - GAP) * 0.5, (BOARD.size().y - GAP) * 0.5);
    let at = Vec2::new((slot % 2) as f32, (slot / 2) as f32) * (size + Vec2::splat(GAP));
    Rect::from_min_size(BOARD.min + at, size)
}

/// Where seat `seat` of the card at `card` sits: a row along its foot.
pub fn seat_rect(card: Rect, seat: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(
            card.min.x + CARD_PAD + seat as f32 * (TILE.x + GAP),
            card.max.y - CARD_PAD - TILE.y,
        ),
        TILE,
    )
}

/// What a drag hands the card reading: who is in hand, and where they would land.
pub fn hand(house: &House, ui: &UiState) -> Option<(HeroId, Option<Slot>)> {
    let drag = ui.drag?;
    Some((drag.hero, house.landing(drag.hero, drag.over)))
}

/// Lay the board out: every card, its seats and its targets.
pub fn lay_out_board(page: &mut Page, content: &Content, house: &House, ui: &UiState) {
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
                    tile(page, content, house, id, rect);
                    match house.hero_in(slot) {
                        Some(seated) if ui.drag.map(|d| d.hero) != Some(seated) => {
                            page.targets.push((rect, Target::Hero(seated)))
                        }
                        _ => page.targets.push((rect, Target::Seat(slot))),
                    }
                }
                None => {
                    page.shape(rect, ink::PAGE, layers::MARK);
                    page.targets.push((rect, Target::Seat(slot)));
                }
            }
        }
        let landing_here = matches!(
            hand.and_then(|h| h.1),
            Some(Slot::Quest { quest: q, .. }) if q == quest
        );
        let hot = ui.pointing_quest == Some(quest) || landing_here;
        let reading = read_card(content, house, quest, &party, watched);
        draw_card(page, &reading, card, hot);
        page.targets.push((card, Target::Quest(quest)));
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

/// `text` at `size`, wrapped into `area`'s width from its top, one logical line;
/// returns the y below it.
fn paragraph(page: &mut Page, text: &str, area: Rect, size: f32, color: Color, panel: Rect) -> f32 {
    let pitch = if size > MIN_TEXT { PITCH_16 } else { PITCH };
    let mut y = area.min.y;
    for (index, piece) in wrap(text, area.size().x, size).into_iter().enumerate() {
        let at = Vec2::new(area.min.x, y);
        if index == 0 {
            page.text(layers::TEXT, at, piece, size, color, panel);
        } else {
            page.continue_text(layers::TEXT, at, piece, size, color, panel);
        }
        y += pitch;
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

/// One quest card, top to bottom: the place; its tags, and its danger in pips; the
/// title; what it needs and what the party brings; the odds bar and the four odds
/// (or, with nobody going, the idle line); renown and the cost of leaving it; the
/// dream and warning lines; and the seats along the foot.
fn draw_card(page: &mut Page, card: &CardReading, rect: Rect, hot: bool) {
    page.shape(rect, if hot { ink::HOT } else { ink::PANEL }, layers::PANEL);
    let x = rect.min.x + CARD_PAD;
    let right = rect.max.x - CARD_PAD;
    let width = right - x;
    let top = rect.min.y;
    page.text(
        layers::TEXT,
        Vec2::new(x, top + 8.0),
        card.place.clone(),
        MIN_TEXT,
        ink::NOTE,
        rect,
    );
    let mut tag_x = x;
    for (tag, feared) in &card.tags {
        let color = if *feared { ink::WARN } else { ink::NOTE };
        page.text(
            layers::TEXT,
            Vec2::new(tag_x, top + 26.0),
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
    // Danger: the word, then a pip per point out of four.
    let pips_x = right - DANGER_LIMIT as f32 * 14.0 + 4.0;
    right_aligned(
        page,
        &card.danger_word,
        pips_x - 8.0,
        top + 26.0,
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
        let at = Vec2::new(pips_x + pip as f32 * 14.0, top + 28.0);
        page.shape(
            Rect::from_min_size(at, Vec2::splat(10.0)),
            color,
            layers::MARK,
        );
    }
    let line = |y: f32| Rect::from_min_size(Vec2::new(x, y), Vec2::new(width, 0.0));
    let mut y = paragraph(page, &card.title, line(top + 46.0), 16.0, ink::BODY, rect) + 4.0;
    page.text(
        layers::TEXT,
        Vec2::new(x, y),
        card.needs.clone(),
        16.0,
        ink::HEADING,
        rect,
    );
    y += PITCH_16;
    if let Some(bring) = &card.you_bring {
        page.text(
            layers::TEXT,
            Vec2::new(x, y),
            bring.clone(),
            16.0,
            ink::BODY,
            rect,
        );
        y += PITCH_16;
    }
    match (&card.odds, &card.idle) {
        (Some(odds), _) => {
            let mut bar_x = x;
            for (band, ways) in card.forecast.ways.iter().enumerate() {
                if *ways == 0 {
                    continue;
                }
                let width = width * *ways as f32 / 36.0;
                let bar = Rect::from_min_size(Vec2::new(bar_x, y + 4.0), Vec2::new(width, 8.0));
                page.shape(bar, BANDS[band], layers::MARK);
                bar_x += width;
            }
            // Succeed and setback on the left, triumph and disaster set to the right.
            for (index, text) in odds.iter().enumerate() {
                let row = y + 18.0 + (index / 2) as f32 * PITCH;
                if index % 2 == 0 {
                    page.text(
                        layers::TEXT,
                        Vec2::new(x, row),
                        text.clone(),
                        MIN_TEXT,
                        ink::BODY,
                        rect,
                    );
                } else {
                    right_aligned(page, text, right, row, MIN_TEXT, ink::BODY, rect);
                }
            }
            y += 18.0 + 2.0 * PITCH;
        }
        (None, Some(idle)) => {
            y = paragraph(page, idle, line(y + 4.0), MIN_TEXT, ink::NOTE, rect);
        }
        (None, None) => {}
    }
    y += 4.0;
    page.text(
        layers::TEXT,
        Vec2::new(x, y),
        card.renown.clone(),
        MIN_TEXT,
        ink::NOTE,
        rect,
    );
    right_aligned(page, &card.unanswered, right, y, MIN_TEXT, ink::NOTE, rect);
    y += PITCH + 4.0;
    for (text, color) in [(&card.dream, ink::GOLD), (&card.warning, ink::WARN)] {
        if let Some(text) = text {
            y = paragraph(page, text, line(y), MIN_TEXT, color, rect);
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
