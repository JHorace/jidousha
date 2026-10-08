//! The variant's family and outsiders (DESIGN decisions 1-6): who may wed into the
//! family, and what wedding in does.
//!
//! Mainline has no outsiders: anyone who comes to the house is its own, may wed anyone
//! the garden allows, and is offered as an heir. Here a wanderer is an outsider — they
//! earn personal renown and nothing for the house — until their renown reaches
//! `MARRY_IN_RENOWN` and they wed a family member, taking the family's name and
//! bringing `renown / DOWRY_SHARE` to the house. `marry_in` is the one eligibility the
//! garden's note, the garden's verdict and the wedding read (`plans::courtship`).

use crate::constants::{DOWRY_SHARE, MARRY_IN_RENOWN};
use crate::hero::{Hero, HeroId};

/// What a hero's standing is toward wedding into the family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarryIn {
    /// Their personal renown.
    pub renown: i32,
    /// The threshold, `MARRY_IN_RENOWN`.
    pub threshold: i32,
    /// Family already, or an outsider at or above the threshold.
    pub eligible: bool,
}

/// `hero`'s standing toward wedding into the family (DESIGN decision 2).
pub fn marry_in(hero: &Hero) -> MarryIn {
    MarryIn {
        renown: hero.renown,
        threshold: MARRY_IN_RENOWN,
        eligible: hero.family || hero.renown >= MARRY_IN_RENOWN,
    }
}

/// `outsider` weds into the family beside `spouse` (DESIGN decision 3): they become
/// family and take the spouse's house name. Returns the dowry the house gains.
pub fn join(heroes: &mut [Hero], outsider: HeroId, spouse: HeroId) -> i32 {
    let name = heroes[spouse].house.clone();
    let hero = &mut heroes[outsider];
    hero.family = true;
    hero.house = name;
    hero.renown / DOWRY_SHARE
}
