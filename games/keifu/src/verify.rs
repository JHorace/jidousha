//! `--verify`: the headless run `tools/verify keifu` drives.
//!
//! It plays the game the window plays — the same `register`, the same camera —
//! with the pointer scripted, and asserts:
//!
//! 1. **The two oracles** of `spec/MODULES.md`, on several seeds: W0's top bar,
//!    and W1's whole Garrick sheet, both as the page's rows and as glyphs in the
//!    recorded frame. The expectations are shipped literals copied from
//!    MODULES.md, never computed from the code under test (`oracles.rs`).
//! 2. **The founding** and every derived quantity W1 owns (`oracles.rs`).
//! 3. **W2's oracle** — Maren and Garrick's fear line and power on the bell — and
//!    a staged story over every W2 rule (`w2.rs`). The oracle's three strings are
//!    printed, labelled, for the owner to hold beside the original's card.
//! 4. **W3's oracle** — Garrick's dream call on a Barrow quest, the card's mark and
//!    the quest sheet's line, a staged triumph that settles him and lays "Garrick's
//!    rest" on his line — and a staged story over witnessing, fulfilment, legacies
//!    and dream rivals (`w3.rs`). The vector is printed, labelled, the same way.
//! 5. **W4's oracle**, the first played one — Garrick and Brannoc dragged onto
//!    "Grave goods" by the scripted pointer, the card read mid-hold and seated, the
//!    demand's band and CONSTANTS §3's mapping over a sweep of seeds, the quest
//!    sheet's §6 lines — then the drags that do not seat, W4's rules, and W2's
//!    and W3's oracles graduated onto the real card (`w4.rs`, `w4_rules.rs`).
//! 6. **W5's oracle**, the first distributional one — year 1's invariant off the
//!    drawn cards on the recorded seeds and off the card reading on a fresh sweep;
//!    the shape ("most summers carry a fair-chance Dream: mark") and the loop's
//!    rules over a fixed battery of seeds run through their summers, with the
//!    distribution printed; and the ghost slot on staged ghost lists (`w5.rs`,
//!    `w5_shape.rs`).
//! 7. **The W0 machinery**: calendar, text conventions, pools, bags (`foundations.rs`).
//! 8. **Readability floors** over every surface this build has, every page of a
//!    sheet longer than the dock, at the native window and two web canvases (`floors.rs`).
//! 9. **The sheet dock** as the player works it: resting, scrolling, a new subject,
//!    the hero in hand, a drop on it (`dock_checks.rs`).
//! 10. **The cast's sprites**: every role imported, every card its role's texture (`cast.rs`).
//! 11. **A picture** of each oracle's screen (`capture.rs`).

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{
    FrameRecord, FrameRecorder, InputScript, InputSnapshot, MemorySource, decode_png,
};

use crate::checks::{Checks, fail};
use crate::content::Content;
use crate::house::{House, RunSeed};
use crate::screen::{Page, Row, Target, UiState, WINDOW, camera};

/// The seeds every oracle is asked on. They must not matter (MODULES.md: the
/// oracles are fully determined by the authored household).
pub const SEEDS: [u64; 3] = [1, 0x5eed, 0xdead_beef];

/// The tick the scripted art arrives on: the first one, so every recorded frame has it.
pub const ART_ARRIVES: u64 = 1;

/// Every sprite file, baked into the binary, by the path the game asks for it by.
///
/// A verify run reads no files; the bytes are the ones the window shows, so a
/// picture that stopped decoding fails here rather than passing on a stub.
pub const ART_FILES: [(&str, &[u8]); 10] = [
    (
        "hero_knight.png",
        include_bytes!("../assets/hero_knight.png"),
    ),
    (
        "hero_warrior.png",
        include_bytes!("../assets/hero_warrior.png"),
    ),
    (
        "hero_ranger.png",
        include_bytes!("../assets/hero_ranger.png"),
    ),
    (
        "hero_scholar.png",
        include_bytes!("../assets/hero_scholar.png"),
    ),
    (
        "hero_priest.png",
        include_bytes!("../assets/hero_priest.png"),
    ),
    ("hero_sage.png", include_bytes!("../assets/hero_sage.png")),
    ("hero_elder.png", include_bytes!("../assets/hero_elder.png")),
    (
        "hero_hermit.png",
        include_bytes!("../assets/hero_hermit.png"),
    ),
    ("hero_child.png", include_bytes!("../assets/hero_child.png")),
    (
        "heirloom_blade.png",
        include_bytes!("../assets/heirloom_blade.png"),
    ),
];

