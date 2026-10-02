//! W8's rules, each behaviour a test named as a sentence (SPEC §15, §17, §18).
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from SPEC.md,
//! CONSTANTS.md or the content — never computed by the code under test.

use jidousha::prelude::Rng;

use crate::bonds::form;
use crate::content::Content;
use crate::dream::Dream;
use crate::heirs::{choose, heir_buttons, heirs};
use crate::hero::{DeedKind, DreamFate, Fate, Heirloom, HeroId};
use crate::house::House;
use crate::ids::{BondKind, Destiny, DreamKind, Place, Tag};
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

fn names(house: &House, list: &[HeroId]) -> Vec<String> {
    list.iter().map(|&h| house.heroes[h].name.clone()).collect()
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

fn blade(name: &str) -> Heirloom {
    Heirloom {
        name: name.to_owned(),
        sprite: "reward-sword".to_owned(),
        aptitude: crate::ids::Aptitude::Might,
        bonus: 2,
        provenance: String::new(),
    }
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
fn an_heir_who_carries_a_burden_is_marked_not_the_dream_and_one_holding_an_heirloom_lays_one_aside()
{
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    let (ysolde, brannoc, odo) = (
        id(&house.heroes, "Ysolde"),
        id(&house.heroes, "Brannoc"),
        id(&house.heroes, "Odo"),
    );
    house.heroes[ysolde].burden = Some(dream(&content, DreamKind::SeeTheSea));
    // A fulfilled burden takes nothing from the dream; the blade is laid aside.
    let mut done = dream(&content, DreamKind::RoofOfTheWorld);
    done.advance_to_stage(3);
    house.heroes[brannoc].burden = Some(done);
    house.heroes[brannoc].heirloom = Some(blade("Emberwake"));
    // A burden and an heirloom: "(not the dream)" only.
    house.heroes[odo].burden = Some(dream(&content, DreamKind::KnownAtCourt));
    house.heroes[odo].heirloom = Some(blade("Grieftaker"));
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    assert_eq!(
        labels(&content, &house, page),
        [
            "Maren, daughter",
            "Pip, grandson",
            "Odo, friend (not the dream)",
            "Ysolde, of the house (not the dream)",
            "Brannoc, of the house (lays one aside)",
            "Wren, of the house",
            "No one. Let it lie."
        ]
    );
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
fn choosing_maren_gives_her_thornfall_and_garricks_dream_as_her_burden_right_after_the_bequest_lines()
 {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    let maren = id(&house.heroes, "Maren");
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    choose(&content, &mut house, page, Some(maren));
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert_eq!(
        lines[2],
        "Maren takes the dream up where Garrick left it: win a triumph at the Barrow."
    );
    assert_eq!(lines[3], "Maren grieves for her father. Dread 3 of 5.");
    let her = &house.heroes[maren];
    assert_eq!(
        her.heirloom.as_ref().map(|h| h.name.as_str()),
        Some("Thornfall")
    );
    let burden = her.burden.as_ref().expect("a burden");
    assert_eq!(
        (burden.kind, burden.owner, burden.current),
        (DreamKind::QuietTheBarrow, Some(garrick), 2)
    );
    assert_eq!(
        her.dream.as_ref().map(|d| d.kind),
        Some(DreamKind::AvengeTheLost)
    );
    let kinds: Vec<(DeedKind, Option<HeroId>)> =
        her.deeds.iter().map(|d| (d.kind, d.other)).collect();
    assert_eq!(
        kinds,
        [
            (DeedKind::Inherited, Some(garrick)),
            (DeedKind::TookUpDream, Some(garrick))
        ]
    );
    let dead = &house.heroes[garrick];
    assert!(dead.heirloom.is_none() && dead.bequest_decided);
    assert_eq!(
        (dead.bequest_heir, dead.dream_fate),
        (Some(maren), DreamFate::PassedOn)
    );
    assert_eq!(
        house.passage.as_ref().expect("a turning").first_undecided(),
        None
    );
    assert!(house.ghosts.is_empty());
}

#[test]
fn an_heir_with_no_dream_takes_the_dream_as_their_own_dream() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    let wren = id(&house.heroes, "Wren");
    turn(&content, &mut house, 1);
    let at = page_of(&house, garrick);
    choose(&content, &mut house, at, Some(wren));
    let taken = house.heroes[wren].dream.as_ref().expect("a dream");
    assert_eq!(
        (taken.kind, taken.owner),
        (DreamKind::QuietTheBarrow, Some(garrick))
    );
    assert!(house.heroes[wren].burden.is_none());
}

#[test]
fn an_heir_who_cannot_take_the_dream_gets_the_heirloom_and_the_dream_walks_as_a_ghost() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    let ysolde = id(&house.heroes, "Ysolde");
    house.heroes[ysolde].burden = Some(dream(&content, DreamKind::SeeTheSea));
    house.heroes[ysolde].heirloom = Some(blade("Emberwake"));
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    choose(&content, &mut house, page, Some(ysolde));
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert_eq!(
        lines[2..4],
        [
            "Ysolde lays Emberwake aside, over the hearth.",
            "No one takes up the dream, and it does not go into the ground with him. There will \
             be a quest at the Barrow: lay Garrick's ghost."
        ]
    );
    assert_eq!(
        house.heroes[ysolde]
            .heirloom
            .as_ref()
            .map(|h| h.name.as_str()),
        Some("Thornfall")
    );
    assert_eq!(
        house.heroes[ysolde].burden.as_ref().map(|d| d.kind),
        Some(DreamKind::SeeTheSea)
    );
    assert_eq!(house.heroes[garrick].dream_fate, DreamFate::LeftToNoOne);
    let ghost = &house.ghosts[0];
    assert_eq!(
        (ghost.hero, ghost.place, ghost.dream.owner),
        (garrick, Place::Barrow, Some(garrick))
    );
}

#[test]
fn no_one_buries_the_heirloom_with_the_dead_and_raises_the_ghost() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    choose(&content, &mut house, page, None);
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert_eq!(lines[2], "Thornfall was laid in the ground with him.");
    assert!(lines[3].starts_with("No one takes up the dream"));
    assert!(house.heroes.iter().all(|h| h.heirloom.is_none()));
    assert_eq!(
        (
            house.heroes[garrick].bequest_heir,
            house.heroes[garrick].bequest_decided
        ),
        (None, true)
    );
    assert_eq!(house.ghosts.len(), 1);
}

