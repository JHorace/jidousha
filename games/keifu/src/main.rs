//! Keifu (系譜): a port of Lineage to Jidousha. Session 1 builds modules W0 and W1.
//!
//! W0 is the foundation: the content in `spec/content/` loaded and validated, the
//! lore tables, the calendar and the Door countdown, the randomness primitives
//! and the text conventions. W1 is the household: the hero model and what derives
//! from it, the founding household in creation order, the roster, the hero card
//! and sheet, and the family screen's membership.
//!
//! What the player can do in this build: point at a hero to read their sheet, and
//! open the family to see everyone who has lived. Nothing advances the year yet.
//!
//! The spec (`spec/SPEC.md`, `spec/CONSTANTS.md`, `spec/content/`) is the only
//! source of game behaviour; `SPEC-GAPS.md` lists every place it fell silent.
//!
//! Run it: `cargo run -p keifu` (`-- --seed N` fixes the run's seed)
//! Check it: `tools/verify keifu`

#![allow(missing_docs)]

mod calendar;
mod capture;
mod chance;
mod checks;
mod constants;
mod content;
mod dream;
mod family;
mod floors;
mod foundations;
mod hero;
mod house;
mod household;
mod ids;
mod json;
mod lore;
mod oracles;
mod screen;
mod sessions;
mod sheet;
mod summer;
mod text;
mod tree;
mod verify;
mod words;

use std::process::ExitCode;

use jidousha::prelude::*;

use crate::content::Content;
use crate::house::{House, RunSeed, draw_seed};
use crate::screen::{Page, Target, UiState, camera, page};

/// The engine seed a windowed run starts from when no `--seed` is given.
///
/// The run's own seed is drawn from it at founding and recorded on the house,
/// exactly as a new house reseeds from a draw of the previous generator (SPEC
/// §22.1). The original seeds from the clock; this port never reads one.
const DEFAULT_SEED: u64 = 0x6b65_6966_7531;

impl Resource for Content {}

/// The window and the run.
pub fn config(seed: u64) -> GameConfig {
    GameConfig {
        title: "Keifu",
        seed,
        window_size: screen::WINDOW,
        ..GameConfig::default()
    }
}

/// Every system, in order. The windowed run and every verify session build this.
pub fn register(app: &mut App) {
    app.add_system(Startup, found_the_house);
    app.add_system(Update, follow_the_pointer);
    app.add_system(Draw, draw_the_page);
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    // The content is checked before a window opens, so a schema mismatch is a
    // message on the terminal rather than a panic inside the first frame.
    if let Err(error) = content::load() {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    if args.iter().any(|arg| arg == "--verify") {
        return verify::run();
    }
    let seed = match args.iter().position(|arg| arg == "--seed") {
        None => DEFAULT_SEED,
        Some(at) => match args.get(at + 1).and_then(|n| n.parse::<u64>().ok()) {
            Some(seed) => seed,
            None => {
                eprintln!(
                    "[keifu] --seed needs a number\n  got {:?}\n  likely cause: a typo on \
                     the command line\n  fix: `cargo run -p keifu -- --seed 42`",
                    args.get(at + 1)
                );
                return ExitCode::FAILURE;
            }
        },
    };
    match run(config(seed), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Load the content, found the house on the run's seed, and set the scene.
fn found_the_house(world: &mut World) {
    let content = match content::load() {
        Ok(content) => content,
        // `main` loaded the same bytes before the window opened, so this is
        // reachable only if the baked-in files changed between two reads of one
        // binary — which they cannot. Loud rather than silent all the same.
        Err(error) => panic!("{error}"),
    };
    let seed = match world.find_resource::<RunSeed>() {
        Some(seed) => seed.0,
        None => draw_seed(world.resource_mut::<Rng>()),
    };
    world.insert_resource(Rng::from_seed(seed));
    let house = match House::found(&content, seed) {
        Ok(house) => house,
        Err(error) => panic!(
            "[keifu] the founding household could not be built\n  {error}\n  likely cause: \
             household.json names a hero it does not define\n  fix: compare the file with \
             spec/content/README.md"
        ),
    };
    world.insert_resource(house);
    world.insert_resource(content);
    world.insert_resource(UiState::default());
    world.insert_resource(camera());
}

/// The one reader both phases use: the page for the current state.
pub fn read_the_page(world: &WorldView<'_>) -> Page {
    page(
        world.resource::<Content>(),
        world.resource::<House>(),
        world.resource::<UiState>(),
    )
}

/// Point at a hero to read them; click a button to open or close the family.
fn follow_the_pointer(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let pointer = input.pointer();
    let clicked = pointer.just_pressed(PointerButton::Primary);
    let at = world.resource::<Camera>().screen_to_world(pointer.screen);
    let target = read_the_page(&world.view()).target_at(at);
    let ui = world.resource_mut::<UiState>();
    ui.pointing = match target {
        Some(Target::Hero(id)) => Some(id),
        _ => None,
    };
    if clicked {
        match target {
            Some(Target::OpenFamily) => {
                *ui = UiState {
                    family_open: true,
                    pointing: None,
                }
            }
            Some(Target::CloseFamily) => *ui = UiState::default(),
            _ => {}
        }
    }
}

/// Submit the page.
fn draw_the_page(ctx: &mut DrawCtx) {
    let page = read_the_page(&ctx.world);
    for shape in &page.shapes {
        ctx.rect(shape.rect, shape.color, Depth::layer(shape.layer));
    }
    let (thickness, color, depth) = tree::link_style();
    for (from, to) in &page.links {
        ctx.line(*from, *to, thickness, color, depth);
    }
    for row in &page.rows {
        ctx.text(row.at, &row.text, row.style);
    }
}
