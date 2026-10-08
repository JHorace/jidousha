//! The black mark — the variant's stain on the family name (DESIGN.md, "Marks").
//!
//! Mainline Keifu has no personal quest and nothing that outlives a failure but a
//! wound, a scar and the house's renown. The variant: a quest is a hero's
//! **personal quest** when it calls their dream (or burden) with "must succeed" or
//! "must triumph" (SPEC §9.6, `calls::dream_call`); a setback or a disaster on it
//! marks them — +1 or +2 — costs the house that much renown now (an outsider's
//! failure costs only their own), and the marks pass on: a newborn carries half
//! the heavier parent's, an heir chosen on a death page half the dead's, and every
//! turning the house pays a toll on the marks its living family carry.
//!
//! `stain` is the one function: the card's preview, the sheet's and the roll's
//! resolution all read it, so what the card promises is what the roll does.

use crate::calls::{Telling, dream_call};
use crate::content::Content;
use crate::hero::{Hero, HeroId};
use crate::ids::Outcome;
use crate::power::QuestFacts;

/// The mark a setback leaves.
pub const SETBACK_MARK: i32 = 1;
/// The mark a disaster leaves.
pub const DISASTER_MARK: i32 = 2;
/// The turning's toll is the living family's marks divided by this.
pub const TOLL_DIVISOR: i32 = 4;
/// A newborn and an heir each carry the marks they inherit divided by this.
pub const DECAY_DIVISOR: i32 = 2;

/// What one failure does to one hero and the house.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stain {
    /// Marks added to the hero.
    pub weight: i32,
    /// House renown lost now (0 for an outsider: the name is not theirs).
    pub house_loss: i32,
    /// Personal renown lost now (never below 0).
    pub personal_loss: i32,
}

/// What `outcome` on their personal quest does to `hero` — `None` unless it failed.
pub fn stain(hero: &Hero, outcome: Outcome) -> Option<Stain> {
    let weight = match outcome {
        Outcome::Setback => SETBACK_MARK,
        Outcome::Disaster => DISASTER_MARK,
        Outcome::Success | Outcome::Triumph => return None,
    };
    Some(Stain {
        weight,
        house_loss: if hero.is_family() { weight } else { 0 },
        personal_loss: weight.min(hero.renown.max(0)),
    })
}

/// The seated members for whom this quest is personal: living, and their dream or
/// burden called with "must succeed" or "must triumph" with this party seated. Never
/// at the Door.
pub fn personal(
    content: &Content,
    heroes: &[Hero],
    quest: QuestFacts<'_>,
    party: &[HeroId],
) -> Vec<HeroId> {
    if quest.door_lock {
        return Vec::new();
    }
    party
        .iter()
        .copied()
        .filter(|&m| heroes[m].is_living())
        .filter(|&m| {
            matches!(
                dream_call(content, heroes, m, quest, party),
                Some(call) if matches!(call.telling, Telling::Succeed | Telling::Triumph)
            )
        })
        .collect()
}

/// The quest card's mark line, read from `stain` at both failing outcomes:
/// "Fail 31%: mark Maren +1/+2, house -1/-2". `fail_percent` is the card's setback
/// percentage, which counts setbacks and disasters together.
pub fn mark_line(heroes: &[Hero], at_stake: &[HeroId], fail_percent: i32) -> Option<String> {
    if at_stake.is_empty() {
        return None;
    }
    let names: Vec<&str> = at_stake.iter().map(|&m| heroes[m].name.as_str()).collect();
    let (mut house_setback, mut house_disaster) = (0, 0);
    for &m in at_stake {
        house_setback += stain(&heroes[m], Outcome::Setback).map_or(0, |s| s.house_loss);
        house_disaster += stain(&heroes[m], Outcome::Disaster).map_or(0, |s| s.house_loss);
    }
    Some(format!(
        "Fail {fail_percent}%: mark {} +{SETBACK_MARK}/+{DISASTER_MARK}, house -{house_setback}/-{house_disaster}",
        names.join(", ")
    ))
}

/// The quest sheet's word on a call that makes the quest personal; the card's mark
/// line carries the numbers.
pub const SHEET_SUFFIX: &str = " Failing marks the name.";

