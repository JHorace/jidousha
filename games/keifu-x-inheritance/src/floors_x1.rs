//! The variant's readability floors (VARIANT.md): the surfaces it adds, judged by
//! `floors.rs`'s `look` at each of its sizes — a personal quest's card with its sheet read
//! whole through the dock; the garden refusing an unproven outsider; Garrick's marked death
//! page at the choice, each candidate's traits and marks above the buttons; a marked,
//! traited hero's sheet; and an outsider's sheet.

use jidousha::testing::FrameRecorder;

use crate::board::Slot;
use crate::checks::Checks;
use crate::floors::{Tally, look};
use crate::hearth::Seat;
use crate::house::House;
use crate::screen::Target;
use crate::verify::{SEEDS, hero_named, point_at, session};
use crate::w7::{seat_at, stay_home_into_winter};
use crate::x1::{marked_choice, seat_hero, stage_outsider};

/// The variant's surfaces. Returns how many were judged.
pub fn x1_surfaces(checks: &mut Checks, tally: &mut Tally, recorder: &mut FrameRecorder) -> usize {
    let before = tally.surfaces;
    // The personal quest: Garrick and Brannoc on Grave goods, the card pointed at.
    let mut sim = session(SEEDS[0]);
    crate::w4::seat(&mut sim, "Garrick", Slot::Quest { quest: 0, seat: 0 });
    crate::w4::seat(&mut sim, "Brannoc", Slot::Quest { quest: 0, seat: 1 });
    point_at(&mut sim, Target::Quest(0), false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "X1, a personal quest's card and its sheet",
        false,
    );
    // The garden refusing an unproven outsider.
    let mut sim = session(SEEDS[0]);
    stay_home_into_winter(&mut sim);
    let outsider = stage_outsider(&mut sim, 4);
    seat_at(&mut sim, "Ysolde", Seat::Garden(0));
    seat_hero(&mut sim, outsider, Seat::Garden(1));
    point_at(&mut sim, Target::Hero(outsider), false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "X1, the garden with an unproven outsider, and their sheet",
        false,
    );
    // Garrick's death page at the choice, with the candidates' lines.
    let mut sim = marked_choice(false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "X1, Garrick's marked death page at the choice",
        false,
    );
    // A marked, traited hero's sheet: Maren, after taking the shame.
    let maren = hero_named(&sim, "Maren");
    if let Some((target, _)) = crate::w8::heir_labels(&crate::verify::page_of(&sim))
        .into_iter()
        .next()
    {
        point_at(&mut sim, target, true);
    }
    point_at(&mut sim, Target::Hero(maren), false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "X1, Maren's sheet: a trait and a mark beside the heirs",
        false,
    );
    {
        let house = sim.world().resource::<House>();
        checks.require(
            !house.heroes[maren].marks.is_empty() || !house.heroes[maren].traits.is_empty(),
            "the sheet floor was judged on a hero who carries neither a trait nor a mark",
            String::new(),
        );
    }
    tally.surfaces - before
}
