//! From a key or a click to a `Choice`. The click is resolved against the same rectangles
//! the screen was drawn from (`screens::controls`), so a row clicks where it is drawn.

use jidousha::prelude::*;

use crate::game::{Choice, Game, Refused, Stage};
use crate::screens::controls;
use crate::view::UiMap;

/// The number keys, in option order.
const DIGITS: [Key; 8] = [
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
    Key::Digit7,
    Key::Digit8,
];

/// What this tick's input chose, if anything: a number key, or a click on a row.
pub fn read_choice(input: &Input, game: &Game, camera: &Camera) -> Option<Choice> {
    for (n, key) in DIGITS.iter().enumerate() {
        if input.just_pressed(*key) {
            return Some(Choice::Option(n));
        }
    }
    if input.just_pressed(Key::Space) || input.just_pressed(Key::Enter) {
        return Some(Choice::Continue);
    }
    let pointer = input.pointer();
    if pointer.just_pressed(PointerButton::Primary) {
        let map = UiMap::for_camera(camera);
        let at = map.to_design(camera.screen_to_world(pointer.screen));
        return controls(game)
            .into_iter()
            .find(|(rect, _)| rect.contains(at))
            .map(|(_, choice)| choice);
    }
    None
}

/// The Update system: apply whatever was chosen, and tell the player why when it did nothing.
pub fn apply_input(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let camera = *world.resource::<Camera>();
    let game = world.resource::<Game>();
    let choice = read_choice(input, game, &camera);
    let restart = matches!(game.stage, Stage::Over(_)) && input.just_pressed(Key::R);
    if restart {
        let seed = game.seed.wrapping_add(1);
        world.insert_resource(Game::new(seed));
        return;
    }
    let Some(choice) = choice else {
        return;
    };
    let game = world.resource_mut::<Game>();
    if let Err(Refused(why)) = game.choose(choice) {
        game.note = why;
    }
}
