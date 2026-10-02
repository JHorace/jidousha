//! W8's newcomers, each behaviour a test named as a sentence: births (SPEC §17.3),
//! comings of age (§17.5, §9.5) and wanderers (§17.2), and their place in the turning.
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from SPEC.md,
//! CONSTANTS.md or the content — never computed by the code under test.

use jidousha::prelude::Rng;

use crate::births::{births, may_have_a_child};
use crate::bonds::{change, form};
use crate::content::Content;
use crate::dream::Dream;
use crate::ghost::Ghost;
use crate::hero::{DeedKind, DreamFate, Fate, HeroId};
use crate::house::House;
use crate::ids::{Aptitude, BondKind, Destiny, DreamKind, Place, Pool, Pronoun, Tag, Vocation};
use crate::passage::PageKind;
use crate::testkit::{house, id};
use crate::text::fmt;
use crate::turning::turn_the_year;
use crate::wanderer::{Odds, wanderer, wanderer_odds};
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
fn pip_comes_of_age_a_priest_under_odo_keeps_his_dream_and_hears_the_seer() {
    let (content, mut house) = house();
    let (pip, odo) = (id(&house.heroes, "Pip"), id(&house.heroes, "Odo"));
    house.heroes[pip].age = 11;
    form(&mut house.heroes, pip, odo, BondKind::Mentor, 1);
    turn(&content, &mut house, 4);
    let pages = &house.passage.as_ref().expect("a turning").pages;
    let page = pages
        .iter()
        .find(|p| p.kind == PageKind::ComingOfAge)
        .expect("a coming of age");
    assert_eq!(page.title, "Pip Thorne comes of age");
    assert_eq!(
        page.lines[..3],
        [
            "Pip is 12, and a priest now. He may quest. He brings 1 less in every aptitude until \
             he is 20. He learns fast: more from a winter's training, and something from any quest \
             that succeeds.",
            "Odo's teaching shows in him. Spirit rises to 3.",
            "He has a dream: to see the sea."
        ]
    );
    let hero = &house.heroes[pip];
    assert_eq!(hero.vocation, Vocation::Priest);
    assert_eq!(hero.aptitudes, [1, 2, 3]);
    assert!(hero.destiny.kind != Destiny::Unspoken);
    let lore = &content.destinies[hero.destiny.kind.index()];
    assert_eq!(
        page.lines[3],
        format!("The Seer took his hand and said: \"{}\"", lore.prophecy)
    );
    assert_eq!(page.lines[4..6], [lore.doom.clone(), lore.gift.clone()]);
    assert_eq!(hero.deeds.last().map(|d| d.kind), Some(DeedKind::CameOfAge));
}

#[test]
fn a_teachers_teaching_does_not_show_past_nine() {
    let (content, mut house) = house();
    let (pip, odo) = (id(&house.heroes, "Pip"), id(&house.heroes, "Odo"));
    house.heroes[pip].age = 11;
    house.heroes[pip].aptitudes = [1, 2, 9];
    form(&mut house.heroes, pip, odo, BondKind::Mentor, 1);
    turn(&content, &mut house, 4);
    assert_eq!(house.heroes[pip].aptitudes, [1, 2, 9]);
    let pages = &house.passage.as_ref().expect("a turning").pages;
    let page = pages
        .iter()
        .find(|p| p.kind == PageKind::ComingOfAge)
        .expect("a coming of age");
    assert!(page.lines[1].starts_with("He has a dream"));
}

#[test]
fn a_calling_is_one_of_the_best_aptitudes_two_when_no_teacher_chose_it() {
    let (content, house) = house();
    let wren = id(&house.heroes, "Wren"); // 2/1/2: Might first on the tie
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..60 {
        let mut copy = house.clone();
        copy.heroes[wren].age = 11;
        turn(&content, &mut copy, seed);
        seen.insert(copy.heroes[wren].vocation);
    }
    assert_eq!(
        seen.into_iter().collect::<Vec<_>>(),
        [Vocation::Knight, Vocation::Warrior]
    );
}

#[test]
fn a_child_whose_parent_fell_on_a_quest_comes_of_age_to_avenge_them_parent_zero_first() {
    let (content, mut house) = house();
    let (wren, brannoc, aud) = (
        id(&house.heroes, "Wren"),
        id(&house.heroes, "Brannoc"),
        id(&house.heroes, "Aud"),
    );
    house.heroes[wren].age = 11;
    let hero = &mut house.heroes[brannoc];
    hero.fate = Fate::Dead;
    hero.grieved = true;
    hero.death_place = Some(Place::Emberfall);
    hero.death_tag = Some(Tag::Fire);
    hero.fate_telling = "fell at Emberfall".to_owned();
    house.heroes[aud].death_place = Some(Place::DrownedCoast);
    house.heroes[aud].death_tag = Some(Tag::Water);
    turn(&content, &mut house, 4);
    let dream = house.heroes[wren].dream.clone().expect("a dream");
    assert_eq!(dream.kind, DreamKind::AvengeTheLost);
    assert_eq!(
        dream.setup.map(|s| (s.place, s.tag, s.lost)),
        Some((Place::Emberfall, Tag::Fire, Some(brannoc)))
    );
    let pages = &house.passage.as_ref().expect("a turning").pages;
    let page = pages
        .iter()
        .find(|p| p.kind == PageKind::ComingOfAge)
        .expect("a coming of age");
    assert_eq!(
        page.lines[1],
        "She has a dream, and the whole house knows why: to avenge her father. Brannoc Hale fell \
         at Emberfall at Emberfall. It was fire."
    );
}

