//! Game flow: UI state, and the input that moves all of it.
//!
//! **Everything the player does arrives through the `InputSnapshot`** — the
//! speed keys and chips, the pan and zoom, the dispatch clicks — so a replay
//! carries the player's pacing for free and the determinism contract stays
//! whole (DESIGN §4). Nothing in this file advances the world: the clock is
//! `clock::advance`'s and the scheduler is `sim::fire_due`'s; this file turns
//! input into orders and UI state.
//!
//! The dispatch vocabulary is two clicks (DESIGN §5): select a character —
//! from any surface that shows one — then tap an open job on a site's board.
//! A site marker **opens the board and orders nothing**; the job row is the
//! one way to give an order, so the work is visible before it is chosen. A
//! refused order bounces — a toast and a log line, never silence.
//!
//! **There is one selection** (`Flow::selected`, UI.md §3b). Every select path
//! writes that one index and every highlight reads it, which is what makes a
//! chip-select and a sprite-select the same act rather than two states over
//! one roster.

use jidousha::prelude::*;

use crate::attention::{self, Mode};
use crate::camera::UiMap;
use crate::clock::{Clock, Rate, stamp};
use crate::constants::Tuning;
use crate::grid::{Grid, Tile};
use crate::lens::Lens;
use crate::meters::{self, METERS};
use crate::modules::ModuleSet;
use crate::sim::Sim;
use crate::tuning::Tuner;
use crate::{camera, layout, sim, sprites, tuning};

/// How many ticks a bounced order's toast stays up — about two and a half
/// seconds at the engine's fixed sixty.
pub const TOAST_TICKS: u64 = 168;

/// A transient message about a click that did not do what it looked like it
/// would.
#[derive(Clone, Debug)]
pub struct Toast {
    /// What it says.
    pub text: String,
    /// The tick it stops being drawn.
    pub until: u64,
}

/// The marker a click-to-focus leaves on the place it jumped to.
///
/// **Presentation, and nothing else**: a ring over a tile for a couple of
/// seconds of wall time, so the eye can find what the camera just moved to.
/// The simulation does not know it exists.
#[derive(Clone, Copy, Debug)]
pub struct Pulse {
    /// The tile it rings.
    pub tile: Tile,
    /// The tick it stops being drawn.
    pub until: u64,
}

/// The UI's state — none of it simulation state.
#[derive(Clone, Debug, Default)]
pub struct Flow {
    /// The notices trail, most recent first: what the *player* did, and what
    /// bounced.
    ///
    /// **Not the feed.** The feed is a view of `Sim::events` and is derived
    /// wherever it is drawn (`attention::feed`); this is the other thing — a
    /// speed change, a refused order, a restart — none of which happened in
    /// the world and none of which has a world-time or a place.
    pub log: Vec<String>,
    /// Whether the feed drawer is open.
    pub feed_open: bool,
    /// Whether the auto-pause config drawer is open.
    pub modes_open: bool,
    /// Whether the roster drawer is open — everyone in one list (wave 1.1's
    /// clarity slice). The `r` key and the ROSTER handle both open it.
    pub roster_open: bool,
    /// Which trait chip's explanation is showing, if one is.
    ///
    /// **A trait, not a place**: tapping the same word anywhere it appears —
    /// a sheet, a roster row — shows the same line, because the line is
    /// derived from the row (`traits::explain`) and not written per surface.
    pub explained: Option<crate::traits::TraitId>,
    /// Whether the feed shows the classes the config ignores, dimmed — the
    /// auditing setting, so a player can see what they told the world to
    /// swallow.
    pub show_ignored: bool,
    /// Which meter chip has been drilled into, if one has.
    pub drilled: Option<usize>,
    /// **Which site's job board is open**, if one is (UI.md §3c).
    ///
    /// The site marker opens this and issues no order; a board row is the one
    /// way to dispatch. It is not the selection and it does not close with it:
    /// the board says what the work is while the character panel says who is
    /// being sent, and a two-click order needs both of them up.
    ///
    /// It shares the left of the screen with the faces list, so opening either
    /// shuts the other — `floors::controls_for` names the pair that is
    /// actually on screen, and the overlap floor is about siblings.
    pub board: Option<usize>,
    /// **The selected character** — the one selection this game has.
    ///
    /// One index over the ten people, which is the same index over the ten
    /// parties (`sim::authored_parties`: a party is a one-person band, in
    /// registry order). Every select path writes this field and every render
    /// that highlights reads it, so there is exactly one ring on the map,
    /// exactly one lit chip on the strip, and the panel is open on exactly
    /// the person the player last picked.
    ///
    /// INVARIANT: the panel is open if and only if this is `Some` — nothing
    /// else opens or closes it (`verify::one_selection` asserts the pair).
    ///
    /// Wave 1.1 made a party a character and left S1's dispatch pick beside
    /// wave 0a's character selection, two indices over one roster, each
    /// drawing its own ring (`FINDINGS.md` G-017). This is the one that
    /// replaced them; there is no second one to keep in step.
    pub selected: Option<usize>,
    /// The click-to-focus marker, if one is up.
    pub pulse: Option<Pulse>,
    /// The transient message, if one is up.
    pub toast: Option<Toast>,
    /// The tuning drawer's state (`tuning.rs`).
    pub tuner: Tuner,
    /// The scenario seed on every stamp (DESIGN carries giri's seed
    /// machinery; S1 never reads the `Rng`, and verify asserts it).
    pub seed: u64,
}
impl Resource for Flow {}

