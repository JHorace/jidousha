//! Resolving the winter (SPEC §11.3): the fire, the training yard, the benches, the
//! garden and the long table, in that order, with training (§11.3 "Training") and
//! crediting a teacher (§11.5).
//!
//! Each step asks `plans.rs` for its plan at the moment it resolves — the same
//! functions the seat previews read — and records the plan it carried out in a
//! `WinterPlan`, so a check can hold the previews read before the winter against what
//! the winter did. Every adult who acts witnesses their winter moment (§9.3) as they
//! act, so a dream can move — and be fulfilled, and leave its legacy — at the hearth.

use jidousha::prelude::Rng;

use crate::bonds::{change, form};
use crate::constants::{BENCHES, DOWRY_SHARE, FIRE_SEATS, GARDEN_SEATS, TALE_RENOWN, TALE_SEATS};
use crate::content::Content;
use crate::fear::shed_dread;
use crate::hearth::Seat;
use crate::hero::{Deed, DeedKind, HeroId};
use crate::house::House;
use crate::ids::{BondKind, Pool, WinterAction};
use crate::moment::Moment;
use crate::outsiders::marrying_in;
use crate::plans::{
    Courtship, Lesson, Rest, Teller, bench_lesson, courtship, rest, tellers, yard_lesson,
};
use crate::text::fmt;
use crate::witness::witness;
use crate::words::W;

/// What every winter seat will do, or did: the one record the previews are drawn from
/// (`plan`) and the resolution writes (`resolve_winter`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WinterPlan {
    /// Each fire seat's rest.
    pub rests: [Option<Rest>; FIRE_SEATS],
    /// The training yard's lesson, if a learner sits there.
    pub yard: Option<Lesson>,
    /// Each bench's lesson, if a child seat is filled.
    pub benches: [Option<Lesson>; BENCHES],
    /// The garden's verdict.
    pub garden: Option<Courtship>,
    /// Each table seat's part.
    pub tellers: [Option<Teller>; TALE_SEATS],
}

/// The plan the hearth's seats hold now (SPEC §11.2): what the previews show.
pub fn plan(content: &Content, house: &House) -> WinterPlan {
    let at = |seat| house.hearth.at(seat);
    let heroes = &house.heroes;
    WinterPlan {
        rests: std::array::from_fn(|i| at(Seat::Fire(i)).map(|hero| rest(&heroes[hero]))),
        yard: at(Seat::Learner).map(|l| yard_lesson(content, heroes, l, at(Seat::Teacher))),
        benches: std::array::from_fn(|b| {
            at(Seat::BenchChild(b))
                .map(|c| bench_lesson(content, heroes, c, at(Seat::BenchTeacher(b))))
        }),
        garden: Some(courtship(heroes, at(Seat::Garden(0)), at(Seat::Garden(1)))),
        tellers: tellers(heroes, std::array::from_fn(|i| at(Seat::Table(i)))),
    }
}