/// Mark every member for whom the quest was personal (read before the roll) and who
/// failed it: their marks rise, the house and they lose renown, and a line says so.
pub fn apply(
    heroes: &mut [Hero],
    house_renown: &mut i32,
    at_stake: &[HeroId],
    outcome: Outcome,
    lines: &mut Vec<String>,
) {
    for &m in at_stake {
        let Some(s) = stain(&heroes[m], outcome) else {
            continue;
        };
        let hero = &mut heroes[m];
        hero.marks += s.weight;
        hero.renown -= s.personal_loss;
        *house_renown = (*house_renown - s.house_loss).max(0);
        lines.push(if s.house_loss > 0 {
            format!(
                "{} failed a personal quest: a black mark on the name (+{}), house renown -{}.",
                hero.name, s.weight, s.house_loss
            )
        } else {
            format!(
                "{} failed a personal quest: a black mark (+{}), an outsider's own.",
                hero.name, s.weight
            )
        });
    }
}

/// The marks a newborn carries: half the heavier parent's.
pub fn born_marks(first: &Hero, second: &Hero) -> i32 {
    first.marks.max(second.marks) / DECAY_DIVISOR
}

/// The turning's toll: the living family's marks, divided.
pub fn toll(heroes: &[Hero]) -> i32 {
    heroes
        .iter()
        .filter(|h| h.is_living() && h.is_family())
        .map(|h| h.marks)
        .sum::<i32>()
        / TOLL_DIVISOR
}

/// The hero sheet's line: "Outsider. Blood: Strong, Bold. Marks: 2." — `None` for
/// family with no traits and no marks, whose sheet reads as mainline's.
pub fn sheet_blood(hero: &Hero) -> Option<String> {
    if hero.is_family() && hero.marks == 0 && crate::genes::traits(&hero.genes).is_empty() {
        return None;
    }
    Some(format!(
        "{}. Blood: {}. Marks: {}.",
        if hero.is_family() {
            "Family"
        } else {
            "Outsider"
        },
        crate::genes::traits_word(&hero.genes),
        hero.marks
    ))
}

/// The lines a birth or an arrival page adds about the newcomer's blood.
pub fn blood_lines(hero: &Hero) -> Vec<String> {
    let mut lines = vec![format!(
        "{}'s blood: {}.",
        hero.name,
        crate::genes::traits_word(&hero.genes)
    )];
    if !hero.is_family() {
        lines.push(format!(
            "{} is an outsider: renown won is {}'s own until a marriage brings {} into the name.",
            hero.name, hero.name, hero.name
        ));
    }
    if hero.marks > 0 {
        lines.push(format!(
            "{} is born under {} black mark{}.",
            hero.name,
            hero.marks,
            if hero.marks == 1 { "" } else { "s" }
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hero::Blood;
    use crate::ids::{Pronoun, Tag};
    use crate::newcomers::{fear_of, newcomer};

    fn hero(blood: Blood, renown: i32) -> Hero {
        let mut h = newcomer(
            "A".into(),
            "B".into(),
            Pronoun::He,
            20,
            0,
            fear_of(Tag::ALL[0]),
        );
        h.blood = blood;
        h.renown = renown;
        h
    }

    #[test]
    fn a_family_setback_marks_one_and_costs_the_house_one() {
        let s = stain(&hero(Blood::Family, 5), Outcome::Setback);
        assert_eq!(
            s,
            Some(Stain {
                weight: 1,
                house_loss: 1,
                personal_loss: 1
            })
        );
    }

    #[test]
    fn an_outsiders_disaster_costs_the_house_nothing() {
        let s = stain(&hero(Blood::Outsider, 1), Outcome::Disaster);
        assert_eq!(
            s,
            Some(Stain {
                weight: 2,
                house_loss: 0,
                personal_loss: 1
            })
        );
    }

    #[test]
    fn a_success_leaves_no_mark() {
        assert_eq!(stain(&hero(Blood::Family, 5), Outcome::Success), None);
    }

    #[test]
    fn a_newborn_carries_half_the_heavier_parents_marks() {
        let mut a = hero(Blood::Family, 0);
        let b = hero(Blood::Family, 0);
        a.marks = 5;
        assert_eq!(born_marks(&a, &b), 2);
    }
}
