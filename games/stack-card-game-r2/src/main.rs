//! Stack duel — a two-player card game fought entirely over one shared stack.
//!
//! You are the Weaver; the Brute is a scripted NPC. Every card goes onto the
//! stack, both sides may answer, and it resolves last in, first out unless a
//! card changes that. `DESIGN.md` beside this crate has the rules and the pool.
//!
//! Run it:   `cargo run -p stack_card_game_r2`
//! Check it: `python3 tools/verify stack_card_game_r2`

#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod cards;
mod checks;
mod duel;
mod npc;
mod scenarios;
mod screen;
mod verify;

use duel::{Duel, Side};
use npc::Action;
use screen::{Pick, Spot, Ui};

/// Ticks the Brute waits before each action, so a person sees each one land.
pub(crate) const NPC_BEAT: u32 = 30;
/// Ticks before a player with nothing playable passes on their own.
pub(crate) const AUTO_PASS_BEAT: u32 = 20;

/// The match and everything the screen needs beside it.
pub(crate) struct Game {
    pub(crate) duel: Duel,
    pub(crate) ui: Ui,
    /// Ticks the side holding priority has been waiting.
    pub(crate) waited: u32,
    /// How many matches this session has started, for the next seed.
    pub(crate) matches: u64,
    /// The session's seed, from `GameConfig::seed` through the engine's `Rng`.
    pub(crate) seed: u64,
}
impl Resource for Game {}

impl Game {
    pub(crate) fn new(seed: u64) -> Game {
        Game {
            duel: Duel::new(seed),
            ui: Ui::default(),
            waited: 0,
            matches: 1,
            seed,
        }
    }
}

pub(crate) fn config() -> GameConfig {
    GameConfig {
        title: "stack duel",
        window_size: screen::WINDOW,
        ..GameConfig::default()
    }
}

/// Every system, in one place, so the verify run plays the game the window does.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, set_the_table);
    // The player first, then the Brute: a key pressed on the tick the Brute's
    // beat runs out is the player's, because priority is checked again after.
    app.add_system(Update, player_acts);
    app.add_system(Update, npc_acts);
    app.add_system(Update, next_match);
    app.add_system(Draw, draw_the_table);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("1-6 pick a card, up/down aim, enter plays, space passes. close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn set_the_table(world: &mut World) {
    world.insert_resource(screen::camera());
    // A verify run may have staged a match before the first tick.
    if world.find_resource::<Game>().is_none() {
        let seed = u64::from(world.resource_mut::<Rng>().next_u32());
        world.insert_resource(Game::new(seed));
    }
}

/// What the player's input asks for this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Slot(usize),
    Up,
    Down,
    Commit,
    Back,
    Pass,
    Tap(Spot),
}

const SLOT_KEYS: [Key; screen::HAND_SLOTS] = [
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
];

fn read_ask(input: &Input, duel: &Duel) -> Option<Ask> {
    for (slot, key) in SLOT_KEYS.iter().enumerate() {
        if input.just_pressed(*key) {
            return Some(Ask::Slot(slot));
        }
    }
    let keys = [
        (Key::ArrowUp, Ask::Up),
        (Key::ArrowDown, Ask::Down),
        (Key::Enter, Ask::Commit),
        (Key::Escape, Ask::Back),
        (Key::Space, Ask::Pass),
    ];
    if let Some((_, ask)) = keys.iter().find(|(key, _)| input.just_pressed(*key)) {
        return Some(*ask);
    }
    let pointer = input.pointer();
    if pointer.just_pressed(PointerButton::Primary) {
        let at = screen::camera().screen_to_world(pointer.screen);
        return screen::spot_at(duel, at).map(Ask::Tap);
    }
    None
}

/// Turn the player's keys and taps into plays and passes.
fn player_acts(world: &mut World) {
    let ask = match world.find_resource::<Input>() {
        Some(input) => read_ask(input, &world.resource::<Game>().duel),
        None => None,
    };
    let game = world.resource_mut::<Game>();
    if game.duel.over.is_some() {
        return;
    }
    let mine = game.duel.priority == Side::You;
    if let Some(ask) = ask {
        game.ui.notice = None;
        handle(game, ask);
        game.waited = 0;
        return;
    }
    // Nothing asked: a player with no possible play passes after a beat.
    if mine && game.ui.pick.is_none() && !game.duel.has_play(Side::You) {
        game.waited += 1;
        if game.waited >= AUTO_PASS_BEAT {
            game.waited = 0;
            refuse(game, Side::You, Action::Pass);
        }
    }
}

