//! The resolution battery's unit half, part two: what a quest does to the people on it
//! (SPEC §7.4) — wounds, the death roll, mending, burning, crowning — and the heir list
//! the crowned leave an heirloom by (§15.1, §15.3). Shipped literals, as part one.

use jidousha::prelude::Rng;

use crate::hero::{DeedKind, Fate};
use crate::house::House;
use crate::ids::{Destiny, Outcome, Place, Pool};
use crate::resolve::resolve_rolled;
use crate::testkit::{aim, house, id, pooled, seat, stage};

#[test]
fn a_setback_wounds_the_unlucky_one_and_a_second_wound_kills_unless_the_seer_shields() {
    let (content, mut house) = house();
    let odo = id(&house.heroes, "Odo");
    seat(&mut house, 0, &[odo]);
    aim(&mut house, 0, [1, 2], -1);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(7), 0, [1, 2]);
    assert_eq!(page.outcome, Outcome::Setback);
    let wounds = pooled(&content, Pool::Wounds, &["Odo"]);
    assert!(
        page.lines.iter().any(|l| wounds.contains(l)),
        "{:?}",
        page.lines
    );
    assert!(house.heroes[odo].wounded);
    let deed = house.heroes[odo]
        .deeds
        .iter()
        .find(|d| d.kind == DeedKind::Wounded)
        .expect("wounded deed");
    assert_eq!(deed.telling, "was wounded at the Barrow");
    // Again, wounded: the second wound is the last.
    seat(&mut house, 0, &[odo]);
    aim(&mut house, 0, [1, 2], -1);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(7), 0, [1, 2]);
    assert!(page.lines.contains(
        &"Odo Fenn was already hurt, and went anyway. The second wound was the last. He was 47.".to_owned()
    ), "{:?}", page.lines);
    let odo_now = &house.heroes[odo];
    assert_eq!(
        (odo_now.fate, odo_now.fate_year, odo_now.fate_age),
        (Fate::Dead, 1, 47)
    );
    assert_eq!(
        odo_now.fate_telling,
        "died of wounds on the road home from the Barrow"
    );
    assert_eq!(house.mourned, [odo]);
    assert!(house.fallen_at(Place::Barrow).contains(&odo));
    assert_eq!(house.slot_of(odo), None);
    // A bed-promised hero is only hurt again.
    let (content, mut house) = crate::testkit::house();
    let odo = id(&house.heroes, "Odo");
    house.heroes[odo].destiny.kind = Destiny::DieInYourBed;
    house.heroes[odo].wounded = true;
    seat(&mut house, 0, &[odo]);
    aim(&mut house, 0, [1, 2], -4);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(7), 0, [1, 2]);
    assert!(page.lines.contains(
        &"Odo was hurt again, badly. It is not the end the Seer spoke of, so it is not the end.".to_owned()
    ));
    assert!(house.heroes[odo].is_living());
}

#[test]
fn a_disaster_death_roll_that_hits_kills_tells_it_and_grieves_on_the_same_page() {
    let (content, mut house) = house();
    let (odo, garrick) = (id(&house.heroes, "Odo"), id(&house.heroes, "Garrick"));
    house.heroes[odo].destiny.kind = Destiny::CarryTheHouse;
    seat(&mut house, 0, &[odo]);
    house.board[0].quest.danger = 7; // min(7 * 0.15, 1): the roll always hits.
    aim(&mut house, 0, [1, 1], -9);
    let renown = house.renown;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(8), 0, [1, 1]);
    assert_eq!(page.outcome, Outcome::Disaster);
    let deaths = pooled(&content, Pool::QuestDeaths, &["Odo Fenn", "the Barrow"]);
    let at = page
        .lines
        .iter()
        .position(|l| deaths.iter().any(|d| *l == format!("{d} He was 47.")))
        .expect("the death line");
    assert_eq!(
        page.lines[at + 1..at + 3],
        [
            "Odo carried the house, and has set it down. The house loses 4 renown.".to_owned(),
            "Garrick grieves for his friend. Dread 3 of 5.".to_owned(),
        ]
    );
    assert_eq!(house.renown, renown - 7 - 4);
    assert_eq!(house.heroes[odo].fate_telling, "fell at the Barrow");
    assert_eq!(house.heroes[odo].death_tag, Some(crate::ids::Tag::Dark));
    assert_eq!(house.heroes[garrick].fear.dread, 3);
    assert!(
        !house.heroes[odo]
            .deeds
            .iter()
            .any(|d| d.kind == DeedKind::SurvivedDisaster)
    );
}

