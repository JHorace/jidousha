//! W9's epitaph rules, each behaviour a test named as a sentence: the wording roll, the
//! first four parts' selection rules branch by branch (the last five are
//! `epitaph_ends_tests.rs`), the priority walk under the budget, the frame's order and the
//! subject named (SPEC §20).
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from SPEC.md and
//! `content/epitaph.json`'s templates — never computed by the code under test.

use jidousha::prelude::Rng;

use crate::bonds::form;
use crate::content::Content;
use crate::epitaph::{Wording, compose, name_the_subject, roll_wording};
use crate::epitaph_parts::parts;
use crate::hero::{Deed, DeedKind, DreamFate, Fate, Hero, HeroId};
use crate::ids::{BondKind, Part, Place};
use crate::testkit::{founded, id};
use crate::text::WritingMemory;

/// A wording on frame 0 with every coin `coin`.
pub fn all(coin: bool) -> Wording {
    Wording {
        frame: 0,
        coins: [coin; 9],
    }
}

/// `part` of `name`'s epitaph under every coin `coin`.
pub fn part(content: &Content, heroes: &[Hero], name: &str, which: Part, coin: bool) -> String {
    parts(content, heroes, id(heroes, name), all(coin))[which.index()].clone()
}

/// Both variants of `which` for `name`: (coin 1, coin 0).
pub fn both(content: &Content, heroes: &[Hero], name: &str, which: Part) -> (String, String) {
    (
        part(content, heroes, name, which, true),
        part(content, heroes, name, which, false),
    )
}

pub fn deed(kind: DeedKind, year: i32, age: i32, place: Option<Place>, weight: i32) -> Deed {
    Deed {
        kind,
        year,
        age,
        place,
        weight,
        other: None,
        telling: String::new(),
    }
}

pub fn kill(heroes: &mut [Hero], who: HeroId, year: i32, age: i32, telling: &str) {
    let hero = &mut heroes[who];
    hero.fate = Fate::Dead;
    hero.fate_year = year;
    hero.fate_age = age;
    hero.fate_telling = telling.to_owned();
}

#[test]
fn a_wording_never_repeats_the_last_frame_rolled_and_its_coins_are_fair() {
    let mut writing = WritingMemory::default();
    let mut rng = Rng::from_seed(9);
    let mut previous = None;
    let mut firsts = [0usize; 3];
    let mut frames = [0usize; 3];
    let mut heads = [0usize; 9];
    const ROLLS: usize = 30_000;
    for n in 0..ROLLS {
        if n % 10 == 0 {
            // A fresh run: its first roll is uniform over all three frames.
            writing = WritingMemory::default();
            previous = None;
        }
        let wording = roll_wording(&mut writing, &mut rng);
        assert!(wording.frame < 3, "frame {}", wording.frame);
        assert_ne!(
            Some(wording.frame),
            previous,
            "a frame repeated the last one"
        );
        if previous.is_none() {
            firsts[wording.frame] += 1;
        }
        frames[wording.frame] += 1;
        for (count, coin) in heads.iter_mut().zip(wording.coins) {
            *count += usize::from(coin);
        }
        previous = Some(wording.frame);
        assert_eq!(writing.last_frame, Some(wording.frame));
    }
    for count in firsts {
        let share = count as f64 / (ROLLS / 10) as f64;
        assert!((share - 1.0 / 3.0).abs() < 0.03, "first frames {firsts:?}");
    }
    for count in frames {
        let share = count as f64 / ROLLS as f64;
        assert!((share - 1.0 / 3.0).abs() < 0.02, "frames {frames:?}");
    }
    for count in heads {
        let share = count as f64 / ROLLS as f64;
        assert!((share - 0.5).abs() < 0.02, "coins {heads:?}");
    }
}

#[test]
fn origin_tells_an_arrival_a_birth_in_play_a_child_of_the_old_house_and_a_founder() {
    let (content, mut heroes) = founded();
    assert_eq!(
        both(&content, &heroes, "Garrick", Part::Origin),
        (
            "He was here when the counting of years began.".to_owned(),
            "He was of the house before anyone counted its years.".to_owned()
        )
    );
    assert_eq!(
        part(&content, &heroes, "Maren", Part::Origin, true),
        "She was the child of Garrick and Elsbeth, and was of the house before its years were \
         counted."
    );
    // The dead before the first year have no origin; Elsbeth died in year -18.
    assert_eq!(part(&content, &heroes, "Elsbeth", Part::Origin, true), "");
    let pip = id(&heroes, "Pip");
    heroes[pip].born_year = 3;
    assert_eq!(
        both(&content, &heroes, "Pip", Part::Origin),
        (
            "He was born under this roof in year 3, the child of Maren.".to_owned(),
            "He was born in the house in year 3, to Maren.".to_owned()
        )
    );
    // Dead in play, a founder is still a founder.
    let garrick = id(&heroes, "Garrick");
    kill(&mut heroes, garrick, 3, 64, "fell at the Barrow");
    assert_eq!(
        part(&content, &heroes, "Garrick", Part::Origin, false),
        "He was of the house before anyone counted its years."
    );
    // An arrival comes first, whatever else is true.
    let odo = id(&heroes, "Odo");
    heroes[odo]
        .deeds
        .push(deed(DeedKind::Arrived, 4, 22, None, 0));
    assert_eq!(
        both(&content, &heroes, "Odo", Part::Origin),
        (
            "He was not born here. He came in year 4, at 22, and stayed.".to_owned(),
            "He came up the road in year 4, aged 22, and was let in.".to_owned()
        )
    );
}

