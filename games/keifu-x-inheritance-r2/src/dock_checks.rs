//! The sheet dock as the player works it, driven by the scripted pointer: resting on
//! the dock keeps its sheet open; the wheel, a mouse grab and a finger all scroll a
//! sheet longer than the dock, and none scrolls past either end; a new subject opens
//! at its top; mid-drag the dock holds the hero in hand; and a hero released over
//! the dock goes back where they were.
//!
//! INVARIANT: the expectations are shipped literals (a name, a heading, a line of
//! Garrick's sheet as MODULES.md's W1 oracle gives it), never read off the dock.

use jidousha::prelude::*;

use crate::checks::Checks;
use crate::dock::{Subject, subject};
use crate::house::House;
use crate::screen::{Target, UiState};
use crate::scripted::{Pointer, center_of, lines_in};
use crate::summer::SHEET;
use crate::verify::{SEEDS, hero_named, page_of, point, point_at, scroll_dock, session};

fn ui(sim: &HeadlessSim) -> UiState {
    *sim.world().resource::<UiState>()
}

fn dock_lines(sim: &HeadlessSim) -> Vec<String> {
    lines_in(&page_of(sim), SHEET)
}

/// Every check on the dock's behaviour; returns the summary line.
pub fn check(checks: &mut Checks) -> String {
    let mut sim = session(SEEDS[0]);
    let garrick = hero_named(&sim, "Garrick");
    point_at(&mut sim, Target::Hero(garrick), false);
    checks.require(
        dock_lines(&sim).first().map(String::as_str) == Some("Garrick Thorne"),
        "pointing at Garrick opens his sheet in the dock, at its top",
        format!("{:?}", dock_lines(&sim).first()),
    );
    point(&mut sim, SHEET.center(), false);
    checks.require(
        subject(&ui(&sim)) == Subject::Hero(garrick),
        "the pointer moved from Garrick's card onto the dock keeps his sheet open",
        format!("{:?}", subject(&ui(&sim))),
    );
    let tall = page_of(&sim).dock;
    checks.require(
        tall.max_first > 0,
        "Garrick's founding sheet is longer than the dock (the case the scroll exists for)",
        format!("{tall:?}"),
    );
    scroll_dock(&mut sim, -3.0);
    checks.require(
        ui(&sim).dock_first == 3 && subject(&ui(&sim)) == Subject::Hero(garrick),
        "three lines of wheel toward the player scroll the sheet down three lines",
        format!("{:?}", ui(&sim)),
    );
    scroll_dock(&mut sim, -1000.0);
    let bottom = page_of(&sim).dock;
    checks.require(
        bottom.first == bottom.max_first
            && bottom.first + bottom.shown == bottom.total
            && dock_lines(&sim).iter().any(|l| l == "+1 Might on quests."),
        "the wheel stops with the sheet's last line in view, Thornfall's +1 Might on it",
        format!("{bottom:?}"),
    );
    scroll_dock(&mut sim, 1000.0);
    checks.require(
        ui(&sim).dock_first == 0,
        "the wheel stops at the sheet's top",
        format!("{:?}", ui(&sim)),
    );
    // A mouse grab: pressed in the dock and dragged up four lines' worth.
    let mut mouse = Pointer::mouse();
    let grip = SHEET.center();
    mouse.press(&mut sim, grip);
    mouse.hold_at(&mut sim, grip - Vec2::new(0.0, 4.0 * crate::dock::PITCH));
    let grabbed = ui(&sim).dock_first;
    mouse.release(&mut sim, grip - Vec2::new(0.0, 4.0 * crate::dock::PITCH));
    checks.require(
        grabbed == 4 && ui(&sim).drag.is_none(),
        "a press in the dock dragged up four lines scrolls the sheet four lines and picks no one up",
        format!("first {grabbed}, {:?}", ui(&sim)),
    );
    // A finger: the same grab, by touch, back down two lines.
    let mut finger = Pointer::finger();
    finger.press(&mut sim, grip);
    finger.hold_at(&mut sim, grip + Vec2::new(0.0, 2.0 * crate::dock::PITCH));
    let touched = ui(&sim).dock_first;
    finger.release(&mut sim, grip + Vec2::new(0.0, 2.0 * crate::dock::PITCH));
    checks.require(
        touched == 2,
        "a finger dragged down two lines in the dock scrolls the sheet back two",
        format!("first {touched}"),
    );
    // A new subject opens at its top.
    let maren = hero_named(&sim, "Maren");
    point_at(&mut sim, Target::Hero(maren), false);
    checks.require(
        ui(&sim).dock_first == 0
            && dock_lines(&sim).first().map(String::as_str) == Some("Maren Thorne"),
        "pointing at another hero opens their sheet at its top",
        format!("{:?}, {:?}", ui(&sim), dock_lines(&sim).first()),
    );
    // Mid-drag, the dock holds the hero in hand; released over the dock, they return.
    let brannoc = hero_named(&sim, "Brannoc");
    let seat = sim.world().resource::<House>().slot_of(brannoc);
    let from = center_of(&sim, Target::Hero(brannoc));
    mouse.press(&mut sim, from);
    mouse.hold_at(&mut sim, crate::board_view::quest_rect(0).center());
    let held = dock_lines(&sim).first().cloned();
    mouse.hold_at(&mut sim, SHEET.center());
    mouse.release(&mut sim, SHEET.center());
    checks.require(
        held.as_deref() == Some("Brannoc Hale"),
        "mid-drag the dock holds the sheet of the hero in hand",
        format!("{held:?}"),
    );
    checks.require(
        sim.world().resource::<House>().slot_of(brannoc) == seat && ui(&sim).drag.is_none(),
        "a hero released over the dock goes back to their seat",
        format!(
            "Brannoc at {:?}, was {seat:?}",
            sim.world().resource::<House>().slot_of(brannoc)
        ),
    );
    "dock: rests open under the pointer, scrolls by wheel, grab and finger within its ends, opens a new sheet at its top, holds the hero in hand, returns a hero dropped on it".to_owned()
}