#[test]
fn a_shielded_hero_survives_the_death_roll_is_wounded_and_records_the_disaster() {
    let (content, mut house) = house();
    let brannoc = id(&house.heroes, "Brannoc"); // FIRE_WILL_END_YOU; Grave goods is not Fire.
    seat(&mut house, 0, &[brannoc]);
    house.board[0].quest.danger = 7;
    aim(&mut house, 0, [1, 1], -9);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(9), 0, [1, 1]);
    assert!(
        page.lines.contains(
            &"Brannoc should have died there. It was not fire, so he did not.".to_owned()
        ),
        "{:?}",
        page.lines
    );
    let b = &house.heroes[brannoc];
    assert!(b.is_living() && b.wounded);
    assert!(b.deeds.iter().any(|d| d.kind == DeedKind::SurvivedDisaster));
}

#[test]
fn maren_is_mended_by_her_first_disaster_and_dies_by_none_of_it() {
    let (content, mut house) = house();
    let maren = id(&house.heroes, "Maren"); // BREAK_AND_BE_MENDED, Water at dread 1.
    seat(&mut house, 0, &[maren]);
    house.board[0].quest.danger = 7;
    aim(&mut house, 0, [1, 1], -9);
    let before = house.heroes[maren].aptitudes;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(10), 0, [1, 1]);
    assert!(page.lines.contains(
        &"Maren broke at the Barrow, as the Seer said, and was carried home, and mended. +1 to every aptitude.".to_owned()
    ), "{:?}", page.lines);
    let m = &house.heroes[maren];
    assert!(m.is_living() && m.wounded && m.destiny.fulfilled);
    assert_eq!(m.fear.dread, 3, "dread 1 + 2");
    assert_eq!(m.aptitudes, [before[0] + 1, before[1] + 1, before[2] + 1]);
    assert_eq!(
        m.scars.last().map(String::as_str),
        Some("Broken at the Barrow, and mended")
    );
    assert!(m.deeds.iter().any(|d| d.kind == DeedKind::Mended));
    assert!(!m.deeds.iter().any(|d| d.kind == DeedKind::SurvivedDisaster));
}

#[test]
fn every_fire_doomed_member_burns_on_a_fire_quests_setback() {
    let (content, mut house) = house();
    let (brannoc, ysolde, odo) = (
        id(&house.heroes, "Brannoc"),
        id(&house.heroes, "Ysolde"),
        id(&house.heroes, "Odo"),
    );
    house.heroes[ysolde].destiny.kind = Destiny::FireWillEndYou;
    stage(&content, &mut house, 3, "The unquiet forge");
    seat(&mut house, 3, &[brannoc, ysolde, odo]);
    aim(&mut house, 3, [2, 2], -2);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(11), 3, [2, 2]);
    assert_eq!(page.outcome, Outcome::Setback);
    assert!(
        page.lines
            .contains(&"Fire was the end of Brannoc Hale, as the Seer said. He was 31.".to_owned()),
        "{:?}",
        page.lines
    );
    assert!(
        page.lines
            .contains(&"Fire was the end of Ysolde Vane, as the Seer said. She was 24.".to_owned())
    );
    for hero in [brannoc, ysolde] {
        let h = &house.heroes[hero];
        assert_eq!((h.fate, h.destiny.fulfilled), (Fate::Dead, true));
        assert_eq!(
            h.fate_telling,
            "met the fire that was foretold, at Emberfall"
        );
    }
}

