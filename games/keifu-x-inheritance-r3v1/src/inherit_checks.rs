//! The variant's decision rows, checked through the screen by the scripted pointer
//! (the spec's decision-surface table; DESIGN.md "Gates to add").
//!
//! **Row 1, the black mark.** Garrick seated alone on "Grave goods" (his dream must
//! triumph at the Barrow, so the quest is his own) with its need staged past reach:
//! the card shows the mark line before Set out; Set out; the quest page tells the
//! mark and Garrick carries exactly the disaster's two.
//! **Row 2, marrying in.** Odo staged an outsider in the garden with Maren: at renown 5
//! the garden reads "outsider: renown 5 of 6" and the winter leaves them unwed; at 6 it
//! reads "marries in: house +3", and the winter weds them, makes Odo family and tells
//! the house's gain.
//! **Row 3, the heir.** Garrick, carrying three marks, dies of old age: Pip's button
//! reads "holds Strong; marks 1"; choosing Pip leaves Pip with exactly that.
//!
//! INVARIANT: every expectation is a shipped literal — never computed by the code
//! under test — so a mutated constant or rule shows as a failure here.

use jidousha::prelude::*;

use crate::board::Slot;
use crate::checks::Checks;
use crate::genes::traits_word;
use crate::hearth::{Group, Seat};
use crate::hero::Blood;
use crate::house::House;
use crate::ids::BondKind;
use crate::screen::Target;
use crate::scripted::card_lines;
use crate::verify::{SEEDS, hero_named, page_of, point_at, session};
use crate::w4::{away, seat};
use crate::w7::{group_lines, seat_at, stay_home_into_winter, turning_lines};
use crate::w8::{go_to_the_choice, heir_holds, heir_labels, stage_garricks_winter};

/// Row 1: the card's mark line for Garrick alone on a quest he cannot meet.
pub const ORACLE_MARK_CARD: &str = "Fail 100%: mark Garrick +1/+2, house -1/-2";
/// Row 1: the quest page's line when the disaster comes.
pub const ORACLE_MARK_TOLD: &str =
    "Garrick failed a personal quest: a black mark on the name (+2), house renown -2.";
/// Row 1: the marks Garrick carries after it.
pub const ORACLE_MARKS: i32 = 2;
/// Row 2: the garden's note below and at the threshold.
pub const ORACLE_REFUSED: &str = "outsider: renown 5 of 6";
pub const ORACLE_ACCEPTED: &str = "marries in: house +3";
/// Row 2: the winter page's line on the wedding.
pub const ORACLE_MARRIED_IN: &str = "Odo marries into the name; the house gains 3 renown.";
/// Row 3: what Pip's button says he will hold, and what he holds after.
pub const ORACLE_PIP_HOLDS: &str = "holds Strong; marks 1";
pub const ORACLE_PIP_TRAITS: &str = "Strong";
pub const ORACLE_PIP_MARKS: i32 = 1;

fn house_of(sim: &HeadlessSim) -> &House {
    sim.world().resource::<House>()
}

/// Row 1's stage: Garrick alone on "Grave goods", its need past his reach.
fn stage_mark(sim: &mut HeadlessSim) {
    sim.world_mut().resource_mut::<House>().board[0]
        .quest
        .demand = 40;
    seat(sim, "Garrick", Slot::Quest { quest: 0, seat: 0 });
    away(sim);
}

/// Row 2's stage: Odo an outsider of `renown` beside Maren in the garden.
fn stage_garden(sim: &mut HeadlessSim, renown: i32) {
    stay_home_into_winter(sim);
    let odo = hero_named(sim, "Odo");
    {
        let house = sim.world_mut().resource_mut::<House>();
        house.heroes[odo].blood = Blood::Outsider;
        house.heroes[odo].renown = renown;
    }
    seat_at(sim, "Odo", Seat::Garden(0));
    seat_at(sim, "Maren", Seat::Garden(1));
    away(sim);
}

/// Row 3's stage: Garrick carrying three marks, dead of old age; his page's choice up.
fn stage_heir(sim: &mut HeadlessSim) {
    stage_garricks_winter(sim);
    let garrick = hero_named(sim, "Garrick");
    sim.world_mut().resource_mut::<House>().heroes[garrick].marks = 3;
    point_at(sim, Target::LetWinterPass, true);
    go_to_the_choice(sim);
}

/// Stage a decision surface for a picture: "mark", "garden" (refusing) or "heir".
pub fn stage(sim: &mut HeadlessSim, which: &str) {
    match which {
        "mark" => stage_mark(sim),
        "garden" => stage_garden(sim, 5),
        _ => stage_heir(sim),
    }
}

