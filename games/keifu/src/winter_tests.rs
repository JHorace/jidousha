//! The winter resolved (SPEC §11.3, §11.5): the order, the lines, the teacher's credit
//! and the rank-limited mentor bond, the garden, the table, the fire, and the winter's
//! dream moments live. Expectations are shipped literals from SPEC.md and the content.

use jidousha::prelude::Rng;

use crate::bonds::form;
use crate::content::Content;
use crate::dream::Dream;
use crate::hearth::Seat;
use crate::hero::{DeedKind, HeroId};
use crate::house::House;
use crate::ids::{BondKind, DreamKind, Pool};
use crate::testkit::{house, id};
use crate::winter::{plan, resolve_winter};

/// A house in year 1's winter, its hearth open, with `seats` filled from wherever they sat.
fn winter(seats: &[(Seat, &str)]) -> (Content, House) {
    let (content, mut house) = house();
    house.calendar.begin_winter();
    house.open_hearth();
    for (seat, name) in seats {
        let hero = id(&house.heroes, name);
        house.unseat(hero);
        house.hearth.put(*seat, Some(hero));
    }
    (content, house)
}

/// Let the winter pass, asserting the plan the seats previewed is the one carried out.
fn pass(content: &Content, house: &mut House) -> Vec<String> {
    let before = plan(content, house);
    let (lines, done) = resolve_winter(content, house, &mut Rng::from_seed(7));
    assert_eq!(done, before, "the winter did what the seats previewed");
    lines
}

fn who(house: &House, name: &str) -> HeroId {
    id(&house.heroes, name)
}

#[test]
fn odo_teaching_pip_on_a_bench_raises_his_spirit_takes_him_as_a_student_and_counts_a_winter_of_odos_dream()
 {
    let (content, mut house) =
        winter(&[(Seat::BenchChild(0), "Pip"), (Seat::BenchTeacher(0), "Odo")]);
    let lines = pass(&content, &mut house);
    assert_eq!(
        lines,
        [
            "Pip trained under Odo. Spirit rises to 3.",
            "Odo has taken Pip as a student.",
            "Odo comes one nearer his dream: teach the young two winters, 1 of 2.",
        ]
    );
    let (odo, pip) = (who(&house, "Odo"), who(&house, "Pip"));
    let heroes = &house.heroes;
    assert_eq!(heroes[pip].aptitudes, [1, 2, 3]);
    assert_eq!(
        heroes[pip].bond_to(odo).map(|b| b.kind),
        Some(BondKind::Mentor)
    );
    let odo_side = heroes[odo]
        .bond_to(pip)
        .map(|b| (b.kind, b.taught, b.since));
    assert_eq!(odo_side, Some((BondKind::Student, true, 1)));
    assert_eq!(heroes[odo].winters_taught, 1);
    let taught: Vec<_> = heroes[odo]
        .deeds
        .iter()
        .filter(|d| d.kind == DeedKind::Taught)
        .collect();
    assert_eq!(taught.len(), 1);
    assert_eq!(taught[0].telling, "first taught the young: Pip");
    assert_eq!(
        heroes[odo]
            .dream
            .as_ref()
            .map(|d| (d.current, d.stages[0].count)),
        Some((0, 1))
    );
}

#[test]
fn a_second_winter_on_the_bench_finishes_odos_first_stage_without_a_second_new_student() {
    let (content, mut house) =
        winter(&[(Seat::BenchChild(0), "Pip"), (Seat::BenchTeacher(0), "Odo")]);
    pass(&content, &mut house);
    let (odo, pip) = (who(&house, "Odo"), who(&house, "Pip"));
    house.open_hearth();
    for (seat, hero) in [(Seat::BenchChild(1), pip), (Seat::BenchTeacher(1), odo)] {
        house.unseat(hero);
        house.hearth.put(seat, Some(hero));
    }
    let lines = pass(&content, &mut house);
    assert_eq!(
        lines,
        [
            "Pip trained under Odo. Spirit rises to 4.",
            "Odo is a step nearer his dream. What is left: stand beside a student on a quest.",
        ]
    );
    assert_eq!(house.heroes[odo].winters_taught, 2);
    let deeds = house.heroes[odo]
        .deeds
        .iter()
        .filter(|d| d.kind == DeedKind::Taught)
        .count();
    assert_eq!(deeds, 1, "only the first winter taught is a deed");
}

