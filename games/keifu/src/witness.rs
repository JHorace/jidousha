//! Witnessing a moment (SPEC §9.3) and fulfilment (§9.4).
//!
//! A hero witnesses a moment by testing their own dream's current stage; only if
//! the own dream made no progress at all is the burden tested. A met stage counts
//! once, and a stage whose count reaches its goal is done — so at most one stage of
//! one dream moves per moment. The last stage done fulfils the dream: the hero is
//! settled, dread drops to 0, and the dream leaves its legacy (`legacy`).

use crate::content::Content;
use crate::dream::{Dream, told_task, told_title};
use crate::hero::{Deed, DeedKind, HeroId};
use crate::house::House;
use crate::legacy::leave_legacy;
use crate::moment::{Moment, stage_met};
use crate::text::{capitalized, fmt};
use crate::words::W;

/// Which of a hero's two dreams.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    /// Their own.
    Own,
    /// The one they carry for someone else.
    Burden,
}

fn held(house: &House, hero: HeroId, which: Held) -> Option<&Dream> {
    let hero = &house.heroes[hero];
    match which {
        Held::Own => hero.dream.as_ref(),
        Held::Burden => hero.burden.as_ref(),
    }
}

fn held_mut(house: &mut House, hero: HeroId, which: Held) -> Option<&mut Dream> {
    let hero = &mut house.heroes[hero];
    match which {
        Held::Own => hero.dream.as_mut(),
        Held::Burden => hero.burden.as_mut(),
    }
}

/// `hero` witnesses `moment` (SPEC §9.3): the own dream first, the burden only if
/// the own dream made no progress. Returns the lines written, in order.
pub fn witness(
    content: &Content,
    house: &mut House,
    hero: HeroId,
    moment: &Moment<'_>,
) -> Vec<String> {
    let mut lines = Vec::new();
    if !advance(content, house, hero, Held::Own, moment, &mut lines) {
        advance(content, house, hero, Held::Burden, moment, &mut lines);
    }
    lines
}

/// Every living hero, in creation order, witnesses a resolved quest or the turning
/// of the year (SPEC §7.1 step 12, §18 step 9). Present = in the moment's party.
pub fn witness_all(content: &Content, house: &mut House, moment: &Moment<'_>) -> Vec<String> {
    let mut lines = Vec::new();
    for hero in 0..house.heroes.len() {
        if house.heroes[hero].is_living() {
            lines.extend(witness(content, house, hero, moment));
        }
    }
    lines
}

/// "his dream" / "Elsbeth's dream" (`lines.dream.bearing_*`).
fn bearing(content: &Content, house: &House, hero: HeroId, dream: &Dream) -> String {
    let words = &content.words;
    match dream.owner {
        Some(owner) => fmt(&words[W::DreamBearingOwned], &[&house.heroes[owner].name]),
        None => {
            let pronoun = house.heroes[hero].pronoun;
            fmt(
                &words[W::DreamBearingOwn],
                &[&content.lore.pronouns[pronoun.index()].possessive],
            )
        }
    }
}

/// Test one dream's current stage against the moment; count, finish the stage, or
/// fulfil it. Returns whether it made any progress.
fn advance(
    content: &Content,
    house: &mut House,
    hero: HeroId,
    which: Held,
    moment: &Moment<'_>,
    lines: &mut Vec<String>,
) -> bool {
    let words = &content.words;
    let year = house.calendar.current_year();
    let Some(dream) = held(house, hero, which) else {
        return false;
    };
    if dream.is_fulfilled() {
        return false;
    }
    let stage = dream.current;
    if !stage_met(content, &house.heroes, hero, dream, stage, moment) {
        return false;
    }
    let bearing = bearing(content, house, hero, dream);
    // SPEC-GAPS KG-14: a task is told about the hero whose line it is.
    let about = house.heroes[hero].pronoun;
    let name = house.heroes[hero].name.clone();
    let Some(dream) = held_mut(house, hero, which) else {
        return false;
    };
    dream.stages[stage].count += 1;
    let (count, goal) = (dream.stages[stage].count, dream.stages[stage].goal);
    if count < goal {
        let task = told_task(content, &dream.stages[stage].task, about);
        lines.push(fmt(
            &words[W::DreamCounted],
            &[
                &name,
                &bearing,
                &task,
                &count.to_string(),
                &goal.to_string(),
            ],
        ));
        return true;
    }
    dream.current += 1;
    let done = told_task(content, &dream.stages[stage].task, about);
    let place = match moment {
        Moment::Quest(quest) => Some(quest.place),
        Moment::Winter(_) | Moment::YearTurn => None,
    };
    if dream.is_fulfilled() {
        let dream = dream.clone();
        lines.extend(fulfil(content, house, hero, &dream, place, year));
        return true;
    }
    let next = told_task(content, &dream.stages[dream.current].task, about);
    lines.push(fmt(&words[W::DreamStageDone], &[&name, &bearing, &next]));
    let hero = &mut house.heroes[hero];
    // SPEC-GAPS KG-15: dated now, at the quest's place if a quest moved it, weight 0.
    hero.deeds.push(Deed {
        kind: DeedKind::DreamStep,
        year,
        age: hero.age,
        place,
        weight: 0,
        other: None,
        telling: fmt(&words[W::DeedDreamStep], &[&done]),
    });
    true
}

