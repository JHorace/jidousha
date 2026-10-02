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
//!
//! The pointer also works the sheet dock (`dock.rs`): resting on it keeps the sheet
//! that was open, the wheel scrolls it a line per line of wheel travel, and a press
//! in it grabs the sheet to drag it up and down — a finger's scroll. A new subject in
//! the dock opens at its top.

use jidousha::prelude::*;

use crate::dock::{PITCH, subject};
use crate::house::House;
use crate::screen::{DockGrab, Drag, Target, UiState};

/// What a pointer resting on `target` points at. Resting on the dock points at what
/// it already did, so the sheet stays open to be read and scrolled.
fn resting(ui: UiState, target: Option<Target>) -> UiState {
    if target == Some(Target::Dock) {
        return UiState { drag: None, ..ui };
    }
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
    let wheel = pointer.scroll;
    let lost = !input.window_focused()
        || input
            .touches()
            .iter()
            .any(|touch| touch.phase == TouchPhase::Cancelled);
    let at = world.resource::<Camera>().screen_to_world(pointer.screen);
    let page = crate::read_the_page(&world.view());
    let target = page.target_at(at);
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
    let next = scroll_the_dock(
        ui,
        next,
        page.dock.max_first,
        at,
        wheel,
        pressed,
        held && !lost,
        target,
    );
    *world.resource_mut::<UiState>() = next;
}

/// The dock's scroll after this tick: back to the top for a new subject; else moved
/// by a grab (a press in the dock, held) or by the wheel, and kept in range.
#[allow(clippy::too_many_arguments)]
fn scroll_the_dock(
    ui: UiState,
    mut next: UiState,
    max_first: usize,
    at: Vec2,
    wheel: f32,
    pressed: bool,
    held: bool,
    target: Option<Target>,
) -> UiState {
    if subject(&ui) != subject(&next) {
        next.dock_first = 0;
        next.dock_wheel = 0.0;
        next.dock_grab = None;
        return next;
    }
    next.dock_grab = match ui.dock_grab {
        Some(grab) if held => Some(grab),
        _ if pressed && held && target == Some(Target::Dock) && next.drag.is_none() => {
            Some(DockGrab {
                y: at.y,
                first: ui.dock_first,
            })
        }
        _ => None,
    };
    let first = match next.dock_grab {
        // Dragging the sheet up shows what is below it.
        Some(grab) => grab.first as f32 + ((grab.y - at.y) / PITCH).round(),
        None => {
            // Wheel travel away from the player (positive) scrolls toward the top.
            let travel = ui.dock_wheel + wheel;
            let whole = travel.trunc();
            next.dock_wheel = travel - whole;
            ui.dock_first as f32 - whole
        }
    };
    next.dock_first = first.clamp(0.0, max_first as f32) as usize;
    next
}