#[test]
fn a_bench_teacher_is_credited_only_for_a_lesson_that_teaches() {
    let (content, mut house) = winter(&[
        (Seat::BenchChild(0), "Wren"),
        (Seat::BenchTeacher(0), "Odo"),
    ]);
    let wren = who(&house, "Wren");
    house.heroes[wren].age = 5;
    let lines = pass(&content, &mut house);
    assert_eq!(
        lines,
        ["Wren is too young for the benches yet, and spent the winter underfoot."]
    );
    let odo = who(&house, "Odo");
    assert!(house.heroes[odo].bond_to(wren).is_none());
    assert_eq!(house.heroes[odo].winters_taught, 0);
}

#[test]
fn a_yard_teacher_is_credited_even_for_a_wasted_lesson_and_a_child_teacher_never() {
    // OQ-21 [emergent]: Pip belongs on a bench, and Odo is credited all the same.
    let (content, mut house) = winter(&[(Seat::Learner, "Pip"), (Seat::Teacher, "Odo")]);
    let lines = pass(&content, &mut house);
    assert_eq!(
        lines,
        [
            "Pip belongs on a bench, not in the training yard.",
            "Odo has taken Pip as a student.",
            "Odo comes one nearer his dream: teach the young two winters, 1 of 2.",
        ]
    );
    // A child in the teacher's seat: wasted, and no credit.
    let (content, mut house) = winter(&[(Seat::Learner, "Odo"), (Seat::Teacher, "Wren")]);
    let lines = pass(&content, &mut house);
    assert_eq!(
        lines,
        ["Odo spent the winter beside Wren, who is too young to teach anyone anything."]
    );
    let (odo, wren) = (who(&house, "Odo"), who(&house, "Wren"));
    assert!(house.heroes[wren].bond_to(odo).is_none());
    assert_eq!(house.heroes[wren].winters_taught, 0);
}

#[test]
fn the_mentor_bond_replaces_a_rivalry_or_a_friendship_but_never_a_marriage_or_a_parent() {
    // Brannoc teaches his rival Ysolde: the rivalry becomes mentorship, and no "new student".
    let (content, mut house) = winter(&[(Seat::Learner, "Ysolde"), (Seat::Teacher, "Brannoc")]);
    let lines = pass(&content, &mut house);
    assert_eq!(lines, ["Ysolde trained under Brannoc. Might rises to 3."]);
    let (ysolde, brannoc) = (who(&house, "Ysolde"), who(&house, "Brannoc"));
    assert_eq!(
        house.heroes[ysolde].bond_to(brannoc).map(|b| b.kind),
        Some(BondKind::Mentor)
    );
    assert_eq!(
        house.heroes[brannoc]
            .bond_to(ysolde)
            .map(|b| (b.kind, b.taught)),
        Some((BondKind::Student, true))
    );
    // Garrick teaches his friend Odo: friend (1) becomes mentor (2).
    let (content, mut house) = winter(&[(Seat::Learner, "Odo"), (Seat::Teacher, "Garrick")]);
    pass(&content, &mut house);
    let (odo, garrick) = (who(&house, "Odo"), who(&house, "Garrick"));
    assert_eq!(
        house.heroes[odo].aptitudes,
        [5, 3, 6],
        "an Elder teaches 2 Might"
    );
    assert_eq!(
        house.heroes[odo].bond_to(garrick).map(|b| b.kind),
        Some(BondKind::Mentor)
    );
    // Maren teaches her son: he stays her child, and she has taught him.
    let (content, mut house) = winter(&[
        (Seat::BenchChild(0), "Pip"),
        (Seat::BenchTeacher(0), "Maren"),
    ]);
    let lines = pass(&content, &mut house);
    assert_eq!(lines, ["Pip trained under Maren. Wits rises to 3."]);
    let (maren, pip) = (who(&house, "Maren"), who(&house, "Pip"));
    assert_eq!(
        house.heroes[pip].bond_to(maren).map(|b| b.kind),
        Some(BondKind::Parent)
    );
    assert_eq!(
        house.heroes[maren].bond_to(pip).map(|b| (b.kind, b.taught)),
        Some((BondKind::Child, true))
    );
    // Odo teaches his wife Maren: still spouses, and taught.
    let (content, mut house) = winter(&[(Seat::Learner, "Maren"), (Seat::Teacher, "Odo")]);
    let (maren, odo) = (who(&house, "Maren"), who(&house, "Odo"));
    form(&mut house.heroes, maren, odo, BondKind::Spouse, 1);
    pass(&content, &mut house);
    assert_eq!(
        house.heroes[odo].bond_to(maren).map(|b| (b.kind, b.taught)),
        Some((BondKind::Spouse, true))
    );
}

