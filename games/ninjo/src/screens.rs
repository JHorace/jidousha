//! The Draw systems: the map, the chrome over it, and the panels as data.
//!
//! One screen (the map), three drawers over it (feed, auto-pause config,
//! tuning) and the attention surfaces over the map itself (`panels.rs`).
//! `content` is the
//! whole of what the chrome and the map's labels say, as data — `verify.rs`
//! and `floors.rs` read it, and `draw_content` is the only code that turns it
//! into quads.
//!
//! **The cast is `Panel` content, interpolation and all.** A person's figure
//! is a world icon at `where_drawn`, whose between-tile half is derived at
//! draw time from the clock and `Time::alpha` (ADR-0041) — `token_position`
//! is the one function, shared with the checks, and nothing writes its answer
//! back. It was drawn straight through `ctx.sprite` until the double-drawn
//! cast (`FINDINGS.md` G-023): the interpolation was taken to need a draw of
//! its own, it did not, and being outside the `Panel` is what put the whole
//! cast outside the floors that would have counted it.
//!
//! What is still drawn here directly is stated in UI.md §6 and nowhere else:
//! the terrain, because a 48x27 map is a thousand quads no panel should
//! carry; and the selection ring and the focus pulse, because both are fills
//! rather than content and neither says anything the floors read.

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

/// The world-minute the map is drawn at: this tick's reading, carried the
/// fraction of the way towards the next one that this frame stands at
/// (ADR-0041, DESIGN §3).
///
/// One formula, because the figures, the ring that marks one of them and the
/// hit-test that answers a click on one all have to agree about where
/// somebody is. A headless draw passes [`TICK`] and lands exactly on the tick.
pub fn reading(clock: &Clock, tuning: &Tuning, alpha: f32) -> f32 {
    clock.previous_reading + (clock.reading(tuning) - clock.previous_reading) * alpha
}

/// The interpolation a headless draw stands at: exactly on the tick.
pub const TICK: f32 = 1.0;

/// How close two figures have to be before they read as one person.
///
/// Half a figure. Wave 1.1's duplicate is what fixes the number: at four and
/// eight units of separation the second copy read as a thickened sprite
/// rather than as a second person (`FINDINGS.md` G-023), and a nudge is only
/// worth drawing if what comes out of it is two people.
const TOGETHER: f32 = layout::HOME * 0.5;

/// How far a nudged figure steps off the place it stands, per ring.
const NUDGE: f32 = 12.0;

/// The four ways a nudge steps, in the order the ranks take them.
///
/// Diagonals, so a step separates a figure in both axes at once and `NUDGE`
/// buys `NUDGE * sqrt(2)` of separation against the rank that did not move.
const NUDGE_WAYS: [Vec2; 4] = [
    Vec2::new(1.0, -1.0),
    Vec2::new(-1.0, 1.0),
    Vec2::new(-1.0, -1.0),
    Vec2::new(1.0, 1.0),
];

/// The draw-time nudge that keeps people standing in one place separate
/// figures, by their rank among the people who are actually there.
///
/// **Rank 0 takes no nudge at all** — and everybody standing alone is rank 0,
/// which is what makes a person on a tile drawn on their tile. Ranks past the
/// four ways step out a further `NUDGE`, so a crowd spreads instead of
/// stacking and no two ranks ever land on one another.
///
/// DELIBERATE: the rank is computed from who is co-located at this instant
/// and never from a registry index. Wave 1.1's nudge was `index * 4` — S1
/// residue from three parties stacked on the town tile — which drew person 9
/// over two tiles from the tile they were standing on, and a figure that lies
/// about where somebody is lies to a click and to a camera focus too.
fn nudge(rank: usize) -> Vec2 {
    let Some(step) = rank.checked_sub(1) else {
        return Vec2::ZERO;
    };
    let ring = (step / NUDGE_WAYS.len() + 1) as f32;
    NUDGE_WAYS[step % NUDGE_WAYS.len()] * (NUDGE * ring)
}

