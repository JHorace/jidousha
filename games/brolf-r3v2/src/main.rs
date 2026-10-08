//! Brolf — golf as a battle royale, top-down, against three NPC golfers.
//!
//! A safe zone closes on the cup in three steps; you and your ball must both
//! stay inside it. Club a rival in reach to disable them and knock their best
//! item loose, or strike their ball away; gather equipment that changes your
//! shots, your ball and your resilience; win by holing out, by being the last
//! golfer in the zone, or by extracting through the gate with your two best
//! items.
//!
//! Run it: `cargo run -p brolf_r3v2`
//! Check it: `tools/verify brolf_r3v2`
//!
//! The design is `tools/yakin/runs/brolf-r3v2/DESIGN.md`.

#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod draw;
mod gates;
mod gates_more;
mod hud;
mod model;
mod npc;
mod players;
mod rules;
mod systems;
mod verify;
mod world;

/// The configuration the window and every verify run share.
pub fn config() -> GameConfig {
    GameConfig {
        title: "brolf",
        seed: model::SEED,
        window_size: model::WINDOW,
        ..GameConfig::default()
    }
}

/// Every system, in run order; a gate asserts the Update order.
pub fn register(app: &mut App) {
    app.add_system(Startup, world::set_up_course);
    app.add_system(Update, systems::player_intent);
    app.add_system(Update, systems::npc_intents);
    app.add_system(Update, systems::apply_contacts);
    app.add_system(Update, systems::apply_walks);
    app.add_system(Update, systems::apply_shots_takes_extracts);
    app.add_system(Update, systems::move_balls);
    app.add_system(Update, systems::zone_grace);
    app.add_system(Update, systems::end_match);
    app.add_system(Draw, draw::draw_course);
    app.add_system(Draw, draw::draw_play);
    app.add_system(Draw, draw::draw_hud);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("{}", hud::LEGEND);
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
