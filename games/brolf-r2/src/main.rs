//! Brolf: top-down golf played as a battle royale, against three NPCs.
//!
//! Four golfers each play one ball on a walled course while a circular safe
//! zone shrinks in four phases; a golfer whose body or ball stays outside it
//! for six seconds is eliminated. Golfers walk, aim, charge and strike; they
//! club a golfer in reach (a stun that drops an item), strike a rival's ball
//! for points, and pick up equipment that changes how they play. Nothing is
//! kept unless it is banked — by extracting at the pad once it opens, or by
//! being the last golfer standing.
//!
//! Controls: WASD walk, ArrowLeft/ArrowRight aim, hold Space to charge and
//! release to strike, F club, E take, X extract.
//!
//! Run it:   `cargo run -p brolf_r2`
//! Check it: `tools/verify brolf_r2`

#![allow(missing_docs)]

mod capture;
mod checks;
mod gates;
mod npc;
mod players;
mod rules;
mod screen;
mod sim;
mod verify;

use std::process::ExitCode;

use jidousha::prelude::*;

/// The window the game opens at, and the shape every extent is stated in.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
/// How many world units the camera spans vertically.
pub const VIEW_HEIGHT: f32 = 18.0;
/// Half the view's height.
pub const HALF_H: f32 = VIEW_HEIGHT / 2.0;
/// Half the view's width — the height times the window's shape.
pub const HALF_W: f32 = HALF_H * WINDOW.aspect();
/// The turf the course is cleared to.
pub const TURF: Color = Color::rgb(0.10, 0.16, 0.11);

/// The draw bands, back to front.
pub mod layers {
    /// The course: fence, cups, pad, zone rings.
    pub const FIELD: i16 = -2;
    /// Golfers, balls, pickups, the aim line.
    pub const PLAY: i16 = 0;
    /// The status panel.
    pub const CHROME: i16 = 2;
    /// The result overlay.
    pub const OVERLAY: i16 = 4;
}

/// The game's configuration: seed 7 everywhere, so the course a person plays
/// is the course the verify run checked.
pub fn config() -> GameConfig {
    GameConfig {
        title: "brolf",
        seed: 7,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// Every system, in the order the schedule gate holds.
pub fn register(app: &mut App) {
    app.add_system(Startup, sim::set_the_course);
    app.add_system(Update, sim::read_player_intent);
    app.add_system(Update, sim::decide_npcs);
    app.add_system(Update, sim::walk);
    app.add_system(Update, sim::act);
    app.add_system(Update, sim::roll_balls);
    app.add_system(Update, sim::settle_balls);
    app.add_system(Update, sim::judge_zone);
    app.add_system(Update, sim::end_match);
    app.add_system(Draw, screen::draw_play);
    app.add_system(Draw, screen::draw_course);
    app.add_system(Draw, screen::draw_chrome);
}

fn main() -> ExitCode {
    if std::env::args().any(|arg| arg == "--verify") {
        return verify::run();
    }
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
