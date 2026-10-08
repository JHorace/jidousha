//! W8's rules, each behaviour a test named as a sentence (SPEC §15, §17, §18).
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from SPEC.md,
//! CONSTANTS.md or the content — never computed by the code under test.

use jidousha::prelude::Rng;

use crate::content::Content;
use crate::dream::Dream;
use crate::heirs::{choose, heir_buttons};
use crate::hero::{DreamFate, Fate, HeroId};
use crate::house::House;
use crate::ids::{Destiny, DreamKind, Tag};
use crate::passage::PageKind;
use crate::testkit::{house, id};
use crate::turning::turn_the_year;
use crate::winter::WinterPlan;

/// Into the winter, and the year turned with no winter lines.
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

/// Old age takes `name` this turning, for certain: 93, and 94 after the turning's +1.
fn certain_death(house: &mut House, name: &str) -> HeroId {
    let hero = id(&house.heroes, name);
    house.heroes[hero].age = 93;
    hero
}

/// The page about `hero`.
fn page_of(house: &House, hero: HeroId) -> usize {
    let passage = house.passage.as_ref().expect("a turning");
    passage
        .pages
        .iter()
        .position(|p| p.about == Some(hero))
        .expect("a page about them")
}

fn labels(content: &Content, house: &House, page: usize) -> Vec<String> {
    let bequest = house.passage.as_ref().expect("a turning").pages[page]
        .bequest
        .clone()
        .expect("a death page");
    heir_buttons(content, &house.heroes, bequest.dead, &bequest.heirs)
        .into_iter()
        .map(|b| b.label)
        .collect()
}

fn dream(content: &Content, kind: DreamKind) -> Dream {
    Dream::build(content, kind, None, None).expect("builds")
}

#[test]
fn when_garrick_dies_of_old_age_his_page_offers_maren_pip_odo_then_the_house_in_founding_order() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    let passage = house.passage.as_ref().expect("a turning");
    assert_eq!(passage.pages[page].kind, PageKind::Death);
    assert_eq!(passage.pages[page].title, "In memory of Garrick Thorne");
    assert_eq!(
        labels(&content, &house, page),
        [
            "Maren, daughter",
            "Pip, grandson",
            "Odo, friend",
            "Ysolde, of the house",
            "Brannoc, of the house",
            "Wren, of the house",
            "No one. Let it lie."
        ]
    );
    assert_eq!(passage.first_undecided(), Some(page));
    assert_eq!(house.heroes[garrick].fate_age, 94);
}

#[test]
fn the_death_page_tells_the_heirloom_and_the_dream_left_before_the_grief() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert_eq!(
        lines[..4],
        [
            "He leaves Thornfall. +1 Might on quests.",
            "He leaves a dream undone: to lay the Barrow's dead to rest. Still to do: win a \
             triumph at the Barrow.",
            "Maren grieves for her father. Dread 3 of 5.",
            "Odo grieves for his friend. Dread 3 of 5."
        ]
    );
    assert!(house.heroes[garrick].grieved);
    assert!(house.mourned.is_empty());
    assert_eq!(
        house.heroes[garrick].bequest_heirloom.as_deref(),
        Some("Thornfall")
    );
}

#[test]
#[should_panic(expected = "a death page undecided")]
fn the_year_does_not_turn_while_a_death_page_waits() {
    let (content, mut house) = house();
    certain_death(&mut house, "Garrick");
    turn(&content, &mut house, 1);
    crate::season::summer_comes(&content, &mut house, &mut Rng::from_seed(2));
}

#[test]
fn a_death_with_nothing_to_leave_is_decided_on_its_page() {
    let (content, mut house) = house();
    let wren = id(&house.heroes, "Wren");
    house.heroes[wren].age = 93;
    turn(&content, &mut house, 1);
    let page = page_of(&house, wren);
    let passage = house.passage.as_ref().expect("a turning");
    assert_eq!(passage.first_undecided(), None);
    assert_eq!(house.heroes[wren].dream_fate, DreamFate::NeverDreamt);
    assert!(house.heroes[wren].bequest_decided);
    assert_eq!(
        passage.pages[page].lines[0],
        "Brannoc grieves for his daughter. Dread 2 of 5."
    );
}

