//! Stack card game — a two-player card duel fought entirely over the stack.
//!
//! You against a scripted Rival. Every card played goes onto one shared stack
//! that resolves top-first only when both seats pass in a row, and half of each
//! deck acts on the stack itself: void an item, push it to the bottom, flip whom
//! it hits, copy it. The joy to protect is reading the stack and outsmarting it,
//! so the stack panel always says what every item will do and in what order.
//!
//! Run it: `cargo run -p stack_card_game_r3v2`
//! Check it: `tools/verify stack_card_game_r3v2`
//!
//! The design is `tools/yakin/runs/stack-card-game-r3v2/DESIGN.md`. This file
//! holds the one game resource, `Flow`, and the systems that move it; the rules
//! are `rules.rs` and `duel.rs`, the screen is `screen.rs`.

#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod cards;
mod checks;
mod decisions;
mod duel;
mod players;
mod rules;
mod screen;
mod sweep;
mod verify;

use cards::Card;
use duel::{Action, Duel, Phase};
use players::{RIVAL_THINK_TICKS, rival_action};
use rules::{Side, legal_targets};

/// The window, and the recorder's viewport: one shape, stated once.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);

/// The game's configuration, shared by the window and the verify run.
pub fn config() -> GameConfig {
    GameConfig {
        title: "stack card game",
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// Every system the game has, in run order. `verify` asserts the Update order.
pub fn register(app: &mut App) {
    app.add_system(Startup, set_the_table);
    // The player's keys first, then the Rival: a press on the tick priority
    // passes is read before the Rival's wait starts counting.
    app.add_system(Update, read_player_input);
    app.add_system(Update, let_the_rival_act);
    // Rects first, text second; the bands, not this order, put text over rects.
    app.add_system(Draw, screen::draw_table);
    app.add_system(Draw, screen::draw_chrome);
}

/// Aiming a stack card: which hand slot, and which legal target the cursor is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choosing {
    /// The hand slot being played.
    pub slot: usize,
    /// An index into `legal_targets(card, stack)`, top-first.
    pub cursor: usize,
}

/// The whole game state: the match, the aiming state, the Rival's wait.
#[derive(Clone, Debug, PartialEq)]
pub struct Flow {
    /// The match.
    pub duel: Duel,
    /// What is being aimed, if anything — one value, never a flag beside it.
    pub choosing: Option<Choosing>,
    /// Ticks the Rival has held priority so far.
    pub rival_wait: u32,
    /// What `Enter` on the result screen rebuilds the match from.
    pub seed: u64,
}
impl Resource for Flow {}

impl Flow {
    /// A fresh match from `seed`.
    pub fn new(seed: u64) -> Self {
        Flow {
            duel: Duel::new(&mut Rng::from_seed(seed)),
            choosing: None,
            rival_wait: 0,
            seed,
        }
    }

    /// The card being aimed and the targets it may take, while choosing.
    pub fn aiming(&self) -> Option<(Card, Vec<rules::ItemId>)> {
        let choosing = self.choosing?;
        let card = *self.duel.seat(Side::You).hand.get(choosing.slot)?;
        Some((card, legal_targets(card, &self.duel.stack)))
    }
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("1-6 play a card, Up/Down/Enter aim it, Esc cancels, Space passes");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Keep a `Flow` a harness staged before tick 1, or deal one from the seeded `Rng`.
fn set_the_table(world: &mut World) {
    world.insert_resource(Camera {
        center: Vec2::new(screen::DESIGN.x * 0.5, screen::DESIGN.y * 0.5),
        height: screen::DESIGN.y,
        clear_color: screen::palette::TABLE,
        ..Camera::default()
    });
    if world.find_resource::<Flow>().is_none() {
        let seed = u64::from(world.resource_mut::<Rng>().next_u32());
        world.insert_resource(Flow::new(seed));
    }
}

/// The digit row plays a hand slot.
const SLOT_KEYS: [Key; 6] = [
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
];

/// What the player's keys do this tick, as a change to the flow.
pub fn respond_to_keys(flow: &mut Flow, pressed: impl Fn(Key) -> bool) {
    if let Phase::Over(_) = flow.duel.phase {
        if pressed(Key::Enter) {
            *flow = Flow::new(flow.seed);
        }
        return;
    }
    if flow.duel.priority != Side::You {
        return;
    }
    if let Some((_, targets)) = flow.aiming() {
        let Some(choosing) = flow.choosing.as_mut() else {
            return;
        };
        let count = targets.len().max(1);
        if pressed(Key::Escape) {
            flow.choosing = None;
        } else if pressed(Key::ArrowDown) {
            choosing.cursor = (choosing.cursor + 1) % count;
        } else if pressed(Key::ArrowUp) {
            choosing.cursor = (choosing.cursor + count - 1) % count;
        } else if pressed(Key::Enter) {
            let play = Action::Play {
                slot: choosing.slot,
                target: targets.get(choosing.cursor).copied(),
            };
            if flow.duel.apply(play).is_ok() {
                flow.choosing = None;
            }
        }
        return;
    }
    flow.choosing = None;
    if pressed(Key::Space) {
        let _ = flow.duel.apply(Action::Pass);
        return;
    }
    let Some(slot) = SLOT_KEYS.iter().position(|&key| pressed(key)) else {
        return;
    };
    if !flow.duel.playable(Side::You, slot) {
        return;
    }
    if flow.duel.seat(Side::You).hand[slot].is_effect() {
        let _ = flow.duel.apply(Action::Play { slot, target: None });
    } else {
        flow.choosing = Some(Choosing { slot, cursor: 0 });
    }
}

/// Read this tick's keys into the flow; ignored while the Rival holds priority.
fn read_player_input(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let pressed: Vec<Key> = SLOT_KEYS
        .into_iter()
        .chain([
            Key::ArrowUp,
            Key::ArrowDown,
            Key::Enter,
            Key::Escape,
            Key::Space,
        ])
        .filter(|&key| input.just_pressed(key))
        .collect();
    if pressed.is_empty() {
        return;
    }
    let flow = world.resource_mut::<Flow>();
    respond_to_keys(flow, |key| pressed.contains(&key));
}

/// Count the Rival's wait in ticks, then let it act by its own rule.
pub fn rival_tick(flow: &mut Flow) {
    if flow.duel.phase != Phase::Live || flow.duel.priority != Side::Rival {
        flow.rival_wait = 0;
        return;
    }
    flow.rival_wait += 1;
    if flow.rival_wait < RIVAL_THINK_TICKS {
        return;
    }
    flow.rival_wait = 0;
    let action = rival_action(&flow.duel);
    if let Err(refused) = flow.duel.apply(action) {
        // CONTRACT: `rival_action` only names plays `Duel::playable` allows, so a
        // refusal is a bug in it: loud in a debug build, and in a release build a
        // pass, because a Rival that stalls would hang the match.
        debug_assert!(false, "the Rival chose {action:?}, refused: {refused}");
        let _ = flow.duel.apply(Action::Pass);
    }
}

fn let_the_rival_act(world: &mut World) {
    rival_tick(world.resource_mut::<Flow>());
}
