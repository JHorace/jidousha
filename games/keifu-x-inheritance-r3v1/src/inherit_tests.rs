//! The variant's rules a decision row does not reach, each a test named as a sentence
//! (DESIGN.md "Gates to add", and the mutation round's escapes).
//!
//! INVARIANT: every expectation is a shipped literal — never computed by the code
//! under test.

use jidousha::prelude::Rng;

use crate::births::births;
use crate::bonds::form;
use crate::content::Content;
use crate::genes::Trait;
use crate::hero::Blood;
use crate::house::House;
use crate::ids::{Aptitude, BondKind, Destiny, Outcome, Place};
use crate::power::{QuestFacts, member_power};
use crate::resolve::resolve_rolled;
use crate::testkit::{aim, house, id, seat};
use crate::turning::turn_the_year;
use crate::winter::WinterPlan;

fn turn(content: &Content, house: &mut House, seed: u64) {
    if !house.calendar.is_winter() {
        house.calendar.begin_winter();
    }
    turn_the_year(
        content,
        house,
        Vec::new(),
        WinterPlan::default(),
        &mut Rng::from_seed(seed),
    );
}

/// Year 2's winter, Maren and Brannoc wed in year 1; the first seed that gives a child.
fn first_child(prepare: impl Fn(&mut House)) -> crate::hero::Hero {
    let (content, mut house) = house();
    let (maren, brannoc) = (id(&house.heroes, "Maren"), id(&house.heroes, "Brannoc"));
    form(&mut house.heroes, maren, brannoc, BondKind::Spouse, 1);
    house.calendar.begin_winter();
    house.calendar.begin_summer();
    house.calendar.begin_winter();
    prepare(&mut house);
    for seed in 0..200 {
        let mut copy = house.clone();
        if !births(&content, &mut copy, &mut Rng::from_seed(seed)).is_empty() {
            return copy.heroes.last().cloned().expect("a child");
        }
    }
    panic!("no birth in 200 seeds");
}

#[test]
fn a_newborn_carries_half_its_heavier_parents_marks_and_the_family_name() {
    let child = first_child(|house| {
        let maren = id(&house.heroes, "Maren");
        house.heroes[maren].marks = 5;
    });
    assert_eq!((child.marks, child.blood), (2, Blood::Family));
}

#[test]
fn a_child_of_two_outsiders_is_an_outsider() {
    let child = first_child(|house| {
        for name in ["Maren", "Brannoc"] {
            let h = id(&house.heroes, name);
            house.heroes[h].blood = Blood::Outsider;
        }
    });
    assert_eq!(child.blood, Blood::Outsider);
}

#[test]
fn a_wanderer_arrives_an_outsider() {
    let (content, mut house) = house();
    crate::wanderer::arrive(&content, &mut house, &mut Rng::from_seed(4));
    assert_eq!(house.heroes.last().map(|h| h.blood), Some(Blood::Outsider));
}

#[test]
fn the_turning_takes_the_toll_of_the_familys_marks_from_the_house() {
    let (content, clean) = house();
    let mut marked = clean.clone();
    let mut clean = clean;
    let maren = id(&marked.heroes, "Maren");
    marked.heroes[maren].marks = 8;
    turn(&content, &mut clean, 1);
    turn(&content, &mut marked, 1);
    // 8 marks over four.
    assert_eq!(marked.renown, clean.renown - 2);
}

#[test]
fn an_outsider_carrier_who_dies_on_a_quest_costs_the_house_nothing_more() {
    let (content, mut house) = house();
    let odo = id(&house.heroes, "Odo");
    house.heroes[odo].destiny.kind = Destiny::CarryTheHouse;
    house.heroes[odo].blood = Blood::Outsider;
    seat(&mut house, 0, &[odo]);
    house.board[0].quest.danger = 7; // the death roll always hits
    aim(&mut house, 0, [1, 1], -9);
    let renown = house.renown;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(8), 0, [1, 1]);
    assert_eq!(page.outcome, Outcome::Disaster);
    assert!(!house.heroes[odo].is_living());
    // The disaster's danger, and no carrier's loss.
    assert_eq!(house.renown, renown - 7);
}

#[test]
fn a_true_bred_strong_hero_brings_two_more_to_a_might_quest() {
    let (_content, mut house) = house();
    let maren = id(&house.heroes, "Maren");
    let might = QuestFacts {
        aptitude: Aptitude::Might,
        place: Place::ALL[0],
        tags: &[],
        door_lock: false,
    };
    let plain = member_power(&house.heroes[maren], might);
    house.heroes[maren].genes = [Some(Trait::Strong), Some(Trait::Strong)];
    assert_eq!(member_power(&house.heroes[maren], might), plain + 2);
}
