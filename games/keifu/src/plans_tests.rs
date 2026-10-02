//! The winter's plans (SPEC §11.4, §11.6, §11.3's fire and table), rule by rule: every
//! excuse in its order, every amount and cap, every courtship verdict, the rest and the
//! tellers. Expectations are shipped literals from SPEC.md and CONSTANTS.md §9.

use crate::bonds::form;
use crate::content::Content;
use crate::hero::{Hero, HeroId};
use crate::ids::{Aptitude, BondKind, Destiny};
use crate::plans::{
    Courtship, Excuse, Lesson, Rest, Teller, bench_lesson, courtship, rest, tellers, yard_lesson,
};
use crate::testkit::{founded, id};

fn learn(aptitude: Aptitude, amount: i32) -> Lesson {
    Lesson::Learn { aptitude, amount }
}

fn wasted(excuse: Excuse) -> Lesson {
    Lesson::Wasted(excuse)
}

/// The founding heroes, and the ids of the names asked for.
fn cast<const N: usize>(names: [&str; N]) -> (Content, Vec<Hero>, [HeroId; N]) {
    let (content, heroes) = founded();
    let ids = names.map(|name| id(&heroes, name));
    (content, heroes, ids)
}

#[test]
fn odo_teaching_pip_on_a_bench_plans_one_spirit_and_the_preview_reads_plus_one_spirit() {
    let (content, heroes, [odo, pip]) = cast(["Odo", "Pip"]);
    let lesson = bench_lesson(&content, &heroes, pip, Some(odo));
    assert_eq!(lesson, learn(Aptitude::Spirit, 1));
    assert_eq!(lesson.note(&content), "+1 Spirit");
}

#[test]
fn a_child_in_the_training_yard_belongs_on_a_bench_whoever_teaches() {
    let (content, heroes, [odo, pip]) = cast(["Odo", "Pip"]);
    assert_eq!(
        yard_lesson(&content, &heroes, pip, Some(odo)),
        wasted(Excuse::BelongsOnBench)
    );
    assert_eq!(
        yard_lesson(&content, &heroes, pip, None),
        wasted(Excuse::BelongsOnBench)
    );
    assert_eq!(
        wasted(Excuse::BelongsOnBench).note(&content),
        "belongs on a bench"
    );
}

#[test]
fn an_adult_in_a_bench_child_seat_is_no_child_and_a_child_alone_needs_a_teacher() {
    let (content, heroes, [maren, odo, pip]) = cast(["Maren", "Odo", "Pip"]);
    assert_eq!(
        bench_lesson(&content, &heroes, maren, Some(odo)),
        wasted(Excuse::IsNoChild)
    );
    assert_eq!(
        bench_lesson(&content, &heroes, pip, None),
        wasted(Excuse::BenchNeedsTeacher)
    );
    assert_eq!(wasted(Excuse::IsNoChild).note(&content), "is no child");
    assert_eq!(
        wasted(Excuse::BenchNeedsTeacher).note(&content),
        "needs a teacher"
    );
}

#[test]
fn a_child_cannot_teach_and_that_is_judged_before_whether_the_learner_may_learn() {
    let (content, heroes, [garrick, maren, pip, wren]) = cast(["Garrick", "Maren", "Pip", "Wren"]);
    assert_eq!(
        yard_lesson(&content, &heroes, maren, Some(pip)),
        wasted(Excuse::ChildTeacher)
    );
    assert_eq!(
        bench_lesson(&content, &heroes, wren, Some(pip)),
        wasted(Excuse::ChildTeacher)
    );
    // Garrick may not learn (his firstborn is grown), and a child teacher is said first.
    assert_eq!(
        yard_lesson(&content, &heroes, garrick, Some(pip)),
        wasted(Excuse::ChildTeacher)
    );
    assert_eq!(
        wasted(Excuse::ChildTeacher).note(&content),
        "a child cannot teach"
    );
}

