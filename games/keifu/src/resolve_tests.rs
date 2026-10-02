//! The resolution battery's unit half: every step of SPEC §7, §7.1-7.4, §10.1 and
//! §12.2 on the founding household, with the dice chosen so the outcome is staged.
//! Expectations are shipped literals from the spec and its content, never arithmetic
//! over the constant under test.

use jidousha::prelude::Rng;

use crate::board::Posted;
use crate::content::Content;
use crate::hero::{DeedKind, DreamFate, Fate, HeroId};
use crate::house::House;
use crate::ids::{BondKind, Destiny, Outcome, Place};
use crate::quest::post;
use crate::resolve::{resolve_rolled, set_out};
use crate::testkit::{house, id};

/// Put `title`'s quest on board slot `slot`, posted at its place's trouble.
fn stage(content: &Content, house: &mut House, slot: usize, title: &str) {
    let template = content
        .quest_templates
        .iter()
        .position(|t| t.title == title)
        .expect("a template by that title");
    let trouble = house.places[content.quest_templates[template].place.index()].trouble;
    let quest = post(content, template, trouble, 1, &mut Rng::from_seed(3));
    house.board[slot] = Posted {
        seats: vec![None; quest.seats as usize],
        quest,
    };
}

/// Seat `who` on board slot `slot`, adding seats past the quest's if a test needs them.
fn seat(house: &mut House, slot: usize, who: &[HeroId]) {
    for (at, &hero) in who.iter().enumerate() {
        house.unseat(hero);
        let seats = &mut house.board[slot].seats;
        if seats.len() <= at {
            seats.push(None);
        }
        seats[at] = Some(hero);
    }
}

/// Set slot `slot`'s demand so the seated party, throwing `dice`, lands on `margin`.
fn aim(house: &mut House, slot: usize, dice: [i32; 2], margin: i32) {
    let party = house.party(slot);
    let power = crate::power::party_power(
        &house.heroes,
        &party,
        house.board[slot].quest.facts(),
        house.patrons,
    );
    house.board[slot].quest.demand = power + dice[0] + dice[1] - 7 - margin;
}

/// Every line written that is one of `pool`'s lines filled with `args`.
fn pooled(content: &Content, pool: crate::ids::Pool, args: &[&str]) -> Vec<String> {
    content.pools[pool.index()]
        .iter()
        .map(|line| crate::text::fmt(line, args))
        .collect()
}

#[test]
fn staying_home_in_year_one_costs_four_renown_and_troubles_four_places() {
    let (content, mut house) = house();
    let places: Vec<Place> = house.board.iter().map(|p| p.quest.place).collect();
    set_out(&content, &mut house, &mut Rng::from_seed(1));
    let telling = house.telling.as_ref().expect("a telling");
    assert!(telling.pages.is_empty());
    assert_eq!(telling.year, 1);
    assert_eq!(telling.meanwhile.len(), 5);
    assert_eq!(
        telling.meanwhile[0],
        "The Barrow's dead have walked a year unanswered. No one went."
    );
    assert_eq!(
        telling.meanwhile[1],
        "The tide has had the Drowned Coast to itself for a year. No one went."
    );
    assert_eq!(
        telling.meanwhile[4],
        "The house is thought less of for it: -4 renown."
    );
    assert_eq!(house.renown, 11);
    for place in places {
        assert_eq!(house.places[place.index()].trouble, 1, "{place:?}");
        assert_eq!(house.places[place.index()].visits, 0);
    }
}

#[test]
fn an_unanswered_quest_costs_one_more_when_troubled_and_one_per_twenty_renown_held() {
    let (content, mut house) = house();
    house.renown = 40;
    house.board[0].quest.trouble = 1;
    house.places[house.board[0].quest.place.index()].trouble = 2;
    set_out(&content, &mut house, &mut Rng::from_seed(1));
    // 1 + 1 + 40/20 for the troubled Barrow, 1 + 0 + 2 for each of the others.
    assert_eq!(house.renown, 40 - (4 + 3 * 3));
    let telling = house.telling.as_ref().expect("a telling");
    assert_eq!(
        telling.meanwhile[0], "The Barrow's dead have walked two years unanswered. No one went.",
        "trouble stays at 2"
    );
    assert_eq!(house.places[0].trouble, 2);
}

