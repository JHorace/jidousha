//! giri - an auto-battler where the pieces have interests (v2, phase P1).
//!
//! The player assembles a party from a roster and resolution is automatic:
//! there is no attack verb, and the only verbs there are are social. Roster
//! members consent, refuse, betray, bond and remember - and since v2 they are
//! somebody: traits, reputation marks, a reason in words for every verdict.
//! `DESIGN.md` beside this file is the whole design; `src/model.rs` holds the
//! state, `src/willing.rs` the decision function, `src/chain.rs` the tutorial
//! as data, and `src/verify.rs` the tutorial run as a test suite.
//!
//! Pointer only. No audio. Since P2, seeded randomness - the engine `Rng`,
//! read at resolution and nowhere else: the outcome is a pure function of
//! (authored beat state, player assignments, constants, variant, seed), and
//! every beat fixes its seed as data, which is what keeps every beat exactly
//! assertable. Willingness never sees a die.
//!
//! Play it:  `cargo run -p giri`
//! On the web: `tools/build-web giri && tools/serve-web giri`
//! Check it: `tools/verify giri`
#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod beats;
mod board;
mod capture;
mod chain;
mod checks;
mod constants;
mod contracts;
mod door;
mod floors;
mod flow;
mod frames;
mod judge;
mod judgment;
mod ladder;
mod layout;
mod library;
mod links;
mod model;
mod mutation;
mod onset;
mod party;
mod presets;
mod pressure;
mod resolution;
mod resolve;
mod restart;
mod scaling;
mod screens;
mod sprites;
mod sweep;
mod theme;
mod traits;
mod tuning;
mod ui;
mod variant;
mod verify;
mod web;
mod willing;

use beats::Dungeon;
use constants::Tuning;
use flow::{Flow, Preview, StartAt};

/// The window the game opens at.
///
/// Twice UI.md §6's reference surface on each axis, so the shipped window is
/// the design at an exact integer scale - which is what "pixel art at integer
/// multiples where possible" means for the default case (UI.md §1.4).
pub const WINDOW: PhysicalSize = PhysicalSize::new(1920, 1080);