#[test]
fn a_court_triumph_crowns_a_hero_of_eight_renown_and_leaves_the_heirloom_to_the_nearest_kin() {
    // Everyone else empty-handed but Maren: the first empty-handed heir is Pip.
    let (content, mut house) = house();
    let (garrick, maren, pip) = (
        id(&house.heroes, "Garrick"),
        id(&house.heroes, "Maren"),
        id(&house.heroes, "Pip"),
    );
    let old = |name: &str, house: &House| {
        let mut heirloom = house.heroes[garrick].heirloom.clone().expect("Thornfall");
        heirloom.name = name.to_owned();
        heirloom
    };
    house.heroes[garrick].destiny.kind = Destiny::WearACrown;
    house.heroes[maren].heirloom = Some(old("Old Thing", &house));
    stage(&content, &mut house, 3, "The tourney");
    seat(&mut house, 3, &[garrick]);
    aim(&mut house, 3, [6, 6], 4);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(12), 3, [6, 6]);
    assert_eq!(page.outcome, Outcome::Triumph);
    // Renown 6 + (2 + 1) = 9 >= 8, read after the reward.
    let crowned = "A crown was offered to Garrick Thorne, as the Seer said. He took it. The house has a patron at Court now.";
    let at = page
        .lines
        .iter()
        .position(|l| l == crowned)
        .expect("the crown line");
    assert_eq!(
        page.lines[at + 1..],
        ["Garrick left Thornfall with Pip before going. A throne has no use for it.".to_owned()]
    );
    let g = &house.heroes[garrick];
    assert_eq!(
        (g.fate, g.fate_telling.as_str()),
        (Fate::Departed, "was called to a throne, and went")
    );
    assert_eq!(
        (g.heirloom.as_ref(), g.bequest_heir, g.bequest_decided),
        (None, Some(pip), true)
    );
    assert_eq!(house.patrons, 1);
    assert_eq!(house.slot_of(garrick), None);
    assert!(house.mourned.is_empty(), "the crowned are not mourned");
    // Everyone holding one: the first heir, Maren, lays hers aside for it.
    let (content, mut house) = crate::testkit::house();
    house.heroes[garrick].destiny.kind = Destiny::WearACrown;
    for hero in 0..house.heroes.len() {
        if hero != garrick {
            house.heroes[hero].heirloom = Some(old(&format!("Thing {hero}"), &house));
        }
    }
    stage(&content, &mut house, 3, "The tourney");
    seat(&mut house, 3, &[garrick]);
    aim(&mut house, 3, [6, 6], 4);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(12), 3, [6, 6]);
    let at = page
        .lines
        .iter()
        .position(|l| l == crowned)
        .expect("the crown line");
    assert_eq!(
        page.lines[at + 1..],
        [
            format!("Maren lays Thing {maren} aside, over the hearth."),
            "Garrick left Thornfall with Maren before going. A throne has no use for it."
                .to_owned(),
        ]
    );
    // No one at all: Court keeps it.
    let (content, mut house) = crate::testkit::house();
    house.heroes[garrick].destiny.kind = Destiny::WearACrown;
    for hero in 0..house.heroes.len() {
        if hero != garrick {
            house.heroes[hero].fate = Fate::Dead;
            house.unseat(hero);
        }
    }
    stage(&content, &mut house, 3, "The tourney");
    seat(&mut house, 3, &[garrick]);
    aim(&mut house, 3, [6, 6], 4);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(12), 3, [6, 6]);
    assert_eq!(
        page.lines.last().map(String::as_str),
        Some("Garrick took Thornfall to Court. There was no one to leave it with.")
    );
    assert!(house.heroes[garrick].heirloom.is_some());
}

#[test]
fn the_heir_list_ranks_children_then_descendants_then_kin_then_the_rest() {
    let (_, heroes) = crate::testkit::founded();
    let garrick = id(&heroes, "Garrick");
    let names: Vec<&str> = crate::harm::heirs(&heroes, garrick)
        .iter()
        .map(|&h| heroes[h].name.as_str())
        .collect();
    // MODULES.md W8: Maren (daughter), Pip (grandson), Odo (friend), then the rest of
    // the living house in founding order.
    assert_eq!(names, ["Maren", "Pip", "Odo", "Ysolde", "Brannoc", "Wren"]);
}

