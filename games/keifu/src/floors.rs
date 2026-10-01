//! Readability floors over every surface this build has.
//!
//! The floors are the ones `crates/jidousha/examples/text` asserts: nothing set
//! below the minimum size, no row outside its panel, no two rows across each
//! other, nothing the built-in face cannot draw — all against measured extents.
//! Plus the camera floor: nothing drawn off screen, with the margin printed.
//! Surfaces: the summer screen with nobody pointed at and with each hero's sheet,
//! and the family screen with nobody pointed at and with each node's remembrance;
//! then W3's settled and blessed sheets, staged; then W4's board — the hand
//! mid-drag, seated cards, quest sheets and history panels, and a staged worst case.

use jidousha::testing::{BackendTextureId, FrameRecord, FrameRecorder};

use crate::checks::Checks;
use crate::house::House;
use crate::screen::{MIN_TEXT, Page, Target, UiState, camera, ink, layers};
use crate::verify::{page_of, point_at, session, set_ui};

/// The surfaces judged, and the smallest clearance seen.
struct Tally {
    surfaces: usize,
    rows: usize,
    smallest: f32,
    clearance: f32,
    font: BackendTextureId,
}

fn judge(
    checks: &mut Checks,
    tally: &mut Tally,
    name: &str,
    page: &Page,
    frame: &FrameRecord,
    overlay: bool,
) {
    let font = tally.font;
    tally.surfaces += 1;
    let front = |row: &&crate::screen::Row| !overlay || row.style.depth.layer >= layers::OVERLAY;
    let rows: Vec<&crate::screen::Row> = page.rows.iter().filter(front).collect();
    tally.rows += rows.len();
    for row in &rows {
        tally.smallest = tally.smallest.min(row.style.size);
        checks.require(
            row.style.size >= MIN_TEXT,
            "a row is set below the readability floor",
            format!(
                "{name}: {:?} at {:.1}, floor {MIN_TEXT:.0}",
                row.text, row.style.size
            ),
        );
        checks.require(
            row.panel.contains_rect(row.bounds()),
            "a row runs out of its panel",
            format!(
                "{name}: {:?} measures {:?} in {:?}",
                row.text,
                row.bounds(),
                row.panel
            ),
        );
        checks.require(
            !row.text.is_empty() && row.text.chars().all(|c| (' '..='~').contains(&c)),
            "a row is empty or says something the built-in face cannot draw",
            format!("{name}: {:?}", row.text),
        );
    }
    // The sheet's two columns keep a gutter: nothing in the left column reaches
    // within half a pad of the panel's midline.
    let sheet = crate::summer::SHEET;
    let midline = sheet.center().x;
    let two_columns = rows.iter().any(|r| r.panel == sheet && r.at.x >= midline);
    for row in rows.iter().filter(|r| two_columns && r.panel == sheet) {
        checks.require(
            row.bounds().max.x <= sheet.max.x - crate::screen::PAD * 0.5,
            "the sheet's right column runs into the panel's edge",
            format!(
                "{name}: {:?} ends at {:.1}, panel edge {:.1}",
                row.text,
                row.bounds().max.x,
                sheet.max.x
            ),
        );
    }
    for row in rows
        .iter()
        .filter(|r| two_columns && r.panel == sheet && r.at.x < midline)
    {
        checks.require(
            row.bounds().max.x <= midline - crate::screen::PAD * 0.5,
            "the sheet's left column runs into its gutter",
            format!(
                "{name}: {:?} ends at {:.1}, midline {midline:.1}",
                row.text,
                row.bounds().max.x
            ),
        );
    }
    for (index, row) in rows.iter().enumerate() {
        for other in rows.iter().skip(index + 1) {
            checks.require(
                !row.bounds().overlaps(other.bounds()),
                "two rows of type lie across each other",
                format!(
                    "{name}: {:?} {:?} and {:?} {:?}",
                    row.text,
                    row.bounds(),
                    other.text,
                    other.bounds()
                ),
            );
        }
    }
    // Everything the page holds was submitted: a rectangle and a line are one quad
    // each, and type one per character.
    let chars: usize = page.rows.iter().map(|row| row.text.chars().count()).sum();
    let expected = page.shapes.len() + page.links.len() + page.figures.len() + chars;
    checks.require(
        frame.quad_count() == expected,
        "the frame does not hold what the page says to draw",
        format!(
            "{name}: {} quads, page has {} shapes, {} links, {chars} characters",
            frame.quad_count(),
            page.shapes.len(),
            page.links.len()
        ),
    );
    let view = camera().visible_bounds();
    for quad in frame.quads() {
        let bounds = quad.bounds();
        checks.require(
            view.contains_rect(bounds),
            "something is drawn off screen",
            format!("{name}: {bounds:?} against {view:?}"),
        );
        // Panels sit flush with the edge by design; the margin worth watching is type's.
        if quad.texture == font {
            let gap = (bounds.min - view.min).min(view.max - bounds.max);
            tally.clearance = tally.clearance.min(gap.x.min(gap.y));
        }
    }
    // The page has to be dark enough for light type to read on it: the
    // requirement, beside the constant it is set from.
    let clear = frame.plan.clear_color;
    checks.require(
        clear == ink::PAGE && clear.r.max(clear.g).max(clear.b) < 0.2,
        "the page is not cleared dark",
        format!("{name}: {clear:?}"),
    );
}

