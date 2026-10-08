//! The black mark (variant, DESIGN.md S3): a family member who goes questing in
//! their own name and fails stains the family name.
//!
//! Mainline has no personal quest; its nearest thing is the dream call (SPEC §9.6),
//! and the variant reads it: a quest is a hero's personal quest when it is a
//! template quest, they are family and seated on it, and their own dream (not a
//! burden) calls them to go. Failing it is a Setback or a Disaster. The mark costs
//! renown now (house and personal), costs the house every year while it is carried,
//! and halves when it passes to an heir (`inheritance::succession`).

use crate::calls::{Telling, dream_call};
use crate::constants::{MARK_HOUSE_RENOWN, MARK_PERSONAL_RENOWN, MARK_YEARLY_RENOWN};
use crate::content::Content;
use crate::harm::{deed, place_name};
use crate::hero::{DeedKind, Hero, HeroId};
use crate::house::House;
use crate::quest::Quest;
use crate::resolve::Afield;
use crate::text::fmt;
use crate::words::W;

/// Whether `quest`, with `party` seated, is `hero`'s personal quest.
///
/// A burden is someone else's dream; a ghost's quest is the dead's business; a lock
/// of the Door is the house's; a call that moves the dream only if the dreamer stays
/// behind is not a call to go. An outsider is not of the name.
pub fn personal_quest(
    content: &Content,
    heroes: &[Hero],
    hero: HeroId,
    quest: &Quest,
    party: &[HeroId],
) -> bool {
    party.contains(&hero)
        && heroes[hero].is_family
        && quest.template().is_some()
        && dream_call(content, heroes, hero, quest.facts(), party)
            .is_some_and(|call| !call.burden && call.telling != Telling::StayBehind)
}

/// What failing a personal quest does to the name and to its bearer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Consequence {
    /// The bearer's marks once this one is added.
    pub marks_after: i32,
    /// House renown it costs now.
    pub house_renown: i32,
    /// Personal renown it costs now (floored at 0 when applied).
    pub personal_renown: i32,
    /// House renown each mark costs at every turning while it is carried.
    pub yearly: i32,
}

/// What a failure would put on `hero`.
///
/// CONTRACT: the quest sheet's stake line and `stain` both call this, so the
/// preview cannot promise what the resolution does not do.
pub fn consequence(house: &House, hero: HeroId) -> Consequence {
    Consequence {
        marks_after: house.heroes[hero].marks + 1,
        house_renown: MARK_HOUSE_RENOWN,
        personal_renown: MARK_PERSONAL_RENOWN,
        yearly: MARK_YEARLY_RENOWN,
    }
}

/// Put a black mark on `hero` for a failed personal quest: the marks, the two
/// renown costs, the quest page's line and the deed.
pub fn stain(f: &Afield<'_>, house: &mut House, hero: HeroId, lines: &mut Vec<String>) {
    let cost = consequence(house, hero);
    let words = &f.content.words;
    let possessive = &f.content.lore.pronouns[house.heroes[hero].pronoun.index()].possessive;
    lines.push(fmt(
        &words[W::QuestBlackMark],
        &[
            &house.heroes[hero].name,
            possessive,
            &house.family_house,
            &cost.house_renown.to_string(),
            &house.heroes[hero].name,
        ],
    ));
    house.add_renown(-cost.house_renown);
    let telling = fmt(&words[W::DeedBlackMark], &[possessive, place_name(f)]);
    let bearer = &mut house.heroes[hero];
    bearer.marks = cost.marks_after;
    bearer.renown = (bearer.renown - cost.personal_renown).max(0);
    deed(f, bearer, DeedKind::BlackMark, f.quest.danger, telling);
}

