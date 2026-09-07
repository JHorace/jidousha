//! `--verify`: the world, moved by a conductor, asserted on, and
//! photographed.
//!
//! The speed-invariance sweep (`sweep.rs`) is the heart: one order script
//! under three speed schedules, transcripts identical to the world-minute.
//! Around it: the pathfinding contracts (the documented tie-break, the road
//! that beats the shorter overland line, the unreachable case), the
//! seed-independence probe (no `Rng` read exists in S1), the one-grid
//! two-readers assertion (every drawn tile against the sim's own grid), the
//! token-position judge (the between-tile interpolation derived, never
//! written back), the attention batteries (the class table, the feed as a
//! view of the transcript, the meters, and `pauses.rs`'s auto-pause claims),
//! the floors, the drawer session, the mutation round, and the captures a
//! person looks at.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::FrameRecord;

use crate::checks::{Checks, greater};
use crate::constants::Tuning;
use crate::grid::{LOCATIONS, Terrain, Tile};
use crate::path::route;
use crate::sweep::{Act, Conducted, Directive, Photo, Session, When, conduct, transcript};
use crate::{
    camera, capture, floors, grid, layout, lens, library, links, modules, mutation, people,
    restart, screens, shots, stores, sweep, theme, traits,
};

/// The surface the reference run draws at: the 960x540 chrome design doubled,
/// which is the window the game opens.
pub const HEADLESS_VIEWPORT: PhysicalSize = crate::WINDOW;

/// The narrow surface the second capture set uses — narrow rather than short
/// on purpose: horizontal shrink is the axis scaling defects live on.
pub const NARROW_VIEWPORT: PhysicalSize = PhysicalSize::new(600, 540);

/// The photographed session: the all-1x script, played the way wave 0a's
/// question is asked — the player tells the world that a completed quest is
/// worth stopping for, and then gets stopped four times.
///
/// **It is the same orders as the sweep's baseline**, at the same world-
/// minutes, so its transcript must come out identical: the auto-pauses stretch
/// wall time and move no world-time address. `run` asserts exactly that, which
/// makes this session the auto-pause half of the invariance claim as well as
/// the source of every screenshot.
pub fn photographed(viewport: PhysicalSize) -> Conducted {
    // The class the config panel is set to stop on, and where its radio is.
    let pause_row = crate::attention::EventClass::QuestComplete.index();
    let pause_slot = crate::attention::Mode::ALL
        .iter()
        .position(|mode| *mode == crate::attention::Mode::PauseAndFocus)
        .unwrap_or(0);
    let click_ui = |when: When, at: Vec2| Directive {
        when,
        what: Act::ClickUi(at),
    };
    let mut script: Vec<Directive> = vec![
        // The config, before the clock starts: a recorded input that changes
        // what the world will do.
        click_ui(When::Tick(12), layout::modes_button().center()),
        click_ui(
            When::Tick(16),
            layout::modes_radio(pause_row, pause_slot).center(),
        ),
        click_ui(When::Tick(24), layout::modes_button().center()),
        Directive {
            when: When::Tick(28),
            what: Act::Tap(Key::Digit1),
        },
    ];
    // Three orders, each given the way a player gives one: pick somebody, open
    // the site's board, tap a job. The order puts the board down with the
    // selection, so the map is clear again between them.
    script.extend(sweep::post(sweep::ORDER_MINUTES[0], 0, 0, 2));
    script.extend(sweep::post(sweep::ORDER_MINUTES[1], 1, 1, 0));
    script.extend(sweep::post(sweep::ORDER_MINUTES[2], 2, 2, 5));
    // **The board re-opened on the site just ordered to**: the `ordered`
    // picture, a row that now reads as the person the player sent.
    script.push(Directive {
        when: When::Minute(56),
        what: Act::ClickWorld(
            layout::marker_rect(LOCATIONS[crate::sim::site_location(2)].tile).center(),
        ),
    });
    // Open the feed and leave it open across the first completions, so the
    // photograph at the first one catches the world stopped with its reason
    // on screen. Every resume is the player pressing 1, which is what a
    // player does; the world-times on the far side are unchanged.
    script.push(click_ui(When::Minute(100), layout::feed_button().center()));
    script.push(click_ui(When::Minute(300), layout::feed_button().center()));
    script.extend(sweep::post(sweep::ORDER_MINUTES[3], 0, 3, 0));
    // The roster, with a chip's explanation open on it: a trait chip tapped on
    // one surface reads the same line it reads on the other.
    script.push(click_ui(
        When::Minute(440),
        layout::roster_button().center(),
    ));
    let ludo = people::roster()
        .iter()
        .position(|person| person.id == "ludo")
        .unwrap_or(0);
    script.push(click_ui(
        When::Minute(452),
        layout::roster_chip(ludo, 2).center(),
    ));
    // **The job board, the wave's own pictures.** Shut the roster, pick Alex —
    // the band's scout, home since minute 212 and idle, so the board answers
    // him with a travel line from his own door and with a fit column that
    // separates the one open scout job from everything else — and open the
    // Deep Cave, whose six rows carry three task types between them and
    // several rows other people already hold.
    let alex = people::roster()
        .iter()
        .position(|person| person.id == "alex")
        .unwrap_or(0);
    script.push(click_ui(
        When::Minute(470),
        layout::roster_button().center(),
    ));
    script.push(click_ui(
        When::Minute(480),
        layout::party_chip(alex).center(),
    ));
    script.push(Directive {
        when: When::Minute(490),
        what: Act::ClickWorld(
            layout::marker_rect(LOCATIONS[crate::sim::site_location(1)].tile).center(),
        ),
    });
    // And the bounce: the Deep Cave's front row is the mushroom haul, which
    // Steve was ordered to at minute 32 and which is therefore never open
    // again — so tapping it is the claimed-row refusal, deterministically.
    script.push(click_ui(When::Minute(510), layout::board_row(0).center()));
    // Shut the board, then somebody to look at: Steve is home from the Deep
    // Cave's second run at minute 512 and rests six world-hours, so his
    // doorstep is a figure to click from there on.
    script.push(click_ui(When::Minute(530), layout::board_close().center()));
    let steve = people::roster()
        .iter()
        .position(|person| person.id == "steve")
        .unwrap_or(0);
    let doorstep = people::roster()[steve].home.center();
    script.push(Directive {
        when: When::Minute(540),
        what: Act::ClickWorld(doorstep),
    });
    // **A trait chip, tapped on the sheet** — the clarity slice, photographed.
    // Steve's third chip is `caring`, a motivator, which is what the wave
    // asks the picture to show.
    script.push(click_ui(When::Minute(550), layout::sheet_chip(2).center()));
    let photos = [
        // The settlement before anything is dispatched: the whole cast
        // standing at their homes, named. Wave 0b's own exit picture - the
        // moment there are people in this world rather than tokens.
        Photo {
            name: "settlement",
            minute: 0,
            tick: 10,
            paused: false,
        },
        // The config panel, with the class the session will be stopped by
        // set to pause.
        Photo {
            name: "modes",
            minute: 0,
            tick: 20,
            paused: false,
        },
        Photo {
            name: "map",
            minute: 44,
            tick: 0,
            paused: false,
        },
        // The feed, at the world-minute the world stopped itself.
        Photo {
            name: "feed",
            minute: sweep::COMPLETIONS[0],
            tick: 0,
            paused: true,
        },
        // **The world living on its own**: the map at a minute when nobody
        // was told to go anywhere and people are on the road anyway.
        Photo {
            name: "living",
            minute: 400,
            tick: 0,
            paused: false,
        },
        // A character looked at, with a chip's explanation open on their
        // sheet and the selection ring on their figure.
        Photo {
            name: "person",
            minute: 590,
            tick: 0,
            paused: false,
        },
        // The roster: everyone, and what they are doing about it.
        Photo {
            name: "roster",
            minute: 462,
            tick: 0,
            paused: false,
        },
        // **The job board**, open on a mixed-type site with the fit column
        // and the travel line up for the selected character: the surface the
        // marker's bare count was standing in for.
        Photo {
            name: "board",
            minute: 500,
            tick: 0,
            paused: false,
        },
        // The same board a few minutes after a row was ordered from: the row
        // now reads as somebody's, and that somebody is on the road.
        Photo {
            name: "ordered",
            minute: 64,
            tick: 0,
            paused: false,
        },
        // And the bounce: a row somebody already has, tapped, with the toast
        // under the bar saying so.
        Photo {
            name: "bounce",
            minute: 520,
            tick: 0,
            paused: false,
        },
    ];
    conduct(&Session {
        tuning: Tuning::SHIPPED,
        modules: crate::modules::ModuleSet::ALL,
        seed: None,
        directives: &script,
        photos: &photos,
        probe_ticks: &[],
        viewport,
        max_ticks: 60_000,
        stop_at_rest: false,
        stop_at_minute: Some(sweep::RUN_UNTIL),
        // The photographed session is played under a config that stops the
        // world at every completion; the player presses 1 again each time.
        resume_after: Some((Key::Digit1, 20)),
    })
}

