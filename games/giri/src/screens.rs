//! The Draw systems: the chrome each screen sits in, and the panels that fill
//! it.
//!
//! Three modes, three screens (UI.md §1.3). The board is drawn every frame; the
//! log drawer and the resolution takeover are overlays over it, on bands of
//! their own so nothing on the board can show through. `content` is the whole
//! of what a screen says as data — `verify.rs` and `floors.rs` read it, and
//! `draw_*` is the only code that turns it into quads.

use jidousha::prelude::*;

use crate::flow::{Flow, Preview, Stage};
use crate::model::Social;
use crate::ui::{self, Panel};
use crate::{board, layout, party, resolution, theme};

/// Everything the board says, as data.
pub fn board_content(flow: &Flow, social: &Social, preview: &Preview) -> Panel {
    let mut panel = board::status_bar(flow);
    panel.absorb(board::quest_row(flow));
    panel.absorb(board::dilemma(flow));
    panel.absorb(board::info(flow, social, preview));
    panel.absorb(party::strip(flow, social, preview));
    panel.absorb(party::send(flow, preview));
    panel.absorb(party::toast(flow));
    panel.text(ui::TextRun::new(
        ui::centered(
            layout::log_button(),
            "LOG",
            theme::SMALL,
            layout::log_button().min.y + 10.0,
        ),
        "LOG",
        theme::SMALL,
        theme::DIM,
    ));
    panel
}

/// Everything the screen the game is on says, as data.
///
/// **The takeover replaces the board rather than covering it** (UI.md §3), so
/// on those two screens the board is not built at all. A board drawn behind an
/// opaque overlay is invisible and still on the frame, which costs a draw and -
/// worse - puts a second row of glyphs on every row an assertion counts.
pub fn content(flow: &Flow, social: &Social, preview: &Preview) -> Panel {
    match flow.stage {
        Stage::Board => {
            let mut panel = board_content(flow, social, preview);
            if flow.log_open {
                panel.absorb(board::log(flow));
            }
            panel
        }
        Stage::Resolution => resolution::takeover(flow),
        Stage::Complete => resolution::complete(),
    }
}

/// The ground: the design rect, and the letterbox around it.
///
/// The camera clears to `VOID` and this fills the 960x540 the layout is stated
/// in with `GROUND`, so the letterbox the scaling contract leaves is visibly a
/// letterbox rather than a mysteriously wider game (UI.md §6).
pub fn draw_ground(ctx: &mut DrawCtx) {
    ui::fill(ctx, layout::design(), theme::GROUND, theme::layers::GROUND);
}

/// The board's chrome: bars, cards, panels and buttons.
pub fn draw_board(ctx: &mut DrawCtx) {
    let flow = ctx.world.resource::<Flow>().clone();
    if flow.stage != Stage::Board {
        return;
    }
    let preview = ctx.world.resource::<Preview>().clone();
    let social = Social::read(&ctx.world);

    // top bar
    ui::fill(ctx, layout::topbar(), theme::BAR, theme::layers::PANEL);
    ui::fill(
        ctx,
        Rect::from_min_size(
            Vec2::new(0.0, layout::topbar().max.y - 2.0),
            Vec2::new(layout::design().size().x, 2.0),
        ),
        theme::RULE,
        theme::layers::CARD,
    );

    // quest cards
    if let Some(beat) = flow.spec() {
        for index in 0..beat.dungeons.len().min(layout::QUEST_SLOTS) {
            let card = layout::quest_card(index);
            let taken = flow.taken == Some(index);
            let lit = flow.taken.is_none() || taken;
            ui::card(
                ctx,
                card,
                if lit { theme::PANEL } else { theme::GROUND },
                if taken {
                    theme::SELECT_RING
                } else if lit {
                    theme::BORDER
                } else {
                    theme::RULE
                },
                theme::layers::PANEL,
            );
        }
    }

    // the info panel, and the release control it grows when a quest is taken
    ui::card(
        ctx,
        layout::info_panel(),
        theme::PANEL,
        theme::BORDER,
        theme::layers::PANEL,
    );
    if flow.taken.is_some() && flow.taken == flow.shown() {
        ui::ghost_button(ctx, layout::release_button(), theme::layers::PIECE - 1);
    }
    ui::ghost_button(ctx, layout::log_button(), theme::layers::PIECE - 1);

    // the party strip
    let strip = layout::party_strip();
    ui::fill(ctx, strip, theme::STRIP, theme::layers::PANEL);
    ui::fill(
        ctx,
        Rect::from_min_size(strip.min, Vec2::new(strip.size().x, 2.0)),
        theme::RULE,
        theme::layers::CARD,
    );
    for (index, member) in social.members.iter().enumerate() {
        let card = layout::party_card(index);
        let inside = flow.party.contains(&member.entity);
        ui::card(
            ctx,
            card,
            if member.alive {
                theme::PANEL
            } else {
                theme::GROUND
            },
            if inside {
                theme::REGARD
            } else if member.alive {
                theme::BORDER
            } else {
                theme::RULE
            },
            theme::layers::PANEL,
        );
    }
    if flow.taken.is_some() {
        ui::button(
            ctx,
            layout::send_button(),
            preview.can_send,
            theme::layers::PIECE - 1,
        );
    }
}

/// The overlays: the drawer, the takeover, and the end of the chain.
pub fn draw_overlay(ctx: &mut DrawCtx) {
    let flow = ctx.world.resource::<Flow>().clone();
    match flow.stage {
        Stage::Board if flow.log_open => {
            ui::fill(
                ctx,
                layout::log_panel(),
                theme::SCRIM,
                theme::layers::OVERLAY,
            );
            ui::border(
                ctx,
                layout::log_panel(),
                theme::BORDER,
                2.0,
                theme::layers::OVERLAY,
            );
        }
        Stage::Board => {}
        Stage::Resolution | Stage::Complete => {
            ui::fill(
                ctx,
                layout::takeover(),
                theme::SCRIM,
                theme::layers::OVERLAY,
            );
            if flow.stage == Stage::Resolution {
                for (index, height) in resolution::card_heights(&flow).iter().enumerate() {
                    let card = layout::event_card(index, &resolution::card_heights(&flow));
                    let kill = flow
                        .events
                        .get(index)
                        .is_some_and(|event| event.kind == crate::resolve::EventKind::Kill);
                    let _ = height;
                    ui::card(
                        ctx,
                        card,
                        theme::BAR,
                        if kill { theme::EMBER } else { theme::RULE },
                        theme::layers::OVERLAY + 1,
                    );
                }
            }
        }
    }
}

/// Every string and every icon of the current screen, drawn.
pub fn draw_content(ctx: &mut DrawCtx) {
    let flow = ctx.world.resource::<Flow>().clone();
    let preview = ctx.world.resource::<Preview>().clone();
    let social = Social::read(&ctx.world);
    let panel = content(&flow, &social, &preview);
    ui::draw(ctx, &panel);
}