impl Flow {
    /// Put a line at the top of the notices.
    pub fn note(&mut self, line: String) {
        self.log.insert(0, line);
    }

    /// Shut every drawer and every panel over the map, and put the selection
    /// down with them.
    ///
    /// One place, because "a drawer and a panel are never both up" is a claim
    /// the floors assert about *pairs of controls*, and the way to keep it
    /// true is to have exactly one function that opens anything. The selection
    /// goes because the character panel *is* the selection (UI.md §3b): there
    /// is no way to shut that panel and leave somebody picked.
    fn close_everything(&mut self) {
        self.feed_open = false;
        self.modes_open = false;
        self.roster_open = false;
        self.tuner.open = false;
        self.drilled = None;
        self.board = None;
        self.selected = None;
    }

    /// Raise a toast, and log the same sentence — nothing appears only in a
    /// toast.
    pub fn bounce(&mut self, tick: u64, text: String) {
        self.note(text.clone());
        self.toast = Some(Toast {
            text,
            until: tick + TOAST_TICKS,
        });
    }
}

/// The session's seed override: `Some` when the page carried `?seed=` or a
/// harness planted one; `None` runs at the scenario's authored seed (zero).
#[derive(Clone, Copy, Debug, Default)]
pub struct SessionSeed(pub Option<u64>);
impl Resource for SessionSeed {}

/// Put the scenario's opening state into the world — startup, and the tuning
/// drawer's APPLY (which restarts the scenario at the new constants, the
/// fork's scenario-boundary reading of giri's beat-boundary rule).
pub fn load_scenario(world: &mut World) {
    let seed = world
        .find_resource::<SessionSeed>()
        .copied()
        .unwrap_or_default()
        .0
        .unwrap_or(0);
    // The Rng is re-seeded so the stamp is honest; S1 never reads it — the
    // plumbing stays for the phases that will (DESIGN §2).
    let tuning = *world.resource::<Tuning>();
    world.insert_resource(Rng::from_seed(seed));
    world.insert_resource(crate::grid::grid());
    let modules = world
        .find_resource::<ModuleSet>()
        .copied()
        .unwrap_or_default();
    world.insert_resource(Sim::opening(&tuning, modules));
    world.insert_resource(Clock::opening());
    let flow = world.resource_mut::<Flow>();
    flow.close_everything();
    flow.toast = None;
    flow.pulse = None;
    flow.show_ignored = false;
    flow.seed = seed;
    flow.explained = None;
    let flow = world.resource_mut::<Flow>();
    // The stamp on the opening line carries seed and module set; the
    // constants ride the drawer's own stamp, which is always on screen while
    // it is open (GDD §9: stamps carry seed, constants, variant, module set).
    flow.note(format!(
        "seed {seed} - {} - the world opens paused; space runs it",
        modules.stamp()
    ));
}

