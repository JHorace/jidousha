//! The Draw systems: the map, the chrome over it, and the panels as data.
//!
//! One screen (the map), two drawers over it (log, tuning). `content` is the
//! whole of what the chrome and the map's labels say, as data — `verify.rs`
//! and `floors.rs` read it, and `draw_content` is the only code that turns it
//! into quads. The terrain and the party tokens are drawn here directly: the
//! terrain because a 48x27 map is a thousand quads no panel should carry, and
//! the tokens because their between-tile position is derived at draw time
//! from the clock and `Time::alpha` (ADR-0041) — `token_position` is the one
//! function, shared with the checks, and nothing writes its answer back.

use jidousha::prelude::*;

use crate::camera::UiMap;
use crate::clock::{Clock, Rate, stamp};
use crate::constants::Tuning;
use crate::flow::Flow;
use crate::grid::{Grid, LOCATIONS, TOWN, Tile};
use crate::sim::{Activity, Sim};
use crate::sprites::Art;
use crate::ui::{self, IconRun, Panel, TextRun};
use crate::{layout, sim, theme, tuning};

/// The tiles the camera can currently see — the game-side culling (DESIGN
/// §8): the map is the largest sprite count this engine has been asked for,
/// and the game culls to camera bounds on its own.
pub fn visible_tiles(grid: &Grid, view: Rect) -> (Tile, Tile) {
    let min = Tile::new(
        (view.min.x / crate::grid::TILE).floor() as i32,
        (view.min.y / crate::grid::TILE).floor() as i32,
    );
    let max = Tile::new(
        (view.max.x / crate::grid::TILE).ceil() as i32,
        (view.max.y / crate::grid::TILE).ceil() as i32,
    );
    (
        Tile::new(min.x.max(0), min.y.max(0)),
        Tile::new(max.x.min(grid.width - 1), max.y.min(grid.height - 1)),
    )
}

/// Where a party's token is drawn, at a fractional world-minute reading.
///
/// Discrete sim state in, presentation out: between tiles the token sits the
/// travelled fraction of the way from the tile it is on to the one it will
/// enter, by the world clock — so it moves with the clock, holds under
/// pause, and never exists anywhere the sim can read it back.
pub fn token_position(party: &sim::Party, now: f32) -> Vec2 {
    let (from, to, entered, next) = match &party.activity {
        Activity::Outbound {
            route,
            index,
            entered_at,
            next_at,
        }
        | Activity::Homebound {
            route,
            index,
            entered_at,
            next_at,
        } => {
            let Some(next_tile) = route.tiles.get(*index).copied() else {
                return party.tile.center();
            };
            (party.tile, next_tile, *entered_at, *next_at)
        }
        _ => return party.tile.center(),
    };
    let span = next.saturating_sub(entered);
    if span == 0 {
        return to.center();
    }
    let fraction = ((now - entered as f32) / span as f32).clamp(0.0, 1.0);
    from.center().lerp(to.center(), fraction)
}

