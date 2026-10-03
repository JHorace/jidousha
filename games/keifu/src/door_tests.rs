//! The Door's unit half (SPEC §16): the last summer's board, the outlook and the best four,
//! the bearer rule, the locks in their order, what carries from one lock into the next,
//! the Door's exceptions to §7.1, its deeds and its prologue — on the founding household
//! moved to the last summer, with the dice chosen so each lock's outcome is staged.
//!
//! INVARIANT: every expectation is a shipped literal worked by hand from `household.json`,
//! `door.json` and CONSTANTS §3/§12 — never arithmetic over the constant under test.
//!
//! Party A (all aged 30, prime): Garrick 9/7/8 with Thornfall (+1 Might), Maren 8/9/7,
//! Ysolde 7/9/9 (the Door's promise +5; fear of the dark at dread 0, -2), Brannoc 9/6/8;
//! bonds Garrick-Maren +2 and Brannoc-Ysolde -1. Might 10+8+10+9+2-1 = 38, Wits 7+9+12+6+1
//! = 35, Spirit 8+7+12+8+1 = 36: margins +4, +1, +2 against 34.
//! Party B (all aged 30): Garrick 8/6/8, Maren 7/9/8, Brannoc 8/6/9, Odo 7/9/9; bonds
//! Garrick-Maren +2, Garrick-Odo +1. Might 9+7+8+7+3 = 34, Wits 6+9+6+9+3 = 33, Spirit
//! 8+8+9+9+3 = 37: margins 0, -1, +3.

use jidousha::prelude::Rng;

use crate::content::Content;
use crate::door::{bearer, best_four, lock_quest, outlook, try_the_door};
use crate::hero::{DeedKind, HeroId};
use crate::house::House;
use crate::ids::{Aptitude, BondKind, Outcome, Place, Tag};
use crate::telling::QuestPage;
use crate::testkit::{house, id, seat};

/// The founding household in the last summer, its board posted and the roster seated.
pub fn at_the_door() -> (Content, House) {
    let (content, mut house) = house();
    for _ in 0..25 {
        house.calendar.begin_winter();
        house.calendar.begin_summer();
    }
    house.prepare_summer(&content, &mut Rng::from_seed(1));
    (content, house)
}

/// Give `name` the base aptitudes `bases` at age 30 (prime: no phase adjustment).
fn make(house: &mut House, name: &str, bases: [i32; 3]) -> HeroId {
    let who = id(&house.heroes, name);
    house.heroes[who].aptitudes = bases;
    house.heroes[who].age = 30;
    who
}

/// Party A, seated on the Door.
pub fn party_a(house: &mut House) -> Vec<HeroId> {
    let party = vec![
        make(house, "Garrick", [9, 7, 8]),
        make(house, "Maren", [8, 9, 7]),
        make(house, "Ysolde", [7, 9, 9]),
        make(house, "Brannoc", [9, 6, 8]),
    ];
    seat(house, 0, &party);
    party
}

/// Party B, seated on the Door.
pub fn party_b(house: &mut House) -> Vec<HeroId> {
    let party = vec![
        make(house, "Garrick", [8, 6, 8]),
        make(house, "Maren", [7, 9, 8]),
        make(house, "Brannoc", [8, 6, 9]),
        make(house, "Odo", [7, 9, 9]),
    ];
    seat(house, 0, &party);
    party
}

fn tried(
    content: &Content,
    house: &mut House,
    dice: [[i32; 2]; 3],
) -> (crate::door::DoorRecord, Vec<QuestPage>) {
    let mut pages = Vec::new();
    let record = try_the_door(
        content,
        house,
        &mut Rng::from_seed(7),
        Some(dice),
        &mut pages,
    );
    (record, pages)
}

#[test]
fn the_last_summer_posts_the_doors_might_lock_alone_four_seats_thirty_four() {
    let (_, house) = at_the_door();
    assert_eq!(house.calendar.current_year(), 26);
    assert_eq!(house.board.len(), 1);
    let quest = &house.board[0].quest;
    assert!(quest.is_door_lock());
    assert_eq!(quest.title, "The lock of iron");
    assert_eq!(quest.place, Place::SealedDoor);
    assert_eq!(quest.aptitude, Aptitude::Might);
    assert_eq!(quest.tags, [Tag::Dark, Tag::Cold]);
    assert_eq!(
        (
            quest.seats,
            quest.demand,
            quest.danger,
            quest.renown,
            quest.trouble
        ),
        (4, 34, 3, 5, 0)
    );
    assert_eq!(house.board[0].seats, [None; 4]);
    assert!(house.board_report.is_none(), "nothing is generated");
}

