//! Brolf: battle-royale golf on one screen, six golfers and a zone that closes on all of them.
//!
//! You and five rivals each walk to your own ball and hit it toward one hole while a safe
//! zone shrinks in five seeded steps; whoever's body or ball stays outside it past a
//! five-second grace is out. Space clubs a rival (a three-second daze, never a kill) or
//! strikes their resting ball; equipment lying on the course changes how you play; you win
//! by holing out, by being last, or by being nearest the pin at the end, or you leave early
//! by extracting on the pad, keeping what you hold.
//!
//! Key modules: `zone`, `shots`, `contact`, `items`, `outcome` (the rules as pure
//! functions), `intent` and `systems` (who does what each tick), `draw` and `text` (what
//! is shown), `verify`, `gates`, `players`, `capture` (the check).
//! Depends on: `jidousha` only.
//! INVARIANT: simulation is seeded and fixed-step; every rule a check or a screen needs
//! is a free function over plain values that the systems also call.
//!
//! Run it: `cargo run -p brolf` (WASD walk, click to shoot, Space to club or strike, E on the pad)
//! Check it: `tools/verify brolf`

#![allow(missing_docs)]

mod capture;
mod checks;
mod contact;
mod draw;
mod gates;
mod gates_end;
mod gates_frames;
mod gates_play;
mod intent;
mod items;
mod outcome;
mod players;
mod shots;
mod systems;
mod text;
mod verify;
mod world;
mod zone;

use std::process::ExitCode;

use jidousha::prelude::*;

/// The window the game opens at, and the shape every extent is stated in.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
/// Half the world height the camera spans.
pub const HALF_H: f32 = 9.0;
/// Half the width, derived from the window's shape.
pub const HALF_W: f32 = HALF_H * WINDOW.aspect();
/// Half the extent of the playable rect, about the origin.
pub const COURSE_HALF: Vec2 = Vec2::new(14.0, 7.4);
/// What the court is cleared to: dark enough for white and blue rings to read against.
pub const COURT: Color = Color::rgb(0.10, 0.20, 0.12);
/// The seed the window and the verify run share: the game a person plays is the game checked.
pub const SEED: u64 = 7;
/// The height of one line of band text, in world units.
pub const TEXT_SIZE: f32 = 0.33;
/// The top of each band line: two in the top band, two in the bottom band.
pub const LINE_Y: [f32; 4] = [-8.95, -8.55, 8.10, 8.50];

/// Draw bands, named once.
pub mod layers {
    /// The court's grass and furniture.
    pub const COURSE: i16 = -1;
    /// The zone's rings.
    pub const ZONE: i16 = 0;
    /// Golfers, balls, equipment.
    pub const PLAY: i16 = 1;
    /// The aim preview and the contact ring.
    pub const AIM: i16 = 2;
    /// Band text and the result.
    pub const UI: i16 = 3;
}

/// The configuration the window and the verify run share.
pub fn config() -> GameConfig {
    GameConfig {
        title: "brolf",
        seed: SEED,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// Camera and the dealt match.
fn set_up(world: &mut World) {
    world.insert_resource(Camera {
        center: Vec2::ZERO,
        height: 2.0 * HALF_H,
        clear_color: COURT,
        viewport: WINDOW,
    });
    world::reset_match(world);
}

/// Register the game: Startup, then Update in the order the rules need, then Draw.
pub fn register(app: &mut App) {
    app.add_system(Startup, set_up);
    app.add_system(Update, systems::decide);
    app.add_system(Update, systems::walk);
    app.add_system(Update, systems::swing);
    app.add_system(Update, systems::shoot);
    app.add_system(Update, systems::fly);
    app.add_system(Update, systems::pickup);
    app.add_system(Update, systems::expose);
    app.add_system(Update, systems::extract);
    app.add_system(Update, systems::settle);
    app.add_system(Draw, draw::draw_course);
    app.add_system(Draw, draw::draw_zone);
    app.add_system(Draw, draw::draw_players);
    app.add_system(Draw, draw::draw_aim);
    app.add_system(Draw, draw::draw_bands);
    app.add_system(Draw, draw::draw_result);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!(
        "WASD walk, click shoots at the pointer, Space clubs or strikes, E extracts on the pad"
    );
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
