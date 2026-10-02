//! The seasons turning (SPEC §2.1): leaving the telling — the one place the house can
//! close — into winter, letting the winter pass, and summer coming.
//!
//! Leaving the telling of an open house begins winter and opens the hearth (§11.1).
//! "Let the winter pass" resolves the winter (§11.3, `winter.rs`) and then turns the
//! year; "Summer comes" advances the calendar and prepares the next summer.
//!
//! W8 SCAFFOLD: the turning of the year (§18) is W8's. Its winter half is real —
//! `let_the_winter_pass` resolves the hearth and writes the winter page — and its
//! turning half is still the calendar alone: nobody ages, dies of age, is born, comes of
//! age or arrives, and `House::mourned` keeps its dead for W8's death pages. Trouble's
//! changes are W6's own (§7.2) and stand. W8 replaces `turn_the_year` whole.

use jidousha::prelude::Rng;

use crate::content::Content;
use crate::house::House;
use crate::passage::Passage;
use crate::winter::resolve_winter;

/// Leave the telling (SPEC §2.1, `scene/scenes/telling.jai:184-196`): at renown 0 the
/// house closes (the Ending, W10's — its scaffold screen is `ending_view.rs`);
/// otherwise winter begins and the hearth is seated.
pub fn leave_the_telling(house: &mut House) {
    assert!(
        house.telling.is_some(),
        "[keifu] the telling was left with no telling open\n  likely cause: the leave \
         control was offered off the telling screen\n  fix: offer it only on its last leaf"
    );
    house.telling = None;
    if house.renown <= 0 {
        house.closed = true;
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
}

/// "Let the winter pass" (SPEC §2.1, `scene/scenes/winter.jai:39-42`): resolve the
/// winter, then turn the year.
pub fn let_the_winter_pass(content: &Content, house: &mut House, rng: &mut Rng) {
    assert!(
        at_the_hearth(house),
        "[keifu] the winter was let pass away from the hearth\n  likely cause: the \
         control was offered off the winter screen\n  fix: offer it only at the hearth"
    );
    let (winter, done) = resolve_winter(content, house, rng);
    turn_the_year(house, winter, done);
}

/// W8 SCAFFOLD: the turning (SPEC §18) as its winter page alone (step 1).
fn turn_the_year(house: &mut House, winter: Vec<String>, done: crate::winter::WinterPlan) {
    house.passage = Some(Passage {
        year: house.calendar.current_year(),
        winter,
        done,
    });
}

/// "Summer comes" (SPEC §2.1, `scene/scenes/turning.jai:45-74`): the turning closes,
/// the calendar moves to next summer, and the board is prepared (§5.1).
pub fn summer_comes(content: &Content, house: &mut House, rng: &mut Rng) {
    assert!(
        house.passage.is_some(),
        "[keifu] summer was brought with no turning open\n  likely cause: the control \
         was offered off the turning screen\n  fix: offer it only on its last leaf"
    );
    house.passage = None;
    house.calendar.begin_summer();
    house.prepare_summer(content, rng);
}
