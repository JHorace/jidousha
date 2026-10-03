//! W7's readability floors: the hearth and the turning's winter page, judged by
//! `floors.rs`'s `look` at each of its sizes — the hearth idle with its help in the dock,
//! each group's help, each hero's sheet beside it, the oracle mid-drag, the played
//! winter seated, stirred hearths whose every seat holds a long excuse, the winter page
//! (the oracle's, with Pip's sheet beside it), the played winter's page and Ysolde's
//! sheet with the road-book, a quiet winter, and a winter page long enough to continue.

use jidousha::prelude::{HeadlessSim, Rng};
use jidousha::testing::FrameRecorder;

use crate::checks::Checks;
use crate::floors::{Tally, look, look_held};
use crate::hearth::Group;
use crate::house::House;
use crate::screen::{Target, UiState};
use crate::scripted::lines_in;
use crate::summer::SHEET;
use crate::verify::{SEEDS, hero_named, page_of, point_at, session};
use crate::w7::{stage_mid_drag, stage_oracle_winter, stage_played_winter, stay_home_into_winter};

/// Every leaf of the open turning, each judged.
fn every_leaf(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    sim: &mut HeadlessSim,
    name: &str,
) -> usize {
    let mut leaf = 0;
    while sim.world().resource::<House>().passage.is_some() && leaf < 20 {
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
        if crate::verify::target_rect(sim, Target::Leaf(leaf)).is_none() {
            break;
        }
    }
    leaf
}

/// W7's surfaces. Returns how many were judged.
pub fn w7_surfaces(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    label: &str,
) -> usize {
    let before = tally.surfaces;
    let mut sim = session(SEEDS[0]);
    stay_home_into_winter(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W7, the hearth, nobody pointed at",
        false,
    );
    let help = crate::verify::content_of(&sim).words[crate::words::W::WinterHelp].to_owned();
    let idle = lines_in(&page_of(&sim), SHEET);
    checks.require(
        idle == [help],
        "the idle dock on the hearth does not hold the winter's help",
        format!("{label}: the dock reads {idle:?}"),
    );
    for group in [
        Group::Hall,
        Group::Fire,
        Group::Training,
        Group::Garden,
        Group::Table,
        Group::Benches,
    ] {
        crate::w7_controls::point_at_group(&mut sim, group);
        let ui = *sim.world().resource::<UiState>();
        let dock = lines_in(&page_of(&sim), SHEET);
        checks.require(
            ui.pointing_group == Some(group) && dock.len() == 2,
            "pointing at a group of winter seats does not open its help in the dock",
            format!(
                "{label}, {group:?}: pointing {:?}, the dock reads {dock:?}",
                ui.pointing_group
            ),
        );
        look(
            checks,
            tally,
            recorder,
            &mut sim,
            &format!("W7, the {group:?}'s help"),
            false,
        );
    }
    let everyone: Vec<usize> = {
        let house = sim.world().resource::<House>();
        (0..house.heroes.len())
            .filter(|&h| house.heroes[h].is_living())
            .collect()
    };
    for id in &everyone {
        point_at(&mut sim, Target::Hero(*id), false);
        let name = format!(
            "W7, the hearth, {}'s sheet",
            sim.world().resource::<House>().heroes[*id].name
        );
        look(checks, tally, recorder, &mut sim, &name, false);
    }
    let mut sim = session(SEEDS[0]);
    let _held = stage_mid_drag(&mut sim);
    mid_drag(checks, &sim, label);
    look_held(
        checks,
        tally,
        recorder,
        &mut sim,
        "W7, Odo in hand over Pip's bench",
    );
    // Stirred hearths: every seat held, long excuses among the notes.
    for case in 0..6u64 {
        let mut sim = session(SEEDS[0]);
        stay_home_into_winter(&mut sim);
        let mut rng = Rng::from_seed(0x7_9000 + case);
        stir_full(sim.world_mut().resource_mut::<House>(), &mut rng);
        look(
            checks,
            tally,
            recorder,
            &mut sim,
            &format!("W7, a stirred hearth {case}"),
            false,
        );
    }
    // The turning's winter page.
    let mut sim = session(SEEDS[0]);
    stage_oracle_winter(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W7, the oracle's winter page",
        false,
    );
    let pip = hero_named(&sim, "Pip");
    point_at(&mut sim, Target::Hero(pip), false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W7, the winter page with Pip's sheet",
        false,
    );
    let mut sim = session(SEEDS[0]);
    let _ = stage_played_winter(&mut sim);
    let ysolde = hero_named(&sim, "Ysolde");
    point_at(&mut sim, Target::Leaf(0), true);
    point_at(&mut sim, Target::Hero(ysolde), false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W7, the played winter with Ysolde's road-book",
        false,
    );
    every_leaf(checks, tally, recorder, &mut sim, "W7, the played winter");
    let mut sim = session(SEEDS[0]);
    stay_home_into_winter(&mut sim);
    point_at(&mut sim, Target::LetWinterPass, true);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W7, a quiet winter",
        false,
    );
    let mut sim = session(SEEDS[0]);
    stage_oracle_winter(&mut sim);
    if let Some(passage) = sim.world_mut().resource_mut::<House>().passage.as_mut() {
        passage.pages[0].lines = (0..8)
            .flat_map(|_| passage.pages[0].lines.clone())
            .collect();
    }
    let leaves = every_leaf(checks, tally, recorder, &mut sim, "W7, a long winter");
    checks.require(
        leaves >= 2,
        "a long winter page does not run on to a second leaf",
        format!("{label}: {leaves} leaves"),
    );
    tally.surfaces - before
}

