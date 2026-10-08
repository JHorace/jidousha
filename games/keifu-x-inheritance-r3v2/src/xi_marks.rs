//! Decision row 1 (variant, DESIGN.md S8, gate G4): whether to send a family
//! member on a personal quest.
//!
//! Garrick and Brannoc seated on "Grave goods" by drag; the quest sheet, read
//! through the dock as the player reads it before "Set out", carries the stake
//! under Garrick's dream line and under no one else's. Then the same house,
//! cloned, resolved on staged dice: a Setback marks Garrick exactly as the stake
//! said, a Triumph marks no one, and the next turning charges the name a year.
//! Every expectation is a shipped literal.

use jidousha::prelude::*;
use jidousha::testing::FrameRecorder;

use crate::checks::{Checks, fail};
use crate::house::House;
use crate::screen::{Target, WINDOW};
use crate::verify::{dock_pages, dock_read, hero_named, point_at, session};
use crate::w5::recorded;

/// Garrick's call line on the sheet, and the stake that must follow it.
const GARRICK_CALL: &str = "Garrick's dream: Win a triumph at the Barrow. He must triumph.";
const GARRICK_STAKE: &str = "If Garrick fails, a black mark on the Thorne name: -2 renown to the house and to him, and -1 a year while it is carried.";
/// Ysolde's call line: she is not seated, so no stake follows it.
const YSOLDE_CALL: &str = "Ysolde's dream: Quest at three different places. She must go.";
/// The quest page's line for the failure.
const GARRICK_MARKED: &str = "Garrick went in his own name and failed. A black mark on the Thorne name: -2 renown to the house, and to Garrick.";
/// The next "Year 2 begins" page's line for one carried mark.
const ONE_MARK_A_YEAR: &str = "The Thorne name carries a black mark: -1 renown.";
/// Garrick's personal renown and the house's, at the founding.
const GARRICK_RENOWN: i32 = 6;
const HOUSE_RENOWN: i32 = 15;

/// The sheet's lines, read through the dock, with the card pointed at.
fn sheet_lines(sim: &mut HeadlessSim) -> Vec<String> {
    point_at(sim, Target::Quest(0), false);
    let pages = dock_pages(sim, &mut FrameRecorder::new(WINDOW));
    dock_read(&pages)
        .into_iter()
        .map(|line| line.text)
        .collect()
}

/// The check, on every recorded seed. Returns the summary line.
pub fn check_personal_quest(checks: &mut Checks) -> String {
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => fail("the content did not load", &error.to_string()),
    };
    for seed in recorded() {
        let mut sim = session(seed);
        let _ = crate::w4::seat_the_oracle(&mut sim);
        let sheet = sheet_lines(&mut sim);
        let call = sheet.iter().position(|line| line == GARRICK_CALL);
        checks.require(
            call.is_some_and(|at| sheet.get(at + 1).map(String::as_str) == Some(GARRICK_STAKE)),
            "row 1: the quest sheet does not state the black mark under the seated dreamer's call",
            format!("seed {seed:#x}: the sheet reads {sheet:?}"),
        );
        let ysolde = sheet.iter().position(|line| line == YSOLDE_CALL);
        checks.require(
            ysolde.is_some_and(|at| !sheet.get(at + 1).is_some_and(|l| l.starts_with("If "))),
            "row 1: a stake follows the call line of a dreamer who is not seated",
            format!("seed {seed:#x}: the sheet reads {sheet:?}"),
        );
        let (garrick, brannoc) = (hero_named(&sim, "Garrick"), hero_named(&sim, "Brannoc"));
        let house = sim.world().resource::<House>().clone();
        let rng = sim.world().resource::<Rng>().clone();

        // A Setback: power 12 against demand 9..11 with two ones.
        let mut failed = house.clone();
        let mut rolled = rng.clone();
        let page = crate::resolve::resolve_rolled(&content, &mut failed, &mut rolled, 0, [1, 1]);
        let (marks, renown) = (failed.heroes[garrick].marks, failed.heroes[garrick].renown);
        checks.require(
            page.lines.iter().any(|line| line == GARRICK_MARKED)
                && marks == 1
                && renown == GARRICK_RENOWN - 2
                && failed.renown == HOUSE_RENOWN - 2
                && failed.heroes[brannoc].marks == 0,
            "row 1: a failed personal quest did not put the stated mark on the dreamer",
            format!(
                "seed {seed:#x}: {:?}; Garrick marks {marks}, renown {renown}; house {}; Brannoc marks {}; lines {:?}",
                page.outcome, failed.renown, failed.heroes[brannoc].marks, page.lines
            ),
        );

        // A Triumph marks no one.
        let mut won = house.clone();
        let page = crate::resolve::resolve_rolled(&content, &mut won, &mut rng.clone(), 0, [6, 6]);
        checks.require(
            !page.lines.iter().any(|line| line.contains("black mark"))
                && won.heroes[garrick].marks == 0,
            "row 1: a won personal quest marked the dreamer",
            format!(
                "seed {seed:#x}: {:?}; marks {}; lines {:?}",
                page.outcome, won.heroes[garrick].marks, page.lines
            ),
        );

        // The turning charges the name a year for the mark it carries.
        failed.heroes[garrick].age = 40;
        failed.calendar.begin_winter();
        failed.open_hearth();
        let before = failed.renown;
        crate::season::let_the_winter_pass(&content, &mut failed, &mut rolled);
        crate::play::choose_every_heir(&content, &mut failed, crate::play::first_heir);
        let year = failed.passage.as_ref().and_then(|passage| {
            passage
                .pages
                .iter()
                .find(|page| page.kind == crate::passage::PageKind::Year)
                .map(|page| page.lines.clone())
        });
        checks.require(
            year.as_ref()
                .is_some_and(|lines| lines.iter().any(|line| line == ONE_MARK_A_YEAR))
                && before == HOUSE_RENOWN - 2
                && failed.renown == HOUSE_RENOWN - 3,
            "row 1: the turning did not charge the house a year for the carried mark",
            format!(
                "seed {seed:#x}: renown {before} -> {}; the year's page {year:?}",
                failed.renown
            ),
        );
    }
    format!(
        "XI row 1 (personal quest): the stake on the sheet before \"Set out\", the mark it states after a Setback, none after a Triumph, -1 a year, on {} seeds",
        recorded().len()
    )
}
