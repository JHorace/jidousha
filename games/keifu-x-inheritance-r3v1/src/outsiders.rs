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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::resolve_rolled;
    use crate::testkit::{aim, founded, house, id, seat};
    use jidousha::prelude::Rng;

    #[test]
    fn an_outsider_below_the_threshold_is_refused_and_at_it_marries_in_with_half_their_renown() {
        let (_content, mut heroes) = founded();
        let (odo, maren) = (id(&heroes, "Odo"), id(&heroes, "Maren"));
        heroes[odo].blood = Blood::Outsider;
        heroes[odo].renown = 5;
        assert_eq!(
            marry_in(&heroes, odo, maren),
            MarryIn::Refused {
                outsider: odo,
                renown: 5
            }
        );
        heroes[odo].renown = 7;
        assert_eq!(
            marry_in(&heroes, maren, odo),
            MarryIn::Accepted {
                outsider: odo,
                transfer: 3
            }
        );
    }

    #[test]
    fn two_outsiders_cannot_wed_whatever_their_renown() {
        let (_content, mut heroes) = founded();
        let (odo, maren) = (id(&heroes, "Odo"), id(&heroes, "Maren"));
        for h in [odo, maren] {
            heroes[h].blood = Blood::Outsider;
            heroes[h].renown = 20;
        }
        assert!(matches!(
            marry_in(&heroes, odo, maren),
            MarryIn::Refused { .. }
        ));
    }

    #[test]
    fn a_party_of_outsiders_wins_the_house_no_renown_and_themselves_their_own() {
        let (content, mut house) = house();
        let odo = id(&house.heroes, "Odo");
        house.heroes[odo].blood = Blood::Outsider;
        seat(&mut house, 0, &[odo]);
        aim(&mut house, 0, [3, 3], 1);
        let (before, own) = (house.renown, house.heroes[odo].renown);
        resolve_rolled(&content, &mut house, &mut Rng::from_seed(5), 0, [3, 3]);
        assert_eq!(house.renown, before);
        assert!(house.heroes[odo].renown > own);
    }

    #[test]
    fn an_outsider_carrier_adds_nothing_to_the_house_beside_family() {
        let (content, mut house) = house();
        let (odo, maren) = (id(&house.heroes, "Odo"), id(&house.heroes, "Maren"));
        house.heroes[odo].blood = Blood::Outsider;
        house.heroes[odo].destiny.kind = crate::ids::Destiny::CarryTheHouse;
        seat(&mut house, 0, &[maren, odo]);
        aim(&mut house, 0, [3, 3], 1);
        let (before, renown) = (house.renown, house.board[0].quest.renown);
        resolve_rolled(&content, &mut house, &mut Rng::from_seed(5), 0, [3, 3]);
        assert_eq!(house.renown, before + renown);
    }

    #[test]
    fn the_turning_toll_is_the_living_familys_marks_over_four_and_ignores_outsiders() {
        let (_content, mut heroes) = founded();
        let (odo, maren, ysolde) = (
            id(&heroes, "Odo"),
            id(&heroes, "Maren"),
            id(&heroes, "Ysolde"),
        );
        heroes[maren].marks = 5;
        heroes[ysolde].marks = 4;
        heroes[odo].marks = 8;
        heroes[odo].blood = Blood::Outsider;
        // 9 marks of living family, over four.
        assert_eq!(crate::marks::toll(&heroes), 2);
    }

    #[test]
    fn strong_adds_one_a_copy_on_a_might_quest_and_bold_eases_a_feared_tag() {
        use crate::genes::{Trait, trait_power};
        use crate::ids::{Aptitude, Place};
        use crate::power::QuestFacts;
        let (_content, mut heroes) = founded();
        let maren = id(&heroes, "Maren");
        heroes[maren].genes = [Some(Trait::Strong), Some(Trait::Strong)];
        let might = QuestFacts {
            aptitude: Aptitude::Might,
            place: Place::ALL[0],
            tags: &[],
            door_lock: false,
        };
        assert_eq!(trait_power(&heroes[maren], might), 2);
        let feared = [heroes[maren].fear.tag];
        heroes[maren].genes = [Some(Trait::Bold), None];
        heroes[maren].fear.conquered = false;
        let wits = QuestFacts {
            aptitude: Aptitude::Wits,
            tags: &feared,
            ..might
        };
        assert_eq!(trait_power(&heroes[maren], wits), 1);
    }
}
