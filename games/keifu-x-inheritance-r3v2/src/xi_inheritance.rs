//! Decision row 3 (variant, DESIGN.md S8, gates G8 and G9): which heir to choose,
//! given what they inherit — and the lean a child is born with.
//!
//! Garrick dies at the winter's turning carrying black marks. His death page says
//! how many and how many an heir would take; pointing at Maren's button shows her
//! sheet — her lean, her fear, her marks — and choosing her gives her exactly what
//! the page and the sheet said. One mark is buried with him; with nothing else to
//! leave, two marks alone hold the page, and "No one" buries them. Then a birth:
//! the page names whom the child takes after, and the child leans that way.

use jidousha::prelude::*;
use jidousha::testing::FrameRecorder;

use crate::checks::{Checks, fail};
use crate::hero::HeroId;
use crate::house::House;
use crate::ids::{Aptitude, Tag};
use crate::inheritance::Lean;
use crate::passage::PageKind;
use crate::screen::{Target, WINDOW};
use crate::verify::{dock_pages, dock_read, hero_named, page_of, point_at, session};
use crate::w5::recorded;
use crate::w8::{ORACLE_HEIRS, go_to_the_choice, heir_labels, page_leaves, stage_garricks_winter};

/// Garrick's death page, for three marks and for one.
const THREE_MARKS: &str = "He carried 3 black marks on the Thorne name. An heir takes 1.";
const ONE_MARK: &str = "He carried one black mark on the Thorne name. It is buried with him.";
/// What choosing Maren, or no one, tells.
const MAREN_TAKES: &str = "Maren carries one of Garrick's black marks now.";
const INTO_THE_GROUND: &str = "Garrick's black marks go into the ground with him.";
/// What Maren's sheet shows of what she has, at the moment of choosing.
const MAREN_LEAN: &str = "Leans to Wits.";

/// Garrick's death page reached with `marks` on him; `bare` takes his heirloom
/// and his dream first, so only the marks can hold the page.
fn death_page(seed: u64, marks: i32, bare: bool) -> (HeadlessSim, HeroId) {
    let mut sim = session(seed);
    let garrick = stage_marked_death(&mut sim, marks, bare);
    (sim, garrick)
}

/// Garrick aged to his death at the winter with `marks` on him (and, if `bare`,
/// neither heirloom nor dream), the winter let pass, at the heir choice. Returns
/// Garrick.
pub fn stage_marked_death(sim: &mut HeadlessSim, marks: i32, bare: bool) -> HeroId {
    stage_garricks_winter(sim);
    let garrick = hero_named(sim, "Garrick");
    {
        let house = sim.world_mut().resource_mut::<House>();
        house.heroes[garrick].marks = marks;
        if bare {
            house.heroes[garrick].heirloom = None;
            house.heroes[garrick].dream = None;
        }
    }
    point_at(sim, Target::LetWinterPass, true);
    go_to_the_choice(sim);
    garrick
}

/// On the open turning, the leaf whose panel tells the dead's black marks — for the
/// picture of the line the heir choice turns on.
pub fn turn_to_the_marks(sim: &mut HeadlessSim) {
    let count = {
        let house = sim.world().resource::<House>();
        house.passage.as_ref().map_or(0, |passage| {
            crate::turning_view::leaves(passage, &house.heroes).len()
        })
    };
    for leaf in 0..count {
        let mut ui = *sim.world().resource::<crate::screen::UiState>();
        ui.leaf = leaf;
        crate::verify::set_ui(sim, ui);
        let shown = crate::scripted::lines_in(&page_of(sim), crate::telling_view::PANEL);
        if shown.iter().any(|line| line.contains("black marks")) {
            return;
        }
    }
    fail(
        "no leaf of the turning tells the black marks",
        "staged with three",
    );
}

/// Every line of turning page 1 (the death page), every leaf.
fn page_one(sim: &mut HeadlessSim) -> Vec<String> {
    page_leaves(sim, 1).into_iter().flatten().collect()
}

/// The heir button whose label starts with `name`.
fn button(sim: &HeadlessSim, name: &str) -> Option<Target> {
    heir_labels(&page_of(sim))
        .into_iter()
        .find(|(_, label)| label.starts_with(name))
        .map(|(target, _)| target)
}