/// **Pathfinding, deterministic by construction** (DESIGN §3): micro-grid
/// contracts for the documented rule, and the authored map's own routes.
pub fn path_contracts(checks: &mut Checks) {
    path_contracts_at(checks, Tuning::SHIPPED);
}

/// The same battery under a stated set. **Every expectation is a shipped
/// literal**, never derived from `tuning` — a check that recomputes its
/// expectation from the constant under test cannot see it move, and the
/// mutation round runs this battery to see exactly that.
pub fn path_contracts_at(checks: &mut Checks, tuning: Tuning) {
    // --- the deliberate tie (the documented tie-break, asserted) -----------
    // A 3x3 of plains: (0,0) to (1,1) has two equal-cost routes. The rule —
    // pop lowest cost then lowest row-major, expand N,E,S,W, replace only on
    // strictly cheaper — settles (1,0) before (0,1), and (0,1)'s equal-cost
    // arrival at (1,1) may not displace it: the route goes east first.
    let tie = parse_grid(checks, "...\n...\n...");
    if let Some(tie) = &tie {
        let found = route(tie, &tuning, Tile::new(0, 0), Tile::new(1, 1));
        let expected = vec![Tile::new(1, 0), Tile::new(1, 1)];
        checks.require(
            found.as_ref().is_some_and(|route| route.tiles == expected),
            "the deliberate tie did not resolve by the documented tie-break",
            format!(
                "on a uniform 3x3 from (0,0) to (1,1) the route is {:?}; the documented rule \
                 (lowest cost, then lowest row-major; N,E,S,W; strictly-cheaper replacement) \
                 says {expected:?}",
                found.map(|route| route.tiles)
            ),
        );
    }

    // --- the road beats the overland shortcut, in miniature ----------------
    // ===   from (0,1) to (2,1): straight through the rough is 2 tiles for
    // .r.   14 minutes; up onto the road and back down is 4 tiles for 10.
    let mini = parse_grid(checks, "===\n.r.");
    if let Some(mini) = &mini {
        let found = route(mini, &tuning, Tile::new(0, 1), Tile::new(2, 1));
        let expected = vec![
            Tile::new(0, 0),
            Tile::new(1, 0),
            Tile::new(2, 0),
            Tile::new(2, 1),
        ];
        checks.require(
            found
                .as_ref()
                .is_some_and(|route| route.tiles == expected && route.cost == 10),
            "the longer road did not beat the shorter overland line",
            format!(
                "the route is {:?}; four road-and-plains tiles cost 10 and two tiles through \
                 the rough cost 14, so the router must take the detour",
                found.map(|route| (route.tiles, route.cost))
            ),
        );
    }

    // --- unreachable says so -----------------------------------------------
    let cut = parse_grid(checks, ".~.\n.~.");
    if let Some(cut) = &cut {
        let found = route(cut, &tuning, Tile::new(0, 0), Tile::new(2, 0));
        checks.require(
            found.is_none(),
            "an unreachable goal produced a route",
            format!("across an unbroken water column the route is {found:?}"),
        );
    }

    // --- the authored map's own routes, the sums the sweep's minutes rest on
    let world = grid::grid();
    let town = LOCATIONS[0].tile;

    // Kawaza -> Watchtower: all road, 47 tiles, 94 minutes — longer in tiles
    // than the 39-tile overland line, and cheaper (DESIGN §3's visible
    // routing).
    let tower = route(&world, &tuning, town, LOCATIONS[1].tile);
    match &tower {
        Some(tower) => {
            let all_road = tower
                .tiles
                .iter()
                .all(|tile| world.get(*tile) == Terrain::Road);
            let manhattan =
                (LOCATIONS[1].tile.x - town.x).abs() + (LOCATIONS[1].tile.y - town.y).abs();
            checks.require(
                all_road && tower.tiles.len() == 47 && tower.cost == 94,
                "the Watchtower road route is not the authored one",
                format!(
                    "the route is {} tiles at {} minutes, all-road {all_road}; the authored \
                     road is 47 road tiles at 2 minutes each",
                    tower.tiles.len(),
                    tower.cost
                ),
            );
            checks.require(
                i32::try_from(tower.tiles.len()).unwrap_or(0) > manhattan,
                "the road route is not longer in tiles than the overland line",
                format!(
                    "the route is {} tiles and the straight line is {manhattan}; the point of \
                     the authored terrain is a longer road that wins on cost",
                    tower.tiles.len()
                ),
            );
        }
        None => checks.require(
            false,
            "the Watchtower is unreachable from the town",
            "grid::MAP no longer connects them by any passable route".to_owned(),
        ),
    }

    // Kawaza -> Deep Cave: 5 road tiles east along the spine, then 7 forest
    // north up the x=12 column — 59 minutes, the cheap way into the slog.
    let cave = route(&world, &tuning, town, LOCATIONS[2].tile);
    let kinds = |route: &crate::path::Route| {
        let mut road = 0;
        let mut plains = 0;
        let mut forest = 0;
        for tile in &route.tiles {
            match world.get(*tile) {
                Terrain::Road => road += 1,
                Terrain::Plains => plains += 1,
                Terrain::Forest => forest += 1,
                _ => {}
            }
        }
        (road, plains, forest)
    };
    checks.require(
        cave.as_ref()
            .is_some_and(|route| kinds(route) == (5, 0, 7) && route.cost == 59),
        "the Deep Cave route is not the authored slog",
        format!(
            "the route is {:?} as (road, plains, forest tiles, cost); the authored line is \
             5 road and 7 forest, 59 minutes",
            cave.map(|route| (kinds(&route), route.cost))
        ),
    );

    // Kawaza -> Black Vault: around the peak ridge's east end — 48 road
    // tiles, 96 minutes, through the wrap column at x=44.
    let vault = route(&world, &tuning, town, LOCATIONS[4].tile);
    checks.require(
        vault.as_ref().is_some_and(|route| {
            route.cost == 96 && route.tiles.len() == 48 && route.tiles.contains(&Tile::new(44, 17))
        }),
        "the Black Vault detour is not the authored wrap around the ridge",
        format!(
            "the route is {:?}; the ridge forces 48 road tiles at 96 minutes through (44,17)",
            vault.map(|route| (route.tiles.len(), route.cost))
        ),
    );

    // A rough crossing, so the rough cost has a literal to break: south into
    // the rough country.
    let rough = route(&world, &tuning, town, Tile::new(7, 20));
    checks.require(
        rough.as_ref().is_some_and(|route| route.cost == 54),
        "the rough crossing does not cost what the shipped set says",
        format!(
            "town to (7,20) is {:?}; one plains (4) and five rough (10 each) is 54",
            rough.map(|route| route.cost),
        ),
    );

    // Terrain data hygiene: every kind's glyph round-trips, and the two
    // impassable kinds answer no cost — the passable flag and the cost table
    // cannot disagree because they are one table.
    for kind in Terrain::ALL.iter().copied() {
        checks.require(
            Terrain::from_glyph(kind.glyph()) == Some(kind),
            "a terrain glyph does not round-trip",
            format!("{kind:?} writes {:?}", kind.glyph()),
        );
        checks.require(
            kind.passable() == kind.cost(&tuning).is_some(),
            "a terrain's passable flag disagrees with its cost",
            format!(
                "{kind:?} is passable={} and cost={:?}",
                kind.passable(),
                kind.cost(&tuning)
            ),
        );
    }
}

