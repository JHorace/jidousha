//! Call of Cthulhu — eldritch beings phone you at night; hang up before your
//! sanity runs out.
//!
//! A text game. Mornings are spent preparing (train, study a being's lore,
//! disrupt a cult); nights are calls, answered with 1/2/3 or a tap. The rules
//! are `DESIGN.md` beside this crate.
//!
//! Run it: `cargo run -p call_of_cthulhu_r2`
//! Check it: `python3 tools/verify call_of_cthulhu_r2`

#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod conductor;
mod decisions;
mod lore;
mod play;
mod players;
mod rule_checks;
mod rules;
mod screen_checks;
mod screens;
mod verify;

use play::Game;
use screens::{DESIGN_H, DESIGN_W, Flat, Pick, WINDOW, layers, palette};

/// The seed a run starts from unless a check says otherwise.
pub const SEED: u64 = 1890;

/// The seed for this run: a check inserts one before tick 1, the window does
/// not and gets `SEED`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunSeed(pub u64);
impl Resource for RunSeed {}

/// The game's configuration, shared by the window and the verify run.
pub fn config() -> GameConfig {
    GameConfig {
        title: "Call of Cthulhu",
        seed: SEED,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// The camera: the design rect, exactly.
pub fn camera() -> Camera {
    Camera {
        center: Vec2::new(DESIGN_W * 0.5, DESIGN_H * 0.5),
        height: DESIGN_H,
        clear_color: palette::NIGHT,
        viewport: WINDOW,
    }
}

/// Every system, in one place, so the verify run plays the game the window
/// does.
pub fn register(app: &mut App) {
    app.add_system(Startup, set_up);
    // The answer is taken before the silence is counted, so an answer on the
    // tick the silence would have cost resets it rather than paying it.
    app.add_system(Update, take_input);
    app.add_system(Update, count_the_silence);
    app.add_system(Draw, draw_the_screen);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("1-7 choose, Enter goes on; or tap. Close the window to quit.");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn set_up(world: &mut World) {
    let seed = world
        .find_resource::<RunSeed>()
        .copied()
        .unwrap_or(RunSeed(SEED));
    world.insert_resource(seed);
    world.insert_resource(camera());
    world.insert_resource(Game::new(seed.0));
}

/// The digit keys, in the order they number a screen's rows.
const DIGITS: [Key; 9] = [
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
    Key::Digit7,
    Key::Digit8,
    Key::Digit9,
];

/// What this tick's input picks on the screen the game is showing, if anything.
pub fn picked(game: &Game, input: &Input, camera: &Camera) -> Option<Pick> {
    let digit = DIGITS.iter().position(|key| input.just_pressed(*key));
    let go_on = input.just_pressed(Key::Enter) || input.just_pressed(Key::Space);
    let pointer = input.pointer();
    let tap = pointer
        .just_pressed(PointerButton::Primary)
        .then(|| camera.screen_to_world(pointer.screen));
    let rows = screens::choices(game);
    if let Some(at) = tap
        && let Some((_, pick)) = rows.iter().find(|(rect, _)| rect.contains(at))
    {
        return Some(*pick);
    }
    rows.iter().map(|(_, pick)| *pick).find(|pick| match pick {
        Pick::Morning(index) | Pick::Answer(index) => digit == Some(*index),
        Pick::GoOn => go_on,
    })
}

/// Apply one pick to the game.
pub fn apply(game: &mut Game, pick: Pick) {
    match pick {
        Pick::Morning(index) => game.choose_morning(index),
        Pick::Answer(index) => game.answer(index),
        Pick::GoOn => game.go_on(),
    }
}

fn take_input(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let camera = world.resource::<Camera>();
    let Some(pick) = picked(world.resource::<Game>(), input, camera) else {
        return;
    };
    apply(world.resource_mut::<Game>(), pick);
}

fn count_the_silence(world: &mut World) {
    world.resource_mut::<Game>().listen();
}

fn draw_the_screen(ctx: &mut DrawCtx) {
    let game = ctx.world.resource::<Game>();
    if !matches!(game.phase, play::Phase::Ended(_)) {
        let (whole, full) = screens::sanity_bar(game.run.sanity);
        ctx.rect(whole, palette::BAR_EMPTY, Depth::layer(layers::BAR));
        ctx.rect(
            full,
            palette::BAR_FULL,
            Depth {
                layer: layers::BAR,
                z: 1.0,
            },
        );
    }
    screens::screen(game).draw(ctx, &Flat, |icon, _| match icon.art {});
}
