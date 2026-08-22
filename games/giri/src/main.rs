//! giri - an auto-battler where the pieces have interests (prototype #1).
//!
//! The player assembles a party from a roster and resolution is automatic:
//! there is no attack verb, and the only verbs there are are social. Roster
//! members consent, refuse, betray, bond and remember. `DESIGN.md` beside this
//! file is the whole design; `src/model.rs` is the social model it describes,
//! `src/beats.rs` is the puzzle chain as data, and `src/verify.rs` is the
//! tutorial run as a test suite.
//!
//! Pointer only. No assets, no audio, no randomness of any kind - v1's outcome
//! is a pure function of (authored beat state, player assignments, tuning
//! constants), which is what makes every beat exactly assertable.
//!
//! Play it:  `cargo run -p giri`
//! On the web: `tools/build-web giri && tools/serve-web giri`
//! Check it: `tools/verify giri`
#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod beats;
mod capture;
mod checks;
mod constants;
mod contracts;
mod flow;
mod judge;
mod model;
mod resolve;
mod ui;
mod verify;

use beats::Dungeon;
use constants::Tuning;
use flow::{Flow, Preview, StartAt};

/// The window the game opens at, and the shape every extent below is stated in.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
/// Half the world height the camera spans - the one number this layout picks.
pub const HALF_H: f32 = 9.0;
/// And half the width, which is that times the shape of the window.
///
/// Derived rather than typed: `HALF_H * (16.0 / 9.0)` would be two facts about
/// one window, and changing `WINDOW` would leave the ratio silently stale.
pub const HALF_W: f32 = HALF_H * WINDOW.aspect();
/// What the camera's `height` is set to.
pub const VIEW_HEIGHT: f32 = HALF_H * 2.0;

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
/// **The Update order is a decision, not a tidy-up.** `handle_pointer` changes
/// the selection and `refresh_preview` recomputes the arithmetic shown for it;
/// in this order a click and the numbers it produces land on the same tick, and
/// reversed the screen would show the previous tick's party. Nothing but a
/// reader protects a system order, so `verify.rs` asserts it out of
/// `schedule_debug()`.
///
/// **The Draw order is a decision too, and the other way round.** `draw_headline`
/// submits glyphs *before* `draw_backdrop` submits the bar behind them: where a
/// game's submission order already agrees with its bands, no assertion over a
/// recorded frame can see a band at all, because the depth sort and the
/// submission sequence produce the same list.
pub fn register(app: &mut App) {
    app.add_system(Startup, open_the_chain);
    app.add_system(Update, flow::handle_pointer);
    app.add_system(Update, flow::refresh_preview);
    app.add_system(Draw, ui::draw_headline);
    app.add_system(Draw, ui::draw_backdrop);
    app.add_system(Draw, ui::draw_roster);
    app.add_system(Draw, ui::draw_main);
    app.add_system(Draw, ui::draw_constants);
}

/// Startup: the camera, the tuning constants, and the first beat.
fn open_the_chain(world: &mut World) {
    // Whatever a harness left in the world before the first tick, or the set
    // the game ships with. A sweep, the mutation round and (next session) the
    // live tuning menu all enter here.
    let tuning = world
        .find_resource::<Tuning>()
        .copied()
        .unwrap_or(Tuning::SHIPPED);
    world.insert_resource(tuning);
    let start = world
        .find_resource::<StartAt>()
        .copied()
        .unwrap_or_default();

    world.insert_resource(Camera {
        center: Vec2::ZERO,
        height: VIEW_HEIGHT,
        clear_color: ui::BACKDROP,
        ..Camera::default()
    });
    world.insert_resource(Flow::default());
    world.insert_resource(Preview::default());
    flow::load_beat(world, start.0);
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
    println!("giri - click a name to offer it a job, then SEND THEM. close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
