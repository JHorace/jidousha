//! The sheets as the dock sets them: each line of the hero sheet (SPEC §19.1), of
//! the quest sheet and its place's history (§5.4), and of the help, wrapped to the
//! dock's width and measured, for `dock::lay_out` to place whole from its scroll.
//! What the sheets say is `sheet::hero_sheet`'s and `quest_sheet::quest_sheet`'s;
//! this is only how they are set.

use jidousha::prelude::*;

use crate::art::{Figure, figure_named};
use crate::content::Content;
use crate::dock::{Line, Mark, PITCH};
use crate::hero::{Hero, HeroId};
use crate::house::House;
use crate::quest_sheet::{Ink as QuestInk, quest_sheet};
use crate::screen::{MIN_TEXT, PAD, ink, wrap};
use crate::sheet::{Ink, hero_sheet};
use crate::words::W;

/// The space a value is kept on the quest sheet's right.
const VALUE_W: f32 = 48.0;
/// The heirloom's sprite on the hero sheet: 16 px art at 2x, 8 px art at 4x.
const HEIRLOOM_FIGURE: f32 = 32.0;
/// The history panel's inset.
const HISTORY_PAD: f32 = 10.0;

/// The help, with nothing open: the summer's, the telling's (`ui.telling.help`), the
/// turning's (`ui.turning.help`) or the hearth's (`ui.winter.help`).
pub fn help_lines(content: &Content, house: &House, width: f32) -> Vec<Line> {
    let help = if house.telling.is_some() {
        W::TellingHelp
    } else if house.passage.is_some() {
        W::TurningHelp
    } else if house.calendar.is_winter() {
        W::WinterHelp
    } else {
        W::SummerHelp
    };
    vec![paragraph(&content.words[help], width, MIN_TEXT, ink::NOTE)]
}

/// The Door's help (SPEC §5.4, the top bar's Door hover), naming the best four.
pub fn door_help_lines(content: &Content, house: &House, width: f32) -> Vec<Line> {
    let help = crate::family::door_help(content, house);
    vec![paragraph(&help, width, MIN_TEXT, ink::NOTE)]
}

/// A group of winter seats' help: its heading, then what its seats do.
pub fn group_lines(content: &Content, group: crate::hearth::Group, width: f32) -> Vec<Line> {
    let (heading, help) = crate::hearth_help::group_help(content, group);
    let mut help = paragraph(&help, width, MIN_TEXT, ink::NOTE);
    help.space = 6.0;
    vec![paragraph(&heading, width, MIN_TEXT, ink::HEADING), help]
}

/// `text` wrapped to `width` as one line of the dock.
fn paragraph(text: &str, width: f32, size: f32, color: Color) -> Line {
    let pieces = wrap(text, width, size);
    Line {
        space: 0.0,
        height: pieces.len() as f32 * PITCH,
        marks: pieces
            .into_iter()
            .enumerate()
            .map(|(index, piece)| {
                crate::dock::text(
                    Vec2::new(0.0, index as f32 * PITCH),
                    piece,
                    size,
                    color,
                    index > 0,
                )
            })
            .collect(),
        history: false,
    }
}