#[test]
fn a_wounded_hero_who_stays_home_mends_and_one_who_goes_does_not() {
    let (content, mut house) = house();
    let (odo, brannoc, maren) = (
        id(&house.heroes, "Odo"),
        id(&house.heroes, "Brannoc"),
        id(&house.heroes, "Maren"),
    );
    house.heroes[odo].wounded = true;
    house.heroes[maren].wounded = true;
    house.heroes[brannoc].wounded = true;
    seat(&mut house, 0, &[brannoc]);
    set_out(&content, &mut house, &mut Rng::from_seed(2));
    let telling = house.telling.as_ref().expect("a telling");
    let tail: Vec<&str> = telling
        .meanwhile
        .iter()
        .rev()
        .take(2)
        .map(String::as_str)
        .collect();
    // Roster seat order: Maren sat before Odo.
    assert_eq!(
        tail,
        [
            "Odo stayed home and mended. The wound has closed.",
            "Maren stayed home and mended. The wound has closed."
        ]
    );
    assert!(!house.heroes[odo].wounded && !house.heroes[maren].wounded);
    assert_eq!(telling.pages.len(), 1);
}

#[test]
fn garrick_and_brannoc_triumphing_on_grave_goods_are_rewarded_taught_and_garrick_settles() {
    let (content, mut house) = house();
    let (garrick, brannoc) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Brannoc"));
    seat(&mut house, 0, &[garrick, brannoc]);
    house.places[Place::Barrow.index()].trouble = 2;
    let renown = house.renown;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(4), 0, [6, 6]);
    assert_eq!(page.power, 12, "Garrick 5 + Thornfall 1, Brannoc 6");
    assert_eq!(page.outcome, Outcome::Triumph);
    assert_eq!(page.margin, 12 + 12 - 7 - page.quest.demand);
    assert_eq!(page.members, [garrick, brannoc]);
    assert!(page.story.contains("Garrick and Brannoc"), "{}", page.story);
    assert_eq!(
        page.lines[0],
        "3 renown to the house, and to each who went."
    );
    // The triumph lesson: Garrick may not learn (his firstborn is grown), so Brannoc.
    let lessons = pooled(&content, crate::ids::Pool::Lessons, &["Brannoc"]);
    assert!(
        lessons
            .iter()
            .any(|l| page.lines[1] == format!("{l} Might rises to 7.")),
        "{}",
        page.lines[1]
    );
    assert_eq!(
        page.lines[2],
        "Garrick has done it: to lay the Barrow's dead to rest. He is settled now, and dread has no hold on him."
    );
    assert_eq!(house.renown, renown + 3);
    assert_eq!(house.heroes[garrick].renown, 6 + 3);
    assert_eq!(house.heroes[brannoc].aptitudes[0], 7);
    assert!(house.heroes[garrick].settled);
    let barrow = house.places[Place::Barrow.index()];
    assert_eq!(
        (
            barrow.visits,
            barrow.triumphs,
            barrow.disasters,
            barrow.trouble
        ),
        (1, 1, 0, 0)
    );
    for hero in [garrick, brannoc] {
        let h = &house.heroes[hero];
        assert!(h.roads_walked.contains(&Place::Barrow));
        let triumph = h
            .deeds
            .iter()
            .find(|d| d.kind == DeedKind::Triumph)
            .expect("a triumph deed");
        assert_eq!((triumph.place, triumph.weight), (Some(Place::Barrow), 2));
        assert_eq!(
            h.bond_to(if hero == garrick { brannoc } else { garrick })
                .map(|b| (b.kind, b.shared_successes)),
            Some((BondKind::Companion, 1))
        );
    }
    // Garrick had quested before (30 times); Brannoc too: no FIRST_QUEST for either.
    assert!(
        house
            .heroes
            .iter()
            .all(|h| h.deeds.iter().all(|d| d.kind != DeedKind::FirstQuest))
    );
    assert_eq!(house.heroes[garrick].quests_faced, 31);
}

