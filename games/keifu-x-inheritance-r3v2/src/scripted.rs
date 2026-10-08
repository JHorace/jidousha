//! The scripted pointer verify drags with, and the readings it takes off the card.
//!
//! A drag is a press, some moves and a release over several ticks, so it is fed
//! through `SnapshotBuilder` — the driver's own accumulator, which applies the edge
//! rules a real mouse goes through (docs/api/jidousha-testing.md: "send events, not
//! states"). A finger goes through the same builder as `Touched` events, so the
//! touch mirror is exercised by the same script.

use jidousha::prelude::*;
use jidousha::testing::{FingerId, InputEvent, InputSnapshot, SnapshotBuilder};

use crate::checks::fail;
use crate::screen::{Page, Target, camera};
use crate::verify::{page_of, target_rect};

/// One pointer — the mouse, or one finger — held across ticks.
pub struct Pointer {
    events: SnapshotBuilder,
    finger: Option<FingerId>,
}

impl Pointer {
    /// The mouse.
    pub fn mouse() -> Self {
        Self {
            events: SnapshotBuilder::new(),
            finger: None,
        }
    }

    /// One finger on the glass.
    pub fn finger() -> Self {
        Self {
            events: SnapshotBuilder::new(),
            finger: Some(FingerId::from_platform(1)),
        }
    }

    fn tick(&mut self, sim: &mut HeadlessSim) {
        sim.world_mut()
            .insert_resource(Input::new(self.events.first_tick_snapshot()));
        sim.tick();
    }

    fn at(&mut self, at: Vec2, phase: TouchPhase) {
        let screen = camera().world_to_screen(at);
        match self.finger {
            Some(finger) => self.events.record(InputEvent::Touched {
                finger,
                phase,
                screen,
            }),
            None => self.events.record(InputEvent::PointerMoved {
                id: PointerId::PRIMARY,
                screen,
            }),
        }
    }

    /// Put the pointer down at `at` (world units) and hold it.
    pub fn press(&mut self, sim: &mut HeadlessSim, at: Vec2) {
        self.at(at, TouchPhase::Began);
        if self.finger.is_none() {
            self.events.record(InputEvent::ButtonPressed {
                id: PointerId::PRIMARY,
                button: PointerButton::Primary,
            });
        }
        self.tick(sim);
    }

    /// Move it, still held, to `at`.
    pub fn hold_at(&mut self, sim: &mut HeadlessSim, at: Vec2) {
        self.at(at, TouchPhase::Moved);
        self.tick(sim);
    }

    /// Let go at `at`.
    pub fn release(&mut self, sim: &mut HeadlessSim, at: Vec2) {
        match self.finger {
            Some(_) => self.at(at, TouchPhase::Ended),
            None => {
                self.at(at, TouchPhase::Ended);
                self.events.record(InputEvent::ButtonReleased {
                    id: PointerId::PRIMARY,
                    button: PointerButton::Primary,
                });
            }
        }
        self.tick(sim);
        sim.world_mut()
            .insert_resource(Input::new(InputSnapshot::new()));
    }

    /// Press and let go at `at` on one tick: a tap.
    pub fn press_release(&mut self, sim: &mut HeadlessSim, at: Vec2) {
        self.at(at, TouchPhase::Began);
        self.events.record(InputEvent::ButtonPressed {
            id: PointerId::PRIMARY,
            button: PointerButton::Primary,
        });
        self.events.record(InputEvent::ButtonReleased {
            id: PointerId::PRIMARY,
            button: PointerButton::Primary,
        });
        self.tick(sim);
        sim.world_mut()
            .insert_resource(Input::new(InputSnapshot::new()));
    }

    /// The system takes the pointer away mid-drag: the window loses focus, or the
    /// touch is cancelled.
    pub fn lose(&mut self, sim: &mut HeadlessSim, at: Vec2) {
        match self.finger {
            Some(_) => self.at(at, TouchPhase::Cancelled),
            None => self.events.record(InputEvent::FocusLost),
        }
        self.tick(sim);
        if self.finger.is_none() {
            self.events.record(InputEvent::FocusGained);
        }
        sim.world_mut()
            .insert_resource(Input::new(InputSnapshot::new()));
    }
}

/// The centre of `target` on the page now; fails the run if it is not there.
pub fn center_of(sim: &HeadlessSim, target: Target) -> Vec2 {
    match target_rect(sim, target) {
        Some(rect) => rect.center(),
        None => fail(
            "a target the drag needs is not on the page",
            &format!("{target:?} is not among the page's hit targets"),
        ),
    }
}

/// Drag whatever is under `from` to `to`, through the midpoint, with `pointer`.
pub fn drag(sim: &mut HeadlessSim, pointer: &mut Pointer, from: Vec2, to: Vec2) {
    pointer.press(sim, from);
    pointer.hold_at(sim, (from + to) * 0.5);
    pointer.hold_at(sim, to);
    pointer.release(sim, to);
}

/// The logical lines whose rows lie in `panel`, top to bottom.
pub fn lines_in(page: &Page, panel: Rect) -> Vec<String> {
    page.logical_lines()
        .into_iter()
        .filter(|(_, rows)| rows.iter().all(|&r| page.rows[r].panel == panel))
        .map(|(line, _)| line)
        .collect()
}

/// The lines on board slot `quest`'s card as the page shows them now.
pub fn card_lines(sim: &HeadlessSim, quest: usize) -> Vec<String> {
    lines_in(&page_of(sim), crate::board_view::quest_rect(quest))
}