#[test]
fn a_ghost_walks_where_its_dream_names_or_where_the_dead_fell_or_at_the_barrow() {
    let (content, mut house) = house();
    let (odo, brannoc, maren) = (
        id(&house.heroes, "Odo"),
        id(&house.heroes, "Brannoc"),
        id(&house.heroes, "Maren"),
    );
    // Odo's WORTHY_STUDENT: where he fell questing, at the High Pass.
    house.heroes[odo].age = 93;
    house.heroes[odo].destiny.kind = Destiny::Unspoken;
    house.heroes[odo].death_place = Some(Place::HighPass);
    // Brannoc's FORGE_A_BLADE: Emberfall, wherever he died.
    house.heroes[brannoc].age = 93;
    house.heroes[brannoc].destiny.kind = Destiny::Unspoken;
    // Maren's AVENGE_THE_LOST: the avenged place, the Drowned Coast.
    house.heroes[maren].age = 93;
    turn(&content, &mut house, 1);
    for dead in [odo, brannoc, maren] {
        let at = page_of(&house, dead);
        choose(&content, &mut house, at, None);
    }
    let places: Vec<(HeroId, Place)> = house.ghosts.iter().map(|g| (g.hero, g.place)).collect();
    assert!(places.contains(&(odo, Place::HighPass)));
    assert!(places.contains(&(brannoc, Place::Emberfall)));
    assert!(places.contains(&(maren, Place::DrownedCoast)));
    // Dead at the Door, or in bed: the Barrow.
    let (content, mut house) = crate::testkit::house();
    let odo = certain_death(&mut house, "Odo");
    house.heroes[odo].destiny.kind = Destiny::Unspoken;
    house.heroes[odo].death_place = Some(Place::SealedDoor);
    turn(&content, &mut house, 1);
    let at = page_of(&house, odo);
    choose(&content, &mut house, at, None);
    assert_eq!(house.ghosts[0].place, Place::Barrow);
}