#[test]
fn a_first_quest_is_recorded_once_with_the_place_and_the_danger() {
    let (content, mut house) = house();
    let ysolde = id(&house.heroes, "Ysolde");
    house.heroes[ysolde].quests_faced = 0;
    seat(&mut house, 0, &[ysolde]);
    resolve_rolled(&content, &mut house, &mut Rng::from_seed(5), 0, [3, 3]);
    let firsts: Vec<_> = house.heroes[ysolde]
        .deeds
        .iter()
        .filter(|d| d.kind == DeedKind::FirstQuest)
        .map(|d| (d.place, d.weight, d.telling.clone()))
        .collect();
    assert_eq!(
        firsts,
        [(
            Some(Place::Barrow),
            2,
            "first quested at the Barrow, aged 24".to_owned()
        )]
    );
    seat(&mut house, 0, &[ysolde]);
    resolve_rolled(&content, &mut house, &mut Rng::from_seed(5), 0, [3, 3]);
    assert_eq!(
        house.heroes[ysolde]
            .deeds
            .iter()
            .filter(|d| d.kind == DeedKind::FirstQuest)
            .count(),
        1
    );
}

#[test]
fn a_disaster_costs_the_house_its_danger_and_a_loss_eases_trouble_by_one() {
    let (content, mut house) = house();
    let ysolde = id(&house.heroes, "Ysolde");
    house.heroes[ysolde].destiny.kind = Destiny::DieInYourBed;
    house.places[Place::Barrow.index()].trouble = 2;
    seat(&mut house, 0, &[ysolde]);
    let renown = house.renown;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(6), 0, [1, 1]);
    assert_eq!(page.outcome, Outcome::Disaster);
    assert!(
        page.lines
            .contains(&"Word of it got about. The house loses 2 renown.".to_owned())
    );
    assert_eq!(house.renown, renown - 2);
    let barrow = house.places[Place::Barrow.index()];
    assert_eq!((barrow.visits, barrow.disasters, barrow.trouble), (1, 1, 1));
}

#[test]
fn a_setback_wounds_the_unlucky_one_and_a_second_wound_kills_unless_the_seer_shields() {
    let (content, mut house) = house();
    let odo = id(&house.heroes, "Odo");
    seat(&mut house, 0, &[odo]);
    aim(&mut house, 0, [1, 2], -1);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(7), 0, [1, 2]);
    assert_eq!(page.outcome, Outcome::Setback);
    let wounds = pooled(&content, crate::ids::Pool::Wounds, &["Odo"]);
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
    let deaths = pooled(
        &content,
        crate::ids::Pool::QuestDeaths,
        &["Odo Fenn", "the Barrow"],
    );
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
fn companions_who_win_twice_become_friends_and_companions_in_a_disaster_become_rivals() {
    let (content, mut house) = house();
    let (ysolde, odo, garrick) = (
        id(&house.heroes, "Ysolde"),
        id(&house.heroes, "Odo"),
        id(&house.heroes, "Garrick"),
    );
    for round in 0..2 {
        seat(&mut house, 1, &[ysolde, odo]);
        aim(&mut house, 1, [3, 3], 0);
        let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(13), 1, [3, 3]);
        let friends = pooled(&content, crate::ids::Pool::Friendships, &["Ysolde", "Odo"]);
        assert_eq!(
            page.lines.iter().any(|l| friends.contains(l)),
            round == 1,
            "{:?}",
            page.lines
        );
    }
    assert_eq!(
        house.heroes[ysolde].bond_to(odo).map(|b| b.kind),
        Some(BondKind::Friend)
    );
    assert!(
        house.heroes[odo]
            .deeds
            .iter()
            .any(|d| d.kind == DeedKind::Befriended && d.other == Some(ysolde))
    );
    // Garrick and Ysolde, strangers, through a disaster: companions, then rivals.
    house.heroes[garrick].destiny.kind = Destiny::DieInYourBed;
    house.heroes[ysolde].destiny.kind = Destiny::DieInYourBed;
    seat(&mut house, 0, &[garrick, ysolde]);
    aim(&mut house, 0, [1, 1], -9);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(14), 0, [1, 1]);
    assert!(page.lines.contains(
        &"Garrick and Ysolde each say the other broke first at the Barrow. They are rivals now, and weaker together.".to_owned()
    ), "{:?}", page.lines);
    assert_eq!(
        house.heroes[garrick].bond_to(ysolde).map(|b| b.kind),
        Some(BondKind::Rival)
    );
}