#[test]
fn a_fulfilled_dream_is_fulfilled_and_a_burden_left_undone_is_the_one_that_passes() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    let pip = id(&house.heroes, "Pip");
    house.heroes[garrick]
        .dream
        .as_mut()
        .expect("a dream")
        .advance_to_stage(3);
    house.heroes[garrick].burden = Some(dream(&content, DreamKind::KnownAtCourt));
    turn(&content, &mut house, 1);
    assert_eq!(house.heroes[garrick].dream_fate, DreamFate::Fulfilled);
    let page = page_of(&house, garrick);
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert!(lines[1].starts_with("He leaves a dream undone: to be known at Court."));
    choose(&content, &mut house, page, Some(pip));
    assert_eq!(
        house.heroes[pip].burden.as_ref().map(|d| d.kind),
        Some(DreamKind::KnownAtCourt)
    );
}

#[test]
fn the_summers_dead_get_their_pages_first_and_are_not_grieved_again() {
    let (content, mut house) = house();
    let (brannoc, garrick) = (
        id(&house.heroes, "Brannoc"),
        certain_death(&mut house, "Garrick"),
    );
    house.heroes[brannoc].fate = Fate::Dead;
    house.heroes[brannoc].grieved = true;
    house.mourned.push(brannoc);
    turn(&content, &mut house, 1);
    let pages = &house.passage.as_ref().expect("a turning").pages;
    let deaths: Vec<Option<HeroId>> = pages
        .iter()
        .filter(|p| p.kind == PageKind::Death)
        .map(|p| p.about)
        .collect();
    assert_eq!(deaths, [Some(brannoc), Some(garrick)]);
    assert!(!pages[1].lines.iter().any(|l| l.contains("grieves")));
}

#[test]
fn old_age_tells_a_sleep_death_takes_a_carrier_s_renown_silently_and_spares_the_fire_doomed() {
    let (content, mut house) = house();
    let odo = certain_death(&mut house, "Odo");
    house.heroes[odo].destiny.kind = Destiny::CarryTheHouse;
    let brannoc = certain_death(&mut house, "Brannoc"); // FIRE_WILL_END_YOU
    let renown = house.renown;
    turn(&content, &mut house, 1);
    assert_eq!(house.heroes[odo].fate, Fate::Dead);
    assert!(
        content.pools[crate::ids::Pool::SleepDeaths.index()]
            .contains(&house.heroes[odo].fate_telling)
    );
    assert_eq!(house.renown, renown - 4);
    assert!(house.heroes[odo].death_place.is_none());
    assert_eq!(house.heroes[brannoc].fate, Fate::Living);
    let all: Vec<String> = house
        .passage
        .as_ref()
        .expect("a turning")
        .pages
        .iter()
        .flat_map(|p| p.lines.clone())
        .collect();
    assert!(!all.iter().any(|l| l.contains("carried the house")));
}

#[test]
fn everyone_living_is_a_year_older_and_the_dead_are_not() {
    let (content, mut house) = house();
    let before: Vec<i32> = house.heroes.iter().map(|h| h.age).collect();
    turn(&content, &mut house, 7);
    for (id, hero) in house.heroes.iter().enumerate().take(before.len()) {
        let want = if hero.fate == Fate::Living || hero.fate_year == 1 {
            before[id] + 1
        } else {
            before[id]
        };
        assert_eq!(hero.age, want, "{}", hero.name);
    }
}