/// Judge every surface.
pub fn check(checks: &mut Checks, recorder: &mut FrameRecorder) -> String {
    let mut tally = Tally {
        surfaces: 0,
        rows: 0,
        smallest: f32::MAX,
        clearance: f32::MAX,
        font: recorder.font_texture(),
    };
    let mut sim = session(crate::verify::SEEDS[0]);
    let frame = crate::verify::frame(recorder, &mut sim);
    judge(
        checks,
        &mut tally,
        "summer, nobody pointed at",
        &page_of(&sim),
        &frame,
        false,
    );
    let seated: Vec<usize> = {
        let house = sim.world().resource::<House>();
        house
            .roster
            .iter()
            .flatten()
            .copied()
            .chain(house.yard())
            .collect()
    };
    for id in &seated {
        point_at(&mut sim, Target::Hero(*id), false);
        let frame = crate::verify::frame(recorder, &mut sim);
        let name = format!(
            "summer, {}'s sheet",
            sim.world().resource::<House>().heroes[*id].name
        );
        judge(checks, &mut tally, &name, &page_of(&sim), &frame, false);
    }
    set_ui(&mut sim, UiState::family());
    let frame = crate::verify::frame(recorder, &mut sim);
    judge(
        checks,
        &mut tally,
        "family, nobody pointed at",
        &page_of(&sim),
        &frame,
        true,
    );
    let everyone = sim.world().resource::<House>().heroes.len();
    for id in 0..everyone {
        point_at(&mut sim, Target::Hero(id), false);
        let frame = crate::verify::frame(recorder, &mut sim);
        let name = format!(
            "family, {}",
            sim.world().resource::<House>().heroes[id].name
        );
        judge(checks, &mut tally, &name, &page_of(&sim), &frame, true);
    }
    // W3's sheets: Garrick settled, blessed and holding a forged blade; Maren with
    // Thornfall and the blessing; Pip blessed; and the family's word for Garrick.
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => crate::checks::fail("the content did not load", &error.to_string()),
    };
    crate::w3::stage_settled(&content, sim.world_mut().resource_mut::<House>());
    set_ui(&mut sim, UiState::default());
    let staged = ["Garrick", "Maren", "Pip"].map(|name| crate::verify::hero_named(&sim, name));
    for id in staged {
        point_at(&mut sim, Target::Hero(id), false);
        let frame = crate::verify::frame(recorder, &mut sim);
        let name = format!(
            "W3, {}'s sheet",
            sim.world().resource::<House>().heroes[id].name
        );
        judge(checks, &mut tally, &name, &page_of(&sim), &frame, false);
    }
    set_ui(&mut sim, UiState::family());
    point_at(&mut sim, Target::Hero(staged[0]), false);
    let frame = crate::verify::frame(recorder, &mut sim);
    judge(
        checks,
        &mut tally,
        "W3, the family on Garrick",
        &page_of(&sim),
        &frame,
        true,
    );
    let w4 = w4_surfaces(checks, &mut tally, recorder);
    checks.require(
        tally.surfaces == 2 + seated.len() + everyone + staged.len() + 1 + w4,
        "a surface was not judged",
        format!("{} surfaces", tally.surfaces),
    );
    format!(
        "floors: {} surfaces, {} rows, smallest type {:.0} (floor {MIN_TEXT:.0}), closest type to the edge {:.2} px",
        tally.surfaces, tally.rows, tally.smallest, tally.clearance
    )
}

/// W4's surfaces: the hand mid-drag; both cards seated; both quest sheets and their
/// history panels; and a staged worst case — a troubled quest of three seats with
/// three seated, bonds, fears and patrons, beside a troubled empty quest whose
/// place has been visited and has its fallen. Returns how many were judged.
fn w4_surfaces(checks: &mut Checks, tally: &mut Tally, recorder: &mut FrameRecorder) -> usize {
    use crate::board::Slot;
    use crate::w4::{away, seat, stage_mid_drag};
    let before = tally.surfaces;
    let mut judge_now = |checks: &mut Checks,
                         tally: &mut Tally,
                         sim: &mut jidousha::prelude::HeadlessSim,
                         name: &str| {
        let frame = crate::verify::frame(recorder, sim);
        judge(checks, tally, name, &page_of(sim), &frame, false);
    };
    let mut sim = session(crate::verify::SEEDS[0]);
    stage_mid_drag(&mut sim);
    judge_now(
        checks,
        tally,
        &mut sim,
        "W4, Brannoc in hand over Grave goods",
    );
    let mut sim = session(crate::verify::SEEDS[0]);
    seat(&mut sim, "Garrick", Slot::Quest { quest: 0, seat: 0 });
    seat(&mut sim, "Brannoc", Slot::Quest { quest: 0, seat: 1 });
    seat(&mut sim, "Maren", Slot::Quest { quest: 1, seat: 0 });
    seat(&mut sim, "Odo", Slot::Quest { quest: 1, seat: 1 });
    away(&mut sim);
    judge_now(checks, tally, &mut sim, "W4, both cards seated");
    for quest in 0..2 {
        point_at(&mut sim, Target::Quest(quest), false);
        judge_now(
            checks,
            tally,
            &mut sim,
            &format!("W4, quest sheet {quest}, seated"),
        );
    }
    let mut sim = session(crate::verify::SEEDS[0]);
    point_at(&mut sim, Target::Quest(1), false);
    judge_now(
        checks,
        tally,
        &mut sim,
        "W4, the bell's sheet, nobody going",
    );
    // The worst case, staged on the session's own house.
    let mut sim = session(crate::verify::SEEDS[0]);
    crate::w4::stage_worst(&mut sim);
    judge_now(
        checks,
        tally,
        &mut sim,
        "W4 staged, three on a troubled quest",
    );
    for quest in 0..2 {
        point_at(&mut sim, Target::Quest(quest), false);
        judge_now(
            checks,
            tally,
            &mut sim,
            &format!("W4 staged, quest sheet {quest}"),
        );
    }
    tally.surfaces - before
}