fn handle(game: &mut Game, ask: Ask) {
    let duel = &game.duel;
    match (ask, game.ui.pick) {
        (Ask::Slot(slot), Some(pick)) if pick.slot == slot => commit(game),
        (Ask::Slot(slot), _) | (Ask::Tap(Spot::Card(slot)), None) => {
            if slot >= duel.seat(Side::You).hand.len() {
                game.ui.notice = Some("no card in that slot".to_owned());
            } else {
                game.ui.pick = Some(Pick { slot, aim: 0 });
            }
        }
        (Ask::Tap(Spot::Card(slot)), Some(pick)) if pick.slot == slot => commit(game),
        (Ask::Tap(Spot::Card(slot)), Some(_)) => {
            game.ui.pick = Some(Pick { slot, aim: 0 });
        }
        (Ask::Up, Some(pick)) => {
            game.ui.pick = Some(Pick {
                aim: pick.aim.saturating_sub(1),
                ..pick
            });
        }
        (Ask::Down, Some(pick)) => {
            let last = screen::marked(duel, &game.ui).len().saturating_sub(1);
            game.ui.pick = Some(Pick {
                aim: (pick.aim + 1).min(last),
                ..pick
            });
        }
        (Ask::Tap(Spot::StackRow(row)), Some(pick)) => {
            let id = duel.stack.iter().rev().nth(row).map(|item| item.id);
            let marks = screen::marked(duel, &game.ui);
            match marks.iter().position(|mark| Some(*mark) == id) {
                Some(aim) => {
                    game.ui.pick = Some(Pick { aim, ..pick });
                    commit(game);
                }
                None => game.ui.notice = Some("that item is not a legal target".to_owned()),
            }
        }
        (Ask::Commit, Some(_)) => commit(game),
        (Ask::Back, _) => game.ui.pick = None,
        (Ask::Pass | Ask::Tap(Spot::Pass), _) => {
            game.ui.pick = None;
            refuse(game, Side::You, Action::Pass);
        }
        (Ask::Up | Ask::Down | Ask::Commit | Ask::Tap(Spot::StackRow(_)), None) => {}
        (Ask::Tap(Spot::Result), _) => {}
    }
}

/// Play the picked card at the aimed target.
fn commit(game: &mut Game) {
    let Some(pick) = game.ui.pick else {
        return;
    };
    let target = screen::aimed(&game.duel, &game.ui);
    game.ui.pick = None;
    refuse(
        game,
        Side::You,
        Action::Play {
            slot: pick.slot,
            target,
        },
    );
}

/// Do `action` for `side`, and put any refusal on screen rather than drop it.
pub(crate) fn refuse(game: &mut Game, side: Side, action: Action) {
    let done = match action {
        Action::Pass => game.duel.pass(side),
        Action::Play { slot, target } => game.duel.play(side, slot, target).map(|_| ()),
    };
    if let Err(refusal) = done {
        game.ui.notice = Some(refusal.line());
    }
}

/// The Brute acts when it holds priority, one beat after it got it.
fn npc_acts(world: &mut World) {
    let game = world.resource_mut::<Game>();
    if game.duel.over.is_some() || game.duel.priority != Side::Npc {
        return;
    }
    game.waited += 1;
    if game.waited < NPC_BEAT {
        return;
    }
    game.waited = 0;
    let action = npc::choose(&game.duel, Side::Npc);
    refuse(game, Side::Npc, action);
}

/// R, or a tap on the result, starts the next match from the next seed.
fn next_match(world: &mut World) {
    let again = world.find_resource::<Input>().is_some_and(|input| {
        input.just_pressed(Key::R)
            || (input.pointer().just_pressed(PointerButton::Primary)
                && screen::RESULT_BOX
                    .contains(screen::camera().screen_to_world(input.pointer().screen)))
    });
    let game = world.resource_mut::<Game>();
    if game.duel.over.is_none() || !again {
        return;
    }
    game.matches += 1;
    let seed = game.seed.wrapping_add(game.matches);
    game.duel = Duel::new(seed);
    game.ui = Ui::default();
    game.waited = 0;
}

fn draw_the_table(ctx: &mut DrawCtx) {
    let game = ctx.world.resource::<Game>();
    screen::draw(ctx, &game.duel, &game.ui);
}
