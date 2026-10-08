//! The screen, as data: every row of text in one `Panel`, every box in one
//! list, every tappable rectangle in one table.
//!
//! Laid out in a 960x540 design space that the camera shows 1:1 (`camera()`),
//! so a design unit is a world unit and the `Mapping` is the identity — the
//! draw, the floors and the frame check read the same numbers.

use jidousha::prelude::*;
use jidousha::ui::{Cell, Panel, TextRun, centered, clipped, wrap};

use crate::duel::{Duel, Effect, Event, Outcome, Side, TURNS, legal_targets};
use crate::resolve::{describe, preview};

// The layout is the screen's vocabulary; every reader of the screen reaches
// it through here.
pub(crate) use crate::layout::*;

/// The player's half-finished play: one value, never a flag beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Pick {
    /// Which hand slot.
    pub(crate) slot: usize,
    /// Which of the card's legal targets the cursor is on, top first.
    pub(crate) aim: usize,
}

/// What the screen shows besides the duel.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Ui {
    pub(crate) pick: Option<Pick>,
    /// The last thing the rules refused, said once.
    pub(crate) notice: Option<String>,
}

fn style(size: f32, color: Color, layer: i16) -> TextStyle {
    TextStyle {
        size,
        color,
        depth: Depth::layer(layer),
        ..TextStyle::default()
    }
}

fn tone(side: Side) -> Color {
    match side {
        Side::You => YOU_TONE,
        Side::Npc => NPC_TONE,
    }
}

/// The ids the current pick may be aimed at, top first — `legal_targets`, or
/// nothing when no card that aims is picked.
pub(crate) fn marked(duel: &Duel, ui: &Ui) -> Vec<u32> {
    let Some(pick) = ui.pick else {
        return Vec::new();
    };
    let Some(&card) = duel.seat(Side::You).hand.get(pick.slot) else {
        return Vec::new();
    };
    if card.spec().aim == crate::cards::Aim::Nothing {
        return Vec::new();
    }
    legal_targets(duel, card)
}

/// The id the aim cursor is on, if a targeting card is picked.
pub(crate) fn aimed(duel: &Duel, ui: &Ui) -> Option<u32> {
    let pick = ui.pick?;
    marked(duel, ui).get(pick.aim).copied()
}

/// One stack row's words: where it falls in the resolution order, and what
/// the preview says it will do.
pub(crate) struct StackLine {
    pub(crate) id: u32,
    /// 1 for the first to resolve; `None` if it leaves the stack unresolved.
    pub(crate) order: Option<usize>,
    pub(crate) text: String,
}

/// The stack, top first, read through `preview` — the panel's rows.
pub(crate) fn stack_lines(duel: &Duel) -> Vec<StackLine> {
    let steps = preview(duel);
    duel.stack
        .iter()
        .rev()
        .map(|item| {
            let found = steps.iter().position(|step| step.item == item.id);
            let fate = match found {
                Some(at) => describe(steps[at].effect),
                None => "countered - never resolves".to_owned(),
            };
            let aim = item.target.map_or(String::new(), |id| format!("@#{id} "));
            let copy = if item.copy { "*" } else { "" };
            StackLine {
                id: item.id,
                order: found.map(|at| at + 1),
                text: format!(
                    "{:<4} #{:<3} {:<7} {:<7}{}-> {}",
                    found.map_or("--".to_owned(), |at| ordinal(at + 1)),
                    item.id,
                    format!("{}{copy}", item.card.name()),
                    item.caster.name(),
                    aim,
                    fate
                ),
            }
        })
        .collect()
}

