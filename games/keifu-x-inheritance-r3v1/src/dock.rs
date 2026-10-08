//! The sheet dock: a panel reserved down the right edge of the summer screen, which
//! holds whatever sheet is open — the hero pointed at or in hand (SPEC §19.1), the
//! quest pointed at with its place's history (§5.4), a group of winter seats' help
//! (§11.2) — and, with none open, the help.
//!
//! Nothing else is ever laid out in the dock and the dock is never laid over
//! anything, so a sheet can be open while the household, the yard, every quest card
//! and the hero in hand all stay in view (owner review of W4, 2026-10-01). Mid-drag
//! the dock holds the sheet of the hero in hand.
//!
//! A sheet longer than the dock scrolls inside it, a whole line at a time: the dock
//! draws the lines from `UiState::dock_first` on that fit whole, never part of one,
//! and a scrollbar beside them shows how much lies above and below. The wheel, or a
//! drag inside the dock (a finger's way), moves it (`pointer.rs`).

use jidousha::prelude::*;

use crate::art::Figure;
use crate::content::Content;
use crate::dock_lines::{group_lines, help_lines, hero_lines, quest_lines};
use crate::hero::HeroId;
use crate::house::House;
use crate::screen::{DockView, PAD, Page, Target, UiState, ink, layers};
use crate::summer::SHEET;

/// The scrollbar's lane at the dock's right: a gap, then the bar.
const LANE: f32 = 10.0;
const BAR: f32 = 6.0;
/// The shortest the scrollbar's thumb is drawn, so it can be found and grabbed.
const THUMB_MIN: f32 = 24.0;
/// The line pitch of 14 px type, and the distance a grab moves the sheet one line.
pub const PITCH: f32 = 17.0;

/// What the dock is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    /// Nothing is open: the help.
    Help,
    /// A hero's sheet.
    Hero(HeroId),
    /// A quest's sheet, by board slot.
    Quest(usize),
    /// A group of winter seats' help.
    Group(crate::hearth::Group),
    /// The Door's help, naming the best four.
    Door,
}

/// What the dock shows for `ui`: the hero in hand, else the hero or quest pointed at.
pub fn subject(ui: &UiState) -> Subject {
    if ui.family_open {
        return Subject::Help;
    }
    match (ui.drag, ui.pointing, ui.pointing_quest, ui.pointing_group) {
        (Some(drag), _, _, _) => Subject::Hero(drag.hero),
        (None, Some(id), _, _) => Subject::Hero(id),
        (None, None, Some(quest), _) => Subject::Quest(quest),
        (None, None, None, Some(group)) => Subject::Group(group),
        (None, None, None, None) if ui.pointing_door => Subject::Door,
        (None, None, None, None) => Subject::Help,
    }
}

/// Where the dock's lines are set: inside its margin, left of the scrollbar's lane.
pub fn text_rect() -> Rect {
    Rect {
        min: SHEET.min + Vec2::splat(PAD),
        max: Vec2::new(SHEET.max.x - PAD - LANE, SHEET.max.y - PAD),
    }
}

/// Where the scrollbar runs.
pub fn lane() -> Rect {
    let text = text_rect();
    Rect {
        min: Vec2::new(SHEET.max.x - PAD - BAR, text.min.y),
        max: Vec2::new(SHEET.max.x - PAD, text.max.y),
    }
}

/// One thing a line draws, placed from the line's top-left.
pub enum Mark {
    /// Type; `continues` marks a wrapped piece of the logical line before it.
    Text {
        at: Vec2,
        text: String,
        size: f32,
        color: Color,
        continues: bool,
    },
    /// A pip, 10 px square.
    Pip { at: Vec2, color: Color },
    /// A sprite.
    Figure { at: Vec2, size: f32, figure: Figure },
}

/// One line of a sheet as the dock sets it: drawn whole or not at all.
pub struct Line {
    /// Air above it, unless it is the first line in view.
    pub space: f32,
    /// How tall it is set.
    pub height: f32,
    /// What it draws.
    pub marks: Vec<Mark>,
    /// It belongs in the place's history panel.
    pub history: bool,
}

