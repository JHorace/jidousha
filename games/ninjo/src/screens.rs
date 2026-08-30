//! The Draw systems: the map, the chrome over it, and the panels as data.
//!
//! One screen (the map), three drawers over it (feed, auto-pause config,
//! tuning) and the attention surfaces over the map itself (`panels.rs`).
//! `content` is the
//! whole of what the chrome and the map's labels say, as data — `verify.rs`
//! and `floors.rs` read it, and `draw_content` is the only code that turns it
//! into quads. The terrain and the party tokens are drawn here directly: the
//! terrain because a 48x27 map is a thousand quads no panel should carry, and
//! the tokens because their between-tile position is derived at draw time
//! from the clock and `Time::alpha` (ADR-0041) — `token_position` is the one
//! function, shared with the checks, and nothing writes its answer back.

use jidousha::prelude::*;

use crate::attention;
use crate::camera::UiMap;
use crate::clock::{Clock, Rate, stamp};
use crate::constants::Tuning;
use crate::flow::Flow;
use crate::grid::{Grid, LOCATIONS, TOWN, Tile};
use crate::lens::Lens;
use crate::sim::{Activity, Sim};
use crate::sprites::Art;
use crate::ui::{self, IconRun, Panel, TextRun};
use crate::{layout, panels, sim, theme, tuning};

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
///
/// **Takes a [`Lens`] and no `Sim`.** That is the structural half of the
/// knowledge-lens rule (`lens.rs`): a screen cannot read around the lens
/// because a screen has nothing else to read. When the knowledge module makes
/// the lens conditional, this function does not change.
pub fn content(flow: &Flow, lens: &Lens<'_>, clock: &Clock, tuning: &Tuning) -> Panel {
    let mut panel = Panel::default();

    // --- top bar ------------------------------------------------------------
    panel.text(TextRun::new(
        layout::title_at(),
        "ninjo",
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
        format!("{}g", lens.treasury()),
        theme::HEAD,
        theme::GOLD,
    ));
    for (rect, label) in [
        (layout::feed_button(), "FEED"),
        (layout::tune_button(), "TUNE"),
        (layout::modes_button(), "MODES"),
    ] {
        panel.text(TextRun::new(
            ui::centered(rect, label, theme::SMALL, rect.min.y + 10.0),
            label,
            theme::SMALL,
            theme::DIM,
        ));
    }
    // **Under an open drawer, the map's own chrome says nothing.** A drawer
    // covers the screen, so a banner or a toast drawn beneath it is a row
    // nobody can read lying across a control somebody can click — and the
    // floors judge exactly that. The tuning drawer carries the toast in its
    // own prose band, so nothing is lost by keeping quiet here.
    let bare = !flow.feed_open && !flow.modes_open && !flow.tuner.open;
    if bare && let Some(toast) = &flow.toast {
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
    for (index, party) in lens.parties().iter().enumerate() {
        let chip = layout::party_chip(index);
        panel.icon(IconRun::new(
            chip.min + Vec2::splat(layout::pchip::PORTRAIT),
            party.token,
            layout::pchip::PORTRAIT_SCALE,
        ));
        let picked = flow.selected == Some(index);
        // The token carries a character's name: the party is a band somebody
        // in the registry fields, and the face on the chip is their portrait.
        panel.text(TextRun::new(
            chip.min + Vec2::new(layout::pchip::NAME_X, layout::pchip::NAME_TOP),
            format!("{} - {}", party.name, lens.name(party.member)),
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
            let open = lens.open_quests(index);
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

    // --- the cast, standing at their homes ---------------------------------
    // Idle: autonomy is wave 1, so a character is at their home tile unless a
    // party they field has them out (`Lens::at_home`, derived and never
    // stored). Names ride along because a face with no name is a token, and
    // the whole point of the people substrate is that these are people.
    for (index, person) in lens.people().iter().enumerate() {
        if !lens.at_home(index) {
            continue;
        }
        let art = person.icon;
        let mut figure = IconRun::new(
            layout::home_rect(person.home).min,
            art,
            art.scale_across(layout::HOME),
        );
        figure.layer = theme::layers::MARKER;
        panel.world_icon(figure);
        let style = theme::text(theme::SMALL, theme::INK);
        let width = style.width_of(person.name);
        let mut label = TextRun::new(
            layout::home_label(person.home, width),
            person.name,
            theme::SMALL,
            theme::INK,
        );
        label.layer = theme::layers::MAP_TEXT;
        panel.world_text(label);
    }

    // --- the attention surfaces over the map (GDD §3, wave 0a) -------------
    if bare {
        panel.absorb(panels::glance(flow, lens));
    }

    // --- drawers ------------------------------------------------------------
    if flow.feed_open {
        panel.absorb(panels::feed_drawer(flow, lens, tuning));
    }
    if flow.modes_open {
        panel.absorb(panels::modes_drawer(lens));
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
    let flow = ctx.world.resource::<Flow>().clone();
    let gallery = ctx.world.resource::<crate::sprites::Gallery>().clone();
    let sim = ctx.world.resource::<Sim>().clone();
    let lens = Lens::on(&sim);

    // The selection ring under a chosen character's figure, and the pulse a
    // click-to-focus left on the place it jumped to. Both presentation: one is
    // UI state, the other is a countdown in wall ticks, and the simulation
    // reads neither.
    if let Some(who) = flow.selected_person
        && lens.at_home(who)
        && let Some(home) = lens.home(who)
    {
        let ring = layout::home_rect(home);
        ctx.rect(
            Rect {
                min: ring.min - Vec2::splat(3.0),
                max: ring.max + Vec2::splat(3.0),
            },
            theme::GOLD,
            Depth {
                layer: theme::layers::MARKER,
                z: -1.0,
            },
        );
    }
    if let Some(pulse) = flow.pulse {
        let rect = pulse.tile.rect();
        ui::border(
            ctx,
            Rect {
                min: rect.min - Vec2::splat(8.0),
                max: rect.max + Vec2::splat(8.0),
            },
            theme::GOLD,
            2.0,
            theme::layers::MAP_TEXT,
        );
    }
    for (index, party) in lens.parties().iter().enumerate() {
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
    let sim_parties = Lens::on(ctx.world.resource::<Sim>()).parties().len();

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
    for rect in [
        layout::feed_button(),
        layout::tune_button(),
        layout::modes_button(),
    ] {
        fill(ctx, rect, theme::GHOST, theme::layers::PIECE - 1);
        border(ctx, rect, theme::BORDER, theme::layers::PIECE - 1);
    }

    // The meters band, and one chip per registered aggregate.
    fill(
        ctx,
        layout::meters_band(),
        theme::STRIP,
        theme::layers::PANEL,
    );
    let drilled = flow.drilled;
    for index in 0..crate::meters::METERS.len() {
        let chip = layout::meter_chip(index);
        fill(ctx, chip, theme::PANEL, theme::layers::CARD);
        border(
            ctx,
            chip,
            if drilled == Some(index) {
                theme::GOLD
            } else {
                theme::BORDER
            },
            theme::layers::CARD,
        );
    }
    if drilled.is_some() {
        fill(
            ctx,
            layout::faces_panel(),
            theme::PANEL,
            theme::layers::CARD,
        );
        border(
            ctx,
            layout::faces_panel(),
            theme::BORDER,
            theme::layers::CARD,
        );
        for row in 0..layout::FACE_ROWS {
            border(
                ctx,
                layout::faces_row(row),
                theme::GHOST,
                theme::layers::PIECE - 1,
            );
        }
    }
    if flow.selected_person.is_some() {
        fill(
            ctx,
            layout::person_panel(),
            theme::PANEL,
            theme::layers::CARD,
        );
        border(
            ctx,
            layout::person_panel(),
            theme::GOLD,
            theme::layers::CARD,
        );
        ghost(ctx, &map, layout::person_close());
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
    if flow.feed_open || flow.modes_open {
        fill(
            ctx,
            layout::feed_panel(),
            theme::SCRIM,
            theme::layers::OVERLAY,
        );
        border(
            ctx,
            layout::feed_panel(),
            theme::BORDER,
            theme::layers::OVERLAY,
        );
    }
    if flow.feed_open {
        ghost(ctx, &map, layout::feed_ignored_toggle());
        let triggered = Lens::on(ctx.world.resource::<Sim>())
            .pause()
            .map(|pause| pause.event);
        let entries = {
            let sim = ctx.world.resource::<Sim>();
            let lens = Lens::on(sim);
            crate::attention::feed(&lens, flow.show_ignored, attention::feed_cap(&active))
        };
        for (row, entry) in entries.iter().take(layout::FEED_ROWS).enumerate() {
            let rect = layout::feed_row(row);
            fill(ctx, rect, theme::GHOST, theme::layers::OVERLAY);
            // The entry an auto-pause fired on wears the gold: the reason line
            // above and the row it names cannot point at two different things.
            if triggered == Some(entry.index) {
                border(ctx, rect, theme::GOLD, theme::layers::OVERLAY + 1);
            }
        }
    }
    if flow.modes_open {
        let held: Vec<crate::attention::Mode> = {
            let sim = ctx.world.resource::<Sim>();
            let lens = Lens::on(sim);
            crate::attention::EventClass::all()
                .into_iter()
                .map(|class| lens.attention().mode(class))
                .collect()
        };
        for (row, mode_held) in held.into_iter().enumerate() {
            for (slot, mode) in crate::attention::Mode::ALL.iter().copied().enumerate() {
                let button = layout::modes_radio(row, slot);
                if mode == mode_held {
                    ui::button(ctx, map.to_world_rect(button), true, theme::layers::OVERLAY);
                } else {
                    ghost(ctx, &map, button);
                }
            }
        }
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
    let panel = content(&flow, &Lens::on(&sim), &clock, &tuning);
    ui::draw(ctx, &panel, &map);
}