/// Let the winter pass (SPEC §11.3), before anyone ages. Returns the winter page's
/// lines and the plan each step carried out.
pub fn resolve_winter(
    content: &Content,
    house: &mut House,
    rng: &mut Rng,
) -> (Vec<String>, WinterPlan) {
    let mut lines = Vec::new();
    let mut done = WinterPlan::default();
    let at = |house: &House, seat| house.hearth.at(seat);
    // 1. The fire, in seat order.
    for i in 0..FIRE_SEATS {
        if let Some(hero) = at(house, Seat::Fire(i)) {
            done.rests[i] = Some(rest_by_the_fire(content, house, hero, rng, &mut lines));
        }
    }
    // 2. The training yard.
    let (learner, teacher) = (at(house, Seat::Learner), at(house, Seat::Teacher));
    match (learner, teacher) {
        (Some(l), _) => {
            let lesson = yard_lesson(content, &house.heroes, l, teacher);
            done.yard = Some(lesson);
            train(content, house, l, teacher, lesson, &mut lines);
            // OQ-21 [emergent]: an adult teacher is credited even for a wasted lesson.
            if let Some(t) = teacher.filter(|&t| house.heroes[t].is_adult()) {
                credit(content, house, t, l, WinterAction::Teach, &mut lines);
            }
        }
        (None, Some(t)) => lines.push(fmt(
            &content.words[W::WinterWaitsForLearner],
            &[&house.heroes[t].name],
        )),
        (None, None) => {}
    }
    // 3. The benches, bench 0 then bench 1.
    for b in 0..BENCHES {
        let (child, minder) = (
            at(house, Seat::BenchChild(b)),
            at(house, Seat::BenchTeacher(b)),
        );
        match (child, minder) {
            (Some(c), _) => {
                let lesson = bench_lesson(content, &house.heroes, c, minder);
                done.benches[b] = Some(lesson);
                train(content, house, c, minder, lesson, &mut lines);
                if let Some(m) = minder.filter(|_| lesson.amount() > 0) {
                    credit(content, house, m, c, WinterAction::MindAChild, &mut lines);
                }
            }
            (None, Some(m)) => lines.push(fmt(
                &content.words[W::WinterBenchNoChild],
                &[&house.heroes[m].name],
            )),
            (None, None) => {}
        }
    }
    // 4. The garden.
    let pair: [Option<HeroId>; GARDEN_SEATS] = std::array::from_fn(|i| at(house, Seat::Garden(i)));
    let verdict = courtship(&house.heroes, pair[0], pair[1]);
    done.garden = Some(verdict);
    court(content, house, pair, verdict, rng, &mut lines);
    // 5. The long table, in seat order.
    let table: [Option<HeroId>; TALE_SEATS] = std::array::from_fn(|i| at(house, Seat::Table(i)));
    done.tellers = tellers(&house.heroes, table);
    for (seat, part) in table.iter().zip(done.tellers) {
        if let (Some(hero), Some(part)) = (*seat, part) {
            tell_the_tale(content, house, hero, part, rng, &mut lines);
        }
    }
    (lines, done)
}

/// An adult who acts witnesses their winter moment (SPEC §9.3); a child's does nothing.
fn moment(
    content: &Content,
    house: &mut House,
    hero: HeroId,
    action: WinterAction,
    lines: &mut Vec<String>,
) {
    if house.heroes[hero].is_adult() {
        lines.extend(witness(content, house, hero, &Moment::Winter(action)));
    }
}

/// Rest by the fire (SPEC §11.3 step 1): the wound heals, a dread is shed unless
/// broken; one of three lines, with a RESTS pool line, or none if neither applied;
/// then an adult's REST moment.
fn rest_by_the_fire(
    content: &Content,
    house: &mut House,
    hero: HeroId,
    rng: &mut Rng,
    lines: &mut Vec<String>,
) -> Rest {
    let planned = rest(&house.heroes[hero]);
    let me = &mut house.heroes[hero];
    let healed = std::mem::replace(&mut me.wounded, false);
    let calmed = shed_dread(me);
    let did = Rest {
        heals: healed,
        calms: calmed,
    };
    debug_assert_eq!(did, planned, "the fire did what its preview said");
    let key = match (healed, calmed) {
        (true, true) => Some(W::WinterRestBoth),
        (true, false) => Some(W::WinterRestMended),
        (false, true) => Some(W::WinterRestCalmed),
        (false, false) => None,
    };
    if let Some(key) = key {
        let name = house.heroes[hero].name.clone();
        let pool = fmt(house.writing.pick(content, Pool::Rests, rng), &[&name]);
        let dread = house.heroes[hero].fear.dread.to_string();
        lines.push(match key {
            W::WinterRestMended => fmt(&content.words[key], &[&pool]),
            _ => fmt(&content.words[key], &[&pool, &dread]),
        });
    }
    moment(content, house, hero, WinterAction::Rest, lines);
    did
}