#[test]
fn a_hero_who_may_not_learn_learns_nothing_alone_or_taught() {
    let (content, mut heroes, [garrick, maren, odo]) = cast(["Garrick", "Maren", "Odo"]);
    // CHILD_WILL_SURPASS_YOU with a grown firstborn (Maren).
    assert_eq!(
        yard_lesson(&content, &heroes, garrick, Some(odo)),
        wasted(Excuse::LearnsNothing)
    );
    assert_eq!(
        yard_lesson(&content, &heroes, garrick, None),
        wasted(Excuse::LearnsNothing)
    );
    // TEACH_A_GREATER may not learn.
    heroes[maren].destiny.kind = Destiny::TeachAGreater;
    assert_eq!(
        yard_lesson(&content, &heroes, maren, Some(odo)),
        wasted(Excuse::LearnsNothing)
    );
    assert_eq!(
        wasted(Excuse::LearnsNothing).note(&content),
        "learns nothing more"
    );
}

#[test]
fn alone_an_adult_trains_their_calling_by_one_or_two_if_young_and_stops_at_six() {
    let (content, mut heroes, [ysolde, brannoc]) = cast(["Ysolde", "Brannoc"]);
    // Ysolde, a Scholar: Wits 5, prime — +1, and no further than 6.
    assert_eq!(
        yard_lesson(&content, &heroes, ysolde, None),
        learn(Aptitude::Wits, 1)
    );
    // A Youth at Wits 3: +2; at Wits 5: +1, capped by 6 - 5.
    heroes[ysolde].age = 15;
    heroes[ysolde].aptitudes = [2, 3, 4];
    assert_eq!(
        yard_lesson(&content, &heroes, ysolde, None),
        learn(Aptitude::Wits, 2)
    );
    heroes[ysolde].aptitudes = [2, 5, 4];
    assert_eq!(
        yard_lesson(&content, &heroes, ysolde, None),
        learn(Aptitude::Wits, 1)
    );
    // Brannoc, a Warrior, already has Might 6: nothing left to find alone.
    assert_eq!(
        yard_lesson(&content, &heroes, brannoc, None),
        wasted(Excuse::NeedsTeacherNow)
    );
    assert_eq!(
        wasted(Excuse::NeedsTeacherNow).note(&content),
        "needs a teacher now"
    );
    // The aptitude is the calling's, not the best one: a Priest at Spirit 1 trains Spirit.
    heroes[brannoc].vocation = crate::ids::Vocation::Priest;
    heroes[brannoc].aptitudes = [6, 2, 1];
    assert_eq!(
        yard_lesson(&content, &heroes, brannoc, None),
        learn(Aptitude::Spirit, 1)
    );
}

#[test]
fn a_teacher_passes_on_their_best_aptitude_and_only_if_they_know_more() {
    let (content, mut heroes, [maren, ysolde, odo]) = cast(["Maren", "Ysolde", "Odo"]);
    // Maren's best is Wits 6; Ysolde has 5; both prime: +1, and at most 6 - 5.
    assert_eq!(
        yard_lesson(&content, &heroes, ysolde, Some(maren)),
        learn(Aptitude::Wits, 1)
    );
    // Ysolde's best is Wits 5; Maren has 6: the teacher knows no more.
    assert_eq!(
        yard_lesson(&content, &heroes, maren, Some(ysolde)),
        wasted(Excuse::TeacherKnowsNoMore)
    );
    // Equal is no more either: Odo's Spirit 6 to a learner with Spirit 6.
    heroes[maren].aptitudes = [4, 6, 6];
    assert_eq!(
        yard_lesson(&content, &heroes, maren, Some(odo)),
        wasted(Excuse::TeacherKnowsNoMore)
    );
    assert_eq!(
        wasted(Excuse::TeacherKnowsNoMore).note(&content),
        "teacher knows no more"
    );
}