#[test]
fn an_adult_learner_witnesses_a_train_moment_even_when_nothing_is_learned() {
    // OQ-20 [emergent]: Brannoc at Might 6 alone learns nothing, and his blade's
    // "Train a winter at the forge" is done all the same.
    let (content, mut house) = winter(&[(Seat::Learner, "Brannoc")]);
    let brannoc = who(&house, "Brannoc");
    if let Some(dream) = house.heroes[brannoc].dream.as_mut() {
        dream.advance_to_stage(1);
    }
    let lines = pass(&content, &mut house);
    assert_eq!(
        lines,
        [
            "Brannoc trained alone all winter and got no better. There is nothing left to find alone.",
            "Brannoc is a step nearer his dream. What is left: carry it to a triumph.",
        ]
    );
}

#[test]
fn a_teacher_or_a_minder_alone_waits_and_says_so() {
    let (content, mut house) = winter(&[(Seat::Teacher, "Odo"), (Seat::BenchTeacher(1), "Maren")]);
    let lines = pass(&content, &mut house);
    assert_eq!(
        lines,
        [
            "Odo waited in the yard for a learner who did not come.",
            "Maren sat on the bench in the yard. No child came.",
        ]
    );
}

#[test]
fn the_winter_resolves_fire_yard_benches_garden_and_table_in_that_order() {
    let (content, mut house) = winter(&[
        (Seat::Table(0), "Maren"),
        (Seat::Garden(0), "Brannoc"),
        (Seat::BenchTeacher(0), "Odo"),
        (Seat::Teacher, "Garrick"),
        (Seat::Fire(0), "Ysolde"),
    ]);
    let ysolde = who(&house, "Ysolde");
    house.heroes[ysolde].fear.dread = 2;
    let lines = pass(&content, &mut house);
    let firsts: Vec<&str> = lines
        .iter()
        .map(|l| l.split(' ').next().unwrap_or(""))
        .collect();
    assert_eq!(lines.len(), 5, "{lines:?}");
    assert!(lines[0].ends_with(" Dread eased to 1."), "{lines:?}");
    assert_eq!(firsts[1..4], ["Garrick", "Odo", "Brannoc"]);
    assert!(
        lines[4].ends_with("+1 renown to the house, and to Maren."),
        "{lines:?}"
    );
}

