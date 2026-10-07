//! What a hero inherits (VARIANT.md): the one function the death page's preview, the heir
//! choice and the birth all call.
//!
//! `inherit(heroes, from, to)` is what `to` would carry having taken `from`'s inheritance:
//! their own traits (traits are blood, rolled at birth; succession passes none) and
//! their own marks plus each of `from`'s whose identity they do not already carry, one
//! generation further down. No other code merges marks.

use crate::hero::{Hero, HeroId};
use crate::ids::Trait;
use crate::marks::Mark;

/// What `to` carries having taken `from`'s inheritance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Carried {
    /// `to`'s own traits.
    pub traits: Vec<Trait>,
    /// `to`'s own marks, then `from`'s that are new to them, at `generation + 1`.
    pub marks: Vec<Mark>,
    /// How many of `marks` are new to `to`.
    pub taken: usize,
}

/// What `to` would carry having taken `from`'s inheritance.
pub fn inherit(heroes: &[Hero], from: HeroId, to: HeroId) -> Carried {
    let mut marks = heroes[to].marks.clone();
    let mut taken = 0;
    for mark in &heroes[from].marks {
        if !marks.iter().any(|m| m.identity() == mark.identity()) {
            marks.push(Mark {
                generation: mark.generation + 1,
                ..*mark
            });
            taken += 1;
        }
    }
    Carried {
        traits: heroes[to].traits.clone(),
        marks,
        taken,
    }
}
