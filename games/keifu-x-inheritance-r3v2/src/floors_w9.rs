//! W9's readability floors: the epitaph wherever it is set, judged by `floors.rs`'s
//! `look` at each of its sizes — a questing death's page and an old age's, every leaf; and
//! a house played twenty-four years, its family screen pointed at each of its dead and
//! departed in turn, so every epitaph a long run composes is judged on the remembrance
//! panel. (The founding's two, Elsbeth's and Aud's, are judged with the founding family.)

use jidousha::testing::FrameRecorder;

use crate::checks::Checks;
use crate::floors::{Tally, look};
use crate::house::House;
use crate::screen::Target;
use crate::verify::{SEEDS, point_at, session};

/// W9's surfaces. Returns how many were judged.
pub fn w9_surfaces(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    _label: &str,
) -> usize {
    let before = tally.surfaces;
    for (name, stage) in [
        (
            "W9, a questing death's page",
            crate::w9::stage_questing_death as fn(&mut jidousha::prelude::HeadlessSim),
        ),
        ("W9, an old age's page", crate::w9::stage_old_age),
    ] {
        let mut sim = session(SEEDS[0]);
        stage(&mut sim);
        let mut leaf = 0;
        while leaf < 40 && crate::verify::target_rect(&sim, Target::Leaf(leaf)).is_some() {
            point_at(&mut sim, Target::Leaf(leaf), true);
            look(
                checks,
                tally,
                recorder,
                &mut sim,
                &format!("{name}, leaf {}", leaf + 1),
                false,
            );
            leaf += 1;
        }
    }
    let mut sim = crate::floors_w8::grown(SEEDS[0], 24);
    point_at(&mut sim, Target::OpenFamily, true);
    let gone: Vec<usize> = {
        let house = sim.world().resource::<House>();
        (0..house.heroes.len())
            .filter(|&id| !house.heroes[id].is_living())
            .collect()
    };
    for id in gone {
        point_at(&mut sim, Target::Hero(id), false);
        let name = format!(
            "W9, year 25's family, {}'s epitaph",
            sim.world().resource::<House>().heroes[id].name
        );
        look(checks, tally, recorder, &mut sim, &name, true);
    }
    tally.surfaces - before
}
