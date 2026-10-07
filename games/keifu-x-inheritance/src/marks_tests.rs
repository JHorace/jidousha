//! The black mark (VARIANT.md): which quests are personal, what a failure costs and writes,
//! how marks weigh and lapse. Expectations are shipped literals from VARIANT.md.

use jidousha::prelude::Rng;

use crate::dream::Dream;
use crate::hero::HeroId;
use crate::house::House;
use crate::ids::{DreamKind, Outcome, Place};
use crate::marks::{Mark, personal, weigh};
use crate::resolve::resolve_rolled;
use crate::testkit::{house, id, seat};

fn mark(origin: HeroId, year: i32) -> Mark {
    Mark {
        origin,
        year,
        place: Place::Barrow,
        generation: 0,
    }
}

#[test]
fn grave_goods_is_personal_to_garrick_whose_dream_needs_a_triumph_there_and_to_no_one_else() {
    let (content, mut house) = house();
    let (garrick, brannoc) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Brannoc"));
    let quest = house.board[0].quest.clone();
    let facts = quest.facts();
    assert_eq!(
        personal(&content, &house.heroes, facts, &[garrick]),
        [garrick]
    );
    assert_eq!(
        personal(&content, &house.heroes, facts, &[garrick, brannoc]),
        [garrick]
    );
    assert_eq!(personal(&content, &house.heroes, facts, &[brannoc]), []);
    // A "go" call (Ysolde's road) makes nothing personal.
    let ysolde = id(&house.heroes, "Ysolde");
    assert_eq!(personal(&content, &house.heroes, facts, &[ysolde]), []);
    // Only family is marked: an outsider carrying Garrick's dream is not.
    crate::wanderer::arrive(&content, &mut house, &mut Rng::from_seed(3));
    let stranger = house.heroes.len() - 1;
    let mut dream = Dream::build(&content, DreamKind::QuietTheBarrow, None, None).expect("a dream");
    dream.advance_to_stage(2);
    house.heroes[stranger].dream = Some(dream);
    assert_eq!(personal(&content, &house.heroes, facts, &[stranger]), []);
}

#[test]
fn a_disaster_on_a_personal_quest_marks_the_name_costs_the_house_and_the_failer_and_tells_it() {
    let (content, mut house) = house();
    let garrick = id(&house.heroes, "Garrick");
    seat(&mut house, 0, &[garrick]);
    let renown = house.renown;
    let personal_renown = house.heroes[garrick].renown;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(4), 0, [1, 1]);
    assert_eq!(page.outcome, Outcome::Disaster);
    let danger = page.quest.danger;
    assert!(
        page.lines
            .contains(&"Garrick fails at the Barrow, and the name is marked: -2 renown to the house, -2 to Garrick.".to_owned()),
        "{:?}",
        page.lines
    );
    assert_eq!(house.heroes[garrick].marks, [mark(garrick, 1)]);
    assert_eq!(house.renown, renown - danger - 2);
    assert_eq!(house.heroes[garrick].renown, personal_renown - 2);
}

#[test]
fn a_disaster_on_a_quest_that_is_not_personal_marks_no_one() {
    let (content, mut house) = house();
    let brannoc = id(&house.heroes, "Brannoc");
    seat(&mut house, 0, &[brannoc]);
    let renown = house.renown;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(4), 0, [1, 1]);
    assert_eq!(page.outcome, Outcome::Disaster);
    assert!(house.heroes.iter().all(|h| h.marks.is_empty()));
    assert!(!page.lines.iter().any(|l| l.contains("the name is marked")));
    assert_eq!(house.renown, renown - page.quest.danger);
}

#[test]
fn a_win_on_a_personal_quest_marks_no_one() {
    let (content, mut house) = house();
    let (garrick, brannoc) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Brannoc"));
    seat(&mut house, 0, &[garrick, brannoc]);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(4), 0, [6, 6]);
    assert_eq!(page.outcome, Outcome::Triumph);
    assert!(house.heroes.iter().all(|h| h.marks.is_empty()));
}

/// A house in `year`'s turning.
fn in_year(house: &mut House, year: i32) {
    house.calendar.year_index = year - 1;
}

#[test]
fn a_mark_weighs_one_renown_at_each_turning_through_year_eight_and_is_forgotten_at_the_ninth() {
    let (content, mut house) = house();
    let (maren, pip) = (id(&house.heroes, "Maren"), id(&house.heroes, "Pip"));
    house.heroes[maren].marks.push(mark(maren, 1));
    for year in 1..=8 {
        in_year(&mut house, year);
        let renown = house.renown;
        let lines = weigh(&content, &mut house);
        assert_eq!(
            lines,
            ["The name carries a mark: -1 renown."],
            "year {year}"
        );
        assert_eq!(house.renown, renown - 1, "year {year}");
    }
    in_year(&mut house, 9);
    let renown = house.renown;
    let lines = weigh(&content, &mut house);
    assert_eq!(
        lines,
        ["Maren's failure at the Barrow is forgotten at last."]
    );
    assert_eq!(house.renown, renown);
    assert!(house.heroes[maren].marks.is_empty());
    in_year(&mut house, 10);
    assert_eq!(weigh(&content, &mut house), Vec::<String>::new());
    // The same failure on a second hero is one mark; a different one makes two.
    in_year(&mut house, 3);
    house.heroes[maren].marks.push(mark(maren, 1));
    house.heroes[pip].marks.push(mark(maren, 1));
    let renown = house.renown;
    assert_eq!(
        weigh(&content, &mut house),
        ["The name carries a mark: -1 renown."]
    );
    house.heroes[pip].marks.push(mark(pip, 2));
    assert_eq!(
        weigh(&content, &mut house),
        ["The name carries 2 marks: -2 renown."]
    );
    assert_eq!(house.renown, renown - 3);
}

#[test]
fn a_mark_on_an_outsider_or_on_the_dead_weighs_nothing() {
    let (content, mut house) = house();
    let garrick = id(&house.heroes, "Garrick");
    crate::wanderer::arrive(&content, &mut house, &mut Rng::from_seed(3));
    let stranger = house.heroes.len() - 1;
    house.heroes[stranger].marks.push(mark(garrick, 1));
    let elsbeth = id(&house.heroes, "Elsbeth");
    house.heroes[elsbeth].marks.push(mark(garrick, 1));
    in_year(&mut house, 2);
    assert_eq!(weigh(&content, &mut house), Vec::<String>::new());
    assert_eq!(house.marks_carried(), []);
}