/// Every black mark the living family carries — what the turning charges the house.
pub fn carried(house: &House) -> i32 {
    house
        .heroes
        .iter()
        .filter(|hero| hero.is_living() && hero.is_family)
        .map(|hero| hero.marks)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{house, id};

    /// "Grave goods" off the first board, and Garrick, whose dream it calls.
    fn grave_goods(_content: &Content, house: &mut House) -> (Quest, HeroId) {
        let garrick = id(&house.heroes, "Garrick");
        let quest = house
            .board
            .iter()
            .find(|posted| posted.quest.title == "Grave goods")
            .map(|posted| posted.quest.clone());
        let Some(quest) = quest else {
            panic!(
                "no Grave goods on the first board: {:?}",
                house
                    .board
                    .iter()
                    .map(|p| &p.quest.title)
                    .collect::<Vec<_>>()
            );
        };
        (quest, garrick)
    }

    #[test]
    fn a_stain_floors_the_bearers_renown_at_zero_and_costs_the_house_two() {
        let (content, mut house) = house();
        let (quest, garrick) = grave_goods(&content, &mut house);
        house.heroes[garrick].renown = 1;
        let renown = house.renown;
        let f = Afield {
            content: &content,
            quest: &quest,
            outcome: crate::ids::Outcome::Setback,
            year: 1,
        };
        let mut lines = Vec::new();
        stain(&f, &mut house, garrick, &mut lines);
        assert_eq!(house.heroes[garrick].renown, 0);
        assert_eq!(house.heroes[garrick].marks, 1);
        assert_eq!(house.renown, renown - 2);
        assert_eq!(lines.len(), 1, "{lines:?}");
    }

    #[test]
    fn a_seated_family_member_called_by_their_own_dream_is_on_a_personal_quest() {
        let (content, mut house) = house();
        let (quest, garrick) = grave_goods(&content, &mut house);
        assert!(personal_quest(
            &content,
            &house.heroes,
            garrick,
            &quest,
            &[garrick]
        ));
    }

    #[test]
    fn a_dreamer_not_seated_is_on_no_personal_quest() {
        let (content, mut house) = house();
        let (quest, garrick) = grave_goods(&content, &mut house);
        let maren = id(&house.heroes, "Maren");
        assert!(!personal_quest(
            &content,
            &house.heroes,
            garrick,
            &quest,
            &[maren]
        ));
    }

    #[test]
    fn an_outsider_with_a_calling_dream_marks_nobody() {
        let (content, mut house) = house();
        let (quest, garrick) = grave_goods(&content, &mut house);
        house.heroes[garrick].is_family = false;
        assert!(!personal_quest(
            &content,
            &house.heroes,
            garrick,
            &quest,
            &[garrick]
        ));
    }

    #[test]
    fn carrying_someone_elses_dream_is_not_going_in_your_own_name() {
        let (content, mut house) = house();
        let (quest, garrick) = grave_goods(&content, &mut house);
        let maren = id(&house.heroes, "Maren");
        house.heroes[maren].burden = house.heroes[garrick].dream.clone();
        house.heroes[maren].dream = None;
        assert!(
            crate::calls::dream_call(&content, &house.heroes, maren, quest.facts(), &[maren])
                .is_some_and(|call| call.burden)
        );
        assert!(!personal_quest(
            &content,
            &house.heroes,
            maren,
            &quest,
            &[maren]
        ));
    }

    #[test]
    fn a_ghosts_quest_is_the_deads_business() {
        let (content, mut house) = house();
        let (mut quest, garrick) = grave_goods(&content, &mut house);
        quest.source = crate::quest::Source::Ghost(garrick);
        assert!(!personal_quest(
            &content,
            &house.heroes,
            garrick,
            &quest,
            &[garrick]
        ));
    }

    #[test]
    fn a_failure_adds_one_mark_and_costs_two_now_and_one_a_year() {
        let (_, mut house) = house();
        let garrick = id(&house.heroes, "Garrick");
        house.heroes[garrick].marks = 2;
        assert_eq!(
            consequence(&house, garrick),
            Consequence {
                marks_after: 3,
                house_renown: 2,
                personal_renown: 2,
                yearly: 1
            }
        );
    }

    #[test]
    fn only_the_living_family_are_charged_for_their_marks() {
        let (_, mut house) = house();
        let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
        house.heroes[garrick].marks = 2;
        house.heroes[maren].marks = 1;
        assert_eq!(carried(&house), 3);
        house.heroes[maren].is_family = false;
        assert_eq!(carried(&house), 2);
        house.heroes[garrick].fate = crate::hero::Fate::Dead;
        assert_eq!(carried(&house), 0);
    }
}
