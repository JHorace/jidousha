//! Stack card game (r3v1): a two-player duel fought entirely over one shared
//! stack, against a scripted rival who plays the stack too.
//!
//! Every played card goes on the stack; both duellists respond; it resolves
//! last in, first out unless a card changes that. The rules are
//! `games/stack-card-game-r3v1/DESIGN.md`; the model is `rules.rs`, the screen
//! `screen.rs`, the rival `rival.rs`.
//!
//! Run it:   `cargo run -p stack_card_game_r3v1`
//! Check it: `tools/verify stack_card_game_r3v1`
//! Web:      `tools/build-web stack_card_game_r3v1`
//!
//! Key systems, in run order: `deal_the_match` (Startup), `rematch`,
//! `take_your_input`, `rival_acts` (Update), `draw_the_table` (Draw).

#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod cards;
mod checks;
mod players;
mod rival;
mod rules;
mod scenarios;
mod screen;
mod verify;

use rival::{Choice, best_choice};
use rules::{Card, Duel, ItemId, Side, deal, legal_targets, pass, play};

/// The window the layout is drawn for: one world unit is one design unit.
pub(crate) const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
/// The design space, in world units: 960 x 540, the window's 16:9.
pub(crate) const VIEW_H: f32 = 540.0;
pub(crate) const VIEW_W: f32 = VIEW_H * WINDOW.aspect();

/// Ticks the rival holds priority before it acts, so a person sees it think.
pub(crate) const RIVAL_THINK: u32 = 45;

/// Draw bands, named once.
pub(crate) mod layers {
    /// Panel backgrounds.
    pub const BACK: i16 = -1;
    /// Card and stack-row boxes.
    pub const CARDS: i16 = 0;
    /// Every row of text over the table.
    pub const TEXT: i16 = 1;
    /// Target marks, over the rows they mark.
    pub const MARKS: i16 = 2;
    /// The result screen's dimmer and box.
    pub const OVERLAY: i16 = 5;
    /// The result screen's words.
    pub const OVERLAY_TEXT: i16 = 6;
}

/// The colours, named once.
pub(crate) mod palette {
    use jidousha::prelude::Color;
    /// The table the duel is fought on.
    pub const TABLE: Color = Color::rgb(0.06, 0.07, 0.10);
    pub const PANEL: Color = Color::rgb(0.11, 0.12, 0.17);
    pub const ROW: Color = Color::rgb(0.17, 0.18, 0.25);
    pub const TOP_ROW: Color = Color::rgb(0.24, 0.24, 0.34);
    pub const CARD: Color = Color::rgb(0.20, 0.22, 0.30);
    pub const CARD_DIM: Color = Color::rgb(0.12, 0.13, 0.17);
    pub const CARD_AIMING: Color = Color::rgb(0.32, 0.30, 0.14);
    pub const BUTTON: Color = Color::rgb(0.22, 0.34, 0.30);
    pub const YOU: Color = Color::rgb(0.45, 0.90, 0.85);
    pub const RIVAL: Color = Color::rgb(1.0, 0.55, 0.35);
    pub const INK: Color = Color::rgb(0.92, 0.92, 0.95);
    pub const FAINT: Color = Color::rgb(0.62, 0.64, 0.72);
    /// The mark on a legal target.
    pub const MARK: Color = Color::rgb(1.0, 0.85, 0.20);
    pub const DIMMER: Color = Color::rgba(0.0, 0.0, 0.0, 0.72);
}

/// Everything the table holds: the duel, and the one piece of input state —
/// which hand card is waiting for its target, if any.
pub(crate) struct Table {
    pub(crate) duel: Duel,
    /// The hand card you picked that still needs a stack item named.
    pub(crate) choosing: Option<usize>,
    /// Ticks the rival has held priority without acting.
    pub(crate) rival_clock: u32,
}
impl Resource for Table {}

