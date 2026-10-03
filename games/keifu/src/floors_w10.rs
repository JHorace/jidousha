//! W10's readability floors: the last summer — the Door card empty, with three seated and
//! with four, mid-drag, and its sheet; the Door's help in the dock — the telling's prologue
//! and every lock's page, a prologue long enough to continue; each verdict page, the four
//! titles and the closed house's; the family at the Ending, each node's epitaph pointed
//! at, on the staged Door's house and on the largest of a run of houses played to their
//! end. Each judged by `floors.rs`'s `look` at each of its sizes.

use jidousha::prelude::HeadlessSim;
use jidousha::testing::FrameRecorder;

use crate::checks::Checks;
use crate::floors::{Tally, look, look_held};
use crate::house::House;
use crate::screen::{Target, UiState};
use crate::scripted::{Pointer, center_of};
use crate::verify::{SEEDS, point_at, session, set_ui};
use crate::w10::{drag_onto_the_door, make_party_a, stage_the_last_summer};
use crate::w10_ending::{STAGED, stage_the_door, try_and_leave};

/// Point at each node of the family the page shows, judging each remembrance.
fn every_node(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    sim: &mut HeadlessSim,
    name: &str,
) {
    look(
        checks,
        tally,
        recorder,
        sim,
        &format!("{name}, nobody pointed at"),
        true,
    );
    let everyone = sim.world().resource::<House>().heroes.len();
    for id in 0..everyone {
        point_at(sim, Target::Hero(id), false);
        let who = sim.world().resource::<House>().heroes[id].name.clone();
        look(
            checks,
            tally,
            recorder,
            sim,
            &format!("{name}, {who}"),
            true,
        );
    }
}

/// Every leaf of the telling open now.
fn every_leaf(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    sim: &mut HeadlessSim,
    name: &str,
) {
    let mut leaf = 0;
    while leaf < 60 && crate::verify::target_rect(sim, Target::Leaf(leaf)).is_some() {
        point_at(sim, Target::Leaf(leaf), true);
        crate::w6::wait(sim, 400);
        look(
            checks,
            tally,
            recorder,
            sim,
            &format!("{name}, leaf {}", leaf + 1),
            false,
        );
        leaf += 1;
    }
}

/// The largest of a run of houses played to their end by the teaching player.
fn largest_ended(sim: &HeadlessSim) -> (House, jidousha::prelude::Rng) {
    let content = crate::verify::content_of(sim);
    (0xa_0000..0xa_0000 + 16)
        .map(|seed| crate::w10_battery::ended(content, seed))
        .max_by_key(|(house, _)| house.heroes.len())
        .unwrap_or_else(|| crate::w10_battery::ended(content, 0xa_0000))
}

/// W10's surfaces. Returns how many were judged.
pub fn w10_surfaces(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    _label: &str,
) -> usize {
    let before = tally.surfaces;
    let mut sim = session(SEEDS[0]);
    stage_the_last_summer(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W10, the last summer, the Door empty",
        false,
    );
    point_at(&mut sim, Target::Quest(0), false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W10, the Door's sheet, nobody before it",
        false,
    );
    point_at(&mut sim, Target::DoorHelp, false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W10, the Door's help",
        false,
    );
    let party = make_party_a(&mut sim);
    for (seat, &who) in party[..3].iter().enumerate() {
        drag_onto_the_door(&mut sim, who, seat);
    }
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W10, the Door with three",
        false,
    );
    let mut mouse = Pointer::mouse();
    let from = center_of(&sim, Target::Hero(party[3]));
    mouse.press(&mut sim, from);
    mouse.hold_at(&mut sim, crate::summer::BOARD.center());
    look_held(
        checks,
        tally,
        recorder,
        &mut sim,
        "W10, Brannoc in hand over the Door",
    );
    mouse.release(&mut sim, crate::summer::BOARD.center());
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W10, the Door with four",
        false,
    );
    point_at(&mut sim, Target::Quest(0), false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W10, the Door's sheet with four",
        false,
    );
    point_at(&mut sim, Target::SetOut, true);
    every_leaf(checks, tally, recorder, &mut sim, "W10, the Door's telling");
    // A prologue long enough to continue: each of the four promised by blood, carrying a
    // blade, conquered, blessed three times over and wounded.
    let mut sim = stage_the_door(SEEDS[0], [9, 9, 9]);
    {
        let house = sim.world_mut().resource_mut::<House>();
        let party = house.party(0);
        let blade = house.heroes[party[0]].heirloom.clone();
        for &who in &party {
            let hero = &mut house.heroes[who];
            hero.destiny.kind = crate::ids::Destiny::OpenTheSealedDoor;
            hero.destiny.blood_of = Some("Ysolde".to_owned());
            hero.heirloom = blade.clone();
            hero.fear.tag = crate::ids::Tag::Cold;
            hero.fear.conquered = true;
            hero.wounded = true;
            for title in ["Odo's patience", "Wren's patience", "Pip's patience"] {
                hero.blessings.push(crate::hero::Blessing {
                    title: title.to_owned(),
                    scope: crate::hero::Scope::Everywhere,
                    power: 1,
                });
            }
        }
    }
    point_at(&mut sim, Target::SetOut, true);
    every_leaf(checks, tally, recorder, &mut sim, "W10, a long prologue");
    // Each verdict, and the family at the Ending on the first.
    for (bases, locks) in STAGED {
        let mut sim = stage_the_door(SEEDS[0], bases);
        try_and_leave(&mut sim);
        look(
            checks,
            tally,
            recorder,
            &mut sim,
            &format!("W10, the verdict, {locks} locks"),
            false,
        );
        if locks == 3 {
            point_at(&mut sim, Target::OpenFamily, true);
            every_node(
                checks,
                tally,
                recorder,
                &mut sim,
                "W10, the family at the Ending",
            );
        }
    }
    let mut sim = session(SEEDS[0]);
    crate::w6_stages::stage_closed(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W10, the closed house's verdict",
        false,
    );
    point_at(&mut sim, Target::OpenFamily, true);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W10, the closed house's family",
        true,
    );
    // The largest house played to its end: its verdict and its whole family.
    let mut sim = session(SEEDS[0]);
    let (house, rng) = largest_ended(&sim);
    sim.world_mut().insert_resource(house);
    sim.world_mut().insert_resource(rng);
    set_ui(&mut sim, UiState::default());
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W10, a played house's verdict",
        false,
    );
    point_at(&mut sim, Target::OpenFamily, true);
    every_node(
        checks,
        tally,
        recorder,
        &mut sim,
        "W10, a played house's family at the Ending",
    );
    tally.surfaces - before
}
