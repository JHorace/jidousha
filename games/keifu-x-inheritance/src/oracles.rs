//! The W0 and W1 oracles of `spec/MODULES.md`, and the founding they stand on.
//!
//! INVARIANT: every expectation here is a shipped literal — a string copied from
//! MODULES.md's oracle, or a number read off `household.json` and CONSTANTS.md by
//! hand — never a value computed by the code under test. A check that derived its
//! expectation from the game would move with any mutation of it.

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord, FrameRecorder};

use crate::checks::Checks;
use crate::hero::{descends_from, firstborn, kin};
use crate::house::House;
use crate::ids::{Aptitude, BondKind, Phase, Place, Vocation};
use crate::screen::{Page, Target, ink};
use crate::verify::{hero_named, page_of, point_at, row_drawn, session};

/// W0's oracle: "Year 1 of 25", "Summer", "House renown 15", and the Door's two lines.
pub const W0_TOP_BAR: [&str; 5] = [
    "Year 1 of 25",
    "Summer",
    "House renown 15",
    "The Sealed Door opens in 25 years. It will ask for four.",
    "Dark, Cold. Locks: Might 34, Wits 34, Spirit 34.",
];

/// W1's oracle, Garrick's sheet, in the order the sheet must show it.
///
/// The two living bonds are checked apart from this order (SPEC-GAPS KG-1).
pub const W1_GARRICK_IN_ORDER: [&str; 22] = [
    "Garrick Thorne",
    "Might 5 (-2)",
    "Wits 5 (+1)",
    "Spirit 4",
    "DREAM",
    "To lay the Barrow's dead to rest",
    "[x] Reach the Barrow",
    "[x] Succeed against the Undead twice",
    "[>] Win a triumph at the Barrow",
    "FEAR",
    "Water (deep water)",
    "Dread",
    "-3 power",
    // The variant's TRAITS section, between FEAR and DESTINY.
    "TRAITS",
    "Strong: +1 Might on quests.",
    "DESTINY, COME",
    "Your child will surpass you.",
    "BONDS",
    "Wife Elsbeth, gone",
    "HEIRLOOM",
    "Thornfall",
    "+1 Might on quests.",
];

/// Garrick's living bonds: both shown, both before the bond to the dead.
pub const W1_GARRICK_LIVING_BONDS: [&str; 2] = ["Friend Odo +1", "Daughter Maren +2"];

/// The founding in creation order (SPEC §4).
const CREATION_ORDER: [&str; 9] = [
    "Elsbeth", "Garrick", "Maren", "Pip", "Ysolde", "Brannoc", "Odo", "Aud", "Wren",
];

/// Living adults seated in creation order; children in the yard (SPEC §5.1).
const ROSTER: [&str; 5] = ["Garrick", "Maren", "Ysolde", "Brannoc", "Odo"];
const YARD: [&str; 2] = ["Pip", "Wren"];

/// Find `want` among `page`'s logical lines from `from` on.
pub fn find_from(page: &Page, from: usize, want: &str) -> Option<usize> {
    page.logical_lines()
        .iter()
        .skip(from)
        .position(|(line, _)| line == want)
        .map(|at| from + at)
}

/// Whether every row of logical line `index` was drawn.
fn line_drawn(page: &Page, frame: &FrameRecord, font: BackendTextureId, index: usize) -> bool {
    let lines = page.logical_lines();
    lines.get(index).is_some_and(|(_, rows)| {
        rows.iter()
            .all(|&row| row_drawn(frame, font, &page.rows[row]))
    })
}

/// The page's logical lines, for a message.
pub fn lines_of(page: &Page) -> Vec<String> {
    page.logical_lines()
        .into_iter()
        .map(|(line, _)| line)
        .collect()
}

/// The W0 and W1 oracles on `seed`. Returns a summary line and the frame of Garrick's sheet.
pub fn check_w0_and_w1(
    checks: &mut Checks,
    recorder: &mut FrameRecorder,
    seed: u64,
) -> (String, FrameRecord) {
    let mut sim = session(seed);
    check_w0_and_w1_on(checks, recorder, &mut sim, seed)
}

