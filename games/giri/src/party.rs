//! The party strip: always present, one card per roster character, with the
//! status-line grammar UI.md §4 states (UI.md §4).
//!
//! **Dead characters stay on the roster** — grayed, skull-marked, unclickable.
//! Memory is a signifier, and the one state a game would otherwise quietly drop
//! is the one invariant 2 most wants kept (UI.md §2).
//!
//! **The status line says exactly one of four things**, and the door decides
//! which: `in - <sum>` for a member, and for a non-member `would join - <sum>`,
//! `refuses - <sum>`, or `<NAME> blocks - <blocker's sum>`. All four come out
//! of `model::Admission` and `model::Willingness`, so the strip, the info
//! panel's can't-join list, the bounce toast and the log are one sentence
//! written once.

use jidousha::prelude::*;

use crate::flow::{Flow, Preview};
use crate::layout::pcard;
use crate::model::{Member, Social};
use crate::sprites::Art;
use crate::ui::{IconRun, Panel, TextRun, columns, wrap};
use crate::{layout, theme};

/// How far a status line may run before it wraps.
fn status_columns() -> usize {
    columns(layout::party_card(0).size().x - 12.0, theme::SMALL)
}

/// One stat, and the icon that has to be beside it (UI.md §7's redundancy
/// floor).
///
/// Handed back as a pair so `floors.rs` can assert the adjacency against the
/// same values the frame was built from, rather than against a comment.
#[derive(Clone, Debug)]
pub struct Stat {
    /// The icon.
    pub icon: IconRun,
    /// The number beside it.
    pub value: TextRun,
}

/// The three stats on one card: desperation, infamy, wealth.
///
/// Centred as a group, so they read as one row rather than as three things
/// that happen to be near each other. **Wealth is here because DESIGN §12 puts
/// it there** — it accumulates shares and it decides nothing a player can see
/// unless it is on screen, and invariant 2 says a number that decides an
/// outcome is a number on screen. UI.md §4 names two stats; where the two
/// documents meet, DESIGN wins.
pub fn stats_of(card: Rect, member: &Member, lit: bool) -> Vec<Stat> {
    let style = theme::text(theme::SMALL, theme::INK);
    let icon = 16.0;
    let gap = 3.0;
    let between = 10.0;
    let shown = [
        (Art::Flame, format!("{}", member.desperation), theme::EMBER),
        (Art::Eye, format!("{}", member.infamy), theme::INFAMY),
        (Art::Coin, format!("{}", member.wealth), theme::GOLD),
    ];
    let width: f32 = shown
        .iter()
        .map(|(_, value, _)| icon + gap + style.width_of(value) + between)
        .sum::<f32>()
        - between;
    let mut x = card.center().x - width * 0.5;
    let top = card.min.y + pcard::STATS_TOP;
    let mut out = Vec::new();
    for (art, value, color) in shown {
        let color = if lit { color } else { theme::FAINT };
        out.push(Stat {
            icon: IconRun::new(Vec2::new(x, top), art, 2.0).tinted(if lit {
                Color::WHITE
            } else {
                Color::rgb(0.42, 0.41, 0.48)
            }),
            value: TextRun::new(
                Vec2::new(x + icon + gap, top + 2.0),
                value.clone(),
                theme::SMALL,
                color,
            ),
        });
        x += icon + gap + theme::text(theme::SMALL, color).width_of(&value) + between;
    }
    out
}

/// A character's outgoing regard edges, as one line.
///
/// Sparse: an absent edge is zero and is not drawn, which is what "sparse"
/// means on a sheet. Carries the heart, because a bond is a signifier with a
/// colour and an icon and not a number in a list (UI.md §2).
pub fn regard_line(social: &Social, who: Entity) -> String {
    let mut parts: Vec<String> = Vec::new();
    for member in &social.members {
        let value = social.regard(who, member.entity);
        if value != 0 {
            parts.push(format!("{} {value:+}", member.name));
        }
    }
    if parts.is_empty() {
        "no bonds yet".to_owned()
    } else {
        parts.join("  ")
    }
}

/// The status line for one character (UI.md §4's grammar, exactly).
pub fn status_line(member: &Member, preview: &Preview, in_party: bool) -> String {
    if !member.alive {
        return "gone".to_owned();
    }
    if in_party {
        return preview
            .entries
            .iter()
            .find(|entry| entry.who == member.entity)
            .map_or_else(
                || "in".to_owned(),
                |entry| format!("in - {}", entry.arithmetic()),
            );
    }
    preview
        .door(member.entity)
        .map_or_else(|| "waiting".to_owned(), |door| door.status_line())
}

