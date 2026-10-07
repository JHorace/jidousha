//! W8's births, each behaviour a test named as a sentence (SPEC §17.3).
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from SPEC.md,
//! CONSTANTS.md or the content — never computed by the code under test.

use jidousha::prelude::Rng;

use crate::births::{births, may_have_a_child};
use crate::bonds::{change, form};
use crate::content::Content;
use crate::hero::{DeedKind, Fate, HeroId};
use crate::house::House;
use crate::ids::{BondKind, Destiny, Pool, Pronoun, Tag};
use crate::passage::PageKind;
use crate::testkit::{house, id};
use crate::text::fmt;
use crate::turning::turn_the_year;
use crate::winter::WinterPlan;

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

/// Year 2's winter, Maren and Brannoc wed in year 1 (and no longer anything else).
fn wed_house() -> (Content, House, HeroId, HeroId) {
    let (content, mut house) = house();
    let (maren, brannoc) = (id(&house.heroes, "Maren"), id(&house.heroes, "Brannoc"));
    form(&mut house.heroes, maren, brannoc, BondKind::Spouse, 1);
    house.calendar.begin_winter();
    house.calendar.begin_summer();
    house.calendar.begin_winter();
    (content, house, maren, brannoc)
}

/// The first seed whose births give a child.
fn a_birth(content: &Content, house: &House) -> (House, Vec<crate::passage::TurnPage>) {
    for seed in 0..200 {
        let mut copy = house.clone();
        let pages = births(content, &mut copy, &mut Rng::from_seed(seed));
        if !pages.is_empty() {
            return (copy, pages);
        }
    }
    panic!("no birth in 200 seeds");
}

#[test]
fn a_wed_pair_may_have_a_child_asked_from_the_partner_created_first_and_never_in_the_wedding_year()
{
    let (_, mut house, maren, brannoc) = wed_house();
    assert_eq!(may_have_a_child(&house, maren), Some(brannoc));
    assert_eq!(
        may_have_a_child(&house, brannoc),
        None,
        "each pair once, from the lower id"
    );
    if let Some(b) = house.heroes[maren]
        .bonds
        .iter_mut()
        .find(|b| b.other == brannoc)
    {
        b.since = 2;
    }
    assert_eq!(
        may_have_a_child(&house, maren),
        None,
        "not in the wedding's year"
    );
}

#[test]
fn both_parents_must_be_eighteen_to_forty_five() {
    let (_, mut house, maren, brannoc) = wed_house();
    for (age, may) in [(17, false), (18, true), (45, true), (46, false)] {
        house.heroes[brannoc].age = age;
        assert_eq!(
            may_have_a_child(&house, maren).is_some(),
            may,
            "Brannoc at {age}"
        );
        house.heroes[brannoc].age = 30;
        house.heroes[maren].age = age;
        assert_eq!(
            may_have_a_child(&house, maren).is_some(),
            may,
            "Maren at {age}"
        );
        house.heroes[maren].age = 30;
    }
}

#[test]
fn the_parents_ages_are_read_after_the_turnings_ageing() {
    let (content, mut house, maren, _) = wed_house();
    house.heroes[maren].age = 45;
    for seed in 0..40 {
        let mut copy = house.clone();
        turn(&content, &mut copy, seed);
        let pages = &copy.passage.as_ref().expect("a turning").pages;
        assert!(
            pages.iter().all(|p| p.kind != PageKind::Birth),
            "46 has no child"
        );
    }
    house.heroes[maren].age = 44;
    let born = (0..40).any(|seed| {
        let mut copy = house.clone();
        turn(&content, &mut copy, seed);
        copy.passage
            .as_ref()
            .expect("a turning")
            .pages
            .iter()
            .any(|p| p.kind == PageKind::Birth)
    });
    assert!(born, "45 after the ageing may have a child");
}

#[test]
fn a_pair_has_at_most_three_children_living_or_dead() {
    let (content, mut house, maren, brannoc) = wed_house();
    for n in 0..3 {
        let mut child = house.heroes[id(&house.heroes, "Wren")].clone();
        child.parents = [Some(maren), Some(brannoc)];
        child.fate = if n == 0 { Fate::Dead } else { Fate::Living };
        house.heroes.push(child);
    }
    assert_eq!(may_have_a_child(&house, maren), None);
    house.heroes.pop();
    assert_eq!(may_have_a_child(&house, maren), Some(brannoc));
    let _ = content;
}