fn ordinal(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// What a pass does from here, in words — the priority box's second line.
pub(crate) fn pass_line(duel: &Duel) -> String {
    let other = duel.priority.other().name();
    match (duel.passes, duel.stack.is_empty()) {
        (0, true) => format!("pass: {other} may play"),
        (0, false) => format!("pass: {other} may respond"),
        (_, true) => "pass: the turn ends".to_owned(),
        (_, false) => "pass: top item resolves".to_owned(),
    }
}

/// The preview's bottom line: life now, and life when the stack is done.
pub(crate) fn outlook_line(duel: &Duel) -> String {
    let [you, npc] = duel.life();
    let [you_after, npc_after] = preview(duel).last().map_or([you, npc], |step| step.life);
    format!("if all pass: WEAVER {you} -> {you_after}   BRUTE {npc} -> {npc_after}")
}

/// One feed row per event worth reading, newest first.
fn feed_lines(duel: &Duel) -> Vec<(String, Color)> {
    duel.log
        .iter()
        .rev()
        .filter_map(|event| match *event {
            Event::Played {
                side,
                card,
                item,
                target,
            } => {
                let aim = target.map_or(String::new(), |id| format!(" at #{id}"));
                Some((
                    format!("{} plays {} #{item}{aim}", side.name(), card.name()),
                    tone(side),
                ))
            }
            Event::Resolved(step) => {
                // The card's name when the row has room for it; the id alone
                // names the item when it does not, so nothing is cut short.
                let full = format!(
                    "#{} {}: {}",
                    step.item,
                    step.card.name(),
                    describe(step.effect)
                );
                let line = if style(12.0, INK, layers::TEXT).width_of(&full) <= FEED_WIDTH {
                    full
                } else {
                    format!("#{} {}", step.item, describe(step.effect))
                };
                let color = if step.effect == Effect::Fizzled {
                    DIM
                } else {
                    INK
                };
                Some((line, color))
            }
            Event::TurnBegan { turn, active } => {
                Some((format!("-- turn {turn}: {} --", active.name()), DIM))
            }
            Event::Passed { .. } => None,
        })
        .take(FEED_ROWS)
        .collect()
}

/// The result banner's two lines for an outcome.
pub(crate) fn result_lines(duel: &Duel, outcome: Outcome) -> [String; 2] {
    let [you, npc] = duel.life();
    let title = match outcome {
        Outcome::Won(Side::You) => "YOU OUTPLAYED THE BRUTE".to_owned(),
        Outcome::Won(Side::Npc) => "THE BRUTE WINS".to_owned(),
        Outcome::Draw => "A DRAW".to_owned(),
    };
    [
        title,
        format!("WEAVER {you} - BRUTE {npc} after turn {}", duel.turn),
    ]
}

/// The line under the banner.
pub(crate) const AGAIN: &str = "R or tap here for another match";

/// Every row of text on screen, as data.
pub(crate) fn panel(duel: &Duel, ui: &Ui) -> Panel<Art> {
    let mut panel = Panel::default();
    let text = layers::TEXT;
    let [you, npc] = [duel.seat(Side::You), duel.seat(Side::Npc)];

    // The Brute, across the top.
    panel.text(TextRun::new(
        Vec2::new(16.0, 12.0),
        format!(
            "BRUTE  life {}  energy {}  hand {}  deck {}",
            npc.life,
            npc.energy,
            npc.hand.len(),
            npc.deck.len()
        ),
        style(16.0, NPC_TONE, text),
    ));
    let whose = if duel.active == Side::You {
        "your turn"
    } else {
        "BRUTE's turn"
    };
    let turn_line = format!("turn {}/{TURNS}  {whose}", duel.turn);
    panel.text(TextRun::new(
        Vec2::new(
            PRIORITY_BOX.max.x - style(16.0, INK, text).width_of(&turn_line),
            12.0,
        ),
        turn_line,
        style(16.0, INK, text),
    ));

    // The stack panel.
    panel.text(TextRun::new(
        Vec2::new(STACK_BOX.min.x + 8.0, STACK_BOX.min.y + 8.0),
        "THE STACK - top first, with the order it resolves in",
        style(14.0, GOLD, text),
    ));
    let lines = stack_lines(duel);
    let marks = marked(duel, ui);
    let aim = aimed(duel, ui);
    let row_cell = Cell::new(Vec2::new(18.0, 5.0), stack_row(0).size().x - 22.0);
    for (row, line) in lines.iter().take(STACK_ROWS).enumerate() {
        let at = stack_row(row).min;
        let item = duel.find_item(line.id);
        let color = if aim == Some(line.id) {
            GOLD
        } else {
            item.map_or(INK, |item| tone(item.caster))
        };
        if marks.contains(&line.id) {
            panel.text(TextRun::new(
                at + Vec2::new(4.0, 5.0),
                ">",
                style(12.0, GOLD, text),
            ));
        }
        panel.text(row_cell.at(at).run(&line.text, style(12.0, color, text)));
    }
    if lines.len() > STACK_ROWS {
        panel.text(TextRun::new(
            stack_row(STACK_ROWS).min + Vec2::new(18.0, 0.0),
            format!("+{} deeper", lines.len() - STACK_ROWS),
            style(12.0, DIM, text),
        ));
    }
    if lines.is_empty() {
        panel.text(TextRun::new(
            stack_row(0).min + Vec2::new(18.0, 5.0),
            "empty - the player with priority may play anything",
            style(12.0, DIM, text),
        ));
    }
    panel.text(TextRun::new(
        Vec2::new(STACK_BOX.min.x + 8.0, STACK_BOX.max.y - 20.0),
        outlook_line(duel),
        style(12.0, INK, text),
    ));

    // Priority.
    let holder = duel.priority;
    panel.text(TextRun::new(
        PRIORITY_BOX.min + Vec2::new(10.0, 10.0),
        format!("PRIORITY: {}", holder.name()),
        style(18.0, tone(holder), text),
    ));
    panel.text(TextRun::new(
        PRIORITY_BOX.min + Vec2::new(10.0, 50.0),
        clipped(&style(12.0, INK, text), &pass_line(duel), 228.0),
        style(12.0, INK, text),
    ));
    let pass_style = style(14.0, INK, text);
    panel.text(TextRun::new(
        centered(PASS_BUTTON, &pass_style, "PASS", PASS_BUTTON.min.y + 8.0),
        "PASS",
        pass_style,
    ));

    // The feed.
    let feed_cell = Cell::new(Vec2::new(8.0, 0.0), FEED_WIDTH);
    for (row, (line, color)) in feed_lines(duel).into_iter().enumerate() {
        let at = FEED_BOX.min + Vec2::new(0.0, 8.0 + row as f32 * 17.0);
        panel.text(feed_cell.at(at).run(&line, style(12.0, color, text)));
    }

    // You, above your hand.
    panel.text(TextRun::new(
        Vec2::new(16.0, 386.0),
        format!(
            "WEAVER  life {}  energy {}  deck {}",
            you.life,
            you.energy,
            you.deck.len()
        ),
        style(16.0, YOU_TONE, text),
    ));
    let hint = match (ui.notice.as_deref(), ui.pick) {
        (Some(notice), _) => notice.to_owned(),
        (None, Some(_)) if !marks.is_empty() => "up/down aim, enter plays, esc back".to_owned(),
        (None, Some(_)) => "enter plays it, esc puts it back".to_owned(),
        (None, None) => "1-6 pick a card, space passes".to_owned(),
    };
    let hint_style = style(12.0, if ui.notice.is_some() { GOLD } else { DIM }, text);
    panel.text(TextRun::new(
        Vec2::new(PRIORITY_BOX.min.x, 390.0),
        clipped(&hint_style, &hint, PRIORITY_BOX.size().x),
        hint_style,
    ));

    // The hand.
    for (slot, &card) in you.hand.iter().enumerate().take(HAND_SLOTS) {
        let at = card_box(slot).min;
        let able = duel.playable(Side::You, slot);
        let color = if able { INK } else { DIM };
        panel.text(TextRun::new(
            at + Vec2::new(8.0, 8.0),
            format!("{} {}", slot + 1, card.name()),
            style(16.0, color, text),
        ));
        let cost = format!("{}e", card.cost());
        let cost_style = style(14.0, if able { GOLD } else { DIM }, text);
        panel.text(TextRun::new(
            at + Vec2::new(CARD_SIZE.x - 8.0 - cost_style.width_of(&cost), 9.0),
            cost,
            cost_style,
        ));
        let rule_style = style(12.0, color, text);
        let rule = wrap(card.spec().rule, rule_style.columns_in(CARD_SIZE.x - 16.0));
        panel.block(at + Vec2::new(8.0, 36.0), &rule, rule_style, 4.0);
    }

    // The result, over everything.
    if let Some(outcome) = duel.over {
        let mut result = Panel::default();
        let [title, score] = result_lines(duel, outcome);
        let big = style(24.0, GOLD, layers::OVERLAY_TEXT);
        let mid = style(16.0, INK, layers::OVERLAY_TEXT);
        let small = style(12.0, DIM, layers::OVERLAY_TEXT);
        for (line, line_style, top) in [
            (title, big, RESULT_BOX.min.y + 40.0),
            (score, mid, RESULT_BOX.min.y + 100.0),
            (AGAIN.to_owned(), small, RESULT_BOX.min.y + 160.0),
        ] {
            result.text(TextRun::new(
                centered(RESULT_BOX, &line_style, &line, top),
                line,
                line_style,
            ));
        }
        panel.absorb(result);
    }
    panel
}

/// Every box on screen: where, what colour, which band.
pub(crate) fn boxes(duel: &Duel, ui: &Ui) -> Vec<(Rect, Color, i16)> {
    let mut out = vec![
        (STACK_BOX, BOX, layers::BOXES),
        (PRIORITY_BOX, BOX, layers::BOXES),
        (FEED_BOX, BOX, layers::BOXES),
        (
            PASS_BUTTON,
            if duel.priority == Side::You {
                BOX_LIT
            } else {
                BOX
            },
            layers::BOXES,
        ),
    ];
    let marks = marked(duel, ui);
    let aim = aimed(duel, ui);
    for (row, item) in duel.stack.iter().rev().take(STACK_ROWS).enumerate() {
        let color = if aim == Some(item.id) {
            AIMED
        } else if marks.contains(&item.id) {
            MARKED
        } else {
            continue;
        };
        out.push((stack_row(row), color, layers::BOXES));
    }
    for slot in 0..duel.seat(Side::You).hand.len().min(HAND_SLOTS) {
        let lit = ui.pick.is_some_and(|pick| pick.slot == slot);
        out.push((card_box(slot), if lit { AIMED } else { BOX }, layers::BOXES));
    }
    if duel.over.is_some() {
        out.push((RESULT_BOX, OVERLAY, layers::OVERLAY));
    }
    out
}

/// Something a tap can land on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Spot {
    Card(usize),
    StackRow(usize),
    Pass,
    Result,
}