/// A piece of type, as a mark.
pub fn text(at: Vec2, text: String, size: f32, color: Color, continues: bool) -> Mark {
    Mark::Text {
        at,
        text,
        size,
        color,
        continues,
    }
}

/// Lay the dock out: its panel, its target, and the open sheet's lines from the scroll.
pub fn lay_out(page: &mut Page, content: &Content, house: &House, ui: &UiState) {
    page.shape(SHEET, ink::PANEL, layers::PANEL);
    page.targets.push((SHEET, Target::Dock));
    let area = text_rect();
    let width = area.size().x;
    let lines = match subject(ui) {
        Subject::Help => help_lines(content, house, width),
        Subject::Hero(id) => hero_lines(content, &house.heroes, id, width),
        Subject::Quest(quest) => quest_lines(content, house, quest, width),
        Subject::Group(group) => group_lines(content, group, width),
        Subject::Door => crate::dock_lines::door_help_lines(content, house, width),
    };
    let max_first = max_first(&lines, area.size().y);
    let first = ui.dock_first.min(max_first);
    // Which lines fit whole from `first`, and where.
    let mut placed = Vec::new();
    let mut y = area.min.y;
    for (index, line) in lines.iter().enumerate().skip(first) {
        let space = if index == first { 0.0 } else { line.space };
        if y + space + line.height > area.max.y {
            break;
        }
        y += space;
        placed.push((index, y));
        y += line.height;
    }
    let history = placed
        .iter()
        .filter(|(index, _)| lines[*index].history)
        .map(|&(index, top)| {
            Rect::from_min_size(
                Vec2::new(area.min.x, top),
                Vec2::new(width, lines[index].height),
            )
        })
        .reduce(|a, b| Rect {
            min: a.min.min(b.min),
            max: a.max.max(b.max),
        });
    if let Some(panel) = history {
        page.shape(panel, ink::HOT, layers::MARK);
    }
    for &(index, top) in &placed {
        let line = &lines[index];
        let panel = match history {
            Some(panel) if line.history => panel,
            _ => SHEET,
        };
        let rows_before = page.rows.len();
        for mark in &line.marks {
            match mark {
                Mark::Text {
                    at,
                    text,
                    size,
                    color,
                    continues,
                } => {
                    let at = Vec2::new(area.min.x, top) + *at;
                    if *continues {
                        page.continue_text(layers::TEXT, at, text.clone(), *size, *color, panel);
                    } else {
                        page.text(layers::TEXT, at, text.clone(), *size, *color, panel);
                    }
                }
                Mark::Pip { at, color } => page.shape(
                    Rect::from_min_size(Vec2::new(area.min.x, top) + *at, Vec2::splat(10.0)),
                    *color,
                    layers::MARK,
                ),
                Mark::Figure { at, size, figure } => page.figure(
                    Rect::from_min_size(Vec2::new(area.min.x, top) + *at, Vec2::splat(*size)),
                    *figure,
                    Color::WHITE,
                    layers::MARK,
                ),
            }
        }
        for row in &mut page.rows[rows_before..] {
            row.dock_line = Some(index);
        }
    }
    if max_first > 0 {
        scrollbar(page, &lines, first, placed.len());
    }
    page.dock = DockView {
        first,
        shown: placed.len(),
        total: lines.len(),
        max_first,
    };
}

/// The largest first line that still fills the dock: the first line from which every
/// line to the end fits whole.
///
/// INVARIANT: every line fits the dock on its own. One that did not could never be
/// drawn — a silent loss — so it panics instead.
fn max_first(lines: &[Line], room: f32) -> usize {
    if let Some(line) = lines.iter().find(|line| line.height > room) {
        panic!(
            "[keifu_x_inheritance_r3v1] a sheet line is {:.0} px tall and the dock has {room:.0}\n  likely cause: \
             a paragraph of content longer than any sheet line so far\n  fix: give the dock \
             more height (summer::SHEET) or break the paragraph in the sheet",
            line.height
        );
    }
    let mut below = 0.0;
    let mut first = lines.len();
    for index in (0..lines.len()).rev() {
        let need = lines[index].height + below;
        if need > room {
            break;
        }
        first = index;
        below = need + lines[index].space;
    }
    first.min(lines.len().saturating_sub(1))
}