#[test]
fn no_child_is_born_to_a_house_of_twelve_or_a_yard_of_six() {
    let (content, house, _, _) = wed_house();
    let wren = id(&house.heroes, "Wren");
    let mut full = house.clone();
    while full.heroes.iter().filter(|h| h.is_living()).count() < 12 {
        let mut adult = full.heroes[wren].clone();
        adult.age = 30;
        adult.bonds.clear();
        full.heroes.push(adult);
    }
    let mut yard = house.clone();
    while yard
        .heroes
        .iter()
        .filter(|h| h.is_living() && !h.is_adult())
        .count()
        < 6
    {
        let mut child = yard.heroes[wren].clone();
        child.bonds.clear();
        yard.heroes.push(child);
    }
    for seed in 0..50 {
        assert!(births(&content, &mut full.clone(), &mut Rng::from_seed(seed)).is_empty());
        assert!(births(&content, &mut yard.clone(), &mut Rng::from_seed(seed)).is_empty());
    }
    let born = (0..50)
        .filter(|&s| !births(&content, &mut house.clone(), &mut Rng::from_seed(s)).is_empty())
        .count();
    assert!((20..=40).contains(&born), "60% of 50: {born}");
}

#[test]
fn a_child_takes_a_quarter_of_both_parents_the_first_parents_house_both_bonds_and_both_blessings() {
    let (content, mut house, maren, brannoc) = wed_house();
    let rest = crate::hero::Blessing {
        title: "Garrick's rest".to_owned(),
        scope: crate::hero::Scope::AgainstTag(Tag::Undead),
        power: 2,
    };
    let patience = crate::hero::Blessing {
        title: "Odo's patience".to_owned(),
        scope: crate::hero::Scope::Everywhere,
        power: 1,
    };
    house.heroes[maren].blessings = vec![rest.clone()];
    house.heroes[brannoc].blessings = vec![rest.clone(), patience.clone()];
    let (born, pages) = a_birth(&content, &house);
    let child = born.heroes.len() - 1;
    let hero = &born.heroes[child];
    // Maren 4/6/3 and Brannoc 6/2/3: 10/4, 8/4, 6/4, each + 0 or 1.
    assert!([2, 3].contains(&hero.aptitudes[0]) && [2, 3].contains(&hero.aptitudes[1]));
    assert!([1, 2].contains(&hero.aptitudes[2]));
    assert_eq!(
        (hero.house.as_str(), hero.age, hero.born_year),
        ("Thorne", 0, 2)
    );
    assert_eq!(hero.parents, [Some(maren), Some(brannoc)]);
    assert_eq!(
        hero.bond_to(maren).map(|b| (b.kind, b.since)),
        Some((BondKind::Parent, 2))
    );
    assert_eq!(
        born.heroes[brannoc].bond_to(child).map(|b| b.kind),
        Some(BondKind::Child)
    );
    let titles: Vec<&str> = hero.blessings.iter().map(|b| b.title.as_str()).collect();
    assert_eq!(titles, ["Garrick's rest", "Odo's patience"]);
    for parent in [maren, brannoc] {
        let deed = born.heroes[parent].deeds.last().expect("a deed");
        assert_eq!((deed.kind, deed.other), (DeedKind::ChildBorn, Some(child)));
    }
    let page = &pages[0];
    assert_eq!(page.title, format!("{} Thorne is born", hero.name));
    let pool: Vec<String> = content.pools[Pool::Births.index()]
        .iter()
        .map(|l| fmt(l, &["Maren", "Brannoc"]))
        .collect();
    assert!(pool.contains(&page.lines[0]), "{:?}", page.lines[0]);
    let object = if hero.pronoun == Pronoun::He {
        "him"
    } else {
        "her"
    };
    assert_eq!(
        page.lines[1],
        format!("They have named {object} {}.", hero.name)
    );
    let he = if hero.pronoun == Pronoun::He {
        "He"
    } else {
        "She"
    };
    assert_eq!(
        page.lines.last().map(String::as_str),
        Some(format!("{he} is born under 2 blessings: Garrick's rest (+2 against Undead) and Odo's patience (+1 on every quest).").as_str())
    );
}

#[test]
fn a_child_will_surpass_a_childless_parent_by_two_in_every_aptitude_and_the_destiny_comes() {
    let (content, mut house) = house();
    let (ysolde, brannoc) = (id(&house.heroes, "Ysolde"), id(&house.heroes, "Brannoc"));
    change(&mut house.heroes, ysolde, brannoc, BondKind::Spouse, 1);
    house.heroes[ysolde].destiny.kind = Destiny::ChildWillSurpassYou;
    house.heroes[brannoc].destiny.kind = Destiny::ChildWillSurpassYou; // Brannoc has Wren
    house.calendar.begin_winter();
    house.calendar.begin_summer();
    house.calendar.begin_winter();
    let (born, _) = a_birth(&content, &house);
    let child = &born.heroes[born.heroes.len() - 1];
    // Ysolde 2/5/4, Brannoc 6/2/3: 8/4, 7/4, 7/4 + 0 or 1, then + 2.
    assert!([4, 5].contains(&child.aptitudes[0]) && [3, 4].contains(&child.aptitudes[1]));
    assert!([3, 4].contains(&child.aptitudes[2]));
    assert!(born.heroes[ysolde].destiny.fulfilled);
    assert!(
        !born.heroes[brannoc].destiny.fulfilled,
        "Brannoc already had a child"
    );
}

