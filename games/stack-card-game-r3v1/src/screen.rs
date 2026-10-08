//! The table as data: every row of text the screen says (a `jidousha::ui`
//! `Panel`), every box under it, the target marks, and the hit-test — all from
//! one set of layout functions, so what is drawn and what a click lands on
//! cannot disagree.
//!
//! Design space is 960 x 540 and one world unit is one design unit (`Flat`).
//!
//! Key functions: `screen`, `boxes`, `marks`, `hit`, `stack_row`, `hand_card`.

use jidousha::prelude::*;
use jidousha::ui::{Cell, Icon, Mapping, Panel, TextRun, centered};

use crate::rules::{Duel, Outcome, Preview, Side, legal_targets, preview};
use crate::{Intent, Table, card_title, layers, palette};

/// The game draws no pictures: a role with no members.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Art {}

impl Icon for Art {
    fn size_at(self, _scale: f32) -> Vec2 {
        match self {}
    }
}

/// One design unit is one world unit.
pub(crate) struct Flat;

impl Mapping for Flat {
    fn to_world(&self, ui: Vec2) -> Vec2 {
        ui
    }
    fn scale(&self) -> f32 {
        1.0
    }
}

/// The stack panel's frame.
pub(crate) const STACK_PANEL: Rect = Rect {
    min: Vec2::new(16.0, 34.0),
    max: Vec2::new(604.0, 396.0),
};
/// Where the first (top) stack row starts, and how far apart rows are.
const STACK_FIRST: Vec2 = Vec2::new(24.0, 62.0);
const ROW_STEP: f32 = 46.0;
const ROW_SIZE: Vec2 = Vec2::new(572.0, 42.0);
/// Rows the panel shows; the rules cap a stack at six plays a round.
pub(crate) const STACK_ROWS: usize = 6;
/// The outcome line under the rows.
const OUTCOME_AT: Vec2 = Vec2::new(24.0, 340.0);

/// Your hand.
const HAND_FIRST: Vec2 = Vec2::new(20.0, 424.0);
const CARD_SIZE: Vec2 = Vec2::new(124.0, 90.0);
const CARD_STEP: f32 = 132.0;

/// The right-hand column: priority, the pass button, the log.
const RIGHT: f32 = 620.0;
const RIGHT_WIDTH: f32 = 324.0;
pub(crate) const PASS_BUTTON: Rect = Rect {
    min: Vec2::new(620.0, 112.0),
    max: Vec2::new(780.0, 146.0),
};
/// How many resolutions the log shows.
const LOG_LINES: usize = 8;

pub(crate) fn style(size: f32, color: Color) -> TextStyle {
    TextStyle {
        face: Face::BUILT_IN,
        size,
        color,
        depth: Depth::layer(layers::TEXT),
    }
}

fn side_color(side: Side) -> Color {
    match side {
        Side::You => palette::YOU,
        Side::Rival => palette::RIVAL,
    }
}

/// The box of the stack row at `slot` (0 = top).
pub(crate) fn stack_row(slot: usize) -> Rect {
    Rect::from_min_size(
        STACK_FIRST + Vec2::new(0.0, ROW_STEP * slot as f32),
        ROW_SIZE,
    )
}

/// The box of hand card `index`.
pub(crate) fn hand_card(index: usize) -> Rect {
    Rect::from_min_size(
        HAND_FIRST + Vec2::new(CARD_STEP * index as f32, 0.0),
        CARD_SIZE,
    )
}

/// What a click at world point `at` asks for.
pub(crate) fn hit(at: Vec2) -> Option<Intent> {
    if PASS_BUTTON.contains(at) {
        return Some(Intent::Pass);
    }
    if let Some(slot) = (0..STACK_ROWS).find(|&slot| stack_row(slot).contains(at)) {
        return Some(Intent::Slot(slot));
    }
    (0..crate::rules::HAND_LIMIT)
        .find(|&index| hand_card(index).contains(at))
        .map(Intent::Card)
}

