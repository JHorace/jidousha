//! W10's pictures, staged: the Door card empty, mid-drag and seated, its sheet, the Door's
//! help; the telling's prologue and a lock's page; each verdict; the family at the Ending
//! on the staged house and on a house played to its end; and another house begun.

use jidousha::prelude::{HeadlessSim, Vec2};

use crate::house::House;
use crate::screen::{Target, UiState};
use crate::scripted::{Pointer, center_of};
use crate::verify::{SEEDS, point, point_at, set_ui};
use crate::w10::{drag_onto_the_door, make_party_a, stage_the_last_summer};
use crate::w10_ending::{STAGED, stage_the_door, try_and_leave};

/// The pointer at rest off everything, for a frame to settle under.
fn rest(sim: &mut HeadlessSim) {
    point(sim, Vec2::new(4.0, 700.0), false);
}

/// Party A on the Door, the last of them held over its card if `held`.
fn party_a_on_the_door(sim: &mut HeadlessSim, held: bool) {
    stage_the_last_summer(sim);
    let party = make_party_a(sim);
    let seated = if held { 3 } else { 4 };
    for (seat, &who) in party[..seated].iter().enumerate() {
        drag_onto_the_door(sim, who, seat);
    }
    if held {
        let mut mouse = Pointer::mouse();
        let from = center_of(sim, Target::Hero(party[3]));
        mouse.press(sim, from);
        mouse.hold_at(sim, crate::summer::BOARD.center());
    }
}

/// Stage W10's picture `name` on `sim`, a fresh session on `SEEDS[0]`.
pub fn stage(sim: &mut HeadlessSim, name: &str) {
    match name {
        "door-empty" => {
            stage_the_last_summer(sim);
            rest(sim);
        }
        "door-help" => {
            stage_the_last_summer(sim);
            point_at(sim, Target::DoorHelp, false);
        }
        "door-drag" => party_a_on_the_door(sim, true),
        "door-card" => {
            party_a_on_the_door(sim, false);
            rest(sim);
        }
        "door-sheet" => {
            party_a_on_the_door(sim, false);
            point_at(sim, Target::Quest(0), false);
        }
        "prologue" | "lock" => {
            party_a_on_the_door(sim, false);
            point_at(sim, Target::SetOut, true);
            if name == "lock" {
                point_at(sim, Target::Leaf(1), true);
            }
            crate::w6::wait(sim, 400);
            rest(sim);
        }
        "family" | "reset" => {
            *sim = stage_the_door(SEEDS[0], STAGED[0].0);
            try_and_leave(sim);
            if name == "family" {
                point_at(sim, Target::OpenFamily, true);
                let ysolde = crate::verify::hero_named(sim, "Ysolde");
                point_at(sim, Target::Hero(ysolde), false);
            } else {
                point_at(sim, Target::BeginAgain, true);
                rest(sim);
            }
        }
        "played" | "played-family" => {
            let content = crate::verify::content_of(sim);
            let (house, rng) = crate::w10_battery::ended(content, 0xa_0003);
            sim.world_mut().insert_resource(house);
            sim.world_mut().insert_resource(rng);
            set_ui(sim, UiState::default());
            if name == "played-family" {
                point_at(sim, Target::OpenFamily, true);
                let first_living = {
                    let house = sim.world().resource::<House>();
                    (0..house.heroes.len())
                        .rev()
                        .find(|&h| house.heroes[h].is_living())
                };
                if let Some(id) = first_living {
                    point_at(sim, Target::Hero(id), false);
                }
            } else {
                rest(sim);
            }
        }
        verdict => {
            let locks = verdict
                .strip_prefix("verdict-")
                .and_then(|n| n.parse::<usize>().ok());
            let Some((bases, _)) = STAGED.iter().find(|(_, l)| Some(*l) == locks) else {
                crate::checks::fail("an unknown W10 picture", verdict);
            };
            *sim = stage_the_door(SEEDS[0], *bases);
            try_and_leave(sim);
            rest(sim);
        }
    }
}