/// Every input of a tick: speed, pan/zoom, and the pointer.
pub fn handle_input(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let (clicked, screen, scroll) = {
        let pointer = input.pointer();
        (
            pointer.just_pressed(PointerButton::Primary),
            pointer.screen,
            pointer.scroll,
        )
    };
    let speed_keys = [
        (Key::Space, None),
        (Key::Digit1, Some(Rate::X1)),
        (Key::Digit2, Some(Rate::X2)),
        (Key::Digit3, Some(Rate::X4)),
    ]
    .into_iter()
    .filter(|(key, _)| input.just_pressed(*key))
    .map(|(_, rate)| rate)
    .collect::<Vec<_>>();
    let roster_key = input.just_pressed(Key::R);
    let pan = Vec2::new(
        f32::from(input.held(Key::ArrowRight)) - f32::from(input.held(Key::ArrowLeft)),
        f32::from(input.held(Key::ArrowDown)) - f32::from(input.held(Key::ArrowUp)),
    );
    let zoom_keys = f32::from(input.held(Key::Minus)) - f32::from(input.held(Key::Equal));
    let tick = world.resource::<Time>().tick;
    let fixed_dt = world.resource::<Time>().fixed_dt.as_f32();

    // Speed changes: the whole of the speed input contract (DESIGN §4) —
    // space toggles pause, 1/2/3 pick a rate (and resume).
    for change in speed_keys {
        apply_speed(world, tick, change);
    }

    // The roster: a registered key opens the same drawer the handle does.
    if roster_key {
        let flow = world.resource_mut::<Flow>();
        let open = flow.roster_open;
        flow.close_everything();
        flow.roster_open = !open;
    }

    // Pan and zoom: presentation, still input-driven and so still replayable.
    {
        let camera = world.resource_mut::<Camera>();
        if pan != Vec2::ZERO {
            camera.center += pan * (camera.height * camera::PAN_RATE * fixed_dt);
        }
        let factor =
            camera::ZOOM_RATE.powf(zoom_keys * fixed_dt) * camera::SCROLL_STEP.powf(-scroll);
        if factor != 1.0 {
            camera.height = (camera.height * factor).clamp(camera::MIN_H, camera::MAX_H);
        }
    }

    // A toast and a focus pulse are both transient by the tick clock, not by
    // the next click. Wall time, both of them: they are presentation.
    {
        let flow = world.resource_mut::<Flow>();
        if flow.toast.as_ref().is_some_and(|toast| tick >= toast.until) {
            flow.toast = None;
        }
        if flow.pulse.is_some_and(|pulse| tick >= pulse.until) {
            flow.pulse = None;
        }
    }

    let at_world = world.resource::<Camera>().screen_to_world(screen);
    let ui = UiMap::for_camera(world.resource::<Camera>());
    let at = ui.ui_of(at_world);

    // The tuning drawer first, and every tick: it covers the screen while it
    // is open, so what it does not want is the only thing anything under it
    // gets.
    if tuning::handle_pointer(world, at, tick, clicked) {
        return;
    }
    if !clicked {
        return;
    }

    // The two attention drawers swallow clicks while they are open, exactly
    // as the tuning drawer does: they cover the screen, and a click that fell
    // through would act on a marker the player cannot see.
    if world.resource::<Flow>().feed_open {
        feed_click(world, at, tick);
        return;
    }
    if world.resource::<Flow>().modes_open {
        modes_click(world, at);
        return;
    }
    if world.resource::<Flow>().roster_open {
        roster_click(world, at);
        return;
    }
    for (handle, open) in [
        (layout::feed_button(), Drawer::Feed),
        (layout::modes_button(), Drawer::Modes),
        (layout::roster_button(), Drawer::Roster),
    ] {
        if !handle.contains(at) {
            continue;
        }
        let flow = world.resource_mut::<Flow>();
        flow.close_everything();
        match open {
            Drawer::Feed => flow.feed_open = true,
            Drawer::Modes => flow.modes_open = true,
            Drawer::Roster => flow.roster_open = true,
        }
        return;
    }

    // The meters band: a chip opens the faces behind its count, and a face
    // opens that character.
    for index in 0..METERS.len() {
        if !layout::meter_chip(index).contains(at) {
            continue;
        }
        let flow = world.resource_mut::<Flow>();
        flow.drilled = (flow.drilled != Some(index)).then_some(index);
        // The faces list and the job board share the left of the screen, so
        // they are never up together — `floors::controls_for` says the same.
        flow.board = None;
        return;
    }
    if let Some(drilled) = world.resource::<Flow>().drilled {
        let faces = {
            let lens = Lens::on(world.resource::<Sim>());
            meters::faces(&lens, drilled)
        };
        for (row, (who, _)) in faces.into_iter().take(layout::FACE_ROWS).enumerate() {
            if layout::faces_row(row).contains(at) {
                world.resource_mut::<Flow>().selected = Some(who);
                return;
            }
        }
        if layout::faces_panel().contains(at) {
            return;
        }
    }
    // **The job board's own controls**: a row is the order, and the close puts
    // the board away.
    //
    // **The board swallows its whole rectangle**, where the character panel's
    // body falls through — and the difference is not an inconsistency. The
    // panel is a detail view that has to be up *through* a dispatch, so a body
    // that took clicks would make the markers under it unorderable. The board
    // is the dispatch surface itself: its rows are the targets, and a marker
    // that answered a click landing between two of them would let a stray tap
    // silently open a different site's board instead of ordering. So a click
    // inside the board is the board's, and the ways out of it are its close,
    // bare ground beyond it, a drawer, or the order itself.
    if let Some(site) = world.resource::<Flow>().board {
        if layout::board_close().contains(at) {
            world.resource_mut::<Flow>().board = None;
            return;
        }
        let jobs = world
            .resource::<Sim>()
            .sites
            .get(site)
            .map_or(0, |board| board.quests.len());
        for slot in 0..jobs.min(layout::BOARD_ROWS) {
            if layout::board_row(slot).contains(at) {
                order_dispatch(world, tick, site, slot);
                return;
            }
        }
        if layout::board_panel().contains(at) {
            return;
        }
    }

    if let Some(who) = world.resource::<Flow>().selected {
        if layout::person_close().contains(at) {
            let flow = world.resource_mut::<Flow>();
            flow.selected = None;
            flow.explained = None;
            return;
        }
        // **A trait chip, tapped**: the same gesture the roster's rows answer,
        // over the same rectangles, showing the same derived line.
        let carried: Vec<crate::traits::TraitId> = {
            let lens = Lens::on(world.resource::<Sim>());
            lens.traits(who).to_vec()
        };
        for (slot, id) in carried.into_iter().take(layout::SHEET_CHIPS).enumerate() {
            if layout::sheet_chip(slot).contains(at) {
                let flow = world.resource_mut::<Flow>();
                flow.explained = (flow.explained != Some(id)).then_some(id);
                return;
            }
        }
        // **The panel's controls swallow a click; its body does not.**
        //
        // The panel is a detail view of the selection, not a drawer: it is up
        // through the whole of a two-click dispatch, and at the reference
        // camera it lies across the Watchtower's and the Black Vault's
        // markers. A body that swallowed clicks would make those two sites
        // undispatchable the moment anybody was selected — dispatch broken by
        // the fix that unified the selection. So the body falls through to the
        // map below it, where a marker orders, a figure moves the selection,
        // and bare ground puts it down and shuts the panel with it. Its own
        // controls — the close button and the trait chips, both handled above
        // — are the only part of it that is a click target.
    }

    // The speed chips do what the keys do.
    let chip_actions = [None, Some(Rate::X1), Some(Rate::X2), Some(Rate::X4)];
    for (index, action) in chip_actions.into_iter().enumerate() {
        if layout::speed_chip(index).contains(at) {
            apply_speed(world, tick, action);
            return;
        }
    }

    // The party strip: a chip is one of the four surfaces a person is
    // selected from, and it selects exactly the way the other three do.
    //
    // **No idle gate here.** Selecting is looking at somebody; ordering is the
    // second click, and that is where being out refuses (`order_dispatch` ->
    // `sim::Refusal::NotIdle`). A chip that refused the *selection* would be a
    // second rule about the one selection, and it is what made the strip's
    // pick feel like a different thing from the map's.
    let party_count = world.resource::<Sim>().parties.len();
    for index in 0..party_count {
        if !layout::party_chip(index).contains(at) {
            continue;
        }
        select(world, index);
        return;
    }

    // Anything under the top bar or on the strip band is chrome, not map.
    if layout::topbar().contains(at) || layout::party_strip().contains(at) {
        return;
    }

    // The map: a click on somebody standing at their home selects them, and
    // opens their panel. Nobody stands on a location's tile (the registry
    // asserts it), so this cannot swallow a dispatch.
    {
        let homes: Vec<(usize, crate::grid::Tile)> = {
            let lens = Lens::on(world.resource::<Sim>());
            (0..lens.people().len())
                .filter(|index| lens.at_home(*index))
                .filter_map(|index| lens.home(index).map(|tile| (index, tile)))
                .collect()
        };
        for (index, home) in homes {
            if !layout::home_rect(home).contains(at_world) {
                continue;
            }
            select(world, index);
            return;
        }
    }

    // **The map: a click on a site's marker opens that site's board.** It
    // issues no order — the job row does, and it is the only thing that does
    // (UI.md §3c). The two clicks a dispatch takes are unchanged in number and
    // changed in what the second one lands on: a named job instead of a site
    // whose front row the player could not see.
    for (site_index, site) in world.resource::<Sim>().sites.clone().iter().enumerate() {
        let marker = layout::marker_rect(crate::grid::LOCATIONS[site.location].tile);
        if !marker.contains(at_world) {
            continue;
        }
        let flow = world.resource_mut::<Flow>();
        flow.board = (flow.board != Some(site_index)).then_some(site_index);
        flow.drilled = None;
        return;
    }

    // Empty ground: nobody there and no board to read, so the selection is put
    // down, the panel shuts with it (UI.md §3b), and the board closes too.
    let flow = world.resource_mut::<Flow>();
    flow.selected = None;
    flow.board = None;
}

