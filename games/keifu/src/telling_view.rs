//! The telling screen (SPEC §8): what set out wrote, page by page, under the top bar,
//! with the sheet dock beside it.
//!
//! Pages in order — in the last summer the Door's prologue first ("The last summer", the
//! four who went and what they carried), then one per resolved quest in board order (at
//! the Door, one per lock tried), then "Meanwhile" (the
//! unanswered lines, their total, home healing, and the closing line if the house
//! has closed), or "A quiet summer" when there is nothing at all. A page longer than
//! the panel continues on further leaves ("What it did to them, continued."); the
//! paging is the port's own — only the order of lines is the spec's (ENGINE-USAGE
//! §3). A quest page's first leaf shows the place, the title, the premise, the
//! members' cards, the story, then the outcome, the roll and the lines.
//!
//! **The typewriter** types the story at `TYPEWRITER_LETTERS_PER_SECOND` from the tick
//! its leaf came on screen (`UiState::typing_from`), read off the frame clock (`Clock`)
//! here and nowhere else. It is the game's only time-based behaviour, and it is pacing
//! only: the house never sees it, so the sim and any replay of it are the same whether
//! a story typed for a second or a minute. Until the story is complete the leaf's
//! outcome, roll and lines wait (SPEC-GAPS KG-37). "Go on" first completes the story,
//! then turns the leaf; the numbered buttons go to a leaf; "Skip ahead" leaves.

use std::ops::Range;

use jidousha::prelude::*;

use crate::constants::{DICE_MIDPOINT, TYPEWRITER_LETTERS_PER_SECOND};
use crate::content::Content;
use crate::hero::{Hero, HeroId};
use crate::house::House;
use crate::screen::{Clock, MIN_TEXT, PAD, PAGE_H, Page, Target, UiState, ink, layers, wrap};
use crate::summer::{CARD, CARD_GAP, LEFT_X, SHEET, TOP_H, hero_card, lay_out_top_bar};
use crate::telling::{QuestPage, Telling};
use crate::telling_nav::NAV_H;
use crate::text::fmt;
use crate::words::W;

/// The page's panel: under the bar, left of the dock.
pub const PANEL: Rect = Rect {
    min: Vec2::new(LEFT_X, TOP_H + 8.0),
    max: Vec2::new(SHEET.min.x - 12.0, PAGE_H - 8.0),
};
/// The story's type, and its pitch.
const STORY: f32 = 16.0;
const STORY_PITCH: f32 = 20.0;
/// The pitch of 14 px type.
const PITCH: f32 = 17.0;
/// Air between a leaf's lines, and between its parts.
const LINE_GAP: f32 = 6.0;
const PART_GAP: f32 = 10.0;

/// Where a leaf's type is set.
pub fn text_area() -> Rect {
    Rect {
        min: PANEL.min + Vec2::splat(PAD),
        max: Vec2::new(PANEL.max.x - PAD, PANEL.max.y - NAV_H - PAD),
    }
}

/// Which page a leaf belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The Door's prologue (SPEC §8: "the Door prologue (last summer only)").
    Prologue,
    /// A resolved quest's page, by index into `Telling::pages`.
    Quest(usize),
    /// "Meanwhile".
    Meanwhile,
    /// "A quiet summer": nothing at all happened.
    Quiet,
}

/// One leaf: a page, whether it is the page's first, and which of its lines it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leaf {
    /// Its page.
    pub part: Part,
    /// The page's first leaf (a quest's carries the header and the story).
    pub first: bool,
    /// Its lines, as indices into the page's lines.
    pub lines: Range<usize>,
}

/// The Meanwhile page's lines (SPEC §8): what set out wrote there, then
/// `ui.telling.house_closed` if the house has closed — at renown 0, which leaving
/// will act on (SPEC §2.1).
///
/// SPEC-GAPS KG-36: the page exists when it has a line, the closing line counting, and
/// "A quiet summer" only when there is no quest page and no Meanwhile line.
pub fn meanwhile_lines(content: &Content, house: &House, telling: &Telling) -> Vec<String> {
    let mut lines = telling.meanwhile.clone();
    // After the Door the Ending comes whatever the renown (SPEC §16.3): nothing closes
    // (SPEC-GAPS KG-74).
    if house.renown <= 0 && telling.door.is_none() {
        lines.push(content.words[W::TellingHouseClosed].to_owned());
    }
    lines
}