#[test]
fn a_child_is_born_brave_of_a_conquered_fear_half_the_time_and_takes_a_broken_one_mostly() {
    let (content, mut house, maren, brannoc) = wed_house();
    house.heroes[maren].fear.conquered = true; // Water
    let (mut brave, mut n) = (0, 0);
    for seed in 0..600 {
        let mut copy = house.clone();
        if births(&content, &mut copy, &mut Rng::from_seed(seed)).is_empty() {
            continue;
        }
        n += 1;
        let fear = &copy.heroes[copy.heroes.len() - 1].fear;
        if fear.born_brave {
            assert!(fear.conquered && fear.tag == Tag::Water);
            brave += 1;
        }
    }
    let rate = f64::from(brave) / f64::from(n);
    assert!((0.42..0.58).contains(&rate), "born brave {brave} of {n}");
    house.heroes[maren].fear.conquered = false;
    house.heroes[maren].fear.tag = Tag::Fire;
    house.heroes[brannoc].fear.broken = true; // Heights
    let (mut heights, mut n) = (0, 0);
    for seed in 0..600 {
        let mut copy = house.clone();
        if births(&content, &mut copy, &mut Rng::from_seed(seed)).is_empty() {
            continue;
        }
        n += 1;
        heights += i32::from(copy.heroes[copy.heroes.len() - 1].fear.tag == Tag::Heights);
    }
    // Maren's fire first at 40%; then Brannoc's broken heights at 80%; a random roll 1/8.
    // P(heights) = 0.6 * (0.8 + 0.2 / 8) = 0.495.
    let rate = f64::from(heights) / f64::from(n);
    assert!((0.42..0.58).contains(&rate), "heights {heights} of {n}");
}

#[test]
fn the_birth_fear_line_names_the_parent_whose_fear_it_is_the_second_first() {
    let (content, mut house, maren, brannoc) = wed_house();
    house.heroes[maren].fear.tag = Tag::Heights; // both fear Heights
    for seed in 0..300 {
        let mut copy = house.clone();
        let pages = births(&content, &mut copy, &mut Rng::from_seed(seed));
        let Some(page) = pages.first() else { continue };
        let child = &copy.heroes[copy.heroes.len() - 1];
        let (he, his) = if child.pronoun == Pronoun::He {
            ("He", "his")
        } else {
            ("She", "her")
        };
        let want = if child.fear.tag == Tag::Heights {
            format!("{he} has {his} father's fear of high places.")
        } else {
            let noun = &content.lore.tags[child.fear.tag.index()].noun;
            format!(
                "Even in the cradle {} is afraid of {noun}.",
                he.to_lowercase()
            )
        };
        assert_eq!(page.lines[2], want);
    }
    let _ = brannoc;
}

#[test]
fn a_newborn_never_has_less_than_one_and_takes_each_roll_of_zero_or_one() {
    let (content, mut house, maren, brannoc) = wed_house();
    // Spirit 1 and 1: 2 / 4 is 0, + 0 or 1, at least 1 — always 1.
    house.heroes[maren].aptitudes[2] = 1;
    house.heroes[brannoc].aptitudes[2] = 1;
    let mut might = std::collections::BTreeSet::new();
    for seed in 0..120 {
        let mut copy = house.clone();
        if births(&content, &mut copy, &mut Rng::from_seed(seed)).is_empty() {
            continue;
        }
        let child = &copy.heroes[copy.heroes.len() - 1];
        assert_eq!(child.aptitudes[2], 1);
        might.insert(child.aptitudes[0]);
    }
    // Might 4 and 6: 10 / 4 is 2, + 0 or 1.
    assert_eq!(might.into_iter().collect::<Vec<_>>(), [2, 3]);
}

#[test]
fn a_conquered_parent_whose_child_is_not_born_brave_lets_the_other_parents_fear_pass() {
    let (content, mut house, maren, brannoc) = wed_house();
    house.heroes[maren].fear.conquered = true; // Water: born brave half the time
    house.heroes[brannoc].fear.broken = true; // Heights: then 80%
    let (mut heights, mut n) = (0, 0);
    for seed in 0..600 {
        let mut copy = house.clone();
        if births(&content, &mut copy, &mut Rng::from_seed(seed)).is_empty() {
            continue;
        }
        n += 1;
        heights += i32::from(copy.heroes[copy.heroes.len() - 1].fear.tag == Tag::Heights);
    }
    // P(heights) = 0.5 * (0.8 + 0.2 / 8) = 0.4125.
    let rate = f64::from(heights) / f64::from(n);
    assert!((0.34..0.49).contains(&rate), "heights {heights} of {n}");
}