/// Where a person *is standing*, which is not the same question as where
/// their picture goes: their own doorstep while they are home, their party's
/// derived position while they are on the road, and no nudge at all.
///
/// [`where_drawn`] is the answer anything that draws, rings or hit-tests
/// wants. This one is what the nudge is computed against and what
/// `floors::judge_cast` holds it to — a figure that is not standing on its
/// person's place has to have somebody else's figure as its reason.
///
/// `None` for an index that names nobody, which is the only answer there is:
/// a person who is not in the registry is not standing anywhere.
pub fn stands_at(lens: &Lens<'_>, who: usize, now: f32) -> Option<Vec2> {
    // INVARIANT: an idle party stands on its member's home tile, so this
    // branch and the one below agree today — and it is here because the
    // *person's* door is the authority for where a person at home stands, not
    // a party's tile bookkeeping. `verify::one_figure_each` asserts the two
    // agree, so the day they stop this fails a run rather than quietly
    // drawing somebody at a stale tile.
    if lens.at_home(who) {
        return lens.home(who).map(|home| home.center());
    }
    lens.parties()
        .get(who)
        .map(|party| token_position(party, now))
}

/// **Where this person is drawn right now** — the one answer, and the whole
/// of what anything on the map may use.
///
/// One person, one figure, one place: their own doorstep while they are home,
/// their party's interpolated position while they are on the road, never
/// both. The figure draw, the name under it, the selection ring and the map's
/// hit-test all read this and nothing else, so a click, a ring and a picture
/// cannot end up in three places.
///
/// The nudge is a **placement**, not a formula over an index: everybody takes
/// the place they are standing if it is clear, and steps out a ring only
/// because somebody already there is close enough to read as them. So a
/// person standing alone is drawn on their own tile, and two people standing
/// together are two figures — which is what `floors::judge_cast` asserts, at
/// its own shipped literal.
///
/// INVARIANT: the party at index `who` is the person at index `who` — a party
/// is a one-person band in registry order (`sim::authored_parties`), which
/// `verify::one_selection` asserts over the whole roster.
///
/// INVARIANT: registry order is the order places are taken in, which is what
/// makes this answer the same one whoever asks it and in whatever order.
///
/// Presentation out of discrete state, derived at draw time and never written
/// back (ADR-0041): the interpolation is a reading of the clock, and nothing
/// here reaches the simulation.
pub fn where_drawn(lens: &Lens<'_>, who: usize, now: f32) -> Option<Rect> {
    stands_at(lens, who, now)?;
    // Everybody ahead of them in registry order has already taken their
    // place; walk the same placement they walked, so the answer does not
    // depend on who asked or when.
    let mut taken: Vec<Vec2> = Vec::with_capacity(who);
    for index in 0..=who {
        let Some(base) = stands_at(lens, index, now) else {
            continue;
        };
        // The ring grows until the spot is clear. One figure already placed
        // blocks at most two slots — the two adjacent rings in its own
        // direction, since the four slots of one ring stand 24 units apart —
        // so a clear slot always exists inside twice the placements made so
        // far, and the bound is a statement of that rather than a fallback.
        let mut rank = 0;
        while rank <= taken.len() * 2
            && taken
                .iter()
                .any(|at| at.distance(base + nudge(rank)) < TOGETHER)
        {
            rank += 1;
        }
        let placed = base + nudge(rank);
        if index == who {
            return Some(Rect::from_center_size(placed, Vec2::splat(layout::HOME)));
        }
        taken.push(placed);
    }
    None
}

/// How far the selection ring stands out past the figure it rings.
pub const RING: f32 = 3.0;

/// **The one selection ring**: the box to draw it in — `None` when nobody is
/// selected.
///
/// The whole of the game's selection highlighting on the map, in one function
/// that reads the one selection (`Flow::selected`) and asks [`where_drawn`]
/// where they are. It computes no position of its own, which is what makes
/// the ring land on the figure in both states rather than beside it.
pub fn selection_ring(flow: &Flow, lens: &Lens<'_>, now: f32) -> Option<Rect> {
    where_drawn(lens, flow.selected?, now)
}