pub(crate) fn config() -> GameConfig {
    GameConfig {
        title: "jidousha - stack card game",
        seed: 7,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// The camera the game and every check share.
pub(crate) fn camera() -> Camera {
    Camera {
        center: Vec2::new(VIEW_W * 0.5, VIEW_H * 0.5),
        height: VIEW_H,
        clear_color: palette::TABLE,
        viewport: WINDOW,
    }
}

/// Every system, so the window and `--verify` run the same program.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, deal_the_match);
    app.add_system(Update, rematch);
    // Yours before the rival's: a key you press on the tick the rival would
    // act is read against the priority you held, never after its move.
    app.add_system(Update, take_your_input);
    app.add_system(Update, rival_acts);
    app.add_system(Draw, draw_the_table);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("1-7 play a card, A-F pick a target, SPACE pass. close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn deal_the_match(world: &mut World) {
    world.insert_resource(camera());
    let duel = deal(world.resource_mut::<Rng>());
    world.insert_resource(Table {
        duel,
        choosing: None,
        rival_clock: 0,
    });
}

/// Enter on the result screen deals a fresh match from the same generator.
fn rematch(world: &mut World) {
    let over = world.resource::<Table>().duel.outcome.is_some();
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    if !(over && input.just_pressed(Key::Enter)) {
        return;
    }
    let duel = deal(world.resource_mut::<Rng>());
    let table = world.resource_mut::<Table>();
    table.duel = duel;
    table.choosing = None;
    table.rival_clock = 0;
}

/// One thing the player asked for this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Intent {
    /// A hand card, by index.
    Card(usize),
    /// A stack slot, counted from the top (0 = A).
    Slot(usize),
    Pass,
    Cancel,
}

pub(crate) const CARD_KEYS: [Key; 7] = [
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
    Key::Digit7,
];
pub(crate) const SLOT_KEYS: [Key; 6] = [Key::A, Key::B, Key::C, Key::D, Key::E, Key::F];

/// What this tick's input asks for, keys first, then a click through the
/// shared hit-test (`screen::hit`).
pub(crate) fn intent(input: &Input, camera: &Camera) -> Option<Intent> {
    if let Some(index) = CARD_KEYS.iter().position(|&key| input.just_pressed(key)) {
        return Some(Intent::Card(index));
    }
    if let Some(slot) = SLOT_KEYS.iter().position(|&key| input.just_pressed(key)) {
        return Some(Intent::Slot(slot));
    }
    if input.just_pressed(Key::Space) {
        return Some(Intent::Pass);
    }
    if input.just_pressed(Key::Escape) {
        return Some(Intent::Cancel);
    }
    if input.pointer().just_pressed(PointerButton::Primary) {
        return screen::hit(camera.screen_to_world(input.pointer().screen));
    }
    None
}

/// The stack item a slot names (0 = top), if there is one.
pub(crate) fn item_at_slot(duel: &Duel, slot: usize) -> Option<ItemId> {
    let len = duel.stack.len();
    (slot < len).then(|| duel.stack[len - 1 - slot].id)
}

/// Act on one intent of yours; a refused move changes nothing.
pub(crate) fn act(table: &mut Table, intent: Intent) {
    let duel = &mut table.duel;
    if duel.outcome.is_some() || duel.priority != Side::You {
        table.choosing = None;
        return;
    }
    match (intent, table.choosing) {
        (Intent::Cancel, _) => table.choosing = None,
        (Intent::Pass, _) => {
            table.choosing = None;
            let _ = pass(duel, Side::You);
        }
        (Intent::Slot(slot), Some(index)) => {
            if let Some(id) = item_at_slot(duel, slot)
                && play(duel, Side::You, index, Some(id)).is_ok()
            {
                table.choosing = None;
            }
        }
        (Intent::Slot(_), None) => {}
        (Intent::Card(index), _) => {
            let Some(&card) = duel.you.hand.get(index) else {
                return;
            };
            if card.cost() > duel.you.focus {
                return;
            }
            if card.needs_target() {
                let aimable = !legal_targets(duel, Side::You, card).is_empty();
                table.choosing = aimable.then_some(index);
            } else {
                table.choosing = None;
                let _ = play(duel, Side::You, index, None);
            }
        }
    }
}

fn take_your_input(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let wanted = intent(input, world.resource::<Camera>());
    let table = world.resource_mut::<Table>();
    if table.duel.priority != Side::You || table.duel.outcome.is_some() {
        table.choosing = None;
    }
    if let Some(wanted) = wanted {
        act(table, wanted);
    }
}

/// The rival thinks for `RIVAL_THINK` ticks, then plays `best_choice`.
fn rival_acts(world: &mut World) {
    let table = world.resource_mut::<Table>();
    if table.duel.outcome.is_some() || table.duel.priority != Side::Rival {
        table.rival_clock = 0;
        return;
    }
    table.rival_clock += 1;
    if table.rival_clock < RIVAL_THINK {
        return;
    }
    table.rival_clock = 0;
    let duel = &mut table.duel;
    let _ = match best_choice(duel, Side::Rival) {
        Choice::Play { index, target } => play(duel, Side::Rival, index, target),
        Choice::Pass => pass(duel, Side::Rival),
    };
}

fn draw_the_table(ctx: &mut DrawCtx) {
    let table = ctx.world.resource::<Table>();
    let boxes = screen::boxes(table);
    let panel = screen::screen(table);
    for (rect, color, layer) in boxes {
        ctx.rect(rect, color, Depth::layer(layer));
    }
    for rect in screen::marks(table) {
        screen::outline(ctx, rect);
    }
    panel.draw(ctx, &screen::Flat, |icon, _| match icon.art {});
}

/// A card's name as the hand shows it.
pub(crate) fn card_title(index: usize, card: Card) -> String {
    format!("{} {}", index + 1, card.name().to_uppercase())
}
