//! The hearth screen (SPEC §11.1-§11.2): the top bar with "Let the winter pass", the
//! hall and the yard down the left, the twelve winter seats in their five groups where the
//! summer's board stands, and the sheet dock down the right edge.
//!
//! **The previews are the plan.** Every seat's note — the fire's "heals", the training
//! yard's "+1 Spirit" or its excuse, the garden's verdict, the table's "house +", a
//! bench's lesson — is read off `winter::plan`, the record the winter's resolution
//! writes as it carries the same plans out (`plans.rs`). Mid-drag the screen shows
//! the seating the release would make — the hand at its landing, a swap's displaced
//! hero where the hand came from, or the hand lifted out of its seat if it would land
//! nowhere — and previews that (SPEC-GAPS KG-41). Hit targets are always the real
//! seating's, so what is under the pointer never moves while it is held.

use std::borrow::Cow;

use jidousha::prelude::*;

use crate::board::Slot;
use crate::board_view::{TILE, draw_hand, tile};
use crate::constants::{BENCHES, FIRE_SEATS, ROSTER_SEATS, TALE_SEATS, YARD_SPOTS};
use crate::content::Content;
use crate::hearth::{Group, Seat};
use crate::hearth_help::group_words;
use crate::house::House;
use crate::screen::{MIN_TEXT, Page, Target, UiState, ink, layers, wrap};
use crate::summer::{
    BOARD, LEFT_X, ROSTER_TOP, button, card_rect, hero_card, lay_out_top_bar, screen_rect, yard_top,
};
use crate::winter::{WinterPlan, plan};
use crate::words::W;

/// "Let the winter pass": under the benches, the right column's width. The top bar
/// has no room for its label beside the Door's countdown and "The family".
pub const PASS_BUTTON: Rect = Rect {
    min: Vec2::new(
        BOARD.min.x + (BOARD.max.x - BOARD.min.x - GAP) * 0.5 + GAP,
        BOARD.min.y + (PAIR_H + GAP) + BENCHES_H + 2.0 * GAP,
    ),
    max: Vec2::new(
        BOARD.max.x,
        BOARD.min.y + (PAIR_H + GAP) + BENCHES_H + 2.0 * GAP + 44.0,
    ),
};
/// The gap between the groups' panels.
const GAP: f32 = 8.0;
/// A panel's inner margin.
const PANEL_PAD: f32 = 10.0;
/// The line pitch of 14 px type.
const PITCH: f32 = 17.0;
/// Where a group's seats begin, below its heading and two lines of its text.
const SEATS_TOP: f32 = 30.0 + 2.0 * PITCH + 6.0;
/// A seat's label row ("learner", "teacher", "child").
const LABEL_H: f32 = PITCH;
/// A two-seat group's panel: heading, text, labels, tiles, a note under each.
const PAIR_H: f32 = SEATS_TOP + LABEL_H + 60.0 + 4.0 + 2.0 * PITCH + PANEL_PAD;
/// The benches' panel: two rows of a child and a teacher.
const BENCHES_H: f32 = SEATS_TOP + 2.0 * (LABEL_H + 60.0) + 12.0 + PANEL_PAD;

/// The groups of winter seats, in the order their panels are laid out.
pub const GROUPS: [Group; 5] = [
    Group::Fire,
    Group::Training,
    Group::Garden,
    Group::Table,
    Group::Benches,
];

/// Where `group`'s panel sits: the fire, the training yard and the garden down the
/// board's left half; the long table and the benches down its right.
pub fn group_rect(group: Group) -> Rect {
    let width = (BOARD.size().x - GAP) * 0.5;
    let left = BOARD.min.x;
    let right = BOARD.min.x + width + GAP;
    let at = |x: f32, y: f32, h: f32| Rect::from_min_size(Vec2::new(x, y), Vec2::new(width, h));
    let row = |n: f32| BOARD.min.y + n * (PAIR_H + GAP);
    match group {
        Group::Fire => at(left, row(0.0), PAIR_H),
        Group::Training => at(left, row(1.0), PAIR_H),
        Group::Garden => at(left, row(2.0), PAIR_H),
        Group::Table => at(right, row(0.0), PAIR_H),
        Group::Benches => at(right, row(1.0), BENCHES_H),
        // The hall is the left column's, not a panel here.
        Group::Hall => hall_rect(),
    }
}