/// The W0 and W1 oracles on the house `sim` holds now — a founding, or another house
/// begun at the Ending (W10) — labelled by its `seed`.
pub fn check_w0_and_w1_on(
    checks: &mut Checks,
    recorder: &mut FrameRecorder,
    sim: &mut HeadlessSim,
    seed: u64,
) -> (String, FrameRecord) {
    let frame = crate::verify::frame(recorder, sim);
    let font = recorder.font_texture();
    let page = page_of(sim);
    for want in W0_TOP_BAR {
        let found = find_from(&page, 0, want);
        checks.require(
            found.is_some_and(|index| line_drawn(&page, &frame, font, index)),
            "W0 oracle: a top-bar reading is not on screen",
            format!(
                "seed {seed:#x}: wanted {want:?}; the top bar's rows are {:?}",
                lines_of(&page).into_iter().take(5).collect::<Vec<_>>()
            ),
        );
    }

    let garrick = hero_named(sim, "Garrick");
    point_at(sim, Target::Hero(garrick), false);
    // Garrick's sheet is longer than the dock, so it is read the way a player reads
    // it: paged through by the wheel, each line once, in order, on the page that
    // shows it (`verify::dock_pages`).
    let pages = crate::verify::dock_pages(sim, recorder);
    let read = crate::verify::dock_read(&pages);
    let sheet: Vec<String> = read.iter().map(|line| line.text.clone()).collect();
    let mut at = 0;
    for want in W1_GARRICK_IN_ORDER {
        match sheet.iter().skip(at).position(|line| line == want) {
            Some(found) => {
                let found = at + found;
                let line = &read[found];
                let (page, frame) = &pages[line.page];
                checks.require(
                    line.rows
                        .iter()
                        .all(|&row| row_drawn(frame, font, &page.rows[row])),
                    "W1 oracle: a sheet line is on the page but not drawn",
                    format!("seed {seed:#x}: {want:?} has no glyphs in its box"),
                );
                at = found + 1;
            }
            None => checks.require(
                false,
                "W1 oracle: Garrick's sheet is missing a line, or shows it out of order",
                format!(
                    "seed {seed:#x}: {want:?} not found after line {at}; the sheet reads {sheet:?}"
                ),
            ),
        }
    }
    let position = |want: &str| sheet.iter().position(|line| line == want);
    let bonds = position("BONDS");
    let gone = position("Wife Elsbeth, gone");
    for want in W1_GARRICK_LIVING_BONDS {
        let found = position(want);
        checks.require(
            matches!((bonds, found, gone), (Some(b), Some(f), Some(g)) if b < f && f < g),
            "W1 oracle: a living bond is missing or not between BONDS and the bond to the dead",
            format!(
                "seed {seed:#x}: {want:?} at {found:?}, BONDS at {bonds:?}, Elsbeth at {gone:?}"
            ),
        );
    }
    let read_pages: Vec<Page> = pages.iter().map(|(page, _)| page.clone()).collect();
    let (page, frame) = match pages.into_iter().next() {
        Some(first) => first,
        None => crate::checks::fail("the dock showed no page", "dock_pages returned none"),
    };
    check_dread_pips(checks, &read_pages, seed);
    check_cards(checks, sim, &page, seed);
    (
        format!(
            "oracles: W0 top bar ({} readings) and W1 Garrick sheet ({} lines) on screen",
            W0_TOP_BAR.len(),
            W1_GARRICK_IN_ORDER.len() + 2
        ),
        frame,
    )
}

/// "fear Water with 2 dread": two filled pips of five beside Dread, none of three
/// beside Courage — on whichever page of the dock shows each row.
fn check_dread_pips(checks: &mut Checks, pages: &[Page], seed: u64) {
    for (label, want_filled, want_of) in [("Dread", 2, 5), ("Courage", 0, 3)] {
        let Some((page, row)) = pages.iter().find_map(|page| {
            page.rows
                .iter()
                .find(|row| row.text == label)
                .map(|row| (page, row))
        }) else {
            checks.require(
                false,
                "W1 oracle: a fear row is missing",
                format!("seed {seed:#x}: no {label:?} row"),
            );
            continue;
        };
        let band = row.bounds();
        let beside: Vec<Color> = page
            .shapes
            .iter()
            .filter(|s| {
                s.rect.min.y >= band.min.y
                    && s.rect.max.y <= band.max.y + 2.0
                    && s.rect.min.x > band.max.x
            })
            .map(|s| s.color)
            .collect();
        let filled = beside.iter().filter(|c| **c == ink::DREAD).count();
        checks.require(
            filled == want_filled && beside.len() == want_of,
            "W1 oracle: Garrick's fear pips are wrong",
            format!("seed {seed:#x}: {filled} filled of {} beside {label:?}, want {want_filled} of {want_of}", beside.len()),
        );
    }
}

