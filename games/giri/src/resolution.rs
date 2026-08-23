//! The resolution takeover: a full-screen replacement for the board, with the
//! event cards and the drift ledger (UI.md §3).
//!
//! **The story surface** (DESIGN §7). Every consequence is narrated
//! mechanically and names the rule inputs: a betrayal is a skull-marked ember
//! card with `desperation 8 >= 6 - regard 0 < 2 - share 2g -> 4g` in small text
//! under it, the payout is a coin card, and the drift ledger carries the
//! desperation arrows, the regard moves, the infamy moves and the hungry-wait
//! line for everybody who sat the round out.
//!
//! **If no blood was spilled, it says so**: absence of an event is also
//! information, and a screen that simply had no kill card would read as a
//! screen that forgot to draw one.
//!
//! Every string here comes out of `model::Resolution`, which `resolve.rs`
//! builds in one pass beside the ASCII narration the log keeps. The takeover
//! and the log cannot describe different runs because neither computes.

use jidousha::prelude::*;

use crate::flow::Flow;
use crate::resolve::{DriftTone, EventKind};
use crate::sprites::Art;
use crate::ui::{IconRun, Panel, TextRun, columns, wrap};
use crate::{layout, theme};

/// How wide an event card's text column is: the card, less the icon gutter on
/// the left and the same padding again on the right.
///
/// The right-hand padding is the half that is easy to leave out, and leaving it
/// out puts the last character of a full line exactly on the card's border.
fn text_width() -> f32 {
    layout::takeover_column().size().x - 60.0 - 14.0
}

/// How tall the card for one event is, given whether it carries small text.
pub fn card_height(sub_lines: usize, text_lines: usize) -> f32 {
    16.0 + text_lines as f32 * (theme::BODY + 2.0) + sub_lines as f32 * (theme::SMALL + 2.0) + 12.0
}

/// The heights of every event card, in order — what `layout::event_card` lays
/// the column out from.
pub fn card_heights(flow: &Flow) -> Vec<f32> {
    let column = text_width();
    flow.events
        .iter()
        .map(|event| {
            let text = wrap(&event.text, columns(column, theme::BODY));
            let sub = event
                .sub
                .as_ref()
                .map(|sub| wrap(sub, columns(column, theme::SMALL)));
            card_height(
                sub.map_or(0, |sub| sub.lines().count()),
                text.lines().count().max(1),
            ) + 10.0
        })
        .collect()
}

/// Everything the takeover draws.
pub fn takeover(flow: &Flow) -> Panel {
    let mut panel = Panel::default();
    let quest = flow.resolved.and_then(|index| flow.quest(index));

    // --- the job it was about ---------------------------------------------
    let head = layout::takeover_head();
    if let Some(quest) = quest {
        panel.icon(IconRun::over(head, Art::for_quest(quest.icon), 5.0));
        panel.text(TextRun::over(
            head + Vec2::new(76.0, 20.0),
            quest.name.to_uppercase(),
            theme::TITLE,
            theme::GOLD,
        ));
    }
    let banner = "- CLEARED -";
    panel.text(TextRun::over(
        crate::ui::centered(layout::takeover(), banner, theme::HEAD, head.y + 74.0),
        banner,
        theme::HEAD,
        theme::GOOD,
    ));

    // --- the event cards ---------------------------------------------------
    let heights = card_heights(flow);
    let column = layout::takeover_column();
    let text_width = text_width();
    let mut bottom = column.min.y;
    for (index, event) in flow.events.iter().enumerate() {
        let card = layout::event_card(index, &heights);
        bottom = card.max.y;
        let (art, scale, tone) = match event.kind {
            EventKind::Kill => (Art::Skull, 3.0, theme::EMBER),
            EventKind::Coin => (Art::Coin, 3.0, theme::GOLD),
            EventKind::Word => (Art::Eye, 3.0, theme::INK),
        };
        panel.icon(IconRun::over(
            Vec2::new(card.min.x + 14.0, card.min.y + 14.0),
            art,
            scale,
        ));
        let mut y = card.min.y + 14.0;
        for line in wrap(&event.text, columns(text_width, theme::BODY)).lines() {
            panel.text(TextRun::over(
                Vec2::new(card.min.x + 60.0, y),
                line,
                theme::BODY,
                tone,
            ));
            y += theme::BODY + 2.0;
        }
        if let Some(sub) = &event.sub {
            for line in wrap(sub, columns(text_width, theme::SMALL)).lines() {
                panel.text(TextRun::over(
                    Vec2::new(card.min.x + 60.0, y),
                    line,
                    theme::SMALL,
                    theme::DIM,
                ));
                y += theme::SMALL + 2.0;
            }
        }
    }

    // --- the drift ledger --------------------------------------------------
    let mut y = bottom + 16.0;
    panel.text(TextRun::over(
        Vec2::new(column.min.x, y),
        "WHAT IT LEFT BEHIND",
        theme::SMALL,
        theme::FAINT,
    ));
    y += theme::SMALL + 8.0;
    for line in &flow.drift {
        let color = match line.tone {
            DriftTone::Cost => theme::EMBER,
            DriftTone::Relief => theme::GOOD,
            DriftTone::Regard => theme::REGARD,
            DriftTone::Infamy => theme::INFAMY,
        };
        // Regard and infamy carry their signifiers; desperation lines carry the
        // flame, so no ledger row is text alone (UI.md §1).
        let art = match line.tone {
            DriftTone::Cost | DriftTone::Relief => Art::Flame,
            DriftTone::Regard => Art::Heart,
            DriftTone::Infamy => Art::Eye,
        };
        panel.icon(IconRun::over(Vec2::new(column.min.x, y - 1.0), art, 2.0));
        panel.text(TextRun::over(
            Vec2::new(column.min.x + 22.0, y),
            line.text.clone(),
            theme::SMALL,
            color,
        ));
        y += theme::SMALL + 4.0;
    }

    let hint = "click anywhere to return to the board";
    panel.text(TextRun::over(
        crate::ui::centered(
            layout::takeover(),
            hint,
            theme::SMALL,
            layout::takeover_hint().y,
        ),
        hint,
        theme::SMALL,
        theme::FAINT,
    ));
    panel
}

/// The end of the chain — the fourth screen, reached exactly once per loop.
pub fn complete() -> Panel {
    let mut panel = Panel::default();
    let screen = layout::takeover();
    let title = "THE CHAIN IS COMPLETE";
    panel.text(TextRun::over(
        crate::ui::centered(screen, title, theme::TITLE, 160.0),
        title,
        theme::TITLE,
        theme::GOOD,
    ));
    let body = wrap(
        "Four beats, no dice, and nothing hidden. Everything that happened was on the \
         sheets before you pressed send.",
        columns(560.0, theme::BODY),
    );
    let mut y = 210.0;
    for line in body.lines() {
        panel.text(TextRun::over(
            crate::ui::centered(screen, line, theme::BODY, y),
            line,
            theme::BODY,
            theme::DIM,
        ));
        y += theme::BODY + 4.0;
    }
    let hint = "click anywhere to play it again";
    panel.text(TextRun::over(
        crate::ui::centered(screen, hint, theme::SMALL, layout::takeover_hint().y),
        hint,
        theme::SMALL,
        theme::FAINT,
    ));
    panel
}
