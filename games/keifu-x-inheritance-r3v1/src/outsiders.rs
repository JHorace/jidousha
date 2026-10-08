//! Outsiders and family — the variant's line around the name (DESIGN.md,
//! "Outsiders").
//!
//! Mainline Keifu takes every wanderer straight into the house: their renown on a
//! quest is the house's, they may wed anyone the garden allows, and they inherit
//! like anyone else. The variant: a wanderer is an **outsider**. Outsiders are cheap
//! bodies — they quest, they earn personal renown — but they confer no family
//! renown, carry nothing for the house, and are never anyone's heir. To marry into
//! the family an outsider needs personal renown of at least `MARRY_IN_RENOWN`;
//! marrying in makes them family and brings half their renown to the house.
//!
//! `marry_in` is the one function: the garden's preview (`plans::courtship`) and the
//! wedding (`winter.rs`) both read it.

use crate::hero::{Blood, Hero, HeroId};

/// The personal renown an outsider needs to marry into the family.
pub const MARRY_IN_RENOWN: i32 = 6;
/// Marrying in brings the outsider's renown divided by this to the house.
pub const TRANSFER_DIVISOR: i32 = 2;

/// What the garden makes of a pair, as far as the name is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarryIn {
    /// Both are family: nothing to decide here.
    Family,
    /// An outsider below the threshold (or two outsiders): refused.
    Refused {
        /// The outsider judged.
        outsider: HeroId,
        /// Their personal renown.
        renown: i32,
    },
    /// An outsider at or above the threshold, courting family: accepted.
    Accepted {
        /// The outsider marrying in.
        outsider: HeroId,
        /// What the house gains from their renown.
        transfer: i32,
    },
}

/// Whether the pair `a`, `b` may wed as far as the name goes.
pub fn marry_in(heroes: &[Hero], a: HeroId, b: HeroId) -> MarryIn {
    let outsider = match (heroes[a].is_family(), heroes[b].is_family()) {
        (true, true) => return MarryIn::Family,
        (false, false) => {
            return MarryIn::Refused {
                outsider: a,
                renown: heroes[a].renown,
            };
        }
        (false, true) => a,
        (true, false) => b,
    };
    let renown = heroes[outsider].renown;
    if renown >= MARRY_IN_RENOWN {
        MarryIn::Accepted {
            outsider,
            transfer: renown / TRANSFER_DIVISOR,
        }
    } else {
        MarryIn::Refused { outsider, renown }
    }
}

/// Whether any of `members` is family (a quest's house renown needs one).
pub fn family_in(heroes: &[Hero], members: &[HeroId]) -> bool {
    members.iter().any(|&m| heroes[m].is_family())
}

/// The quest page's line when a party of outsiders wins.
pub fn no_family_renown_line(renown: i32) -> String {
    format!("Renown +{renown} each, their own: no one of the family went, so the house gains none.")
}

/// Make `outsider` family; return the renown the house gains.
pub fn take_into_the_name(heroes: &mut [Hero], outsider: HeroId, transfer: i32) -> i32 {
    heroes[outsider].blood = Blood::Family;
    transfer
}
