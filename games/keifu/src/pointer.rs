//! The pointer: what it points at, the two family buttons, and the drag that
//! seats heroes (SPEC §5.3).
//!
//! A press on a seated hero's card picks them up; while the button is held they
//! stay in hand and the board previews where they would land; the release puts
//! them down through `House::drop_hero`, at `House::landing` — the same answer the
//! preview read. A press and release on one tick is a tap, and picks nothing up.
//! A drag the system takes away (a cancelled touch, the window losing focus, the
//! button gone without a release) is undone: the hero stays where they were.
//! The engine mirrors the first finger onto this pointer, so all of it works by
//! touch with nothing more written here.

use jidousha::prelude::*;

use crate::house::House;
use crate::screen::{Drag, Target, UiState};

/// What a pointer resting on `target` points at.
fn resting(ui: UiState, target: Option<Target>) -> UiState {
    UiState {
        pointing: match target {
            Some(Target::Hero(id)) => Some(id),
            _ => None,
        },
        pointing_quest: match target {
            Some(Target::Quest(quest)) if !ui.family_open => Some(quest),
            _ => None,
        },
        drag: None,
        ..ui
    }
}

/// Follow the pointer for one tick.
pub fn follow_the_pointer(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let pointer = input.pointer();
    let pressed = pointer.just_pressed(PointerButton::Primary);
    let held = pointer.held(PointerButton::Primary);
    let released = pointer.just_released(PointerButton::Primary);
    let lost = !input.window_focused()
        || input
            .touches()
            .iter()
            .any(|touch| touch.phase == TouchPhase::Cancelled);
    let at = world.resource::<Camera>().screen_to_world(pointer.screen);
    let target = crate::read_the_page(&world.view()).target_at(at);
    let ui = *world.resource::<UiState>();
    let next = match ui.drag {
        Some(_) if lost || (!held && !released) => resting(ui, target),
        Some(drag) if released => {
            let house = world.resource_mut::<House>();
            let onto = house.landing(drag.hero, target);
            house.drop_hero(drag.from, onto);
            resting(ui, target)
        }
        Some(drag) => UiState {
            drag: Some(Drag {
                at,
                over: target,
                ..drag
            }),
            pointing: None,
            pointing_quest: None,
            ..ui
        },
        None if pressed => match target {
            Some(Target::OpenFamily) => UiState::family(),
            Some(Target::CloseFamily) => UiState::default(),
            Some(Target::Hero(hero)) if !ui.family_open && held && !released => {
                match world.resource::<House>().slot_of(hero) {
                    Some(from) => UiState {
                        drag: Some(Drag {
                            hero,
                            from,
                            at,
                            over: target,
                        }),
                        pointing: None,
                        pointing_quest: None,
                        ..ui
                    },
                    // A child in the yard is not seated in summer and cannot be dragged.
                    None => resting(ui, target),
                }
            }
            _ => resting(ui, target),
        },
        None => resting(ui, target),
    };
    *world.resource_mut::<UiState>() = next;
}
