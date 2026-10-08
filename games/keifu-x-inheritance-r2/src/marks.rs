//! The variant's black marks on the family name (DESIGN decisions 8-9): what a mark is,
//! the name's marks, what one weighs when it passes to an heir, and the yearly drain.
//!
//! Mainline has no marks. Here a failed oath (`oath.rs`) puts a mark of the quest's
//! danger on the swearer; while a living family member carries it, it drains house
//! renown at every turning; at a death page it passes to the chosen heir at half its
//! weight (struck at 0) or goes into the ground with "No one" (`inheritance.rs`).

use crate::constants::{MARK_DRAIN, MARK_HALVING};
use crate::content::Content;
use crate::hero::{Hero, HeroId};
use crate::house::House;
use crate::ids::Place;
use crate::text::{fmt, name_list};
use crate::words::W;

/// One black mark on the name: the oath it was failed on, where, when, by whom, and
/// how heavy it is now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mark {
    /// The quest sworn on: "Grave goods".
    pub title: String,
    /// Where it was failed.
    pub place: Place,
    /// The year it was failed.
    pub year: i32,
    /// Who failed it.
    pub by: HeroId,
    /// Its weight: the quest's danger when taken, halved at each passing.
    pub weight: i32,
}

/// How a mark arrives at an heir.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Passing {
    /// At this weight.
    Arrives(i32),
    /// Halved to nothing: struck from the name.
    Struck,
}

/// What a mark of `weight` becomes at a passing: `weight / MARK_HALVING`, struck at 0.
pub fn passing(weight: i32) -> Passing {
    match weight / MARK_HALVING {
        0 => Passing::Struck,
        halved => Passing::Arrives(halved),
    }
}

/// The marks on the name: every mark a living family member carries, hero by hero in
/// creation order, each in the order it was taken.
pub fn house_marks(house: &House) -> Vec<(HeroId, &Mark)> {
    house
        .heroes
        .iter()
        .enumerate()
        .filter(|(_, hero)| hero.is_living() && hero.family)
        .flat_map(|(id, hero)| hero.marks.iter().map(move |mark| (id, mark)))
        .collect()
}

/// "Grave goods (4) and The bell under the tide (1)".
pub fn mark_list(content: &Content, marks: &[&Mark]) -> String {
    let items: Vec<String> = marks
        .iter()
        .map(|mark| format!("{} ({})", mark.title, mark.weight))
        .collect();
    let items: Vec<&str> = items.iter().map(String::as_str).collect();
    name_list(content, &items)
}

/// The death page's line for what `hero` leaves on the name, if they carry any mark.
pub fn leaves_marks(content: &Content, hero: &Hero) -> Option<String> {
    if hero.marks.is_empty() {
        return None;
    }
    let he = crate::text::capitalized(&content.lore.pronouns[hero.pronoun.index()].subject);
    let marks: Vec<&Mark> = hero.marks.iter().collect();
    Some(fmt(
        &content.words[W::DeathLeavesMarks],
        &[&he, &mark_list(content, &marks)],
    ))
}

/// The turning's drain (DESIGN decision 9, beside the tales): `MARK_DRAIN` house renown
/// per mark on the name, with its line; nothing with none.
pub fn drain(content: &Content, house: &mut House) -> Vec<String> {
    let count = house_marks(house).len() as i32;
    if count == 0 {
        return Vec::new();
    }
    let lost = count * MARK_DRAIN;
    house.add_renown(-lost);
    let words = &content.words;
    vec![if count == 1 {
        fmt(&words[W::MarksDrainOne], &[&lost.to_string()])
    } else {
        fmt(
            &words[W::MarksDrainMany],
            &[&count.to_string(), &lost.to_string()],
        )
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mark_halves_at_each_passing_and_is_struck_at_nothing() {
        assert_eq!(passing(4), Passing::Arrives(2));
        assert_eq!(passing(2), Passing::Arrives(1));
        assert_eq!(passing(1), Passing::Struck);
        assert_eq!(passing(3), Passing::Arrives(1));
    }
}
