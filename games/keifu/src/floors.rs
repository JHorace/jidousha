//! Readability floors over every surface this build has, at every size it is shown at.
//!
//! The floors are the ones `crates/jidousha/examples/text` asserts: nothing set
//! below the minimum size, no row outside its panel, no two rows across each
//! other, nothing the built-in face cannot draw — all against measured extents.
//! Plus the camera floor: nothing drawn off screen, with the margin printed. Plus
//! the sheet dock's: nothing laid under it, its lines inside its margin and clear of
//! its scrollbar, a scrollbar whenever a sheet is longer than the dock, and every
//! line of such a sheet reached by scrolling — each page judged like a surface.
//! Surfaces: the summer screen with nobody pointed at and with each hero's sheet,
//! and the family screen with nobody pointed at and with each node's remembrance;
//! then W3's settled and blessed sheets, staged; then W4's board — the hand
//! mid-drag, seated cards, quest sheets and history panels, and a staged worst case.
//! All of it at the window the game opens at and at two web canvases (`SIZES`).

use jidousha::prelude::{Camera, PhysicalSize, Rect};
use jidousha::testing::{BackendTextureId, FrameRecord, FrameRecorder};

use crate::checks::Checks;
use crate::house::House;
use crate::screen::{MIN_TEXT, Page, Target, UiState, WINDOW, camera, fitted, ink, layers};
use crate::summer::SHEET;
use crate::verify::{page_of, point_at, scroll_dock, session, set_ui};

/// The sizes every surface is judged at: the window the game opens at; the canvas
/// `tools/serve-web keifu --check`'s 640x480 browser gives the page (measured from
/// the page's DOM, 2026-10-02: the header and the control strip take the rest); and
/// a 4:3 window, narrower than the page, where the camera fits the width.
pub const SIZES: [(&str, PhysicalSize); 3] = [
    ("native 1280x720", WINDOW),
    ("web 640x329", PhysicalSize::new(640, 329)),
    ("web 1024x768", PhysicalSize::new(1024, 768)),
];

/// The surfaces judged at one size, and the smallest clearance seen.
pub struct Tally {
    size: PhysicalSize,
    pub surfaces: usize,
    pages: usize,
    rows: usize,
    smallest: f32,
    clearance: f32,
    font: BackendTextureId,
}

impl Tally {
    /// The camera this size is drawn with.
    fn camera(&self) -> Camera {
        fitted(self.size)
    }
}

