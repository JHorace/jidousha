//! What a hero inherits (VARIANT.md): `inherit`, the death page's preview lines, the heir
//! choice's marks, and a child born under them. Expectations are shipped literals.

use jidousha::prelude::Rng;

use crate::bonds::form;
use crate::content::Content;
use crate::heirs::{choose, heir_lines};
use crate::hero::HeroId;
use crate::house::House;
use crate::ids::{BondKind, Place};
use crate::inheritance::inherit;
use crate::marks::Mark;
use crate::testkit::{house, id};
use crate::turning::turn_the_year;
use crate::winter::WinterPlan;

fn mark(origin: HeroId, year: i32, generation: i32) -> Mark {
    Mark {
        origin,
        year,
        place: Place::Barrow,
        generation,
    }
}

/// Garrick marked in year 1 and dying this turning.
fn marked_deathbed() -> (Content, House, HeroId, usize) {
    let (content, mut house) = house();
    let garrick = id(&house.heroes, "Garrick");
    house.heroes[garrick].marks.push(mark(garrick, 1, 0));
    house.heroes[garrick].age = 93;
    house.calendar.begin_winter();
    turn_the_year(
        &content,
        &mut house,
        Vec::new(),
        WinterPlan::default(),
        &mut Rng::from_seed(2),
    );
    let page = house
        .passage
        .as_ref()
        .expect("a turning")
        .pages
        .iter()
        .position(|p| p.about == Some(garrick))
        .expect("Garrick's page");
    (content, house, garrick, page)
}

#[test]
fn inheriting_takes_the_dead_heros_marks_a_generation_down_once_and_none_of_their_traits() {
    let (_, mut house) = house();
    let (garrick, maren, pip) = (
        id(&house.heroes, "Garrick"),
        id(&house.heroes, "Maren"),
        id(&house.heroes, "Pip"),
    );
    house.heroes[garrick].marks = vec![mark(garrick, 1, 0), mark(maren, 2, 1)];
    house.heroes[maren].marks = vec![mark(maren, 2, 0)];
    let carried = inherit(&house.heroes, garrick, maren);
    assert_eq!(carried.marks, [mark(maren, 2, 0), mark(garrick, 1, 1)]);
    assert_eq!(carried.taken, 1, "the second is hers already");
    assert_eq!(carried.traits, house.heroes[maren].traits);
    let carried = inherit(&house.heroes, garrick, pip);
    assert_eq!((carried.taken, carried.marks.len()), (2, 2));
    assert!(
        carried.traits.is_empty(),
        "Pip has none, and Garrick's Strong does not pass"
    );
}

#[test]
fn the_death_page_previews_each_candidates_traits_and_marks_and_where_no_one_leaves_them() {
    let (content, house, garrick, page) = marked_deathbed();
    let bequest = house.passage.as_ref().expect("a turning").pages[page]
        .bequest
        .clone()
        .expect("a death page");
    assert!(bequest.leaves, "a marked death page waits");
    let lines = heir_lines(&content, &house.heroes, garrick, &bequest.heirs);
    assert_eq!(
        lines,
        [
            "Maren, daughter: Sharp; marks 1 (+1)",
            "Pip, grandson: no trait; marks 1 (+1)",
            "Odo, friend: Steadfast; marks 1 (+1)",
            "Ysolde, of the house: no trait; marks 1 (+1)",
            "Brannoc, of the house: Strong; marks 1 (+1)",
            "Wren, of the house: no trait; marks 1 (+1)",
            "No one: the marks fall to Maren, blood of the name.",
        ]
    );
}

#[test]
fn choosing_an_heir_gives_them_the_marks_exactly_as_previewed_and_costs_them_a_renown() {
    let (content, mut house, garrick, page) = marked_deathbed();
    let maren = id(&house.heroes, "Maren");
    let renown = house.heroes[maren].renown;
    choose(&content, &mut house, page, Some(maren));
    assert_eq!(house.heroes[maren].marks, [mark(garrick, 1, 1)]);
    assert_eq!(house.heroes[maren].traits, [crate::ids::Trait::Sharp]);
    assert_eq!(house.heroes[maren].renown, renown - 1);
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert!(
        lines.contains(&"Maren takes the name's shame with the rest: a mark.".to_owned()),
        "{lines:?}"
    );
    assert_eq!(house.marks_carried(), [mark(garrick, 1, 1)]);
}

#[test]
fn choosing_no_one_sends_the_marks_to_the_blood_of_the_name_and_the_ground_when_there_is_none() {
    let (content, mut house, garrick, page) = marked_deathbed();
    let maren = id(&house.heroes, "Maren");
    choose(&content, &mut house, page, None);
    assert_eq!(house.heroes[maren].marks, [mark(garrick, 1, 1)]);
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert!(
        lines.contains(&"Garrick's mark falls to Maren, blood of the name.".to_owned()),
        "{lines:?}"
    );
    // Without the daughter living the shame is buried with him.
    let (content, mut house) = house_with_maren_dead();
    let garrick = id(&house.heroes, "Garrick");
    let page = house
        .passage
        .as_ref()
        .expect("a turning")
        .pages
        .iter()
        .position(|p| p.about == Some(garrick))
        .expect("Garrick's page");
    choose(&content, &mut house, page, None);
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert!(
        lines.contains(&"Garrick's shame goes into the ground with him.".to_owned()),
        "{lines:?}"
    );
    assert!(house.marks_carried().is_empty());
}