/// **Select a person** — the one thing every select surface does.
///
/// Clicking whoever is already selected puts them down, which is the dismiss
/// gesture the map figure and the strip chip have always had; clicking anybody
/// else moves the selection to them. The panel follows the field, so there is
/// nothing else to open.
fn select(world: &mut World, who: usize) {
    let flow = world.resource_mut::<Flow>();
    flow.selected = (flow.selected != Some(who)).then_some(who);
}

/// Which drawer a handle opens.
#[derive(Clone, Copy, Debug)]
enum Drawer {
    /// The feed.
    Feed,
    /// The auto-pause config.
    Modes,
    /// The roster.
    Roster,
}

/// A click inside the open roster drawer.
///
/// A row's name opens that character's panel; a trait chip opens that trait's
/// explanation. Anything else shuts the drawer, exactly as the feed's and the
/// config's own rule.
fn roster_click(world: &mut World, at: Vec2) {
    let people = world.resource::<Sim>().people.len();
    for who in 0..people.min(layout::ROSTER_ROWS) {
        let carried: Vec<crate::traits::TraitId> = {
            let lens = Lens::on(world.resource::<Sim>());
            lens.traits(who).to_vec()
        };
        for (slot, id) in carried.into_iter().take(layout::SHEET_CHIPS).enumerate() {
            if layout::roster_chip(who, slot).contains(at) {
                let flow = world.resource_mut::<Flow>();
                flow.explained = (flow.explained != Some(id)).then_some(id);
                return;
            }
        }
        if layout::roster_open(who).contains(at) {
            let flow = world.resource_mut::<Flow>();
            flow.close_everything();
            flow.selected = Some(who);
            return;
        }
    }
    world.resource_mut::<Flow>().roster_open = false;
}