/// Everything the screen says, as data: the chrome in UI units, the map's
/// labels in world units.
///
/// **Takes a [`Lens`] and no `Sim`.** That is the structural half of the
/// knowledge-lens rule (`lens.rs`): a screen cannot read around the lens
/// because a screen has nothing else to read. When the knowledge module makes
/// the lens conditional, this function does not change.
///
/// The `Grid` rides beside it because the site panel previews a journey, and
/// the journey is the pathfinder's own answer over the terrain the sim walks
/// (`Lens::travel` -> `sim::route_out`). The grid is static authored terrain
/// and `draw_map` has always read it directly; what goes through the lens is
/// the *answer*, which is what a screen would otherwise be tempted to compute.
pub fn content(
    flow: &Flow,
    lens: &Lens<'_>,
    grid: &Grid,
    clock: &Clock,
    tuning: &Tuning,
    now: f32,
) -> Panel {
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
        (layout::roster_button(), "ROSTER"),
        (layout::ledger_button(), "LEDGER"),
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
    let bare = !flow.feed_open
        && !flow.modes_open
        && !flow.tuner.open
        && !flow.roster_open
        && !flow.ledger_open;
    if bare && let Some(toast) = &flow.toast {
        panel.text(TextRun::new(
            layout::toast_at(),
            toast.text.clone(),
            theme::SMALL,
            theme::GOLD,
        ));
    }

    // --- party strip --------------------------------------------------------
    // One chip per person since wave 1.1, in two rows of five: a party is a
    // one-person band, and the chip is one of the four doors onto the one
    // selection. A lit chip *is* that selection, not a second one.
    //
    // **A drawer covers it**, and a strip drawn under one is a row of text
    // lying across a control somebody can click - which is what the floors
    // refuse, and what a person reading the drawer sees as a smear.
    if bare {
        panel.text(TextRun::new(
            layout::party_label(),
            "EVERYONE - click somebody, then a job on a site's board to ask them for it",
            theme::SMALL,
            theme::DIM,
        ));
        let names = lens.names();
        for (index, party) in lens.parties().iter().enumerate() {
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
                party.status(&names),
                theme::SMALL,
                match party.activity {
                    Activity::Idle => theme::DIM,
                    Activity::Working { .. } => theme::REGARD,
                    _ => theme::INK,
                },
            ));
        }
    }

    // --- the map's own words: markers' labels and quest counts -------------
    // **Under an open drawer the map says nothing**, exactly as the banner and
    // the toast do not: a drawer covers the screen, and a label nobody can
    // read under a scrim is still a row of text lying across a control
    // somebody can click. The markers and the figures stay - they are
    // pictures, and the scrim is what hides them.
    //
    // **An open job board is the same rule.** It is a panel over the map that
    // lies across markers and their labels at the reference camera, and the
    // words it hides are the words it carries in full: a site's name and the
    // count of what is open there. So while a board is up the map is a picture
    // and the board is the words (UI.md §3c).
    let worded = bare && flow.board.is_none();
    for (index, spec) in LOCATIONS.iter().enumerate() {
        let style = theme::text(theme::SMALL, theme::INK);
        if worded {
            let width = style.width_of(spec.name);
            let at = layout::marker_label(spec.tile, width);
            let mut label = TextRun::new(at, spec.name, theme::SMALL, theme::INK);
            label.layer = theme::layers::MAP_TEXT;
            panel.world_text(label);
        }
        if worded && index != TOWN {
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

    // --- the cast, one figure each, where they stand -----------------------
    // **One person, one figure, one place** (UI.md §3b): every character is
    // drawn exactly once, at `where_drawn` — their own doorstep while they are
    // home, their party's interpolated position while they are on the road,
    // never both. The token draw that used to live in `draw_map` is this loop:
    // it moved into the `Panel` so the floors can judge what is on the map at
    // all (UI.md §6, `floors::judge_figures`), and `ui::draw` culls a
    // world icon to the camera exactly as the token loop did.
    //
    // Names ride along because a face with no name is a token, and the whole
    // point of the people substrate is that these are people — but only at a
    // doorstep, where a name has a tent under it to belong to. A name pinned
    // to a moving figure is what the party strip's status line is for.
    for (index, person) in lens.people().iter().enumerate() {
        let Some(figure) = where_drawn(lens, index, now) else {
            continue;
        };
        let art = person.icon;
        let mut drawn = IconRun::new(figure.min, art, art.scale_across(layout::HOME));
        drawn.layer = theme::layers::TOKEN;
        panel.world_icon(drawn);
        if !worded || !lens.at_home(index) {
            continue;
        }
        let style = theme::text(theme::SMALL, theme::INK);
        let width = style.width_of(person.name);
        let mut label = TextRun::new(
            layout::figure_label(figure, width),
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
        // --- and the site panel a marker opens (UI.md §3c) -----------------
        if let Some(site) = flow.board {
            panel.absorb(crate::board::site_board(
                flow,
                lens,
                grid,
                tuning,
                clock.minutes,
                site,
            ));
        }
    }

    // --- drawers ------------------------------------------------------------
    if flow.feed_open {
        panel.absorb(panels::feed_drawer(flow, lens, tuning));
    }
    if flow.modes_open {
        panel.absorb(panels::modes_drawer(lens));
    }
    if flow.ledger_open {
        panel.absorb(crate::ledger::ledger_drawer(flow, lens));
    }
    if flow.roster_open {
        panel.absorb(panels::roster_drawer(flow, lens));
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
    let now = reading(ctx.world.resource::<Clock>(), &tuning, alpha);
    let flow = ctx.world.resource::<Flow>().clone();
    let sim = ctx.world.resource::<Sim>().clone();
    let lens = Lens::on(&sim);

    // **One selection, one ring** (UI.md §3b): the selected character, ringed
    // where they are drawn. And the pulse a click-to-focus left on the place it
    // jumped to. Both presentation: one is UI state, the other is a countdown
    // in wall ticks, and the simulation reads neither.
    if let Some(ring) = selection_ring(&flow, &lens, now) {
        ctx.rect(
            Rect {
                min: ring.min - Vec2::splat(RING),
                max: ring.max + Vec2::splat(RING),
            },
            theme::GOLD,
            Depth {
                layer: theme::layers::TOKEN,
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
}

/// The chrome's fills and buttons, through the UI mapping.
pub fn draw_chrome(ctx: &mut DrawCtx) {
    let map = UiMap::for_camera(ctx.world.resource::<Camera>());
    let flow = ctx.world.resource::<Flow>().clone();
    let clock = *ctx.world.resource::<Clock>();
    let active = *ctx.world.resource::<Tuning>();
    let sim_parties = Lens::on(ctx.world.resource::<Sim>()).parties().len();
    // The base screen, as `content` reckons it: no drawer over the map.
    let bare = !flow.feed_open
        && !flow.modes_open
        && !flow.tuner.open
        && !flow.roster_open
        && !flow.ledger_open;

    let fill = |ctx: &mut DrawCtx, rect: Rect, color: Color, layer: i16| {
        ui::fill(ctx, map.to_world_rect(rect), color, layer);
    };
    let border = |ctx: &mut DrawCtx, rect: Rect, color: Color, layer: i16| {
        ui::border(ctx, map.to_world_rect(rect), color, 2.0 * map.scale, layer);
    };

    // Top bar and party strip.
    fill(ctx, layout::topbar(), theme::BAR, theme::layers::PANEL);
    if bare {
        fill(
            ctx,
            layout::party_strip(),
            theme::STRIP,
            theme::layers::PANEL,
        );
    }
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
    // The job board: its ground, its rows, and its close. A row is a target,
    // and a target with no edge is a thing nobody knows they may tap.
    if let Some(site) = flow.board {
        let jobs = ctx
            .world
            .resource::<Sim>()
            .sites
            .get(site)
            .map_or(0, |board| board.quests.len());
        fill(
            ctx,
            layout::board_panel(),
            theme::PANEL,
            theme::layers::CARD,
        );
        border(ctx, layout::board_panel(), theme::GOLD, theme::layers::CARD);
        ghost_at(ctx, &map, layout::board_close(), theme::layers::CARD);
        for slot in 0..jobs.min(layout::BOARD_ROWS) {
            ghost_at(ctx, &map, layout::board_row(slot), theme::layers::CARD);
        }
    }
    if flow.selected.is_some() {
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
    // The strip's own chrome, only where the strip is drawn: a drawer hides
    // it (`content`), and a row of empty boxes under a drawer is a surface
    // saying nothing loudly.
    if bare {
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
    }

    // Drawers.
    if flow.feed_open || flow.modes_open || flow.roster_open || flow.ledger_open {
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
    if flow.roster_open {
        // Every row's own ground, and the chips over it: a chip is a target,
        // and a target with no edge is a thing nobody knows they may tap.
        for row in 0..layout::ROSTER_ROWS.min(ctx.world.resource::<Sim>().people.len()) {
            fill(
                ctx,
                layout::roster_open(row),
                theme::GHOST,
                theme::layers::OVERLAY,
            );
            for slot in 0..layout::SHEET_CHIPS {
                ghost(ctx, &map, layout::roster_chip(row, slot));
            }
        }
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
    // **The ledger's own targets**: a withdrawal per standing posting, and
    // the standing rates' steppers and postings — every one of them an edge,
    // because a target with no edge is a thing nobody knows they may tap.
    if flow.ledger_open {
        let standing: Vec<usize> = Lens::on(ctx.world.resource::<Sim>())
            .postings()
            .iter()
            .rev()
            .take(layout::LEDGER_ROWS)
            .enumerate()
            .filter(|(_, posting)| posting.status == crate::asks::Status::Open)
            .map(|(row, _)| row)
            .collect();
        for row in standing {
            ghost(ctx, &map, layout::ledger_withdraw(row));
        }
        for row in 0..crate::traits::TaskType::ALL.len() {
            ghost(ctx, &map, layout::rates_down(row));
            ghost(ctx, &map, layout::rates_up(row));
            ghost(ctx, &map, layout::rates_post(row));
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
    // The character panel's own trait chips, when one is open over the map.
    if flow.selected.is_some() && !flow.tuner.open {
        for slot in 0..layout::SHEET_CHIPS {
            ghost_at(ctx, &map, layout::sheet_chip(slot), theme::layers::CARD);
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
    ghost_at(ctx, map, rect, theme::layers::OVERLAY + 1);
}

/// The same, on a stated band — a control that sits on the base screen rather
/// than inside a drawer has to be under the panel's own text, not over it.
fn ghost_at(ctx: &mut DrawCtx, map: &UiMap, rect: Rect, layer: i16) {
    ui::fill(ctx, map.to_world_rect(rect), theme::GHOST, layer);
    ui::border(
        ctx,
        map.to_world_rect(rect),
        theme::BORDER,
        2.0 * map.scale,
        layer,
    );
}

/// Every string and icon of the screen, drawn through the mapping.
pub fn draw_content(ctx: &mut DrawCtx) {
    let map = UiMap::for_camera(ctx.world.resource::<Camera>());
    let flow = ctx.world.resource::<Flow>().clone();
    let clock = *ctx.world.resource::<Clock>();
    let tuning = *ctx.world.resource::<Tuning>();
    let sim = ctx.world.resource::<Sim>().clone();
    let grid = ctx.world.resource::<Grid>().clone();
    let now = reading(&clock, &tuning, ctx.world.resource::<Time>().alpha);
    let panel = content(&flow, &Lens::on(&sim), &grid, &clock, &tuning, now);
    ui::draw(ctx, &panel, &map);
}