/// Every tappable rectangle, with what it is — the hit-test and the floors'
/// control list both read this.
pub(crate) fn spots(duel: &Duel) -> Vec<(Spot, Rect)> {
    if duel.over.is_some() {
        return vec![(Spot::Result, RESULT_BOX)];
    }
    let mut out = vec![(Spot::Pass, PASS_BUTTON)];
    for slot in 0..duel.seat(Side::You).hand.len().min(HAND_SLOTS) {
        out.push((Spot::Card(slot), card_box(slot)));
    }
    for row in 0..duel.stack.len().min(STACK_ROWS) {
        out.push((Spot::StackRow(row), stack_row(row)));
    }
    out
}

/// What a tap at `at` (design units) lands on.
pub(crate) fn spot_at(duel: &Duel, at: Vec2) -> Option<Spot> {
    spots(duel)
        .into_iter()
        .find(|(_, rect)| rect.contains(at))
        .map(|(spot, _)| spot)
}

/// Draw the screen: the boxes, then the panel through the flat mapping.
pub(crate) fn draw(ctx: &mut DrawCtx, duel: &Duel, ui: &Ui) {
    for (rect, color, layer) in boxes(duel, ui) {
        ctx.rect(rect, color, Depth::layer(layer));
    }
    panel(duel, ui).draw(ctx, &Flat, |icon, _| match icon.art {});
}