/// The hero sheet: one column, a line of the sheet to a line of the dock.
pub fn hero_lines(content: &Content, heroes: &[Hero], id: HeroId, width: f32) -> Vec<Line> {
    let sheet = hero_sheet(content, heroes, id);
    let mut lines = Vec::new();
    for (index, line) in sheet.lines.iter().enumerate() {
        let (size, color) = match line.ink {
            Ink::Name => (20.0, ink::BODY),
            Ink::Heading => (MIN_TEXT, ink::HEADING),
            Ink::Body | Ink::Current => (MIN_TEXT, ink::BODY),
            Ink::Note | Ink::Upcoming => (MIN_TEXT, ink::NOTE),
            Ink::Warning => (MIN_TEXT, ink::WARN),
            Ink::Done | Ink::Gone => (MIN_TEXT, ink::GONE),
        };
        let mut space = 0.0;
        if line.ink == Ink::Heading && index > 0 {
            space += 6.0;
        }
        // Where the sheet's second column began: destiny on, a little more air.
        if index == sheet.second_column && index > 0 {
            space += 6.0;
        }
        // Stage marks, so done / current / upcoming read without colour as well.
        let text = match line.ink {
            Ink::Done => format!("[x] {}", line.text),
            Ink::Current => format!("[>] {}", line.text),
            Ink::Upcoming => format!("[ ] {}", line.text),
            _ => line.text.clone(),
        };
        let pitch = if size > MIN_TEXT { size + 6.0 } else { PITCH };
        let pips = line.pips.map_or(0.0, |(_, of)| 12.0 + of as f32 * 14.0);
        let mut marks = Vec::new();
        let pieces = wrap(&text, width - pips, size);
        let rows = pieces.len();
        for (piece_index, piece) in pieces.into_iter().enumerate() {
            let end = TextStyle {
                size,
                ..TextStyle::default()
            }
            .width_of(&piece);
            let y = piece_index as f32 * pitch;
            if piece_index == 0
                && let Some((filled, of)) = line.pips
            {
                for pip in 0..of {
                    let color = if pip < filled {
                        ink::DREAD
                    } else {
                        ink::PIP_EMPTY
                    };
                    marks.push(Mark::Pip {
                        at: Vec2::new(end + 12.0 + pip as f32 * 14.0, y + 2.0),
                        color,
                    });
                }
            }
            marks.push(crate::dock::text(
                Vec2::new(0.0, y),
                piece,
                size,
                color,
                piece_index > 0,
            ));
        }
        let mut height = rows as f32 * pitch;
        // The heirloom's sprite, at 2x, at the right of its heading's row: the row is
        // made the sprite's height so the sprite never reaches the next line.
        if line.ink == Ink::Heading
            && line.text == content.words[W::SheetHeirloom]
            && let Some(heirloom) = &heroes[id].heirloom
        {
            let figure: Figure = match figure_named(&heirloom.sprite) {
                Some(figure) => figure,
                None => panic!(
                    "[keifu_x_inheritance_r3v1] the heirloom {} is drawn with {:?}, which no imported sprite \
                     plays\n  fix: import one (art/import_sprites.py) and add its role",
                    heirloom.name, heirloom.sprite
                ),
            };
            for mark in &mut marks {
                if let Mark::Text { at, .. } = mark {
                    at.y += (HEIRLOOM_FIGURE - PITCH) * 0.5;
                }
            }
            marks.push(Mark::Figure {
                at: Vec2::new(width - HEIRLOOM_FIGURE, 0.0),
                size: HEIRLOOM_FIGURE,
                figure,
            });
            height = HEIRLOOM_FIGURE;
        }
        lines.push(Line {
            space,
            height,
            marks,
            history: false,
        });
    }
    lines
}

/// The quest sheet for board slot `quest`: its lines with their values on the right,
/// then the place's history in a panel of its own.
pub fn quest_lines(content: &Content, house: &House, quest: usize, width: f32) -> Vec<Line> {
    let sheet = quest_sheet(content, house, quest, &house.party(quest));
    let mut lines = Vec::new();
    for (index, line) in sheet.lines.iter().enumerate() {
        let (size, color) = match line.ink {
            QuestInk::Title => (20.0, ink::BODY),
            QuestInk::Heading => (MIN_TEXT, ink::HEADING),
            QuestInk::Body => (MIN_TEXT, ink::BODY),
            QuestInk::Note => (MIN_TEXT, ink::NOTE),
        };
        let pitch = if size > MIN_TEXT { size + 8.0 } else { PITCH };
        let value_w = line.value.as_ref().map_or(0.0, |_| VALUE_W);
        let pieces = wrap(&line.text, width - value_w, size);
        let wrapped = pieces.len() > 1;
        let mut height = pieces.len() as f32 * pitch;
        // A wrapped paragraph keeps a little air below it.
        if wrapped && line.ink == QuestInk::Body {
            height += 4.0;
        }
        let mut marks: Vec<Mark> = pieces
            .into_iter()
            .enumerate()
            .map(|(index, piece)| {
                crate::dock::text(
                    Vec2::new(0.0, index as f32 * pitch),
                    piece,
                    size,
                    color,
                    index > 0,
                )
            })
            .collect();
        // After the whole line, so a wrapped line stays one logical line.
        if let Some(value) = &line.value {
            let style = TextStyle {
                size: MIN_TEXT,
                ..TextStyle::default()
            };
            marks.push(crate::dock::text(
                Vec2::new(width - style.width_of(value), 0.0),
                value.clone(),
                MIN_TEXT,
                ink::BODY,
                false,
            ));
        }
        lines.push(Line {
            space: if line.ink == QuestInk::Heading && index > 0 {
                6.0
            } else {
                0.0
            },
            height,
            marks,
            history: false,
        });
    }
    // The history panel: each entry a line of the dock, so it scrolls with the sheet.
    let entries = sheet.history.len();
    for (index, entry) in sheet
        .history
        .iter()
        .map(|text| wrap(text, width - 2.0 * HISTORY_PAD, MIN_TEXT))
        .enumerate()
    {
        let top = if index == 0 { HISTORY_PAD } else { 0.0 };
        let bottom = if index + 1 == entries {
            HISTORY_PAD
        } else {
            0.0
        };
        let rows = entry.len();
        lines.push(Line {
            space: if index == 0 { PAD } else { 0.0 },
            height: top + rows as f32 * PITCH + bottom,
            marks: entry
                .into_iter()
                .enumerate()
                .map(|(row, piece)| {
                    crate::dock::text(
                        Vec2::new(HISTORY_PAD, top + row as f32 * PITCH),
                        piece,
                        MIN_TEXT,
                        ink::BODY,
                        row > 0,
                    )
                })
                .collect(),
            history: true,
        });
    }
    lines
}