#[test]
fn rivals_who_triumph_together_come_home_friends() {
    let (content, mut house) = house();
    let (brannoc, ysolde) = (id(&house.heroes, "Brannoc"), id(&house.heroes, "Ysolde"));
    seat(&mut house, 0, &[brannoc, ysolde]);
    aim(&mut house, 0, [6, 6], 5);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(15), 0, [6, 6]);
    assert!(page.lines.contains(
        &"Brannoc and Ysolde went out rivals. Nobody can stay a rival through a day like that. They came home friends.".to_owned()
    ), "{:?}", page.lines);
    assert_eq!(
        house.heroes[ysolde].bond_to(brannoc).map(|b| b.kind),
        Some(BondKind::Friend)
    );
}

#[test]
fn a_parent_and_child_who_fail_together_each_take_dread_parent_first() {
    let (content, mut house) = house();
    let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
    seat(&mut house, 0, &[maren, garrick]);
    aim(&mut house, 0, [3, 3], -2);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(16), 0, [3, 3]);
    let parent = page
        .lines
        .iter()
        .position(|l| l == "Garrick saw his daughter in the worst of it, and has not stopped seeing it. Dread 3 of 5.")
        .expect("the parent's line");
    assert_eq!(
        page.lines[parent + 1],
        "Maren saw her father go down. Dread 2 of 5."
    );
    assert_eq!(
        (
            house.heroes[garrick].fear.dread,
            house.heroes[maren].fear.dread
        ),
        (3, 2)
    );
}

#[test]
fn a_steadied_win_adds_courage_three_conquers_and_an_unsteadied_loss_adds_two_dread() {
    let (content, mut house) = house();
    let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
    house.heroes[maren].fear.courage = 2;
    seat(&mut house, 1, &[maren, garrick]);
    aim(&mut house, 1, [3, 3], 0);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(17), 1, [3, 3]);
    let courage = pooled(
        &content,
        crate::ids::Pool::Courages,
        &["Maren", "deep water"],
    );
    assert!(
        page.lines.iter().any(|l| courage
            .iter()
            .any(|c| *l == format!("{c}, with Garrick beside her. Courage 3 of 3."))),
        "{:?}",
        page.lines
    );
    assert!(
        page.lines
            .iter()
            .any(|l| l.starts_with("Maren has conquered her fear of deep water."))
    );
    assert!(house.heroes[maren].fear.conquered);
    assert_eq!(
        house.heroes[garrick].fear.courage, 1,
        "Garrick, steadied by his daughter"
    );
    assert_eq!(
        house.heroes[maren].fears_faced,
        1 + 0,
        "household.json: Maren had faced it 0 times"
    );
    // Garrick alone on the bell, lost: dread +2 and the DREADS line, at 5 he breaks.
    house.heroes[garrick].fear.dread = 3;
    seat(&mut house, 1, &[garrick]);
    aim(&mut house, 1, [3, 3], -2);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(18), 1, [3, 3]);
    let dreads = pooled(
        &content,
        crate::ids::Pool::Dreads,
        &["Garrick", "deep water"],
    );
    let at = page
        .lines
        .iter()
        .position(|l| dreads.iter().any(|d| *l == format!("{d} Dread 5 of 5.")))
        .expect("the dread line");
    assert!(page.lines[at + 1].starts_with("Something in Garrick gave way."));
    assert!(house.heroes[garrick].fear.broken);
}

