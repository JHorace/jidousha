//! Restaurant Nemesis (r3v2): a silly restaurant sim where bad service breeds
//! themed nemeses who come back, with followers.
//!
//! Each day five units of kitchen capacity meet six customers who want more;
//! a bad enough failure spawns a titled problem customer who returns every
//! second day and whose followers turn tomorrow's customers into copies of
//! them. Survive seven days with money and reputation intact.
//!
//! Run it:   `cargo run -p restaurant_nemesis_r3v2`
//! Check it: `tools/verify restaurant_nemesis_r3v2`
//!
//! Key systems: `set_the_scene` (Startup), `advance` (Update, input.rs),
//! `draw` (Draw, screen.rs). The design is
//! `tools/yakin/runs/restaurant-nemesis-r3v2/DESIGN.md`.

#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod gates;
mod input;
mod lines;
mod players;
mod rules;
mod screen;
mod sim;
mod turn;
mod verify;

/// The window the layout is drawn for.
pub(crate) const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
/// The seed the windowed run plays.
pub(crate) const WINDOW_SEED: u64 = 7;

/// Draw bands, named once.
pub(crate) mod layers {
    /// Row separators and backgrounds.
    pub const BACK: i16 = 0;
    /// Every row of chrome text.
    pub const CHROME: i16 = 1;
    /// The nemesis card: its backing and its words.
    pub const OVERLAY: i16 = 2;
}

/// The colours, named once.
pub(crate) mod palette {
    use jidousha::prelude::Color;
    /// The dining room the whole screen is cleared to.
    pub const INK: Color = Color::rgb(0.08, 0.07, 0.09);
    pub const TEXT: Color = Color::rgb(0.93, 0.92, 0.88);
    pub const FAINT: Color = Color::rgb(0.62, 0.60, 0.58);
    pub const NEMESIS: Color = Color::rgb(1.0, 0.80, 0.25);
    pub const SPAWN: Color = Color::rgb(1.0, 0.38, 0.32);
    pub const SAFE: Color = Color::rgb(0.45, 0.88, 0.50);
    pub const NOTICE: Color = Color::rgb(1.0, 0.35, 0.95);
    pub const SEPARATOR: Color = Color::rgb(0.16, 0.15, 0.18);
    pub const CARD: Color = Color::rgb(0.17, 0.12, 0.10);
    pub const CARD_EDGE: Color = Color::rgb(1.0, 0.80, 0.25);
}

pub(crate) fn config(seed: u64) -> GameConfig {
    GameConfig {
        title: "Restaurant Nemesis",
        seed,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// The camera: one world unit is one design unit, 960 x 540 at 16:9.
pub(crate) fn camera() -> Camera {
    Camera {
        center: Vec2::new(480.0, 270.0),
        height: 540.0,
        clear_color: palette::INK,
        viewport: WINDOW,
    }
}

/// Every system, so the window and `--verify` run the same program.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, input::advance);
    app.add_system(Draw, screen::draw);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!(
        "digits serve, S/O on a nemesis card, ENTER closes the kitchen. close the window to quit"
    );
    match run(config(WINDOW_SEED), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// The camera, and day 1 drawn from the world generator.
fn set_the_scene(world: &mut World) {
    world.insert_resource(camera());
    let seed = world.find_resource::<Seed>().map_or(WINDOW_SEED, |s| s.0);
    let game = turn::new_game(seed, world.resource_mut::<Rng>());
    world.insert_resource(game);
}

/// The run's seed, for a restart; a harness inserts it before Startup, the
/// window plays `WINDOW_SEED`.
pub(crate) struct Seed(pub(crate) u64);
impl Resource for Seed {}
