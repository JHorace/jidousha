//! What each winter seat will do (SPEC §11.4, §11.6, §11.3's fire and table): the
//! lesson planned with every excuse, the courtship verdict, the rest, and the teller's
//! part.
//!
//! These are the one source for the winter: the hearth's seat previews (§11.2,
//! `preview.rs`) and the winter's resolution (`winter.rs`) both ask these functions,
//! with the same heroes in the same seats, so a seat cannot preview a lesson the
//! winter would not give. Nothing here changes the house.

use crate::constants::{
    APTITUDE_LIMIT, CHILD_TAUGHT_LIMIT, COURTING_AGE_GAP, GREATER_LESSON, GREATER_TAUGHT,
    MARRY_IN_RENOWN, MARRYING_AGE, SEASONED_TEACHER_BONUS, SELF_TAUGHT_LIMIT, TEACHABLE_AGE,
    WINTER_GAIN_LIMIT, WINTER_LESSON, YOUNG_LEARNER_BONUS,
};
use crate::content::Content;
use crate::destiny::may_still_learn;
use crate::hero::{Hero, HeroId, kin};
use crate::ids::{Aptitude, BondKind, Destiny, Phase};
use crate::outsiders::{is_family, may_marry_in};
use crate::words::W;

/// Why a lesson was wasted (SPEC §11.4, `ui.winter.lesson_excuses`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Excuse {
    /// The teacher is a child.
    ChildTeacher,
    /// The learner may not learn (§13).
    LearnsNothing,
    /// A child alone in the general plan (unreachable through the seats).
    NeedsTeacher,
    /// Alone, and already at the self-taught limit.
    NeedsTeacherNow,
    /// The teacher knows no more of their best aptitude than the learner.
    TeacherKnowsNoMore,
    /// A child under the teachable age.
    TooYoung,
    /// A child at the child's limit.
    ChildLimit,
    /// A child in the training yard.
    BelongsOnBench,
    /// An adult in a bench's child seat.
    IsNoChild,
    /// A bench's child with no teacher.
    BenchNeedsTeacher,
}

impl Excuse {
    /// The seat note: "teacher knows no more".
    pub fn note(self, content: &Content) -> &str {
        &content.words[self.words().0]
    }

    /// The winter page's line, about the learner and (for the two that name one) the
    /// teacher: "%1 trained under %2 all winter, but %2 had nothing left to teach."
    pub fn wasted(self, content: &Content, learner: &str, teacher: Option<&str>) -> String {
        let template = &content.words[self.words().1];
        match (self, teacher) {
            (Excuse::ChildTeacher | Excuse::TeacherKnowsNoMore, Some(teacher)) => {
                crate::text::fmt(template, &[learner, teacher])
            }
            (Excuse::ChildTeacher | Excuse::TeacherKnowsNoMore, None) => panic!(
                "[keifu] the excuse {self:?} names a teacher and none sat\n  likely cause: \
                 a lesson planned with a teacher was told without one\n  fix: pass the \
                 teacher the plan had"
            ),
            _ => crate::text::fmt(template, &[learner]),
        }
    }

    fn words(self) -> (W, W) {
        match self {
            Excuse::ChildTeacher => (W::ExcuseChildTeacher, W::ExcuseChildTeacherWasted),
            Excuse::LearnsNothing => (W::ExcuseLearnsNothing, W::ExcuseLearnsNothingWasted),
            Excuse::NeedsTeacher => (W::ExcuseNeedsTeacher, W::ExcuseNeedsTeacherWasted),
            Excuse::NeedsTeacherNow => (W::ExcuseNeedsTeacherNow, W::ExcuseNeedsTeacherNowWasted),
            Excuse::TeacherKnowsNoMore => (
                W::ExcuseTeacherKnowsNoMore,
                W::ExcuseTeacherKnowsNoMoreWasted,
            ),
            Excuse::TooYoung => (W::ExcuseTooYoung, W::ExcuseTooYoungWasted),
            Excuse::ChildLimit => (W::ExcuseChildLimit, W::ExcuseChildLimitWasted),
            Excuse::BelongsOnBench => (W::ExcuseBelongsOnBench, W::ExcuseBelongsOnBenchWasted),
            Excuse::IsNoChild => (W::ExcuseIsNoChild, W::ExcuseIsNoChildWasted),
            Excuse::BenchNeedsTeacher => {
                (W::ExcuseBenchNeedsTeacher, W::ExcuseBenchNeedsTeacherWasted)
            }
        }
    }
}

/// A planned lesson: what will be learned, or why nothing will.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lesson {
    /// `amount` (> 0) of `aptitude`.
    Learn {
        /// What is learned.
        aptitude: Aptitude,
        /// How much.
        amount: i32,
    },
    /// Nothing, and why.
    Wasted(Excuse),
}

impl Lesson {
    /// What the lesson adds: 0 when wasted.
    pub fn amount(self) -> i32 {
        match self {
            Lesson::Learn { amount, .. } => amount,
            Lesson::Wasted(_) => 0,
        }
    }

