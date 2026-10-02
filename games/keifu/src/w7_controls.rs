//! W7's controls: each group's help as shipped, a child carried to the hall, a hand
//! released over nothing, the quiet winter, and the turning's leaves and "Skip ahead".
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from the content.

use jidousha::prelude::*;

use crate::board::Slot;
use crate::checks::Checks;
use crate::hearth::{Group, Seat};
use crate::hearth_view::{group_rect, seat_rect};
use crate::house::House;
use crate::screen::{Target, UiState};
use crate::scripted::{Pointer, center_of, drag, lines_in};
use crate::summer::SHEET;
use crate::verify::{SEEDS, hero_named, page_of, point_at, session};
use crate::w7::{group_lines, seat_at, stage_oracle_winter, stay_home_into_winter, turning_lines};

/// Each group's help as the dock shows it when its heading is pointed at: `ui.winter`'s
/// `*_help` with CONSTANTS §9-§10's numbers, as shipped.
pub const GROUP_HELP: [(Group, &str, &str); 6] = [
    (
        Group::Hall,
        "THE HALL",
        "There are more hands than seats. Whoever is left in the hall keeps the house through the winter and gains nothing by it.",
    ),
    (
        Group::Fire,
        "BY THE FIRE",
        "Only the fire heals. A wounded hero left in the hall goes into summer wounded.",
    ),
    (
        Group::Training,
        "THE TRAINING YARD",
        "Alone, a hero trains their own calling, up to 6. A teacher passes on their best aptitude, up to their own. Veterans and elders teach well. The young learn fast.",
    ),
    (
        Group::Garden,
        "THE GARDEN",
        "Two may wed when both are 18 or older and no more than 15 years apart. Kin cannot wed. From the year after they wed, a pair may have a child, 60 times in a hundred each winter, while both are 18 to 45. Rivals who walk here a winter make peace.",
    ),
    (
        Group::Table,
        "THE LONG TABLE",
        "Tell the tale: +1 renown to the teller, and +1 to the house, once a winter however many tell it. It is also the last step of many dreams.",
    ),
    (
        Group::Benches,
        "THE BENCHES IN THE YARD",
        "From the age of 6, a child learns 1 a winter from whoever sits beside them, up to 4. Their first teacher decides their calling. Drag the child from the yard, and an adult beside them.",
    ),
];

/// Point at a group's heading.
pub fn point_at_group(sim: &mut HeadlessSim, group: Group) {
    let r = if group == Group::Hall {
        crate::hearth_view::hall_rect()
    } else {
        group_rect(group)
    };
    let heading = Rect::from_min_size(r.min, Vec2::new(r.size().x, 14.0));
    crate::verify::point(sim, heading.center(), false);
}

/// The quiet winter, the turning's controls, a child carried to the hall, a hand
/// released over nothing, each group's help. Returns the summary.
pub fn check_controls(checks: &mut Checks) -> String {
    let mut sim = session(SEEDS[0]);
    stay_home_into_winter(&mut sim);
    for (group, heading, help) in GROUP_HELP {
        point_at_group(&mut sim, group);
        let dock = lines_in(&page_of(&sim), SHEET);
        checks.require(
            dock == [heading, help],
            "a group's help in the dock is not its shipped heading and help",
            format!("{group:?}: {dock:?}"),
        );
    }
    let fire = group_lines(&sim, Group::Fire);
    checks.require(
        fire.contains(&"Rest. A wound heals, 1 dread is shed.".to_owned()),
        "the fire's text does not say one dread is shed",
        format!("{fire:?}"),
    );
    seated_hearth(checks);
    // A child can be carried anywhere in winter: Wren from the yard to the hall.
    let wren = hero_named(&sim, "Wren");
    let free = sim
        .world()
        .resource::<House>()
        .roster
        .iter()
        .position(Option::is_none);
    if let Some(free) = free {
        let from = center_of(&sim, Target::Hero(wren));
        let to = center_of(&sim, Target::Seat(Slot::Roster(free)));
        drag(&mut sim, &mut Pointer::mouse(), from, to);
    }
    let landed = sim.world().resource::<House>().slot_of(wren);
    checks.require(
        free.is_some() && landed == free.map(Slot::Roster),
        "a child cannot be carried from the yard to the hall in winter",
        format!("free hall seat {free:?}, Wren now in {landed:?}"),
    );
    // Released over nothing, a hero returns where they sat.
    let odo = hero_named(&sim, "Odo");
    let before = sim.world().resource::<House>().slot_of(odo);
    let from = center_of(&sim, Target::Hero(odo));
    drag(
        &mut sim,
        &mut Pointer::mouse(),
        from,
        Vec2::new(700.0, 30.0),
    );
    let after = sim.world().resource::<House>().slot_of(odo);
    checks.require(
        before.is_some() && before == after,
        "a hero released over nothing in winter does not return to their seat",
        format!("{before:?} then {after:?}"),
    );
    // Nobody in a winter seat: the quiet winter.
    point_at(&mut sim, Target::LetWinterPass, true);
    let quiet = turning_lines(&sim);
    checks.require(
        quiet[..3]
            == [
                "What the winter did",
                "The winter of year 1",
                "A quiet winter. The house kept to the fire.",
            ]
            && lines_in(&page_of(&sim), SHEET).first().map(String::as_str)
                == Some("The year turns. Everyone is a year older. Point at a line to read who it is about."),
        "an empty hearth does not pass as \"A quiet winter.\"",
        format!("{quiet:?}"),
    );
    point_at(&mut sim, Target::Skip, true);
    let house = sim.world().resource::<House>();
    checks.require(
        house.passage.is_none()
            && house.calendar.current_year() == 2
            && !house.calendar.is_winter(),
        "\"Skip ahead\" on the turning does not bring the summer",
        format!("year {}", house.calendar.current_year()),
    );
    // A winter page long enough to continue: the oracle's lines told eight times over.
    let mut sim = session(SEEDS[0]);
    stage_oracle_winter(&mut sim);
    {
        let house = sim.world_mut().resource_mut::<House>();
        if let Some(passage) = house.passage.as_mut() {
            passage.winter = (0..8).flat_map(|_| passage.winter.clone()).collect();
        }
    }
    point_at(&mut sim, Target::GoOn, true);
    let second = turning_lines(&sim);
    let ui = *sim.world().resource::<UiState>();
    checks.require(
        ui.leaf == 1
            && second.first().map(String::as_str) == Some("What the winter did, continued")
            && sim.world().resource::<House>().passage.is_some(),
        "a winter page longer than a leaf does not continue under \"What the winter did, continued\"",
        format!("leaf {}, {second:?}", ui.leaf),
    );
    "W7 controls: a child carried to the hall, a hand released over nothing returns, the quiet winter, the turning's leaves and \"Skip ahead\"".to_owned()
}

