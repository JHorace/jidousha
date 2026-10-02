//! The resolution battery's unit half, part one: set out, the unanswered, healing at
//! home, and the steps of SPEC §7.1 up to the reward (§7.3), on the founding household
//! with the dice chosen so the outcome is staged. Expectations are shipped literals
//! from the spec and its content, never arithmetic over the constant under test.

use jidousha::prelude::Rng;

use crate::hero::{DeedKind, Fate};
use crate::ids::{BondKind, Destiny, Outcome, Place, Pool};
use crate::resolve::{resolve_rolled, set_out};
use crate::testkit::{aim, house, id, pooled, seat};

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
    let lessons = pooled(&content, Pool::Lessons, &["Brannoc"]);
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
fn the_roll_counts_the_houses_patrons() {
    let (content, mut house) = house();
    let (garrick, brannoc) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Brannoc"));
    house.patrons = 2;
    seat(&mut house, 0, &[garrick, brannoc]);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(21), 0, [3, 3]);
    assert_eq!(page.power, 14, "12 and two patrons");
}

#[test]
fn a_carrier_adds_one_more_to_the_house_and_says_so() {
    let (content, mut house) = house();
    let brannoc = id(&house.heroes, "Brannoc");
    house.heroes[brannoc].destiny.kind = Destiny::CarryTheHouse;
    seat(&mut house, 0, &[brannoc]);
    aim(&mut house, 0, [6, 6], 5);
    let renown = house.renown;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(22), 0, [6, 6]);
    assert_eq!(
        page.lines[..2],
        [
            "3 renown to the house, and to Brannoc.".to_owned(),
            "Brannoc carries the house, as the Seer said. +1 renown more.".to_owned(),
        ]
    );
    assert_eq!(house.renown, renown + 3 + 1);
    assert_eq!(
        house.heroes[brannoc].renown,
        2 + 3,
        "the carrier's extra is the house's only"
    );
}

#[test]
fn a_success_records_no_triumph_and_teaches_no_triumph_lesson() {
    let (content, mut house) = house();
    let brannoc = id(&house.heroes, "Brannoc");
    seat(&mut house, 0, &[brannoc]);
    aim(&mut house, 0, [3, 3], 1);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(23), 0, [3, 3]);
    assert_eq!(page.outcome, Outcome::Success);
    assert_eq!(page.lines[0], "2 renown to the house, and to Brannoc.");
    assert!(
        !page.lines.iter().any(|l| l.contains(" rises to ")),
        "{:?}",
        page.lines
    );
    assert!(
        !house.heroes[brannoc]
            .deeds
            .iter()
            .any(|d| d.kind == DeedKind::Triumph)
    );
    assert_eq!(house.heroes[brannoc].aptitudes[0], 6);
}

#[test]
fn the_triumph_lesson_goes_to_the_lowest_base_and_never_past_nine() {
    let (content, mut house) = house();
    let (ysolde, brannoc) = (id(&house.heroes, "Ysolde"), id(&house.heroes, "Brannoc"));
    seat(&mut house, 0, &[brannoc, ysolde]);
    aim(&mut house, 0, [6, 6], 5);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(24), 0, [6, 6]);
    let lessons = pooled(&content, Pool::Lessons, &["Ysolde"]);
    assert!(
        page.lines.iter().any(|l| lessons
            .iter()
            .any(|x| *l == format!("{x} Might rises to 3."))),
        "Ysolde's Might 2 is lower than Brannoc's 6: {:?}",
        page.lines
    );
    assert_eq!(house.heroes[brannoc].aptitudes[0], 6);
    // At 9, nothing more is learned.
    let (content, mut house) = crate::testkit::house();
    house.heroes[brannoc].aptitudes[0] = 9;
    seat(&mut house, 0, &[brannoc]);
    aim(&mut house, 0, [6, 6], 5);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(24), 0, [6, 6]);
    assert!(
        !page.lines.iter().any(|l| l.contains(" rises to ")),
        "{:?}",
        page.lines
    );
    assert_eq!(house.heroes[brannoc].aptitudes[0], 9);
}

#[test]
fn a_setbacks_unlucky_one_is_any_member_with_equal_chance() {
    let (content, founded) = house();
    let party = [
        id(&founded.heroes, "Brannoc"),
        id(&founded.heroes, "Ysolde"),
        id(&founded.heroes, "Odo"),
    ];
    let mut wounded = [0u32; 3];
    let trials = 1200;
    for trial in 0..trials {
        let mut house = founded.clone();
        seat(&mut house, 0, &party);
        aim(&mut house, 0, [3, 3], -2);
        resolve_rolled(
            &content,
            &mut house,
            &mut Rng::from_seed(5000 + trial),
            0,
            [3, 3],
        );
        for (at, &hero) in party.iter().enumerate() {
            wounded[at] += u32::from(house.heroes[hero].wounded);
        }
    }
    // A third each: 400, within four standard deviations (~65).
    assert_eq!(wounded.iter().sum::<u32>(), 1200, "{wounded:?}");
    assert!(
        wounded.iter().all(|&w| (335..=465).contains(&w)),
        "{wounded:?}"
    );
}

#[test]
fn the_story_is_the_quests_ending_for_its_outcome() {
    let (content, mut house) = house();
    let (garrick, brannoc) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Brannoc"));
    for (margin, story) in [
        (
            -9,
            "The robbers had woken what they came to rob. Garrick and Brannoc walked into the middle of it.",
        ),
        (
            -2,
            "Garrick and Brannoc found the robbers' lantern and one boot. Then the passage came down.",
        ),
        (
            1,
            "Garrick and Brannoc brought the robbers out. They were glad to be arrested.",
        ),
    ] {
        let mut house = house.clone();
        house.heroes[garrick].destiny.kind = Destiny::DieInYourBed;
        house.heroes[brannoc].destiny.kind = Destiny::DieInYourBed;
        seat(&mut house, 0, &[garrick, brannoc]);
        aim(&mut house, 0, [2, 2], margin);
        let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(25), 0, [2, 2]);
        assert_eq!(page.story, story);
    }
    seat(&mut house, 0, &[garrick]);
}

#[test]
fn the_crown_claims_only_at_the_kings_court() {
    let (content, mut house) = house();
    let garrick = id(&house.heroes, "Garrick");
    house.heroes[garrick].destiny.kind = Destiny::WearACrown;
    house.heroes[garrick].renown = 10;
    seat(&mut house, 0, &[garrick]);
    aim(&mut house, 0, [6, 6], 5);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(26), 0, [6, 6]);
    assert_eq!(page.outcome, Outcome::Triumph);
    assert!(house.heroes[garrick].is_living() && house.patrons == 0);
}

#[test]
fn the_road_is_shared_only_by_the_living() {
    let (content, mut house) = house();
    let (ysolde, odo) = (id(&house.heroes, "Ysolde"), id(&house.heroes, "Odo"));
    house.heroes[ysolde].destiny.kind = Destiny::DieInYourBed;
    seat(&mut house, 0, &[ysolde, odo]);
    house.board[0].quest.danger = 7;
    aim(&mut house, 0, [1, 1], -9);
    resolve_rolled(&content, &mut house, &mut Rng::from_seed(27), 0, [1, 1]);
    assert_eq!(house.heroes[odo].fate, Fate::Dead);
    assert_eq!(
        house.heroes[ysolde].bond_to(odo),
        None,
        "no bond formed with the dead"
    );
}
