//! The telling's pure readings: the roll line's three margin tellings, and the hero a
//! line is about. Shipped literals from `ui-text.json`.

use jidousha::prelude::Rng;

use crate::resolve::resolve_rolled;
use crate::telling_view::{about, roll_line};
use crate::testkit::{aim, house_without_traits, id, seat};

#[test]
fn the_roll_line_tells_an_exact_a_beaten_and_a_missed_margin() {
    let (content, founded) = house_without_traits();
    let brannoc = id(&founded.heroes, "Brannoc");
    for (margin, telling) in [
        (0, "met it exactly"),
        (3, "beat it by 3"),
        (-2, "missed by 2"),
    ] {
        let mut house = founded.clone();
        seat(&mut house, 0, &[brannoc]);
        aim(&mut house, 0, [4, 5], margin);
        let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(50), 0, [4, 5]);
        let demand = page.quest.demand;
        assert_eq!(
            roll_line(&content, &page),
            format!("Needed Might {demand}. Brought 6. Dice 4 and 5, less 7: {telling}.")
        );
    }
}

#[test]
fn a_line_is_about_the_first_hero_it_names_as_a_word() {
    let (_, heroes) = crate::testkit::founded();
    assert_eq!(
        about(&heroes, "Garrick saw his daughter in the worst of it."),
        Some(id(&heroes, "Garrick"))
    );
    assert_eq!(
        about(&heroes, "Maren lays Thornfall aside, for Garrick."),
        Some(id(&heroes, "Maren"))
    );
    assert_eq!(
        about(&heroes, "The house is thought less of for it: -4 renown."),
        None
    );
}
