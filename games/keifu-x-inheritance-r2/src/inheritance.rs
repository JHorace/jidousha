//! The variant's inheritance (DESIGN decisions 11-12): a newborn's aptitudes bred from
//! both parents, and what an heir takes from the dead.
//!
//! Mainline gives a newborn the average of both parents' bases, a little more by
//! chance; here each aptitude comes from one parent or the other by a coin, and an
//! aptitude both parents have as their best breeds true. Mainline's heir takes the
//! heirloom and the undone dream; here they take the dead's blessings and marks too.
//! `succession` is the one function the death page's preview (the WOULD INHERIT section
//! in the dock) and the choice itself (`heirs::bequeath`) read, so what the player is
//! shown is what the heir gets.

use jidousha::prelude::Rng;

use crate::blessing::blessing_effect;
use crate::chance::{between, index};
use crate::constants::{APTITUDE_LIMIT, BRED_TRUE_BONUS, DOMINANT_SHARE, NEWBORN_APTITUDE_LEAST};
use crate::content::Content;
use crate::dream::told_title;
use crate::heirs::{can_take_dream, undone_dream};
use crate::hero::{Hero, HeroId};
use crate::ids::Aptitude;
use crate::marks::{Mark, Passing, passing};
use crate::text::fmt;
use crate::words::W;

/// A newborn's base aptitudes from `first` (the parent created first) and `second`
/// (DESIGN decision 11): per aptitude in order, a coin names the dominant parent (0 the
/// first, 1 the second), then `max(dominant / DOMINANT_SHARE + U{0,1}, 1)`. Returns the
/// bases and the aptitude that bred true, if both parents' best is the same — that one
/// gets `BRED_TRUE_BONUS`, at most `APTITUDE_LIMIT`.
pub fn conceive(first: &Hero, second: &Hero, rng: &mut Rng) -> ([i32; 3], Option<Aptitude>) {
    let mut aptitudes = [0; 3];
    for aptitude in Aptitude::ALL {
        let dominant = if index(rng, 2) == 0 { first } else { second };
        aptitudes[aptitude.index()] = (dominant.base(*aptitude) / DOMINANT_SHARE
            + between(rng, 0, 1))
        .max(NEWBORN_APTITUDE_LEAST);
    }
    let best = first.best_aptitude();
    let bred_true = (best == second.best_aptitude()).then_some(best);
    if let Some(aptitude) = bred_true {
        let value = &mut aptitudes[aptitude.index()];
        *value = (*value + BRED_TRUE_BONUS).min(APTITUDE_LIMIT);
    }
    (aptitudes, bred_true)
}

/// What an heir would take from the dead (DESIGN decision 12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Succession {
    /// The heirloom's name, if the dead holds one.
    pub heirloom: Option<String>,
    /// The undone dream's told title, if the heir can take it.
    pub dream: Option<String>,
    /// Each of the dead's blessings the heir lacks, by title.
    pub blessings: Vec<String>,
    /// Each of the dead's marks, and how it arrives.
    pub marks: Vec<(Mark, Passing)>,
}

/// What `heir` takes if chosen as `dead`'s heir: the heirloom; the undone dream, if
/// `heirs::can_take_dream`; every blessing of the dead whose title the heir lacks; and
/// every mark, at half its weight or struck.
pub fn succession(content: &Content, heroes: &[Hero], dead: HeroId, heir: HeroId) -> Succession {
    let gone = &heroes[dead];
    let taker = &heroes[heir];
    let dream = undone_dream(gone)
        .filter(|_| can_take_dream(taker))
        .map(|(_, dream)| {
            let owner = dream.owner.map_or(gone.pronoun, |o| heroes[o].pronoun);
            told_title(content, dream, owner)
        });
    Succession {
        heirloom: gone.heirloom.as_ref().map(|h| h.name.clone()),
        dream,
        blessings: gone
            .blessings
            .iter()
            .filter(|b| !taker.blessings.iter().any(|t| t.title == b.title))
            .map(|b| b.title.clone())
            .collect(),
        marks: gone
            .marks
            .iter()
            .map(|mark| (mark.clone(), passing(mark.weight)))
            .collect(),
    }
}