#[test]
fn each_lock_is_its_aptitude_against_thirty_four_with_no_wobble_or_creep() {
    let (content, _) = at_the_door();
    for (lock, aptitude, title) in [
        (0, Aptitude::Might, "The lock of iron"),
        (1, Aptitude::Wits, "The lock of riddles"),
        (2, Aptitude::Spirit, "The lock of breath"),
    ] {
        let quest = lock_quest(&content, lock);
        assert_eq!((quest.aptitude, quest.title.as_str()), (aptitude, title));
        assert_eq!(
            (quest.demand, quest.danger, quest.renown, quest.seats),
            (34, 3, 5, 4)
        );
        assert!(
            quest.facts().door_lock,
            "the Door's promise counts at every lock"
        );
    }
}

#[test]
fn the_control_reads_try_the_door_and_waits_for_one_before_it() {
    let (content, mut house) = at_the_door();
    assert_eq!(
        crate::summer::set_out_label(&content, &house),
        "Try the Door"
    );
    assert!(!crate::summer::may_set_out(&house));
    let garrick = id(&house.heroes, "Garrick");
    seat(&mut house, 0, &[garrick]);
    assert_eq!(
        crate::summer::set_out_label(&content, &house),
        "Try the Door"
    );
    assert!(crate::summer::may_set_out(&house));
}

#[test]
fn party_a_brings_thirty_eight_thirty_five_thirty_six_and_opens_all_three_fifty_nine_in_a_hundred()
{
    let (content, mut house) = at_the_door();
    let party = party_a(&mut house);
    let seen = outlook(&content, &house.heroes, &party, house.patrons);
    assert_eq!(seen.powers, [38, 35, 36]);
    assert_eq!(seen.ways, [35, 26, 30]);
    assert_eq!(
        [
            seen.lock_percent(0),
            seen.lock_percent(1),
            seen.lock_percent(2)
        ],
        [97, 72, 83]
    );
    assert_eq!(seen.all_percent(), 59);
}

#[test]
fn party_b_brings_thirty_four_thirty_three_thirty_seven_and_opens_all_three_twenty_two_in_a_hundred()
 {
    let (content, mut house) = at_the_door();
    let party = party_b(&mut house);
    let seen = outlook(&content, &house.heroes, &party, house.patrons);
    assert_eq!(seen.powers, [34, 33, 37]);
    assert_eq!(seen.ways, [21, 15, 33]);
    assert_eq!(seen.all_percent(), 22);
    // A patron at Court adds one at every lock.
    house.patrons = 1;
    let seen = outlook(&content, &house.heroes, &party, house.patrons);
    assert_eq!(seen.powers, [35, 34, 38]);
    assert_eq!(seen.ways, [26, 21, 35]);
    // 26 * 21 * 35 = 19110 of 46656: 40.96 in 100.
    assert_eq!(seen.all_percent(), 41);
}

#[test]
fn all_three_rounds_a_half_up_as_percent_does() {
    let (content, _) = at_the_door();
    let mut seen = outlook(&content, &[], &[], 0);
    // 18 * 18 * 18 of 46656 is exactly 12.5 in 100.
    seen.ways = [18, 18, 18];
    assert_eq!(seen.all_percent(), 13);
    seen.ways = [36, 36, 35];
    assert_eq!(seen.all_percent(), 97);
    seen.ways = [0, 36, 36];
    assert_eq!(seen.all_percent(), 0);
}

#[test]
fn nobody_on_the_door_brings_nothing_and_opens_nothing() {
    let (content, house) = at_the_door();
    let seen = outlook(&content, &house.heroes, &[], 3);
    assert_eq!(seen.powers, [0, 0, 0]);
    assert_eq!(seen.ways, [0, 0, 0]);
}

#[test]
fn the_founding_households_best_four_is_the_first_four_bringing_twenty_two_twenty_two_eighteen() {
    // Year 1: five living adults, every four of them at 0 in 100; the first is kept.
    let (content, house) = house();
    let best = best_four(&content, &house);
    let names: Vec<&str> = best
        .party
        .iter()
        .map(|&m| house.heroes[m].name.as_str())
        .collect();
    assert_eq!(names, ["Garrick", "Maren", "Ysolde", "Brannoc"]);
    assert_eq!(best.powers, [22, 22, 18]);
    assert_eq!(best.all_percent(), 0);
}