/// The succession check, on every recorded seed. Returns the summary line.
pub fn check_succession(checks: &mut Checks) -> String {
    for seed in recorded() {
        let (mut sim, garrick) = death_page(seed, 3, false);
        let lines = page_one(&mut sim);
        let labels: Vec<String> = heir_labels(&page_of(&sim))
            .into_iter()
            .map(|(_, l)| l)
            .collect();
        checks.require(
            lines.iter().any(|line| line == THREE_MARKS) && labels == ORACLE_HEIRS,
            "row 3: Garrick's death page does not say what an heir would take of his marks",
            format!("seed {seed:#x}: {labels:?}; the page {lines:?}"),
        );
        let Some(maren_button) = button(&sim, "Maren") else {
            fail(
                "row 3: Maren is not on Garrick's heir list",
                &format!("{labels:?}"),
            );
        };
        point_at(&mut sim, maren_button, false);
        let pages = dock_pages(&mut sim, &mut FrameRecorder::new(WINDOW));
        let sheet: Vec<String> = dock_read(&pages)
            .into_iter()
            .map(|line| line.text)
            .collect();
        checks.require(
            sheet.first().map(String::as_str) == Some("Maren Thorne")
                && sheet.iter().any(|line| line == MAREN_LEAN)
                && sheet.iter().any(|line| line.contains("deep water"))
                && !sheet.iter().any(|line| line == "BLACK MARKS"),
            "row 3: pointing at Maren's button does not show what she has before choosing",
            format!("seed {seed:#x}: the sheet reads {sheet:?}"),
        );
        point_at(&mut sim, maren_button, true);
        let lines = page_one(&mut sim);
        let house = sim.world().resource::<House>();
        let maren = hero_named(&sim, "Maren");
        let hero = &house.heroes[maren];
        checks.require(
            lines.iter().any(|line| line == MAREN_TAKES)
                && hero.marks == 1
                && hero.lean == Some(Lean { aptitude: Aptitude::Wits, from: None })
                && hero.fear.tag == Tag::Water
                && house.heroes[garrick].marks == 0
                && hero.heirloom.as_ref().is_some_and(|h| h.name == "Thornfall")
                && hero.burden.is_some(),
            "row 3: the heir's state after choosing is not what the page and her sheet showed",
            format!(
                "seed {seed:#x}: marks {}, lean {:?}, fear {:?}, heirloom {:?}, burden {}, Garrick's marks {}; the page {lines:?}",
                hero.marks,
                hero.lean,
                hero.fear.tag,
                hero.heirloom.as_ref().map(|h| &h.name),
                hero.burden.is_some(),
                house.heroes[garrick].marks
            ),
        );

        let (mut sim, _) = death_page(seed, 1, false);
        let lines = page_one(&mut sim);
        if let Some(maren_button) = button(&sim, "Maren") {
            point_at(&mut sim, maren_button, true);
        }
        let maren = hero_named(&sim, "Maren");
        let marks = sim.world().resource::<House>().heroes[maren].marks;
        checks.require(
            lines.iter().any(|line| line == ONE_MARK) && marks == 0,
            "row 3: a single mark is not buried with its bearer",
            format!("seed {seed:#x}: Maren's marks {marks}; the page {lines:?}"),
        );

        let (mut sim, _) = death_page(seed, 2, true);
        let no_one = button(&sim, "No one");
        if let Some(no_one) = no_one {
            point_at(&mut sim, no_one, true);
        }
        let lines = page_one(&mut sim);
        let carried: i32 = sim
            .world()
            .resource::<House>()
            .heroes
            .iter()
            .map(|h| h.marks)
            .sum();
        checks.require(
            no_one.is_some() && lines.iter().any(|line| line == INTO_THE_GROUND) && carried == 0,
            "row 3: two marks alone do not hold the page, or \"No one\" does not bury them",
            format!(
                "seed {seed:#x}: page waits {}, marks left {carried}; the page {lines:?}",
                no_one.is_some()
            ),
        );
    }
    format!(
        "XI row 3 (succession): the marks an heir takes on the page and the heir's lean, fear and marks on the sheet, equal after choosing; one buried, two held, on {} seeds",
        recorded().len()
    )
}

/// The birth check: the child leans after the parent the page names.
pub fn check_birth(checks: &mut Checks) -> String {
    let seed = recorded()[0];
    let mut sim = session(seed);
    crate::w8::stage_turned_year(&mut sim);
    crate::w8::turn_to_kind(&mut sim, PageKind::Birth);
    let house = sim.world().resource::<House>().clone();
    let Some(page) = house.passage.as_ref().and_then(|passage| {
        passage
            .pages
            .iter()
            .find(|page| page.kind == PageKind::Birth)
    }) else {
        fail(
            "row 3: the staged turning has no birth",
            &format!("seed {seed:#x}"),
        );
    };
    let Some(child) = page.about else {
        fail("row 3: the birth page is about no one", "");
    };
    let hero = &house.heroes[child];
    let [maren, brannoc] = ["Maren", "Brannoc"].map(|name| hero_named(&sim, name));
    let pronoun = if hero.pronoun == crate::ids::Pronoun::He {
        "he"
    } else {
        "she"
    };
    let after = |parent: HeroId, aptitude: &str| {
        format!(
            "{} takes after {}: {pronoun} leans to {aptitude}.",
            hero.name, house.heroes[parent].name
        )
    };
    let (parent, aptitude, line) = if page.lines.contains(&after(maren, "Wits")) {
        (maren, Aptitude::Wits, after(maren, "Wits"))
    } else {
        (brannoc, Aptitude::Might, after(brannoc, "Might"))
    };
    let shown = lines_on_screen(&mut sim);
    let share = (house.heroes[maren].base(aptitude) + house.heroes[brannoc].base(aptitude)) / 4;
    checks.require(
        page.lines.contains(&line)
            && shown.contains(&line)
            && hero.lean
                == Some(Lean {
                    aptitude,
                    from: Some(parent),
                })
            && hero.is_family
            && hero.base(aptitude) > share
            && hero.base(aptitude) <= 9,
        "row 3: the newborn does not lean after the parent the birth page names",
        format!(
            "seed {seed:#x}: lean {:?}, family {}, base {} (share {share}); the page {:?}",
            hero.lean,
            hero.is_family,
            hero.base(aptitude),
            page.lines
        ),
    );
    format!("XI birth: {line:?}, and the child leans so")
}

/// The open turning's panel lines on its current page, every leaf of it.
fn lines_on_screen(sim: &mut HeadlessSim) -> Vec<String> {
    let index = {
        let house = sim.world().resource::<House>();
        house
            .passage
            .as_ref()
            .and_then(|passage| {
                passage
                    .pages
                    .iter()
                    .position(|page| page.kind == PageKind::Birth)
            })
            .unwrap_or(0)
    };
    page_leaves(sim, index).into_iter().flatten().collect()
}
