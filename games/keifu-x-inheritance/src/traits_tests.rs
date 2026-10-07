//! Traits (VARIANT.md): which pass, how two parents' combine, how much chance is in it.
//! Seeded, so a run replays; expectations are shipped literals from VARIANT.md.

use jidousha::prelude::Rng;

use crate::ids::{Aptitude, Trait};
use crate::traits::{Source, trait_from};

/// How many draws `trait_from` consumed on `seed`, and what it gave.
fn run(first: Option<Trait>, second: Option<Trait>, seed: u64) -> (u32, Option<(Trait, Source)>) {
    let mut rng = Rng::from_seed(seed);
    let got = trait_from(first, second, Aptitude::Might, &mut rng);
    let mut probe = Rng::from_seed(seed);
    let next = rng.next_f32();
    let mut used = 0;
    while probe.next_f32() != next {
        used += 1;
        assert!(used < 8, "the generator moved by an unreasonable count");
    }
    (used, got)
}

#[test]
fn two_strong_parents_give_a_strong_child_and_consume_no_roll() {
    for seed in 0..50 {
        let (used, got) = run(Some(Trait::Strong), Some(Trait::Strong), seed);
        assert_eq!((used, got), (0, Some((Trait::Strong, Source::Parent(1)))));
    }
}

#[test]
fn one_strong_parent_passes_it_on_about_half_the_seeds_and_the_rest_spring_rarely() {
    let passed = (0..400)
        .filter(|&seed| {
            matches!(
                run(Some(Trait::Strong), None, seed).1,
                Some((Trait::Strong, Source::Parent(0)))
            )
        })
        .count();
    assert!((160..=240).contains(&passed), "{passed} of 400");
    for seed in 0..400 {
        let (used, got) = run(Some(Trait::Strong), None, seed);
        match got {
            // Passed on the one roll, or sprung on the very trait the parent holds (3).
            Some((Trait::Strong, Source::Parent(0))) => assert!(matches!(used, 1 | 3), "{used}"),
            None => assert_eq!(used, 2, "one pass roll, one spring roll that missed"),
            Some((_, Source::Sprung)) => assert_eq!(used, 3),
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn a_gift_and_a_flaw_leave_the_child_one_of_the_two_never_none_on_one_coin() {
    let (mut gift, mut flaw) = (0, 0);
    for seed in 0..400 {
        let (used, got) = run(Some(Trait::Strong), Some(Trait::Frail), seed);
        assert_eq!(used, 1);
        match got {
            Some((Trait::Strong, Source::Parent(0))) => gift += 1,
            Some((Trait::Frail, Source::Parent(1))) => flaw += 1,
            other => panic!("{other:?}"),
        }
    }
    assert!(gift > 140 && flaw > 140, "{gift} gifts, {flaw} flaws");
}

#[test]
fn neither_parent_having_one_a_child_springs_a_trait_one_seed_in_eight_gift_and_flaw_evenly() {
    let (mut gifts, mut flaws, mut none) = (0, 0, 0);
    for seed in 0..800 {
        let (used, got) = run(None, None, seed);
        match got {
            None => {
                none += 1;
                assert_eq!(used, 1);
            }
            Some((held, Source::Sprung)) => {
                assert_eq!(used, 2);
                assert_eq!(held.aptitude(), Aptitude::Might);
                if held.gift() { gifts += 1 } else { flaws += 1 }
            }
            other => panic!("{other:?}"),
        }
    }
    let sprung = gifts + flaws;
    assert!((70..=130).contains(&sprung), "{sprung} of 800 sprang");
    assert!(gifts > 25 && flaws > 25, "{gifts} gifts, {flaws} flaws");
    assert_eq!(sprung + none, 800);
}

#[test]
fn a_trait_gift_adds_one_and_a_flaw_takes_one_on_its_own_aptitude() {
    assert_eq!(Trait::Strong.power(), 1);
    assert_eq!(Trait::Frail.power(), -1);
    assert_eq!(Trait::Sharp.aptitude(), Aptitude::Wits);
    assert_eq!(Trait::of(Aptitude::Spirit, false), Trait::Faint);
    assert_eq!(Trait::of(Aptitude::Might, true), Trait::Strong);
}

#[test]
fn the_founders_carry_the_authored_traits_and_the_dead_theirs_so_descent_can_be_read() {
    let (_, heroes) = crate::testkit::founded();
    let traits = |name: &str| heroes[crate::testkit::id(&heroes, name)].traits.clone();
    assert_eq!(traits("Garrick"), [Trait::Strong]);
    assert_eq!(traits("Maren"), [Trait::Sharp]);
    assert_eq!(traits("Brannoc"), [Trait::Strong]);
    assert_eq!(traits("Odo"), [Trait::Steadfast]);
    assert_eq!(traits("Elsbeth"), [Trait::Sharp]);
    for none in ["Pip", "Ysolde", "Aud", "Wren"] {
        assert!(traits(none).is_empty(), "{none}");
    }
}

#[test]
fn a_birth_tells_each_trait_the_child_has_and_whose_it_is() {
    use crate::bonds::form;
    use crate::ids::BondKind;
    let (content, mut house) = crate::testkit::house();
    let (maren, brannoc) = (
        crate::testkit::id(&house.heroes, "Maren"),
        crate::testkit::id(&house.heroes, "Brannoc"),
    );
    form(&mut house.heroes, maren, brannoc, BondKind::Spouse, 1);
    house.calendar.begin_winter();
    house.calendar.begin_summer();
    house.calendar.begin_winter();
    let (mut told, mut sprung) = (0, 0);
    for seed in 0..120 {
        let mut copy = house.clone();
        let pages = crate::births::births(&content, &mut copy, &mut Rng::from_seed(seed));
        let Some(page) = pages.first() else { continue };
        let child = &copy.heroes[page.about.expect("a birth page is about the child")];
        let said: Vec<&String> = page
            .lines
            .iter()
            .filter(|l| l.contains(" is ") && l.contains("like") || l.contains("neither parent"))
            .collect();
        assert_eq!(said.len(), child.traits.len(), "{seed}: {:?}", page.lines);
        for held in &child.traits {
            let telling = match held {
                Trait::Strong => "strong",
                Trait::Sharp => "sharp",
                _ => "",
            };
            if telling.is_empty() {
                sprung += 1;
                continue;
            }
            told += 1;
            assert!(
                said.iter()
                    .any(|l| l.contains(&format!(" is {telling}, like "))),
                "{seed}: {said:?}"
            );
        }
    }
    assert!(told > 10 && sprung > 0, "{told} told, {sprung} sprung");
}