/// The art store a verify run uses: the real PNGs, arriving on `ART_ARRIVES`.
pub fn store() -> Assets {
    let mut source = MemorySource::new();
    for (path, bytes) in ART_FILES {
        let texture = match decode_png(bytes) {
            Ok(texture) => texture,
            Err(error) => fail(
                "a sprite no longer decodes",
                &format!(
                    "games/keifu/assets/{path} is baked into verify and does not decode: {error}"
                ),
            ),
        };
        source.insert_texture(path, texture);
        source.complete_at(path, ART_ARRIVES);
    }
    Assets::new(source)
}

/// A fresh game on `seed`, founded (Startup has run), with the scripted art store.
pub fn session(seed: u64) -> HeadlessSim {
    let mut sim = headless(crate::config(seed), crate::register);
    sim.world_mut().insert_resource(RunSeed(seed));
    // Before Startup, which installs the file-backed store only if none is here.
    sim.world_mut().insert_resource(store());
    sim.tick();
    sim
}

/// Settle the art, then record one frame — every frame this run records.
pub fn frame(recorder: &mut FrameRecorder, sim: &mut HeadlessSim) -> FrameRecord {
    recorder.settle_assets(sim, ART_ARRIVES);
    recorder.draw(sim)
}

/// Move the pointer to `at` (world units) for one tick, clicking if asked.
pub fn point(sim: &mut HeadlessSim, at: Vec2, click: bool) {
    let mut script = InputScript::new().pointer_at(1, camera().world_to_screen(at));
    if click {
        script = script.click(PointerButton::Primary, 1);
    }
    sim.world_mut()
        .insert_resource(Input::new(script.snapshot_at(1)));
    sim.tick();
    sim.world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));
}

/// The page as the game reads it now.
pub fn page_of(sim: &HeadlessSim) -> Page {
    crate::read_the_page(&sim.world().view())
}

/// Where `target` is on the current page.
pub fn target_rect(sim: &HeadlessSim, target: Target) -> Option<Rect> {
    page_of(sim)
        .targets
        .iter()
        .find(|(_, t)| *t == target)
        .map(|(rect, _)| *rect)
}

/// Point at `target` (and click it, if asked). Fails the run if it is not on screen.
pub fn point_at(sim: &mut HeadlessSim, target: Target, click: bool) {
    let Some(rect) = target_rect(sim, target) else {
        fail(
            "a target the run needs is not on the page",
            &format!("{target:?} is not among the page's hit targets"),
        );
    };
    point(sim, rect.center(), click);
}

/// Turn the wheel `lines` with the pointer resting on the sheet dock, for one tick:
/// negative is toward the player, down the sheet.
pub fn scroll_dock(sim: &mut HeadlessSim, lines: f32) {
    use jidousha::testing::{InputEvent, SnapshotBuilder};
    let mut events = SnapshotBuilder::new();
    events.record(InputEvent::PointerMoved {
        id: PointerId::PRIMARY,
        screen: camera().world_to_screen(crate::summer::SHEET.center()),
    });
    events.record(InputEvent::Scrolled {
        id: PointerId::PRIMARY,
        lines,
    });
    sim.world_mut()
        .insert_resource(Input::new(events.first_tick_snapshot()));
    sim.tick();
    sim.world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));
}

/// The open sheet, paged through the dock from where it is to its end by the wheel,
/// a dock's worth of lines a turn: each page as the game reads it, and its frame.
/// A page shows whole lines only; the last page can repeat lines from the one
/// before it, where the scroll stops at the sheet's end (`dock_read` takes each line
/// once).
pub fn dock_pages(sim: &mut HeadlessSim, recorder: &mut FrameRecorder) -> Vec<(Page, FrameRecord)> {
    let mut pages = Vec::new();
    loop {
        let record = frame(recorder, sim);
        let page = page_of(sim);
        let view = page.dock;
        pages.push((page, record));
        if view.first + view.shown >= view.total || pages.len() > view.total {
            return pages;
        }
        scroll_dock(sim, -(view.shown as f32));
    }
}

/// One line of the dock's sheet, as paged through: what it says, the page it was
/// drawn on, and its rows there.
pub struct DockLine {
    /// The logical line.
    pub text: String,
    /// Which page of `dock_pages`.
    pub page: usize,
    /// Its rows on that page.
    pub rows: Vec<usize>,
}