/// The game's configuration, shared by the window and the verify run, so what
/// is verified is what a person plays.
pub fn config() -> GameConfig {
    GameConfig {
        title: "giri",
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// Every system this game has, in one place and in one order.
///
/// **The Update order is three decisions, not a tidy-up.** `scaling::fit` runs
/// first, so the click handler after it converts pointer pixels through the
/// same camera the frame the player clicked on was drawn with. `handle_pointer`
/// changes what is taken and who is in, and `refresh_preview` recomputes the
/// arithmetic shown for it; in this order a click and the numbers it produces
/// land on the same tick, and reversed the screen would show the previous
/// tick's party. Nothing but a reader protects a system order, so `verify.rs`
/// asserts it out of `schedule_debug()`.
///
/// **The Draw order is a decision too, and the other way round.** `draw_content`
/// submits every glyph *last*, after the chrome that sits behind it, and the
/// bands in `theme::layers` are what actually put them in front - where a
/// game's submission order already agrees with its bands, no assertion over a
/// recorded frame can see a band at all, because the depth sort and the
/// submission sequence produce the same list. `draw_overlay` submits the log
/// drawer's scrim *before* `draw_content` submits the drawer's own text, so
/// only OVERLAY sorting under OVERLAY_TEXT keeps that text readable.
pub fn register(app: &mut App) {
    app.add_system(Startup, open_the_chain);
    app.add_system(Update, scaling::fit);
    app.add_system(Update, flow::handle_pointer);
    app.add_system(Update, flow::refresh_preview);
    app.add_system(Draw, screens::draw_ground);
    app.add_system(Draw, screens::draw_board);
    app.add_system(Draw, screens::draw_overlay);
    app.add_system(Draw, screens::draw_content);
}

/// Startup: the camera, the tuning constants, the variant and the seed, and
/// the first beat.
fn open_the_chain(world: &mut World) {
    // Whatever a harness left in the world before the first tick, or the set
    // the game ships with. A sweep, the mutation round and the live tuning
    // menu all enter here.
    let planted = world.find_resource::<Tuning>().copied();
    // **Before the first beat**, which is what makes `?constants=` a simulation
    // input rather than a mid-run change (DESIGN §8a). A harness that planted a
    // set wins over the page: `--verify` is not a browser, and a sweep asking
    // for one set and getting another would be a sweep of the wrong thing.
    let asked = planted.is_none().then(web::constants).flatten();
    let mut carried = asked.is_some();
    let mut faults: Vec<String> = Vec::new();
    let tuning = match asked {
        Some(Ok(parsed)) => {
            // Accepted links say so: a link is a claim about what a playtest
            // ran with, and the console is where somebody checks it without
            // opening the drawer (DESIGN §8a: a run is reproducible only if it
            // says what it ran with).
            println!("[giri] ?constants= accepted - {}", parsed.stamp());
            parsed
        }
        Some(Err(error)) => {
            faults.push(error.message());
            Tuning::SHIPPED
        }
        None => planted.unwrap_or(Tuning::SHIPPED),
    };
    world.insert_resource(tuning);

    // `?seed=` and `?variant=` are the rest of the repro-link family (DESIGN
    // §8b, §12) — simulation inputs, read once, before the first beat. A
    // harness plants the resources directly and the page never overrides one
    // it planted, for the sweep reason above.
    if world.find_resource::<flow::SessionSeed>().is_none() {
        let seed = match web::seed() {
            Some(Ok(seed)) => {
                println!("[giri] ?seed= accepted - {seed}");
                // An accepted link opens the drawer exactly as a constants
                // link does (UI.md §12's rule, extended to the family): the
                // stamp is where somebody checks what the URL actually did.
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
    if world.find_resource::<variant::VariantId>().is_none() {
        let chosen = match web::variant() {
            Some(Ok(variant)) => {
                println!("[giri] ?variant= accepted - {}", variant.key());
                carried = true;
                variant
            }
            Some(Err(message)) => {
                faults.push(message);
                variant::VariantId::default()
            }
            None => variant::VariantId::default(),
        };
        world.insert_resource(chosen);
    }
    let start = world
        .find_resource::<StartAt>()
        .copied()
        .unwrap_or_default();

    // `center` and `height` are the game's and are refitted every tick by
    // `scaling::fit` from whatever `viewport` the driver last stamped. The
    // viewport put here is only ever read on the first tick and under
    // `headless`, where nothing stamps one at all - which is exactly the case a
    // scripted click has to agree with (jidousha-testing.md's viewport trap).
    let surface = world
        .find_resource::<scaling::Surface>()
        .copied()
        .unwrap_or_default();
    world.insert_resource(scaling::camera_for(surface.0));
    world.insert_resource(Flow::default());
    world.insert_resource(Preview::default());
    flow::install_art(world);
    flow::load_beat(world, start.0);

    // The drawer starts holding what is in effect, so nothing is pending before
    // the player has touched anything.
    //
    // **A link that carries constants opens the drawer on them** - accepted or
    // refused. A refused one has to be loud on the page (UI.md §12) and an
    // accepted one has the same problem in the other direction: a playtest link
    // whose weights are only in the URL is a link nobody checked. One click
    // closes it, and the constants it was carrying are the ones on screen.
    let flow = world.resource_mut::<Flow>();
    flow.tuner.pending = tuning;
    flow.tuner.open = carried;
    if !faults.is_empty() {
        for fault in &faults {
            eprintln!("[giri] {fault}");
        }
        flow.tuner.fault = Some(faults.join("  /  "));
        flow.tuner.open = true;
    }
}

/// A job, as the dungeon panel states it: what it takes, what it holds, and
/// what one body gets if everybody comes home.
///
/// A function rather than a `format!` in the draw system: the font draws an
/// unknown character as a box at a letter's width, so no assertion over drawn
/// quads can see a wrong one and the string itself is the only instrument.
pub fn job_line(dungeon: &Dungeon) -> String {
    format!(
        "{} - takes {}, pot {}, cut {}, {} each",
        dungeon.name,
        dungeon.headcount,
        dungeon.pot,
        dungeon.cut,
        model::share_each(
            dungeon.pot,
            dungeon.cut,
            i32::try_from(dungeon.headcount).unwrap_or(i32::MAX)
        ),
    )
}

fn main() -> ExitCode {
    // `tools/verify giri` runs this same binary with `--verify`: same systems,
    // same config, no window, scripted pointer, and assertions instead of a
    // person.
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!(
        "giri - a job board and a roster of people with interests. click a job to take \
         it, click people to add them, read the chip under the party, then SEND PARTY. \
         close the window to quit"
    );
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