#[test]
fn veterans_and_elders_teach_well_the_young_learn_fast_and_two_is_the_cap() {
    let (content, mut heroes, [garrick, maren, brannoc, odo, ysolde]) =
        cast(["Garrick", "Maren", "Brannoc", "Odo", "Ysolde"]);
    // Odo, a Veteran, to Brannoc (Spirit 3): 1 + 1 = 2.
    assert_eq!(
        yard_lesson(&content, &heroes, brannoc, Some(odo)),
        learn(Aptitude::Spirit, 2)
    );
    // Maren, prime, to Brannoc: Wits 2 under 6 — 1.
    assert_eq!(
        yard_lesson(&content, &heroes, brannoc, Some(maren)),
        learn(Aptitude::Wits, 1)
    );
    // A Youth under a prime teacher: 1 + 1 = 2.
    heroes[ysolde].age = 16;
    assert_eq!(
        yard_lesson(&content, &heroes, ysolde, Some(brannoc)),
        learn(Aptitude::Might, 2)
    );
    // A Youth under a Veteran: 1 + 1 + 1, capped at 2.
    assert_eq!(
        yard_lesson(&content, &heroes, ysolde, Some(odo)),
        learn(Aptitude::Spirit, 2)
    );
    // Garrick, an Elder with Might 7, to Brannoc's Might 6: at most taught - known = 1.
    assert_eq!(
        yard_lesson(&content, &heroes, brannoc, Some(garrick)),
        learn(Aptitude::Might, 1)
    );
}

#[test]
fn a_teach_a_greater_teacher_counts_as_knowing_nine_and_adds_one_for_adults() {
    let (content, mut heroes, [maren, ysolde, brannoc, pip]) =
        cast(["Maren", "Ysolde", "Brannoc", "Pip"]);
    heroes[maren].destiny.kind = Destiny::TeachAGreater;
    // Prime to prime: 1 + 1 = 2 Wits, though Maren knows only 6 and Ysolde 5.
    assert_eq!(
        yard_lesson(&content, &heroes, ysolde, Some(maren)),
        learn(Aptitude::Wits, 2)
    );
    // Taught as 9 even past what the teacher knows: Ysolde at Wits 6, 7, 8.
    heroes[ysolde].aptitudes = [2, 7, 4];
    assert_eq!(
        yard_lesson(&content, &heroes, ysolde, Some(maren)),
        learn(Aptitude::Wits, 2)
    );
    heroes[ysolde].aptitudes = [2, 8, 4];
    assert_eq!(
        yard_lesson(&content, &heroes, ysolde, Some(maren)),
        learn(Aptitude::Wits, 1)
    );
    heroes[ysolde].aptitudes = [2, 9, 4];
    assert_eq!(
        yard_lesson(&content, &heroes, ysolde, Some(maren)),
        wasted(Excuse::TeacherKnowsNoMore)
    );
    // A Veteran greater teacher to a Youth: the cap of 2, then +1 — three.
    heroes[maren].age = 45;
    heroes[brannoc].age = 15;
    assert_eq!(
        yard_lesson(&content, &heroes, brannoc, Some(maren)),
        learn(Aptitude::Wits, 3)
    );
    // A child learns 1 from a greater teacher: no bonus on a bench.
    assert_eq!(
        bench_lesson(&content, &heroes, pip, Some(maren)),
        learn(Aptitude::Wits, 1)
    );
}