/// The hall's region: its heading and its twelve seats.
pub fn hall_rect() -> Rect {
    let last = card_rect(ROSTER_TOP, ROSTER_SEATS - 1);
    Rect {
        min: Vec2::new(LEFT_X, ROSTER_TOP - 20.0),
        max: Vec2::new(last.max.x, last.max.y),
    }
}

/// A group's seats, as rows of up to two, each seat with its label key if it has one.
fn rows(group: Group) -> Vec<[(Seat, Option<W>); 2]> {
    match group {
        Group::Fire => vec![[(Seat::Fire(0), None), (Seat::Fire(1), None)]],
        Group::Training => vec![[
            (Seat::Learner, Some(W::WinterLearner)),
            (Seat::Teacher, Some(W::WinterTeacher)),
        ]],
        Group::Garden => vec![[(Seat::Garden(0), None), (Seat::Garden(1), None)]],
        Group::Table => vec![[(Seat::Table(0), None), (Seat::Table(1), None)]],
        Group::Benches => (0..BENCHES)
            .map(|b| {
                [
                    (Seat::BenchChild(b), Some(W::WinterChild)),
                    (Seat::BenchTeacher(b), Some(W::WinterBenchTeacher)),
                ]
            })
            .collect(),
        Group::Hall => Vec::new(),
    }
}

/// Where `seat`'s tile sits.
pub fn seat_rect(seat: Seat) -> Rect {
    if let Seat::Yard(spot) = seat {
        return card_rect(yard_top(), spot);
    }
    for group in GROUPS {
        for (r, row) in rows(group).iter().enumerate() {
            for (i, (s, _)) in row.iter().enumerate() {
                if *s == seat {
                    let panel = group_rect(group);
                    let y = panel.min.y + SEATS_TOP + LABEL_H + r as f32 * (LABEL_H + 60.0 + 12.0);
                    let x = panel.min.x + PANEL_PAD + i as f32 * (TILE.x + GAP);
                    return Rect::from_min_size(Vec2::new(x, y), TILE);
                }
            }
        }
    }
    panic!(
        "[keifu_x_inheritance_r2] {seat:?} has no place on the hearth screen\n  likely cause: a seat added \
         to Seat without a row in hearth_view::rows\n  fix: give it a row"
    )
}

/// What each winter seat's preview says (SPEC §11.2), read off the plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Notes {
    /// Under each fire seat: "heals", "calms", "rests".
    pub fire: [Option<String>; FIRE_SEATS],
    /// Beside the training yard: the lesson, or "no learner".
    pub training: Option<String>,
    /// Beside the garden: the verdict.
    pub garden: Option<String>,
    /// Under each table seat: "house +", "own +", "a child".
    pub table: [Option<String>; TALE_SEATS],
    /// Beside each bench: the lesson, or "no child".
    pub benches: [Option<String>; BENCHES],
}

/// The notes a plan shows, given who sits in the hearth `house` holds. SPEC-GAPS KG-44:
/// a pair's lesson when its learner's seat is filled, "no learner" / "no child" when
/// only its teacher sits.
pub fn notes(content: &Content, house: &House, plan: &WinterPlan) -> Notes {
    let words = &content.words;
    let sat = |seat| house.hearth.at(seat).is_some();
    let pair = |lesson: Option<crate::plans::Lesson>, other: Seat, alone: W| match lesson {
        Some(lesson) => Some(lesson.note(content)),
        None if sat(other) => Some(words[alone].to_owned()),
        None => None,
    };
    Notes {
        fire: plan.rests.map(|r| r.map(|r| r.note(content).to_owned())),
        training: pair(plan.yard, Seat::Teacher, W::WinterNoLearner),
        garden: plan
            .garden
            .map(|c| c.note(content))
            .filter(|note| !note.is_empty()),
        table: plan.tellers.map(|t| t.map(|t| t.note(content).to_owned())),
        benches: std::array::from_fn(|b| {
            pair(plan.benches[b], Seat::BenchTeacher(b), W::WinterNoChild)
        }),
    }
}

/// SPEC-GAPS KG-41: the house as the release would leave it: the hand at its landing,
/// or lifted out of its seat if it would land nowhere. Without a drag, the house.
pub fn previewed<'h>(house: &'h House, ui: &UiState) -> Cow<'h, House> {
    let Some(drag) = ui.drag else {
        return Cow::Borrowed(house);
    };
    let mut preview = house.clone();
    match house.landing(drag.hero, drag.over) {
        Some(onto) => {
            preview.drop_hero(drag.from, Some(onto));
        }
        None => preview.unseat(drag.hero),
    }
    Cow::Owned(preview)
}