#[test]
fn the_best_four_is_the_first_strictly_best_of_every_four_in_creation_order() {
    let (content, mut house) = at_the_door();
    party_a(&mut house);
    // Odo strong enough that the four without Brannoc is better: Odo 9/9/9 at 30.
    make(&mut house, "Odo", [9, 9, 9]);
    let best = best_four(&content, &house);
    let names: Vec<&str> = best
        .party
        .iter()
        .map(|&m| house.heroes[m].name.as_str())
        .collect();
    // Garrick, Maren, Ysolde, Odo: Might 10+8+10+9+2+1 = 40, Wits 7+9+12+9+3 = 40,
    // Spirit 8+7+12+9+3 = 39 — margins +6, +6, +5: every lock 36 of 36.
    assert_eq!(names, ["Garrick", "Maren", "Ysolde", "Odo"]);
    assert_eq!(best.powers, [40, 40, 39]);
    assert_eq!(best.all_percent(), 100);
}

#[test]
fn the_best_four_counts_a_wounded_and_a_refusing_hero_and_takes_everyone_when_four_or_fewer() {
    let (content, mut house) = at_the_door();
    party_a(&mut house);
    let odo = make(&mut house, "Odo", [9, 9, 9]);
    // Odo broken by the cold would be bounced from the Door, and still counts (OQ-14) —
    // his dread staged at 0, so his penalty is the least a fear costs (-2 at each lock):
    // with him the four bring 38, 38, 37, better than party A's 38, 35, 36.
    house.heroes[odo].fear.tag = Tag::Cold;
    house.heroes[odo].fear.broken = true;
    let best = best_four(&content, &house);
    assert!(best.party.contains(&odo), "a refuser is not left out");
    // Down to four living adults: all of them, whoever they are.
    let brannoc = id(&house.heroes, "Brannoc");
    house.heroes[brannoc].fate = crate::hero::Fate::Dead;
    house.heroes[odo].wounded = true;
    let best = best_four(&content, &house);
    let names: Vec<&str> = best
        .party
        .iter()
        .map(|&m| house.heroes[m].name.as_str())
        .collect();
    assert_eq!(names, ["Garrick", "Maren", "Ysolde", "Odo"]);
}

#[test]
fn the_bearer_is_the_strongest_alone_the_first_on_ties_patrons_aside() {
    let (content, mut house) = at_the_door();
    let party = party_a(&mut house);
    house.patrons = 3;
    let name = |lock: usize| {
        let who = bearer(&house.heroes, &party, &lock_quest(&content, lock));
        who.map(|m| house.heroes[m].name.clone())
    };
    // Might alone: Garrick 10 and Ysolde 10 (the first kept), Brannoc 9, Maren 8.
    assert_eq!(name(0).as_deref(), Some("Garrick"));
    // Wits: Ysolde 12. Spirit: Ysolde 12.
    assert_eq!(name(1).as_deref(), Some("Ysolde"));
    assert_eq!(name(2).as_deref(), Some("Ysolde"));
    assert_eq!(bearer(&house.heroes, &[], &lock_quest(&content, 0)), None);
}

#[test]
fn the_locks_are_tried_might_wits_spirit_each_told_by_its_ending_naming_the_bearer() {
    let (content, mut house) = at_the_door();
    party_a(&mut house);
    // Margins +4 +2 +3 at a 7 each: a triumph, then two successes.
    let (record, pages) = tried(&content, &mut house, [[3, 4], [4, 3], [5, 2]]);
    let titles: Vec<&str> = pages.iter().map(|p| p.quest.title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "The lock of iron",
            "The lock of riddles",
            "The lock of breath"
        ]
    );
    let outcomes: Vec<Outcome> = pages.iter().map(|p| p.outcome).collect();
    assert_eq!(
        outcomes,
        [Outcome::Triumph, Outcome::Success, Outcome::Success]
    );
    assert_eq!(pages[0].margin, 4);
    assert_eq!(
        pages[0].story,
        "Garrick, Maren, Ysolde and Brannoc came down the last steps to the lock of iron. \
         Garrick took hold of the bar and lifted it as if it had been waiting to be asked."
    );
    assert_eq!(
        pages[1].story,
        "At the second lock a riddle older than the house was cut into the lintel. Garrick, \
         Maren, Ysolde and Brannoc argued over it half the night, and at dawn Ysolde gave it \
         back, word for word, turned around. The lock of riddles turned."
    );
    assert_eq!(record.locks_opened(), 3);
    let bearers: Vec<&str> = record
        .tried
        .iter()
        .map(|t| house.heroes[t.bearer].name.as_str())
        .collect();
    assert_eq!(bearers, ["Garrick", "Ysolde", "Ysolde"]);
}

