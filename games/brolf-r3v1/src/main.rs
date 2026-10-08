//! Brolf — golf as a battle royale, against five NPC golfers.
//!
//! Walk to your ball, aim, shoot; club rivals and strike their balls; take
//! equipment; stay — you *and* your ball — inside the shrinking zone; and
//! leave by extracting, by winning, or by being knocked out. DESIGN.md in
//! `tools/yakin/runs/brolf-r3v1/` is the design this builds.
//!
//! Run it:   `cargo run -p brolf_r3v1`
//! Check it: `tools/verify brolf_r3v1`
//! On the web: `tools/build-web brolf_r3v1 && tools/serve-web brolf_r3v1`
//!
//! The whole match is one `Match` resource and every rule is a free function
//! over it (`sim.rs`, `rules.rs`, `zone.rs`, `npc.rs`), so the `--verify`
//! players roll the same game forward that the window plays.

#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod draw;
mod events;
mod npc;
mod players;
mod rules;
mod scenes;
mod sim;
mod text;
mod verify;
mod zone;

use events::Intent;
use sim::{Brain, Match};

/// The window, and the one shape every layout number derives from.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
/// Half the camera's height, in world units.
pub const HALF_H: f32 = 18.0;
/// Half the camera's width, from the window's own aspect.
pub const HALF_W: f32 = HALF_H * WINDOW.aspect();

/// The seed a fresh window starts from; `--verify` names its own.
pub const FIRST_SEED: u64 = 7;

/// Draw bands, stated once.
pub mod layers {
    /// The course and its markings.
    pub const COURSE: i16 = 0;
    /// Cups, pads and pickups.
    pub const FIXTURES: i16 = 1;
    /// The zone rings, over the fixtures so they read through them.
    pub const ZONE: i16 = 2;
    /// Balls and golfers.
    pub const PLAY: i16 = 3;
    /// The aim, the cue and labels over the play.
    pub const GUIDES: i16 = 4;
    /// The status bar and the hint line.
    pub const HUD: i16 = 5;
    /// The result card, over everything.
    pub const RESULT: i16 = 6;
}

/// What the player asked for this tick, read from the keyboard.
#[derive(Clone, Copy, Default)]
pub struct PlayerIntent(pub Intent);
impl Resource for PlayerIntent {}

impl Resource for Match {}

/// The window's configuration, shared with `--verify`.
pub fn config() -> GameConfig {
    GameConfig {
        title: "brolf",
        seed: FIRST_SEED,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// The camera, one value for the game and for every check.
pub fn camera() -> Camera {
    Camera {
        center: Vec2::ZERO,
        height: HALF_H * 2.0,
        clear_color: draw::palette::ROUGH,
        viewport: WINDOW,
    }
}

/// Every system, in run order. `read_the_player` comes before
/// `step_the_match`: the tick acts on this tick's keys (verify asserts it).
pub fn register(app: &mut App) {
    app.add_system(Startup, set_up_the_match);
    app.add_system(Update, read_the_player);
    app.add_system(Update, step_the_match);
    app.add_system(Update, restart_on_enter);
    app.add_system(Draw, draw::draw_the_course);
    app.add_system(Draw, draw::draw_the_play);
    app.add_system(Draw, draw::draw_the_guides);
    app.add_system(Draw, draw::draw_the_hud);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!(
        "WASD walk  arrows aim/power  SPACE shoot  F club/strike  E take  X extract  ENTER next match"
    );
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// The camera and the first match — unless a check put one in already.
fn set_up_the_match(world: &mut World) {
    world.insert_resource(camera());
    if world.find_resource::<Match>().is_none() {
        world.insert_resource(Match::new(FIRST_SEED));
    }
    world.insert_resource(PlayerIntent::default());
}

/// The keyboard, as an intent. Held keys steer; edges commit.
pub fn intent_from(input: &Input) -> Intent {
    let axis = |minus: Key, plus: Key| f32::from(input.held(plus)) - f32::from(input.held(minus));
    Intent {
        walk: Vec2::new(axis(Key::A, Key::D), axis(Key::W, Key::S)),
        turn: axis(Key::ArrowLeft, Key::ArrowRight),
        power: axis(Key::ArrowDown, Key::ArrowUp),
        shoot: input.just_pressed(Key::Space),
        contact: input.just_pressed(Key::F),
        take: input.just_pressed(Key::E),
        extract: input.just_pressed(Key::X),
    }
}

fn read_the_player(world: &mut World) {
    let intent = world
        .find_resource::<Input>()
        .map(intent_from)
        .unwrap_or_default();
    world.insert_resource(PlayerIntent(intent));
}

/// Everyone's intents for this tick: the player's from `player`, every NPC's
/// from its brain.
pub fn intents(game: &Match, player: Intent) -> Vec<Intent> {
    (0..game.golfers.len())
        .map(|who| match game.golfers[who].brain {
            Brain::Player => player,
            _ => npc::npc_intent(game, who, 1.0),
        })
        .collect()
}

fn step_the_match(world: &mut World) {
    let player = world.resource::<PlayerIntent>().0;
    let all = intents(world.resource::<Match>(), player);
    sim::step(world.resource_mut::<Match>(), &all);
}

/// Enter on the result screen starts the next match.
fn restart_on_enter(world: &mut World) {
    let pressed = world
        .find_resource::<Input>()
        .is_some_and(|input| input.just_pressed(Key::Enter));
    let game = world.resource::<Match>();
    if pressed && game.over() {
        let next = game.seed + 1;
        world.insert_resource(Match::new(next));
    }
}
