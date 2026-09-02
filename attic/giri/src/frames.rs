//! Judging what a beat's screens drew: every row and icon of a screen's
//! content, found on the recorded frame (the transcript half of the beat
//! judgement; the world half is `judge.rs`).
//!
//! Every screen is asked the same question — is every row of what this screen
//! *says* on the frame, all of it — against `screens::content`, which is the
//! same function the draw system renders. One layout, two readers.

use jidousha::testing::{BackendTextureId, FrameRecord};

use crate::checks::{Checks, near};
use crate::verify::BeatRun;
use crate::{layout, screens, theme, ui};

/// How many of a row's glyphs were drawn, counted inside the row's own box.
///
/// `ctx.text` puts the top-left of the first character's *cell* at `at` and
/// advances along the row, so a run's glyphs share `bounds().min.y` and span
/// exactly `width_of(text)`. Counting *inside that span* rather than "everything
/// to the right of it" is what lets two runs share a row - the status bar puts
/// four on one - without either counting the other's characters.
pub fn glyph_run(frame: &FrameRecord, font: BackendTextureId, run: &ui::TextRun) -> usize {
    let box_ = run.bounds();
    frame
        .quads()
        .iter()
        .filter(|quad| {
            quad.texture == font
                && near(quad.bounds().min.y, box_.min.y)
                // Half a world unit of slack on each side: a row's own glyphs
                // span exactly `width_of(text)` in exact arithmetic, and
                // whether the last one lands a hair over is a rounding question
                // no assertion should turn on. One glyph is nine units wide, so
                // this cannot swallow a neighbouring row's first character.
                && quad.bounds().min.x >= box_.min.x - 0.5
                && quad.bounds().max.x <= box_.max.x + 0.5
        })
        .count()
}

/// Judge what a beat's screens drew.
///
/// Every screen is asked the same question — is every row of what this screen
/// *says* on the frame, all of it — against `screens::content`, which is the
/// same function the draw system renders. One layout, two readers.
pub fn judge_frames(checks: &mut Checks, run: &BeatRun) {
    let beat = run.index + 1;
    for (mode, frame, flow, social, preview) in [
        (
            "the board with the quest taken",
            &run.board_frame,
            &run.board_flow,
            &run.at_assembly,
            &run.board_preview,
        ),
        (
            "the board with the party staged",
            &run.ready_frame,
            &run.ready_flow,
            &run.at_assembly,
            &run.ready,
        ),
        (
            "the resolution takeover",
            &run.report_frame,
            &run.report_flow,
            &run.after,
            &run.report_preview,
        ),
    ] {
        let Some(frame) = frame else { continue };
        let panel = screens::content(flow, social, preview, &run.tuning, run.variant);
        for text_run in &panel.runs {
            let drawn = glyph_run(frame, run.font, text_run);
            checks.require(
                drawn == text_run.text.chars().count(),
                "a row of a screen is not drawn as the string it is",
                format!(
                    "beat {beat}, {mode}: {:?} at ({:.1}, {:.1}) is {} characters and {drawn} \
                     glyphs landed in its box",
                    text_run.text,
                    text_run.at.x,
                    text_run.at.y,
                    text_run.text.chars().count(),
                ),
            );
        }
        for icon in &panel.icons {
            let covered = frame.quads().iter().any(|quad| {
                quad.texture != run.font
                    && near(quad.bounds().min.x, icon.at.x)
                    && near(quad.bounds().min.y, icon.at.y)
            });
            checks.require(
                covered,
                "an icon a screen says it draws is not on the frame",
                format!(
                    "beat {beat}, {mode}: {:?} at ({:.1}, {:.1}) has no quad",
                    icon.art, icon.at.x, icon.at.y
                ),
            );
        }
    }

    // --- a card's edge says whether its character is in (UI.md §2) --------
    //
    // Two channels for one fact: the status line says "in" and the border says
    // it again in teal. Asserted because a border is the one signifier no
    // string check can see, and because the colour is what a player reads at a
    // glance while the arithmetic is what they read on purpose.
    if let Some(frame) = &run.ready_frame {
        let quads = frame.quads();
        for member in &run.at_assembly.members {
            let card = layout::party_card(member.roster_index);
            let edge = quads
                .iter()
                .find(|quad| {
                    quad.texture != run.font
                        && near(quad.bounds().min.x, card.min.x)
                        && near(quad.bounds().min.y, card.min.y)
                        && quad.bounds().size().y < 4.0
                })
                .map(|quad| quad.tint);
            let inside = run.ready_flow.party.contains(&member.entity);
            let wanted = if inside {
                theme::REGARD
            } else if member.alive {
                theme::BORDER
            } else {
                theme::RULE
            };
            checks.require(
                edge == Some(wanted),
                "a party card's edge does not say whether its character is in",
                format!(
                    "beat {beat}: {} is {}in the party and the card's top edge is {edge:?}, \
                     wanted {wanted:?}",
                    member.name,
                    if inside { "" } else { "not " }
                ),
            );
        }
    }

    // --- the send verb exists only while a quest is taken (UI.md §3) -------
    if let Some(frame) = &run.board_frame {
        let button = layout::send_button();
        // *Either* face colour: a button that exists and is disabled is the
        // state UI.md §3 asks for once a quest is taken and the party is still
        // short, and looking only for the live gold would call that absent.
        let drawn = frame.quads().iter().any(|quad| {
            quad.texture != run.font
                && (quad.tint == theme::GOLD || quad.tint == theme::BUTTON_DEAD)
                && near(quad.bounds().min.x, button.min.x)
                && near(quad.bounds().min.y, button.min.y)
        });
        checks.require(
            drawn == run.board_flow.taken.is_some(),
            "the send verb's presence does not follow whether a quest is taken",
            format!(
                "beat {beat}: a quest is {}taken and the button face is {}drawn; UI.md §3 says \
                 it exists only while one is",
                if run.board_flow.taken.is_some() {
                    ""
                } else {
                    "not "
                },
                if drawn { "" } else { "not " },
            ),
        );
        // And it is disabled with a stated reason rather than silently dead.
        checks.require(
            run.board_preview.can_send || !run.board_preview.blocked.is_empty(),
            "the send verb is disabled without saying why",
            format!(
                "beat {beat}: the gate cannot send and its stated reason is {:?}",
                run.board_preview.blocked
            ),
        );
    }

    // --- the takeover replaces the board entirely (UI.md §3) ---------------
    if let Some(frame) = &run.report_frame {
        let middle = layout::design().center();
        let front = frame.covering(middle).into_iter().next();
        checks.require(
            front.is_some_and(|quad| {
                quad.tint == theme::SCRIM || quad.texture == run.font || quad.tint == theme::BAR
            }),
            "the resolution screen does not replace the board it took over",
            format!(
                "beat {beat}: the front-most quad at the middle of the screen is {:?}; the \
                 takeover is a full-screen replacement, not a panel over a board",
                front.map(|quad| quad.tint)
            ),
        );
        let cards = frame
            .quads()
            .iter()
            .filter(|quad| quad.tint == theme::BAR)
            .count();
        checks.require(
            cards >= run.report_flow.events.len(),
            "the resolution screen drew fewer event cards than the run produced",
            format!(
                "beat {beat}: {} events and {cards} card fills",
                run.report_flow.events.len()
            ),
        );
    }
}