/// The scrollbar: the lane, and a thumb whose length is the share of the sheet in
/// view and whose place is the share above it.
fn scrollbar(page: &mut Page, lines: &[Line], first: usize, shown: usize) {
    let lane = lane();
    page.shape(lane, ink::PIP_EMPTY, layers::MARK);
    let tall = |range: std::ops::Range<usize>| -> f32 {
        lines[range].iter().map(|l| l.space + l.height).sum()
    };
    let whole = tall(0..lines.len());
    let above = tall(0..first);
    let seen = tall(first..first + shown);
    let length = (lane.size().y * seen / whole).max(THUMB_MIN);
    let top = (lane.min.y + lane.size().y * above / whole).min(lane.max.y - length);
    page.shape(
        Rect::from_min_size(Vec2::new(lane.min.x, top), Vec2::new(BAR, length)),
        ink::NOTE,
        layers::TEXT,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::Drag;
    use crate::testkit::{house, id};

    fn line(height: f32, space: f32) -> Line {
        Line {
            space,
            height,
            marks: Vec::new(),
            history: false,
        }
    }

    #[test]
    fn a_sheet_that_fits_the_dock_does_not_scroll() {
        assert_eq!(max_first(&[line(100.0, 0.0), line(100.0, 6.0)], 300.0), 0);
    }

    #[test]
    fn a_long_sheet_scrolls_until_its_last_line_is_in_view_and_no_further() {
        // Five lines of 100 in a dock of 250: from the fourth on, the rest fit.
        let lines: Vec<Line> = (0..5).map(|_| line(100.0, 0.0)).collect();
        assert_eq!(max_first(&lines, 250.0), 3);
    }

    #[test]
    fn the_air_above_a_line_counts_only_below_the_first_line_in_view() {
        // 100 + (20 + 100) = 220 fits 220 from the second line; the first line's
        // own air is never spent at the top of the dock.
        let lines = [line(100.0, 0.0), line(100.0, 50.0), line(100.0, 20.0)];
        assert_eq!(max_first(&lines, 220.0), 1);
    }

    #[test]
    #[should_panic(expected = "a sheet line is")]
    fn a_line_taller_than_the_dock_panics_rather_than_vanishing() {
        let _ = max_first(&[line(400.0, 0.0)], 300.0);
    }

    #[test]
    fn the_dock_holds_the_hero_in_hand_over_whatever_is_pointed_at() {
        let (_, house) = house();
        let brannoc = id(&house.heroes, "Brannoc");
        let garrick = id(&house.heroes, "Garrick");
        let held = UiState {
            pointing: Some(garrick),
            drag: Some(Drag {
                hero: brannoc,
                from: crate::board::Slot::Roster(3),
                at: Vec2::ZERO,
                over: None,
            }),
            ..UiState::default()
        };
        assert_eq!(subject(&held), Subject::Hero(brannoc));
        assert_eq!(subject(&UiState::default()), Subject::Help);
        let quest = UiState {
            pointing_quest: Some(1),
            ..UiState::default()
        };
        assert_eq!(subject(&quest), Subject::Quest(1));
    }

    #[test]
    fn a_scrolled_sheet_draws_whole_lines_from_its_first_and_reports_them() {
        let (content, house) = house();
        let garrick = id(&house.heroes, "Garrick");
        let ui = UiState {
            pointing: Some(garrick),
            dock_first: 5,
            ..UiState::default()
        };
        let mut page = Page::default();
        lay_out(&mut page, &content, &house, &ui);
        let lines: Vec<usize> = page.rows.iter().filter_map(|r| r.dock_line).collect();
        assert_eq!(page.dock.first, 5);
        assert_eq!(lines.first(), Some(&5));
        assert_eq!(lines.last(), Some(&(5 + page.dock.shown - 1)));
        assert!(
            page.rows
                .iter()
                .filter(|r| r.dock_line.is_some())
                .all(|r| text_rect().contains_rect(r.bounds()))
        );
    }
}