/// The cards: Garrick's figure is grey (he is an elder) and Maren's is not; one dread
/// pip per point of dread (Garrick 2, Maren 1).
fn check_cards(checks: &mut Checks, sim: &HeadlessSim, page: &Page, seed: u64) {
    // An elder's sprite is tinted the grey ink; anyone else's is untinted.
    let grey = |c: Color| c == ink::GONE;
    for (name, want_grey, want_pips) in [("Garrick", true, 2), ("Maren", false, 1)] {
        let id = hero_named(sim, name);
        let Some(card) = page
            .targets
            .iter()
            .find(|(_, t)| *t == Target::Hero(id))
            .map(|(r, _)| *r)
        else {
            checks.require(
                false,
                "a card is missing",
                format!("seed {seed:#x}: {name}"),
            );
            continue;
        };
        let inside: Vec<Color> = page
            .shapes
            .iter()
            .filter(|s| card.contains_rect(s.rect) && s.rect != card)
            .map(|s| s.color)
            .collect();
        let pips = inside.iter().filter(|c| **c == ink::DREAD).count();
        let figure = page
            .figures
            .iter()
            .find(|f| card.contains_rect(f.rect))
            .map(|f| f.tint);
        checks.require(
            pips == want_pips && figure.is_some_and(|c| grey(c) == want_grey),
            "a card's dread pips or figure tint are wrong",
            format!("seed {seed:#x}: {name} has {pips} pips (want {want_pips}), figure {figure:?} (grey wanted: {want_grey})"),
        );
    }
}

