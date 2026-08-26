//! The party strip: always present, one card per roster character — the sheet
//! view, at v2's interim presentation (UI.md §4, §13).
//!
//! **The v2 card is the v2 sheet**: portrait, name, desperation with its
//! source, wealth, trait chips (icon + name), reputation marks as lines,
//! regard edges, and the **verdict-and-reasons status line** — the one piece
//! of presentation DESIGN §12 says ships with the mechanics, because it is
//! how v2 is playtestable at all. The numeric sums moved behind inspection
//! (the toast and the info panel carry the margin); the card carries the
//! judgment and its leading cause as words.
//!
//! **Dead characters stay on the roster** — grayed, skull-marked, unclickable.
//! Memory is a signifier (UI.md §2).
//!
//! **The status line says exactly one of these**, and the door decides which:
//! `in - <reason>` for a member, and for a non-member `would join - <reason>`,
//! `reluctant - <reason>`, `refuses - <reason>`, or `<NAME> blocks -
//! <blocker's reason>`. All of them come out of `willing::Admission` and
//! `willing::Willingness`, so the strip, the info panel's can't-join list, the
//! bounce toast and the log are one sentence written once.

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

/// How far a mark or regard row may run beside its icon.
fn line_columns() -> usize {
    columns(layout::party_card(0).size().x - 38.0, theme::SMALL)
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

/// The two stats on one card: desperation and wealth.
///
/// **The eye is gone from this row** — the v1 public scalar is retired (DESIGN
/// §5) and what everyone knows is the mark lines below. Wealth stays because
/// invariant 2 says a number that decides an outcome is a number on screen.
pub fn stats_of(card: Rect, member: &Member, lit: bool) -> Vec<Stat> {
    let icon = 16.0;
    let gap = 3.0;
    let between = 12.0;
    let shown = [
        (Art::Flame, format!("{}", member.desperation), theme::EMBER),
        (Art::Coin, format!("{}", member.wealth), theme::GOLD),
    ];
    let mut x = card.min.x + pcard::RIGHT_COL;
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

/// A character's marks, as one line — what everyone knows (DESIGN §5).
pub fn marks_line(member: &Member) -> String {
    if member.marks.is_empty() {
        "no marks".to_owned()
    } else {
        member
            .marks
            .iter()
            .map(|mark| mark.name())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The status line UI.md §4 states the grammar of: the verdict, and the
/// leading reason as words (DESIGN §6, §12 — the line that ships with the
/// mechanics).
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
                |entry| format!("in - {}", entry.top_reason()),
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
        let dim_icon = Color::rgb(0.42, 0.41, 0.48);

        // --- the header block: portrait left, name/stats/source beside it ---
        let face = Art::portrait_for(member.name, member.roster_index);
        panel.icon(
            IconRun::new(
                Vec2::new(
                    card.min.x + pcard::PORTRAIT_LEFT,
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
        let col = card.min.x + pcard::RIGHT_COL;
        panel.text(TextRun::new(
            Vec2::new(col, card.min.y + pcard::NAME_TOP),
            member.name,
            theme::BODY,
            if lit { theme::INK } else { theme::FAINT },
        ));
        for stat in stats_of(card, member, lit) {
            panel.icon(stat.icon);
            panel.text(stat.value);
        }
        // The source: why the flame burns (DESIGN §3). One row, authored to
        // fit the header column.
        panel.text(TextRun::new(
            Vec2::new(col, card.min.y + pcard::SOURCE_TOP),
            member.source,
            theme::SMALL,
            if lit { theme::DIM } else { theme::FAINT },
        ));

        // --- trait chips: icon + name, one row per trait (UI.md §13) --------
        let mut y = card.min.y + pcard::TRAITS_TOP;
        for trait_id in &member.traits {
            let def = trait_id.def();
            panel.icon(
                IconRun::new(Vec2::new(card.min.x + 6.0, y), def.icon, 2.0).tinted(if lit {
                    Color::WHITE
                } else {
                    dim_icon
                }),
            );
            panel.text(TextRun::new(
                Vec2::new(card.min.x + 26.0, y + 2.0),
                def.name,
                theme::SMALL,
                if lit { theme::INK } else { theme::FAINT },
            ));
            y += pcard::TRAIT_PITCH;
        }
        y += 4.0;

        // --- marks: the eye and what everyone knows -------------------------
        panel.icon(
            IconRun::new(Vec2::new(card.min.x + 6.0, y), Art::Eye, 2.0).tinted(if lit {
                Color::WHITE
            } else {
                dim_icon
            }),
        );
        let marked = !member.marks.is_empty();
        for row in wrap(&marks_line(member), line_columns()).lines() {
            panel.text(TextRun::new(
                Vec2::new(card.min.x + 26.0, y + 2.0),
                row,
                theme::SMALL,
                if !lit {
                    theme::FAINT
                } else if marked {
                    theme::MARK
                } else {
                    theme::FAINT
                },
            ));
            y += theme::SMALL + 2.0;
        }
        y += 4.0;

        // --- the edges, with the heart beside them ---------------------------
        panel.icon(
            IconRun::new(Vec2::new(card.min.x + 6.0, y), Art::Heart, 2.0).tinted(if lit {
                Color::WHITE
            } else {
                dim_icon
            }),
        );
        for row in wrap(&regard_line(social, member.entity), line_columns()).lines() {
            panel.text(TextRun::new(
                Vec2::new(card.min.x + 26.0, y + 2.0),
                row,
                theme::SMALL,
                if lit { theme::REGARD } else { theme::FAINT },
            ));
            y += theme::SMALL + 2.0;
        }

        // --- the verdict and its reason, pinned at the card's foot -----------
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
        // not ask twice (DESIGN §6), and the line saying so is on the card.
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