/// Training (SPEC §11.3): a useful lesson raises the base (`lines.winter.trained`),
/// a wasted one writes its excuse; then an adult learner's TRAIN moment — even if
/// nothing was learned (OQ-20).
fn train(
    content: &Content,
    house: &mut House,
    learner: HeroId,
    teacher: Option<HeroId>,
    lesson: Lesson,
    lines: &mut Vec<String>,
) {
    let words = &content.words;
    let name = house.heroes[learner].name.clone();
    let teacher_name = teacher.map(|t| house.heroes[t].name.clone());
    match lesson {
        Lesson::Learn { aptitude, amount } => {
            let me = &mut house.heroes[learner];
            me.aptitudes[aptitude.index()] += amount;
            let source = match &teacher_name {
                Some(t) => fmt(&words[W::WinterTrainedUnder], &[t]),
                None => words[W::WinterTrainedAlone].to_owned(),
            };
            lines.push(fmt(
                &words[W::WinterTrained],
                &[
                    &name,
                    &source,
                    &content.lore.aptitudes[aptitude.index()],
                    &me.base(aptitude).to_string(),
                ],
            ));
        }
        Lesson::Wasted(excuse) => {
            lines.push(excuse.wasted(content, &name, teacher_name.as_deref()));
        }
    }
    moment(content, house, learner, WinterAction::Train, lines);
}

/// Credit a teacher (SPEC §11.5): the mentor bond under the rank rule, `taught` on
/// the teacher's side whatever its kind, a winter taught (the first a TAUGHT deed),
/// the new-student line if they had no bond at all, and the teacher's moment.
fn credit(
    content: &Content,
    house: &mut House,
    teacher: HeroId,
    learner: HeroId,
    action: WinterAction,
    lines: &mut Vec<String>,
) {
    let year = house.calendar.current_year();
    let newly_bound = house.heroes[teacher].bond_to(learner).is_none();
    // Rank-limited: replaces companion, friend or rival; never spouse, parent or child.
    form(&mut house.heroes, learner, teacher, BondKind::Mentor, year);
    if let Some(bond) = house.heroes[teacher]
        .bonds
        .iter_mut()
        .find(|bond| bond.other == learner)
    {
        bond.taught = true;
    }
    let learner_name = house.heroes[learner].name.clone();
    let me = &mut house.heroes[teacher];
    me.winters_taught += 1;
    if me.winters_taught == 1 {
        // SPEC-GAPS KG-43: dated now at the teacher's age, no place, weight 0, the learner.
        me.deeds.push(Deed {
            kind: DeedKind::Taught,
            year,
            age: me.age,
            place: None,
            weight: 0,
            other: Some(learner),
            telling: fmt(&content.words[W::DeedTaught], &[&learner_name]),
        });
    }
    if newly_bound {
        lines.push(fmt(
            &content.words[W::WinterNewStudent],
            &[&house.heroes[teacher].name, &learner_name],
        ));
    }
    moment(content, house, teacher, action, lines);
}