/// A click inside the open feed drawer.
///
/// **Clicking an entry is click-to-focus**: the camera goes to the event's
/// place and a pulse marker rings the tile for a couple of seconds. Both are
/// presentation — the camera is not simulation state and neither is the
/// marker — so a replay that watched somewhere else still runs the same world.
fn feed_click(world: &mut World, at: Vec2, tick: u64) {
    if layout::feed_ignored_toggle().contains(at) {
        let flow = world.resource_mut::<Flow>();
        flow.show_ignored = !flow.show_ignored;
        return;
    }
    let tuning = *world.resource::<Tuning>();
    let focus = {
        let flow = world.resource::<Flow>();
        let lens = Lens::on(world.resource::<Sim>());
        let entries = attention::feed(&lens, flow.show_ignored, attention::feed_cap(&tuning));
        (0..layout::FEED_ROWS)
            .find(|row| layout::feed_row(*row).contains(at))
            .and_then(|row| entries.get(row).copied())
            .and_then(|entry| lens.events().get(entry.index).map(|event| event.tile))
    };
    let Some(tile) = focus else {
        world.resource_mut::<Flow>().feed_open = false;
        return;
    };
    world.resource_mut::<Camera>().center = tile.center();
    let flow = world.resource_mut::<Flow>();
    flow.feed_open = false;
    flow.pulse = Some(Pulse {
        tile,
        until: tick + attention::pulse_ticks(&tuning),
    });
}

