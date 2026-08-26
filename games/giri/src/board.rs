//! The quest board: the status bar, the quest row, the fixed info panel, the
//! beat's dilemma, and the log drawer (UI.md §3).
//!
//! The panel on the right is **fixed**, never a cursor-following bubble, and it
//! is **party-reactive**: every requirement, every share and every "can't join"
//! line is checked against the party the player has right now, so a quest is
//! always read against the people available for it.
//!
//! Hover peeks, click takes, and a taken quest locks the panel to itself with a
//! RELEASE control. `flow.rs` owns those transitions; this file draws what they
//! produce.

use jidousha::prelude::*;

use crate::beats::Dungeon;
use crate::flow::{Flow, Preview};
use crate::model::{Social, share_each};
use crate::sprites::Art;
use crate::ui::{IconRun, Panel, TextRun, columns, wrap};
use crate::{layout, theme};

/// The bar across the top: who we are, which beat, and what the player has.
pub fn status_bar(flow: &Flow) -> Panel {
    let mut panel = Panel::default();
    let bar = layout::topbar();
    let baseline = bar.min.y + 11.0;
    panel.text(TextRun::new(
        Vec2::new(16.0, baseline),
        "GIRI",
        theme::HEAD,
        theme::GOLD,
    ));
    panel.text(TextRun::new(
        Vec2::new(72.0, baseline + 1.0),
        "quest board",
        theme::SMALL,
        theme::DIM,
    ));
    panel.text(TextRun::new(
        Vec2::new(200.0, baseline + 1.0),
        beat_line(flow),
        theme::SMALL,
        theme::DIM,
    ));
    // Gold is the player's interest, so it carries the coin (UI.md §2: a stat
    // never appears as a bare number).
    panel.icon(IconRun::new(
        Vec2::new(876.0, baseline - 1.0),
        Art::Coin,
        2.0,
    ));
    panel.text(TextRun::new(
        Vec2::new(898.0, baseline + 1.0),
        format!("{}g", flow.gold),
        theme::SMALL,
        theme::GOLD,
    ));
    panel
}

/// Which beat this is, as the bar prints it.
pub fn beat_line(flow: &Flow) -> String {
    match flow.spec() {
        Some(beat) => format!(
            "beat {} of {} - {}",
            flow.beat + 1,
            crate::beats::CHAIN.len(),
            beat.title
        ),
        None => format!(
            "{} of {} beats done",
            crate::beats::CHAIN.len(),
            crate::beats::CHAIN.len()
        ),
    }
}

/// The quest row: one card per job the beat offers.
pub fn quest_row(flow: &Flow) -> Panel {
    let mut panel = Panel::default();
    let Some(beat) = flow.spec() else {
        return panel;
    };
    for (index, quest) in beat.dungeons.iter().enumerate().take(layout::QUEST_SLOTS) {
        let card = layout::quest_card(index);
        let lit = flow.taken.is_none() || flow.taken == Some(index);
        let (ink, dim) = if lit {
            (theme::INK, theme::DIM)
        } else {
            (theme::FAINT, theme::FAINT)
        };
        let icon = Art::for_quest(quest.icon);
        let scale = icon.scale_across(layout::quest_icon::CARD);
        let width = icon.size_at(scale).x;
        panel.icon(
            IconRun::new(
                Vec2::new(card.center().x - width * 0.5, card.min.y + 10.0),
                icon,
                scale,
            )
            .tinted(if lit {
                Color::WHITE
            } else {
                Color::rgb(0.45, 0.44, 0.52)
            }),
        );
        let name = wrap(quest.name, columns(card.size().x - 16.0, theme::SMALL));
        let mut y = card.min.y + 90.0;
        for line in name.lines() {
            let text = line.to_uppercase();
            panel.text(TextRun::new(
                crate::ui::centered(card, &text, theme::SMALL, y),
                text,
                theme::SMALL,
                ink,
            ));
            y += theme::SMALL + 2.0;
        }
        let takes = format!("takes {}", quest.headcount);
        panel.text(TextRun::new(
            crate::ui::centered(card, &takes, theme::SMALL, card.min.y + 126.0),
            takes,
            theme::SMALL,
            dim,
        ));
        // The pot, with the coin beside it — never a bare number (UI.md §2).
        let pot = format!("{}g", quest.pot);
        let group = 16.0 + 4.0 + theme::text(theme::SMALL, ink).width_of(&pot);
        let left = card.center().x - group * 0.5;
        panel.icon(IconRun::new(
            Vec2::new(left, card.min.y + 144.0),
            Art::Coin,
            2.0,
        ));
        panel.text(TextRun::new(
            Vec2::new(left + 20.0, card.min.y + 146.0),
            pot,
            theme::SMALL,
            if lit { theme::GOLD } else { theme::FAINT },
        ));
    }
    panel
}