/// Fulfilment (SPEC §9.4): settled for good, dread 0, the line, a DREAM_FULFILLED
/// deed, and the dream's legacy. Returns the lines.
fn fulfil(
    content: &Content,
    house: &mut House,
    hero: HeroId,
    dream: &Dream,
    place: Option<crate::ids::Place>,
    year: i32,
) -> Vec<String> {
    let words = &content.words;
    let owner = dream.owner.map(|owner| &house.heroes[owner]);
    let me = &house.heroes[hero];
    let title = told_title(content, dream, owner.map_or(me.pronoun, |o| o.pronoun));
    let pronouns = &content.lore.pronouns[me.pronoun.index()];
    let he = capitalized(&pronouns.subject);
    let line = match owner {
        Some(owner) => fmt(
            &words[W::DreamFulfilledOwned],
            &[&me.name, &owner.name, &title, &he, &pronouns.object],
        ),
        None => fmt(
            &words[W::DreamFulfilled],
            &[&me.name, &title, &he, &pronouns.object],
        ),
    };
    let me = &mut house.heroes[hero];
    // SPEC-GAPS KG-15: as the DREAM_STEP deed.
    me.settled = true;
    me.fear.dread = 0;
    me.deeds.push(Deed {
        kind: DeedKind::DreamFulfilled,
        year,
        age: me.age,
        place,
        weight: 0,
        other: None,
        telling: fmt(&words[W::DeedDreamFulfilled], &[&title]),
    });
    let mut lines = vec![line];
    lines.extend(leave_legacy(content, house, hero, dream, year));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hero::Scope;
    use crate::ids::{DreamKind, Outcome, Place, Tag, WinterAction};
    use crate::moment::QuestMoment;
    use crate::testkit::{house, id};

    fn quest<'m>(
        place: Place,
        tags: &'m [Tag],
        outcome: Outcome,
        party: &'m [HeroId],
    ) -> Moment<'m> {
        Moment::Quest(QuestMoment {
            place,
            tags,
            outcome,
            party,
        })
    }

    #[test]
    fn garrick_triumphing_at_the_barrow_is_settled_and_his_rest_blesses_his_line() {
        let (content, mut house) = house();
        let garrick = id(&house.heroes, "Garrick");
        let party = [garrick];
        let lines = witness_all(
            &content,
            &mut house,
            &quest(Place::Barrow, &[Tag::Dark], Outcome::Triumph, &party),
        );
        assert_eq!(
            lines,
            [
                "Garrick has done it: to lay the Barrow's dead to rest. He is settled now, and dread has no hold on him.",
                "It leaves a blessing, Garrick's rest: +2 against Undead, for Garrick, Maren and Pip and every child born to them.",
            ]
        );
        let heroes = &house.heroes;
        assert!(heroes[garrick].settled);
        assert_eq!(heroes[garrick].fear.dread, 0);
        assert!(
            heroes[garrick]
                .dream
                .as_ref()
                .is_some_and(Dream::is_fulfilled)
        );
        for name in ["Garrick", "Maren", "Pip"] {
            let blessings = &heroes[id(heroes, name)].blessings;
            assert_eq!(blessings.len(), 1, "{name}");
            assert_eq!(
                (
                    blessings[0].title.as_str(),
                    blessings[0].scope,
                    blessings[0].power
                ),
                ("Garrick's rest", Scope::AgainstTag(Tag::Undead), 2)
            );
        }
        for name in ["Ysolde", "Brannoc", "Odo", "Wren"] {
            assert!(heroes[id(heroes, name)].blessings.is_empty(), "{name}");
        }
        assert_eq!(
            heroes[garrick].legacy,
            (
                crate::ids::LegacyKind::Blessing,
                "Garrick's rest".to_owned()
            )
        );
        let kinds: Vec<DeedKind> = heroes[garrick].deeds.iter().map(|d| d.kind).collect();
        assert_eq!(kinds, [DeedKind::DreamFulfilled, DeedKind::LeftLegacy]);
        assert_eq!(
            heroes[garrick].deeds[0].telling,
            "fulfilled the dream: to lay the Barrow's dead to rest"
        );
        assert_eq!(heroes[garrick].deeds[1].telling, "left Garrick's rest");
    }

    #[test]
    fn a_triumph_elsewhere_or_a_success_at_the_barrow_or_watching_from_home_moves_nothing() {
        let (content, mut house) = house();
        let (garrick, odo) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Odo"));
        for (place, outcome, party) in [
            (Place::HighPass, Outcome::Triumph, vec![garrick]),
            (Place::Barrow, Outcome::Success, vec![garrick]),
            (Place::Barrow, Outcome::Triumph, vec![odo]),
        ] {
            let lines = witness(
                &content,
                &mut house,
                garrick,
                &quest(place, &[Tag::Undead], outcome, &party),
            );
            assert!(lines.is_empty(), "{place:?} {outcome:?}");
        }
        assert!(!house.heroes[garrick].settled);
    }

    #[test]
    fn a_counted_stage_tells_its_count_and_a_done_stage_names_what_is_left() {
        let (content, mut house) = house();
        let garrick = id(&house.heroes, "Garrick");
        if let Some(dream) = house.heroes[garrick].dream.as_mut() {
            dream.current = 1;
            dream.stages[1].count = 0;
        }
        let party = [garrick];
        let undead = quest(Place::Deepwood, &[Tag::Undead], Outcome::Success, &party);
        assert_eq!(
            witness(&content, &mut house, garrick, &undead),
            ["Garrick comes one nearer his dream: succeed against the Undead twice, 1 of 2."]
        );
        assert!(
            house.heroes[garrick].deeds.is_empty(),
            "a count is not a step"
        );
        assert_eq!(
            witness(&content, &mut house, garrick, &undead),
            ["Garrick is a step nearer his dream. What is left: win a triumph at the Barrow."]
        );
        let deed = &house.heroes[garrick].deeds[0];
        assert_eq!(
            (deed.kind, deed.place, deed.telling.as_str()),
            (
                DeedKind::DreamStep,
                Some(Place::Deepwood),
                "came a step nearer: succeed against the Undead twice"
            )
        );
        assert_eq!(
            house.heroes[garrick].dream.as_ref().map(|d| d.current),
            Some(2)
        );
    }

    #[test]
    fn the_own_dream_moves_first_and_the_burden_only_when_the_own_did_not() {
        let (content, mut house) = house();
        let (pip, maren) = (id(&house.heroes, "Pip"), id(&house.heroes, "Maren"));
        let mut carried = house.heroes[maren].dream.clone();
        if let Some(dream) = carried.as_mut() {
            dream.owner = Some(maren);
        }
        house.heroes[pip].burden = carried;
        let party = [pip];
        let coast = quest(Place::DrownedCoast, &[Tag::Water], Outcome::Setback, &party);
        assert_eq!(
            witness(&content, &mut house, pip, &coast),
            ["Pip is a step nearer his dream. What is left: succeed against Water."]
        );
        assert_eq!(
            house.heroes[pip]
                .burden
                .as_ref()
                .map(|d| (d.current, d.stages[1].count)),
            Some((1, 0))
        );
        assert_eq!(
            witness(&content, &mut house, pip, &coast),
            ["Pip is a step nearer Maren's dream. What is left: succeed there against Water."]
        );
        assert_eq!(house.heroes[pip].dream.as_ref().map(|d| d.current), Some(1));
    }

    #[test]
    fn a_fulfilled_own_dream_leaves_the_moment_to_the_burden_and_an_owned_dream_is_done_for_its_owner()
     {
        let (content, mut house) = house();
        let (pip, elsbeth) = (id(&house.heroes, "Pip"), id(&house.heroes, "Elsbeth"));
        if let Some(dream) = house.heroes[pip].dream.as_mut() {
            dream.advance_to_stage(3);
        }
        let mut carried = house.heroes[elsbeth].dream.clone();
        if let Some(dream) = carried.as_mut() {
            dream.owner = Some(elsbeth);
            dream.advance_to_stage(2);
        }
        house.heroes[pip].burden = carried;
        let lines = witness(
            &content,
            &mut house,
            pip,
            &Moment::Winter(WinterAction::TellTheTale),
        );
        assert_eq!(
            lines,
            [
                "Pip has done what Elsbeth could not: to see the sea. It is finished. He is settled now, and dread has no hold on him.",
                "It leaves a tale: The tale of Elsbeth and the sea. It will be told as long as there is a house: +1 renown every year.",
            ]
        );
        assert_eq!(house.tales.len(), 1);
        assert_eq!((house.tales[0].about, house.tales[0].since), (elsbeth, 1));
        assert_eq!(
            house.heroes[pip].legacy.1,
            "The tale of Elsbeth and the sea"
        );
    }

    #[test]
    fn any_moment_stages_complete_one_per_moment_and_a_grown_child_finishes_the_cradle_ring() {
        let (content, mut house) = house();
        let (brannoc, wren) = (id(&house.heroes, "Brannoc"), id(&house.heroes, "Wren"));
        house.heroes[brannoc].dream =
            Some(Dream::build(&content, DreamKind::SeeAChildGrown, None, None).expect("builds"));
        assert_eq!(
            witness(&content, &mut house, brannoc, &Moment::YearTurn),
            ["Brannoc is a step nearer his dream. What is left: have a child."]
        );
        assert_eq!(
            witness(
                &content,
                &mut house,
                brannoc,
                &Moment::Winter(WinterAction::Court)
            ),
            ["Brannoc is a step nearer his dream. What is left: see a child come of age."]
        );
        assert!(
            witness(&content, &mut house, brannoc, &Moment::YearTurn).is_empty(),
            "Wren is eight"
        );
        house.heroes[wren].age = 12;
        assert_eq!(
            witness(&content, &mut house, brannoc, &Moment::YearTurn),
            [
                "Brannoc has done it: to see his child grown. He is settled now, and dread has no hold on him.",
                "It leaves an heirloom: the Hale cradle-ring. +2 Spirit on quests. Made by Brannoc Hale in year 1, the year a child of the house came of age.",
            ]
        );
        assert_eq!(
            house.heroes[brannoc]
                .heirloom
                .as_ref()
                .map(|h| h.name.as_str()),
            Some("the Hale cradle-ring")
        );
    }

    #[test]
    fn the_dead_witness_nothing_and_the_settled_keep_no_dread() {
        let (content, mut house) = house();
        let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
        house.heroes[garrick].fate = crate::hero::Fate::Dead;
        let party = [garrick, maren];
        assert!(
            witness_all(
                &content,
                &mut house,
                &quest(Place::Barrow, &[], Outcome::Triumph, &party)
            )
            .is_empty()
        );
        assert!(
            !house.heroes[garrick]
                .dream
                .as_ref()
                .is_some_and(Dream::is_fulfilled)
        );
        house.heroes[garrick].fate = crate::hero::Fate::Living;
        let mut dreadful = house.heroes[garrick].clone();
        dreadful.fear.dread = 4;
        house.heroes[garrick] = dreadful;
        witness(
            &content,
            &mut house,
            garrick,
            &quest(Place::Barrow, &[], Outcome::Triumph, &party),
        );
        assert_eq!(
            (
                house.heroes[garrick].settled,
                house.heroes[garrick].fear.dread
            ),
            (true, 0)
        );
        assert!(!crate::fear::can_dread(&house.heroes[garrick]));
    }
}
