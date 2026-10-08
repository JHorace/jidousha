//! The seasons turning (SPEC §2.1): leaving the telling — the one place the house can
//! close — into winter, letting the winter pass, and summer coming.
//!
//! Leaving the telling of an open house begins winter and opens the hearth (§11.1);
//! leaving the Door's, or a spent house's, enters the Ending (§16.3, §23).
//! "Let the winter pass" resolves the winter (§11.3, `winter.rs`) and then turns the
//! year (§18, `turning.rs`); "Summer comes" advances the calendar and prepares the next
//! summer — and refuses while a death page still waits for its heir (§15.2, §18.1).

use jidousha::prelude::Rng;

use crate::content::Content;
use crate::ending::enter_the_ending;
use crate::house::House;
use crate::turning::turn_the_year;
use crate::winter::resolve_winter;

/// Leave the telling (SPEC §2.1, `scene/scenes/telling.jai:184-196`): after the Door the
/// Ending, whatever the renown (§16.3); at renown 0 the house closes into the Ending;
/// otherwise winter begins and the hearth is seated. The Ending remembers the fallen and
/// the living as it is entered (§23, `ending.rs`), drawing their wordings from `rng`.
pub fn leave_the_telling(content: &Content, house: &mut House, rng: &mut Rng) {
    assert!(
        house.telling.is_some(),
        "[keifu_x_inheritance_r2] the telling was left with no telling open\n  likely cause: the leave \
         control was offered off the telling screen\n  fix: offer it only on its last leaf"
    );
    let door = house.telling.take().and_then(|telling| telling.door);
    if let Some(door) = door {
        enter_the_ending(content, house, rng, Some(&door));
        return;
    }
    if house.renown <= 0 {
        house.closed = true;
        enter_the_ending(content, house, rng, None);
        return;
    }
    house.calendar.begin_winter();
    house.open_hearth();
}

/// Whether the hearth is up: winter, no turning open, the house not closed.
pub fn at_the_hearth(house: &House) -> bool {
    house.calendar.is_winter()
        && house.passage.is_none()
        && house.telling.is_none()
        && !house.closed
        && house.ending.is_none()
}

/// "Let the winter pass" (SPEC §2.1, `scene/scenes/winter.jai:39-42`): resolve the
/// winter, then turn the year.
pub fn let_the_winter_pass(content: &Content, house: &mut House, rng: &mut Rng) {
    assert!(
        at_the_hearth(house),
        "[keifu_x_inheritance_r2] the winter was let pass away from the hearth\n  likely cause: the \
         control was offered off the winter screen\n  fix: offer it only at the hearth"
    );
    let (winter, done) = resolve_winter(content, house, rng);
    turn_the_year(content, house, winter, done, rng);
}

/// Whether the year may turn: a turning is open and no death page waits for its heir
/// (SPEC §15.2: "The turning cannot proceed past an undecided page").
pub fn may_turn(house: &House) -> bool {
    house
        .passage
        .as_ref()
        .is_some_and(|passage| passage.first_undecided().is_none())
}

/// "Summer comes" (SPEC §2.1, `scene/scenes/turning.jai:45-74`): the turning closes,
/// the calendar moves to next summer, and the board is prepared (§5.1). Refused —
/// loudly, since the screen never offers it — while a death page is undecided.
pub fn summer_comes(content: &Content, house: &mut House, rng: &mut Rng) {
    assert!(
        house.passage.is_some(),
        "[keifu_x_inheritance_r2] summer was brought with no turning open\n  likely cause: the control \
         was offered off the turning screen\n  fix: offer it only on its last leaf"
    );
    assert!(
        may_turn(house),
        "[keifu_x_inheritance_r2] summer was brought with a death page undecided\n  likely cause: the \
         control was offered before every heir was chosen\n  fix: SPEC §18.1 — the year \
         does not turn until each death page has its choice"
    );
    house.passage = None;
    house.calendar.begin_summer();
    house.prepare_summer(content, rng);
}