#[test]
#[should_panic(expected = "waits for none")]
fn a_death_page_is_chosen_on_once() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    choose(&content, &mut house, page, None);
    choose(&content, &mut house, page, None);
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
fn heirs_rank_spouse_above_parents_and_siblings_above_the_taught() {
    let (_, mut heroes) = crate::testkit::founded();
    let (maren, garrick, pip, ysolde, odo, wren) = (
        id(&heroes, "Maren"),
        id(&heroes, "Garrick"),
        id(&heroes, "Pip"),
        id(&heroes, "Ysolde"),
        id(&heroes, "Odo"),
        id(&heroes, "Wren"),
    );
    form(&mut heroes, maren, odo, BondKind::Spouse, 1);
    // Wren a daughter of Garrick's too: Maren's half-sister by a shared parent.
    heroes[wren].parents[1] = Some(garrick);
    // Maren taught Ysolde.
    form(&mut heroes, ysolde, maren, BondKind::Mentor, 1);
    if let Some(b) = heroes[maren].bonds.iter_mut().find(|b| b.other == ysolde) {
        b.taught = true;
    }
    let list: Vec<&str> = heirs(&heroes, maren)
        .iter()
        .map(|&h| heroes[h].name.as_str())
        .collect();
    // Pip (child), Odo (spouse), Garrick (parent), Wren (a sister by Garrick), Ysolde (taught).
    assert_eq!(list, ["Pip", "Odo", "Garrick", "Wren", "Ysolde", "Brannoc"]);
    let _ = pip;
}

#[test]
fn an_heir_is_named_by_their_kinship_to_the_dead() {
    let (content, mut heroes) = crate::testkit::founded();
    let (maren, garrick, odo, wren, ysolde) = (
        id(&heroes, "Maren"),
        id(&heroes, "Garrick"),
        id(&heroes, "Odo"),
        id(&heroes, "Wren"),
        id(&heroes, "Ysolde"),
    );
    form(&mut heroes, maren, odo, BondKind::Spouse, 1);
    heroes[wren].parents[1] = Some(garrick);
    form(&mut heroes, ysolde, maren, BondKind::Mentor, 1);
    let list = heirs(&heroes, maren);
    let labels: Vec<String> = heir_buttons(&content, &heroes, maren, &list)
        .into_iter()
        .map(|b| b.label)
        .collect();
    assert_eq!(
        labels,
        [
            "Pip, son",
            "Odo, husband",
            "Garrick, father",
            "Wren, sister",
            "Ysolde, student",
            "Brannoc, of the house",
            "No one. Let it lie."
        ]
    );
}

#[test]
fn the_heir_list_holds_at_most_eight() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    let odo = id(&house.heroes, "Odo");
    for _ in 0..6 {
        let mut copy = house.heroes[odo].clone();
        copy.bonds.clear();
        copy.age = 30;
        house.heroes.push(copy);
    }
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    let bequest = house.passage.as_ref().expect("a turning").pages[page]
        .bequest
        .clone()
        .expect("a bequest");
    assert_eq!(bequest.heirs.len(), 8);
    assert_eq!(names(&house, &bequest.heirs)[..3], ["Maren", "Pip", "Odo"]);
    assert_eq!(labels(&content, &house, page).len(), 9);
}

#[test]
fn passing_a_dream_on_makes_dream_rivals_of_those_who_want_the_same() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    let (wren, ysolde) = (id(&house.heroes, "Wren"), id(&house.heroes, "Ysolde"));
    house.heroes[ysolde].dream = Some(dream(&content, DreamKind::QuietTheBarrow));
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    choose(&content, &mut house, page, Some(wren));
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert!(
        lines[3].starts_with("Wren wants what Ysolde wants"),
        "{lines:?}"
    );
    assert_eq!(
        house.heroes[wren].bond_to(ysolde).map(|b| b.kind),
        Some(BondKind::Rival)
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