/// The beat's dilemma, and the one concept it teaches, in the board's own
/// empty quarter.
///
/// `teaches` is authored on every beat and was drawn nowhere: a beat that
/// states its lesson in its own data and keeps it off the screen is a tutorial
/// that only the source code is doing.
pub fn dilemma(flow: &Flow) -> Panel {
    let mut panel = Panel::default();
    let box_ = layout::dilemma();
    let (sentence, teaches) = match flow.spec() {
        Some(beat) => (beat.dilemma.to_owned(), Some(beat.teaches)),
        None => (
            "The chain is finished. Every beat came out the way its numbers said it would."
                .to_owned(),
            None,
        ),
    };
    panel.block(
        box_.min,
        &wrap(&sentence, columns(box_.size().x, theme::SMALL)),
        theme::SMALL,
        theme::DIM,
    );
    // The lesson gives its slot up to a toast while one is showing: a bounced
    // click is the more urgent of the two lines and they say the same kind of
    // thing in the same place (`layout::beat_note`).
    if let (Some(teaches), None) = (teaches, &flow.toast) {
        panel.block(
            layout::beat_note(),
            &wrap(
                &format!("this beat teaches: {teaches}"),
                columns(layout::beat_note_width(), theme::SMALL),
            ),
            theme::SMALL,
            theme::FAINT,
        );
    }
    panel
}

/// The fixed info panel, checked live against the current party (UI.md §3).
pub fn info(flow: &Flow, social: &Social, preview: &Preview) -> Panel {
    let mut panel = Panel::default();
    let content = layout::info_content();
    let wide = content.size().x;
    let Some(quest) = flow.shown_quest() else {
        panel.text(TextRun::new(
            content.min,
            flow.spec().map_or("THE CHAIN IS DONE", |beat| beat.title),
            theme::HEAD,
            theme::GOLD,
        ));
        panel.block(
            content.min + Vec2::new(0.0, 26.0),
            &wrap(
                "Hover a quest to read it against the people you have. Click to take it on.",
                columns(wide, theme::SMALL),
            ),
            theme::SMALL,
            theme::DIM,
        );
        return panel;
    };

    let icon = Art::for_quest(quest.icon);
    panel.icon(IconRun::new(
        content.min,
        icon,
        icon.scale_across(layout::quest_icon::DETAIL),
    ));
    let title = quest.name.to_uppercase();
    panel.block(
        content.min + Vec2::new(56.0, 8.0),
        &wrap(&title, columns(wide - 56.0, theme::HEAD)),
        theme::HEAD,
        theme::GOLD,
    );
    panel.block(
        content.min + Vec2::new(0.0, 54.0),
        &wrap(quest.blurb, columns(wide, theme::SMALL)),
        theme::SMALL,
        theme::DIM,
    );

    // --- requirements, against the party the player has right now ---------
    let party = flow.party.len();
    let head_ok = party == quest.headcount;
    panel.text(TextRun::new(
        content.min + Vec2::new(0.0, 96.0),
        format!("{} party {}/{}", mark(head_ok), party, quest.headcount),
        theme::BODY,
        tone(head_ok),
    ));
    let mut y = 114.0;
    if quest.requires != crate::beats::Requirement::AnyParty {
        let met = preview.requirement_ok;
        for (index, line) in wrap(
            &format!("{} {}", mark(met), quest.requires.describe()),
            columns(wide, theme::SMALL),
        )
        .lines()
        .enumerate()
        {
            panel.text(TextRun::new(
                content.min + Vec2::new(0.0, y + index as f32 * (theme::SMALL + 2.0)),
                line,
                theme::SMALL,
                tone(met),
            ));
        }
        y += 30.0;
    }

    // --- the arithmetic of the split, for this party size -----------------
    panel.block(
        content.min + Vec2::new(0.0, y + 8.0),
        &wrap(&pay_line(quest, party), columns(wide, theme::SMALL)),
        theme::SMALL,
        theme::INK,
    );
    y += 8.0 + 2.0 * (theme::SMALL + 2.0);

    // --- who cannot join this party, and why ------------------------------
    let refusals = cannot_join(social, preview);
    if !refusals.is_empty() {
        // The heading in ember, the arithmetic in dim: the fact that somebody
        // cannot come is the alarming half, and the sums under it are what the
        // player does something about (UI.md §2 - refusal states in ember).
        let mut row = panel.block(
            content.min + Vec2::new(0.0, y + 10.0),
            "can't join this party:",
            theme::SMALL,
            theme::EMBER,
        );
        for refusal in &refusals {
            row = panel.block(
                Vec2::new(content.min.x, row),
                &wrap(refusal, columns(wide, theme::SMALL)),
                theme::SMALL,
                theme::DIM,
            );
        }
    }

    // --- what the panel is doing: locked, or peeking ----------------------
    let release = layout::release_button();
    let taken_here = flow.taken == flow.shown();
    if taken_here {
        panel.text(TextRun::new(
            crate::ui::centered(release, "RELEASE QUEST", theme::SMALL, release.min.y + 11.0),
            "RELEASE QUEST",
            theme::SMALL,
            theme::DIM,
        ));
    } else {
        panel.block(
            Vec2::new(content.min.x, release.min.y + 4.0),
            &wrap(
                if flow.taken.is_some() {
                    "peeking - your taken quest stays locked"
                } else {
                    "click the card to take this quest on"
                },
                columns(wide, theme::SMALL),
            ),
            theme::SMALL,
            theme::FAINT,
        );
    }
    panel
}