#[test]
fn the_fire_heals_and_calms_with_a_rest_line_and_writes_nothing_when_neither_applies() {
    let (content, mut house) = winter(&[(Seat::Fire(0), "Maren"), (Seat::Fire(1), "Odo")]);
    let maren = who(&house, "Maren");
    house.heroes[maren].wounded = true;
    let lines = pass(&content, &mut house);
    let rests = [
        "Maren rested by the fire.",
        "Maren slept late and mended.",
        "Maren kept to the hearth all winter.",
    ];
    assert_eq!(
        lines.len(),
        1,
        "Odo, unwounded and calm, rests without a line: {lines:?}"
    );
    assert!(
        rests
            .iter()
            .any(|r| lines[0] == format!("{r} The wound closed. Dread eased to 0.")),
        "{lines:?}"
    );
    assert!(!house.heroes[maren].wounded);
    assert_eq!(house.heroes[maren].fear.dread, 0);
    // Healed only, and calmed only.
    let (content, mut house) = winter(&[(Seat::Fire(0), "Odo"), (Seat::Fire(1), "Garrick")]);
    let odo = who(&house, "Odo");
    house.heroes[odo].wounded = true;
    let lines = pass(&content, &mut house);
    assert_eq!(lines.len(), 2);
    assert!(
        lines[0].starts_with("Odo ") && lines[0].ends_with(". The wound closed."),
        "{lines:?}"
    );
    assert!(
        lines[1].starts_with("Garrick ") && lines[1].ends_with(" Dread eased to 1."),
        "{lines:?}"
    );
    // Nobody resting draws nothing from the pool.
    let (content, mut house) = winter(&[(Seat::Fire(0), "Odo")]);
    pass(&content, &mut house);
    assert_eq!(house.writing.last(Pool::Rests), None);
}

#[test]
fn two_who_may_wed_are_wed_with_a_wedding_line_and_a_deed_each() {
    let (content, mut house) = winter(&[(Seat::Garden(0), "Maren"), (Seat::Garden(1), "Odo")]);
    let lines = pass(&content, &mut house);
    let weddings = [
        "Maren and Odo were wed at midwinter. The whole house stood witness.",
        "Maren and Odo were wed by the hearth, with snow at the windows.",
        "Maren and Odo walked in the garden until it was decided. They are wed.",
    ];
    assert_eq!(lines.len(), 1);
    assert!(weddings.contains(&lines[0].as_str()), "{lines:?}");
    let (maren, odo) = (who(&house, "Maren"), who(&house, "Odo"));
    for (me, other, full) in [
        (maren, odo, "wed Odo Fenn"),
        (odo, maren, "wed Maren Thorne"),
    ] {
        let bond = house.heroes[me].bond_to(other).map(|b| (b.kind, b.since));
        assert_eq!(bond, Some((BondKind::Spouse, 1)));
        let wed: Vec<_> = house.heroes[me]
            .deeds
            .iter()
            .filter(|d| d.kind == DeedKind::Wed)
            .collect();
        assert_eq!(wed.len(), 1);
        assert_eq!((wed[0].telling.as_str(), wed[0].other), (full, Some(other)));
    }
}

#[test]
fn rivals_in_the_garden_make_peace_and_any_other_courting_comes_to_nothing() {
    let (content, mut house) = winter(&[(Seat::Garden(0), "Ysolde"), (Seat::Garden(1), "Brannoc")]);
    let lines = pass(&content, &mut house);
    assert_eq!(
        lines,
        ["Ysolde and Brannoc walked in the garden all winter, arguing. They came in friends."]
    );
    let (ysolde, brannoc) = (who(&house, "Ysolde"), who(&house, "Brannoc"));
    assert_eq!(
        house.heroes[brannoc].bond_to(ysolde).map(|b| b.kind),
        Some(BondKind::Friend)
    );
    assert_eq!(
        house.heroes[ysolde].bond_to(brannoc).map(|b| b.kind),
        Some(BondKind::Friend)
    );
    // Alone in the second seat: about the first occupied seat.
    let (content, mut house) = winter(&[(Seat::Garden(1), "Odo")]);
    assert_eq!(
        pass(&content, &mut house),
        ["Odo walked in the garden. Nothing came of it."]
    );
    // Kin.
    let (content, mut house) = winter(&[(Seat::Garden(0), "Garrick"), (Seat::Garden(1), "Maren")]);
    assert_eq!(
        pass(&content, &mut house),
        ["Garrick walked in the garden. Nothing came of it."]
    );
}

