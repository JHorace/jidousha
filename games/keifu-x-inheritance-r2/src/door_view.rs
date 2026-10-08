//! The Door card and the Door sheet (SPEC §16.4, `lineage/quest-card.jai:272-369`): what
//! the player reads in the last summer at the moment of choosing the four.
//!
//! Both are pure readings of `door::outlook` for the party on the Door — the seated one,
//! or the one a drag previews — so the card's "you bring" and "All three locks open: P in
//! 100" are the powers and chances the locks will roll against (the one-source rule). The
//! card fills the board; its four seats run along its foot. The sheet, in the dock when
//! the card is pointed at, shows every member's solo power at each lock, the bonds between
//! them and the patron at Court, summed to the card's number.

use jidousha::prelude::*;

use crate::board::Slot;
use crate::board_view::{seat_rect, tile};
use crate::content::Content;
use crate::door::{lock_quest, outlook};
use crate::forecast::{card_percentages, forecast};
use crate::hero::HeroId;
use crate::house::House;
use crate::ids::Place;
use crate::power::{bonds_power, party_power, you_bring};
use crate::quest_card::preview_seats;
use crate::quest_sheet::{Ink, QuestSheet, SheetLine};
use crate::screen::{MIN_TEXT, Page, Target, UiState, ink, layers, wrap};
use crate::summer::BOARD;
use crate::text::{fmt, signed};
use crate::words::W;

/// How a line of the Door card is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardInk {
    /// "The Sealed Door".
    Place,
    /// A lock's line, "The lock of iron: needs Might 34".
    Lock,
    /// "you bring 22", "All three locks open: 4 in 100.".
    Number,
    /// The card's prose and a lock's odds.
    Note,
}

/// The Door card's lines, top to bottom, for `party` (SPEC §16.4): the place, its tags
/// and "It asks for four.", what the locks are; then, with anyone before it, "All three
/// locks open: P in 100." and each lock — what it needs, "you bring N" and its four odds;
/// with nobody, "No one stands before it.". SPEC-GAPS KG-71: only §16.4's numbers and the
/// `door_sheet.card_*` words — no fear, refusal or dream line.
pub fn read_door_card(
    content: &Content,
    house: &House,
    party: &[HeroId],
) -> Vec<(String, CardInk)> {
    let words = &content.words;
    let place = &content.lore.places[Place::SealedDoor.index()];
    let tags: Vec<&str> = place
        .tags
        .iter()
        .map(|tag| content.lore.tags[tag.index()].title.as_str())
        .collect();
    let mut out = vec![
        (place.title.clone(), CardInk::Place),
        (fmt(&words[W::DoorCardTags], &tags), CardInk::Note),
        (words[W::DoorCardText].to_owned(), CardInk::Note),
    ];
    if party.is_empty() {
        out.push((words[W::DoorCardEmpty].to_owned(), CardInk::Note));
        return out;
    }
    let seen = outlook(content, &house.heroes, party, house.patrons);
    out.push((
        fmt(
            &words[W::DoorCardAllThree],
            &[&seen.all_percent().to_string()],
        ),
        CardInk::Number,
    ));
    for (lock, words_of) in content.door.locks.iter().enumerate() {
        let aptitude = &content.lore.aptitudes[words_of.aptitude.index()];
        out.push((
            fmt(
                &words[W::DoorCardLock],
                &[&words_of.title, aptitude, &words_of.demand.to_string()],
            ),
            CardInk::Lock,
        ));
        out.push((you_bring(content, seen.powers[lock]), CardInk::Number));
        let odds = card_percentages(&forecast(seen.powers[lock], words_of.demand, true));
        let odds: Vec<String> = odds.iter().map(i32::to_string).collect();
        let odds: Vec<&str> = odds.iter().map(String::as_str).collect();
        out.push((fmt(&words[W::DoorCardOdds], &odds), CardInk::Note));
    }
    out
}

