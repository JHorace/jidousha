//! W6's readability floors: every page type the telling has, judged by `floors.rs`'s
//! `look` at each of its sizes — the Meanwhile, a story half typed, a quest page whole
//! and each of its leaves, a member's sheet beside it, a forging with the ring on the
//! sheet, a page and a Meanwhile long enough to continue on further leaves, the quiet
//! summer, and the closed house's verdict. The set-out control is on every summer
//! surface the earlier waves judge.

use jidousha::prelude::HeadlessSim;
use jidousha::testing::FrameRecorder;

use crate::checks::Checks;
use crate::floors::{Tally, look};
use crate::house::House;
use crate::screen::Target;
use crate::telling::Telling;
use crate::verify::{SEEDS, hero_named, point_at, session};
use crate::w6::telling_lines;
use crate::w6_stages::{stage_closed, stage_page, stage_ring, stage_stay_home, stage_typing};

/// Every leaf of the open telling, from the first, each judged once its story is typed.
fn every_leaf(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    sim: &mut HeadlessSim,
    name: &str,
) -> usize {
    let mut leaf = 0;
    while sim.world().resource::<House>().telling.is_some() && leaf < 40 {
        point_at(sim, Target::Leaf(leaf), true);
        // Ten seconds: any story types out whole at 90 letters a second.
        crate::w6::wait(sim, 600);
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

/// W6's surfaces. Returns how many were judged.
pub fn w6_surfaces(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    label: &str,
) -> usize {
    let before = tally.surfaces;
    let mut sim = session(SEEDS[0]);
    stage_stay_home(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W6, the stay-home Meanwhile",
        false,
    );
    let mut sim = session(SEEDS[0]);
    stage_typing(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W6, a story half typed",
        false,
    );
    let mut sim = session(SEEDS[0]);
    stage_page(&mut sim);
    let garrick = hero_named(&sim, "Garrick");
    point_at(&mut sim, Target::Hero(garrick), false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W6, the played page with Garrick's sheet",
        false,
    );
    every_leaf(checks, tally, recorder, &mut sim, "W6, the played summer");
    let mut sim = session(SEEDS[0]);
    stage_ring(&mut sim);
    // From the top of Garrick's sheet: `look` pages it down and finds the ring itself.
    crate::verify::scroll_dock(&mut sim, 100.0);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W6, a forging, the ring on the sheet",
        false,
    );
    // A page and a Meanwhile long enough to run on: the played page with its lines
    // told thirty times over — long enough whatever the summer's dice gave it (a success
    // writes one line) — and the Meanwhile's eight times.
    let mut sim = session(SEEDS[0]);
    stage_page(&mut sim);
    {
        let house = sim.world_mut().resource_mut::<House>();
        if let Some(telling) = house.telling.as_mut() {
            let page = &mut telling.pages[0];
            page.lines = (0..30).flat_map(|_| page.lines.clone()).collect();
            telling.meanwhile = (0..8).flat_map(|_| telling.meanwhile.clone()).collect();
        }
    }
    let leaves = every_leaf(checks, tally, recorder, &mut sim, "W6, a long telling");
    point_at(&mut sim, Target::Leaf(1), true);
    let more = telling_lines(&sim);
    checks.require(
        leaves >= 4 && more.first().map(String::as_str) == Some("What it did to them, continued."),
        "a page longer than a leaf does not continue under \"What it did to them, continued.\"",
        format!("{label}: {leaves} leaves; leaf 2 reads {more:?}"),
    );
    point_at(&mut sim, Target::Leaf(leaves - 1), true);
    let last = telling_lines(&sim);
    checks.require(
        last.first().map(String::as_str) == Some("Meanwhile, continued"),
        "a Meanwhile longer than a leaf does not continue under \"Meanwhile, continued\"",
        format!("{label}: the last leaf reads {last:?}"),
    );
    // The quiet summer: nothing at all.
    let mut sim = session(SEEDS[0]);
    sim.world_mut().resource_mut::<House>().telling = Some(Telling {
        year: 1,
        ..Telling::default()
    });
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W6, a quiet summer",
        false,
    );
    checks.require(
        telling_lines(&sim)
            == [
                "A quiet summer",
                "No one left the house, and nothing was asked of it.",
            ],
        "an empty telling does not read \"A quiet summer\"",
        format!("{label}: {:?}", telling_lines(&sim)),
    );
    let mut sim = session(SEEDS[0]);
    stage_closed(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W6, the closed house",
        false,
    );
    tally.surfaces - before
}
