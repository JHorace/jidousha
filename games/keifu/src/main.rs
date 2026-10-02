//! Keifu (系譜): a port of Lineage to Jidousha. Session 1 built modules W0 and W1;
//! session 2 built W2 and gave the cast its sprites; session 3 built W3; session 4
//! built W4; session 5 built W5; session 6 built W6; session 7 builds W7.
//!
//! W0 is the foundation: the content in `spec/content/` loaded and validated, the
//! lore tables, the calendar and the Door countdown, the randomness primitives
//! and the text conventions. W1 is the household: the hero model and what derives
//! from it, the founding household in creation order, the roster, the hero card
//! and sheet, and the family screen's membership. W2 is bonds, fears, grief and
//! destinies as state and pure rules (`bonds`, `fear`, `grief`, `destiny`), with
//! the power sum and fear line its oracle reads (`power`). W3 is dreams and
//! legacies: moments and their predicates (`moment`), witnessing and fulfilment
//! (`witness`), dream calls (`calls`), legacies and the heir of the blood
//! (`legacy`, `blessing`), and dream rivals (`rivals`). W4 is quests and the
//! forecast: the quest model and its stakes (`quest`), the board's seats and the
//! rules a drop obeys (`board`), the 36-pair forecast (`forecast`), the power
//! breakdown line by line (`power_lines`), the quest card and sheet as information
//! (`quest_card`, `quest_sheet`, drawn by `board_view`), and the drag (`pointer`).
//! W5 is board generation: planning, the reading of likely parties over ordered
//! pairs, the score and the sixteen-attempt loop with its welcome rule, easing,
//! template memory and place order (`generation`, `reading`, `easing`), and the
//! first ghost's quest (`ghost`). W6 is the summer resolved and told: set out
//! (`resolve`), every step of a quest in order — the reward (`reward`), wounds,
//! deaths, mending, burning and crowning (`harm`), facing the fear (`facing`),
//! sharing the road (`road`), witnessing live, the ghost laid — the unanswered
//! costs and healing at home; the telling as data (`telling`) and as a screen with
//! its typewriter (`telling_view`); and leaving it (`season`), where the house can
//! close (`ending_view`, a W10 SCAFFOLD) or winter begins. W7 is the winter: the
//! hearth opened and seated (`hearth`), every seat's plan — the lesson with every
//! excuse, the courtship verdict, the rest, the teller's part (`plans`) — previewed on
//! the hearth screen (`hearth_view`) from the same plan the winter's resolution carries
//! out in §11.3's order, with the teacher's credit and the rank-limited mentor bond
//! (`winter`), and the winter's dream moments witnessed live. The turning after it is a
//! W8 SCAFFOLD (`season`, `passage`, `turning_view`): the winter page, then summer.
//!
//! What the player can do in this build: point at a hero to read their sheet,
//! point at a quest to read its sheet and its place's history — both open in the
//! sheet dock down the right edge (`dock`), which scrolls a long sheet and covers
//! nothing — drag heroes onto quests and watch the card's odds move while they are
//! held, set out (or stay home) and read the telling page by page, and open the
//! family. Leaving the telling opens the hearth: drag anyone to any winter seat and
//! read its preview, point at a group for its help, let the winter pass and read what
//! it did, and summer comes.
//!
//! The spec (`spec/SPEC.md`, `spec/CONSTANTS.md`, `spec/content/`) is the only
//! source of game behaviour; `SPEC-GAPS.md` lists every place it fell silent.
//!
//! Run it: `cargo run -p keifu` (`-- --seed N` fixes the run's seed)
//! Check it: `tools/verify keifu`

#![allow(missing_docs)]

