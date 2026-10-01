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
//! 3. **The W0 machinery**: calendar, text conventions, pools, bags (`foundations.rs`).
//! 4. **Readability floors** over every surface this build has (`floors.rs`).
//! 5. **A picture** of each oracle's screen (`capture.rs`).

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, InputScript, InputSnapshot};

use crate::checks::{Checks, fail};
use crate::content::Content;
use crate::house::{House, RunSeed};
use crate::screen::{Page, Row, Target, UiState, WINDOW, camera};

/// The seeds every oracle is asked on. They must not matter (MODULES.md: the
/// oracles are fully determined by the authored household).
pub const SEEDS: [u64; 3] = [1, 0x5eed, 0xdead_beef];

/// A fresh game on `seed`, founded (Startup has run).
pub fn session(seed: u64) -> HeadlessSim {
    let mut sim = headless(crate::config(seed), crate::register);
    sim.world_mut().insert_resource(RunSeed(seed));
    sim.tick();
    sim
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
    summary.push(crate::sessions::check_family(&mut checks, &mut recorder));
    summary.push(crate::sessions::check_seeds(&mut checks, &content));
    summary.push(crate::sessions::check_staged_sheets(&mut checks));
    summary.push(crate::floors::check(&mut checks, &mut recorder));
    let Some(garrick_frame) = garrick_frame else {
        fail("no frame of Garrick's sheet was recorded", "SEEDS is empty");
    };
    let captures = crate::capture::capture_all(&mut checks, &mut recorder, &garrick_frame);

    let (passed, failed) = checks.counts();
    if failed == 0 {
        println!(
            "verified keifu: W0 and W1 oracles hold on {} seeds, {passed} checks",
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