#[test]
fn a_lock_opened_is_the_living_bearers_deed_and_every_living_member_stood_at_the_door() {
    let (content, mut house) = at_the_door();
    let party = party_a(&mut house);
    let (_, _) = tried(&content, &mut house, [[6, 6], [6, 6], [6, 6]]);
    let opened = |name: &str| -> Vec<(i32, i32, Option<Place>)> {
        house.heroes[id(&house.heroes, name)]
            .deeds
            .iter()
            .filter(|d| d.kind == DeedKind::OpenedALock)
            .map(|d| (d.weight, d.year, d.place))
            .collect()
    };
    assert_eq!(opened("Garrick"), [(0, 26, Some(Place::SealedDoor))]);
    assert_eq!(
        opened("Ysolde"),
        [
            (1, 26, Some(Place::SealedDoor)),
            (2, 26, Some(Place::SealedDoor))
        ]
    );
    assert!(opened("Maren").is_empty() && opened("Brannoc").is_empty());
    let garrick = &house.heroes[party[0]];
    let telling: Vec<&str> = garrick
        .deeds
        .iter()
        .filter(|d| d.kind == DeedKind::OpenedALock)
        .map(|d| d.telling.as_str())
        .collect();
    assert_eq!(telling, ["opened the lock of iron at the Sealed Door"]);
    for &member in &party {
        let stood: Vec<i32> = house.heroes[member]
            .deeds
            .iter()
            .filter(|d| d.kind == DeedKind::StoodAtTheDoor)
            .map(|d| d.weight)
            .collect();
        assert_eq!(stood, [3], "{}", house.heroes[member].name);
    }
}

#[test]
fn a_lock_that_does_not_give_opens_nothing_and_the_stood_deed_weighs_the_locks_that_did() {
    let (content, mut house) = at_the_door();
    let party = party_a(&mut house);
    // Might +9 (a triumph), Wits 35 + 2 - 41 = -4 (a setback), Spirit 36 - 2 (the wound)
    // - 1 (Ysolde's dread 3) + 12 - 41 = +4: opened.
    let (record, pages) = tried(&content, &mut house, [[6, 6], [1, 1], [6, 6]]);
    assert_eq!(pages[1].outcome, Outcome::Setback);
    assert_eq!(record.locks_opened(), 2);
    assert!(!record.tried[1].opened);
    let ysolde = id(&house.heroes, "Ysolde");
    assert!(
        !house.heroes[ysolde]
            .deeds
            .iter()
            .any(|d| d.kind == DeedKind::OpenedALock && d.weight == 1),
        "a setback opens nothing"
    );
    for &member in party.iter().filter(|&&m| house.heroes[m].is_living()) {
        let stood = house.heroes[member]
            .deeds
            .iter()
            .find(|d| d.kind == DeedKind::StoodAtTheDoor)
            .map(|d| d.weight);
        assert_eq!(stood, Some(2));
    }
}

#[test]
fn a_wound_and_dread_at_one_lock_carry_into_the_next() {
    let (content, mut house) = at_the_door();
    party_a(&mut house);
    // Might 38 + 2 - 41 = -1: a setback. Someone is wounded (-2 at the next lock), and
    // Ysolde, who fears the dark with no steadying companion, takes 2 dread: her penalty
    // grows from 2 to 3. Wits 35 - 3 = 32, whoever the wound fell to.
    let (record, pages) = tried(&content, &mut house, [[1, 1], [6, 6], [6, 6]]);
    assert_eq!(pages[0].outcome, Outcome::Setback);
    assert_eq!(pages[1].power, 32);
    let ysolde = id(&house.heroes, "Ysolde");
    assert!(house.heroes[ysolde].fear.dread >= 2);
    assert_eq!(record.tried[1].standing.len(), 4);
    let wounded = house.heroes.iter().filter(|h| h.wounded).count();
    assert_eq!(wounded, 1, "the wound is still on them after the Door");
}

