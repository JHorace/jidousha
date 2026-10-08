//! Input → action: the one `Update` system. At most one action a tick — the
//! first matching key in a fixed order — mapped by phase, then `turn::apply`.
//!
//! Key functions: `advance`, `action_for`.

use jidousha::prelude::*;

use crate::sim::{Game, Phase};
use crate::turn::{Action, apply};

/// The keys the game reads, in the order a tick checks them.
pub(crate) const KEYS: [Key; 14] = [
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
    Key::Digit7,
    Key::Digit8,
    Key::Digit9,
    Key::S,
    Key::O,
    Key::Escape,
    Key::Enter,
    Key::P,
];

fn digit(key: Key) -> Option<usize> {
    KEYS[..9].iter().position(|&k| k == key)
}

/// What `key` asks for on `phase`.
pub(crate) fn action_for(phase: Phase, key: Key) -> Option<Action> {
    match phase {
        Phase::Service { focus: None } => match key {
            Key::Enter => Some(Action::CloseKitchen),
            _ => digit(key).map(Action::Serve),
        },
        Phase::Service { focus: Some(_) } => match key {
            Key::S => Some(Action::ServeNemesis),
            Key::O => Some(Action::Overwhelm),
            Key::Escape => Some(Action::Unfocus),
            _ => None,
        },
        Phase::Ledger => match key {
            Key::P => Some(Action::Prep),
            Key::Enter => Some(Action::OpenTomorrow),
            _ => digit(key).filter(|&slot| slot < 3).map(Action::Spend),
        },
        Phase::Over { .. } => (key == Key::Enter).then_some(Action::Restart),
    }
}

/// Read this tick's keys and apply the first one that means something.
pub(crate) fn advance(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let phase = world.resource::<Game>().phase;
    let Some(action) = KEYS
        .iter()
        .filter(|&&key| input.just_pressed(key))
        .find_map(|&key| action_for(phase, key))
    else {
        return;
    };
    let mut rng = world.resource::<Rng>().clone();
    apply(world.resource_mut::<Game>(), action, &mut rng);
    world.insert_resource(rng);
}
