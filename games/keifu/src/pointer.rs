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

use crate::content::Content;
use crate::dock::{PITCH, subject};
use crate::house::{House, begin_another_house};
use crate::resolve::set_out;
use crate::screen::{Clock, DockGrab, Drag, Target, UiState};
use crate::season::{
    at_the_hearth, leave_the_telling, let_the_winter_pass, may_turn, summer_comes,
};
use crate::telling_view::{leaves, story_complete};
use crate::turning_view::{first_leaf_of, furthest, last_leaf_of, leaves as turning_leaves};

/// What a pointer resting on `target` points at. Resting on the dock points at what
/// it already did, so the sheet stays open to be read and scrolled.
fn resting(ui: UiState, target: Option<Target>) -> UiState {
    if target == Some(Target::Dock) {
        return UiState { drag: None, ..ui };
    }
    UiState {
        pointing: match target {
            Some(Target::Hero(id) | Target::Heir(_, Some(id))) => Some(id),
            _ => None,
        },
        pointing_quest: match target {
            Some(Target::Quest(quest)) if !ui.family_open => Some(quest),
            _ => None,
        },
        pointing_group: match target {
            Some(Target::Group(group)) if !ui.family_open => Some(group),
            _ => None,
        },
        pointing_door: target == Some(Target::DoorHelp) && !ui.family_open,
        pointing_outlook: target == Some(Target::Outlook) && !ui.family_open,
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
            pointing_group: None,
            ..ui
        },
        None if pressed => match target {
            Some(Target::OpenFamily) => UiState::family(),
            Some(Target::CloseFamily) => UiState::default(),
            Some(
                control @ (Target::SetOut
                | Target::GoOn
                | Target::Leaf(_)
                | Target::Skip
                | Target::BeginAgain
                | Target::LetWinterPass
                | Target::Heir(..)),
            ) if !ui.family_open => press(world, ui, control),
            // Only the summer and the hearth seat heroes: on the telling and the turning a
            // card is read, never lifted.
            Some(Target::Hero(hero))
                if !ui.family_open && held && !released && seating(world.resource::<House>()) =>
            {
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
                        pointing_group: None,
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

/// Whether the summer screen is up: summer, no telling open, the house not closed.
fn in_summer(house: &House) -> bool {
    house.telling.is_none()
        && !house.closed
        && house.ending.is_none()
        && !house.calendar.is_winter()
}

/// Whether a screen that seats heroes is up: the summer's, or the hearth's.
fn seating(house: &House) -> bool {
    in_summer(house) || at_the_hearth(house)
}

/// Run a rule on the house with the run's generator: the house and the generator are
/// cloned out, changed, and put back, so the content can be read beside them.
fn with_house(world: &mut World, rule: impl FnOnce(&Content, &mut House, &mut Rng)) {
    let mut house = world.resource::<House>().clone();
    let mut rng = world.resource::<Rng>().clone();
    rule(world.resource::<Content>(), &mut house, &mut rng);
    world.insert_resource(house);
    world.insert_resource(rng);
}

/// A control pressed (SPEC §5.3, §8): set out; on the telling, "Go on" — the story
/// whole first, then the next leaf, then leaving — a numbered leaf, or "Skip ahead";
/// and, once the house has ended, "Begin another house" (SPEC §23). Returns the
/// UI state after it: a new screen opens at its top, typing from this tick.
fn press(world: &mut World, ui: UiState, control: Target) -> UiState {
    let tick = world.resource::<Time>().tick;
    let fresh = UiState {
        typing_from: tick,
        ..UiState::default()
    };
    let clock = Clock {
        tick,
        dt: world.resource::<Time>().fixed_dt.0,
    };
    let turn_to = |leaf: usize| UiState {
        leaf,
        typing_from: tick,
        revealed: false,
        pointing: None,
        dock_first: 0,
        ..ui
    };
    let house = world.resource::<House>();
    match control {
        // "Try the Door" waits for at least one before it (SPEC §5.3, §16.1).
        Target::SetOut if in_summer(house) && crate::summer::may_set_out(house) => {
            with_house(world, set_out);
            fresh
        }
        Target::LetWinterPass if at_the_hearth(house) => {
            with_house(world, let_the_winter_pass);
            fresh
        }
        // The turning (SPEC §18.1): it will not go past an undecided death page — "Go on"
        // there does nothing, a leaf beyond it does nothing, "Skip ahead" goes to it —
        // and summer comes only once every page is decided. SPEC-GAPS KG-51: "past" is
        // past the first undecided page's last leaf, where its choice is drawn.
        Target::GoOn | Target::Leaf(_) | Target::Skip if house.passage.is_some() => {
            let Some(passage) = &house.passage else {
                return ui;
            };
            let leaves = turning_leaves(passage, &house.heroes);
            let current = ui.leaf.min(leaves.len() - 1);
            let reach = furthest(passage, &leaves);
            let undecided = passage.first_undecided();
            match (control, undecided) {
                (Target::Leaf(leaf), _) if leaf <= reach => return turn_to(leaf),
                (Target::Leaf(_), _) => return ui,
                (Target::GoOn, _) if current < reach => return turn_to(current + 1),
                (Target::Skip, Some(page)) => return turn_to(last_leaf_of(&leaves, page)),
                _ if !may_turn(house) => return ui,
                _ => with_house(world, summer_comes),
            }
            fresh
        }
        Target::Heir(at, heir) if house.passage.is_some() => {
            with_house(world, |content, house, _| {
                crate::heirs::choose(content, house, at, heir)
            });
            let house = world.resource::<House>();
            let Some(passage) = &house.passage else {
                return ui;
            };
            // The leaves are rebuilt; the view stays on the page chosen on.
            turn_to(first_leaf_of(&turning_leaves(passage, &house.heroes), at))
        }
        // SPEC-GAPS KG-37: a numbered button turns to a leaf and types its story again.
        Target::GoOn | Target::Leaf(_) | Target::Skip => {
            let Some(telling) = &house.telling else {
                return ui;
            };
            let leaves = leaves(world.resource::<Content>(), house, telling);
            let current = ui.leaf.min(leaves.len() - 1);
            let leave = match control {
                Target::Leaf(leaf) => return turn_to(leaf.min(leaves.len() - 1)),
                Target::Skip => true,
                _ if !story_complete(telling, &leaves[current], &ui, clock) => {
                    return UiState {
                        revealed: true,
                        ..ui
                    };
                }
                _ if current + 1 < leaves.len() => return turn_to(current + 1),
                _ => true,
            };
            if leave {
                with_house(world, leave_the_telling);
            }
            fresh
        }
        Target::BeginAgain if house.ending.is_some() => {
            if let Err(error) = begin_another_house(world) {
                panic!("[keifu] another house could not be founded\n  {error}");
            }
            fresh
        }
        _ => ui,
    }
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