/// The sheet's lines across the pages, in order, each once: on each page, the
/// logical lines set in the dock for sheet lines no earlier page showed.
pub fn dock_read(pages: &[(Page, FrameRecord)]) -> Vec<DockLine> {
    let mut read = Vec::new();
    let mut unseen = 0;
    for (index, (page, _)) in pages.iter().enumerate() {
        let mut next = unseen;
        for (text, rows) in page.logical_lines() {
            let Some(line) = page.rows[rows[0]].dock_line else {
                continue;
            };
            if line < unseen {
                continue;
            }
            next = next.max(line + 1);
            read.push(DockLine {
                text,
                page: index,
                rows,
            });
        }
        unseen = next;
    }
    read
}

/// The id of the founding hero named `name`.
pub fn hero_named(sim: &HeadlessSim, name: &str) -> usize {
    let house = sim.world().resource::<House>();
    match house.heroes.iter().position(|hero| hero.name == name) {
        Some(id) => id,
        None => fail(
            "a founding hero is missing",
            &format!("no hero is named {name:?}"),
        ),
    }
}

/// Whether `row` was drawn: exactly one font quad per character, inside its box.
pub fn row_drawn(
    frame: &FrameRecord,
    font: jidousha::testing::BackendTextureId,
    row: &Row,
) -> bool {
    let bounds = row.bounds();
    let inside = frame
        .quads()
        .into_iter()
        .filter(|quad| quad.texture == font)
        .filter(|quad| {
            let b = quad.bounds();
            b.min.x >= bounds.min.x - 0.01
                && b.min.y >= bounds.min.y - 0.01
                && b.max.x <= bounds.max.x + 0.01
                && b.max.y <= bounds.max.y + 0.01
        })
        .count();
    inside >= row.text.chars().count()
}

/// Run every check, print the verdict and the summary, and capture the pictures.
pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => fail("the content did not load", &error.to_string()),
    };
    let mut summary = Vec::new();
    summary.push(crate::foundations::check(&mut checks, &content));
    summary.push(crate::oracles::check_founding(&mut checks));
    let mut recorder = FrameRecorder::new(WINDOW);
    let mut garrick_frame = None;
    for seed in SEEDS {
        let (line, frame) = crate::oracles::check_w0_and_w1(&mut checks, &mut recorder, seed);
        if seed == SEEDS[0] {
            summary.push(line);
            garrick_frame = Some(frame);
        }
    }
    let (w2_line, w2_vector) = crate::w2::check_oracle(&mut checks);
    summary.push(w2_line);
    summary.extend(w2_vector);
    summary.push(crate::w2::check_rules(&mut checks));
    let (w3_line, w3_vector) = crate::w3::check_oracle(&mut checks);
    summary.push(w3_line);
    summary.extend(w3_vector);
    summary.push(crate::w3::check_rules(&mut checks));
    let (w4_line, w4_vector) = crate::w4::check_oracle(&mut checks);
    summary.push(w4_line);
    summary.extend(w4_vector);
    summary.push(crate::w4::check_drags(&mut checks));
    summary.push(crate::w4_rules::check_rules(&mut checks));
    let (inherited, inherited_vector) = crate::w4_rules::check_inherited(&mut checks);
    summary.push(inherited);
    summary.extend(inherited_vector);
    summary.extend(crate::w5::check_invariant(&mut checks, &content));
    summary.extend(crate::w5_shape::check_shape(&mut checks, &content));
    summary.push(crate::w5::check_ghost_slot(&mut checks, &content));
    summary.push(crate::sessions::check_family(&mut checks, &mut recorder));
    summary.push(crate::sessions::check_seeds(&mut checks, &content));
    summary.push(crate::sessions::check_staged_sheets(&mut checks));
    summary.push(crate::cast::check_art(&mut checks, &mut recorder));
    summary.push(crate::dock_checks::check(&mut checks));
    summary.extend(crate::floors::check(&mut checks));
    let Some(garrick_frame) = garrick_frame else {
        fail("no frame of Garrick's sheet was recorded", "SEEDS is empty");
    };
    let captures = crate::capture::capture_all(&mut checks);

    let (passed, failed) = checks.counts();
    if failed == 0 {
        println!(
            "verified keifu: W0, W1, W2, W3, W4 and W5 oracles hold on {} seeds, {passed} checks",
            SEEDS.len()
        );
    } else {
        println!(
            "verify keifu FAILED: {failed} of {} checks",
            passed + failed
        );
    }
    for line in &summary {
        println!("  {line}");
    }
    for line in &captures {
        println!("  {line}");
    }
    println!();
    println!("{}", garrick_frame.transcript());
    checks.verdict()
}

/// The UI state, for staging a screen.
pub fn set_ui(sim: &mut HeadlessSim, ui: UiState) {
    sim.world_mut().insert_resource(ui);
}

/// The content, read back from a session.
pub fn content_of(sim: &HeadlessSim) -> &Content {
    sim.world().resource::<Content>()
}