/// The dock's WOULD INHERIT section for `heir` on `dead`'s death page: its heading,
/// then the heirloom, the dream, the blessings and each mark — or "Nothing but the
/// name." All of it read off `succession`.
pub fn would_inherit(
    content: &Content,
    heroes: &[Hero],
    dead: HeroId,
    heir: HeroId,
) -> Vec<String> {
    let words = &content.words;
    let gone = &heroes[dead];
    let taking = succession(content, heroes, dead, heir);
    let mut out = vec![words[W::SheetWouldInherit].to_owned()];
    if let (Some(name), Some(heirloom)) = (&taking.heirloom, &gone.heirloom) {
        let effect = fmt(
            &content.legacies.heirloom_effect,
            &[
                &heirloom.bonus.to_string(),
                &content.lore.aptitudes[heirloom.aptitude.index()],
            ],
        );
        out.push(fmt(&words[W::SheetInheritHeirloom], &[name, &effect]));
    }
    if let Some(dream) = &taking.dream {
        out.push(fmt(&words[W::SheetInheritDream], &[dream]));
    }
    for title in &taking.blessings {
        if let Some(blessing) = gone.blessings.iter().find(|b| b.title == *title) {
            out.push(fmt(
                &words[W::SheetInheritBlessing],
                &[title, &blessing_effect(content, blessing)],
            ));
        }
    }
    for (mark, arrives) in &taking.marks {
        out.push(match arrives {
            Passing::Arrives(weight) => fmt(
                &words[W::SheetInheritMark],
                &[&weight.to_string(), &mark.title, &mark.year.to_string()],
            ),
            Passing::Struck => fmt(
                &words[W::SheetInheritMarkStruck],
                &[&mark.title, &mark.year.to_string()],
            ),
        });
    }
    if out.len() == 1 {
        out.push(words[W::SheetInheritNothing].to_owned());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{founded, id};

    fn parents(first: [i32; 3], second: [i32; 3]) -> (Hero, Hero) {
        let (_, heroes) = founded();
        let mut a = heroes[id(&heroes, "Odo")].clone();
        let mut b = heroes[id(&heroes, "Ysolde")].clone();
        a.aptitudes = first;
        b.aptitudes = second;
        (a, b)
    }

    #[test]
    fn a_newborn_takes_each_aptitude_from_one_parent_or_the_other() {
        let (a, b) = parents([7, 2, 2], [3, 6, 2]);
        // Each base is half one parent's, maybe one more, at least 1: Might from 7 (3,
        // 4) or 3 (1, 2); Wits from 2 (1, 2) or 6 (3, 4); Spirit from 2 (1, 2).
        let allowed: [&[i32]; 3] = [&[1, 2, 3, 4], &[1, 2, 3, 4], &[1, 2]];
        for seed in 0..64 {
            let mut rng = Rng::from_seed(seed);
            let (child, bred) = conceive(&a, &b, &mut rng);
            assert_eq!(
                bred, None,
                "Might and Wits are their bests: nothing breeds true"
            );
            for (value, allowed) in child.iter().zip(allowed) {
                assert!(allowed.contains(value), "{child:?} on seed {seed}");
            }
        }
        let mut rng = Rng::from_seed(1);
        assert_eq!(conceive(&a, &b, &mut rng).0, [1, 4, 1]);
    }

    #[test]
    fn a_child_of_two_parents_who_share_a_best_aptitude_is_bred_true() {
        let (a, b) = parents([6, 2, 2], [6, 2, 2]);
        for seed in 0..32 {
            let mut rng = Rng::from_seed(seed);
            let (child, bred) = conceive(&a, &b, &mut rng);
            assert_eq!(bred, Some(Aptitude::Might));
            // 6 / 2 = 3, +0 or +1, +1 bred true.
            assert!([4, 5].contains(&child[0]), "{child:?} on seed {seed}");
        }
    }
}