/// "1st", "2nd", ...
pub(crate) fn ordinal(place: usize) -> String {
    let n = place + 1;
    let suffix = match n {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// A stack slot's letter (0 = A).
pub(crate) fn letter(slot: usize) -> char {
    char::from(b'A' + slot as u8)
}

/// The slot an item id sits in, counted from the top.
fn slot_of(duel: &Duel, id: crate::rules::ItemId) -> Option<usize> {
    duel.position(id).map(|at| duel.stack.len() - 1 - at)
}

/// The two lines a stack row says: what it is, and what it will do when.
pub(crate) fn row_lines(duel: &Duel, ahead: &Preview, slot: usize) -> (String, String) {
    let item = duel.stack[duel.stack.len() - 1 - slot];
    let copy = if item.copy { " copy" } else { "" };
    let aim = match item.target {
        Some(id) => match slot_of(duel, id) {
            Some(target) => format!(" -> {}", letter(target)),
            None => " -> gone".to_owned(),
        },
        None => String::new(),
    };
    let first = format!(
        "{}  {}{copy} ({}){aim}",
        letter(slot),
        item.card.name(),
        item.controller.name()
    );
    let second = match ahead.order_of(item.id) {
        Some(place) => format!(
            "resolves {}: {}",
            ordinal(place),
            ahead.steps[place].describe()
        ),
        None => "does not resolve".to_owned(),
    };
    (first, second)
}

/// The line under the stack: where life stands if both pass all the way.
pub(crate) fn outcome_line(ahead: &Preview) -> String {
    format!(
        "if both pass: you {} - rival {}",
        ahead.you_life, ahead.rival_life
    )
}

/// The priority indicator's second line.
pub(crate) fn priority_note(duel: &Duel) -> String {
    let other = duel.priority.opponent().name();
    if !duel.passed {
        return format!("{other} may still respond");
    }
    if duel.stack.is_empty() {
        format!("{other} passed: a pass ends the round")
    } else {
        format!("{other} passed: a pass resolves A")
    }
}

fn stats_line(duel: &Duel, side: Side) -> String {
    let who = duel.side(side);
    format!(
        "{}  life {}  shield {}  focus {}/{}  hand {}  deck {}",
        side.name().to_uppercase(),
        who.life,
        who.shield,
        who.focus,
        crate::rules::FOCUS_PER_ROUND,
        who.hand.len(),
        who.deck.len()
    )
}

/// Every row of text the table says, as data.
pub(crate) fn screen(table: &Table) -> Panel<Art> {
    let duel = &table.duel;
    let ahead = preview(duel);
    let mut panel = Panel::default();
    let row = style(14.0, palette::INK);
    let small = style(12.0, palette::FAINT);

    // Top bar.
    panel.text(TextRun::new(
        Vec2::new(20.0, 10.0),
        stats_line(duel, Side::Rival),
        style(14.0, palette::RIVAL),
    ));
    let round = format!("round {} - {} leads", duel.round, duel.leader.name());
    panel.text(TextRun::new(
        Vec2::new(940.0 - row.width_of(&round), 10.0),
        round,
        row,
    ));

    // The stack panel.
    panel.text(TextRun::new(
        STACK_PANEL.min + Vec2::new(8.0, 6.0),
        "THE STACK - top resolves first",
        style(16.0, palette::INK),
    ));
    if duel.stack.is_empty() {
        panel.text(TextRun::new(
            STACK_FIRST + Vec2::new(10.0, 12.0),
            "empty",
            small,
        ));
    }
    let aiming = aim_targets(table);
    for slot in 0..duel.stack.len().min(STACK_ROWS) {
        let at = stack_row(slot).min;
        let item = duel.stack[duel.stack.len() - 1 - slot];
        let (first, second) = row_lines(duel, &ahead, slot);
        panel.text(
            Cell::new(at + Vec2::new(10.0, 5.0), 440.0)
                .run(&first, style(14.0, side_color(item.controller))),
        );
        panel.text(Cell::new(at + Vec2::new(30.0, 24.0), 530.0).run(&second, small));
        if aiming.contains(&slot) {
            let tag = "<- target";
            let mark = TextStyle {
                depth: Depth::layer(layers::MARKS),
                ..style(14.0, palette::MARK)
            };
            panel.text(TextRun::new(
                Vec2::new(
                    stack_row(slot).max.x - mark.width_of(tag) - 10.0,
                    at.y + 5.0,
                ),
                tag,
                mark,
            ));
        }
    }
    panel.text(TextRun::new(OUTCOME_AT, outcome_line(&ahead), row));
    let copies = ahead
        .steps
        .iter()
        .filter(|step| step.item.copy && duel.position(step.item.id).is_none());
    for (line, step) in copies.take(2).enumerate() {
        let text = format!(
            "then {} copy ({}): {}",
            step.item.card.name(),
            step.item.controller.name(),
            step.describe()
        );
        panel.text(
            Cell::new(
                OUTCOME_AT + Vec2::new(0.0, 20.0 + 16.0 * line as f32),
                560.0,
            )
            .run(&text, small),
        );
    }

    // Priority, the pass button, the prompt.
    panel.text(TextRun::new(
        Vec2::new(RIGHT, 40.0),
        "PRIORITY",
        style(16.0, palette::INK),
    ));
    panel.text(TextRun::new(
        Vec2::new(RIGHT, 62.0),
        duel.priority.name().to_uppercase(),
        style(24.0, side_color(duel.priority)),
    ));
    panel.text(Cell::new(Vec2::new(RIGHT, 92.0), RIGHT_WIDTH).run(&priority_note(duel), small));
    let label = "PASS (space)";
    panel.text(TextRun::new(
        centered(PASS_BUTTON, &row, label, PASS_BUTTON.min.y + 10.0),
        label,
        row,
    ));
    let prompt = if table.choosing.is_some() {
        "pick a lit item: A-F, Esc cancels"
    } else if duel.priority == Side::Rival {
        "the rival is reading the stack"
    } else {
        "play a card (1-7) or pass"
    };
    panel.text(Cell::new(Vec2::new(RIGHT, 156.0), RIGHT_WIDTH).run(prompt, small));

    // The log, newest first.
    panel.text(TextRun::new(
        Vec2::new(RIGHT, 186.0),
        "LOG",
        style(14.0, palette::INK),
    ));
    for (line, step) in duel.resolved.iter().rev().take(LOG_LINES).enumerate() {
        let text = format!(
            "{} {}: {}",
            step.item.controller.name(),
            step.item.card.name(),
            step.describe()
        );
        panel.text(
            Cell::new(Vec2::new(RIGHT, 206.0 + 16.0 * line as f32), RIGHT_WIDTH).run(&text, small),
        );
    }

    // Your side.
    panel.text(TextRun::new(
        Vec2::new(20.0, 404.0),
        stats_line(duel, Side::You),
        style(14.0, palette::YOU),
    ));
    for (index, &card) in duel.you.hand.iter().enumerate() {
        let at = hand_card(index).min;
        let ink = if playable(table, index) {
            palette::INK
        } else {
            palette::FAINT
        };
        panel.text(TextRun::new(
            at + Vec2::new(6.0, 6.0),
            card_title(index, card),
            style(14.0, ink),
        ));
        panel.text(TextRun::new(
            at + Vec2::new(6.0, 28.0),
            format!("cost {}", card.cost()),
            small,
        ));
        panel.text(TextRun::new(at + Vec2::new(6.0, 46.0), card.blurb(), small));
        if table.choosing == Some(index) {
            panel.text(TextRun::new(
                at + Vec2::new(6.0, 68.0),
                "aiming",
                style(12.0, palette::MARK),
            ));
        }
    }
    panel.text(TextRun::new(
        Vec2::new(20.0, 520.0),
        "1-7 play a card   A-F pick a target   SPACE pass   Esc cancel   or click",
        style(12.0, palette::FAINT),
    ));

    if let Some(outcome) = duel.outcome {
        panel.absorb(result(duel, outcome));
    }
    panel
}

/// The result screen's words, on the overlay band.
fn result(duel: &Duel, outcome: Outcome) -> Panel<Art> {
    let mut panel = Panel::default();
    let banner = match outcome {
        Outcome::Won(Side::You) => "YOU WIN",
        Outcome::Won(Side::Rival) => "THE RIVAL WINS",
        Outcome::Draw => "A DRAW",
    };
    let lines = [
        (banner.to_owned(), 32.0, 196.0),
        (
            format!(
                "you {} - rival {}, round {}",
                duel.you.life, duel.rival.life, duel.round
            ),
            16.0,
            244.0,
        ),
        ("press ENTER for a rematch".to_owned(), 14.0, 276.0),
    ];
    for (text, size, y) in lines {
        let ink = TextStyle {
            depth: Depth::layer(layers::OVERLAY_TEXT),
            ..style(size, palette::INK)
        };
        panel.text(TextRun::new(
            Vec2::new(480.0 - ink.width_of(&text) * 0.5, y),
            text,
            ink,
        ));
    }
    panel
}

/// The result screen's box, centred.
pub(crate) const RESULT_BOX: Rect = Rect {
    min: Vec2::new(250.0, 176.0),
    max: Vec2::new(710.0, 306.0),
};

/// Whether hand card `index` could be played right now.
fn playable(table: &Table, index: usize) -> bool {
    let duel = &table.duel;
    let Some(&card) = duel.you.hand.get(index) else {
        return false;
    };
    duel.priority == Side::You
        && duel.outcome.is_none()
        && card.cost() <= duel.you.focus
        && (!card.needs_target() || !legal_targets(duel, Side::You, card).is_empty())
}

/// The stack slots marked as legal targets while you aim (0 = top) — read off
/// `legal_targets`, the rule resolution uses.
pub(crate) fn aim_targets(table: &Table) -> Vec<usize> {
    let duel = &table.duel;
    let Some(card) = table.choosing.and_then(|index| duel.you.hand.get(index)) else {
        return Vec::new();
    };
    let mut slots: Vec<usize> = legal_targets(duel, Side::You, *card)
        .into_iter()
        .filter_map(|id| slot_of(duel, id))
        .collect();
    slots.sort_unstable();
    slots
}

/// Every box under the text: panels, stack rows, cards, the pass button.
pub(crate) fn boxes(table: &Table) -> Vec<(Rect, Color, i16)> {
    let duel = &table.duel;
    let mut all = vec![
        (STACK_PANEL, palette::PANEL, layers::BACK),
        (PASS_BUTTON, palette::BUTTON, layers::CARDS),
    ];
    for slot in 0..duel.stack.len().min(STACK_ROWS) {
        let tone = if slot == 0 {
            palette::TOP_ROW
        } else {
            palette::ROW
        };
        all.push((stack_row(slot), tone, layers::CARDS));
    }
    for index in 0..duel.you.hand.len() {
        let tone = if table.choosing == Some(index) {
            palette::CARD_AIMING
        } else if playable(table, index) {
            palette::CARD
        } else {
            palette::CARD_DIM
        };
        all.push((hand_card(index), tone, layers::CARDS));
    }
    if duel.outcome.is_some() {
        let view = Rect::from_min_size(Vec2::ZERO, Vec2::new(crate::VIEW_W, crate::VIEW_H));
        all.push((view, palette::DIMMER, layers::OVERLAY));
        all.push((RESULT_BOX, palette::PANEL, layers::OVERLAY));
    }
    all
}

/// The rows to outline as legal targets.
pub(crate) fn marks(table: &Table) -> Vec<Rect> {
    aim_targets(table).into_iter().map(stack_row).collect()
}

/// Thickness of a target mark's outline.
pub(crate) const MARK_LINE: f32 = 3.0;

/// Four lines around `rect`, in the mark colour, over the row.
pub(crate) fn outline(ctx: &mut DrawCtx, rect: Rect) {
    let corners = [
        rect.min,
        Vec2::new(rect.max.x, rect.min.y),
        rect.max,
        Vec2::new(rect.min.x, rect.max.y),
    ];
    for index in 0..4 {
        ctx.line(
            corners[index],
            corners[(index + 1) % 4],
            MARK_LINE,
            palette::MARK,
            Depth::layer(layers::MARKS),
        );
    }
}