/// The notes on screen now: the previewed seating's plan.
pub fn notes_now(content: &Content, house: &House, ui: &UiState) -> Notes {
    let shown = previewed(house, ui);
    notes(content, &shown, &plan(content, &shown))
}

/// `text` wrapped into `width` from `at`, one logical line; returns the y below it.
fn paragraph(page: &mut Page, text: &str, at: Vec2, width: f32, color: Color, panel: Rect) -> f32 {
    let mut y = at.y;
    for (index, piece) in wrap(text, width, MIN_TEXT).into_iter().enumerate() {
        let at = Vec2::new(at.x, y);
        if index == 0 {
            page.text(layers::TEXT, at, piece, MIN_TEXT, color, panel);
        } else {
            page.continue_text(layers::TEXT, at, piece, MIN_TEXT, color, panel);
        }
        y += PITCH;
    }
    y
}

/// Lay the hearth out.
pub fn lay_out(page: &mut Page, content: &Content, house: &House, ui: &UiState) {
    let words = &content.words;
    lay_out_top_bar(page, content, house);
    button(
        page,
        PASS_BUTTON,
        &words[W::WinterButton],
        Target::LetWinterPass,
        layers::PANEL,
    );
    let shown = previewed(house, ui);
    let in_hand = ui.drag.map(|drag| drag.hero);
    // Targets are the real seating's: a seated hero, else the seat.
    let target = |slot: Slot| match house.hero_in(slot) {
        Some(id) if in_hand != Some(id) => Target::Hero(id),
        _ => Target::Seat(slot),
    };
    // The hall and the yard, down the left.
    let screen = screen_rect();
    page.text(
        layers::TEXT,
        Vec2::new(LEFT_X, ROSTER_TOP - 20.0),
        &words[W::WinterHall],
        MIN_TEXT,
        ink::HEADING,
        screen,
    );
    for seat in 0..ROSTER_SEATS {
        let slot = Slot::Roster(seat);
        card_at(
            page,
            content,
            &shown,
            ui,
            slot,
            card_rect(ROSTER_TOP, seat),
            target(slot),
        );
    }
    page.text(
        layers::TEXT,
        Vec2::new(LEFT_X, yard_top() - 20.0),
        &words[W::WinterChildren],
        MIN_TEXT,
        ink::HEADING,
        screen,
    );
    for spot in 0..YARD_SPOTS {
        let slot = Slot::Hearth(Seat::Yard(spot));
        card_at(
            page,
            content,
            &shown,
            ui,
            slot,
            card_rect(yard_top(), spot),
            target(slot),
        );
    }
    page.targets.push((hall_rect(), Target::Group(Group::Hall)));
    // The twelve winter seats, with their previews.
    let notes = notes_now(content, house, ui);
    for group in GROUPS {
        lay_out_group(page, content, &shown, ui, group, &notes, &target);
    }
    crate::dock::lay_out(page, content, house, ui);
    draw_hand(page, content, house, ui);
}

/// A hall or yard card: the hero sitting there in the previewed seating, or an empty seat.
fn card_at(
    page: &mut Page,
    content: &Content,
    shown: &House,
    ui: &UiState,
    slot: Slot,
    rect: Rect,
    target: Target,
) {
    match shown.hero_in(slot) {
        Some(id) => {
            hero_card(
                page,
                content,
                &shown.heroes,
                id,
                rect,
                ui.pointing == Some(id),
            );
            // `hero_card` pushed the hero's own target; the real seating's replaces it.
            if let Some(last) = page.targets.last_mut() {
                *last = (rect, target);
            }
        }
        None => {
            page.shape(rect, ink::PANEL, layers::PANEL);
            page.targets.push((rect, target));
        }
    }
}