    /// The seat's preview: "+1 Spirit" (`lines.lesson.amount`), or the excuse.
    pub fn note(self, content: &Content) -> String {
        match self {
            Lesson::Learn { aptitude, amount } => crate::text::fmt(
                &content.words[W::LessonAmount],
                &[
                    &amount.to_string(),
                    &content.lore.aptitudes[aptitude.index()],
                ],
            ),
            Lesson::Wasted(excuse) => excuse.note(content).to_owned(),
        }
    }
}

/// The training yard's lesson (SPEC §11.4): a child learner belongs on a bench;
/// otherwise the general plan.
pub fn yard_lesson(
    content: &Content,
    heroes: &[Hero],
    learner: HeroId,
    teacher: Option<HeroId>,
) -> Lesson {
    if !heroes[learner].is_adult() {
        return Lesson::Wasted(Excuse::BelongsOnBench);
    }
    general(content, heroes, learner, teacher)
}

/// A bench's lesson (SPEC §11.4): an adult in the child seat is no child; a child with
/// no teacher needs one; otherwise the general plan.
pub fn bench_lesson(
    content: &Content,
    heroes: &[Hero],
    child: HeroId,
    teacher: Option<HeroId>,
) -> Lesson {
    if heroes[child].is_adult() {
        return Lesson::Wasted(Excuse::IsNoChild);
    }
    if teacher.is_none() {
        return Lesson::Wasted(Excuse::BenchNeedsTeacher);
    }
    general(content, heroes, child, teacher)
}

/// The general plan (SPEC §11.4), its checks in order.
fn general(content: &Content, heroes: &[Hero], learner: HeroId, teacher: Option<HeroId>) -> Lesson {
    let l = &heroes[learner];
    if teacher.is_some_and(|t| !heroes[t].is_adult()) {
        return Lesson::Wasted(Excuse::ChildTeacher);
    }
    if !may_still_learn(heroes, learner) {
        return Lesson::Wasted(Excuse::LearnsNothing);
    }
    let Some(teacher) = teacher else {
        if !l.is_adult() {
            return Lesson::Wasted(Excuse::NeedsTeacher);
        }
        let aptitude = content.lore.vocation_aptitudes[l.vocation.index()];
        let known = l.base(aptitude);
        if known >= SELF_TAUGHT_LIMIT {
            return Lesson::Wasted(Excuse::NeedsTeacherNow);
        }
        let amount = (WINTER_LESSON + youth(l)).min(SELF_TAUGHT_LIMIT - known);
        return Lesson::Learn { aptitude, amount };
    };
    let t = &heroes[teacher];
    let aptitude = t.best_aptitude();
    let known = l.base(aptitude);
    let greater = t.destiny.kind == Destiny::TeachAGreater;
    let taught = if greater {
        GREATER_TAUGHT
    } else {
        t.base(aptitude)
    };
    if taught <= known {
        return Lesson::Wasted(Excuse::TeacherKnowsNoMore);
    }
    if !l.is_adult() {
        if l.age < TEACHABLE_AGE {
            return Lesson::Wasted(Excuse::TooYoung);
        }
        if known >= CHILD_TAUGHT_LIMIT {
            return Lesson::Wasted(Excuse::ChildLimit);
        }
        return Lesson::Learn {
            aptitude,
            amount: WINTER_LESSON,
        };
    }
    let seasoned = if matches!(t.phase(), Phase::Veteran | Phase::Elder) {
        SEASONED_TEACHER_BONUS
    } else {
        0
    };
    let mut amount = (WINTER_LESSON + seasoned + youth(l)).min(WINTER_GAIN_LIMIT);
    if greater {
        amount += GREATER_LESSON;
    }
    let amount = amount.min(taught - known).min(APTITUDE_LIMIT - known);
    Lesson::Learn { aptitude, amount }
}

/// A Youth learns fast: +1 to an adult lesson.
fn youth(hero: &Hero) -> i32 {
    if hero.phase() == Phase::Youth {
        YOUNG_LEARNER_BONUS
    } else {
        0
    }
}

/// The garden's verdict (SPEC §11.6, `ui.winter.courtship_notes`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Courtship {
    /// Both seats empty.
    Nobody,
    /// One seat empty.
    Waiting,
    /// Kin cannot wed.
    Kin,
    /// Either is under the marrying age.
    TooYoung,
    /// More than the courting gap apart.
    TooFarApart,
    /// Rivals make peace instead.
    Rivals,
    /// Either has a living spouse.
    WedAlready,
    /// One is an outsider whose renown is under the threshold (variant).
    Unproven {
        /// The outsider.
        outsider: HeroId,
        /// Their personal renown.
        renown: i32,
    },
    /// They wed.
    WillWed,
}