mod art;
mod births;
mod blessing;
mod board;
mod board_view;
mod bonds;
mod calendar;
mod calls;
mod capture;
mod cast;
mod chance;
mod checks;
mod coming_of_age;
mod constants;
mod content;
mod death_page;
mod destiny;
mod dock;
mod dock_checks;
mod dock_lines;
mod dream;
mod dream_lore;
mod easing;
mod ending_view;
mod facing;
mod family;
mod fear;
mod floors;
mod floors_w5;
mod floors_w6;
mod floors_w7;
mod forecast;
mod foundations;
mod generation;
mod ghost;
mod grief;
mod harm;
#[cfg(test)]
mod harm_tests;
mod hearth;
mod hearth_help;
mod hearth_view;
mod heirs;
mod hero;
mod house;
mod household;
mod ids;
mod json;
mod legacy;
mod legacy_lore;
mod lore;
mod moment;
mod newcomers;
mod oracles;
mod passage;
mod plans;
#[cfg(test)]
mod plans_tests;
mod play;
mod pointer;
mod power;
mod power_lines;
mod quest;
mod quest_card;
mod quest_sheet;
mod reading;
mod resolve;
#[cfg(test)]
mod resolve_tests;
mod reward;
mod rivals;
mod road;
#[cfg(test)]
mod road_tests;
mod screen;
mod scripted;
mod season;
mod sessions;
mod sheet;
mod summer;
mod telling;
#[cfg(test)]
mod telling_tests;
mod telling_view;
#[cfg(test)]
mod testkit;
mod text;
mod tree;
mod turning;
mod turning_lore;
mod turning_view;
mod verify;
mod w2;
mod w3;
mod w4;
mod w4_rules;
mod w5;
mod w5_shape;
mod w6;
mod w6_battery;
mod w6_stages;
mod w7;
mod w7_battery;
mod w7_controls;
mod wanderer;
mod winter;
#[cfg(test)]
mod winter_tests;
mod witness;
mod words;

use std::process::ExitCode;

use jidousha::prelude::*;

use crate::content::Content;
use crate::house::{House, RunSeed, draw_seed};
use crate::screen::{Page, UiState, camera, page};

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
    app.add_system(Update, fit_the_camera);
    app.add_system(Update, pointer::follow_the_pointer);
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
    let house = match House::found(&content, seed, world.resource_mut::<Rng>()) {
        Ok(house) => house,
        Err(error) => panic!(
            "[keifu] the founding household could not be built\n  {error}\n  likely cause: \
             household.json names a hero it does not define\n  fix: compare the file with \
             spec/content/README.md"
        ),
    };
    world.insert_resource(house);
    world.insert_resource(content);
    // Only if nothing has installed a store already: a verify run puts a scripted
    // one in before Startup, so when the art arrives is part of its script.
    if world.find_resource::<Assets>().is_none() {
        world.insert_resource(art::store());
    }
    let art = art::Art::load(world.resource_mut::<Assets>());
    world.insert_resource(art);
    world.insert_resource(UiState::default());
    world.insert_resource(camera());
}

/// Keep the whole page in view at the window's shape (`screen::fitted`). The
/// viewport is the one the driver stamped last frame, so a resize is followed from
/// the frame after it arrives.
fn fit_the_camera(world: &mut World) {
    let camera = world.resource_mut::<Camera>();
    *camera = screen::fitted(camera.viewport);
}

/// The one reader both phases use: the page for the current state, at the frame clock.
pub fn read_the_page(world: &WorldView<'_>) -> Page {
    let time = world.resource::<Time>();
    let clock = screen::Clock {
        tick: time.tick,
        dt: time.fixed_dt.0,
    };
    page(
        world.resource::<Content>(),
        world.resource::<House>(),
        world.resource::<UiState>(),
        clock,
    )
}

/// Submit the page.
fn draw_the_page(ctx: &mut DrawCtx) {
    let page = read_the_page(&ctx.world);
    let sprites: Vec<(Transform, Sprite)> = {
        let art = ctx.world.resource::<art::Art>();
        page.figures
            .iter()
            .map(|mark| {
                let sprite = Sprite {
                    size: mark.rect.size(),
                    tint: mark.tint,
                    layer: mark.layer,
                    ..Sprite::new(art.texture(mark.figure))
                };
                (Transform::at(mark.rect.center()), sprite)
            })
            .collect()
    };
    for (transform, sprite) in &sprites {
        ctx.sprite(transform, sprite);
    }
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