/// "Needed Might 10. Brought 12. Dice 3 and 5, less 7: beat it by 3." (SPEC §8).
pub fn roll_line(content: &Content, page: &QuestPage) -> String {
    let words = &content.words;
    let telling = match page.margin {
        0 => words[W::TellingMarginExact].to_owned(),
        m if m > 0 => fmt(&words[W::TellingMarginBeat], &[&m.to_string()]),
        m => fmt(&words[W::TellingMarginMissed], &[&(-m).to_string()]),
    };
    fmt(
        &words[W::TellingRoll],
        &[
            &content.lore.aptitudes[page.quest.aptitude.index()],
            &page.quest.demand.to_string(),
            &page.power.to_string(),
            &page.dice[0].to_string(),
            &page.dice[1].to_string(),
            &DICE_MIDPOINT.to_string(),
            &telling,
        ],
    )
}

fn rows(text: &str, size: f32) -> usize {
    wrap(text, text_area().size().x, size).len()
}

fn line_height(text: &str) -> f32 {
    rows(text, MIN_TEXT) as f32 * PITCH
}

/// How tall a quest page's header is: place, title, premise, cards, story, outcome, roll.
fn header_height(content: &Content, page: &QuestPage) -> f32 {
    PITCH
        + STORY_PITCH
        + line_height(&page.quest.premise)
        + PART_GAP
        + CARD.y
        + PART_GAP
        + rows(&page.story, STORY) as f32 * STORY_PITCH
        + PART_GAP
        + STORY_PITCH
        + line_height(&roll_line(content, page))
        + PART_GAP
}

/// Split one page's `lines` into leaves under a header of `first_header` and of
/// `more_header` after it.
fn split(part: Part, lines: &[String], first_header: f32, more_header: f32, out: &mut Vec<Leaf>) {
    let room = text_area().size().y;
    let mut at = 0;
    let mut first = true;
    loop {
        let mut used = if first { first_header } else { more_header };
        let start = at;
        while at < lines.len() && used + line_height(&lines[at]) <= room {
            used += line_height(&lines[at]) + LINE_GAP;
            at += 1;
        }
        // INVARIANT: every leaf after the first holds a line, or the page could never end.
        assert!(
            at > start || at == lines.len() || first,
            "[keifu] a telling line is taller than a leaf: {:?}\n  likely cause: a very long \
             line of content\n  fix: give the telling's panel more height",
            lines[at]
        );
        out.push(Leaf {
            part,
            first,
            lines: start..at,
        });
        first = false;
        if at >= lines.len() {
            return;
        }
    }
}

/// How tall the Door prologue's first leaf's header is: the heading, the intro, the cards.
fn prologue_header(content: &Content) -> f32 {
    PITCH + line_height(&content.words[W::TellingDoorIntro]) + PART_GAP + CARD.y + PART_GAP
}

/// The telling's leaves, in order (SPEC §8).
pub fn leaves(content: &Content, house: &House, telling: &Telling) -> Vec<Leaf> {
    let mut out = Vec::new();
    let heading = PITCH + PART_GAP;
    if let Some(door) = &telling.door {
        split(
            Part::Prologue,
            &door.prologue,
            prologue_header(content),
            heading,
            &mut out,
        );
    }
    for (index, page) in telling.pages.iter().enumerate() {
        split(
            Part::Quest(index),
            &page.lines,
            header_height(content, page),
            heading,
            &mut out,
        );
    }
    let meanwhile = meanwhile_lines(content, house, telling);
    if !meanwhile.is_empty() {
        split(Part::Meanwhile, &meanwhile, heading, heading, &mut out);
    }
    if out.is_empty() {
        out.push(Leaf {
            part: Part::Quiet,
            first: true,
            lines: 0..0,
        });
    }
    out
}