/// A micro-grid for the contracts, or a recorded failure.
fn parse_grid(checks: &mut Checks, text: &str) -> Option<grid::Grid> {
    match grid::Grid::parse(text) {
        Ok(grid) => Some(grid),
        Err(why) => {
            checks.require(
                false,
                "a contract's micro-grid does not parse",
                format!("{text:?}: {why}"),
            );
            None
        }
    }
}

/// **No `Rng` read exists in S1**: the whole event transcript is identical
/// under far-apart seeds — the plumbing and the stamps remain, and the dice
/// decide nothing.
fn seed_independence(checks: &mut Checks) {
    let script = sweep::speed_scripts().remove(1).1; // all-4x: the fast one
    let mut first_session = Session::plain(Tuning::SHIPPED, &script, 60_000);
    first_session.seed = Some(7);
    let first = conduct(&first_session);
    let mut second_session = Session::plain(Tuning::SHIPPED, &script, 60_000);
    second_session.seed = Some(7_777_777);
    let second = conduct(&second_session);
    checks.require(
        transcript(&first.events) == transcript(&second.events),
        "the seed reached the simulation",
        format!(
            "at seed 7 the transcript is {:?} and at seed 7777777 it is {:?}; S1 has no \
             randomness and no Rng read may exist",
            transcript(&first.events),
            transcript(&second.events)
        ),
    );
}

/// **One grid, two readers**: every terrain tile on the frame carries exactly
/// the colour of the kind the sim's grid holds there — the map cannot lie
/// (DESIGN §3).
pub fn judge_terrain(checks: &mut Checks, frame: &FrameRecord, viewport: PhysicalSize) {
    let world = grid::grid();
    let camera = run_camera(viewport);
    let view = camera.visible_bounds();
    let (min, max) = screens::visible_tiles(&world, view);
    // One walk over the quads: collect the tint of every tile-shaped,
    // tile-aligned quad. The depth sort puts the terrain band first, so the
    // first claim on a tile is the terrain's.
    let mut seen: Vec<(Tile, Color)> = Vec::new();
    for quad in frame.quads() {
        let bounds = quad.bounds();
        let size = bounds.size();
        if !crate::checks::near(size.x, grid::TILE) || !crate::checks::near(size.y, grid::TILE) {
            continue;
        }
        let x = bounds.min.x / grid::TILE;
        let y = bounds.min.y / grid::TILE;
        if !crate::checks::near(x, x.round()) || !crate::checks::near(y, y.round()) {
            continue;
        }
        let tile = Tile::new(x.round() as i32, y.round() as i32);
        if world.contains(tile) && !seen.iter().any(|(seen, _)| *seen == tile) {
            seen.push((tile, quad.tint));
        }
    }
    let mut wrong = 0usize;
    let mut missing = 0usize;
    let mut example = String::new();
    for y in min.y..=max.y {
        for x in min.x..=max.x {
            let tile = Tile::new(x, y);
            let wanted = world.get(tile).color();
            match seen.iter().find(|(seen, _)| *seen == tile) {
                Some((_, tint)) if *tint == wanted => {}
                Some((_, tint)) => {
                    wrong += 1;
                    if example.is_empty() {
                        example = format!(
                            "({x},{y}) is {:?} in the grid and drawn {tint:?}",
                            world.get(tile)
                        );
                    }
                }
                None => {
                    missing += 1;
                    if example.is_empty() {
                        example = format!("({x},{y}) has no tile quad at all");
                    }
                }
            }
        }
    }
    checks.require(
        wrong == 0 && missing == 0,
        "the drawn map disagrees with the grid the sim consults",
        format!(
            "{wrong} tiles drawn the wrong kind and {missing} not drawn, of {} visible; first: \
             {example} - one grid, two readers (DESIGN §3)",
            (max.x - min.x + 1) * (max.y - min.y + 1)
        ),
    );
}

/// The camera a conducted run holds at a viewport (no pan input in any
/// scripted session, so this is `camera_for`, already legal for `fit`).
pub fn run_camera(viewport: PhysicalSize) -> Camera {
    camera::camera_for(viewport)
}

/// **A tap is a click, and the engine did that** (`jidousha-api.md`: the
/// first finger down is mirrored onto the primary pointer).
///
/// Verified rather than built: this game has no touch code at all, and the
/// point of the check is that it does not need any — a finger landing on a
/// character's figure selects them, through the same hit-test a mouse uses,
/// with no `PointerMoved` and no `ButtonPressed` in the snapshot.
fn touch_selects(checks: &mut Checks) {
    let cast = people::roster();
    let Some(steve) = cast.iter().position(|person| person.id == "steve") else {
        checks.require(
            false,
            "the touch probe names somebody who is not in the roster",
            "people::roster has no \"steve\"".to_owned(),
        );
        return;
    };
    let script = [Directive {
        when: When::Tick(5),
        what: Act::TouchWorld(cast[steve].home.center()),
    }];
    let mut session = Session::plain(Tuning::SHIPPED, &script, 12);
    session.stop_at_rest = false;
    session.probe_ticks = &[10];
    let conducted = conduct(&session);
    let selected = conducted.probe(10).and_then(|(_, flow, ..)| flow.selected);
    checks.require(
        selected == Some(steve),
        "a finger on a character's figure did not select them",
        format!(
            "after a touch at {:?}'s doorstep the panel is open on {:?}; the engine mirrors \
             the first finger onto the primary pointer, so a tap is a click and this game \
             writes no touch code",
            cast[steve].name,
            selected.map(|who| cast[who].name)
        ),
    );
}