/// Record a frame at the tally's size: the camera fitted to it for the draw, and
/// the game's own camera back after, so the scripted pointer keeps meaning what it
/// says.
fn frame_at(
    recorder: &mut FrameRecorder,
    sim: &mut jidousha::prelude::HeadlessSim,
    size: PhysicalSize,
) -> FrameRecord {
    *sim.world_mut().resource_mut::<Camera>() = fitted(size);
    let record = crate::verify::frame(recorder, sim);
    *sim.world_mut().resource_mut::<Camera>() = camera();
    record
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
    // A control keeps its own room: no row but its label within 4 px of a button.
    let controls = page.targets.iter().filter(|(_, t)| {
        matches!(
            t,
            Target::OpenFamily
                | Target::CloseFamily
                | Target::SetOut
                | Target::GoOn
                | Target::Leaf(_)
                | Target::Skip
                | Target::BeginAgain
                | Target::LetWinterPass
        )
    });
    for (rect, target) in controls {
        let room = Rect {
            min: rect.min - jidousha::prelude::Vec2::splat(4.0),
            max: rect.max + jidousha::prelude::Vec2::splat(4.0),
        };
        for row in rows.iter().filter(|r| r.panel != *rect) {
            checks.require(
                !row.bounds().overlaps(room),
                "a row of type crowds a control",
                format!(
                    "{name}: {:?} at {:?} against {target:?} at {rect:?}",
                    row.text,
                    row.bounds()
                ),
            );
        }
    }
    // The dock: its lines inside its margin and clear of the scrollbar's lane;
    // nothing else's target under it; a scrollbar whenever its sheet is longer.
    // The lane is measured from the dock's edge, not from `text_rect`, so a text
    // rect widened under the bar is caught rather than agreed with.
    let dock_text = crate::dock::text_rect();
    let lane = crate::dock::lane();
    for row in rows.iter().filter(|r| SHEET.contains_rect(r.panel)) {
        checks.require(
            dock_text.contains_rect(row.bounds()) && row.bounds().max.x < lane.min.x,
            "a line in the sheet dock runs into its margin or under its scrollbar",
            format!(
                "{name}: {:?} measures {:?}, the dock sets type in {dock_text:?}",
                row.text,
                row.bounds()
            ),
        );
    }
    // The closed house's verdict (W10 SCAFFOLD) is the one screen without the dock.
    let docked = page.targets.iter().any(|(_, t)| *t == Target::Dock);
    checks.require(
        docked || overlay || page.targets.iter().any(|(_, t)| *t == Target::BeginAgain),
        "a screen that should have the sheet dock has none",
        name.to_owned(),
    );
    if !overlay && docked {
        for (rect, target) in page.targets.iter().filter(|(_, t)| *t != Target::Dock) {
            checks.require(
                !rect.overlaps(SHEET),
                "something the pointer can reach lies under the sheet dock",
                format!("{name}: {target:?} at {rect:?}, the dock {SHEET:?}"),
            );
        }
        let lane = crate::dock::lane();
        let thumb = page
            .shapes
            .iter()
            .any(|s| s.color == ink::NOTE && lane.contains_rect(s.rect));
        checks.require(
            thumb == (page.dock.max_first > 0),
            "a sheet longer than the dock shows no scrollbar, or one that fits shows one",
            format!("{name}: thumb drawn {thumb}, dock {:?}", page.dock),
        );
        checks.require(
            page.dock.shown > 0 && page.dock.first <= page.dock.max_first,
            "the dock draws none of its sheet, or is scrolled past its end",
            format!("{name}: {:?}", page.dock),
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
    let view = tally.camera().visible_bounds();
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

/// Judge the surface `sim` shows now; if its sheet is longer than the dock, page
/// through it with the wheel and judge every page, then require that every line of
/// the sheet was on a page. Leaves the dock scrolled back to the top.
pub fn look(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    sim: &mut jidousha::prelude::HeadlessSim,
    name: &str,
    overlay: bool,
) {
    let frame = frame_at(recorder, sim, tally.size);
    let page = page_of(sim);
    judge(checks, tally, name, &page, &frame, overlay);
    if overlay || page.dock.max_first == 0 {
        return;
    }
    let total = page.dock.total;
    let mut seen = vec![false; total];
    let mut mark = |page: &Page| {
        for line in page.rows.iter().filter_map(|row| row.dock_line) {
            if let Some(seen) = seen.get_mut(line) {
                *seen = true;
            }
        }
    };
    mark(&page);
    let mut view = page.dock;
    let mut turns = 0;
    while view.first + view.shown < total && turns < total {
        turns += 1;
        scroll_dock(sim, -(view.shown as f32));
        let frame = frame_at(recorder, sim, tally.size);
        let page = page_of(sim);
        judge(
            checks,
            tally,
            &format!("{name}, scrolled {turns}"),
            &page,
            &frame,
            false,
        );
        tally.surfaces -= 1;
        tally.pages += 1;
        mark(&page);
        checks.require(
            page.dock.first > view.first,
            "the wheel does not move a sheet longer than the dock",
            format!("{name}: {:?} after {view:?}", page.dock),
        );
        view = page.dock;
    }
    let missed: Vec<usize> = (0..total).filter(|&line| !seen[line]).collect();
    checks.require(
        missed.is_empty() && view.first == view.max_first,
        "scrolling the dock does not reach every line of its sheet",
        format!("{name}: lines {missed:?} of {total} never shown; ended at {view:?}"),
    );
    scroll_dock(sim, total as f32);
    checks.require(
        page_of(sim).dock.first == 0,
        "the wheel does not bring a scrolled sheet back to its top",
        format!("{name}: {:?}", page_of(sim).dock),
    );
}

/// Judge every surface at every size; a summary line per size.
pub fn check(checks: &mut Checks) -> Vec<String> {
    SIZES
        .iter()
        .map(|(label, size)| {
            let mut recorder = FrameRecorder::new(*size);
            let mut tally = Tally {
                size: *size,
                surfaces: 0,
                pages: 0,
                rows: 0,
                smallest: f32::MAX,
                clearance: f32::MAX,
                font: recorder.font_texture(),
            };
            battery(checks, &mut tally, &mut recorder, label);
            let camera = tally.camera();
            let pixels = tally.smallest * size.height as f32 / camera.height;
            format!(
                "floors, {label}: {} surfaces and {} scrolled pages, {} rows, smallest type {:.0} (floor {MIN_TEXT:.0}; {pixels:.1} px on this surface), closest type to the edge {:.2}",
                tally.surfaces, tally.pages, tally.rows, tally.smallest, tally.clearance
            )
        })
        .collect()
}

/// Every surface, at the tally's size.
fn battery(checks: &mut Checks, tally: &mut Tally, recorder: &mut FrameRecorder, label: &str) {
    let mut sim = session(crate::verify::SEEDS[0]);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "summer, nobody pointed at",
        false,
    );
    // The idle dock holds the help, and nothing else.
    let help = crate::verify::content_of(&sim).words[crate::words::W::SummerHelp].to_owned();
    let idle = crate::scripted::lines_in(&page_of(&sim), SHEET);
    checks.require(
        idle == [help],
        "the idle dock does not hold the help",
        format!("{label}: the dock reads {idle:?}"),
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
        let name = format!(
            "summer, {}'s sheet",
            sim.world().resource::<House>().heroes[*id].name
        );
        look(checks, tally, recorder, &mut sim, &name, false);
    }
    set_ui(&mut sim, UiState::family());
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "family, nobody pointed at",
        true,
    );
    let everyone = sim.world().resource::<House>().heroes.len();
    for id in 0..everyone {
        point_at(&mut sim, Target::Hero(id), false);
        let name = format!(
            "family, {}",
            sim.world().resource::<House>().heroes[id].name
        );
        look(checks, tally, recorder, &mut sim, &name, true);
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
        let name = format!(
            "W3, {}'s sheet",
            sim.world().resource::<House>().heroes[id].name
        );
        look(checks, tally, recorder, &mut sim, &name, false);
    }
    set_ui(&mut sim, UiState::family());
    point_at(&mut sim, Target::Hero(staged[0]), false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W3, the family on Garrick",
        true,
    );
    let w4 = w4_surfaces(checks, tally, recorder);
    let w5 = crate::floors_w5::w5_surfaces(checks, tally, recorder, label);
    let w6 = crate::floors_w6::w6_surfaces(checks, tally, recorder, label);
    checks.require(
        tally.surfaces == 2 + seated.len() + everyone + staged.len() + 1 + w4 + w5 + w6,
        "a surface was not judged",
        format!("{label}: {} surfaces", tally.surfaces),
    );
}

/// W4's surfaces: the hand mid-drag; both cards seated; both quest sheets and their
/// history panels; and a staged worst case — a troubled quest of three seats with
/// three seated, bonds, fears and patrons, beside a troubled empty quest whose
/// place has been visited and has its fallen. Returns how many were judged.
fn w4_surfaces(checks: &mut Checks, tally: &mut Tally, recorder: &mut FrameRecorder) -> usize {
    use crate::board::Slot;
    use crate::w4::{away, seat, stage_mid_drag};
    let before = tally.surfaces;
    let mut sim = session(crate::verify::SEEDS[0]);
    stage_mid_drag(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W4, Brannoc in hand over Grave goods",
        false,
    );
    mid_drag(checks, &sim);
    let mut sim = session(crate::verify::SEEDS[0]);
    seat(&mut sim, "Garrick", Slot::Quest { quest: 0, seat: 0 });
    seat(&mut sim, "Brannoc", Slot::Quest { quest: 0, seat: 1 });
    seat(&mut sim, "Maren", Slot::Quest { quest: 1, seat: 0 });
    seat(&mut sim, "Odo", Slot::Quest { quest: 1, seat: 1 });
    away(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W4, both cards seated",
        false,
    );
    for quest in 0..4 {
        point_at(&mut sim, Target::Quest(quest), false);
        let name = format!("W4, quest sheet {quest}, seated");
        look(checks, tally, recorder, &mut sim, &name, false);
    }
    let mut sim = session(crate::verify::SEEDS[0]);
    point_at(&mut sim, Target::Quest(1), false);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W4, the bell's sheet, nobody going",
        false,
    );
    // The worst case, staged on the session's own house.
    let mut sim = session(crate::verify::SEEDS[0]);
    crate::w4::stage_worst(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W4 staged, three on a troubled quest",
        false,
    );
    for quest in 0..4 {
        point_at(&mut sim, Target::Quest(quest), false);
        let name = format!("W4 staged, quest sheet {quest}");
        look(checks, tally, recorder, &mut sim, &name, false);
    }
    tally.surfaces - before
}

