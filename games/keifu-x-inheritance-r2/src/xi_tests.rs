//! The variant's rules as pure functions (DESIGN.md "Unit tests"): the garden's verdict
//! over outsiders, the table's first family teller, and the family-only heir list.

use crate::heirs::heirs;
use crate::plans::{Courtship, Teller, courtship, tellers};
use crate::testkit::{founded, id};

#[test]
fn an_outsider_below_the_threshold_is_unproven_and_at_it_weds_in() {
    let (_, mut heroes) = founded();
    let (maren, brannoc) = (id(&heroes, "Maren"), id(&heroes, "Brannoc"));
    heroes[brannoc].family = false;
    heroes[brannoc].renown = 3;
    // An outsider takes the first seat or the second alike.
    for (a, b) in [(brannoc, maren), (maren, brannoc)] {
        assert_eq!(
            courtship(&heroes, Some(a), Some(b)),
            Courtship::Unproven {
                who: brannoc,
                renown: 3,
                threshold: 4
            }
        );
    }
    heroes[brannoc].renown = 4;
    assert_eq!(
        courtship(&heroes, Some(maren), Some(brannoc)),
        Courtship::WillWedIn {
            outsider: brannoc,
            spouse: maren
        }
    );
    heroes[brannoc].family = true;
    heroes[brannoc].renown = 0;
    assert_eq!(
        courtship(&heroes, Some(maren), Some(brannoc)),
        Courtship::WillWed
    );
}

#[test]
fn two_outsiders_are_refused_the_garden() {
    let (_, mut heroes) = founded();
    let (maren, brannoc) = (id(&heroes, "Maren"), id(&heroes, "Brannoc"));
    heroes[brannoc].family = false;
    heroes[maren].family = false;
    heroes[brannoc].renown = 9;
    heroes[maren].renown = 9;
    assert_eq!(
        courtship(&heroes, Some(maren), Some(brannoc)),
        Courtship::NeitherFamily
    );
}

#[test]
fn the_first_family_teller_tells_for_the_house() {
    let (_, mut heroes) = founded();
    let (odo, maren, pip) = (id(&heroes, "Odo"), id(&heroes, "Maren"), id(&heroes, "Pip"));
    heroes[odo].family = false;
    assert_eq!(
        tellers(&heroes, [Some(odo), Some(maren)]),
        [Some(Teller::Own), Some(Teller::House)]
    );
    assert_eq!(
        tellers(&heroes, [Some(pip), Some(odo)]),
        [Some(Teller::Child), Some(Teller::Own)]
    );
    assert_eq!(
        tellers(&heroes, [Some(maren), Some(odo)]),
        [Some(Teller::House), Some(Teller::Own)]
    );
}

#[test]
fn outsiders_are_never_heirs() {
    let (_, mut heroes) = founded();
    let (garrick, odo) = (id(&heroes, "Garrick"), id(&heroes, "Odo"));
    assert!(heirs(&heroes, garrick).contains(&odo));
    heroes[odo].family = false;
    assert!(!heirs(&heroes, garrick).contains(&odo));
    assert!(heirs(&heroes, odo).is_empty());
}