#[test]
fn a_hero_who_breaks_at_the_door_keeps_going_and_pays_the_fear_at_every_lock() {
    let (content, mut house) = at_the_door();
    party_a(&mut house);
    let ysolde = id(&house.heroes, "Ysolde");
    house.heroes[ysolde].fear.dread = 4;
    // At dread 4 her Might is 7 - 4 + 5 = 8: the party brings 36. 36 + 2 - 41 = -3, a
    // setback: 2 dread more, and she breaks at 5.
    let (record, pages) = tried(&content, &mut house, [[1, 1], [6, 6], [6, 6]]);
    assert_eq!(pages[0].outcome, Outcome::Setback);
    assert!(house.heroes[ysolde].fear.broken);
    assert!(
        record.tried[1].standing.contains(&ysolde),
        "a broken hero keeps going"
    );
    // Wits: her 9 - (2 + 5/2) + 5 = 10, the others' 7 + 9 + 6 and +1: 33, less the wound
    // the setback gave one of them.
    assert_eq!(pages[1].power, 31);
    assert_eq!(
        house.heroes[ysolde].fears_faced, 1,
        "a broken fear is not faced again"
    );
}

#[test]
fn whoever_dies_at_one_lock_does_not_stand_at_the_next_and_lies_among_the_doors_fallen() {
    let (content, mut house) = at_the_door();
    // Party C, weak in Might: 3+1 + 3 + (3-2+5) + 3, +2 -1 = 17, a disaster at any dice.
    // Garrick and Ysolde come wounded: a disaster kills them whatever the roll (the roll,
    // or the second wound). Maren is mended instead of rolling; Brannoc's fire shields him.
    let party = vec![
        make(&mut house, "Garrick", [3, 9, 9]),
        make(&mut house, "Maren", [3, 9, 9]),
        make(&mut house, "Ysolde", [3, 9, 9]),
        make(&mut house, "Brannoc", [3, 9, 9]),
    ];
    let (garrick, maren, ysolde, brannoc) = (party[0], party[1], party[2], party[3]);
    seat(&mut house, 0, &party);
    house.heroes[garrick].wounded = true;
    house.heroes[ysolde].wounded = true;
    for seed in 0..8 {
        let mut staged = house.clone();
        let mut pages = Vec::new();
        let record = try_the_door(
            &content,
            &mut staged,
            &mut Rng::from_seed(seed),
            Some([[6, 6], [6, 6], [6, 6]]),
            &mut pages,
        );
        assert_eq!(pages[0].outcome, Outcome::Disaster);
        // At the lock of riddles only Maren (mended at 9, the cap, and wounded: 9 - 2) and
        // Brannoc (9 - 2)
        // stand: 14, a disaster, and Maren — wounded and mended already — dies of it.
        assert_eq!(record.tried[1].standing, [maren, brannoc]);
        assert_eq!(pages[1].power, 14);
        assert_eq!(pages[1].outcome, Outcome::Disaster);
        // At the lock of breath Brannoc stands alone, and lives: the fire is his end.
        assert_eq!(record.tried[2].standing, [brannoc]);
        assert!(staged.heroes[brannoc].is_living());
        for dead in [garrick, ysolde, maren] {
            assert!(!staged.heroes[dead].is_living());
            assert!(staged.fallen_at(Place::SealedDoor).contains(&dead));
            assert_eq!(staged.heroes[dead].death_place, Some(Place::SealedDoor));
            // The roll's "fell at", or the second wound's "died of wounds".
            let fate = &staged.heroes[dead].fate_telling;
            assert!(
                fate == "fell at the Sealed Door"
                    || fate == "died of wounds on the road home from the Sealed Door",
                "{fate}"
            );
            assert_eq!(staged.heroes[dead].fate_year, 26);
        }
        assert_eq!(
            staged.mourned,
            [garrick, ysolde, maren],
            "in order of death"
        );
        // Three disasters cost the house 3 each, untold.
        assert_eq!(staged.renown, 15 - 9);
        assert!(
            !pages
                .iter()
                .flat_map(|p| &p.lines)
                .any(|l| l.starts_with("Word of it got about."))
        );
        assert_eq!(record.locks_opened(), 0);
    }
}