/// Lay the Door card out over the board: its lines, its four seats, its targets.
pub fn lay_out_door(page: &mut Page, content: &Content, house: &House, ui: &UiState) {
    let card = BOARD;
    let hand = crate::board_view::hand(house, ui);
    let seats = preview_seats(house, 0, hand);
    let party: Vec<HeroId> = seats.iter().flatten().copied().collect();
    for (seat, shown) in seats.iter().enumerate() {
        let slot = Slot::Quest { quest: 0, seat };
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
    let landing_here = matches!(hand.and_then(|h| h.1), Some(Slot::Quest { quest: 0, .. }));
    let hot = ui.pointing_quest == Some(0) || landing_here;
    page.shape(card, if hot { ink::HOT } else { ink::PANEL }, layers::PANEL);
    let x = card.min.x + 12.0;
    let width = card.size().x - 24.0;
    let mut y = card.min.y + 10.0;
    for (text, kind) in read_door_card(content, house, &party) {
        let (size, color, pitch, before) = match kind {
            CardInk::Place => (20.0, ink::HEADING, 26.0, 0.0),
            CardInk::Lock => (16.0, ink::HEADING, 20.0, 8.0),
            CardInk::Number => (16.0, ink::BODY, 20.0, 0.0),
            CardInk::Note => (MIN_TEXT, ink::NOTE, 17.0, 0.0),
        };
        y += before;
        for (index, piece) in wrap(&text, width, size).into_iter().enumerate() {
            let at = Vec2::new(x, y);
            if index == 0 {
                page.text(layers::TEXT, at, piece, size, color, card);
            } else {
                page.continue_text(layers::TEXT, at, piece, size, color, card);
            }
            y += pitch;
        }
        y += 4.0;
    }
    page.targets.push((card, Target::Quest(0)));
}

fn line(ink: Ink, text: impl Into<String>, value: Option<String>) -> SheetLine {
    SheetLine {
        text: text.into(),
        ink,
        value,
    }
}

/// The Door sheet for `party` (SPEC §16.4): with nobody before it, how the locks are
/// weighed; else "All three open: P in 100." and, per lock, its title, "bring N, open P in
/// 100", each member's solo power, the bonds between them and the patron bonus — which add
/// up to the lock's power, asserted.
pub fn door_sheet(content: &Content, house: &House, party: &[HeroId]) -> QuestSheet {
    let words = &content.words;
    let place = &content.lore.places[Place::SealedDoor.index()];
    let mut lines = vec![line(Ink::Title, place.title.clone(), None)];
    if party.is_empty() {
        lines.push(line(Ink::Body, &words[W::DoorSheetEmpty], None));
        return QuestSheet {
            lines,
            history: Vec::new(),
        };
    }
    let heroes = &house.heroes;
    let seen = outlook(content, heroes, party, house.patrons);
    lines.push(line(
        Ink::Body,
        fmt(
            &words[W::DoorSheetAllThree],
            &[&seen.all_percent().to_string()],
        ),
        None,
    ));
    let bonds = bonds_power(heroes, party);
    let patrons = house.patrons * crate::constants::PATRON_POWER;
    for lock in 0..content.door.locks.len() {
        let quest = lock_quest(content, lock);
        lines.push(line(Ink::Heading, quest.title.clone(), None));
        lines.push(line(
            Ink::Body,
            fmt(
                &words[W::DoorSheetLock],
                &[
                    &seen.powers[lock].to_string(),
                    &seen.lock_percent(lock).to_string(),
                ],
            ),
            None,
        ));
        let aptitude = &content.lore.aptitudes[quest.aptitude.index()];
        let mut sum = 0;
        // SPEC-GAPS KG-72: a member's solo power reads as the quest sheet's member line.
        for &member in party {
            let solo = party_power(heroes, &[member], quest.facts(), 0);
            sum += solo;
            lines.push(line(
                Ink::Note,
                fmt(
                    &words[W::PowerLineMember],
                    &[&heroes[member].name, aptitude, &solo.to_string()],
                ),
                None,
            ));
        }
        lines.push(line(
            Ink::Note,
            &words[W::DoorSheetBonds],
            Some(signed(bonds)),
        ));
        if patrons > 0 {
            lines.push(line(
                Ink::Note,
                &words[W::DoorSheetPatron],
                Some(signed(patrons)),
            ));
        }
        let total = sum + bonds + patrons;
        assert_eq!(
            total, seen.powers[lock],
            "[keifu_x_inheritance_r2] the Door sheet's lines for lock {lock} add to {total}, the card says {}\n  \
             likely cause: a power line read apart from power::party_power\n  fix: build the \
             sheet from the same member_power, bonds and patrons",
            seen.powers[lock]
        );
    }
    QuestSheet {
        lines,
        history: Vec::new(),
    }
}
