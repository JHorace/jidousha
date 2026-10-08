//! Call of Cthulhu: the Old Ones ring your telephone, and every exchange on
//! the line costs sanity. Learn their lore by day so you can answer them in
//! the ways that end the call soonest; never anger them.
//!
//! A text game on `jidousha::ui`. The rules are pure functions (`rules`), the
//! run is a state machine (`play`), the screens are panels built from it
//! (`screen`), and the two systems here only turn input into a command and a
//! panel into quads. `--verify` plays it headless (`verify`). The design note
//! is `DESIGN.md` beside this crate.

#![allow(missing_docs)]

mod capture;
mod checks;
mod lore;
mod play;
mod players;
mod rules;
mod screen;
mod verify;

use std::process::ExitCode;

use jidousha::prelude::*;

use crate::play::{Command, Run};
use crate::screen::{UiMap, WINDOW, camera, layout};

/// The run, as the world holds it.
pub(crate) struct Game(pub(crate) Run);
impl Resource for Game {}

/// The seed a windowed run plays.
const SEED: u64 = 1926;

pub(crate) fn config(seed: u64) -> GameConfig {
    GameConfig {
        title: "Call of Cthulhu",
        seed,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// Every system, in the order they run.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, take_the_call);
    app.add_system(Draw, draw_the_screen);
}

/// The camera, and a run on the config's seed — unless a check put one here.
fn set_the_scene(world: &mut World) {
    world.insert_resource(camera());
    if world.find_resource::<Game>().is_none() {
        let seed = world.resource::<Rng>().clone().next_u32();
        world.insert_resource(Game(Run::new(u64::from(seed))));
    }
}

/// The digit keys, in row order.
const DIGITS: [Key; 6] = [
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
];

/// What this tick's input asks the run to do, if anything.
///
/// A tap on a numbered row chooses it; a tap anywhere on a screen with no rows
/// goes on. The rows are the ones the screen drew (`Layout::choices`).
pub(crate) fn command_from(
    input: &Input,
    run: &Run,
    map: &UiMap,
    camera: &Camera,
) -> Option<Command> {
    if let Some(row) = DIGITS.iter().position(|key| input.just_pressed(*key)) {
        return Some(Command::Choose(row));
    }
    if input.just_pressed(Key::Space) || input.just_pressed(Key::Enter) {
        return Some(Command::Continue);
    }
    if input.pointer().just_pressed(PointerButton::Primary) {
        let at = map.to_design(camera.screen_to_world(input.pointer().screen));
        let choices = layout(run).choices;
        if choices.is_empty() {
            return Some(Command::Continue);
        }
        return choices
            .iter()
            .position(|row| row.contains(at))
            .map(Command::Choose);
    }
    None
}

fn take_the_call(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let camera = *world.resource::<Camera>();
    let map = UiMap::for_camera(&camera);
    let command = command_from(input, &world.resource::<Game>().0, &map, &camera);
    if let Some(command) = command {
        world.resource_mut::<Game>().0.step(command);
    }
}

fn draw_the_screen(ctx: &mut DrawCtx) {
    let map = UiMap::for_camera(ctx.world.resource::<Camera>());
    let panel = layout(&ctx.world.resource::<Game>().0).panel;
    panel.draw(ctx, &map, |icon, _| match icon.art {});
}

fn main() -> ExitCode {
    if std::env::args().any(|arg| arg == "--verify") {
        return verify::run();
    }
    match run(config(SEED), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
