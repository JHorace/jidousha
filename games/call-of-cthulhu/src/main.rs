//! Call of Cthulhu — eldritch beings phone you at night, and every exchange on the line
//! costs sanity. Learn their lore in the morning and the answers that end a call quickly
//! are the ones you know. Anger one and it costs more than the drain. See `DESIGN.md`.
//!
//! `cargo run -p call-of-cthulhu` plays it; `--verify` runs the checks headless.

#![allow(missing_docs)]

mod capture;
mod checks;
mod decisions;
mod driver;
mod game;
mod input;
mod lore;
mod palette;
mod players;
mod rules;
mod screens;
mod verify;
mod view;

use std::process::ExitCode;

use jidousha::prelude::*;

use crate::game::Game;
use crate::screens::{DESIGN_H, DESIGN_W};
use crate::view::UiMap;

/// The window, and the recorder's viewport: one shape, 16:9.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);

/// The seed a run starts from unless a check inserts another.
pub const DEFAULT_SEED: u64 = 0xC7A1;

/// A seed a check puts in the world before the first tick.
pub struct RunSeed(pub u64);
impl Resource for RunSeed {}

/// The camera: the design rectangle at one world unit per design unit on a 16:9 view.
pub fn camera() -> Camera {
    Camera {
        center: Vec2::new(DESIGN_W * 0.5, DESIGN_H * 0.5),
        height: DESIGN_H,
        clear_color: palette::BG,
        viewport: WINDOW,
    }
}

/// How the game is configured.
pub fn config() -> GameConfig {
    GameConfig {
        title: "Call of Cthulhu",
        seed: DEFAULT_SEED,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// The systems, in the order they run.
pub fn register(app: &mut App) {
    app.add_system(Startup, start);
    app.add_system(Update, input::apply_input);
    app.add_system(Draw, draw_screen);
}

fn start(world: &mut World) {
    let seed = world
        .find_resource::<RunSeed>()
        .map_or(DEFAULT_SEED, |seed| seed.0);
    world.insert_resource(Game::new(seed));
    world.insert_resource(camera());
}

fn draw_screen(ctx: &mut DrawCtx) {
    let map = UiMap::for_camera(ctx.world.resource::<Camera>());
    let game = ctx.world.resource::<Game>();
    let panel = screens::panel(game);
    let bars = screens::bars(game);
    for (rect, color) in bars {
        ctx.rect(map.rect_to_world(rect), color, Depth::layer(0));
    }
    panel.draw(ctx, &map, |icon, _scale| match icon.art {});
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