#[test]
fn nobody_left_standing_ends_the_door_early() {
    let (content, mut house) = at_the_door();
    let garrick = make(&mut house, "Garrick", [9, 9, 9]);
    seat(&mut house, 0, &[garrick]);
    house.heroes[garrick].wounded = true;
    // Alone and wounded: Might 9 + 1 - 2 = 8, a disaster at any dice; the death roll or the
    // second wound kills him, and no one is left for the second lock.
    let (record, pages) = tried(&content, &mut house, [[6, 6], [6, 6], [6, 6]]);
    assert_eq!(pages.len(), 1);
    assert_eq!(record.tried.len(), 1);
    assert!(!house.heroes[garrick].is_living());
    assert!(
        house.heroes[garrick]
            .deeds
            .iter()
            .all(|d| d.kind != DeedKind::StoodAtTheDoor),
        "the dead do not stand at the Door"
    );
}

#[test]
fn at_the_door_the_renown_counts_and_nothing_else_of_the_reward_is_told_taught_or_shared() {
    let (content, mut house) = at_the_door();
    let party = party_a(&mut house);
    let before: Vec<[i32; 3]> = party.iter().map(|&m| house.heroes[m].aptitudes).collect();
    let (_, pages) = tried(&content, &mut house, [[6, 6], [6, 6], [6, 6]]);
    assert!(pages.iter().all(|p| p.outcome == Outcome::Triumph));
    // 5 + 1 a triumph, three times, for the house and for each.
    assert_eq!(house.renown, 15 + 18);
    assert_eq!(house.heroes[party[1]].renown, 4 + 18);
    for page in &pages {
        assert!(
            !page.lines.iter().any(|l| l.contains("renown to the house")),
            "no reward line at the Door: {:?}",
            page.lines
        );
    }
    let after: Vec<[i32; 3]> = party.iter().map(|&m| house.heroes[m].aptitudes).collect();
    assert_eq!(before, after, "no triumph lesson at the Door");
    for &member in &party {
        assert!(
            house.heroes[member]
                .deeds
                .iter()
                .all(|d| d.kind != DeedKind::Triumph)
        );
    }
    // No road shared: the rivals stay rivals, the strangers stay strangers.
    let (garrick, ysolde, brannoc) = (party[0], party[2], party[3]);
    assert_eq!(
        house.heroes[brannoc]
            .bond_to(ysolde)
            .map(|b| (b.kind, b.shared_successes)),
        Some((BondKind::Rival, 0))
    );
    assert!(house.heroes[garrick].bond_to(brannoc).is_none());
    // Everything else applies: the Door's history, the roads walked.
    assert_eq!(house.places[Place::SealedDoor.index()].visits, 3);
    assert_eq!(house.places[Place::SealedDoor.index()].triumphs, 3);
    assert!(
        house.heroes[garrick]
            .roads_walked
            .contains(&Place::SealedDoor)
    );
}

#[test]
fn a_first_quest_at_the_door_is_still_a_first_quest() {
    let (content, mut house) = at_the_door();
    let pip = make(&mut house, "Pip", [9, 9, 9]);
    seat(&mut house, 0, &[pip]);
    let (_, _) = tried(&content, &mut house, [[6, 6], [6, 6], [6, 6]]);
    let first: Vec<(i32, Option<Place>)> = house.heroes[pip]
        .deeds
        .iter()
        .filter(|d| d.kind == DeedKind::FirstQuest)
        .map(|d| (d.weight, d.place))
        .collect();
    assert_eq!(first, [(3, Some(Place::SealedDoor))]);
}

#[test]
fn the_prologue_names_the_four_what_each_carried_who_went_beside_whom_and_what_they_brought() {
    let (content, mut house) = at_the_door();
    let party: Vec<HeroId> = ["Garrick", "Maren", "Ysolde", "Brannoc"]
        .iter()
        .map(|n| id(&house.heroes, n))
        .collect();
    seat(&mut house, 0, &party);
    let lines = crate::door_prologue::prologue(&content, &house, &party);
    assert_eq!(
        lines,
        [
            "In the last summer the Sealed Door stood open, as it had been foretold for \
             twenty-five years. The house sent Garrick Thorne, Maren Thorne, Ysolde Vane and \
             Brannoc Hale.",
            "Garrick was 62, and of the house before its years were counted. He carried \
             Thornfall. Carried by every Thorne since the first. Nobody remembers who made it.",
            "Maren was 38, the daughter of Garrick and Elsbeth.",
            "Ysolde was 24, and of the house before its years were counted. The Seer had said \
             it: \"You will open the Sealed Door.\" +5 at every lock. She had been afraid of \
             the dark all her life, and went down into it anyway: -2.",
            "Brannoc was 31, and of the house before its years were counted.",
            "Garrick went down beside his daughter Maren (+2), at every lock.",
            "Ysolde went down beside her rival Brannoc (-1), at every lock.",
            "Against three locks of 34, 34, and 34, they brought Might 22, Wits 22, and \
             Spirit 18. The dark and the cold lay ahead.",
        ]
    );
}

