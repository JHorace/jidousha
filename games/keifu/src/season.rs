//! Leaving the telling (SPEC §2.1, §8): the one place the house can close, and the
//! year moving on.
//!
//! W7/W8 SCAFFOLD: winter (W7) and the turning of the year (W8) do not exist yet. Until
//! they land, leaving the telling of an open house passes the season with nothing but
//! what the calendar already does — winter begins and ends, the year counts on, the
//! next summer is prepared (its board generated, the household reseated). Nobody
//! ages, rests, learns, dies of age, is born or arrives; `House::mourned` keeps its
//! dead for W8's death pages. Trouble's changes are W6's own (§7.2) and stand. W7 and
//! W8 replace `pass_the_year` whole.

use jidousha::prelude::Rng;

use crate::content::Content;
use crate::house::House;

/// Leave the telling (SPEC §2.1, `scene/scenes/telling.jai:184-196`): at renown 0 the
/// house closes (the Ending, W10's — its scaffold screen is `ending_view.rs`);
/// otherwise the year moves on.
pub fn leave_the_telling(content: &Content, house: &mut House, rng: &mut Rng) {
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
    pass_the_year(content, house, rng);
}

/// W7/W8 SCAFFOLD: the winter and the turning, as the calendar alone.
fn pass_the_year(content: &Content, house: &mut House, rng: &mut Rng) {
    house.calendar.begin_winter();
    house.calendar.begin_summer();
    house.prepare_summer(content, rng);
}