#[test]
fn a_child_learns_one_from_six_until_four_and_nothing_younger_or_beyond() {
    let (content, mut heroes, [odo, pip, wren]) = cast(["Odo", "Pip", "Wren"]);
    // Wren, 8, Spirit 2: one.
    assert_eq!(
        bench_lesson(&content, &heroes, wren, Some(odo)),
        learn(Aptitude::Spirit, 1)
    );
    heroes[wren].age = 6;
    assert_eq!(
        bench_lesson(&content, &heroes, wren, Some(odo)),
        learn(Aptitude::Spirit, 1)
    );
    heroes[wren].age = 5;
    assert_eq!(
        bench_lesson(&content, &heroes, wren, Some(odo)),
        wasted(Excuse::TooYoung)
    );
    // Pip at Spirit 3 learns; at 4 he has learned all a child can.
    heroes[pip].aptitudes = [1, 2, 3];
    assert_eq!(
        bench_lesson(&content, &heroes, pip, Some(odo)),
        learn(Aptitude::Spirit, 1)
    );
    heroes[pip].aptitudes = [1, 2, 4];
    assert_eq!(
        bench_lesson(&content, &heroes, pip, Some(odo)),
        wasted(Excuse::ChildLimit)
    );
    // A child taught by a veteran gets no more than one.
    heroes[pip].aptitudes = [1, 2, 1];
    assert_eq!(
        bench_lesson(&content, &heroes, pip, Some(odo)),
        learn(Aptitude::Spirit, 1)
    );
    // "Teacher knows no more" is judged before the child's own limits.
    heroes[wren].aptitudes = [2, 1, 6];
    assert_eq!(
        bench_lesson(&content, &heroes, wren, Some(odo)),
        wasted(Excuse::TeacherKnowsNoMore)
    );
    assert_eq!(
        wasted(Excuse::TooYoung).note(&content),
        "too young to learn"
    );
    assert_eq!(wasted(Excuse::ChildLimit).note(&content), "is a child yet");
}

#[test]
fn every_excuse_tells_its_wasted_line_with_the_learner_and_the_teacher_it_names() {
    let (content, _, _) = cast([]);
    let lines: Vec<String> = [
        Excuse::ChildTeacher,
        Excuse::LearnsNothing,
        Excuse::NeedsTeacher,
        Excuse::NeedsTeacherNow,
        Excuse::TeacherKnowsNoMore,
        Excuse::TooYoung,
        Excuse::ChildLimit,
        Excuse::BelongsOnBench,
        Excuse::IsNoChild,
        Excuse::BenchNeedsTeacher,
    ]
    .iter()
    .map(|excuse| excuse.wasted(&content, "Pip", Some("Wren")))
    .collect();
    assert_eq!(
        lines,
        [
            "Pip spent the winter beside Wren, who is too young to teach anyone anything.",
            "Pip trained all winter and learned nothing. The Seer said as much.",
            "Pip sat on the bench alone all winter. A child needs someone to learn from.",
            "Pip trained alone all winter and got no better. There is nothing left to find alone.",
            "Pip trained under Wren all winter, but Wren had nothing left to teach.",
            "Pip is too young for the benches yet, and spent the winter underfoot.",
            "Pip has learned all a child can. The rest waits for coming of age.",
            "Pip belongs on a bench, not in the training yard.",
            "Pip is grown, and the benches are for children.",
            "Pip sat on the bench alone all winter. A child needs someone to learn from.",
        ]
    );
}