/// **The culling is honest, both ways**: zoomed in, far tiles are not
/// submitted; and nothing is submitted wildly outside the view. Staged
/// directly — a scripted run never zooms.
fn culling_probe(checks: &mut Checks) {
    let mut sim = headless(crate::config(), crate::register);
    sim.world_mut().insert_resource(Tuning::SHIPPED);
    sim.world_mut()
        .insert_resource(camera::Surface(HEADLESS_VIEWPORT));
    sim.tick();
    // Zoom hard onto the town, then let `fit` clamp it legal on the next
    // tick, exactly as a played zoom would.
    {
        let world = sim.world_mut();
        let camera = world.resource_mut::<Camera>();
        camera.height = camera::MIN_H;
        camera.center = LOCATIONS[0].tile.center();
    }
    sim.tick();
    let camera = *sim.world().resource::<Camera>();
    let mut recorder = jidousha::testing::FrameRecorder::new(HEADLESS_VIEWPORT);
    {
        let assets = sim.world_mut().resource_mut::<Assets>();
        let _ = crate::sprites::settle(assets);
    }
    recorder.settle_assets(&mut sim, 2);
    let frame = recorder.draw(&mut sim);
    let view = camera.visible_bounds();
    let world = grid::grid();
    let total = world.width * world.height;
    let tiles_drawn = frame
        .quads()
        .iter()
        .filter(|quad| {
            let size = quad.bounds().size();
            crate::checks::near(size.x, grid::TILE) && crate::checks::near(size.y, grid::TILE)
        })
        .count();
    checks.require(
        i32::try_from(tiles_drawn).unwrap_or(i32::MAX) < total,
        "zoomed in, the whole map is still submitted - the culling is not culling",
        format!("{tiles_drawn} tile quads were submitted of {total} tiles, at a view of {view:?}"),
    );
    // Nothing wildly off-screen: a run is culled per run, not per glyph, so
    // the margin is one label's width.
    let margin = 220.0;
    let expanded = Rect {
        min: view.min - Vec2::splat(margin),
        max: view.max + Vec2::splat(margin),
    };
    for quad in frame.quads() {
        let bounds = quad.bounds();
        checks.require(
            bounds.overlaps(expanded),
            "something was submitted far outside the camera's view",
            format!("a quad at {bounds:?} against a view of {view:?}"),
        );
        if !bounds.overlaps(expanded) {
            break;
        }
    }
    checks.require(
        camera.height > camera::MIN_H - 0.01 && camera.height < camera::MIN_H + 0.01,
        "the zoom clamp did not hold the staged zoom at the floor",
        format!(
            "the camera height is {} and the floor is {}",
            camera.height,
            camera::MIN_H
        ),
    );
}

/// **The module-off matrix** (GDD §9): the suite, once per module, with that
/// module switched off. Green is the definition of modular.
///
/// The registry is empty in wave 0b, so the matrix is one pass — the
/// everything-on baseline — and it runs it. That is deliberately not a skip:
/// the harness plants a `ModuleSet` before `Startup`, conducts a real run
/// under it, and asserts the world moved and came to rest, so wave 1.1's
/// autonomy module lands into machinery that already works rather than into a
/// second thing to get right in the same session.
///
/// **The baseline pass asserts the authored timeline; a module-off pass does
/// not.** A module being off is *supposed* to change what happens — that is
/// what a module is. What every pass owes is a world that runs: no panic, the
/// script consumed, everything home at the end, and the shared state's own
/// arithmetic intact under whatever is loaded.
fn module_matrix(checks: &mut Checks) -> String {
    let mut notes = Vec::new();
    for (what, set) in modules::matrix() {
        let script = sweep::speed_scripts().remove(1).1; // all-4x: the fast one
        let mut session = Session::plain(Tuning::SHIPPED, &script, 25_000);
        session.modules = set;
        let conducted = conduct(&session);
        // **What every pass owes is a world that runs**: no panic, the script
        // consumed, the window run out, and the shared state's arithmetic
        // intact. "At rest" is only meaningful with the scorer switched off —
        // a world where people decide things is never finished deciding, and
        // that is the module's whole point.
        checks.require(
            conducted.minutes >= sweep::WINDOW,
            "a module-off pass did not run out its world-time window",
            format!(
                "with {what} ({}) the clock ended at minute {} after {} ticks",
                set.stamp(),
                conducted.minutes,
                conducted.ticks
            ),
        );
        if !set.enabled(crate::autonomy::MODULE) {
            checks.require(
                conducted.sim.at_rest(),
                "with autonomy off the world did not come to rest",
                format!(
                    "with {what} ({}) somebody is still abroad after {} ticks; the row's \
                     degrades-to sentence says everybody idles at their own door until the \
                     player dispatches them",
                    set.stamp(),
                    conducted.ticks
                ),
            );
            checks.require(
                !conducted
                    .events
                    .iter()
                    .any(|event| event.class == crate::attention::EventClass::ActionStarted),
                "with autonomy off somebody decided something anyway",
                format!(
                    "the transcript holds {} action-started events, and the module that emits \
                     them is switched off",
                    conducted
                        .events
                        .iter()
                        .filter(|event| event.class == crate::attention::EventClass::ActionStarted)
                        .count()
                ),
            );
        }
        checks.require(
            !conducted.events.is_empty(),
            "a module-off pass produced a world where nothing happened",
            format!(
                "with {what} ({}) the transcript is empty; green means the world still \
                 runs, not that it stopped having anything to run",
                set.stamp()
            ),
        );
        if set == modules::ModuleSet::ALL {
            sweep::judge_orders(checks, &conducted, "the everything-on matrix pass");
        }
        traits::arithmetic(checks, &Tuning::SHIPPED);
        stores::judge_at(checks, &Tuning::SHIPPED);
        notes.push(format!("{what} ({} events)", conducted.events.len()));
    }
    format!(
        "{} pass(es) over {} module(s): {}",
        modules::matrix().len(),
        modules::MODULES.len(),
        notes.join(", ")
    )
}

/// **Regard drift is on the one scheduler** (GDD §4.2), so its cadence is a
/// world-time fact and not a tick fact — which means it is speed-invariant
/// like everything else, and a conducted run's drift count is a literal.
fn drift_cadence(checks: &mut Checks) {
    let mut counts = Vec::new();
    for (name, script) in sweep::speed_scripts() {
        let conducted = conduct(&Session::plain(Tuning::SHIPPED, &script, 60_000));
        counts.push((name, conducted.sim.shared.drifts(), conducted.minutes));
    }
    // The scenario runs to minute 800 and the shipped cadence is every four
    // world-hours, so drift falls due at 240, 480 and 720. A shipped literal,
    // so `drift_hours` moving breaks it (the mutation round leans on that).
    for (name, drifts, minutes) in &counts {
        checks.require(
            *drifts == 3,
            "regard did not drift on the schedule the shipped cadence says",
            format!(
                "under {name} the world ran {minutes} world-minutes and drift ran {drifts} \
                 times; every four hours over 800 minutes is 3 (at 240, 480 and 720)"
            ),
        );
    }
    let first = counts.first().map(|(_, drifts, _)| *drifts);
    checks.require(
        counts.iter().all(|(_, drifts, _)| Some(*drifts) == first),
        "how often regard drifts depends on the speed the player watches at",
        format!("the drift counts under the three speed scripts are {counts:?}"),
    );
}

/// Every selection ring on a frame: a gold, square, marker-sized fill.
///
/// **The count is the whole point.** Wave 1.1's double-selection bug drew two
/// of these — one from the dispatch pick over parties, one from the character
/// selection over people — so the instrument that catches it has to count
/// rings on the photographed frame rather than read a field. The size band is
/// wide enough to catch both of the old rings (38 units and 36) and no chrome:
/// nothing else this game draws is a gold square anywhere near marker size.
fn rings(frame: &FrameRecord) -> Vec<Rect> {
    frame
        .quads()
        .iter()
        .filter(|quad| quad.tint == theme::GOLD)
        .map(|quad| quad.bounds())
        .filter(|bounds| {
            let size = bounds.size();
            crate::checks::near(size.x, size.y) && size.x > 33.0 && size.x < 48.0
        })
        .collect()
}

/// What one selection script left behind: the selection, the rings drawn, and
/// the name the character panel put in its name row.
struct Selected {
    /// `Flow::selected` at the probe.
    who: Option<usize>,
    /// Every ring on the photographed frame.
    rings: Vec<Rect>,
    /// The panel's name row, if the panel drew one.
    named: Option<String>,
}

