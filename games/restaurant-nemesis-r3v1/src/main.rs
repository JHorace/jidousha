//! Restaurant Nemesis — a silly restaurant sim where bad service breeds
//! titled nemeses who come back, with followers.
//!
//! Seven days, four rounds a day, a kitchen too small for every order. Fail a
//! diner badly and they return as "Mustard Monster", pickier about mustard and
//! with a following that orders like them. Beat a nemesis three ways; keep
//! the money above zero through day 7. The design is
//! `tools/yakin/runs/restaurant-nemesis-r3v1/DESIGN.md`.
//!
//! Run it:   `cargo run -p restaurant_nemesis_r3v1`
//! Check it: `tools/verify restaurant_nemesis_r3v1`

#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod content;
mod day;
mod players;
mod rules;
mod scenes;
mod screens;
mod sim;
mod verify;

use sim::{Game, Phase, Serve};

/// The window, and the shape the design space is fitted to.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);

/// The seed a window starts on.
pub const FIRST_SEED: u64 = 3;

/// One thing the player can ask for. Every key becomes one of these, and
/// `apply` is the only thing that changes the game from input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Pick the row above or below.
    Pick(i32),
    /// A digit key: a serve at service (0-3), a takedown slot at the ledger
    /// (1-3).
    Digit(u8),
    /// Toggle the temp cook.
    Temp,
    /// Cook the round, open the day, or start a new run.
    Go,
}

/// The key each command is typed with.
pub fn key_for(command: Command) -> Key {
    match command {
        Command::Pick(step) if step < 0 => Key::ArrowUp,
        Command::Pick(_) => Key::ArrowDown,
        Command::Digit(0) => Key::Digit0,
        Command::Digit(1) => Key::Digit1,
        Command::Digit(2) => Key::Digit2,
        Command::Digit(_) => Key::Digit3,
        Command::Temp => Key::T,
        Command::Go => Key::Enter,
    }
}

/// Every command, for reading keys back.
const COMMANDS: [Command; 8] = [
    Command::Pick(-1),
    Command::Pick(1),
    Command::Digit(0),
    Command::Digit(1),
    Command::Digit(2),
    Command::Digit(3),
    Command::Temp,
    Command::Go,
];

/// What this tick's keys ask for.
pub fn commands_from(input: &Input) -> Vec<Command> {
    COMMANDS
        .into_iter()
        .filter(|&command| input.just_pressed(key_for(command)))
        .collect()
}

/// The serve a digit names.
pub fn serve_for(digit: u8) -> Serve {
    match digit {
        0 => Serve::Skip,
        1 => Serve::Standard,
        2 => Serve::Careful,
        _ => Serve::AllOut,
    }
}

/// Apply one command where the run is.
pub fn apply(game: &mut Game, command: Command) {
    match (game.phase, command) {
        (Phase::Service, Command::Pick(step)) => {
            let rows = game.queue.len() as i32;
            if rows > 0 {
                game.selected = (game.selected as i32 + step).rem_euclid(rows) as usize;
            }
        }
        (Phase::Service, Command::Digit(digit)) => {
            sim::set_serve(game, serve_for(digit));
        }
        (Phase::Service, Command::Go) => sim::cook_round(game),
        (Phase::Ledger, Command::Digit(digit)) if digit >= 1 => {
            sim::toggle_takedown(game, usize::from(digit - 1));
        }
        (Phase::Ledger, Command::Temp) => game.spends.temp = !game.spends.temp,
        (Phase::Ledger, Command::Go) => day::open_day(game),
        (Phase::Over(_), Command::Go) => *game = Game::new(game.seed + 1),
        _ => {}
    }
}

impl Resource for Game {}

/// The window's configuration, shared with `--verify`.
pub fn config() -> GameConfig {
    GameConfig {
        title: "restaurant nemesis",
        seed: FIRST_SEED,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// The camera: exactly the 960x540 design rect.
pub fn camera() -> Camera {
    Camera {
        center: screens::DESIGN * 0.5,
        height: screens::DESIGN.y,
        clear_color: screens::palette::FLOOR,
        viewport: WINDOW,
    }
}

/// Every system, in order.
pub fn register(app: &mut App) {
    app.add_system(Startup, open_the_restaurant);
    app.add_system(Update, take_the_orders);
    app.add_system(Draw, screens::draw_the_screen);
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

/// The camera and the first run, unless a check put one in already.
fn open_the_restaurant(world: &mut World) {
    world.insert_resource(camera());
    if world.find_resource::<Game>().is_none() {
        world.insert_resource(Game::new(FIRST_SEED));
    }
}

/// This tick's keys, applied in order.
fn take_the_orders(world: &mut World) {
    let commands = world
        .find_resource::<Input>()
        .map(commands_from)
        .unwrap_or_default();
    let game = world.resource_mut::<Game>();
    for command in commands {
        apply(game, command);
    }
}