#[test]
fn the_garden_judges_empty_seats_kin_age_distance_rivalry_and_marriage_in_that_order() {
    let (content, mut heroes, [garrick, maren, pip, ysolde, brannoc, odo]) =
        cast(["Garrick", "Maren", "Pip", "Ysolde", "Brannoc", "Odo"]);
    assert_eq!(courtship(&heroes, None, None), Courtship::Nobody);
    assert_eq!(courtship(&heroes, Some(odo), None), Courtship::Waiting);
    assert_eq!(courtship(&heroes, None, Some(odo)), Courtship::Waiting);
    // Maren and Pip are kin, and he is ten: kin is said first.
    assert_eq!(courtship(&heroes, Some(maren), Some(pip)), Courtship::Kin);
    assert_eq!(
        courtship(&heroes, Some(garrick), Some(maren)),
        Courtship::Kin
    );
    // Garrick (62) and Ysolde (24): too far apart. Exactly fifteen is not.
    assert_eq!(
        courtship(&heroes, Some(garrick), Some(ysolde)),
        Courtship::TooFarApart
    );
    // The younger first is as far apart.
    assert_eq!(
        courtship(&heroes, Some(ysolde), Some(garrick)),
        Courtship::TooFarApart
    );
    heroes[ysolde].age = 47;
    heroes[garrick].age = 62;
    assert_eq!(
        courtship(&heroes, Some(garrick), Some(ysolde)),
        Courtship::WillWed
    );
    heroes[ysolde].age = 24;
    // Under eighteen is too young, and that is said before the distance.
    heroes[pip].age = 17;
    assert_eq!(
        courtship(&heroes, Some(pip), Some(odo)),
        Courtship::TooYoung
    );
    // Either seat: the young one second is too young as well.
    assert_eq!(
        courtship(&heroes, Some(odo), Some(pip)),
        Courtship::TooYoung
    );
    heroes[pip].age = 18;
    heroes[pip].parents = [None, None];
    heroes[pip].bonds.clear();
    heroes[maren].bonds.retain(|b| b.other != pip);
    assert_eq!(
        courtship(&heroes, Some(pip), Some(ysolde)),
        Courtship::WillWed
    );
    // Brannoc and Ysolde are rivals: they will make peace.
    assert_eq!(
        courtship(&heroes, Some(brannoc), Some(ysolde)),
        Courtship::Rivals
    );
    // Brannoc is a widower: he may wed again.
    assert_eq!(
        courtship(&heroes, Some(brannoc), Some(maren)),
        Courtship::WillWed
    );
    // One with a living spouse is wed already.
    form(&mut heroes, odo, ysolde, BondKind::Spouse, 1);
    assert_eq!(
        courtship(&heroes, Some(maren), Some(odo)),
        Courtship::WedAlready
    );
    let notes: Vec<String> = [
        Courtship::Nobody,
        Courtship::Waiting,
        Courtship::Kin,
        Courtship::TooYoung,
        Courtship::TooFarApart,
        Courtship::Rivals,
        Courtship::WedAlready,
        Courtship::WillWed,
    ]
    .iter()
    .map(|c| c.note(&content))
    .collect();
    assert_eq!(
        notes,
        [
            "",
            "waits alone",
            "they are kin",
            "under 18",
            "too far apart",
            "will make peace",
            "one is wed",
            "will wed"
        ]
    );
}

#[test]
fn the_fire_heals_a_wound_and_calms_dread_unless_broken_and_says_heals_first() {
    let (content, mut heroes, [garrick, maren, odo]) = cast(["Garrick", "Maren", "Odo"]);
    // Garrick: dread 2, unwounded — calms.
    assert_eq!(
        rest(&heroes[garrick]),
        Rest {
            heals: false,
            calms: true
        }
    );
    assert_eq!(rest(&heroes[garrick]).note(&content), "calms");
    // Odo: no dread, no wound — rests.
    assert_eq!(rest(&heroes[odo]).note(&content), "rests");
    // Maren wounded with dread 1: both, and "heals" is said.
    heroes[maren].wounded = true;
    assert_eq!(
        rest(&heroes[maren]),
        Rest {
            heals: true,
            calms: true
        }
    );
    assert_eq!(rest(&heroes[maren]).note(&content), "heals");
    // A broken hero sheds nothing.
    heroes[garrick].fear.broken = true;
    assert!(!rest(&heroes[garrick]).calms);
}

#[test]
fn the_first_adult_at_the_table_tells_it_for_the_house_a_second_for_themselves_and_a_child_not_at_all()
 {
    let (content, heroes, [odo, pip, maren]) = cast(["Odo", "Pip", "Maren"]);
    assert_eq!(
        tellers(&heroes, [Some(odo), Some(maren)]),
        [Some(Teller::House), Some(Teller::Own)]
    );
    assert_eq!(
        tellers(&heroes, [Some(pip), Some(maren)]),
        [Some(Teller::Child), Some(Teller::House)]
    );
    assert_eq!(
        tellers(&heroes, [None, Some(odo)]),
        [None, Some(Teller::House)]
    );
    let notes: Vec<&str> = [Teller::House, Teller::Own, Teller::Child]
        .iter()
        .map(|t| t.note(&content))
        .collect();
    assert_eq!(notes, ["house +", "own +", "a child"]);
}