/// Seat every living hero across the twelve winter seats (the yard's children first
/// into the child seats), so every group's notes are drawn at once.
fn stir_full(house: &mut House, rng: &mut Rng) {
    use crate::hearth::Seat;
    let living: Vec<usize> = (0..house.heroes.len())
        .filter(|&h| house.heroes[h].is_living())
        .collect();
    for &h in &living {
        house.heroes[h].wounded = crate::chance::chance(rng, 0.4);
        house.heroes[h].fear.dread = crate::chance::between(rng, 0, 4);
    }
    house.roster = [None; crate::constants::ROSTER_SEATS];
    house.hearth.clear();
    let mut seats: Vec<Seat> = Seat::all()
        .into_iter()
        .filter(|s| !matches!(s, Seat::Yard(_)))
        .collect();
    for &h in &living {
        if seats.is_empty() {
            break;
        }
        let seat = seats.remove(crate::chance::index(rng, seats.len()));
        house.hearth.put(seat, Some(h));
    }
}

/// Mid-drag on the hearth: the hand, the bench's preview beside it, the hall it was
/// lifted from, all clear of the dock — which holds the held hero's sheet.
fn mid_drag(checks: &mut Checks, sim: &HeadlessSim, label: &str) {
    let page = page_of(sim);
    let hand = page
        .shapes
        .iter()
        .find(|s| s.layer == crate::screen::layers::HAND)
        .map(|s| s.rect);
    let benches = crate::hearth_view::group_rect(Group::Benches);
    let preview = page
        .rows
        .iter()
        .find(|r| r.panel == benches && r.text == crate::w7::ORACLE_PREVIEW)
        .map(|r| r.bounds());
    let clear = |rect: Option<jidousha::prelude::Rect>| rect.is_some_and(|r| !r.overlaps(SHEET));
    let sheet = lines_in(&page, SHEET);
    checks.require(
        clear(hand) && clear(preview) && sheet.first().map(String::as_str) == Some("Odo Fenn"),
        "mid-drag on the hearth, the hand or the bench's preview is missing or under the dock, or the dock does not hold Odo's sheet",
        format!("{label}: hand {hand:?}, preview {preview:?}, dock {:?}", sheet.first()),
    );
}