/// Everything the screen says, as data: the chrome in UI units, the map's
/// labels in world units.
pub fn content(flow: &Flow, sim: &Sim, clock: &Clock, tuning: &Tuning) -> Panel {
    let mut panel = Panel::default();

    // --- top bar ------------------------------------------------------------
    panel.text(TextRun::new(
        layout::title_at(),
        "giri-rt",
        theme::HEAD,
        theme::INK,
    ));
    panel.text(TextRun::new(
        layout::clock_at(),
        stamp(clock.minutes),
        theme::HEAD,
        theme::GOLD,
    ));
    for (index, label) in chip_labels().into_iter().enumerate() {
        let chip = layout::speed_chip(index);
        let active = match index {
            0 => clock.paused,
            1 => !clock.paused && clock.rate == Rate::X1,
            2 => !clock.paused && clock.rate == Rate::X2,
            _ => !clock.paused && clock.rate == Rate::X4,
        };
        panel.text(TextRun::new(
            ui::centered(chip, label, theme::SMALL, chip.min.y + 10.0),
            label,
            theme::SMALL,
            if active { theme::GROUND } else { theme::DIM },
        ));
    }
    panel.icon(IconRun::new(layout::treasury_icon_at(), Art::Coin, 2.0));
    panel.text(TextRun::new(
        layout::treasury_text_at(),
        format!("{}g", sim.treasury),
        theme::HEAD,
        theme::GOLD,
    ));
    for (rect, label) in [
        (layout::log_button(), "LOG"),
        (layout::tune_button(), "TUNE"),
    ] {
        panel.text(TextRun::new(
            ui::centered(rect, label, theme::SMALL, rect.min.y + 10.0),
            label,
            theme::SMALL,
            theme::DIM,
        ));
    }
    if let Some(toast) = &flow.toast {
        panel.text(TextRun::new(
            layout::toast_at(),
            toast.text.clone(),
            theme::SMALL,
            theme::GOLD,
        ));
    }

    // --- party strip --------------------------------------------------------
    panel.text(TextRun::new(
        layout::party_label(),
        "PARTIES - click an idle one, then a site on the map",
        theme::SMALL,
        theme::DIM,
    ));
    for (index, party) in sim.parties.iter().enumerate() {
        let chip = layout::party_chip(index);
        panel.icon(IconRun::new(
            chip.min + Vec2::splat(layout::pchip::PORTRAIT),
            party.token,
            layout::pchip::PORTRAIT_SCALE,
        ));
        let picked = flow.selected == Some(index);
        panel.text(TextRun::new(
            chip.min + Vec2::new(layout::pchip::NAME_X, layout::pchip::NAME_TOP),
            party.name,
            theme::SMALL,
            if picked { theme::GOLD } else { theme::INK },
        ));
        panel.text(TextRun::new(
            chip.min + Vec2::new(layout::pchip::NAME_X, layout::pchip::STATUS_TOP),
            party.status(),
            theme::SMALL,
            match party.activity {
                Activity::Idle => theme::DIM,
                Activity::Working { .. } => theme::REGARD,
                _ => theme::INK,
            },
        ));
    }

    // --- the map's own words: markers' labels and quest counts -------------
    for (index, spec) in LOCATIONS.iter().enumerate() {
        let style = theme::text(theme::SMALL, theme::INK);
        let width = style.width_of(spec.name);
        let at = layout::marker_label(spec.tile, width);
        let mut label = TextRun::new(at, spec.name, theme::SMALL, theme::INK);
        label.layer = theme::layers::MAP_TEXT;
        panel.world_text(label);
        if index != TOWN {
            let open = sim
                .sites
                .iter()
                .find(|site| site.location == index)
                .map_or(0, |site| site.quests.len() - site.claimed);
            let line = match open {
                0 => "dry".to_owned(),
                1 => "1 quest".to_owned(),
                more => format!("{more} quests"),
            };
            let width = style.width_of(&line);
            let mut count = TextRun::new(
                layout::marker_label(spec.tile, width) + Vec2::new(0.0, theme::SMALL + 2.0),
                line,
                theme::SMALL,
                if open == 0 { theme::FAINT } else { theme::DIM },
            );
            count.layer = theme::layers::MAP_TEXT;
            panel.world_text(count);
        }
        let art = Art::for_icon(spec.icon);
        let mut marker = IconRun::new(
            layout::marker_rect(spec.tile).min,
            art,
            art.scale_across(layout::MARKER),
        );
        marker.layer = theme::layers::MARKER;
        panel.world_icon(marker);
    }

    // --- drawers ------------------------------------------------------------
    if flow.log_open {
        panel.absorb(log_panel_content(flow));
    }
    if flow.tuner.open {
        panel.absorb(tuning::drawer(flow, tuning));
    }
    panel
}

/// The chips' labels, in chip order.
pub fn chip_labels() -> [&'static str; layout::CHIPS] {
    ["PAUSE", "1x", "2x", "4x"]
}

/// The log drawer's rows, as data.
fn log_panel_content(flow: &Flow) -> Panel {
    let mut panel = Panel::default();
    panel.text(TextRun::over(
        layout::log_title(),
        "LOG - newest first - every event carries its world-time",
        theme::SMALL,
        theme::DIM,
    ));
    for (index, line) in flow.log.iter().take(layout::LOG_ROWS).enumerate() {
        panel.text(TextRun::over(
            layout::log_row(index),
            line.clone(),
            theme::SMALL,
            theme::INK,
        ));
    }
    panel
}

/// The terrain, drawn from the sim's own grid — the second reader of the one
/// grid (DESIGN §3), culled to the camera's bounds.
pub fn draw_map(ctx: &mut DrawCtx) {
    let grid = ctx.world.resource::<Grid>();
    let view = ctx.world.resource::<Camera>().visible_bounds();
    let (min, max) = visible_tiles(grid, view);
    for y in min.y..=max.y {
        for x in min.x..=max.x {
            let tile = Tile::new(x, y);
            ctx.rect(
                tile.rect(),
                grid.get(tile).color(),
                Depth::layer(theme::layers::TERRAIN),
            );
        }
    }

    // The party tokens: discrete state, smooth at draw time (ADR-0041). The
    // fractional reading is interpolated between the previous tick's and this
    // one's by `Time::alpha`; headless draws land exactly on the tick.
    let alpha = ctx.world.resource::<Time>().alpha;
    let tuning = *ctx.world.resource::<Tuning>();
    let clock = ctx.world.resource::<Clock>();
    let now = clock.previous_reading + (clock.reading(&tuning) - clock.previous_reading) * alpha;
    let flow = ctx.world.resource::<Flow>();
    let gallery = ctx.world.resource::<crate::sprites::Gallery>().clone();
    let sim = ctx.world.resource::<Sim>();
    for (index, party) in sim.parties.iter().enumerate() {
        let at = token_position(party, now)
            - Vec2::splat(layout::TOKEN * 0.5)
            // Two parties on one tile stay two tokens: a draw-time nudge per
            // party, never written back.
            + Vec2::new(index as f32 * 4.0, index as f32 * -4.0);
        // Culled like the terrain: a token panned off screen submits nothing.
        if !Rect::from_min_size(at, Vec2::splat(layout::TOKEN)).overlaps(view) {
            continue;
        }
        if flow.selected == Some(index) {
            ctx.rect(
                Rect::from_min_size(at - Vec2::splat(2.0), Vec2::splat(layout::TOKEN + 4.0)),
                theme::GOLD,
                Depth {
                    layer: theme::layers::TOKEN,
                    z: -1.0,
                },
            );
        }
        let sprite = gallery.sprite(party.token, 2.0, theme::layers::TOKEN, Color::WHITE);
        ctx.sprite(&Transform::at(at), &sprite);
    }
}