impl Courtship {
    /// The garden's preview: "will wed", "under 18", ... (`NOBODY` is empty).
    pub fn note(self, content: &Content, heroes: &[Hero]) -> String {
        let words = &content.words;
        match self {
            Courtship::Nobody => words[W::CourtNobody].to_owned(),
            Courtship::Waiting => words[W::CourtWaiting].to_owned(),
            Courtship::Kin => words[W::CourtKin].to_owned(),
            Courtship::TooYoung => {
                crate::text::fmt(&words[W::CourtTooYoung], &[&MARRYING_AGE.to_string()])
            }
            Courtship::TooFarApart => words[W::CourtTooFarApart].to_owned(),
            Courtship::Rivals => words[W::CourtRivals].to_owned(),
            Courtship::WedAlready => words[W::CourtWedAlready].to_owned(),
            Courtship::Unproven { outsider, renown } => crate::text::fmt(
                &words[W::CourtUnproven],
                &[
                    &heroes[outsider].name,
                    &renown.to_string(),
                    &MARRY_IN_RENOWN.to_string(),
                ],
            ),
            Courtship::WillWed => words[W::CourtWillWed].to_owned(),
        }
    }
}

/// The courtship verdict for the garden's two seats (SPEC §11.6), checks in order.
pub fn courtship(heroes: &[Hero], a: Option<HeroId>, b: Option<HeroId>) -> Courtship {
    let (a, b) = match (a, b) {
        (None, None) => return Courtship::Nobody,
        (Some(a), Some(b)) => (a, b),
        _ => return Courtship::Waiting,
    };
    if kin(heroes, a, b) {
        return Courtship::Kin;
    }
    if heroes[a].age < MARRYING_AGE || heroes[b].age < MARRYING_AGE {
        return Courtship::TooYoung;
    }
    if (heroes[a].age - heroes[b].age).abs() > COURTING_AGE_GAP {
        return Courtship::TooFarApart;
    }
    if heroes[a]
        .bond_to(b)
        .is_some_and(|bond| bond.kind == BondKind::Rival)
    {
        return Courtship::Rivals;
    }
    if has_living_spouse(heroes, a) || has_living_spouse(heroes, b) {
        return Courtship::WedAlready;
    }
    if let Some((outsider, renown)) = may_marry_in(heroes, a, b) {
        return Courtship::Unproven { outsider, renown };
    }
    Courtship::WillWed
}

/// A living spouse: widowed heroes may remarry (SPEC §11.6). SPEC-GAPS KG-45: living is
/// fate LIVING, so a crowned spouse does not bar a new one.
fn has_living_spouse(heroes: &[Hero], hero: HeroId) -> bool {
    heroes[hero]
        .bonds
        .iter()
        .any(|bond| bond.kind == BondKind::Spouse && heroes[bond.other].is_living())
}

/// What resting by the fire will do (SPEC §11.3 step 1, §10.2 shedding).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rest {
    /// A wound will close.
    pub heals: bool,
    /// Dread will ease by one (not if broken; SPEC-GAPS KG-11: nothing at 0).
    pub calms: bool,
}

/// The rest `hero` will have by the fire.
pub fn rest(hero: &Hero) -> Rest {
    Rest {
        heals: hero.wounded,
        calms: crate::fear::can_shed(hero),
    }
}

impl Rest {
    /// The fire's preview (`ui.winter.rest_notes`): "heals" for a wound, else "calms"
    /// for dread, else "rests". SPEC-GAPS KG-42: a hero who would do both reads "heals".
    pub fn note(self, content: &Content) -> &str {
        let words = &content.words;
        if self.heals {
            &words[W::RestHeals]
        } else if self.calms {
            &words[W::RestCalms]
        } else {
            &words[W::RestRests]
        }
    }
}

/// A long-table seat's part (SPEC §11.3 step 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Teller {
    /// The first adult: renown to the house and to them.
    House,
    /// A second adult: renown to them only.
    Own,
    /// A child: ignored entirely.
    Child,
}

impl Teller {
    /// The table's preview: "house +", "own +", "a child".
    pub fn note(self, content: &Content) -> &str {
        let words = &content.words;
        match self {
            Teller::House => &words[W::WinterHousePlus],
            Teller::Own => &words[W::WinterOwnPlus],
            Teller::Child => &words[W::WinterAChild],
        }
    }
}

/// Each table seat's part, in seat order: the first adult tells it for the house, a
/// second adult for themselves, a child not at all.
pub fn tellers<const N: usize>(heroes: &[Hero], table: [Option<HeroId>; N]) -> [Option<Teller>; N] {
    let mut house_told = false;
    table.map(|seat| {
        let hero = seat?;
        if !heroes[hero].is_adult() {
            return Some(Teller::Child);
        }
        // Variant: the first *family* adult tells it for the house; an outsider adult
        // tells it for themselves, whichever seat.
        Some(if is_family(&heroes[hero]) && !house_told {
            house_told = true;
            Teller::House
        } else {
            Teller::Own
        })
    })
}