#[test]
fn roads_tell_the_yards_teachers_the_child_who_never_went_the_house_kept_and_the_roads_walked() {
    let (content, mut heroes) = founded();
    let (pip, odo, wren, ysolde) = (
        id(&heroes, "Pip"),
        id(&heroes, "Odo"),
        id(&heroes, "Wren"),
        id(&heroes, "Ysolde"),
    );
    assert_eq!(part(&content, &heroes, "Pip", Part::Roads, true), "");
    form(&mut heroes, pip, odo, BondKind::Mentor, 1);
    assert_eq!(
        part(&content, &heroes, "Pip", Part::Roads, true),
        "Odo taught him on the bench in the yard."
    );
    heroes[wren].fate = Fate::Dead;
    assert_eq!(
        part(&content, &heroes, "Wren", Part::Roads, true),
        "She was never old enough for the roads."
    );
    heroes[ysolde].quests_faced = 0;
    assert_eq!(
        both(&content, &heroes, "Ysolde", Part::Roads),
        (
            "She kept the house, and never went out from it.".to_owned(),
            "She never went out on the roads.".to_owned()
        )
    );
    // Garrick quested before the first year: no FIRST_QUEST deed (SPEC-GAPS KG-40).
    assert_eq!(
        part(&content, &heroes, "Garrick", Part::Roads, true),
        "He went out 30 times in all."
    );
    heroes[ysolde].quests_faced = 1;
    let first = deed(DeedKind::FirstQuest, 1, 24, Some(Place::Barrow), 2);
    heroes[ysolde].deeds.push(first);
    assert_eq!(
        part(&content, &heroes, "Ysolde", Part::Roads, true),
        "She went out once, at 24, to the Barrow."
    );
    heroes[ysolde].quests_faced = 3;
    assert_eq!(
        both(&content, &heroes, "Ysolde", Part::Roads),
        (
            "Her first road was to the Barrow, at 24. She went out three times in all.".to_owned(),
            "She first went out at 24, to the Barrow, and went out three times in all.".to_owned()
        )
    );
}

#[test]
fn triumph_tells_the_heaviest_triumph_the_later_on_a_tie() {
    let (content, mut heroes) = founded();
    assert_eq!(part(&content, &heroes, "Brannoc", Part::Triumph, true), "");
    let brannoc = id(&heroes, "Brannoc");
    for (year, place, weight) in [
        (2, Place::Barrow, 2),
        (3, Place::Emberfall, 3),
        (5, Place::Deepwood, 3),
        (6, Place::HighPass, 1),
    ] {
        let triumph = deed(DeedKind::Triumph, year, 31, Some(place), weight);
        heroes[brannoc].deeds.push(triumph);
    }
    assert_eq!(
        both(&content, &heroes, "Brannoc", Part::Triumph),
        (
            "He did something at the Deepwood in year 5 that is still spoken of.".to_owned(),
            "His best day was at the Deepwood, in year 5.".to_owned()
        )
    );
}

#[test]
fn fear_tells_born_brave_before_conquered_before_grief_before_broken_before_faced() {
    let (content, mut heroes) = founded();
    let (maren, odo, wren, elsbeth) = (
        id(&heroes, "Maren"),
        id(&heroes, "Odo"),
        id(&heroes, "Wren"),
        id(&heroes, "Elsbeth"),
    );
    assert_eq!(
        part(&content, &heroes, "Garrick", Part::Fear, true),
        "He was afraid of deep water, and was kept from it."
    );
    assert_eq!(
        part(&content, &heroes, "Pip", Part::Fear, true),
        "Even small, he was afraid of the dark."
    );
    assert_eq!(
        both(&content, &heroes, "Elsbeth", Part::Fear),
        (
            "She never stopped being afraid of high places. She went twice against it all the \
             same."
                .to_owned(),
            "She was afraid of high places all her life, and went anyway.".to_owned()
        )
    );
    heroes[maren]
        .deeds
        .push(deed(DeedKind::Broken, 3, 40, Some(Place::DrownedCoast), 0));
    assert_eq!(
        both(&content, &heroes, "Maren", Part::Fear),
        (
            "In year 3 deep water broke her, and she never faced deep water again.".to_owned(),
            "She went against deep water once too often. After year 3 she would not.".to_owned()
        )
    );
    let mut grief = deed(DeedKind::Broken, 4, 41, None, 0);
    grief.other = Some(elsbeth);
    heroes[maren].deeds.push(grief);
    assert_eq!(
        part(&content, &heroes, "Maren", Part::Fear, true),
        "The year Elsbeth died, grief broke her, and she would never face deep water again."
    );
    heroes[odo]
        .deeds
        .push(deed(DeedKind::ConqueredFear, 4, 50, None, 0));
    heroes[odo].fears_faced = 3;
    assert_eq!(
        both(&content, &heroes, "Odo", Part::Fear),
        (
            "He conquered his fear of crowds in year 4.".to_owned(),
            "He was afraid of crowds until year 4, and then he was not.".to_owned()
        )
    );
    heroes[wren].fear.born_brave = true;
    heroes[wren]
        .deeds
        .push(deed(DeedKind::ConqueredFear, 4, 12, None, 0));
    assert_eq!(
        part(&content, &heroes, "Wren", Part::Fear, true),
        "She was born unafraid of fire, which was a gift from those before her."
    );
}