#[test]
fn a_fire_quests_disaster_burns_the_fire_doomed_before_any_death_roll() {
    let (content, mut house) = house();
    let brannoc = id(&house.heroes, "Brannoc");
    stage(&content, &mut house, 3, "The unquiet forge");
    seat(&mut house, 3, &[brannoc]);
    aim(&mut house, 3, [1, 1], -9);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(30), 3, [1, 1]);
    assert_eq!(page.outcome, Outcome::Disaster);
    assert!(
        page.lines
            .contains(&"Fire was the end of Brannoc Hale, as the Seer said. He was 31.".to_owned())
    );
    assert_eq!(
        house.heroes[brannoc].fate_telling,
        "met the fire that was foretold, at Emberfall"
    );
}

#[test]
fn mending_never_raises_an_aptitude_past_nine() {
    let (content, mut house) = house();
    let maren = id(&house.heroes, "Maren");
    house.heroes[maren].aptitudes = [9, 9, 9];
    seat(&mut house, 0, &[maren]);
    house.board[0].quest.danger = 7;
    aim(&mut house, 0, [1, 1], -9);
    resolve_rolled(&content, &mut house, &mut Rng::from_seed(31), 0, [1, 1]);
    assert_eq!(house.heroes[maren].aptitudes, [9, 9, 9]);
}

#[test]
fn a_questing_death_is_against_the_quests_first_tag() {
    let (content, mut house) = house();
    let odo = id(&house.heroes, "Odo");
    stage(&content, &mut house, 0, "The lamps in the Barrow"); // Dark, Undead
    seat(&mut house, 0, &[odo]);
    house.board[0].quest.danger = 7;
    aim(&mut house, 0, [1, 1], -9);
    resolve_rolled(&content, &mut house, &mut Rng::from_seed(32), 0, [1, 1]);
    assert_eq!(house.heroes[odo].fate, Fate::Dead);
    assert_eq!(house.heroes[odo].death_tag, Some(crate::ids::Tag::Dark));
}

#[test]
fn a_rival_ranks_with_everyone_else_and_those_taught_above_a_friend() {
    let (_, mut heroes) = crate::testkit::founded();
    let brannoc = id(&heroes, "Brannoc");
    let names = |heroes: &[crate::hero::Hero], dead| -> Vec<String> {
        crate::harm::heirs(heroes, dead)
            .iter()
            .map(|&h| heroes[h].name.clone())
            .collect()
    };
    // Brannoc: his daughter, then everyone else in founding order — Ysolde, his rival, too.
    assert_eq!(
        names(&heroes, brannoc),
        ["Wren", "Garrick", "Maren", "Pip", "Ysolde", "Odo"]
    );
    // Odo taught Ysolde: she ranks above Garrick, his friend.
    let (odo, ysolde) = (id(&heroes, "Odo"), id(&heroes, "Ysolde"));
    crate::bonds::form(&mut heroes, ysolde, odo, crate::ids::BondKind::Mentor, 1);
    if let Some(bond) = heroes[odo].bonds.iter_mut().find(|b| b.other == ysolde) {
        bond.taught = true;
    }
    assert_eq!(
        names(&heroes, odo),
        ["Ysolde", "Garrick", "Maren", "Pip", "Brannoc", "Wren"]
    );
}

#[test]
fn a_wounded_hero_the_death_roll_spares_dies_of_the_wound_and_did_not_survive() {
    let (content, mut house) = house();
    let odo = id(&house.heroes, "Odo");
    house.heroes[odo].wounded = true;
    seat(&mut house, 0, &[odo]);
    house.board[0].quest.danger = 0; // min(0 * 0.15, 1): the roll never hits.
    aim(&mut house, 0, [1, 1], -9);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(33), 0, [1, 1]);
    assert!(page.lines.contains(
        &"Odo Fenn was already hurt, and went anyway. The second wound was the last. He was 47.".to_owned()
    ));
    let o = &house.heroes[odo];
    assert_eq!(
        o.fate_telling,
        "died of wounds on the road home from the Barrow"
    );
    assert!(!o.deeds.iter().any(|d| d.kind == DeedKind::SurvivedDisaster));
}