/// Run a script over the paused opening world and read the selection off it.
///
/// The scenario opens paused, so nobody moves and every figure stays on its
/// own doorstep: what changes between these runs is the clicking, which is
/// exactly what a selection test wants to vary and nothing else.
fn selection_run(script: &[Directive]) -> (Conducted, Selected) {
    let photos = [Photo {
        name: "selection",
        minute: 0,
        tick: 24,
        paused: false,
    }];
    let mut session = Session::plain(Tuning::SHIPPED, script, 26);
    session.probe_ticks = &[24];
    session.photos = &photos;
    let conducted = conduct(&session);
    let who = conducted.probe(24).and_then(|(_, flow, ..)| flow.selected);
    let (rings, named) = conducted.photo("selection").map_or_else(
        || (Vec::new(), None),
        |shot| {
            let panel = screens::content(
                &shot.flow,
                &lens::Lens::on(&shot.sim),
                &grid::grid(),
                &shot.clock,
                &Tuning::SHIPPED,
            );
            let name_at = layout::person_panel().min + layout::sheet::NAME;
            let named = panel
                .runs
                .iter()
                .find(|row| {
                    crate::checks::near(row.at.x, name_at.x)
                        && crate::checks::near(row.at.y, name_at.y)
                })
                .map(|row| row.text.clone());
            (rings(&shot.frame), named)
        },
    );
    (conducted, Selected { who, rings, named })
}

/// **One selection** (UI.md §3b): the wave-1.1 double-selection bug, as the
/// reproduction that failed before the fix and the rule that replaced it.
///
/// The bug: the S1 dispatch pick (an index over parties) and the wave-0a
/// character selection (an index over people) survived wave 1.1 as two
/// independent fields over what had become one roster, each drawing its own
/// gold ring — chip-select somebody, sprite-select somebody else, and two
/// people are lit (`FINDINGS.md` G-017, the owner's playtest). What this
/// battery asserts is the rule that replaced them: one field, written by every
/// select path, read by everything that highlights.
///
/// Every expectation here is a shipped literal — **one** ring, and the box it
/// is in — rather than arithmetic over the field under test, so a second
/// selection reappearing is a failure this run can see.
fn one_selection(checks: &mut Checks) -> Option<Conducted> {
    let cast = people::roster();
    let tuning = Tuning::SHIPPED;
    // The index identity the one selection stands on: a party is a one-person
    // band in registry order, so party `i` *is* person `i`. Asserted rather
    // than assumed, because the unified index is only meaningful while it
    // holds.
    let opening = crate::sim::Sim::opening(&tuning, modules::ModuleSet::ALL);
    let paired = opening.parties.len() == opening.people.len()
        && opening
            .parties
            .iter()
            .enumerate()
            .all(|(index, party)| party.member == index);
    checks.require(
        paired,
        "a party is no longer the character at the same index",
        format!(
            "{} parties over {} people, and the members read {:?}; the one selection is one \
             index over both lists",
            opening.parties.len(),
            opening.people.len(),
            opening
                .parties
                .iter()
                .map(|party| party.member)
                .collect::<Vec<_>>()
        ),
    );

    // --- the reproduction: chip-select one, sprite-select another ----------
    let (first, second) = (0usize, 3usize);
    let script = [
        Directive {
            when: When::Tick(6),
            what: Act::ClickUi(layout::party_chip(first).center()),
        },
        Directive {
            when: When::Tick(14),
            what: Act::ClickWorld(cast[second].home.center()),
        },
    ];
    let (reproduction, after) = selection_run(&script);
    checks.require(
        after.who == Some(second),
        "a sprite-select did not move the selection off the chip-selected character",
        format!(
            "the chip picked {} and the sprite picked {}, and the selection reads {:?}",
            cast[first].name,
            cast[second].name,
            after.who.map(|who| cast[who].name)
        ),
    );
    let wanted = Rect {
        min: layout::home_rect(cast[second].home).min - Vec2::splat(screens::RING),
        max: layout::home_rect(cast[second].home).max + Vec2::splat(screens::RING),
    };
    checks.require(
        after.rings.len() == 1,
        "the map drew more than one selection ring",
        format!(
            "{} rings landed on the frame at {:?} after chip-selecting {} and then \
             sprite-selecting {}; there is one selection and it draws one ring",
            after.rings.len(),
            after.rings,
            cast[first].name,
            cast[second].name
        ),
    );
    checks.require(
        after.rings.first().is_some_and(|ring| {
            crate::checks::near(ring.min.x, wanted.min.x)
                && crate::checks::near(ring.min.y, wanted.min.y)
        }),
        "the one selection ring is not on the selected character",
        format!(
            "the ring is at {:?} and {}'s doorstep wants it at {wanted:?}",
            after.rings.first(),
            cast[second].name
        ),
    );
    checks.require(
        after.named.as_deref() == Some(cast[second].name),
        "the character panel is keyed to somebody other than the selection",
        format!(
            "the selection is {:?} and the panel's name row says {:?}",
            after.who.map(|who| cast[who].name),
            after.named
        ),
    );

    // --- the same act from every surface ------------------------------------
    // Four surfaces show a person; selecting on any of them is one act with
    // one outcome, which is the whole of the unified rule.
    let idle_face = {
        let lens = lens::Lens::on(&opening);
        crate::meters::faces(&lens, 0).first().map(|(who, _)| *who)
    };
    let Some(idle_face) = idle_face else {
        checks.require(
            false,
            "the idle meter counts nobody in the opening world",
            "every character is idle at minute zero, so its faces list cannot be empty".to_owned(),
        );
        return Some(reproduction);
    };
    let surfaces: [(&str, usize, Vec<Directive>); 4] = [
        (
            "the party strip's chip",
            1,
            vec![Directive {
                when: When::Tick(6),
                what: Act::ClickUi(layout::party_chip(1).center()),
            }],
        ),
        (
            "the map sprite",
            4,
            vec![Directive {
                when: When::Tick(6),
                what: Act::ClickWorld(cast[4].home.center()),
            }],
        ),
        (
            "the roster row's name",
            7,
            vec![
                Directive {
                    when: When::Tick(6),
                    what: Act::ClickUi(layout::roster_button().center()),
                },
                Directive {
                    when: When::Tick(12),
                    what: Act::ClickUi(layout::roster_open(7).center()),
                },
            ],
        ),
        (
            "the faces list",
            idle_face,
            vec![
                Directive {
                    when: When::Tick(6),
                    what: Act::ClickUi(layout::meter_chip(0).center()),
                },
                Directive {
                    when: When::Tick(12),
                    what: Act::ClickUi(layout::faces_row(0).center()),
                },
            ],
        ),
    ];
    for (surface, wanted_who, script) in surfaces {
        let (_, landed) = selection_run(&script);
        checks.require(
            landed.who == Some(wanted_who)
                && landed.rings.len() == 1
                && landed.named.as_deref() == Some(cast[wanted_who].name),
            "a select surface does not do what the others do",
            format!(
                "{surface} picked {}: the selection reads {:?}, {} ring(s) drew, and the panel \
                 says {:?}; every surface selects the same one person, rings them once and \
                 opens the panel on them",
                cast[wanted_who].name,
                landed.who.map(|who| cast[who].name),
                landed.rings.len(),
                landed.named
            ),
        );
    }
    Some(reproduction)
}

/// One scripted order over the paused opening world: pick somebody, open the
/// site's board, tap a job row.
///
/// The paused world is what makes the comparison clean — dispatch works while
/// the clock holds ("paused - the clock holds, orders still work"), the order
/// is addressed at minute zero, and nothing else moves.
fn ordered_from(site: usize, slot: usize, pick: Act) -> Conducted {
    let marker = layout::marker_rect(LOCATIONS[crate::sim::site_location(site)].tile);
    let script = [
        Directive {
            when: When::Tick(6),
            what: pick,
        },
        Directive {
            when: When::Tick(14),
            what: Act::ClickWorld(marker.center()),
        },
        Directive {
            when: When::Tick(22),
            what: Act::ClickUi(layout::board_row(slot).center()),
        },
    ];
    let mut session = Session::plain(Tuning::SHIPPED, &script, 32);
    session.probe_ticks = &[30];
    conduct(&session)
}

