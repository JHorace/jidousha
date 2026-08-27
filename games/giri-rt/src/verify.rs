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
//! written back), the floors, the drawer session, the mutation round, and
//! the captures a person looks at.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::FrameRecord;

use crate::checks::{Checks, greater};
use crate::constants::Tuning;
use crate::grid::{LOCATIONS, Terrain, Tile};
use crate::path::route;
use crate::sim::Activity;
use crate::sweep::{Act, Conducted, Directive, Photo, Session, When, conduct, transcript};
use crate::{
    camera, capture, floors, frames, grid, layout, library, links, mutation, restart, screens,
    sweep, theme,
};

/// The surface the reference run draws at: the 960x540 chrome design doubled,
/// which is the window the game opens.
pub const HEADLESS_VIEWPORT: PhysicalSize = crate::WINDOW;

/// The narrow surface the second capture set uses — narrow rather than short
/// on purpose: horizontal shrink is the axis scaling defects live on.
pub const NARROW_VIEWPORT: PhysicalSize = PhysicalSize::new(600, 540);

/// The photographed session: the all-1x script, with the log drawer opened
/// once the first quests have completed — the two screenshots the phase owes
/// (parties mid-travel on different routes; the log after a completed quest).
pub fn photographed(viewport: PhysicalSize) -> Conducted {
    let mut script: Vec<Directive> = vec![Directive {
        when: When::Tick(5),
        what: Act::Tap(Key::Digit1),
    }];
    script.extend(sweep::order(8, 0, 0));
    script.extend(sweep::order(12, 1, 1));
    script.extend(sweep::order(20, 2, 2));
    // Open the log once OWL's and CRANE's quests have completed, so the
    // capture shows quest-complete lines; close it again before the second
    // wave (a click anywhere closes the drawer).
    script.push(Directive {
        when: When::Minute(180),
        what: Act::ClickUi(layout::log_button().center()),
    });
    script.push(Directive {
        when: When::Minute(184),
        what: Act::ClickUi(Vec2::new(480.0, 460.0)),
    });
    script.extend(sweep::order(330, 0, 3));
    let photos = [
        Photo {
            name: "map",
            minute: 40,
            tick: 0,
        },
        Photo {
            name: "log",
            minute: 181,
            tick: 0,
        },
    ];
    conduct(&Session {
        tuning: Tuning::SHIPPED,
        seed: None,
        directives: &script,
        photos: &photos,
        probe_ticks: &[],
        viewport,
        max_ticks: 60_000,
        stop_at_rest: true,
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

    // Ebisu -> Watchtower: all road, 47 tiles, 94 minutes — longer in tiles
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

    // Ebisu -> Deep Cave: 5 road tiles east along the spine, then 7 forest
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

    // Ebisu -> Black Vault: around the peak ridge's east end — 48 road
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

/// **The tokens sit where the derivation says** (ADR-0041, DESIGN §3): the
/// mid-travel frame carries a token-sized quad at each party's derived
/// position — presentation read from discrete state, never written back.
fn judge_tokens(checks: &mut Checks, shot: &sweep::Shot) {
    let tuning = Tuning::SHIPPED;
    let reading = shot.clock.reading(&tuning);
    let mut travelling = 0;
    for (index, party) in shot.sim.parties.iter().enumerate() {
        if matches!(
            party.activity,
            Activity::Outbound { .. } | Activity::Homebound { .. }
        ) {
            travelling += 1;
        }
        let expected = screens::token_position(party, reading) - Vec2::splat(layout::TOKEN * 0.5)
            + Vec2::new(index as f32 * 4.0, index as f32 * -4.0);
        let drawn = shot.frame.quads().iter().any(|quad| {
            let bounds = quad.bounds();
            crate::checks::near(bounds.min.x, expected.x)
                && crate::checks::near(bounds.min.y, expected.y)
                && crate::checks::near(bounds.size().x, layout::TOKEN)
        });
        checks.require(
            drawn,
            "a party token is not drawn at its derived position",
            format!(
                "{}'s token should sit at ({:.1}, {:.1}) at clock reading {reading:.2} and no \
                 token-sized quad does",
                party.name, expected.x, expected.y
            ),
        );
    }
    checks.require(
        travelling >= 2,
        "the mid-travel photograph does not show two parties on the road",
        format!(
            "{travelling} of {} parties are travelling at the photographed minute; \
             simultaneity is the point",
            shot.sim.parties.len()
        ),
    );
    let routes: Vec<Option<Tile>> = shot
        .sim
        .parties
        .iter()
        .map(|party| match &party.activity {
            Activity::Outbound { route, .. } => route.tiles.last().copied(),
            _ => None,
        })
        .collect();
    let distinct = routes
        .iter()
        .flatten()
        .collect::<std::collections::BTreeSet<_>>();
    checks.require(
        distinct.len() >= 2,
        "the two travelling parties are not on visibly different routes",
        format!("outbound goals: {routes:?}"),
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
    if let Some(shot) = photographed_run.photo("map") {
        judge_terrain(&mut checks, &shot.frame, HEADLESS_VIEWPORT);
        judge_tokens(&mut checks, shot);
        frames::judge_chrome(&mut checks, &photographed_run, shot, "the mid-travel map");
        floors::judge_frame_floor(
            &mut checks,
            photographed_run.font,
            &shot.frame,
            "the mid-travel map",
        );
    } else {
        checks.require(
            false,
            "the mid-travel photograph was never taken",
            "the conductor's photo schedule names minute 40".to_owned(),
        );
    }
    if let Some(shot) = photographed_run.photo("log") {
        checks.require(
            shot.flow.log_open,
            "the log photograph was taken with the drawer shut",
            format!("log_open is {}", shot.flow.log_open),
        );
        let complete = shot.flow.log.iter().any(|line| line.contains("completed"));
        checks.require(
            complete,
            "the log photograph carries no completed quest",
            format!("the log at the photograph reads {:?}", shot.flow.log),
        );
        frames::judge_chrome(&mut checks, &photographed_run, shot, "the log drawer");
    } else {
        checks.require(
            false,
            "the log photograph was never taken",
            "the conductor's photo schedule names minute 181".to_owned(),
        );
    }

    // --- the culling, both ways --------------------------------------------
    culling_probe(&mut checks);

    // --- the layout floors --------------------------------------------------
    floors::layout_floors(&mut checks);
    floors::tuner_floors(&mut checks);
    floors::content_floors(&mut checks, &baseline);
    let ui_report = floors::uimap_contract(&mut checks);

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
            "fire_due",
            "collect_events",
            "the log trails its events by a tick",
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

    // --- the art library, every string, and the link grammar ---------------
    library::library(&mut checks);
    library::printable_strings(&mut checks, &baseline);
    links::link_contracts(&mut checks);

    // --- the round that says whether any of it is an instrument ------------
    let mutations = mutation::mutation_round(&mut checks);

    // --- the pictures a person looks at ------------------------------------
    let narrow = photographed(NARROW_VIEWPORT);
    let captured = capture::capture_screens(&mut checks, &photographed_run, &narrow, &drawer);

    let verdict = checks.verdict();
    println!(
        "verified giri-rt over {} conducted events, {} world-minutes of scenario",
        baseline.events.len(),
        baseline.minutes
    );
    println!(
        "  constants in effect: {}",
        tuning.readout().replace('\n', "  ")
    );
    println!("  {sweep_summary}");
    println!("  seed 0 stamped; transcripts identical at seeds 7 and 7777777 (no Rng read in S1)");
    println!("  ui mapping: {ui_report}");
    println!("  mutation round: {mutations}");
    println!("{captured}");
    if let Some(shot) = photographed_run.photo("map") {
        print!("{}", shot.frame.transcript());
    }
    verdict
}
