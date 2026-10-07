//! Stack Duel: a two-player card duel fought over one shared stack.
//! See DESIGN.md for the rules. `--verify` runs the headless check.
#![allow(missing_docs)]

mod capture;
mod cards;
mod checks;
mod game;
mod layout;
mod npc;
mod players;
mod rowcheck;
mod rulechecks;
mod rules;
mod ui;
mod verify;

use game::{Game, SLOT_KEYS};
use jidousha::prelude::*;
use std::process::ExitCode;

/// The first match's seed; each new match adds one.
pub const FIRST_SEED: u64 = 7;

/// Every key the game reads.
const KEYS: [Key; 6] = [
    Key::Enter,
    Key::Space,
    Key::Escape,
    Key::ArrowUp,
    Key::ArrowDown,
    Key::ArrowLeft,
];

pub fn config() -> GameConfig {
    GameConfig {
        title: "Stack Duel",
        seed: FIRST_SEED,
        ..GameConfig::default()
    }
}

pub fn register(app: &mut App) {
    app.add_system(Startup, set_up);
    app.add_system(Update, control);
    app.add_system(Update, npc_acts);
    app.add_system(Draw, ui::draw);
}

fn set_up(world: &mut World) {
    world.insert_resource(Camera {
        clear_color: ui::palette::BACKDROP,
        height: ui::VIEW_HEIGHT,
        ..Camera::default()
    });
    world.insert_resource(Game::new(FIRST_SEED));
}

fn control(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let mut pressed: Vec<Key> = Vec::new();
    for key in SLOT_KEYS
        .iter()
        .chain(KEYS.iter())
        .chain([Key::ArrowRight].iter())
    {
        if input.just_pressed(*key) {
            pressed.push(*key);
        }
    }
    let game = world.resource_mut::<Game>();
    for key in pressed {
        game.press(key);
    }
}

fn npc_acts(world: &mut World) {
    world.resource_mut::<Game>().npc_step();
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
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
