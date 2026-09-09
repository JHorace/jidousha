//! The Draw systems. Every one of them reads `FeedState` and the layout
//! constants in `game.rs` and nothing else — the same reader the checks use, so
//! a picture and an assertion cannot disagree.
//!
//! Bands come from `game::layers`; submission order does not decide draw order.
//! The panel is submitted before the square on purpose, so a check can see the
//! `PANEL` band lift it back on top (jidousha-testing.md: a band is only
//! visible where it disagrees with the submission order).

use jidousha::prelude::*;

use crate::backend::FeedState;
use crate::game::{self, layers};
use crate::sim::Pop;

/// Draw `text` centred on `center_x`, with its top at `y`.
fn centered(ctx: &mut DrawCtx, center_x: f32, y: f32, text: &str, style: TextStyle) {
    ctx.text(
        Vec2::new(center_x - style.width_of(text) * 0.5, y),
        text,
        style,
    );
}

/// Cut a name to what a leaderboard row has room for.
fn short(name: &str, width: usize) -> &str {
    match name.char_indices().nth(width) {
        Some((byte, _)) => &name[..byte],
        None => name,
    }
}

/// The leaderboard panel: a filled band, a header carrying the global count,
/// ten ranked rows, and the player's own row pinned below when they are not in
/// the ten (GDD §4.2).
pub fn draw_panel(ctx: &mut DrawCtx) {
    let panel = game::leaderboard_panel();
    let state = ctx.world.resource::<FeedState>();
    ctx.rect(panel, game::PANEL, Depth::layer(layers::PANEL));

    let header = TextStyle {
        size: 0.42,
        color: game::TEXT,
        depth: Depth::layer(layers::TEXT),
        ..TextStyle::default()
    };
    centered(
        ctx,
        panel.center().x,
        panel.min.y + 0.22,
        &format!("TOP FEEDERS   {} FED", state.global),
        header,
    );

    let row_style = TextStyle {
        size: 0.4,
        color: game::TEXT,
        depth: Depth::layer(layers::TEXT),
        ..TextStyle::default()
    };
    let top_y = panel.min.y + 0.95;
    let pitch = 0.46;
    for (index, entry) in state.board.top.iter().enumerate() {
        let line = format!(
            "{:>2} {:<13} {:>7}",
            index + 1,
            short(&entry.name, 13),
            entry.count
        );
        let style = TextStyle {
            color: if entry.you {
                game::TEXT_YOU
            } else {
                game::TEXT
            },
            ..row_style
        };
        centered(
            ctx,
            panel.center().x,
            top_y + index as f32 * pitch,
            &line,
            style,
        );
    }

    if let Some(row) = &state.board.your_row {
        centered(
            ctx,
            panel.center().x,
            panel.max.y - 0.55,
            &format!(
                "#{:<4} {:<11} {:>7}",
                state.board.your_rank, "YOU", row.count
            ),
            TextStyle {
                color: game::TEXT_YOU,
                ..row_style
            },
        );
    }
}

/// The orange man: a square sized by the global feed count, with the tap bounce
/// applied and the whole thing clamped so mashing cannot shove it off screen.
pub fn draw_square(ctx: &mut DrawCtx) {
    let global = ctx.world.resource::<FeedState>().global;
    let pop = ctx.world.resource::<Pop>().scale;
    let side = (game::square_size(global) * pop).min(game::SQUARE_MAX);
    ctx.rect(
        Rect::from_center_size(game::SQUARE_CENTER, Vec2::splat(side)),
        game::ORANGE,
        Depth::layer(layers::MAN),
    );
}

/// The title and the player's own count, above the man.
pub fn draw_readout(ctx: &mut DrawCtx) {
    let state = ctx.world.resource::<FeedState>();
    centered(
        ctx,
        0.0,
        game::VIEW_MIN.y + 0.5,
        "FAT ORANGE MAN",
        TextStyle {
            size: 0.8,
            color: game::ORANGE,
            depth: Depth::layer(layers::TEXT),
            ..TextStyle::default()
        },
    );
    centered(
        ctx,
        0.0,
        game::VIEW_MIN.y + 1.6,
        &format!("YOU HAVE FED HIM {}", state.personal),
        TextStyle {
            size: 0.45,
            color: game::TEXT,
            depth: Depth::layer(layers::TEXT),
            ..TextStyle::default()
        },
    );
}

/// The feed button: a filled rectangle with its label centred inside.
pub fn draw_button(ctx: &mut DrawCtx) {
    let button = game::feed_button();
    ctx.rect(button, game::BUTTON, Depth::layer(layers::BUTTON));
    let label = TextStyle {
        size: 0.9,
        color: game::BUTTON_TEXT,
        depth: Depth::layer(layers::TEXT),
        ..TextStyle::default()
    };
    centered(
        ctx,
        button.center().x,
        button.center().y - label.size * 0.5,
        "FEED HIM",
        label,
    );
}