#[test]
fn a_youth_learns_twice_from_one_triumph_and_only_below_five() {
    let (content, mut house) = house();
    let ysolde = id(&house.heroes, "Ysolde");
    house.heroes[ysolde].age = 15;
    house.heroes[ysolde].aptitudes[0] = 3;
    seat(&mut house, 0, &[ysolde]);
    aim(&mut house, 0, [6, 6], 6);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(19), 0, [6, 6]);
    assert_eq!(page.lines[0], "3 renown to the house, and to Ysolde.");
    let lessons: Vec<&String> = page
        .lines
        .iter()
        .filter(|l| l.contains(" Might rises to "))
        .collect();
    assert_eq!(lessons.len(), 2, "{lessons:?}");
    assert!(lessons[0].ends_with("Might rises to 4.") && lessons[1].ends_with("Might rises to 5."));
    seat(&mut house, 0, &[ysolde]);
    aim(&mut house, 0, [6, 6], 1);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(19), 0, [6, 6]);
    assert!(
        !page.lines.iter().any(|l| l.contains(" rises to ")),
        "base 5: no youth lesson"
    );
}

#[test]
fn winning_a_ghosts_quest_lays_it_and_swap_removes_it_from_the_list() {
    let (content, mut house) = house();
    let (garrick, maren, odo, ysolde) = (
        id(&house.heroes, "Garrick"),
        id(&house.heroes, "Maren"),
        id(&house.heroes, "Odo"),
        id(&house.heroes, "Ysolde"),
    );
    let ghost = |hero: HeroId, house: &House| crate::ghost::Ghost {
        hero,
        dream: house.heroes[garrick].dream.clone().expect("a dream"),
        place: Place::Barrow,
    };
    for dead in [garrick, maren, odo] {
        house.heroes[dead].fate = Fate::Dead;
        house.unseat(dead);
    }
    house.ghosts = vec![
        ghost(garrick, &house),
        ghost(maren, &house),
        ghost(odo, &house),
    ];
    let quest = crate::ghost::ghost_quest(&content, &house.heroes, &house.ghosts[0], 0, 1);
    house.board[0] = Posted {
        seats: vec![None; 2],
        quest,
    };
    seat(&mut house, 0, &[ysolde]);
    aim(&mut house, 0, [4, 4], 1);
    let tales = house.tales.len();
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(20), 0, [4, 4]);
    assert!(
        page.story
            .starts_with("Ysolde said aloud what Garrick had meant to do"),
        "{}",
        page.story
    );
    assert_eq!(
        page.lines.last().map(String::as_str),
        Some(
            "Garrick Thorne is at rest. What he wanted is let go, not done. It leaves a tale: The laying of Garrick's ghost. +1 renown every year."
        )
    );
    assert_eq!(house.tales.len(), tales + 1);
    assert_eq!(house.heroes[garrick].dream_fate, DreamFate::LaidToRest);
    assert_eq!(house.heroes[garrick].laid_year, Some(1));
    let order: Vec<HeroId> = house.ghosts.iter().map(|g| g.hero).collect();
    assert_eq!(
        order,
        [odo, maren],
        "the last ghost takes the laid one's place"
    );
    assert!(
        house.heroes[ysolde]
            .deeds
            .iter()
            .any(|d| d.kind == DeedKind::LaidGhost)
    );
}
