//! W8's readability floors: the turning's new pages and the grown house, judged by
//! `floors.rs`'s `look` at each of its sizes — Garrick's death page waiting for its heir,
//! with an heir's sheet in the dock; the stirred page's eight heirs and "No one"; every
//! leaf of a turned year with a page of every kind; a death page long enough to continue
//! with its choice on the leaf after; and a house grown over fifteen played years — its
//! summer, its family, and the sheets of a newborn and a wanderer — and grown over
//! twenty-four, its year-25 summer and its family at its fullest.

use jidousha::prelude::{HeadlessSim, Rng};
use jidousha::testing::FrameRecorder;

use crate::checks::Checks;
use crate::floors::{Tally, look};
use crate::hero::DeedKind;
use crate::house::House;
use crate::screen::{Target, UiState};
use crate::verify::{SEEDS, page_of, point_at, session, set_ui};
use crate::w8::{go_to_the_choice, heir_labels, stage_garricks_winter, stage_turned_year, stir};

/// Every leaf of the open turning, each judged; returns how many.
fn every_leaf(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    sim: &mut HeadlessSim,
    name: &str,
) -> usize {
    let mut leaf = 0;
    while leaf < 40 && crate::verify::target_rect(sim, Target::Leaf(leaf)).is_some() {
        point_at(sim, Target::Leaf(leaf), true);
        look(
            checks,
            tally,
            recorder,
            sim,
            &format!("{name}, leaf {}", leaf + 1),
            false,
        );
        leaf += 1;
    }
    leaf
}

/// W8's surfaces. Returns how many were judged.
pub fn w8_surfaces(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    label: &str,
) -> usize {
    let before = tally.surfaces;
    let mut sim = session(SEEDS[0]);
    stage_garricks_winter(&mut sim);
    point_at(&mut sim, Target::LetWinterPass, true);
    go_to_the_choice(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W8, Garrick's death page",
        false,
    );
    if let Some((target, _)) = heir_labels(&page_of(&sim)).first().cloned() {
        point_at(&mut sim, target, false);
        look(
            checks,
            tally,
            recorder,
            &mut sim,
            "W8, Garrick's page, Maren's sheet beside it",
            false,
        );
    }
    let mut sim = session(SEEDS[0]);
    stage_garricks_winter(&mut sim);
    let _ = stir(&mut sim, true);
    point_at(&mut sim, Target::LetWinterPass, true);
    go_to_the_choice(&mut sim);
    let heirs = heir_labels(&page_of(&sim)).len();
    // The variant's (rewritten from mainline's eight): the wanderer is an outsider and no
    // heir, so six heirs and no one.
    checks.require(
        heirs == 7,
        "the stirred page does not show six heirs and no one",
        format!("{label}: {heirs} buttons"),
    );
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W8, the stirred page, eight buttons",
        false,
    );
    // A death page long enough to continue: its lines told six times over.
    {
        let house = sim.world_mut().resource_mut::<House>();
        if let Some(page) = house.passage.as_mut().and_then(|p| p.pages.get_mut(1)) {
            page.lines = (0..6).flat_map(|_| page.lines.clone()).collect();
        }
    }
    set_ui(&mut sim, UiState::default());
    let leaves = every_leaf(checks, tally, recorder, &mut sim, "W8, a long death page");
    checks.require(
        leaves >= 3,
        "a long death page does not continue",
        format!("{label}: {leaves} leaves"),
    );
    let mut sim = session(SEEDS[0]);
    stage_turned_year(&mut sim);
    let leaves = every_leaf(checks, tally, recorder, &mut sim, "W8, a turned year");
    checks.require(
        leaves >= 6,
        "the turned year does not show a page of every kind",
        format!("{label}: {leaves} leaves"),
    );
    // A house grown over fifteen played years.
    let mut sim = grown(SEEDS[0], 15);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W8, a grown house's summer",
        false,
    );
    let (newborn, wanderer) = {
        let house = sim.world().resource::<House>();
        let living = |pred: &dyn Fn(&crate::hero::Hero) -> bool| {
            (0..house.heroes.len())
                .find(|&id| house.heroes[id].is_living() && pred(&house.heroes[id]))
        };
        (
            living(&|h| h.born_year >= 1),
            living(&|h| h.deeds.iter().any(|d| d.kind == DeedKind::Arrived)),
        )
    };
    for (who, id) in [("a newborn", newborn), ("a wanderer", wanderer)] {
        if let Some(id) = id {
            point_at(&mut sim, Target::Hero(id), false);
            look(
                checks,
                tally,
                recorder,
                &mut sim,
                &format!("W8, a grown house, {who}'s sheet"),
                false,
            );
        }
    }
    point_at(&mut sim, Target::OpenFamily, true);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W8, a grown house's family",
        true,
    );
    // Twenty-four years on: the tree at its fullest, and the summer before the Door.
    let mut sim = grown(SEEDS[0], 24);
    let heroes = sim.world().resource::<House>().heroes.len();
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W8, year 25's summer",
        false,
    );
    point_at(&mut sim, Target::OpenFamily, true);
    let name = format!("W8, year 25's family of {heroes}");
    look(checks, tally, recorder, &mut sim, &name, true);
    tally.surfaces - before
}

/// A session whose house has been played `years` years on, in its summer.
pub fn grown(seed: u64, years: i32) -> HeadlessSim {
    let mut sim = session(seed);
    let content = crate::verify::content_of(&sim);
    let mut house = sim.world().resource::<House>().clone();
    let mut rng = Rng::from_seed(seed);
    crate::play::play_years(content, &mut house, &mut rng, years);
    sim.world_mut().insert_resource(house);
    sim.world_mut().insert_resource(rng);
    sim.tick();
    sim
}
