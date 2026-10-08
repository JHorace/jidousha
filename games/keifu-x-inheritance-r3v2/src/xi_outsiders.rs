//! Decision row 2 (variant, DESIGN.md S8, gate G6): whether to let an outsider
//! marry into the family.
//!
//! A wanderer arrives at the winter and is seated in the garden beside Ysolde by
//! drag. With 1 renown the garden reads "renown 1 of 4" and the wanderer's sheet
//! says what the house asks; letting the winter pass refuses them, they stay an
//! outsider and are no heir of Garrick's. With 4 the garden reads "marries in",
//! the winter weds them into the name, and Garrick's death page offers them.
//! Every expectation is a shipped literal.

use jidousha::prelude::*;
use jidousha::testing::FrameRecorder;

use crate::checks::{Checks, fail};
use crate::hearth::{Group, Seat};
use crate::house::House;
use crate::ids::BondKind;
use crate::passage::PageKind;
use crate::screen::{Target, WINDOW};
use crate::verify::{dock_pages, dock_read, hero_named, page_of, point_at, session};
use crate::w7::{group_lines, seat_at, stay_home_into_winter};
use crate::w8::{ORACLE_HEIRS, go_to_the_choice, heir_labels};

/// What a staged garden came to.
struct Garden {
    /// The garden panel's lines before the winter was let pass.
    notes: Vec<String>,
    /// The wanderer's sheet, read through the dock.
    sheet: Vec<String>,
    /// The winter page's lines.
    winter: Vec<String>,
    /// The heir buttons on Garrick's death page.
    heirs: Vec<String>,
    name: String,
    is_family: bool,
    wed: bool,
}

/// Year 1 at home into the winter; a wanderer of `renown`, aged 30, arrives and is
/// seated in the garden beside Ysolde by drag. Returns the wanderer.
pub fn stage_garden(
    sim: &mut HeadlessSim,
    content: &crate::content::Content,
    renown: i32,
) -> usize {
    stay_home_into_winter(sim);
    let mut rng = sim.world().resource::<Rng>().clone();
    let wanderer = {
        let house = sim.world_mut().resource_mut::<House>();
        let _ = crate::wanderer::arrive(content, house, &mut rng);
        let wanderer = house.heroes.len() - 1;
        house.heroes[wanderer].age = 30;
        house.heroes[wanderer].renown = renown;
        house.open_hearth();
        wanderer
    };
    sim.world_mut().insert_resource(rng);
    let name = sim.world().resource::<House>().heroes[wanderer]
        .name
        .clone();
    seat_at(sim, &name, Seat::Garden(0));
    seat_at(sim, "Ysolde", Seat::Garden(1));
    wanderer
}

/// Year 1 at home into the winter; a wanderer of `renown` arrives and is seated in
/// the garden beside Ysolde; Garrick is aged to his death; the winter is let pass.
fn garden(seed: u64, renown: i32) -> Garden {
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => fail("the content did not load", &error.to_string()),
    };
    let mut sim = session(seed);
    let wanderer = stage_garden(&mut sim, &content, renown);
    let name = sim.world().resource::<House>().heroes[wanderer]
        .name
        .clone();
    let notes = group_lines(&sim, Group::Garden);
    point_at(&mut sim, Target::Hero(wanderer), false);
    let pages = dock_pages(&mut sim, &mut FrameRecorder::new(WINDOW));
    let sheet = dock_read(&pages)
        .into_iter()
        .map(|line| line.text)
        .collect();
    let (garrick, ysolde) = (hero_named(&sim, "Garrick"), hero_named(&sim, "Ysolde"));
    sim.world_mut().resource_mut::<House>().heroes[garrick].age = 93;
    point_at(&mut sim, Target::LetWinterPass, true);
    let house = sim.world().resource::<House>();
    let winter = house
        .passage
        .as_ref()
        .and_then(|passage| {
            passage
                .pages
                .iter()
                .find(|page| page.kind == PageKind::Winter)
        })
        .map(|page| page.lines.clone())
        .unwrap_or_default();
    let is_family = house.heroes[wanderer].is_family;
    let wed = house.heroes[wanderer]
        .bond_to(ysolde)
        .is_some_and(|bond| bond.kind == BondKind::Spouse)
        && house.heroes[ysolde]
            .bond_to(wanderer)
            .is_some_and(|bond| bond.kind == BondKind::Spouse);
    go_to_the_choice(&mut sim);
    let heirs = heir_labels(&page_of(&sim))
        .into_iter()
        .map(|(_, label)| label)
        .collect();
    Garden {
        notes,
        sheet,
        winter,
        heirs,
        name,
        is_family,
        wed,
    }
}

/// The check, on the first two recorded seeds. Returns the summary line.
pub fn check_marry_in(checks: &mut Checks) -> String {
    for seed in crate::w5::recorded().into_iter().take(2) {
        let refused = garden(seed, 1);
        let name = refused.name.clone();
        checks.require(
            refused.notes.iter().any(|line| line == "renown 1 of 4"),
            "row 2: the garden does not show an unproven outsider's renown and the house's ask",
            format!("seed {seed:#x}: the garden reads {:?}", refused.notes),
        );
        checks.require(
            refused
                .sheet
                .iter()
                .any(|line| line == "An outsider. Renown 1 of 4 to marry in."),
            "row 2: the outsider's sheet does not say what the house asks",
            format!("seed {seed:#x}: the sheet reads {:?}", refused.sheet),
        );
        let unproven = format!(
            "{name} walked in the garden with Ysolde. An outsider with 1 renown may not marry in: the house asks 4."
        );
        checks.require(
            refused.winter.contains(&unproven) && !refused.is_family && !refused.wed,
            "row 2: a below-threshold outsider was not refused",
            format!(
                "seed {seed:#x}: family {}, wed {}; the winter page {:?}",
                refused.is_family, refused.wed, refused.winter
            ),
        );
        checks.require(
            refused.heirs == ORACLE_HEIRS,
            "row 2: an outsider was offered as an heir",
            format!("seed {seed:#x}: {:?}", refused.heirs),
        );

        let accepted = garden(seed, 4);
        let name = accepted.name.clone();
        checks.require(
            accepted.notes.iter().any(|line| line == "marries in"),
            "row 2: the garden does not say a proven outsider marries in",
            format!("seed {seed:#x}: the garden reads {:?}", accepted.notes),
        );
        let married =
            format!("{name} is of the Thorne name now. 4 renown was enough: the house asked 4.");
        checks.require(
            accepted.winter.contains(&married) && accepted.is_family && accepted.wed,
            "row 2: an at-threshold outsider did not marry into the family",
            format!(
                "seed {seed:#x}: family {}, wed {}; the winter page {:?}",
                accepted.is_family, accepted.wed, accepted.winter
            ),
        );
        let mut want: Vec<String> = ORACLE_HEIRS
            .iter()
            .map(|label| (*label).to_owned())
            .collect();
        want.insert(6, format!("{name}, of the house"));
        checks.require(
            accepted.heirs == want,
            "row 2: one who married in is not offered as an heir",
            format!("seed {seed:#x}: {:?}", accepted.heirs),
        );
    }
    "XI row 2 (marrying in): renown 1 of 4 refused and no heir, 4 of 4 wed into the name and an heir, on 2 seeds".to_owned()
}