/// The whole strip.
pub fn strip(flow: &Flow, social: &Social, preview: &Preview) -> Panel {
    let mut panel = Panel::default();
    panel.text(TextRun::new(
        layout::party_label(),
        "YOUR PEOPLE - click to add or remove",
        theme::SMALL,
        theme::DIM,
    ));
    for (index, member) in social.members.iter().enumerate() {
        let card = layout::party_card(index);
        let in_party = flow.party.contains(&member.entity);
        let lit = member.alive;

        let face = Art::portrait_for(member.name, member.roster_index);
        let width = face.size_at(pcard::PORTRAIT_SCALE).x;
        panel.icon(
            IconRun::new(
                Vec2::new(
                    card.center().x - width * 0.5,
                    card.min.y + pcard::PORTRAIT_TOP,
                ),
                face,
                pcard::PORTRAIT_SCALE,
            )
            .tinted(if lit {
                Color::WHITE
            } else {
                Color::rgb(0.32, 0.32, 0.36)
            }),
        );
        if !lit {
            // The skull sits over the portrait's shoulder, where it cannot be
            // read as part of the face.
            panel.icon(IconRun::new(
                Vec2::new(card.max.x - 26.0, card.min.y + pcard::PORTRAIT_TOP),
                Art::Skull,
                2.0,
            ));
        }
        panel.text(TextRun::new(
            crate::ui::centered(card, member.name, theme::BODY, card.min.y + pcard::NAME_TOP),
            member.name,
            theme::BODY,
            if lit { theme::INK } else { theme::FAINT },
        ));
        for stat in stats_of(card, member, lit) {
            panel.icon(stat.icon);
            panel.text(stat.value);
        }
        // The edges, with the heart beside them.
        let edges = regard_line(social, member.entity);
        panel.icon(
            IconRun::new(
                Vec2::new(card.min.x + 6.0, card.min.y + pcard::REGARD_TOP),
                Art::Heart,
                2.0,
            )
            .tinted(if lit {
                Color::WHITE
            } else {
                Color::rgb(0.42, 0.41, 0.48)
            }),
        );
        let mut y = card.min.y + pcard::REGARD_TOP + 1.0;
        for row in wrap(&edges, columns(card.size().x - 32.0, theme::SMALL)).lines() {
            panel.text(TextRun::new(
                Vec2::new(card.min.x + 26.0, y),
                row,
                theme::SMALL,
                if lit { theme::REGARD } else { theme::FAINT },
            ));
            y += theme::SMALL + 2.0;
        }

        let line = status_line(member, preview, in_party);
        let color = status_color(member, preview, in_party);
        let mut y = card.min.y + pcard::STATUS_TOP;
        for row in wrap(&line, status_columns()).lines() {
            panel.text(TextRun::new(
                crate::ui::centered(card, row, theme::SMALL, y),
                row,
                theme::SMALL,
                color,
            ));
            y += theme::SMALL + 2.0;
        }
    }
    panel
}

/// What colour a status line reads in: refusal and blocking in ember, joined in
/// teal, available in dim (UI.md §2).
pub fn status_color(member: &Member, preview: &Preview, in_party: bool) -> Color {
    if !member.alive {
        return theme::FAINT;
    }
    if in_party {
        // A member who has gone negative since they joined - a bonded partner
        // was removed - reads in ember and stays in the party. The door does
        // not ask twice (DESIGN §3.2), and the number saying so is on the card.
        return match preview
            .entries
            .iter()
            .find(|entry| entry.who == member.entity)
        {
            Some(entry) if !entry.joins() => theme::EMBER,
            _ => theme::REGARD,
        };
    }
    match preview.door(member.entity) {
        Some(door) if door.admitted() => theme::DIM,
        Some(_) => theme::EMBER,
        None => theme::DIM,
    }
}

/// The send verb and the reason it is not available.
///
/// **The button exists only while a quest is taken** (UI.md §3), which is a
/// different thing from being disabled: with nothing taken there is no verb to
/// offer, and a permanently greyed button teaches a player it is broken.
pub fn send(flow: &Flow, preview: &Preview) -> Panel {
    let mut panel = Panel::default();
    if flow.taken.is_none() {
        let text = "take a quest to send them out";
        let width = theme::text(theme::SMALL, theme::DIM).width_of(text);
        panel.text(TextRun::new(
            Vec2::new(
                layout::send_reason_right().x - width,
                layout::send_button().min.y + 12.0,
            ),
            text,
            theme::SMALL,
            theme::DIM,
        ));
        return panel;
    }
    let button = layout::send_button();
    panel.text(TextRun::new(
        crate::ui::centered(button, "SEND PARTY", theme::BODY, button.min.y + 13.0),
        "SEND PARTY",
        theme::BODY,
        if preview.can_send {
            theme::GROUND
        } else {
            theme::DIM
        },
    ));
    if !preview.blocked.is_empty() {
        let width = theme::text(theme::SMALL, theme::EMBER).width_of(&preview.blocked);
        panel.text(TextRun::new(
            Vec2::new(
                layout::send_reason_right().x - width,
                layout::send_reason_right().y,
            ),
            preview.blocked.clone(),
            theme::SMALL,
            theme::EMBER,
        ));
    }
    panel
}

/// The transient message a bounced click raises.
pub fn toast(flow: &Flow) -> Panel {
    let mut panel = Panel::default();
    if let Some(toast) = &flow.toast {
        panel.block(
            layout::beat_note(),
            &wrap(
                &toast.text,
                columns(layout::beat_note_width(), theme::SMALL),
            ),
            theme::SMALL,
            theme::EMBER,
        );
    }
    panel
}