#[test]
fn a_part_too_long_for_the_budget_is_skipped_and_a_later_one_still_chosen() {
    let (content, mut heroes) = founded();
    let (maren, pip) = (id(&heroes, "Maren"), id(&heroes, "Pip"));
    kill(&mut heroes, maren, 3, 40, "fell at the Drowned Coast");
    heroes[maren].dream_fate = DreamFate::PassedOn;
    heroes[maren].bequest_heir = Some(pip);
    heroes[maren].bequest_heirloom = Some("Thornfall".to_owned());
    // ORIGIN 1, END 2 (coin 1), DREAM 2 (coin 0): five; PROPHECY's two do not fit, LEFT's
    // one does; LOVE, TRIUMPH, FEAR and ROADS find the budget spent.
    let mut coins = [true; 9];
    coins[Part::Dream.index()] = false;
    let wording = Wording { frame: 0, coins };
    assert_eq!(
        compose(&content, &heroes, maren, wording),
        "Maren Thorne was the child of Garrick and Elsbeth, and was of the house before its \
         years were counted. She wanted to avenge her mother, and died before it was done. Pip \
         carries it now. In year 3 she fell at the Drowned Coast. She was 40. Thornfall went to \
         Pip."
    );
}

#[test]
fn elsbeths_epitaph_drops_her_roads_only_when_her_fear_takes_two_sentences() {
    let (content, heroes) = founded();
    let elsbeth = id(&heroes, "Elsbeth");
    let mut coins = [true; 9];
    assert_eq!(
        compose(&content, &heroes, elsbeth, Wording { frame: 0, coins }),
        "Elsbeth Thorne never stopped being afraid of high places. She went twice against it all \
         the same. She wanted to see the sea, and died before it was done. She loved Garrick \
         Thorne, and Maren. She was lost at the Drowned Coast, before the first year. She was 41."
    );
    coins[Part::Fear.index()] = false;
    assert_eq!(
        compose(&content, &heroes, elsbeth, Wording { frame: 1, coins }),
        "Elsbeth Thorne was lost at the Drowned Coast, before the first year. She was 41. She \
         went out eleven times in all. She wanted to see the sea, and died before it was done. \
         She was afraid of high places all her life, and went anyway. She loved Garrick Thorne, \
         and Maren."
    );
    assert_eq!(
        compose(&content, &heroes, elsbeth, Wording { frame: 2, coins }),
        "Elsbeth Thorne wanted to see the sea, and died before it was done. She went out eleven \
         times in all. She was afraid of high places all her life, and went anyway. She loved \
         Garrick Thorne, and Maren. She was lost at the Drowned Coast, before the first year. \
         She was 41."
    );
}

#[test]
fn the_first_part_names_its_subject_by_a_leading_pronoun_a_possessive_or_one_inside() {
    let (content, heroes) = founded();
    let (garrick, maren) = (
        &heroes[id(&heroes, "Garrick")],
        &heroes[id(&heroes, "Maren")],
    );
    let named = |hero: &Hero, part: &str| name_the_subject(&content, hero, part);
    assert_eq!(
        named(garrick, "He went out 30 times in all."),
        "Garrick Thorne went out 30 times in all."
    );
    assert_eq!(
        named(
            maren,
            "Her first road was to the Barrow, at 24. She went out once."
        ),
        "Maren Thorne's first road was to the Barrow, at 24. She went out once."
    );
    assert_eq!(
        named(
            maren,
            "In year 3 she fell at the Drowned Coast. She was 40."
        ),
        "In year 3 Maren Thorne fell at the Drowned Coast. She was 40."
    );
    assert_eq!(
        named(garrick, "The Seer said he would outlive those he loved."),
        "The Seer said Garrick Thorne would outlive those he loved."
    );
    assert_eq!(
        named(garrick, "Thornfall went to Maren."),
        "Thornfall went to Maren."
    );
}