fn house_with_maren_dead() -> (Content, House) {
    let (content, mut house) = house();
    let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
    house.heroes[garrick].marks.push(mark(garrick, 1, 0));
    house.heroes[garrick].age = 93;
    house.heroes[maren].fate = crate::hero::Fate::Dead;
    house.unseat(maren);
    house.calendar.begin_winter();
    turn_the_year(
        &content,
        &mut house,
        Vec::new(),
        WinterPlan::default(),
        &mut Rng::from_seed(2),
    );
    (content, house)
}

#[test]
fn a_child_is_born_under_the_marks_both_parents_carry_counted_once() {
    let (content, mut house) = house();
    let (maren, brannoc) = (id(&house.heroes, "Maren"), id(&house.heroes, "Brannoc"));
    form(&mut house.heroes, maren, brannoc, BondKind::Spouse, 1);
    house.heroes[maren].marks.push(mark(maren, 1, 0));
    house.heroes[brannoc].marks.push(mark(maren, 1, 1));
    house.calendar.begin_winter();
    house.calendar.begin_summer();
    house.calendar.begin_winter();
    for seed in 0..200 {
        let mut copy = house.clone();
        let pages = crate::births::births(&content, &mut copy, &mut Rng::from_seed(seed));
        let Some(page) = pages.first() else { continue };
        let child = page.about.expect("a birth page is about the child");
        assert_eq!(copy.heroes[child].marks, [mark(maren, 1, 1)]);
        assert!(
            page.lines
                .iter()
                .any(|l| l.ends_with("is born under a mark on the name.")),
            "{:?}",
            page.lines
        );
        assert_eq!(copy.marks_carried(), [mark(maren, 1, 0)]);
        return;
    }
    panic!("no birth in 200 seeds");
}

#[test]
fn a_candidate_who_already_carries_one_of_two_marks_shows_two_with_one_new_and_pays_for_one() {
    let (content, mut house, garrick, page) = marked_deathbed();
    let maren = id(&house.heroes, "Maren");
    house.heroes[garrick].marks.push(mark(maren, 2, 0));
    house.heroes[maren].marks.push(mark(maren, 2, 0));
    let bequest = house.passage.as_ref().expect("a turning").pages[page]
        .bequest
        .clone()
        .expect("a death page");
    let lines = heir_lines(&content, &house.heroes, garrick, &bequest.heirs);
    assert_eq!(lines[0], "Maren, daughter: Sharp; marks 2 (+1)");
    assert_eq!(lines[1], "Pip, grandson: no trait; marks 2 (+2)");
    let renown = house.heroes[maren].renown;
    choose(&content, &mut house, page, Some(maren));
    assert_eq!(house.heroes[maren].renown, renown - 1);
    let pip = id(&house.heroes, "Pip");
    let (content, mut house, garrick, page) = marked_deathbed();
    house.heroes[garrick].marks.push(mark(garrick, 2, 0));
    house.heroes[pip].renown = 5;
    choose(&content, &mut house, page, Some(pip));
    assert_eq!(
        house.heroes[pip].renown, 3,
        "a renown for each of two marks"
    );
}

#[test]
fn marks_are_told_apart_by_place_and_by_year() {
    let (_, mut house) = house();
    let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
    house.heroes[maren].marks.push(mark(garrick, 1, 0));
    house.heroes[garrick].marks.push(Mark {
        place: Place::HighPass,
        ..mark(garrick, 1, 0)
    });
    assert_eq!(inherit(&house.heroes, garrick, maren).taken, 1);
    house.heroes[garrick].marks = vec![mark(garrick, 2, 0)];
    assert_eq!(inherit(&house.heroes, garrick, maren).taken, 1);
}

#[test]
fn a_death_page_with_only_a_mark_to_leave_still_waits() {
    let (content, mut house) = house();
    let garrick = id(&house.heroes, "Garrick");
    house.heroes[garrick].heirloom = None;
    house.heroes[garrick].dream = None;
    house.heroes[garrick].marks.push(mark(garrick, 1, 0));
    house.heroes[garrick].age = 93;
    house.calendar.begin_winter();
    turn_the_year(
        &content,
        &mut house,
        Vec::new(),
        WinterPlan::default(),
        &mut Rng::from_seed(2),
    );
    let page = house
        .passage
        .as_ref()
        .expect("a turning")
        .pages
        .iter()
        .find(|p| p.about == Some(garrick))
        .expect("a page");
    assert!(page.bequest.as_ref().expect("a bequest").undecided());
}

#[test]
fn a_child_is_born_under_the_second_parents_marks_too() {
    let (content, mut house) = house();
    let (maren, brannoc) = (id(&house.heroes, "Maren"), id(&house.heroes, "Brannoc"));
    form(&mut house.heroes, maren, brannoc, BondKind::Spouse, 1);
    house.heroes[brannoc].marks.push(mark(brannoc, 1, 0));
    house.calendar.begin_winter();
    house.calendar.begin_summer();
    house.calendar.begin_winter();
    for seed in 0..200 {
        let mut copy = house.clone();
        let pages = crate::births::births(&content, &mut copy, &mut Rng::from_seed(seed));
        if let Some(page) = pages.first() {
            let child = page.about.expect("a child");
            assert_eq!(copy.heroes[child].marks, [mark(brannoc, 1, 1)]);
            return;
        }
    }
    panic!("no birth");
}