/// The chrome's fills and buttons, through the UI mapping.
pub fn draw_chrome(ctx: &mut DrawCtx) {
    let map = UiMap::for_camera(ctx.world.resource::<Camera>());
    let flow = ctx.world.resource::<Flow>().clone();
    let clock = *ctx.world.resource::<Clock>();
    let active = *ctx.world.resource::<Tuning>();
    let sim_parties = ctx.world.resource::<Sim>().parties.len();

    let fill = |ctx: &mut DrawCtx, rect: Rect, color: Color, layer: i16| {
        ui::fill(ctx, map.to_world_rect(rect), color, layer);
    };
    let border = |ctx: &mut DrawCtx, rect: Rect, color: Color, layer: i16| {
        ui::border(ctx, map.to_world_rect(rect), color, 2.0 * map.scale, layer);
    };

    // Top bar and party strip.
    fill(ctx, layout::topbar(), theme::BAR, theme::layers::PANEL);
    fill(
        ctx,
        layout::party_strip(),
        theme::STRIP,
        theme::layers::PANEL,
    );
    for index in 0..layout::CHIPS {
        let chip = layout::speed_chip(index);
        let active_chip = match index {
            0 => clock.paused,
            1 => !clock.paused && clock.rate == Rate::X1,
            2 => !clock.paused && clock.rate == Rate::X2,
            _ => !clock.paused && clock.rate == Rate::X4,
        };
        fill(
            ctx,
            chip,
            if active_chip {
                theme::GOLD
            } else {
                theme::GHOST
            },
            theme::layers::PIECE,
        );
        if !active_chip {
            border(ctx, chip, theme::BORDER, theme::layers::PIECE);
        }
    }
    for rect in [layout::log_button(), layout::tune_button()] {
        fill(ctx, rect, theme::GHOST, theme::layers::PIECE - 1);
        border(ctx, rect, theme::BORDER, theme::layers::PIECE - 1);
    }
    for index in 0..sim_parties {
        let chip = layout::party_chip(index);
        fill(ctx, chip, theme::PANEL, theme::layers::CARD);
        border(
            ctx,
            chip,
            if flow.selected == Some(index) {
                theme::GOLD
            } else {
                theme::BORDER
            },
            theme::layers::CARD,
        );
    }

    // Drawers.
    if flow.log_open {
        fill(
            ctx,
            layout::log_panel(),
            theme::SCRIM,
            theme::layers::OVERLAY,
        );
        border(
            ctx,
            layout::log_panel(),
            theme::BORDER,
            theme::layers::OVERLAY,
        );
    }
    if flow.tuner.open {
        fill(
            ctx,
            layout::tuner_panel(),
            theme::BAR,
            theme::layers::OVERLAY,
        );
        border(
            ctx,
            layout::tuner_panel(),
            theme::TUNE_EDGE,
            theme::layers::OVERLAY,
        );
        for index in 0..crate::presets::PRESETS.len() {
            ghost(ctx, &map, layout::tuner_preset(index));
        }
        for index in 0..crate::constants::Field::ALL.len() {
            ghost(ctx, &map, layout::tuner_minus(index));
            ghost(ctx, &map, layout::tuner_plus(index));
        }
        let live = tuning::dirty(&flow.tuner.pending, &active);
        let apply = map.to_world_rect(layout::tuner_apply());
        ui::button(ctx, apply, live, theme::layers::OVERLAY + 1);
    }
}

/// A ghost button on the overlay band, through the UI mapping.
fn ghost(ctx: &mut DrawCtx, map: &UiMap, rect: Rect) {
    ui::fill(
        ctx,
        map.to_world_rect(rect),
        theme::GHOST,
        theme::layers::OVERLAY + 1,
    );
    ui::border(
        ctx,
        map.to_world_rect(rect),
        theme::BORDER,
        2.0 * map.scale,
        theme::layers::OVERLAY + 1,
    );
}

/// Every string and icon of the screen, drawn through the mapping.
pub fn draw_content(ctx: &mut DrawCtx) {
    let map = UiMap::for_camera(ctx.world.resource::<Camera>());
    let flow = ctx.world.resource::<Flow>().clone();
    let clock = *ctx.world.resource::<Clock>();
    let tuning = *ctx.world.resource::<Tuning>();
    let sim = ctx.world.resource::<Sim>().clone();
    let panel = content(&flow, &sim, &clock, &tuning);
    ui::draw(ctx, &panel, &map);
}