/// Row 1. Returns its summary line.
fn check_mark(checks: &mut Checks) -> String {
    let mut sim = session(SEEDS[0]);
    let garrick = hero_named(&sim, "Garrick");
    stage_mark(&mut sim);
    let card = card_lines(&sim, 0);
    checks.require(
        card.iter().any(|l| l == ORACLE_MARK_CARD),
        "row 1: the quest card does not show the mark a failure would put on the name",
        format!("card before Set out: {card:?}"),
    );
    point_at(&mut sim, Target::SetOut, true);
    let house = house_of(&sim);
    let told: Vec<String> = house
        .telling
        .as_ref()
        .map(|t| t.pages.iter().flat_map(|p| p.lines.clone()).collect())
        .unwrap_or_default();
    checks.require(
        told.iter().any(|l| l == ORACLE_MARK_TOLD),
        "row 1: the quest page does not tell the mark",
        format!("{told:?}"),
    );
    let marks = house.heroes[garrick].marks;
    checks.require(
        marks == ORACLE_MARKS,
        "row 1: the house does not carry exactly the mark the card showed",
        format!("Garrick carries {marks}, want {ORACLE_MARKS}"),
    );
    format!("inheritance row 1: card {ORACLE_MARK_CARD:?}; after Set out Garrick carries {marks}")
}

/// Row 2's stage, read: the garden's lines, and the winter let pass.
fn court(sim: &mut HeadlessSim, renown: i32) -> Vec<String> {
    stage_garden(sim, renown);
    let lines = group_lines(sim, Group::Garden);
    point_at(sim, Target::LetWinterPass, true);
    lines
}

/// Row 2. Returns its summary line.
fn check_marry_in(checks: &mut Checks) -> String {
    let mut sim = session(SEEDS[0]);
    let (odo, maren) = (hero_named(&sim, "Odo"), hero_named(&sim, "Maren"));
    let below = court(&mut sim, 5);
    let wed = |sim: &HeadlessSim| {
        house_of(sim).heroes[odo]
            .bond_to(maren)
            .is_some_and(|b| b.kind == BondKind::Spouse)
    };
    checks.require(
        below.iter().any(|l| l == ORACLE_REFUSED)
            && !wed(&sim)
            && house_of(&sim).heroes[odo].blood == Blood::Outsider,
        "row 2: an outsider below the threshold is not shown refused, or wed anyway",
        format!("garden {below:?}; wed {}", wed(&sim)),
    );
    let mut sim = session(SEEDS[0]);
    let at = court(&mut sim, 6);
    let winter = turning_lines(&sim);
    checks.require(
        at.iter().any(|l| l == ORACLE_ACCEPTED)
            && wed(&sim)
            && house_of(&sim).heroes[odo].blood == Blood::Family
            && winter.iter().any(|l| l == ORACLE_MARRIED_IN),
        "row 2: an outsider at the threshold is not shown accepted, or does not marry in",
        format!("garden {at:?}; wed {}; winter {winter:?}", wed(&sim)),
    );
    format!("inheritance row 2: {ORACLE_REFUSED:?} refused, {ORACLE_ACCEPTED:?} wed")
}

/// Row 3. Returns its summary line.
fn check_heir(checks: &mut Checks) -> String {
    let mut sim = session(SEEDS[0]);
    let pip = hero_named(&sim, "Pip");
    stage_heir(&mut sim);
    let page = page_of(&sim);
    let shown = heir_holds(&page)
        .into_iter()
        .find(|(t, _)| matches!(t, Target::Heir(_, Some(h)) if *h == pip));
    let target = heir_labels(&page)
        .into_iter()
        .map(|(t, _)| t)
        .find(|t| matches!(t, Target::Heir(_, Some(h)) if *h == pip));
    checks.require(
        shown.as_ref().and_then(|(_, s)| s.as_deref()) == Some(ORACLE_PIP_HOLDS),
        "row 3: Pip's heir button does not show what he would inherit",
        format!("{shown:?}"),
    );
    if let Some(target) = target {
        point_at(&mut sim, target, true);
    }
    let hero = &house_of(&sim).heroes[pip];
    let (traits, marks) = (traits_word(&hero.genes), hero.marks);
    checks.require(
        traits == ORACLE_PIP_TRAITS && marks == ORACLE_PIP_MARKS,
        "row 3: the heir does not hold what the button showed",
        format!("Pip holds {traits}; marks {marks}"),
    );
    format!("inheritance row 3: Pip shown {ORACLE_PIP_HOLDS:?}, holds {traits}; marks {marks}")
}

/// The three decision rows. Returns their summary lines.
pub fn check(checks: &mut Checks) -> Vec<String> {
    vec![
        check_mark(checks),
        check_marry_in(checks),
        check_heir(checks),
    ]
}