/// The founding household and every derived quantity W1 owns.
pub fn check_founding(checks: &mut Checks) -> String {
    let sim = session(crate::verify::SEEDS[0]);
    let house = sim.world().resource::<House>();
    let names: Vec<&str> = house.heroes.iter().map(|h| h.name.as_str()).collect();
    checks.require(
        names == CREATION_ORDER,
        "the founding is not in creation order",
        format!("{names:?}"),
    );
    let by = |name: &str| house.heroes.iter().position(|h| h.name == name);
    let roster: Vec<&str> = house
        .roster
        .iter()
        .flatten()
        .map(|&id| house.heroes[id].name.as_str())
        .collect();
    checks.require(
        roster == ROSTER,
        "the roster is not the living adults in creation order",
        format!("{roster:?}"),
    );
    let yard: Vec<&str> = house
        .yard()
        .iter()
        .map(|&id| house.heroes[id].name.as_str())
        .collect();
    checks.require(
        yard == YARD,
        "the yard is not the living children in creation order",
        format!("{yard:?}"),
    );
    checks.require(
        house.renown == 15,
        "house renown at founding is not 15",
        format!("{}", house.renown),
    );
    let (
        Some(elsbeth),
        Some(garrick),
        Some(maren),
        Some(pip),
        Some(odo),
        Some(aud),
        Some(wren),
        Some(brannoc),
    ) = (
        by("Elsbeth"),
        by("Garrick"),
        by("Maren"),
        by("Pip"),
        by("Odo"),
        by("Aud"),
        by("Wren"),
        by("Brannoc"),
    )
    else {
        return "founding: a founding hero is missing".to_owned();
    };
    let h = &house.heroes;
    for (id, living) in [
        (elsbeth, false),
        (aud, false),
        (garrick, true),
        (wren, true),
    ] {
        checks.require(
            h[id].is_living() == living && (living || (h[id].grieved && h[id].bequest_decided)),
            "a founding hero's fate or grief is wrong",
            format!(
                "{}: living {}, grieved {}, decided {}",
                h[id].name,
                h[id].is_living(),
                h[id].grieved,
                h[id].bequest_decided
            ),
        );
    }
    checks.require(
        house.fallen_at(Place::DrownedCoast) == [elsbeth],
        "Elsbeth is not among the Drowned Coast's fallen",
        format!("{:?}", house.fallen_at(Place::DrownedCoast)),
    );
    let phases = [
        (garrick, Phase::Elder),
        (maren, Phase::Prime),
        (odo, Phase::Veteran),
        (pip, Phase::Child),
    ];
    for (id, phase) in phases {
        checks.require(
            h[id].phase() == phase,
            "a phase is wrong",
            format!("{} aged {} is {:?}", h[id].name, h[id].age, h[id].phase()),
        );
    }
    let best = [
        (garrick, Aptitude::Might),
        (odo, Aptitude::Spirit),
        (elsbeth, Aptitude::Wits),
        (pip, Aptitude::Wits),
    ];
    for (id, aptitude) in best {
        checks.require(
            h[id].best_aptitude() == aptitude,
            "best aptitude is wrong (ties go to the lower index)",
            format!(
                "{} {:?} -> {:?}",
                h[id].name,
                h[id].aptitudes,
                h[id].best_aptitude()
            ),
        );
    }
    checks.require(
        h[pip].vocation == Vocation::Knight
            && h[wren].vocation == Vocation::Knight
            && h[wren].dream.is_none(),
        "children are not default Knights, or Wren dreams",
        format!(
            "Pip {:?}, Wren {:?} dream {:?}",
            h[pip].vocation,
            h[wren].vocation,
            h[wren].dream.as_ref().map(|d| d.kind)
        ),
    );
    let kinds = |id: usize| {
        h[id]
            .bonds
            .iter()
            .map(|b| (b.kind, h[b.other].name.as_str()))
            .collect::<Vec<_>>()
    };
    checks.require(
        kinds(garrick)
            == [
                (BondKind::Spouse, "Elsbeth"),
                (BondKind::Child, "Maren"),
                (BondKind::Friend, "Odo"),
            ],
        "Garrick's bonds are not spouse Elsbeth, child Maren, friend Odo in formation order",
        format!("{:?}", kinds(garrick)),
    );
    checks.require(
        kinds(brannoc)
            == [
                (BondKind::Spouse, "Aud"),
                (BondKind::Child, "Wren"),
                (BondKind::Rival, "Ysolde"),
            ],
        "Brannoc's bonds are not spouse Aud, child Wren, rival Ysolde",
        format!("{:?}", kinds(brannoc)),
    );
    checks.require(
        kin(h, maren, pip)
            && kin(h, garrick, maren)
            && !kin(h, garrick, pip)
            && !kin(h, odo, garrick),
        "kin is wrong: parent/child are kin, grandparents and friends are not",
        "Maren-Pip, Garrick-Maren, Garrick-Pip, Odo-Garrick".to_owned(),
    );
    checks.require(
        descends_from(h, pip, garrick)
            && descends_from(h, pip, elsbeth)
            && !descends_from(h, garrick, pip),
        "descent is wrong",
        "Pip from Garrick and Elsbeth; not Garrick from Pip".to_owned(),
    );
    checks.require(
        firstborn(h, garrick) == Some(maren)
            && firstborn(h, brannoc) == Some(wren)
            && firstborn(h, pip).is_none(),
        "firstborn is wrong",
        format!(
            "Garrick {:?}, Brannoc {:?}, Pip {:?}",
            firstborn(h, garrick),
            firstborn(h, brannoc),
            firstborn(h, pip)
        ),
    );
    let maren_dream = h[maren].dream.as_ref().map(|d| {
        (
            d.title.as_str(),
            d.current,
            d.stages[1].task.as_str(),
            d.stages[2].task.as_str(),
        )
    });
    checks.require(
        maren_dream
            == Some((
                "To avenge my mother",
                1,
                "Return to the Drowned Coast",
                "Succeed there against Water",
            )),
        "Maren's dream is not AVENGE_THE_LOST built for Elsbeth at stage 2",
        format!("{maren_dream:?}"),
    );
    let garrick_dream = h[garrick].dream.as_ref().map(|d| {
        (
            d.current,
            d.stages.iter().map(|s| s.count).collect::<Vec<_>>(),
        )
    });
    checks.require(
        garrick_dream == Some((2, vec![1, 2, 0])),
        "Garrick's dream is not at stage 3 with the first two counted complete",
        format!("{garrick_dream:?}"),
    );
    checks.require(
        h[garrick].heirloom.as_ref().is_some_and(|t| {
            t.name == "Thornfall" && t.bonus == 1 && t.aptitude == Aptitude::Might
        }),
        "Garrick does not hold Thornfall +1 Might",
        format!(
            "{:?}",
            h[garrick].heirloom.as_ref().map(|t| (&t.name, t.bonus))
        ),
    );
    format!(
        "founding: {} heroes in creation order, roster {}, yard {}",
        names.len(),
        roster.len(),
        yard.len()
    )
}