/// **The posting reads the one selection** (UI.md §3b, still two clicks).
///
/// The gesture did not change: select somebody, open a board, tap a job. What
/// changed in wave 1.1 is that "select somebody" is one act with four doors,
/// and what changed in 1.2 is what the tap means — so the posting a
/// sprite-select produces has to be *the same posting* the strip's chip
/// produces, byte-identical in the transcript, which is the shape every other
/// check in this file reads the world through.
///
/// The whole battery runs on the paused opening world: dispatch works while
/// the clock holds ("paused - the clock holds, orders still work"), the order
/// is addressed at minute zero, and nothing else in the world moves to muddy
/// the comparison.
fn posting_reads_the_selection(checks: &mut Checks) {
    let cast = people::roster();
    let who = 0usize;
    let by_chip = ordered_from(0, 0, Act::ClickUi(layout::party_chip(who).center()));
    let by_sprite = ordered_from(0, 0, Act::ClickWorld(cast[who].home.center()));
    checks.require(
        !by_chip.events.is_empty(),
        "the scripted order produced no event at all, so the paths cannot be compared",
        format!(
            "a chip-select of {} and a click on the Watchtower's marker emitted {} events",
            cast[who].name,
            by_chip.events.len()
        ),
    );
    checks.require(
        transcript(&by_chip.events) == transcript(&by_sprite.events),
        "the same order given through two select surfaces is two different orders",
        format!(
            "the chip path emitted {:?} and the sprite path {:?}; dispatch reads the one \
             selection, so the surface the selection came from cannot reach the world",
            transcript(&by_chip.events),
            transcript(&by_sprite.events)
        ),
    );

    // **Every site is still dispatchable while the panel is open.** The panel
    // opens on any selection now and it lies across two of the four markers at
    // the reference camera, so this is the check that the fix did not quietly
    // make those two sites unorderable.
    for site in 0..LOCATIONS.len() - 1 {
        let run = ordered_from(site, 0, Act::ClickWorld(cast[who].home.center()));
        let departed = run
            .events
            .iter()
            .any(|event| event.class == crate::attention::EventClass::Departed);
        checks.require(
            departed,
            "a site cannot be ordered to while the character panel is open over it",
            format!(
                "{} was selected and {} clicked, and the transcript is {:?}; the panel's body \
                 is not a click target and a marker under it still takes the order",
                cast[who].name,
                LOCATIONS[crate::sim::site_location(site)].name,
                transcript(&run.events)
            ),
        );
    }

    // **A posting to somebody who is out is carried, not refused** (wave
    // 1.2). Wave 1.1's order bounced with `NotIdle`, because an order is a
    // thing a body does and a body was already busy; an ask is a thing a
    // person hears, and asks travel. So the second posting is *made*, stands
    // on the ledger unheard, and moves nobody — the messenger's arrival is
    // what delivers it (`compliance::messengers`).
    let away = {
        let marker = |site: usize| {
            layout::marker_rect(LOCATIONS[crate::sim::site_location(site)].tile).center()
        };
        let script = [
            Directive {
                when: When::Tick(6),
                what: Act::ClickUi(layout::party_chip(who).center()),
            },
            Directive {
                when: When::Tick(14),
                what: Act::ClickWorld(marker(0)),
            },
            Directive {
                when: When::Tick(22),
                what: Act::ClickUi(layout::board_row(0).center()),
            },
            Directive {
                when: When::Tick(30),
                what: Act::ClickUi(layout::party_chip(who).center()),
            },
            Directive {
                when: When::Tick(38),
                what: Act::ClickWorld(marker(1)),
            },
            Directive {
                when: When::Tick(46),
                what: Act::ClickUi(layout::board_row(0).center()),
            },
        ];
        let mut session = Session::plain(Tuning::SHIPPED, &script, 56);
        session.probe_ticks = &[54];
        conduct(&session)
    };
    let departures = away
        .events
        .iter()
        .filter(|event| event.class == crate::attention::EventClass::Departed)
        .count();
    let ledger = away.sim.postings.all();
    checks.require(
        departures == 1 && ledger.len() == 2,
        "a posting made to somebody who is out did not stand on the ledger",
        format!(
            "{departures} departure(s) were emitted and the ledger holds {} postings; the \
             second ask was made to {} after they had already left, and an ask that travels \
             is made and carried rather than refused",
            ledger.len(),
            cast[who].name
        ),
    );
    checks.require(
        ledger.last().is_some_and(|posting| {
            posting.heard.is_empty() && posting.status == crate::asks::Status::Open
        }),
        "an ask to somebody who is out was heard where they are not",
        format!(
            "the second posting reads {:?}; nothing binds until it is heard, and {} is on \
             the road",
            ledger
                .last()
                .map(|posting| (posting.status, posting.heard.len())),
            cast[who].name
        ),
    );
}