#[test]
fn the_turning_closes_on_the_door_countdown_and_in_year_25_on_the_door_open() {
    let (content, mut house) = house();
    turn(&content, &mut house, 3);
    let last = |house: &House| {
        house
            .passage
            .as_ref()
            .expect("a turning")
            .pages
            .last()
            .expect("a page")
            .lines
            .last()
            .cloned()
    };
    assert_eq!(
        last(&house).as_deref(),
        Some("The year turns. The Sealed Door opens in 24 years.")
    );
    let (content, mut house) = crate::testkit::house();
    for _ in 0..23 {
        house.calendar.begin_winter();
        house.calendar.begin_summer();
    }
    turn(&content, &mut house, 3);
    assert_eq!(house.calendar.current_year(), 24);
    assert_eq!(
        last(&house).as_deref(),
        Some("The year turns. The Sealed Door opens in 1 year.")
    );
    house.calendar.begin_summer();
    turn(&content, &mut house, 3);
    assert_eq!(
        last(&house).as_deref(),
        Some("The year turns. The Sealed Door stands open. It asks for four.")
    );
}

#[test]
fn the_house_tales_are_told_at_every_turning_for_a_renown_each() {
    let (content, mut house) = house();
    let pip = id(&house.heroes, "Pip");
    let tale = |title: &str| crate::house::Tale {
        title: title.to_owned(),
        about: pip,
        since: 1,
    };
    house.tales.push(tale("The tale of Pip and the sea"));
    let renown = house.renown;
    turn(&content, &mut house, 3);
    let year = house
        .passage
        .as_ref()
        .expect("a turning")
        .pages
        .iter()
        .find(|p| p.kind == PageKind::Year)
        .expect("a year page")
        .clone();
    assert_eq!(year.title, "Year 2 begins");
    assert_eq!(
        year.lines[0],
        "The tale of Pip and the sea is still told in the valley. +1 renown."
    );
    assert_eq!(house.renown, renown + 1);
    let (content, mut house) = crate::testkit::house();
    house.tales.push(tale("The tale of Pip and the sea"));
    house.tales.push(tale("The laying of Garrick's ghost"));
    turn(&content, &mut house, 3);
    let year = house
        .passage
        .as_ref()
        .expect("a turning")
        .pages
        .iter()
        .find(|p| p.kind == PageKind::Year)
        .expect("a year page")
        .clone();
    assert_eq!(
        year.lines[0],
        "The valley tells 2 tales of the house now, and the newest is the laying of Garrick's ghost. +2 renown."
    );
    assert_eq!(house.renown, 17);
}

#[test]
fn a_hero_entering_a_new_phase_is_told_it_but_never_on_becoming_a_youth() {
    let (content, mut house) = house();
    let (odo, maren, pip) = (
        id(&house.heroes, "Odo"),
        id(&house.heroes, "Maren"),
        id(&house.heroes, "Pip"),
    );
    house.heroes[odo].age = 54;
    house.heroes[maren].age = 39;
    house.heroes[pip].age = 11;
    turn(&content, &mut house, 3);
    let year = house
        .passage
        .as_ref()
        .expect("a turning")
        .pages
        .iter()
        .find(|p| p.kind == PageKind::Year)
        .expect("a year page")
        .clone();
    assert_eq!(
        year.lines[..2],
        [
            "Maren is 40: a veteran now. Her Might fades by 1 and her Wits grows by 1. She teaches well now.",
            "Odo is 55: an elder now. His Might fades by 2 and his Wits grows by 1. He teaches well, and each winter may be his last."
        ]
    );
}

#[test]
fn the_door_s_promisee_dying_of_age_passes_the_promise_on_her_page() {
    let (content, mut house) = house();
    let ysolde = certain_death(&mut house, "Ysolde");
    turn(&content, &mut house, 1);
    let page = page_of(&house, ysolde);
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert!(
        lines.contains(
            &"Ysolde was promised to the Sealed Door, and has no child living. The Door will open \
          for no one by name."
                .to_owned()
        )
    );
    let _ = Tag::Dark;
}
