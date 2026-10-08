//! W8's heirs, each behaviour a test named as a sentence: the ranking, the names and
//! marks the death page reads, and what the choice does (SPEC §15.1-§15.2, §14.4).
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from SPEC.md,
//! CONSTANTS.md or the content — never computed by the code under test.

use jidousha::prelude::Rng;

use crate::bonds::form;
use crate::content::Content;
use crate::dream::Dream;
use crate::heirs::{choose, heir_buttons, heirs};
use crate::hero::{DeedKind, DreamFate, Heirloom, HeroId};
use crate::house::House;
use crate::ids::{BondKind, Destiny, DreamKind, Place};
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
fn a_child_created_after_a_grandchild_still_ranks_first() {
    let (_, mut heroes) = crate::testkit::founded();
    let garrick = id(&heroes, "Garrick");
    let mut late = heroes[id(&heroes, "Wren")].clone();
    late.name = "Late".to_owned();
    late.parents = [Some(garrick), None];
    late.bonds.clear();
    heroes.push(late);
    let child = heroes.len() - 1;
    form(&mut heroes, child, garrick, BondKind::Parent, 1);
    let list: Vec<&str> = heirs(&heroes, garrick)
        .iter()
        .map(|&h| heroes[h].name.as_str())
        .collect();
    assert_eq!(list[..3], ["Maren", "Late", "Pip"]);
}

#[test]
fn a_parent_known_only_by_the_parents_field_ranks_with_the_parents() {
    let (_, mut heroes) = crate::testkit::founded();
    let (wren, garrick) = (id(&heroes, "Wren"), id(&heroes, "Garrick"));
    heroes[wren].parents[1] = Some(garrick);
    let list: Vec<&str> = heirs(&heroes, wren)
        .iter()
        .map(|&h| heroes[h].name.as_str())
        .collect();
    // Garrick by the field, Maren a sister by him, Brannoc by the bond: all rank 3.
    assert_eq!(list[..3], ["Garrick", "Maren", "Brannoc"]);
}

#[test]
fn a_parent_known_only_by_the_bond_ranks_above_those_taught() {
    let (_, mut heroes) = crate::testkit::founded();
    let (wren, odo, pip) = (id(&heroes, "Wren"), id(&heroes, "Odo"), id(&heroes, "Pip"));
    form(&mut heroes, wren, odo, BondKind::Parent, 1);
    form(&mut heroes, pip, wren, BondKind::Mentor, 1);
    if let Some(b) = heroes[wren].bonds.iter_mut().find(|b| b.other == pip) {
        b.taught = true;
    }
    let list: Vec<&str> = heirs(&heroes, wren)
        .iter()
        .map(|&h| heroes[h].name.as_str())
        .collect();
    assert_eq!(list[..3], ["Brannoc", "Odo", "Pip"]);
}

#[test]
fn a_companion_is_named_of_the_house() {
    let (content, mut heroes) = crate::testkit::founded();
    let (garrick, ysolde) = (id(&heroes, "Garrick"), id(&heroes, "Ysolde"));
    form(&mut heroes, garrick, ysolde, BondKind::Companion, 1);
    let list = heirs(&heroes, garrick);
    let labels: Vec<String> = heir_buttons(&content, &heroes, garrick, &list)
        .into_iter()
        .map(|b| b.label)
        .collect();
    assert!(
        labels.contains(&"Ysolde, of the house".to_owned()),
        "{labels:?}"
    );
}

#[test]
fn no_one_is_marked_not_the_dream_when_the_dead_leave_no_dream() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    let ysolde = id(&house.heroes, "Ysolde");
    house.heroes[garrick]
        .dream
        .as_mut()
        .expect("a dream")
        .advance_to_stage(3);
    house.heroes[ysolde].burden = Some(dream(&content, DreamKind::SeeTheSea));
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    assert!(labels(&content, &house, page).contains(&"Ysolde, of the house".to_owned()));
}

#[test]
fn a_burden_and_an_own_dream_both_undone_leave_the_burden() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    let pip = id(&house.heroes, "Pip");
    house.heroes[garrick].burden = Some(dream(&content, DreamKind::KnownAtCourt));
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert!(lines[1].starts_with("He leaves a dream undone: to be known at Court."));
    choose(&content, &mut house, page, Some(pip));
    let burden = house.heroes[pip].burden.as_ref().expect("a burden");
    assert_eq!(burden.kind, DreamKind::KnownAtCourt);
}

#[test]
fn a_buried_heirloom_is_named_with_a_capital() {
    let (content, mut house) = house();
    let garrick = certain_death(&mut house, "Garrick");
    if let Some(h) = house.heroes[garrick].heirloom.as_mut() {
        h.name = "the Thorne cradle-ring".to_owned();
    }
    turn(&content, &mut house, 1);
    let page = page_of(&house, garrick);
    choose(&content, &mut house, page, None);
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert_eq!(
        lines[2],
        "The Thorne cradle-ring was laid in the ground with him."
    );
}

#[test]
fn a_dream_taken_up_is_told_about_its_owner() {
    let (content, mut house) = house();
    let odo = certain_death(&mut house, "Odo");
    let maren = id(&house.heroes, "Maren");
    house.heroes[odo].destiny.kind = Destiny::Unspoken;
    house.heroes[odo]
        .dream
        .as_mut()
        .expect("a dream")
        .advance_to_stage(2);
    turn(&content, &mut house, 1);
    let page = page_of(&house, odo);
    choose(&content, &mut house, page, Some(maren));
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert_eq!(
        lines[1],
        "Maren takes the dream up where Odo left it: see a student succeed without him."
    );
}

#[test]
fn the_choice_s_lines_come_before_the_door_s_promise() {
    let (content, mut house) = house();
    let ysolde = certain_death(&mut house, "Ysolde");
    let wren = id(&house.heroes, "Wren");
    turn(&content, &mut house, 1);
    let page = page_of(&house, ysolde);
    choose(&content, &mut house, page, Some(wren));
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert!(
        lines[1].starts_with("Wren takes the dream up where Ysolde left it"),
        "{lines:?}"
    );
    assert!(
        lines[2].starts_with("Ysolde was promised to the Sealed Door"),
        "{lines:?}"
    );
}

#[test]
fn a_dream_left_undone_is_told_about_the_dead_who_carried_it() {
    let (content, mut house) = house();
    let maren = certain_death(&mut house, "Maren");
    let odo = id(&house.heroes, "Odo");
    let mut student = house.heroes[odo].dream.clone().expect("a dream");
    student.advance_to_stage(2);
    student.owner = Some(odo);
    house.heroes[maren].burden = Some(student);
    house.heroes[maren].destiny.kind = Destiny::Unspoken;
    turn(&content, &mut house, 1);
    let page = page_of(&house, maren);
    let lines = &house.passage.as_ref().expect("a turning").pages[page].lines;
    assert_eq!(
        lines[0],
        "She leaves a dream undone: to find a worthy student. Still to do: see a student succeed without her."
    );
}