/// **The job board is where the asking happens** (UI.md §3c) — the job
/// board's own battery, carried forward from the wave that built it.
///
/// Five claims, and each of them is one of the failure modes the board makes
/// possible: the marker must ask nothing, the claim must be the named row,
/// a row somebody has must refuse out loud, a row tapped with nobody selected
/// must refuse out loud, and the fit and travel the panel prints must be the
/// sim's own answers rather than a second computation beside them. What wave
/// 1.2 changed is the verb — the tap posts and the answer is the scorer's —
/// so the claims read the same and mean one step more: the departure they
/// look for is one somebody agreed to.
fn the_board_is_the_ask(checks: &mut Checks) {
    let tuning = Tuning::SHIPPED;
    let grid = grid::grid();
    let cast = people::roster();
    let who = 0usize;
    let marker =
        |site: usize| layout::marker_rect(LOCATIONS[crate::sim::site_location(site)].tile).center();

    // --- 1: the marker opens the board and asks nothing ---------------------
    // The two ways to ask that a marker and a job row would have been is
    // exactly the thing the job-board session removed; a marker that still
    // asked would make the board a decoration over a posting nobody could
    // see.
    let looked = {
        let script = [
            Directive {
                when: When::Tick(6),
                what: Act::ClickUi(layout::party_chip(who).center()),
            },
            Directive {
                when: When::Tick(14),
                what: Act::ClickWorld(marker(1)),
            },
        ];
        let mut session = Session::plain(tuning, &script, 24);
        session.probe_ticks = &[22];
        conduct(&session)
    };
    let opened = looked.probe(22).and_then(|(_, flow, ..)| flow.board);
    checks.require(
        opened == Some(1) && looked.events.is_empty(),
        "a site marker still makes a posting",
        format!(
            "with {} selected, clicking the Deep Cave's marker left the board {opened:?} and              emitted {:?}; the marker opens the board and the job row is the one way to order",
            cast[who].name,
            transcript(&looked.events)
        ),
    );

    // --- 2: the claim is the row that was tapped ----------------------------
    // Slot three of the Watchtower, which is neither the front of the list nor
    // the back: a dispatch that kept first-open underneath claims slot zero
    // and this check reads the difference off the board itself.
    let named = ordered_from(0, 3, Act::ClickUi(layout::party_chip(who).center()));
    let states: Vec<crate::sim::JobState> = named
        .sim
        .sites
        .first()
        .map(|site| site.states.clone())
        .unwrap_or_default();
    let job = named
        .sim
        .sites
        .first()
        .and_then(|site| site.quest(3))
        .map(|quest| quest.name)
        .unwrap_or_default();
    checks.require(
        states.get(3) == Some(&crate::sim::JobState::Claimed { by: who })
            && states
                .iter()
                .take(3)
                .all(|state| *state == crate::sim::JobState::Open),
        "the ask was agreed to for a job other than the one the row named",
        format!(
            "tapping the Watchtower's fourth row left the board reading {states:?}; the row              the player tapped is {job:?} and the claim has to be that row"
        ),
    );
    checks.require(
        named.events.iter().any(|event| {
            event.class == crate::attention::EventClass::Departed && event.note.contains(job)
        }),
        "the departure does not name the job the row named",
        format!(
            "the transcript is {:?} and the row tapped was {job:?}",
            transcript(&named.events)
        ),
    );

    // --- 3: a row somebody already has refuses, and says why ----------------
    let taken = {
        let script = [
            Directive {
                when: When::Tick(6),
                what: Act::ClickUi(layout::party_chip(who).center()),
            },
            Directive {
                when: When::Tick(14),
                what: Act::ClickWorld(marker(0)),
            },
            Directive {
                when: When::Tick(22),
                what: Act::ClickUi(layout::board_row(3).center()),
            },
            Directive {
                when: When::Tick(30),
                what: Act::ClickUi(layout::party_chip(who + 1).center()),
            },
            Directive {
                when: When::Tick(38),
                what: Act::ClickWorld(marker(0)),
            },
            Directive {
                when: When::Tick(46),
                what: Act::ClickUi(layout::board_row(3).center()),
            },
        ];
        let mut session = Session::plain(tuning, &script, 56);
        session.probe_ticks = &[54];
        conduct(&session)
    };
    let (notice, selected, departures) = {
        let probe = taken.probe(54);
        (
            probe.and_then(|(_, flow, ..)| flow.log.first().cloned()),
            probe.and_then(|(_, flow, ..)| flow.selected),
            taken
                .events
                .iter()
                .filter(|event| event.class == crate::attention::EventClass::Departed)
                .count(),
        )
    };
    checks.require(
        departures == 1
            && notice.as_deref() == Some(crate::sim::Refusal::Taken.message("", "", job).as_str()),
        "a claimed job row took a second posting, or refused one in silence",
        format!(
            "{departures} departure(s) were emitted and the top notice reads {notice:?}; the              second character tapped a row {} had already taken",
            cast[who].name
        ),
    );
    checks.require(
        selected == Some(who + 1),
        "a bounced job row put the selection down",
        format!(
            "the selection reads {:?} after the bounce; a refusal says why and changes              nothing else",
            selected.map(|who| cast[who].name)
        ),
    );

    // --- 4: a row tapped with nobody selected refuses, and says why ---------
    let unmanned = {
        let script = [
            Directive {
                when: When::Tick(6),
                what: Act::ClickWorld(marker(0)),
            },
            Directive {
                when: When::Tick(14),
                what: Act::ClickUi(layout::board_row(0).center()),
            },
        ];
        let mut session = Session::plain(tuning, &script, 24);
        session.probe_ticks = &[22];
        conduct(&session)
    };
    let said = unmanned
        .probe(22)
        .and_then(|(_, flow, ..)| flow.log.first().cloned());
    checks.require(
        unmanned.events.is_empty() && said.is_some_and(|line| line.starts_with("select somebody")),
        "a job row tapped with nobody selected did something, or said nothing",
        format!(
            "the transcript is {:?} and the top notice is {:?}",
            transcript(&unmanned.events),
            unmanned
                .probe(22)
                .and_then(|(_, flow, ..)| flow.log.first().cloned())
        ),
    );

    // --- 5: the fit and the travel are the sim's own answers ---------------
    // Not "a number near the sim's": the panel is rebuilt here and every fit
    // it prints is looked up in `traits::competence_at`, and its travel line
    // in `sim::route_out` — the two functions the scorer and the dispatch use.
    let ines = cast
        .iter()
        .position(|person| person.id == "ines")
        .unwrap_or(0);
    let staged = crate::sim::Sim::opening(&tuning, modules::ModuleSet::ALL);
    let mut flow = crate::flow::Flow {
        selected: Some(ines),
        board: Some(1),
        ..crate::flow::Flow::default()
    };
    let panel = screens::content(
        &flow,
        &lens::Lens::on(&staged),
        &grid,
        &crate::clock::Clock::opening(),
        &tuning,
    );
    let says = |text: &str| panel.runs.iter().any(|row| row.text.contains(text));
    for quest in &staged.sites[1].quests {
        let fit = traits::competence_at(quest.task, &cast[ines].traits);
        checks.require(
            says(&format!("fit {fit}")),
            "a job row's fit is not the aptitude the sim reads",
            format!(
                "{:?} is {} work, {} answers {fit} to traits::competence_at, and no row of                  the board says so",
                quest.name,
                quest.task.id(),
                cast[ines].name
            ),
        );
    }
    let route = crate::sim::route_out(&grid, &tuning, &staged, ines, 1);
    checks.require(
        route.as_ref().is_some_and(|route| {
            says(&format!("{} min away", route.cost))
        }),
        "the board's travel line is not the journey the sim would walk",
        format!(
            "sim::route_out puts {}'s door {:?} from the Deep Cave and the header does not              say it; a preview that can disagree with the journey is the whole failure mode",
            cast[ines].name,
            route.map(|route| (route.tiles.len(), route.cost))
        ),
    );
    // And with nobody selected there is no fit column and no travel line at
    // all — a board read by nobody is still a board, and a fit for nobody is
    // a number about nothing.
    flow.selected = None;
    let empty = screens::content(
        &flow,
        &lens::Lens::on(&staged),
        &grid,
        &crate::clock::Clock::opening(),
        &tuning,
    );
    checks.require(
        !empty.runs.iter().any(|row| row.text.starts_with("fit "))
            && !empty.runs.iter().any(|row| row.text.contains("min away")),
        "the board guesses a fit and a journey for nobody",
        format!(
            "with nobody selected the board still prints {:?}",
            empty
                .runs
                .iter()
                .filter(|row| row.text.starts_with("fit ") || row.text.contains("min away"))
                .map(|row| row.text.clone())
                .collect::<Vec<_>>()
        ),
    );
}