#[test]
fn a_parent_lost_at_the_door_or_in_bed_is_not_avenged() {
    let (content, mut house) = house();
    let (wren, brannoc) = (id(&house.heroes, "Wren"), id(&house.heroes, "Brannoc"));
    house.heroes[wren].age = 11;
    house.heroes[brannoc].fate = Fate::Dead;
    house.heroes[brannoc].grieved = true;
    house.heroes[brannoc].death_place = Some(Place::SealedDoor);
    house.heroes[brannoc].death_tag = Some(Tag::Dark);
    turn(&content, &mut house, 4);
    let dream = house.heroes[wren].dream.clone().expect("a dream");
    assert_ne!(dream.kind, DreamKind::AvengeTheLost);
}

#[test]
fn a_child_takes_up_an_ancestors_ghost_at_coming_of_age_and_the_ghost_is_quiet() {
    let (content, mut house) = house();
    let (wren, brannoc, garrick) = (
        id(&house.heroes, "Wren"),
        id(&house.heroes, "Brannoc"),
        id(&house.heroes, "Garrick"),
    );
    house.heroes[wren].age = 11;
    house.heroes[brannoc].fate = Fate::Dead;
    house.heroes[brannoc].grieved = true;
    let blade = Dream::build(&content, DreamKind::ForgeABlade, None, None).expect("builds");
    let barrow = house.heroes[garrick].dream.clone().expect("a dream");
    let mut owned = blade.clone();
    owned.owner = Some(brannoc);
    house.ghosts = vec![
        Ghost {
            hero: garrick,
            dream: barrow,
            place: Place::Barrow,
        },
        Ghost {
            hero: brannoc,
            dream: owned,
            place: Place::Emberfall,
        },
        Ghost {
            hero: garrick,
            dream: blade,
            place: Place::Deepwood,
        },
    ];
    turn(&content, &mut house, 4);
    let taken = house.heroes[wren].dream.clone().expect("a dream");
    assert_eq!(
        (taken.kind, taken.owner),
        (DreamKind::ForgeABlade, Some(brannoc))
    );
    // A swap remove: the last ghost takes the taken one's place.
    let left: Vec<(HeroId, Place)> = house.ghosts.iter().map(|g| (g.hero, g.place)).collect();
    assert_eq!(left, [(garrick, Place::Barrow), (garrick, Place::Deepwood)]);
    assert_eq!(house.heroes[brannoc].dream_fate, DreamFate::PassedOn);
    assert_eq!(house.heroes[brannoc].bequest_heir, Some(wren));
    let pages = &house.passage.as_ref().expect("a turning").pages;
    let page = pages
        .iter()
        .find(|p| p.kind == PageKind::ComingOfAge)
        .expect("a coming of age");
    assert_eq!(
        page.lines[1],
        "The house's unfinished business finds her. She takes up the dream of Brannoc Hale: to \
         forge a blade worth a name. The ghost is quiet."
    );
}

#[test]
fn a_child_with_nothing_to_take_up_rolls_a_dream_no_one_living_holds() {
    let (content, house) = house();
    let wren = id(&house.heroes, "Wren");
    for seed in 0..30 {
        let mut copy = house.clone();
        copy.heroes[wren].age = 11;
        // Garrick young, so old age takes no dreamer from the claims this turning.
        let garrick = id(&copy.heroes, "Garrick");
        copy.heroes[garrick].age = 30;
        turn(&content, &mut copy, seed);
        let kind = copy.heroes[wren].dream.as_ref().expect("a dream").kind;
        let free = [
            DreamKind::RoofOfTheWorld,
            DreamKind::KnownAtCourt,
            DreamKind::SeeAChildGrown,
        ];
        assert!(free.contains(&kind), "{kind:?}");
    }
}

#[test]
fn a_wanderer_comes_for_certain_to_fewer_than_five_adults_never_to_ten_and_else_by_renown() {
    let (_, mut house) = house();
    assert_eq!(wanderer_odds(&house), Odds::Chance(0.35));
    house.renown = 40;
    assert_eq!(wanderer_odds(&house), Odds::Chance(0.6));
    house.renown = 90;
    assert_eq!(wanderer_odds(&house), Odds::Chance(0.6));
    let odo = id(&house.heroes, "Odo");
    house.heroes[odo].fate = Fate::Dead;
    assert_eq!(wanderer_odds(&house), Odds::Certain);
    let wren = id(&house.heroes, "Wren");
    while house.heroes.iter().filter(|h| h.is_living()).count() < 10 {
        let child = house.heroes[wren].clone();
        house.heroes.push(child);
    }
    assert_eq!(wanderer_odds(&house), Odds::NoRoom);
}