/// A hearth seat's hero lifted and moved, a swap on the hearth, the lesson's note beside
/// its own bench, and the teacher alone's "no learner" and "no child".
fn seated_hearth(checks: &mut Checks) {
    let mut sim = session(SEEDS[0]);
    stay_home_into_winter(&mut sim);
    seat_at(&mut sim, "Pip", Seat::BenchChild(0));
    seat_at(&mut sim, "Odo", Seat::BenchTeacher(0));
    // The note sits beside its own bench: level with bench 0's tiles.
    let page = page_of(&sim);
    let tile = seat_rect(Seat::BenchTeacher(0));
    let note = page
        .rows
        .iter()
        .find(|r| r.panel == group_rect(Group::Benches) && r.text == "+1 Spirit")
        .map(|r| r.bounds());
    checks.require(
        note.is_some_and(|n| {
            n.min.y >= tile.min.y && n.max.y <= tile.max.y && n.min.x > tile.max.x
        }),
        "a bench's lesson is not previewed beside its own seats",
        format!("note {note:?}, bench 0's teacher seat {tile:?}"),
    );
    // A seated hero is lifted from a hearth seat and carried to another.
    let (pip, wren) = (hero_named(&sim, "Pip"), hero_named(&sim, "Wren"));
    let wren_from = sim.world().resource::<House>().slot_of(wren);
    let (from, onto) = (
        center_of(&sim, Target::Hero(wren)),
        center_of(&sim, Target::Hero(pip)),
    );
    drag(&mut sim, &mut Pointer::mouse(), from, onto);
    let house = sim.world().resource::<House>();
    checks.require(
        house.hearth.at(Seat::BenchChild(0)) == Some(wren) && house.slot_of(pip) == wren_from,
        "a hero dropped on a hearth seat's hero does not swap with them",
        format!(
            "bench 0 holds {:?}; Pip now in {:?}, Wren came from {wren_from:?}",
            house.hearth.at(Seat::BenchChild(0)),
            house.slot_of(pip)
        ),
    );
    seat_at(&mut sim, "Odo", Seat::Table(1));
    let house = sim.world().resource::<House>();
    checks.require(
        house.hearth.at(Seat::Table(1)) == Some(hero_named(&sim, "Odo"))
            && house.hearth.at(Seat::BenchTeacher(0)).is_none(),
        "a hero on a hearth seat cannot be lifted and carried to another",
        format!(
            "table 1 holds {:?}, bench 0's teacher seat {:?}",
            house.hearth.at(Seat::Table(1)),
            house.hearth.at(Seat::BenchTeacher(0))
        ),
    );
    // A teacher alone: "no learner" in the yard, "no child" on a bench.
    seat_at(&mut sim, "Garrick", Seat::Teacher);
    seat_at(&mut sim, "Maren", Seat::BenchTeacher(1));
    let training = group_lines(&sim, Group::Training);
    let benches = group_lines(&sim, Group::Benches);
    checks.require(
        training.contains(&"no learner".to_owned()) && benches.contains(&"no child".to_owned()),
        "a teacher alone does not preview \"no learner\" in the yard, or \"no child\" on a bench",
        format!("training {training:?}; benches {benches:?}"),
    );
}