/// **The selection is presentation, derived from recorded clicks.**
///
/// Nothing about selecting somebody reaches the world: two runs of the same
/// scenario, one of them clicking its way around every select surface and the
/// other clicking nothing at all, produce the same transcript to the
/// world-minute. That is the whole claim that the one selection added no sim
/// state, and it is what makes a replay of either run the same world.
fn selection_moves_nothing(checks: &mut Checks) {
    let cast = people::roster();
    let run_at_speed = |script: &[Directive]| {
        let mut session = Session::plain(Tuning::SHIPPED, script, 4_000);
        session.stop_at_minute = Some(240);
        conduct(&session)
    };
    let quiet = [Directive {
        when: When::Tick(4),
        what: Act::Tap(Key::Digit1),
    }];
    let clicking = [
        Directive {
            when: When::Tick(4),
            what: Act::Tap(Key::Digit1),
        },
        Directive {
            when: When::Tick(20),
            what: Act::ClickUi(layout::party_chip(2).center()),
        },
        Directive {
            when: When::Tick(28),
            what: Act::ClickWorld(cast[5].home.center()),
        },
        Directive {
            when: When::Tick(36),
            what: Act::ClickUi(layout::roster_button().center()),
        },
        Directive {
            when: When::Tick(44),
            what: Act::ClickUi(layout::roster_open(8).center()),
        },
        // And a click on bare ground, which puts the selection down.
        Directive {
            when: When::Tick(52),
            what: Act::ClickWorld(Vec2::ZERO),
        },
    ];
    let (quiet, clicking) = (run_at_speed(&quiet), run_at_speed(&clicking));
    checks.require(
        !quiet.events.is_empty() && transcript(&quiet.events) == transcript(&clicking.events),
        "selecting people changed what happened in the world",
        format!(
            "{} events with no selecting and {} with a click on every select surface; the \
             selection is UI state derived from recorded input and reaches nothing",
            quiet.events.len(),
            clicking.events.len()
        ),
    );
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let tuning = Tuning::SHIPPED;

    // --- the signature test and the exact-time scripts ---------------------
    let (sweep_summary, baseline) = sweep::run(&mut checks);
    // --- the pathfinder's documented rule ----------------------------------
    path_contracts(&mut checks);
    // --- no randomness -----------------------------------------------------
    seed_independence(&mut checks);

    // --- the photographed session, and everything read off its frames -----
    let photographed_run = photographed(HEADLESS_VIEWPORT);
    // The photographed run is the same orders as the baseline plus two log
    // clicks; its transcript must match the baseline's exactly (the drawer
    // is presentation).
    checks.require(
        transcript(&photographed_run.events) == transcript(&baseline.events),
        "opening the log drawer changed the world",
        format!(
            "the photographed session's transcript differs from the baseline's; a drawer is \
             presentation and may not touch an outcome ({} vs {} events)",
            photographed_run.events.len(),
            baseline.events.len()
        ),
    );
    shots::judge(&mut checks, &photographed_run, &tuning);

    // --- the culling, both ways --------------------------------------------
    culling_probe(&mut checks);
    // --- and a tap, which the engine already made a click ------------------
    touch_selects(&mut checks);

    // --- one selection, and the posting that reads it ----------------------
    let reproduction = one_selection(&mut checks);
    posting_reads_the_selection(&mut checks);
    selection_moves_nothing(&mut checks);
    // --- the job board, which is where the asking happens ------------------
    the_board_is_the_ask(&mut checks);

    // --- the layout floors --------------------------------------------------
    floors::layout_floors(&mut checks);
    floors::drawer_floors(&mut checks);
    floors::content_floors(&mut checks, &baseline);
    let ui_report = floors::uimap_contract(&mut checks);
    let legibility = floors::map_legibility(&mut checks);

    // --- the tuning drawer: one scripted session, read three ways ----------
    let drawer = restart::drawer_run();
    restart::judge(&mut checks, &drawer);
    floors::judge_tuner_screen(&mut checks, &drawer);

    // --- the schedule order, which nothing else can see --------------------
    let order = &baseline.schedule;
    let marks = |name: &str| order.find(name);
    for (first, second, why) in [
        (
            "fit",
            "handle_input",
            "the camera is fitted after the click that converts through it",
        ),
        (
            "remember",
            "advance",
            "the previous clock reading is kept after the clock moved, so the token \
             interpolation reads a future it should not have",
        ),
        (
            "handle_input",
            "advance",
            "orders are read after the clock advanced, so a dispatch at minute M lands at M+1",
        ),
        (
            "advance",
            "fire_due",
            "occurrences fire before the clock reaches them",
        ),
        (
            "handle_input",
            "fire_due",
            "a change to the auto-pause config lands a tick after the events it was meant \
             to catch",
        ),
    ] {
        let (a, b) = (marks(first), marks(second));
        checks.require(
            a.is_some() && b.is_some() && a < b,
            "a system order the game depends on has been reversed",
            format!("{first} is at {a:?} and {second} at {b:?} in the schedule; {why}"),
        );
    }

    // --- the background ----------------------------------------------------
    if let Some(shot) = photographed_run.photo("map") {
        let cleared = shot.frame.plan.clear_color;
        checks.require(
            cleared == theme::VOID,
            "the screen was cleared to a colour the game does not name",
            format!(
                "it cleared to {cleared:?}; the void's constant is {:?}",
                theme::VOID
            ),
        );
        let brightness = cleared.r.max(cleared.g).max(cleared.b);
        checks.require(
            greater(0.25, brightness) && greater(cleared.a, 0.99),
            "the screen is not dark enough for light text to read against",
            format!(
                "its brightest channel is {brightness:.3} at alpha {:.2}",
                cleared.a
            ),
        );
    }

    // --- the attention architecture (GDD §3, wave 0a) ----------------------
    crate::attention::vocabulary(&mut checks);
    crate::attention::judge_at(&mut checks, &tuning);
    crate::attention::feed_is_a_view(&mut checks, &baseline);
    crate::meters::registry(&mut checks, &tuning);
    let pauses = crate::pauses::judge(&mut checks);

    // --- the people substrate: the vocabulary, the registry, the stores ----
    traits::vocabulary(&mut checks);
    traits::arithmetic(&mut checks, &tuning);
    people::registry(&mut checks, &tuning);
    stores::judge_at(&mut checks, &tuning);
    lens::identity(&mut checks, &tuning);
    drift_cadence(&mut checks);

    // --- the module scaffold, and the matrix it exists to be iterated by ----
    modules::registry(&mut checks);
    let matrix = module_matrix(&mut checks);
    let scorer = crate::autonomy::judge_module(&mut checks, &baseline);
    let compliance = crate::compliance::judge_module(&mut checks, &baseline);
    let asked = crate::compliance::ask_run();
    crate::compliance::judge_shots(&mut checks, &asked);
    crate::compliance::judge_at(&mut checks, &tuning);

    // --- the art library, every string, and the link grammar ---------------
    library::library(&mut checks);
    library::printable_strings(&mut checks, &baseline);
    links::link_contracts(&mut checks);

    // --- the round that says whether any of it is an instrument ------------
    let mutations = mutation::mutation_round(&mut checks);

    // --- the pictures a person looks at ------------------------------------
    let narrow = photographed(NARROW_VIEWPORT);
    let captured = capture::capture_screens(
        &mut checks,
        &photographed_run,
        &narrow,
        &drawer,
        reproduction.as_ref(),
        &asked,
    );

    let verdict = checks.verdict();
    println!(
        "verified ninjo over {} conducted events, {} world-minutes of scenario",
        baseline.events.len(),
        baseline.minutes
    );
    println!(
        "  stamp: seed 0, {}, {}",
        tuning.stamp(),
        modules::ModuleSet::ALL.stamp()
    );
    println!(
        "  constants in effect: {}",
        tuning.readout().replace('\n', "  ")
    );
    println!("  {sweep_summary}");
    println!("  seed 0 stamped; transcripts identical at seeds 7 and 7777777 (no Rng read in S1)");
    println!("  ui mapping: {ui_report}");
    println!("  map text: {legibility}");
    println!(
        "  people: {} in the registry, {} traits over {} kinds, {} marks, {} reaction cells",
        crate::people::roster().len(),
        traits::TRAITS.len(),
        traits::TraitKind::ALL.len(),
        traits::MarkId::ALL.len(),
        traits::REACTIONS.len()
    );
    println!(
        "  attention: {} classes, {} meter chips, config {}",
        crate::attention::CLASSES.len(),
        crate::meters::METERS.len(),
        crate::attention::Attention::opening().stamp()
    );
    println!("  auto-pause: {pauses}");
    println!("  module-off matrix: {matrix}");
    println!("  scorer: {scorer}");
    println!("  asks: {compliance}");
    println!(
        "  standing rates: {} - {} postings on the ledger at the end of the run",
        baseline.sim.rates.stamp(),
        baseline.sim.postings.all().len()
    );
    println!("  module set: {}", modules::ModuleSet::ALL.stamp());
    println!("  mutation round: {mutations}");
    println!("{captured}");
    if let Some(shot) = photographed_run.photo("map") {
        print!("{}", shot.frame.transcript());
    }
    verdict
}
