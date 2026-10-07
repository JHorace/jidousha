//! Traits (VARIANT.md): the variant's rudimentary genetics. Six traits, a gift and a
//! flaw per aptitude; a hero holds at most one per aptitude. A gift adds
//! `TRAIT_POWER` to a quest of its aptitude, a flaw takes it off; nothing else reads a
//! trait. Traits are blood: founders are authored with them, a child rolls them from
//! its parents at birth (`trait_from`, once per aptitude), succession passes none.

use jidousha::prelude::Rng;

use crate::chance::{chance, index};
use crate::constants::{TRAIT_PASS_CHANCE, TRAIT_POWER, TRAIT_SPRING_CHANCE};
use crate::hero::Hero;
use crate::ids::{Aptitude, Trait};

impl Trait {
    /// The aptitude it bears on.
    pub fn aptitude(self) -> Aptitude {
        match self {
            Trait::Strong | Trait::Frail => Aptitude::Might,
            Trait::Sharp | Trait::Dull => Aptitude::Wits,
            Trait::Steadfast | Trait::Faint => Aptitude::Spirit,
        }
    }

    /// A gift, not a flaw.
    pub fn gift(self) -> bool {
        matches!(self, Trait::Strong | Trait::Sharp | Trait::Steadfast)
    }

    /// The gift (`gift`) or the flaw of `aptitude`.
    pub fn of(aptitude: Aptitude, gift: bool) -> Trait {
        match (aptitude, gift) {
            (Aptitude::Might, true) => Trait::Strong,
            (Aptitude::Might, false) => Trait::Frail,
            (Aptitude::Wits, true) => Trait::Sharp,
            (Aptitude::Wits, false) => Trait::Dull,
            (Aptitude::Spirit, true) => Trait::Steadfast,
            (Aptitude::Spirit, false) => Trait::Faint,
        }
    }

    /// What it adds to a quest of its aptitude: +1 for a gift, -1 for a flaw.
    pub fn power(self) -> i32 {
        if self.gift() {
            TRAIT_POWER
        } else {
            -TRAIT_POWER
        }
    }
}

/// The trait `hero` has in `aptitude`, if any.
pub fn trait_in(hero: &Hero, aptitude: Aptitude) -> Option<Trait> {
    hero.traits
        .iter()
        .copied()
        .find(|t| t.aptitude() == aptitude)
}

/// What `hero`'s traits add to a quest of `aptitude`.
pub fn power(hero: &Hero, aptitude: Aptitude) -> i32 {
    trait_in(hero, aptitude).map_or(0, Trait::power)
}

/// Where a child's trait came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The first parent's, or the second's (also when both had it).
    Parent(usize),
    /// Neither parent had it, and the child sprang it.
    Sprung,
}

/// One aptitude's trait for a child of parents holding `first` and `second` there, in
/// roll order: both the same, the child has it with no roll (both none included); one
/// parent only, one roll against `TRAIT_PASS_CHANCE`; a gift and a flaw, one coin for
/// which; then, only if the child still has none, one roll against
/// `TRAIT_SPRING_CHANCE` and, if it hits, one coin for a gift or a flaw.
pub fn trait_from(
    first: Option<Trait>,
    second: Option<Trait>,
    aptitude: Aptitude,
    rng: &mut Rng,
) -> Option<(Trait, Source)> {
    let passed = match (first, second) {
        (Some(a), Some(b)) if a == b => Some((a, Source::Parent(1))),
        (Some(a), Some(b)) => {
            let from = index(rng, 2);
            Some(if from == 0 {
                (a, Source::Parent(0))
            } else {
                (b, Source::Parent(1))
            })
        }
        (Some(a), None) => chance(rng, TRAIT_PASS_CHANCE as f32).then_some((a, Source::Parent(0))),
        (None, Some(b)) => chance(rng, TRAIT_PASS_CHANCE as f32).then_some((b, Source::Parent(1))),
        (None, None) => None,
    };
    passed.or_else(|| {
        chance(rng, TRAIT_SPRING_CHANCE as f32).then(|| {
            let held = Trait::of(aptitude, index(rng, 2) == 0);
            // A spring that lands on a parent's own trait is told as theirs, never as
            // "neither parent is" (the second parent first, as a shared one is).
            let source = if second == Some(held) {
                Source::Parent(1)
            } else if first == Some(held) {
                Source::Parent(0)
            } else {
                Source::Sprung
            };
            (held, source)
        })
    })
}

/// A child's traits from `first` and `second`, Might then Wits then Spirit, with where
/// each came from.
pub fn birth_traits(first: &Hero, second: &Hero, rng: &mut Rng) -> Vec<(Trait, Source)> {
    Aptitude::ALL
        .iter()
        .filter_map(|&a| trait_from(trait_in(first, a), trait_in(second, a), a, rng))
        .collect()
}