#[test]
fn a_wanderer_is_drawn_from_the_standing_the_house_renown_draws_and_arrives_dated_next_year() {
    let (content, mut house) = house();
    let odo = id(&house.heroes, "Odo");
    house.heroes[odo].fate = Fate::Dead;
    house.calendar.begin_winter();
    for (renown, gift, other, telling) in [
        (
            11,
            (3, 5),
            (1, 3),
            "The house has little name, and draws who it can.",
        ),
        (
            12,
            (4, 6),
            (2, 3),
            "The house has a fair name, and drew a fair hand.",
        ),
        (
            30,
            (5, 7),
            (2, 4),
            "The house has a great name. The best come looking for it.",
        ),
    ] {
        for seed in 0..40 {
            let mut copy = house.clone();
            copy.renown = renown;
            let page = wanderer(&content, &mut copy, &mut Rng::from_seed(seed)).expect("certain");
            let id = copy.heroes.len() - 1;
            let hero = &copy.heroes[id];
            assert!((17..=36).contains(&hero.age) && hero.born_year == 2 - hero.age);
            let gift_at = content.lore.vocation_aptitudes[hero.vocation.index()];
            for aptitude in Aptitude::ALL {
                let (lo, hi) = if *aptitude == gift_at { gift } else { other };
                assert!(
                    (lo..=hi).contains(&hero.base(*aptitude)),
                    "{aptitude:?} {}",
                    hero.base(*aptitude)
                );
            }
            assert!((0..=2).contains(&hero.renown));
            assert!(
                hero.destiny.kind != Destiny::Unspoken
                    && hero.destiny.kind != Destiny::OpenTheSealedDoor
            );
            let kind = hero.dream.as_ref().expect("a dream").kind;
            assert!(kind != DreamKind::AvengeTheLost);
            assert!(hero.age >= 35 || kind != DreamKind::WorthyStudent);
            let deed = hero.deeds.last().expect("a deed");
            assert_eq!((deed.kind, deed.year), (DeedKind::Arrived, 2));
            assert_eq!(page.title, format!("{} arrives", hero.full_name()));
            assert!(page.lines[1].ends_with(telling), "{:?}", page.lines[1]);
            let names = if hero.pronoun == Pronoun::He {
                &content.names.him
            } else {
                &content.names.her
            };
            assert!(names.contains(&hero.name) && content.names.houses.contains(&hero.house));
        }
    }
}

#[test]
fn a_turnings_pages_come_in_the_order_of_its_steps() {
    let (content, mut house, maren, brannoc) = wed_house();
    let (pip, garrick, odo) = (
        id(&house.heroes, "Pip"),
        id(&house.heroes, "Garrick"),
        id(&house.heroes, "Odo"),
    );
    house.heroes[pip].age = 11;
    house.heroes[garrick].age = 93;
    house.heroes[odo].fate = Fate::Dead; // four adults after Garrick: a wanderer is certain
    house.heroes[odo].grieved = true;
    house.mourned.push(odo);
    let _ = (maren, brannoc);
    for seed in 0..60 {
        let mut copy = house.clone();
        turn(&content, &mut copy, seed);
        let kinds: Vec<PageKind> = copy
            .passage
            .as_ref()
            .expect("a turning")
            .pages
            .iter()
            .map(|p| p.kind)
            .collect();
        let births = kinds.iter().filter(|k| **k == PageKind::Birth).count();
        let mut want = vec![PageKind::Winter, PageKind::Death, PageKind::Death];
        want.extend(std::iter::repeat_n(PageKind::Birth, births));
        want.extend([PageKind::ComingOfAge, PageKind::Arrival]);
        assert_eq!(kinds[..want.len()], want[..], "seed {seed}: {kinds:?}");
    }
}

#[test]
fn the_tales_come_before_the_phase_lines_and_each_hero_s_dream_moment_follows_their_own() {
    let (content, mut house) = house();
    let (maren, odo, pip) = (
        id(&house.heroes, "Maren"),
        id(&house.heroes, "Odo"),
        id(&house.heroes, "Pip"),
    );
    house.tales.push(crate::house::Tale {
        title: "The tale of Pip and the sea".to_owned(),
        about: pip,
        since: 1,
    });
    house.heroes[maren].age = 39;
    house.heroes[odo].age = 54;
    house.heroes[odo].renown = 5;
    house.heroes[odo].dream =
        Some(Dream::build(&content, DreamKind::KnownAtCourt, None, None).expect("builds"));
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
    assert!(year.lines[0].starts_with("The tale of Pip and the sea"));
    assert!(year.lines[1].starts_with("Maren is 40"));
    assert!(year.lines[2].starts_with("Odo is 55"));
    assert!(year.lines[3].starts_with("Odo"), "{:?}", year.lines);
    assert_eq!(house.heroes[odo].dream.as_ref().map(|d| d.current), Some(1));
}
