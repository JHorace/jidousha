//! Game flow: UI state, and the input that moves all of it.
//!
//! **Everything the player does arrives through the `InputSnapshot`** — the
//! speed keys and chips, the pan and zoom, the dispatch clicks — so a replay
//! carries the player's pacing for free and the determinism contract stays
//! whole (DESIGN §4). Nothing in this file advances the world: the clock is
//! `clock::advance`'s and the scheduler is `sim::fire_due`'s; this file turns
//! input into orders and UI state.
//!
//! The dispatch vocabulary is two clicks (DESIGN §5): select an idle party on
//! the strip, then click a site's marker on the map. A refused order bounces
//! — a toast and a log line, never silence.

use jidousha::prelude::*;

use crate::camera::UiMap;
use crate::clock::{Clock, Rate, stamp};
use crate::constants::Tuning;
use crate::grid::Grid;
use crate::lens::Lens;
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

/// The UI's state — none of it simulation state.
#[derive(Clone, Debug, Default)]
pub struct Flow {
    /// The party picked for dispatch, if one is.
    pub selected: Option<usize>,
    /// The log, most recent first. Secondary by design; every event is also
    /// in `Sim::events`, which is the record.
    pub log: Vec<String>,
    /// How many sim events have been copied into the log so far.
    pub logged_events: usize,
    /// Whether the log drawer is open.
    pub log_open: bool,
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
    /// Put a line at the top of the log.
    pub fn note(&mut self, line: String) {
        self.log.insert(0, line);
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
    world.insert_resource(Sim::opening(&tuning));
    world.insert_resource(Clock::opening());
    let flow = world.resource_mut::<Flow>();
    flow.selected = None;
    flow.log_open = false;
    flow.toast = None;
    flow.logged_events = 0;
    flow.seed = seed;
    let modules = world
        .find_resource::<ModuleSet>()
        .copied()
        .unwrap_or_default();
    let flow = world.resource_mut::<Flow>();
    // The stamp on the opening line carries seed and module set; the
    // constants ride the drawer's own stamp, which is always on screen while
    // it is open (GDD §9: stamps carry seed, constants, variant, module set).
    flow.note(format!(
        "seed {seed} - {} - the world opens paused; space runs it",
        modules.stamp()
    ));
}

/// Copy newly fired events into the log, newest first.
///
/// Registered after `sim::fire_due`, so a span's events land the tick they
/// fire.
pub fn collect_events(world: &mut World) {
    let lines: Vec<String> = {
        // The log is a screen, so it reads the world through the lens like
        // every other one (`lens.rs`).
        let lens = Lens::on(world.resource::<Sim>());
        let from = world.resource::<Flow>().logged_events;
        lens.events()[from..]
            .iter()
            .map(|event| event.line(&lens))
            .collect()
    };
    if lines.is_empty() {
        return;
    }
    let total = world.resource::<Sim>().events.len();
    let flow = world.resource_mut::<Flow>();
    for line in lines {
        flow.note(line);
    }
    flow.logged_events = total;
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

    // A toast is transient by the clock, not by the next click.
    {
        let flow = world.resource_mut::<Flow>();
        if flow.toast.as_ref().is_some_and(|toast| tick >= toast.until) {
            flow.toast = None;
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

    // The log drawer swallows clicks while it is open.
    if world.resource::<Flow>().log_open {
        world.resource_mut::<Flow>().log_open = false;
        return;
    }
    if layout::log_button().contains(at) {
        let flow = world.resource_mut::<Flow>();
        flow.log_open = true;
        flow.tuner.open = false;
        return;
    }

    // The speed chips do what the keys do.
    let chip_actions = [None, Some(Rate::X1), Some(Rate::X2), Some(Rate::X4)];
    for (index, action) in chip_actions.into_iter().enumerate() {
        if layout::speed_chip(index).contains(at) {
            apply_speed(world, tick, action);
            return;
        }
    }

    // The party strip: click an idle party to pick it up, a picked one to put
    // it down.
    let party_count = world.resource::<Sim>().parties.len();
    for index in 0..party_count {
        if !layout::party_chip(index).contains(at) {
            continue;
        }
        let idle = world
            .resource::<Sim>()
            .parties
            .get(index)
            .is_some_and(|party| party.activity == sim::Activity::Idle);
        let name = world
            .resource::<Sim>()
            .parties
            .get(index)
            .map_or("someone", |party| party.name);
        let flow = world.resource_mut::<Flow>();
        if flow.selected == Some(index) {
            flow.selected = None;
        } else if idle {
            flow.selected = Some(index);
        } else {
            let text = format!("{name} is out - only an idle party takes orders");
            flow.bounce(tick, text);
        }
        return;
    }

    // Anything under the top bar or on the strip band is chrome, not map.
    if layout::topbar().contains(at) || layout::party_strip().contains(at) {
        return;
    }

    // The map: a click on a site's marker dispatches the picked party.
    for (site_index, site) in world.resource::<Sim>().sites.clone().iter().enumerate() {
        let marker = layout::marker_rect(crate::grid::LOCATIONS[site.location].tile);
        if !marker.contains(at_world) {
            continue;
        }
        order_dispatch(world, tick, site_index);
        return;
    }
}

/// One speed change: `None` is the pause toggle, `Some` picks a rate and
/// resumes.
fn apply_speed(world: &mut World, tick: u64, change: Option<Rate>) {
    let clock = world.resource_mut::<Clock>();
    let said = match change {
        None => {
            clock.paused = !clock.paused;
            if clock.paused {
                "paused - the clock holds, orders still work".to_owned()
            } else {
                format!("running at {}", clock.rate.label())
            }
        }
        Some(rate) => {
            clock.rate = rate;
            clock.paused = false;
            format!("running at {}", rate.label())
        }
    };
    let minutes = clock.minutes;
    let flow = world.resource_mut::<Flow>();
    flow.note(format!("{} - {said}", stamp(minutes)));
    let _ = tick;
}

/// The dispatch order: the picked party to this site, at the clock's minute.
fn order_dispatch(world: &mut World, tick: u64, site_index: usize) {
    let Some(party_index) = world.resource::<Flow>().selected else {
        let site = crate::grid::LOCATIONS[sim::site_location(site_index)].name;
        let text = format!("pick an idle party on the strip, then click {site}");
        world.resource_mut::<Flow>().bounce(tick, text);
        return;
    };
    let now = world.resource::<Clock>().minutes;
    let tuning = *world.resource::<Tuning>();
    let grid = world.resource::<Grid>().clone();
    let refused = {
        let sim = world.resource_mut::<Sim>();
        sim::dispatch(sim, &grid, &tuning, now, party_index, site_index).err()
    };
    match refused {
        None => {
            world.resource_mut::<Flow>().selected = None;
        }
        Some(refusal) => {
            let party = world
                .resource::<Sim>()
                .parties
                .get(party_index)
                .map_or("someone", |party| party.name)
                .to_owned();
            let site = crate::grid::LOCATIONS[sim::site_location(site_index)].name;
            let text = refusal.message(&party, site);
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