/// Mid-drag, the critical state: the hand, the card it is held over with its live
/// "you bring", and the roster it was lifted from all show, and none is under the
/// dock — which holds the held hero's sheet.
fn mid_drag(checks: &mut Checks, sim: &jidousha::prelude::HeadlessSim) {
    let page = page_of(sim);
    let Some(drag) = sim.world().resource::<UiState>().drag else {
        checks.require(false, "the mid-drag stage holds no one", String::new());
        return;
    };
    let card = crate::board_view::quest_rect(0);
    let hand = page
        .shapes
        .iter()
        .find(|s| s.layer == layers::HAND)
        .map(|s| s.rect);
    let bring = page
        .rows
        .iter()
        .find(|r| r.panel == card && r.text.starts_with("you bring"))
        .map(|r| r.bounds());
    let roster = crate::summer::card_rect(crate::summer::ROSTER_TOP, 0);
    let clear = |rect: Option<Rect>| rect.is_some_and(|r| !r.overlaps(SHEET));
    checks.require(
        clear(hand) && clear(bring) && clear(Some(card)) && clear(Some(roster)),
        "mid-drag, the hand, the card's live \"you bring\" or the roster is missing or under the dock",
        format!("hand {hand:?}, \"you bring\" {bring:?}, card {card:?}, roster {roster:?}"),
    );
    let sheet = crate::scripted::lines_in(&page, SHEET);
    let name = sim.world().resource::<House>().heroes[drag.hero]
        .name
        .clone();
    checks.require(
        sheet.first().is_some_and(|l| l.starts_with(&name)),
        "mid-drag, the dock does not hold the sheet of the hero in hand",
        format!("in hand {name}; the dock reads {:?}", sheet.first()),
    );
}