/// One group's panel: heading, text, seats with their labels, and the notes.
fn lay_out_group(
    page: &mut Page,
    content: &Content,
    shown: &House,
    ui: &UiState,
    group: Group,
    notes: &Notes,
    target: &dyn Fn(Slot) -> Target,
) {
    let panel = group_rect(group);
    let hot = ui.pointing_group == Some(group);
    page.shape(
        panel,
        if hot { ink::HOT } else { ink::PANEL },
        layers::PANEL,
    );
    let (heading, text, _) = group_words(content, group);
    let x = panel.min.x + PANEL_PAD;
    let width = panel.size().x - 2.0 * PANEL_PAD;
    page.text(
        layers::TEXT,
        Vec2::new(x, panel.min.y + 8.0),
        heading,
        MIN_TEXT,
        ink::HEADING,
        panel,
    );
    paragraph(
        page,
        &text,
        Vec2::new(x, panel.min.y + 28.0),
        width,
        ink::NOTE,
        panel,
    );
    for (r, row) in rows(group).iter().enumerate() {
        for (seat, label) in row {
            let rect = seat_rect(*seat);
            if let Some(label) = label {
                page.text(
                    layers::TEXT,
                    Vec2::new(rect.min.x, rect.min.y - LABEL_H),
                    &content.words[*label],
                    MIN_TEXT,
                    ink::NOTE,
                    panel,
                );
            }
            match shown.hearth.at(*seat) {
                Some(id) => tile(page, content, shown, id, rect),
                None => page.shape(rect, ink::PAGE, layers::MARK),
            }
            page.targets.push((rect, target(Slot::Hearth(*seat))));
            let under = match seat {
                Seat::Fire(i) => notes.fire[*i].as_deref(),
                Seat::Table(i) => notes.table[*i].as_deref(),
                _ => None,
            };
            if let Some(note) = under {
                note_under(page, note, rect, panel);
            }
        }
        let beside = match group {
            Group::Training => notes.training.as_deref(),
            Group::Garden => notes.garden.as_deref(),
            Group::Benches => notes.benches[r].as_deref(),
            _ => None,
        };
        if let Some(note) = beside {
            let last = seat_rect(row[1].0);
            let at = Vec2::new(last.max.x + GAP + 4.0, last.min.y + 4.0);
            paragraph(
                page,
                note,
                at,
                panel.max.x - PANEL_PAD - at.x,
                ink::GOLD,
                panel,
            );
        }
    }
    page.targets.push((panel, Target::Group(group)));
}

/// A seat's own note, centred under its tile.
fn note_under(page: &mut Page, note: &str, tile: Rect, panel: Rect) {
    let width = TextStyle {
        size: MIN_TEXT,
        ..TextStyle::default()
    }
    .width_of(note);
    let at = Vec2::new(tile.center().x - width * 0.5, tile.max.y + 4.0);
    page.text(layers::TEXT, at, note, MIN_TEXT, ink::GOLD, panel);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_pair_reads_left_to_right_learner_before_teacher_and_child_before_teacher() {
        let pairs = [
            (Seat::Fire(0), Seat::Fire(1)),
            (Seat::Learner, Seat::Teacher),
            (Seat::Garden(0), Seat::Garden(1)),
            (Seat::Table(0), Seat::Table(1)),
            (Seat::BenchChild(0), Seat::BenchTeacher(0)),
            (Seat::BenchChild(1), Seat::BenchTeacher(1)),
        ];
        for (left, right) in pairs {
            let (a, b) = (seat_rect(left), seat_rect(right));
            assert!(
                a.max.x < b.min.x && a.min.y == b.min.y,
                "{left:?} {right:?}"
            );
        }
        assert!(seat_rect(Seat::BenchChild(0)).max.y < seat_rect(Seat::BenchChild(1)).min.y);
    }

    #[test]
    fn the_pass_control_sits_under_the_benches_inside_the_board() {
        let benches = group_rect(Group::Benches);
        assert!(PASS_BUTTON.min.y > benches.max.y && PASS_BUTTON.min.x == benches.min.x);
        assert!(BOARD.contains_rect(PASS_BUTTON));
    }

    #[test]
    fn every_winter_seat_has_a_tile_inside_its_group_and_no_two_tiles_overlap() {
        let seats: Vec<Seat> = crate::hearth::Seat::all()
            .into_iter()
            .filter(|s| !matches!(s, Seat::Yard(_)))
            .collect();
        assert_eq!(seats.len(), 12);
        for (i, a) in seats.iter().enumerate() {
            let rect = seat_rect(*a);
            assert!(
                GROUPS
                    .iter()
                    .any(|g| { group_rect(*g).contains_rect(rect) }),
                "{a:?}"
            );
            for b in &seats[i + 1..] {
                let other = seat_rect(*b);
                let apart = rect.max.x <= other.min.x
                    || other.max.x <= rect.min.x
                    || rect.max.y <= other.min.y
                    || other.max.y <= rect.min.y;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
        for group in GROUPS {
            let panel = group_rect(group);
            assert!(BOARD.contains_rect(panel), "{group:?}");
        }
    }
}