/// The pot, the cut, and what one body gets — for the party the player has.
pub fn pay_line(quest: &Dungeon, party: usize) -> String {
    let split = i32::try_from(party).unwrap_or(i32::MAX);
    let each = share_each(quest.pot, quest.cut, split);
    format!(
        "pot {}g - your cut {}g -> {}g split {} way{} = {}",
        quest.pot,
        quest.cut,
        (quest.pot - quest.cut).max(0),
        party,
        if party == 1 { "" } else { "s" },
        if party == 0 {
            "nothing yet".to_owned()
        } else {
            format!("{each}g each")
        },
    )
}

/// Everybody on the roster who could not be added right now, and why.
///
/// The door's two failures read differently and are named differently: a
/// refusal is the character's own, a block is somebody else's on their behalf
/// (DESIGN §3.2's door rule).
pub fn cannot_join(social: &Social, preview: &Preview) -> Vec<String> {
    preview
        .doors
        .iter()
        .filter(|(_, admission)| !admission.admitted())
        .map(|(who, admission)| format!("{} {}", social.name(*who), admission.status_line()))
        .collect()
}

/// ASCII for a requirement that passes or fails.
///
/// The mockup's tick and cross are two characters the engine's font draws as
/// boxes (DESIGN §7: the atlas covers space through `~`), and no assertion over
/// drawn quads could see the difference — so they are spelled in the alphabet
/// the font has, and the colour carries the same fact a second time.
fn mark(ok: bool) -> &'static str {
    if ok { "[+]" } else { "[x]" }
}

fn tone(ok: bool) -> Color {
    if ok { theme::GOOD } else { theme::EMBER }
}

/// The log drawer: reverse-chronological, one line each (UI.md §3).
pub fn log(flow: &Flow) -> Panel {
    let mut panel = Panel::default();
    panel.text(TextRun::over(
        Vec2::new(28.0, 52.0),
        "EXPEDITION LOG - click anywhere to close",
        theme::SMALL,
        theme::DIM,
    ));
    if flow.log.is_empty() {
        panel.text(TextRun::over(
            layout::log_row(0),
            "nothing has happened yet",
            theme::SMALL,
            theme::FAINT,
        ));
    }
    // **Wrapped, and the drawer's budget is rows rather than entries.** A log
    // entry used to be assumed to fit one row, which held while every entry was
    // a sentence about one click; the line an APPLY writes carries the whole
    // constants stamp and is three times the drawer's width. An entry that ran
    // off the side would be drawn outside the design rect, which is a failure
    // the bounds check only sees on a frame that has the drawer open.
    let mut row = 0;
    for line in &flow.log {
        for piece in wrap(
            line,
            columns(layout::log_panel().size().x - 56.0, theme::SMALL),
        )
        .lines()
        {
            if row >= layout::LOG_ROWS {
                return panel;
            }
            panel.text(TextRun::over(
                layout::log_row(row),
                piece,
                theme::SMALL,
                theme::DIM,
            ));
            row += 1;
        }
    }
    panel
}