#[test]
fn a_wedding_in_the_garden_moves_a_dream_of_being_wed_at_the_court_moment() {
    let (content, mut house) = winter(&[(Seat::Garden(0), "Brannoc"), (Seat::Garden(1), "Maren")]);
    let maren = who(&house, "Maren");
    let dream = Dream::build(&content, DreamKind::SeeAChildGrown, None, None).expect("builds");
    house.heroes[maren].dream = Some(dream);
    let lines = pass(&content, &mut house);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(
        lines[1],
        "Maren is a step nearer her dream. What is left: have a child."
    );
}

#[test]
fn the_table_gives_the_house_one_renown_once_and_each_adult_teller_one_and_ignores_a_child() {
    let (content, mut house) = winter(&[(Seat::Table(0), "Odo"), (Seat::Table(1), "Maren")]);
    let renown = house.renown;
    let lines = pass(&content, &mut house);
    let told = [
        "Odo told the tale at the long table. People will repeat it.",
        "Odo told it plainly, and the hall was silent to the end.",
        "Odo told the tale three nights running, and it grew a little each night.",
    ];
    assert_eq!(lines.len(), 2);
    assert!(
        told.iter()
            .any(|t| lines[0] == format!("{t} +1 renown to the house, and to Odo.")),
        "{lines:?}"
    );
    assert_eq!(
        lines[1],
        "Maren told it again after, in her own way. +1 renown to Maren. The house has heard it once already."
    );
    assert_eq!(house.renown, renown + 1);
    let (odo, maren) = (who(&house, "Odo"), who(&house, "Maren"));
    assert_eq!(
        (house.heroes[odo].renown, house.heroes[maren].renown),
        (4, 5)
    );
    for hero in [odo, maren] {
        assert!(
            house.heroes[hero]
                .deeds
                .iter()
                .any(|d| d.kind == DeedKind::ToldTheTale)
        );
    }
    // A child at the table first: ignored, and the adult after tells it for the house.
    let (content, mut house) = winter(&[(Seat::Table(0), "Pip"), (Seat::Table(1), "Odo")]);
    let lines = pass(&content, &mut house);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].ends_with("+1 renown to the house, and to Odo."));
    assert_eq!(house.heroes[who(&house, "Pip")].renown, 0);
}

#[test]
fn ysolde_telling_the_tale_at_the_end_of_every_road_forges_her_road_book() {
    let (content, mut house) = winter(&[(Seat::Table(0), "Ysolde")]);
    let ysolde = who(&house, "Ysolde");
    if let Some(dream) = house.heroes[ysolde].dream.as_mut() {
        dream.advance_to_stage(2);
    }
    let lines = pass(&content, &mut house);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(
        lines[1..],
        [
            "Ysolde has done it: to walk every road. She is settled now, and dread has no hold on her.",
            "It leaves an heirloom: Ysolde's road-book. +2 Wits on quests. Every road there is, written down by Ysolde Vane and finished in year 1.",
        ]
    );
    let heirloom = house.heroes[ysolde]
        .heirloom
        .as_ref()
        .map(|h| (h.name.as_str(), h.sprite.as_str()));
    assert_eq!(heirloom, Some(("Ysolde's road-book", "reward-guidebook")));
    assert!(house.heroes[ysolde].settled);
}

#[test]
fn a_teller_whose_renown_reaches_four_is_known_at_court_at_the_tale_moment() {
    // KNOWN_AT_COURT's "Earn 4 renown" holds at any moment; the tale's +1 comes first.
    let (content, mut house) = winter(&[(Seat::Table(0), "Odo")]);
    let odo = who(&house, "Odo");
    let dream = Dream::build(&content, DreamKind::KnownAtCourt, None, None).expect("builds");
    house.heroes[odo].dream = Some(dream);
    let lines = pass(&content, &mut house);
    assert_eq!(
        lines[1],
        "Odo is a step nearer his dream. What is left: succeed at the King's Court."
    );
}