/// A click inside the open auto-pause config drawer.
///
/// **The write goes into the simulation**, not into the UI: the config is sim
/// state, so this click is a recorded input that changes what the world does,
/// and a replay carries it (`attention.rs`).
fn modes_click(world: &mut World, at: Vec2) {
    for (row, class) in attention::EventClass::all().into_iter().enumerate() {
        for (slot, mode) in Mode::ALL.iter().copied().enumerate() {
            if !layout::modes_radio(row, slot).contains(at) {
                continue;
            }
            world.resource_mut::<Sim>().attention.set(class, mode);
            return;
        }
    }
    // Anything that is not one of this drawer's own controls shuts it — the
    // handle included, so the handle toggles. Exactly the feed's rule.
    world.resource_mut::<Flow>().modes_open = false;
}

/// One speed change: `None` is the pause toggle, `Some` picks a rate and
/// resumes.
///
/// **This is also where an auto-pause is acknowledged.** The world stopped
/// itself and said why; the player's next speed input is them having read it,
/// so the reason is cleared here and the next pause-class event can record a
/// new one. A speed input that changes nothing says nothing: a player holding
/// down `1` should not fill the notices with a rate it is already at.
fn apply_speed(world: &mut World, tick: u64, change: Option<Rate>) {
    let clock = world.resource_mut::<Clock>();
    let (said, moved) = match change {
        None => {
            clock.paused = !clock.paused;
            if clock.paused {
                (
                    "paused - the clock holds, orders still work".to_owned(),
                    true,
                )
            } else {
                (format!("running at {}", clock.rate.label()), true)
            }
        }
        Some(rate) => {
            let moved = clock.paused || clock.rate != rate;
            clock.rate = rate;
            clock.paused = false;
            (format!("running at {}", rate.label()), moved)
        }
    };
    let minutes = clock.minutes;
    sim::acknowledge_pause(world.resource_mut::<Sim>());
    if moved {
        let flow = world.resource_mut::<Flow>();
        flow.note(format!("{} - {said}", stamp(minutes)));
    }
    let _ = tick;
}

/// The dispatch order: the selected character to **this job at this site**, at
/// the clock's minute.
///
/// **It reads the one selection** and no pick of its own, so the two clicks are
/// the same two clicks whichever surface the first one landed on — a chip, a
/// figure, a roster row, a face. The second click is a job row on the site's
/// board (UI.md §3c), which is the whole of what changed: the order names the
/// work instead of naming a site and taking whatever was in front.
///
/// A row tapped with nobody selected, a row somebody already has, and a row
/// ordered for a character who is out all **bounce with their reason** and
/// change nothing else (`sim::Refusal`).
fn order_dispatch(world: &mut World, tick: u64, site_index: usize, slot: usize) {
    let site = crate::grid::LOCATIONS[sim::site_location(site_index)].name;
    let job = world
        .resource::<Sim>()
        .sites
        .get(site_index)
        .and_then(|board| board.quest(slot))
        .map_or("that job", |quest| quest.name)
        .to_owned();
    let Some(party_index) = world.resource::<Flow>().selected else {
        let text = format!("select somebody first, then tap {job}");
        world.resource_mut::<Flow>().bounce(tick, text);
        return;
    };
    let now = world.resource::<Clock>().minutes;
    let tuning = *world.resource::<Tuning>();
    let grid = world.resource::<Grid>().clone();
    let refused = {
        let sim = world.resource_mut::<Sim>();
        sim::dispatch(
            sim,
            &grid,
            &tuning,
            now,
            party_index,
            sim::JobId {
                site: site_index,
                slot,
            },
            sim::Motive::ordered(),
        )
        .err()
    };
    match refused {
        None => {
            // **A given order puts the board down with the selection.** The
            // board is the surface you choose on, and the choice is made; a
            // board left open would also lie across the map's other markers,
            // so the next order could not reach them.
            let flow = world.resource_mut::<Flow>();
            flow.selected = None;
            flow.board = None;
        }
        Some(refusal) => {
            let party = world
                .resource::<Sim>()
                .parties
                .get(party_index)
                .map_or("someone", |party| party.name)
                .to_owned();
            let text = refusal.message(&party, site, &job);
            world.resource_mut::<Flow>().bounce(tick, text);
        }
    }
}

/// The art store and its handles, inserted before anything draws.
pub fn install_art(world: &mut World) {
    let mut assets = sprites::store();
    let gallery = sprites::Gallery::load(&mut assets);
    world.insert_resource(assets);
    world.insert_resource(gallery);
}