#[test]
fn the_prologue_tells_a_grandchild_an_arrival_the_blood_a_conquest_a_blessing_and_a_wound() {
    let (content, mut house) = at_the_door();
    let (pip, wren, ysolde, odo) = (
        id(&house.heroes, "Pip"),
        id(&house.heroes, "Wren"),
        id(&house.heroes, "Ysolde"),
        id(&house.heroes, "Odo"),
    );
    // Pip: Maren's son, Garrick and Elsbeth's grandson, promised by blood, wounded.
    house.heroes[pip].age = 20;
    house.heroes[pip].destiny.kind = crate::ids::Destiny::OpenTheSealedDoor;
    house.heroes[pip].destiny.blood_of = Some("Ysolde".to_owned());
    house.heroes[pip].wounded = true;
    // Wren: afraid of the cold no more — conquered in year 9; blessed everywhere.
    house.heroes[wren].age = 18;
    house.heroes[wren].fear.tag = Tag::Cold;
    house.heroes[wren].fear.conquered = true;
    house.heroes[wren].deeds.push(crate::epitaph_tests::deed(
        DeedKind::ConqueredFear,
        9,
        14,
        None,
        0,
    ));
    house.heroes[wren].blessings.push(crate::hero::Blessing {
        title: "Odo's patience".to_owned(),
        scope: crate::hero::Scope::Everywhere,
        power: 1,
    });
    // Ysolde born brave of the dark; Odo come up the road in year 3.
    house.heroes[ysolde].fear.born_brave = true;
    house.heroes[ysolde].fear.conquered = true;
    house.heroes[odo].deeds.push(crate::epitaph_tests::deed(
        DeedKind::Arrived,
        3,
        40,
        None,
        0,
    ));
    let lines = crate::door_prologue::prologue(&content, &house, &[pip, wren, ysolde, odo]);
    assert_eq!(
        lines[1],
        "Pip was 20, the son of Maren and the grandson of Garrick and Elsbeth. The Door was \
         promised to Ysolde, and he went as Ysolde's blood: +5 at every lock. He had been \
         afraid of the dark all his life, and went down into it anyway: -2. He went \
         wounded: -2."
    );
    assert_eq!(
        lines[2],
        "Wren was 18, the daughter of Brannoc and Aud. She had conquered her fear of the cold \
         in year 9, and here it counted: +2. Odo's patience went with her: +1."
    );
    assert_eq!(
        lines[3],
        "Ysolde was 24, and of the house before its years were counted. The Seer had said \
         it: \"You will open the Sealed Door.\" +5 at every lock. She was born unafraid of \
         the dark, and here it counted: +2."
    );
    assert_eq!(lines[4], "Odo was 47, and had come up the road in year 3.");
    // No conquest on record: the conquered line without a year.
    house.heroes[wren].deeds.clear();
    let lines = crate::door_prologue::prologue(&content, &house, &[wren]);
    assert_eq!(
        lines[1],
        "Wren was 18, the daughter of Brannoc and Aud. She had conquered her fear of the cold, \
         and here it counted: +2. Odo's patience went with her: +1."
    );
}

#[test]
fn an_heirloom_of_ones_own_making_is_carried_as_ones_own() {
    let (content, mut house) = at_the_door();
    let maren = id(&house.heroes, "Maren");
    house.heroes[maren].heirloom = Some(crate::hero::Heirloom {
        name: "Maren's road-book".to_owned(),
        sprite: "reward-guidebook".to_owned(),
        aptitude: Aptitude::Wits,
        bonus: 2,
        provenance: "Kept by Maren, year 4.".to_owned(),
    });
    let lines = crate::door_prologue::prologue(&content, &house, &[maren]);
    assert_eq!(
        lines[1],
        "Maren was 38, the daughter of Garrick and Elsbeth. She carried her own road-book. Kept \
         by Maren, year 4."
    );
}
