//! The black mark (VARIANT.md): a failed personal quest marks the family name.
//!
//! A quest is **personal** to a seated family hero when their own dream (never a burden)
//! calls it for the party as seated and the call needs it won. When a personal quest ends
//! in a setback or a disaster the hero takes a `Mark` — heritable, and weighed on the
//! house a renown a year until `MARK_YEARS` years have passed. `personal` names who the
//! quest is personal to (the card, the sheet and the resolution all call it);
//! `cost` is what a failure costs (the sheet's preview and `mark_the_name` both read it);
//! `mark_the_name` is the one writer of a mark; `weigh` is the turning's step 8b.
//!
//! INVARIANT: no hero carries one identity — `(origin, year, place)` — twice.

use crate::calls::{Telling, dream_call};
use crate::constants::{MARK_HOUSE_COST, MARK_PERSONAL_COST, MARK_YEARLY, MARK_YEARS};
use crate::content::Content;
use crate::hero::{Deed, DeedKind, Hero, HeroId};
use crate::house::House;
use crate::ids::Place;
use crate::outsiders::is_family;
use crate::power::QuestFacts;
use crate::resolve::Afield;
use crate::text::fmt;
use crate::words::W;

/// A black mark on the name: who failed, when, where, and how many times it has been
/// passed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mark {
    /// The hero whose quest failed.
    pub origin: HeroId,
    /// The year it failed.
    pub year: i32,
    /// Where.
    pub place: Place,
    /// How many times it has been passed on: 0 for the one who failed.
    pub generation: i32,
}

impl Mark {
    /// What makes two marks the same failure.
    pub fn identity(&self) -> (HeroId, i32, Place) {
        (self.origin, self.year, self.place)
    }

    /// The last year it weighs on the name at a turning.
    pub fn through(&self) -> i32 {
        self.year + MARK_YEARS - 1
    }
}

/// What failing a personal quest costs, now and later: the preview and the resolution
/// read the same numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cost {
    /// House renown, at once.
    pub house: i32,
    /// The failer's personal renown, at once.
    pub personal: i32,
    /// House renown, a year.
    pub yearly: i32,
    /// The last year it weighs, for a failure in `year`.
    pub through: i32,
}

/// The cost of a failure in `year`.
pub fn cost(year: i32) -> Cost {
    Cost {
        house: MARK_HOUSE_COST,
        personal: MARK_PERSONAL_COST,
        yearly: MARK_YEARLY,
        through: year + MARK_YEARS - 1,
    }
}

/// The seated heroes `quest` is personal to, in seat order: family whose own dream calls
/// it for `party` and needs it won (a "must succeed" or "must triumph" call; a "go" call
/// makes nothing personal). Never at the Door.
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
        .filter(|&member| is_family(&heroes[member]))
        .filter(|&member| {
            dream_call(content, heroes, member, quest, party).is_some_and(|call| {
                !call.burden && matches!(call.telling, Telling::Succeed | Telling::Triumph)
            })
        })
        .collect()
}

/// Mark the name for `member`'s failed personal quest (the one writer of a mark): the
/// mark on them, the house's and their renown paid, the line and the deed.
pub fn mark_the_name(f: &Afield<'_>, house: &mut House, member: HeroId, out: &mut Vec<String>) {
    let words = &f.content.words;
    let price = cost(f.year);
    let mark = Mark {
        origin: member,
        year: f.year,
        place: f.quest.place,
        generation: 0,
    };
    let hero = &mut house.heroes[member];
    if !hero.marks.iter().any(|m| m.identity() == mark.identity()) {
        hero.marks.push(mark);
    }
    hero.renown = (hero.renown - price.personal).max(0);
    let first = hero.name.clone();
    house.add_renown(-price.house);
    out.push(fmt(
        &words[W::MarkFallen],
        &[
            &first,
            crate::harm::place_name(f),
            &price.house.to_string(),
            &price.personal.to_string(),
        ],
    ));
    let hero = &mut house.heroes[member];
    hero.deeds.push(Deed {
        kind: DeedKind::Marked,
        year: f.year,
        age: hero.age,
        place: Some(f.quest.place),
        weight: f.quest.danger,
        other: None,
        telling: words[W::DeedMarked].to_owned(),
    });
}

/// The turning's step 8b: marks past `MARK_YEARS` are forgotten (a line for each distinct
/// one lifted from a living family member), then each distinct mark the living family
/// carries costs the house `MARK_YEARLY`. Returns the lines.
pub fn weigh(content: &Content, house: &mut House) -> Vec<String> {
    let words = &content.words;
    let year = house.calendar.current_year();
    let mut lines = Vec::new();
    let mut lapsed: Vec<Mark> = Vec::new();
    for hero in house.heroes.iter_mut() {
        let carried_by_the_living = hero.is_living() && is_family(hero);
        hero.marks.retain(|mark| {
            let spent = year - mark.year >= MARK_YEARS;
            if spent
                && carried_by_the_living
                && !lapsed.iter().any(|m| m.identity() == mark.identity())
            {
                lapsed.push(*mark);
            }
            !spent
        });
    }
    for mark in &lapsed {
        lines.push(fmt(
            &words[W::MarkLapsed],
            &[
                &house.heroes[mark.origin].name,
                &content.lore.places[mark.place.index()].name,
            ],
        ));
    }
    let carried = house.marks_carried().len() as i32;
    if carried > 0 {
        house.add_renown(-carried * MARK_YEARLY);
        lines.push(if carried == 1 {
            fmt(
                &words[W::MarkWeighsOne],
                &[&(carried * MARK_YEARLY).to_string()],
            )
        } else {
            fmt(
                &words[W::MarkWeighsMany],
                &[&carried.to_string(), &(carried * MARK_YEARLY).to_string()],
            )
        });
    }
    lines
}