/// How much of a leaf's story is typed: `(typed, of)` letters, or `None` for a leaf
/// with no story.
pub fn typing(
    telling: &Telling,
    leaf: &Leaf,
    ui: &UiState,
    clock: Clock,
) -> Option<(usize, usize)> {
    let Part::Quest(index) = leaf.part else {
        return None;
    };
    if !leaf.first {
        return None;
    }
    let of = telling.pages[index].story.chars().count();
    if ui.revealed {
        return Some((of, of));
    }
    let seconds = clock.tick.saturating_sub(ui.typing_from) as f32 * clock.dt;
    let typed = (seconds * TYPEWRITER_LETTERS_PER_SECOND).floor() as usize;
    Some((typed.min(of), of))
}

/// Whether the leaf on screen has its story whole (or has none).
pub fn story_complete(telling: &Telling, leaf: &Leaf, ui: &UiState, clock: Clock) -> bool {
    typing(telling, leaf, ui, clock).is_none_or(|(typed, of)| typed >= of)
}

/// The hero a line is about, for pointing at it: the house's hero whose first name
/// the line names first (presentation; the lines are the content's sentences).
pub fn about(heroes: &[Hero], line: &str) -> Option<HeroId> {
    let mut best: Option<(usize, HeroId)> = None;
    for (id, hero) in heroes.iter().enumerate() {
        let mut from = 0;
        while let Some(found) = line[from..].find(&hero.name) {
            let at = from + found;
            let end = at + hero.name.len();
            let before = line[..at].chars().next_back();
            let after = line[end..].chars().next();
            let word = !before.is_some_and(char::is_alphanumeric)
                && !after.is_some_and(char::is_alphanumeric);
            if word {
                if best.is_none_or(|(b, _)| at < b) {
                    best = Some((at, id));
                }
                break;
            }
            from = end;
        }
    }
    best.map(|(_, id)| id)
}

/// Set `text` wrapped at `size` from `y`, up to `typed` letters of it; returns the y
/// below it. Pointing at it points at the hero it is about, when `about` names one.
#[allow(clippy::too_many_arguments)]
fn paragraph(
    page: &mut Page,
    text: &str,
    y: f32,
    size: f32,
    pitch: f32,
    color: Color,
    typed: Option<usize>,
    about: Option<HeroId>,
) -> f32 {
    let area = text_area();
    let pieces = wrap(text, area.size().x, size);
    let mut left = typed.unwrap_or(usize::MAX);
    let mut y = y;
    let top = y;
    for (index, piece) in pieces.iter().enumerate() {
        if left == 0 {
            break;
        }
        let shown: String = piece.chars().take(left).collect();
        left = left.saturating_sub(piece.chars().count() + 1);
        let at = Vec2::new(area.min.x, y);
        if index == 0 {
            page.text(layers::TEXT, at, shown, size, color, PANEL);
        } else {
            page.continue_text(layers::TEXT, at, shown, size, color, PANEL);
        }
        y += pitch;
    }
    let bottom = top + pieces.len() as f32 * pitch;
    if let Some(id) = about {
        let rect = Rect::from_min_size(
            Vec2::new(area.min.x, top),
            Vec2::new(area.size().x, bottom - top),
        );
        page.targets.push((rect, Target::Hero(id)));
    }
    bottom
}

