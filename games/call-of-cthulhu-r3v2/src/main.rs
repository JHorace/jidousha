//! Call of Cthulhu: eldritch beings telephone you at night, and every exchange
//! on the line costs sanity. Study their lore by day so you can give the
//! answers that make them hang up satisfied; offend one and it screams.
//!
//! Keys: the digit row picks a numbered option, Enter continues past a dawn or
//! an ending. Five days, three beings; lose when sanity runs out, win by
//! hearing the dawn of day six.
//!
//! Run it: `cargo run -p call_of_cthulhu_r3v2` · check it:
//! `python3 tools/verify call_of_cthulhu_r3v2`.

#![allow(missing_docs)]

mod beings;
mod capture;
mod checks;
mod flow;
mod players;
mod rules;
mod screen;
mod verify;
mod verify_screens;

use std::process::ExitCode;

use flow::{Choice, Game, Screen, step};
use jidousha::prelude::*;
use screen::{BAR_AT, BAR_H, BAR_W, DESIGN, Identity, LOW_SANITY, layers, palette};

/// The window, and the aspect the design rect is laid out for.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
/// The seed the shipped game plays.
pub const SEED: u64 = 1928;

/// The digit keys, in option order.
pub const DIGITS: [Key; 7] = [
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
    Key::Digit7,
];

pub fn config() -> GameConfig {
    GameConfig {
        title: "call of cthulhu",
        seed: SEED,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

pub fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, play);
    app.add_system(Draw, draw_screen);
}

/// The camera over the design rect, and a fresh run.
fn set_the_scene(world: &mut World) {
    world.insert_resource(camera());
    let order = rules::rotation(world.resource_mut::<Rng>());
    world.insert_resource(Game::new(order));
}

/// The camera the game and every check build: the design rect, exactly.
pub fn camera() -> Camera {
    Camera {
        center: DESIGN * 0.5,
        height: DESIGN.y,
        clear_color: palette::NIGHT,
        ..Camera::default()
    }
}

/// The choice this tick's keys make on the current screen, if any.
pub fn choice_from(input: &Input, game: &Game) -> Option<Choice> {
    if input.just_pressed(Key::Enter) && matches!(game.screen, Screen::Dawn | Screen::End { .. }) {
        return Some(Choice::Continue);
    }
    let count = game.option_count();
    DIGITS
        .iter()
        .take(count)
        .position(|key| input.just_pressed(*key))
        .map(Choice::Pick)
}

/// The only place input is read: one choice a tick, applied by `step`.
fn play(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let Some(choice) = choice_from(input, world.resource::<Game>()) else {
        return;
    };
    if matches!(world.resource::<Game>().screen, Screen::End { .. }) {
        let order = rules::rotation(world.resource_mut::<Rng>());
        world.insert_resource(Game::new(order));
        return;
    }
    let _ = step(world.resource_mut::<Game>(), choice);
}

/// The bar's fill width for a sanity value.
pub fn bar_fill(sanity: i32) -> f32 {
    BAR_W * sanity.clamp(0, rules::START_SANITY as i32) as f32 / rules::START_SANITY as f32
}

/// Draw the screen's panel, then the sanity bar beside it.
fn draw_screen(ctx: &mut DrawCtx) {
    let Some(game) = ctx.world.find_resource::<Game>() else {
        return;
    };
    let panel = screen::screen(game);
    let sanity = game.sanity;
    panel.draw(ctx, &Identity, |icon, _| match icon.art {});
    ctx.rect(
        Rect::from_min_size(BAR_AT, Vec2::new(BAR_W, BAR_H)),
        palette::DIM,
        Depth::layer(layers::BAR),
    );
    let fill = if sanity < LOW_SANITY {
        palette::EMBER
    } else {
        palette::SEA
    };
    ctx.rect(
        Rect::from_min_size(BAR_AT, Vec2::new(bar_fill(sanity), BAR_H)),
        fill,
        Depth {
            layer: layers::BAR,
            z: 1.0,
        },
    );
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
