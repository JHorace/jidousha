//! Playing through the turning in the checks: choosing every heir a turning waits for,
//! by a policy, off the house (the batteries) or through the screen (the played checks).
//!
//! Only the checks choose this way; the player chooses on the death page. Each choice
//! goes through `heirs::choose`, the one function the heir buttons press.

use jidousha::prelude::HeadlessSim;

use crate::content::Content;
use crate::hero::HeroId;
use crate::house::House;
use crate::passage::Bequest;
use crate::screen::Target;
use crate::verify::{page_of, point_at};

/// Choose on every undecided death page, in page order, the heir `pick` names.
pub fn choose_every_heir(
    content: &Content,
    house: &mut House,
    pick: impl Fn(&House, &Bequest) -> Option<HeroId>,
) {
    while let Some(at) = house.passage.as_ref().and_then(|p| p.first_undecided()) {
        let Some(bequest) = house
            .passage
            .as_ref()
            .and_then(|p| p.pages[at].bequest.clone())
        else {
            return;
        };
        let heir = pick(house, &bequest);
        crate::heirs::choose(content, house, at, heir);
    }
}

/// The first heir offered.
pub fn first_heir(_: &House, bequest: &Bequest) -> Option<HeroId> {
    bequest.heirs.first().copied()
}

/// On the screen: the first heir button the page shows, if a choice is up.
pub fn heir_button(sim: &HeadlessSim) -> Option<Target> {
    let targets = page_of(sim).targets;
    let first = |some: bool| {
        targets
            .iter()
            .map(|(_, t)| *t)
            .find(|t| matches!(t, Target::Heir(_, h) if h.is_some() == some))
    };
    first(true).or_else(|| first(false))
}

/// Read the open turning to its end through the screen: "Go on", choosing the first heir
/// on each death page that waits, until summer comes.
pub fn read_to_summer(sim: &mut HeadlessSim) {
    let mut guard = 0;
    while sim.world().resource::<House>().passage.is_some() && guard < 200 {
        match heir_button(sim) {
            Some(heir) => point_at(sim, heir, true),
            None => point_at(sim, Target::GoOn, true),
        }
        guard += 1;
    }
}
