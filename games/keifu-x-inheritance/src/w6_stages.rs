//! W6's stages — the screens the pictures and the floors are taken on — and the staged
//! forging, asked. Each stage plays the summer the way a player would: the scripted
//! pointer on the real controls.

use jidousha::prelude::*;

use crate::checks::Checks;
use crate::house::House;
use crate::screen::Target;
use crate::verify::{SEEDS, hero_named, page_of, point_at, session};
use crate::w6::wait;

/// Stage: year 1, nobody seated, "Stay home": the Meanwhile leaf.
pub fn stage_stay_home(sim: &mut HeadlessSim) {
    point_at(sim, Target::SetOut, true);
}

/// Stage: the played summer set out, its story half a second into typing.
pub fn stage_typing(sim: &mut HeadlessSim) {
    crate::w4::seat_the_oracle(sim);
    point_at(sim, Target::SetOut, true);
    wait(sim, 30);
}

/// Stage: the played summer's first page, the story completed by "Go on".
pub fn stage_page(sim: &mut HeadlessSim) {
    stage_typing(sim);
    point_at(sim, Target::GoOn, true);
}

/// Stage: a forging on the telling. Garrick carries Aud's undone "See a child grown"
/// as a burden at its last stage; Odo goes to the bell; at the quest's moment
/// Garrick's own dream does not move, so the burden does — his child is grown — and
/// it leaves the cradle-ring, Thornfall passing to Maren. The story completed, the
/// pointer rests on the line that names Garrick, so his sheet is in the dock.
pub fn stage_ring(sim: &mut HeadlessSim) {
    let (garrick, aud, odo) = (
        hero_named(sim, "Garrick"),
        hero_named(sim, "Aud"),
        hero_named(sim, "Odo"),
    );
    {
        let house = sim.world_mut().resource_mut::<House>();
        let Some(mut burden) = house.heroes[aud].dream.clone() else {
            crate::checks::fail(
                "the forging's stage needs Aud's dream",
                "household.json gives Aud none",
            );
        };
        burden.owner = Some(aud);
        house.heroes[garrick].burden = Some(burden);
    }
    crate::w4::seat(sim, "Odo", crate::board::Slot::Quest { quest: 1, seat: 0 });
    checks_odo_seated(sim, odo);
    point_at(sim, Target::SetOut, true);
    point_at(sim, Target::GoOn, true);
    point_at(sim, Target::Hero(garrick), false);
    // Down his sheet to the heirloom, a line at a time, until the ring is drawn.
    for _ in 0..80 {
        let ring = page_of(sim).figures.iter().any(|f| {
            f.figure == crate::art::Figure::Ring && crate::summer::SHEET.contains_rect(f.rect)
        });
        if ring {
            return;
        }
        crate::verify::scroll_dock(sim, -1.0);
    }
}

/// Odo must be on the bell for the stage to mean anything; a drag that did not take is
/// a broken stage, not a picture.
fn checks_odo_seated(sim: &HeadlessSim, odo: usize) {
    if sim.world().resource::<House>().party(1) != [odo] {
        crate::checks::fail(
            "the forging's stage did not seat Odo on the bell",
            "the drag did not take",
        );
    }
}

/// Stage: a house at renown 1 stays home and leaves its telling: the closed verdict.
pub fn stage_closed(sim: &mut HeadlessSim) {
    sim.world_mut().resource_mut::<House>().renown = 1;
    point_at(sim, Target::SetOut, true);
    point_at(sim, Target::GoOn, true);
}

/// The staged forging, asked: the cradle-ring left on the page, given to Garrick,
/// Thornfall passed to Maren, and the ring's sprite on his sheet in the dock.
pub fn check_forging(checks: &mut Checks) -> String {
    let mut sim = session(SEEDS[0]);
    stage_ring(&mut sim);
    let (garrick, maren) = (hero_named(&sim, "Garrick"), hero_named(&sim, "Maren"));
    let house = sim.world().resource::<House>();
    let lines = house
        .telling
        .as_ref()
        .and_then(|t| t.pages.first())
        .map(|p| p.lines.clone())
        .unwrap_or_default();
    let passed = lines.iter().position(|l| {
        l == "Garrick has two hands and one of them is full. Thornfall passes to Maren."
    });
    let forged = lines.iter().position(|l| l == "It leaves an heirloom: the Thorne cradle-ring. +2 Spirit on quests. Made by Garrick Thorne in year 1, the year a child of the house came of age.");
    let holds = |id: usize| house.heroes[id].heirloom.as_ref().map(|h| h.name.clone());
    let ring_drawn = page_of(&sim).figures.iter().any(|f| {
        f.figure == crate::art::Figure::Ring && crate::summer::SHEET.contains_rect(f.rect)
    });
    checks.require(
        matches!((passed, forged), (Some(p), Some(f)) if f == p + 1)
            && holds(garrick).as_deref() == Some("the Thorne cradle-ring")
            && holds(maren).as_deref() == Some("Thornfall")
            && ring_drawn,
        "a forging on the telling does not leave the cradle-ring, pass Thornfall on, and show the ring on the sheet",
        format!("lines {lines:?}; Garrick {:?}, Maren {:?}; ring drawn {ring_drawn}", holds(garrick), holds(maren)),
    );
    "W6 forging (staged): Garrick's carried \"See a child grown\" fulfilled at the bell's moment leaves the Thorne cradle-ring, Thornfall to Maren, the ring on his sheet".to_owned()
}
