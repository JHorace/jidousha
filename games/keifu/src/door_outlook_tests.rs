//! The Door's outlook (SPEC §16.4) and its bearer rule (§16.2), unit-tested: what each
//! party brings to each lock and opens all three at, the best four of every four in
//! creation order, and who bears each lock. Parties A and B are `door_tests.rs`'s, their
//! numbers worked there by hand.
//!
//! INVARIANT: every expectation is a shipped literal worked by hand from `household.json`,
//! `door.json` and CONSTANTS §3/§12.

use crate::door::{bearer, best_four, lock_quest, outlook};
use crate::door_tests::{at_the_door, make, party_a, party_b};
use crate::ids::Tag;
use crate::testkit::{house, id};

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
