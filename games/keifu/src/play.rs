//! Playing in the checks: choosing every heir a turning waits for, by a policy, off the
//! house (the batteries) or through the screen (the played checks); and the whole-year
//! battery's way of playing a year — the answerable quests, a wedding and a teller.
//!
//! Only the checks choose this way; the player chooses on the death page. Each choice
//! goes through `heirs::choose`, the one function the heir buttons press.

use jidousha::prelude::{HeadlessSim, Rng};

use crate::board::Posted;
use crate::content::Content;
use crate::hearth::Seat;
use crate::hero::HeroId;
use crate::house::House;
use crate::passage::Bequest;
use crate::plans::{Courtship, courtship};
use crate::reading::{adults, answerable, likely_party, success_ways};
use crate::resolve::set_out;
use crate::screen::Target;
use crate::season::{leave_the_telling, let_the_winter_pass, summer_comes};
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
        let still = house.passage.as_ref().and_then(|p| p.first_undecided());
        if still == Some(at) {
            crate::checks::fail(
                "a death page was chosen on and still waits",
                &format!("turning page {at}: heirs::choose did not record the choice"),
            );
        }
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

/// Seat every quest the house can answer (its likely party's success at least one half,
/// 18 of 36), from the adults no earlier quest took; leave the rest unanswered.
pub fn seat_answerable(house: &mut House) {
    let mut free = adults(&house.heroes);
    for slot in 0..house.board.len() {
        let quest = house.board[slot].quest.clone();
        let party = likely_party(&house.heroes, &free, &quest, None);
        if !answerable(success_ways(&house.heroes, &party, &quest, house.patrons)) {
            continue;
        }
        free.retain(|id| !party.contains(id));
        for &hero in &party {
            house.unseat(hero);
        }
        let Posted { seats, .. } = &mut house.board[slot];
        for (at, &hero) in party.iter().enumerate() {
            seats[at] = Some(hero);
        }
    }
}

/// The first pair in the hall who would wed, to the garden; the first adult left, to the
/// long table.
pub fn seat_the_winter(house: &mut House) {
    let hall: Vec<HeroId> = house.roster.iter().flatten().copied().collect();
    'pairs: for (i, &a) in hall.iter().enumerate() {
        for &b in &hall[i + 1..] {
            if courtship(&house.heroes, Some(a), Some(b)) == Courtship::WillWed {
                house.unseat(a);
                house.unseat(b);
                house.hearth.put(Seat::Garden(0), Some(a));
                house.hearth.put(Seat::Garden(1), Some(b));
                break 'pairs;
            }
        }
    }
    if let Some(&teller) = house.roster.iter().flatten().next() {
        house.unseat(teller);
        house.hearth.put(Seat::Table(0), Some(teller));
    }
}

/// Play `house` on for `years` years as the battery does — the answerable quests, a
/// wedding and a teller each winter, every heir by the rotation — stopping in a summer.
/// For staging a grown house; the checks are the battery's own.
pub fn play_years(content: &Content, house: &mut House, rng: &mut Rng, years: i32) {
    let seed = house.seed;
    for _ in 0..years {
        seat_answerable(house);
        set_out(content, house, rng);
        leave_the_telling(house);
        if house.closed {
            return;
        }
        seat_the_winter(house);
        let_the_winter_pass(content, house, rng);
        choose_every_heir(content, house, rotating(seed));
        summer_comes(content, house, rng);
    }
}

/// The heir the battery chooses: by a rotation over the list and "no one".
pub fn rotating(seed: u64) -> impl Fn(&House, &Bequest) -> Option<HeroId> {
    move |house, bequest| {
        let n = bequest.heirs.len() + 1;
        let at = (seed as usize + bequest.dead + house.calendar.current_year() as usize) % n;
        bequest.heirs.get(at).copied()
    }
}