/// Lay the telling out: the bar, the leaf on screen, its navigation, and the dock.
pub fn lay_out(
    page: &mut Page,
    content: &Content,
    house: &House,
    telling: &Telling,
    ui: &UiState,
    clock: Clock,
) {
    let words = &content.words;
    lay_out_top_bar(page, content, house);
    page.shape(PANEL, ink::PANEL, layers::PANEL);
    let leaves = leaves(content, house, telling);
    let current = ui.leaf.min(leaves.len() - 1);
    let leaf = &leaves[current];
    let area = text_area();
    let mut y = area.min.y;
    let typed = typing(telling, leaf, ui, clock);
    let complete = story_complete(telling, leaf, ui, clock);
    let (heading, lines): (String, Vec<String>) = match leaf.part {
        Part::Prologue => {
            let Some(door) = &telling.door else {
                panic!(
                    "[keifu] a prologue leaf in a telling with no Door\n  likely cause: the \
                     leaves were read off another telling\n  fix: build them with leaves()"
                );
            };
            let lines = door.prologue[leaf.lines.clone()].to_vec();
            if !leaf.first {
                (words[W::TellingDoorIntroMore].to_owned(), lines)
            } else {
                y = paragraph(
                    page,
                    &words[W::TellingDoorHeading],
                    y,
                    MIN_TEXT,
                    PITCH,
                    ink::HEADING,
                    None,
                    None,
                );
                y = paragraph(
                    page,
                    &words[W::TellingDoorIntro],
                    y,
                    MIN_TEXT,
                    PITCH,
                    ink::NOTE,
                    None,
                    None,
                );
                y += PART_GAP;
                for (seat, &member) in door.party.iter().enumerate() {
                    let rect = Rect::from_min_size(
                        Vec2::new(area.min.x + seat as f32 * (CARD.x + CARD_GAP), y),
                        CARD,
                    );
                    hero_card(
                        page,
                        content,
                        &house.heroes,
                        member,
                        rect,
                        ui.pointing == Some(member),
                    );
                }
                y += CARD.y + PART_GAP;
                (String::new(), lines)
            }
        }
        Part::Quest(index) => {
            let quest_page = &telling.pages[index];
            let lines = quest_page.lines[leaf.lines.clone()].to_vec();
            if !leaf.first {
                (words[W::TellingQuestMore].to_owned(), lines)
            } else {
                let place = &content.lore.places[quest_page.quest.place.index()].title;
                y = paragraph(page, place, y, MIN_TEXT, PITCH, ink::HEADING, None, None);
                y = paragraph(
                    page,
                    &quest_page.quest.title,
                    y,
                    STORY,
                    STORY_PITCH,
                    ink::BODY,
                    None,
                    None,
                );
                y = paragraph(
                    page,
                    &quest_page.quest.premise,
                    y,
                    MIN_TEXT,
                    PITCH,
                    ink::NOTE,
                    None,
                    None,
                );
                y += PART_GAP;
                for (seat, &member) in quest_page.members.iter().enumerate() {
                    let rect = Rect::from_min_size(
                        Vec2::new(area.min.x + seat as f32 * (CARD.x + CARD_GAP), y),
                        CARD,
                    );
                    hero_card(
                        page,
                        content,
                        &house.heroes,
                        member,
                        rect,
                        ui.pointing == Some(member),
                    );
                }
                y += CARD.y + PART_GAP;
                let typed = typed.map(|(typed, _)| typed);
                y = paragraph(
                    page,
                    &quest_page.story,
                    y,
                    STORY,
                    STORY_PITCH,
                    ink::BODY,
                    typed,
                    None,
                );
                y += PART_GAP;
                if complete {
                    let outcome = &content.lore.outcomes[quest_page.outcome.index()];
                    y = paragraph(
                        page,
                        outcome,
                        y,
                        STORY,
                        STORY_PITCH,
                        ink::HEADING,
                        None,
                        None,
                    );
                    y = paragraph(
                        page,
                        &roll_line(content, quest_page),
                        y,
                        MIN_TEXT,
                        PITCH,
                        ink::NOTE,
                        None,
                        None,
                    );
                    y += PART_GAP;
                }
                (String::new(), lines)
            }
        }
        Part::Meanwhile => {
            let all = meanwhile_lines(content, house, telling);
            let heading = if leaf.first {
                W::TellingMeanwhile
            } else {
                W::TellingMeanwhileMore
            };
            (words[heading].to_owned(), all[leaf.lines.clone()].to_vec())
        }
        Part::Quiet => {
            y = paragraph(
                page,
                &words[W::TellingQuietTitle],
                y,
                MIN_TEXT,
                PITCH,
                ink::HEADING,
                None,
                None,
            );
            y += PART_GAP;
            (String::new(), vec![words[W::TellingQuietText].to_owned()])
        }
    };
    if !heading.is_empty() {
        y = paragraph(page, &heading, y, MIN_TEXT, PITCH, ink::HEADING, None, None) + PART_GAP;
    }
    if complete {
        for line in &lines {
            y = paragraph(
                page,
                line,
                y,
                MIN_TEXT,
                PITCH,
                ink::BODY,
                None,
                about(&house.heroes, line),
            ) + LINE_GAP;
        }
    }
    crate::telling_nav::lay_out_nav(page, content, house, telling, leaves.len(), current);
    crate::dock::lay_out(page, content, house, ui);
}
