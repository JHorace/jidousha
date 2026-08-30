//! ninjo (人情) - the settlement, its people, and the world they live in.
//!
//! You are the head of a small settlement: responsible for everyone, in command
//! of no one. `GDD.md` is the game's design; `DESIGN.md` is the substrate this
//! build stands on - a real-time-with-pause tile world where every occurrence
//! has a world-time and a place, proved out under the name giri-rt and adopted
//! here (`VARIANT.md` records that verdict).
//!
//! What runs today: wave 0b. The substrate's dispatch/travel/resolve loop, plus
//! **the people substrate** - a character registry standing at its home tiles,
//! the trait vocabulary, and the shared-state stores (regard, bonds and
//! grudges, marks) that waves 1 and up write into. Autonomy, needs, petitions
//! and asks are later waves; nobody decides anything for themselves yet.
//!
//! The seams this build exists to lay: `src/lens.rs` is the one read-path every
//! screen goes through, `src/stores.rs` holds the shared state and the only
//! functions that write it, `src/modules.rs` is the registry the module-off
//! verify matrix iterates, and `src/sim.rs` is the one scheduler.
//!
//! Pointer and keyboard. No audio. **No randomness**: the seed plumbing and
//! stamps remain from giri, and no `Rng` read exists yet - verify asserts
//! the whole event transcript identical under far-apart seeds.
//!
//! Play it:  `cargo run -p ninjo`
//! On the web: `tools/build-web ninjo && tools/serve-web ninjo`
//! Check it: `tools/verify ninjo`
#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod camera;
mod capture;
mod checks;
mod clock;
mod constants;
mod floors;
mod flow;
mod frames;
mod grid;
mod layout;
mod lens;
mod library;
mod links;
mod modules;
mod mutation;
mod path;
mod people;
mod presets;
mod restart;
mod screens;
mod sim;
mod sprites;
mod stores;
mod sweep;
mod theme;
mod traits;
mod tuning;
mod ui;
mod verify;
mod web;

use constants::Tuning;
use flow::Flow;

/// The window the game opens at: twice the 960x540 reference surface on each
/// axis, so the shipped window is the chrome design at an exact integer
/// scale.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1920, 1080);

/// The game's configuration, shared by the window and the verify run, so what
/// is verified is what a person plays.
pub fn config() -> GameConfig {
    GameConfig {
        title: "ninjo",
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// Every system this game has, in one place and in one order.
///
/// **The Update order is the phase's contract, not a tidy-up.**
/// `camera::fit` runs first so the click handler converts pointer pixels
/// through the same camera the frame the player clicked on was drawn with;
/// `clock::remember` keeps the previous tick's reading before anything moves
/// it (ADR-0041's idiom, applied to the clock); `flow::handle_input` turns
/// the snapshot into orders **before** `clock::advance` carries the minutes,
/// so an order given at minute M is dispatched at minute M; `sim::fire_due`
/// runs after the advance and fires everything the span crossed, in
/// world-time order; `flow::collect_events` copies the new events into the
/// log the same tick. `verify.rs` asserts this order out of
/// `schedule_debug()`, because nothing but a reader protects it.
pub fn register(app: &mut App) {
    app.add_system(Startup, open_the_world);
    app.add_system(Update, camera::fit);
    app.add_system(Update, clock::remember);
    app.add_system(Update, flow::handle_input);
    app.add_system(Update, clock::advance);
    app.add_system(Update, sim::fire_due);
    app.add_system(Update, flow::collect_events);
    app.add_system(Draw, screens::draw_map);
    app.add_system(Draw, screens::draw_chrome);
    app.add_system(Draw, screens::draw_content);
}

/// Startup: the camera, the constants, the seed, and the opening scenario.
fn open_the_world(world: &mut World) {
    // Whatever a harness left in the world before the first tick, or the set
    // the game ships with. A sweep, the mutation round and the live tuning
    // drawer all enter here.
    let planted = world.find_resource::<Tuning>().copied();
    // **Before the scenario opens**, which is what makes `?constants=` a
    // simulation input rather than a mid-run change. A harness that planted a
    // set wins over the page: `--verify` is not a browser.
    let asked = planted.is_none().then(web::constants).flatten();
    let mut carried = asked.is_some();
    let mut faults: Vec<String> = Vec::new();
    let tuning = match asked {
        Some(Ok(parsed)) => {
            println!("[ninjo] ?constants= accepted - {}", parsed.stamp());
            parsed
        }
        Some(Err(error)) => {
            faults.push(error.message());
            Tuning::SHIPPED
        }
        None => planted.unwrap_or(Tuning::SHIPPED),
    };
    world.insert_resource(tuning);
    // The module set is a simulation input like the constants: whatever a
    // harness planted stands, and a played run has everything on. It rides
    // every stamp (GDD §9).
    if world.find_resource::<modules::ModuleSet>().is_none() {
        world.insert_resource(modules::ModuleSet::ALL);
    }

    // `?seed=` is the rest of the repro-link family — a simulation input,
    // read once. S1 never reads the Rng; the stamp is what the seed is for.
    if world.find_resource::<flow::SessionSeed>().is_none() {
        let seed = match web::seed() {
            Some(Ok(seed)) => {
                println!("[ninjo] ?seed= accepted - {seed}");
                carried = true;
                Some(seed)
            }
            Some(Err(message)) => {
                faults.push(message);
                None
            }
            None => None,
        };
        world.insert_resource(flow::SessionSeed(seed));
    }

    // The camera: the map framed whole, at the default zoom. The viewport put
    // here is only ever read under `headless`, where nothing stamps one at
    // all — which is exactly the case a scripted click has to agree with
    // (jidousha-testing.md's viewport trap).
    let surface = world
        .find_resource::<camera::Surface>()
        .copied()
        .unwrap_or_default();
    world.insert_resource(camera::camera_for(surface.0));
    world.insert_resource(Flow::default());
    flow::install_art(world);
    flow::load_scenario(world);

    // The drawer starts holding what is in effect, so nothing is pending
    // before the player has touched anything. A link that carries constants
    // opens the drawer on them — accepted or refused.
    let flow = world.resource_mut::<Flow>();
    flow.tuner.pending = tuning;
    flow.tuner.open = carried;
    if !faults.is_empty() {
        for fault in &faults {
            eprintln!("[ninjo] {fault}");
        }
        flow.tuner.fault = Some(faults.join("  /  "));
        flow.tuner.open = true;
    }
}

fn main() -> ExitCode {
    // `tools/verify ninjo` runs this same binary with `--verify`: same
    // systems, same config, no window, scripted input, and assertions instead
    // of a person.
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!(
        "ninjo - the world moves. space runs the clock, 1/2/3 set the speed, arrows pan, \
         -/= zoom. click an idle party on the strip, then a site on the map, and watch it \
         go. close the window to quit"
    );
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