/// The garden (SPEC §11.3 step 4): a wedding, rivals making peace, or nothing come of
/// it; then each adult occupant's COURT moment.
fn court(
    content: &Content,
    house: &mut House,
    pair: [Option<HeroId>; GARDEN_SEATS],
    verdict: Courtship,
    rng: &mut Rng,
    lines: &mut Vec<String>,
) {
    let words = &content.words;
    let year = house.calendar.current_year();
    match (verdict, pair) {
        (Courtship::WillWed, [Some(a), Some(b)]) => {
            form(&mut house.heroes, a, b, BondKind::Spouse, year);
            let (an, bn) = (house.heroes[a].name.clone(), house.heroes[b].name.clone());
            lines.push(fmt(
                house.writing.pick(content, Pool::Weddings, rng),
                &[&an, &bn],
            ));
            for (me, other) in [(a, b), (b, a)] {
                let other_name = house.heroes[other].full_name();
                let hero = &mut house.heroes[me];
                // SPEC-GAPS KG-43: as the TAUGHT deed, the spouse as `other`.
                hero.deeds.push(Deed {
                    kind: DeedKind::Wed,
                    year,
                    age: hero.age,
                    place: None,
                    weight: 0,
                    other: Some(other),
                    telling: fmt(&words[W::DeedWed], &[&other_name]),
                });
            }
            // Variant: an outsider who weds in becomes family (`outsiders::may_marry_in`
            // already held the garden to the threshold).
            if let Some(outsider) = marrying_in(&house.heroes, a, b) {
                let spouse = if outsider == a { b } else { a };
                let name = house.heroes[spouse].house.clone();
                let hero = &mut house.heroes[outsider];
                hero.family = true;
                hero.house = name.clone();
                let dowry = hero.renown / DOWRY_SHARE;
                let first = hero.name.clone();
                house.add_renown(dowry);
                // The name changes everywhere it is drawn, the dead's epitaphs included.
                for dead in 0..house.heroes.len() {
                    if house.heroes[dead].wording.is_some() {
                        crate::epitaph::recompose(content, &mut house.heroes, dead);
                    }
                }
                lines.push(fmt(
                    &words[W::WinterMarriedIn],
                    &[&first, &name, &dowry.to_string()],
                ));
            }
        }
        (Courtship::Rivals, [Some(a), Some(b)]) => {
            change(&mut house.heroes, a, b, BondKind::Friend, year);
            lines.push(fmt(
                &words[W::WinterRivalsMakePeace],
                &[&house.heroes[a].name, &house.heroes[b].name],
            ));
        }
        (Courtship::Nobody, _) => {}
        (Courtship::WillWed | Courtship::Rivals, _) => panic!(
            "[keifu] the garden's verdict {verdict:?} came with an empty seat\n  likely \
             cause: courtship() judged a pair it was not given\n  fix: SPEC §11.6 checks \
             the empty seats first"
        ),
        (_, pair) => {
            if let Some(first) = pair.iter().flatten().next() {
                lines.push(fmt(
                    &words[W::WinterCourtingFailed],
                    &[&house.heroes[*first].name],
                ));
            }
        }
    }
    for hero in pair.into_iter().flatten() {
        moment(content, house, hero, WinterAction::Court, lines);
    }
}

/// One adult at the long table (SPEC §11.3 step 5): +1 personal renown and a
/// TOLD_THE_TALE deed; the first also gives the house +1 (`lines.winter.tale_told`), a
/// second is told again (`lines.winter.tale_told_again`); then the TELL_THE_TALE moment.
/// A child at the table is ignored entirely.
fn tell_the_tale(
    content: &Content,
    house: &mut House,
    hero: HeroId,
    part: Teller,
    rng: &mut Rng,
    lines: &mut Vec<String>,
) {
    let words = &content.words;
    if part == Teller::Child {
        return;
    }
    let year = house.calendar.current_year();
    let me = &mut house.heroes[hero];
    me.renown += TALE_RENOWN;
    // SPEC-GAPS KG-43: dated now at the teller's age, no place, weight 0, nobody else.
    me.deeds.push(Deed {
        kind: DeedKind::ToldTheTale,
        year,
        age: me.age,
        place: None,
        weight: 0,
        other: None,
        telling: words[W::DeedToldTheTale].to_owned(),
    });
    let name = me.name.clone();
    let possessive = content.lore.pronouns[me.pronoun.index()].possessive.clone();
    let renown = TALE_RENOWN.to_string();
    match part {
        Teller::House => {
            house.add_renown(TALE_RENOWN);
            let told = fmt(house.writing.pick(content, Pool::TalesTold, rng), &[&name]);
            lines.push(fmt(&words[W::WinterTaleTold], &[&told, &renown, &name]));
        }
        Teller::Own => lines.push(fmt(
            &words[W::WinterTaleToldAgain],
            &[&name, &possessive, &renown, &name],
        )),
        Teller::Child => {}
    }
    moment(content, house, hero, WinterAction::TellTheTale, lines);
}
