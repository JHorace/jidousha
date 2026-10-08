//! The decision-surface checks: staged stacks, driven through the real input
//! path, judged on the recorded frame and against what then resolves.
//!
//! One check per row of the task's decision table (DESIGN.md, "Decision
//! surfaces"): `respond-window`, `pass-resolves-as-previewed`,
//! `target-reorder`, `target-counter`. Every expected string and life total is
//! a shipped literal worked out by hand from the rules, never read back from
//! the game — a check that asked the game what to expect could not see the
//! game change.
//!
//! Key items: `Stage`, `staged`, `run_all`.

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder,
};

use crate::checks::{Checks, judge, on_screen, panel_has};
use crate::rules::{Card, Duel, Item, ItemId, Side, Step, deal, preview};
use crate::screen::{Art, aim_targets, screen, stack_row};
use crate::{Table, WINDOW, camera, config, palette, register};
use jidousha::ui::Panel;

/// Ticks a stage waits for the rival before calling it stuck.
const PATIENCE: u32 = 600;

/// One staged duel, running in the real game.
pub(crate) struct Stage {
    sim: HeadlessSim,
    recorder: FrameRecorder,
    keyboard: SnapshotBuilder,
    pub(crate) font: BackendTextureId,
}

impl Stage {
    pub(crate) fn new(duel: Duel) -> Self {
        let mut sim = headless(config(), register);
        sim.tick();
        let table = sim.world_mut().resource_mut::<Table>();
        table.duel = duel;
        table.choosing = None;
        table.rival_clock = 0;
        let recorder = FrameRecorder::new(WINDOW);
        let font = recorder.font_texture();
        Self {
            sim,
            recorder,
            keyboard: SnapshotBuilder::new(),
            font,
        }
    }

    pub(crate) fn table(&self) -> &Table {
        self.sim.world().resource::<Table>()
    }

    pub(crate) fn table_mut(&mut self) -> &mut Table {
        self.sim.world_mut().resource_mut::<Table>()
    }

    fn step(&mut self) {
        let snapshot = self.keyboard.first_tick_snapshot();
        self.sim.world_mut().insert_resource(Input::new(snapshot));
        self.sim.tick();
    }

    /// Tap `key` for one tick.
    pub(crate) fn press(&mut self, key: Key) {
        self.keyboard.record(InputEvent::KeyPressed(key));
        self.keyboard.record(InputEvent::KeyReleased(key));
        self.step();
    }

    /// Click the primary button at world point `at`, for one tick.
    pub(crate) fn click(&mut self, at: Vec2) {
        let screen = camera().world_to_screen(at);
        let id = PointerId::PRIMARY;
        let button = PointerButton::Primary;
        self.keyboard
            .record(InputEvent::PointerMoved { id, screen });
        self.keyboard
            .record(InputEvent::ButtonPressed { id, button });
        self.keyboard
            .record(InputEvent::ButtonReleased { id, button });
        self.step();
    }

    /// Let the rival act until you hold priority or the match is over; how
    /// many ticks that took, or `None` if it never came back.
    pub(crate) fn ticks_to_priority(&mut self) -> Option<u32> {
        for waited in 0..PATIENCE {
            let duel = &self.table().duel;
            if duel.priority == Side::You || duel.outcome.is_some() {
                return Some(waited);
            }
            self.step();
        }
        None
    }

    pub(crate) fn wait_for_priority(&mut self) -> bool {
        self.ticks_to_priority().is_some()
    }

    /// Pass whenever you hold priority until the stack is empty.
    pub(crate) fn pass_it_all(&mut self) -> bool {
        for _ in 0..PATIENCE * 4 {
            let duel = &self.table().duel;
            if duel.stack.is_empty() || duel.outcome.is_some() {
                return true;
            }
            if duel.priority == Side::You {
                self.press(Key::Space);
            } else {
                self.step();
            }
        }
        false
    }

    /// One frame of whatever the table is showing, and the panel it says.
    pub(crate) fn frame(&mut self) -> (FrameRecord, Panel<Art>) {
        let panel = screen(self.table());
        (self.recorder.draw(&mut self.sim), panel)
    }
}

/// A duel with `stack` on it (bottom first; a target is an index into it),
/// your `hand` and `focus`, an empty-handed rival leading, you on priority.
pub(crate) fn staged(
    stack: &[(Card, Side, Option<usize>)],
    hand: &[Card],
    focus: i32,
    passed: bool,
) -> Duel {
    let mut duel = deal(&mut Rng::from_seed(1));
    duel.you.hand = hand.to_vec();
    duel.you.focus = focus;
    duel.rival.hand.clear();
    duel.rival.focus = 0;
    duel.leader = Side::Rival;
    duel.priority = Side::You;
    duel.passed = passed;
    duel.stack = stack
        .iter()
        .enumerate()
        .map(|(at, &(card, controller, target))| Item {
            id: ItemId(at as u32 + 1),
            card,
            controller,
            target: target.map(|index| ItemId(index as u32 + 1)),
            copy: false,
        })
        .collect();
    duel.next_id = stack.len() as u32 + 1;
    duel
}

fn view() -> Rect {
    camera().visible_bounds()
}

/// Every literal row in `rows` is on the panel the frame was judged against.
fn shows(checks: &mut Checks, check: &str, panel: &Panel<Art>, rows: &[&str]) {
    for row in rows {
        checks.require(
            panel_has(panel, row),
            "a decision surface does not say what the stack will do",
            format!("{check}: no row reads {row:?}"),
        );
    }
}

/// What resolved since `before`, by card, in order.
fn resolved_since(table: &Table, before: usize) -> Vec<Step> {
    table.duel.resolved[before..].to_vec()
}

fn lives(table: &Table) -> (i32, i32) {
    (table.duel.you.life, table.duel.rival.life)
}

/// The outlined rows on a frame: which stack slots carry a mark-coloured quad
/// that is not a glyph — the outline, not the `<- target` tag beside it.
fn marked_slots(frame: &FrameRecord, font: BackendTextureId) -> Vec<usize> {
    (0..crate::screen::STACK_ROWS)
        .filter(|&slot| {
            let row = stack_row(slot);
            frame.quads().iter().any(|quad| {
                quad.tint == palette::MARK
                    && quad.texture != font
                    && row.contains(quad.bounds().center())
            })
        })
        .collect()
}

/// Row one: with three items up, everything is shown; a response changes the
/// preview, and the stack then resolves exactly as the new preview said.
fn respond_window(checks: &mut Checks) -> Option<FrameRecord> {
    let check = "respond-window";
    let mut stage = Stage::new(staged(
        &[
            (Card::Haymaker, Side::Rival, None),
            (Card::Ward, Side::You, None),
            (Card::Strike, Side::Rival, None),
        ],
        &[Card::Bury, Card::Turn, Card::Strike],
        2,
        false,
    ));
    let (frame, panel) = stage.frame();
    judge(checks, check, &panel, &frame, stage.font, view());
    shows(
        checks,
        check,
        &panel,
        &[
            "A  Strike (rival)",
            "resolves 1st: you take 2",
            "B  Ward (you)",
            "resolves 2nd: you +3 shield",
            "C  Haymaker (rival)",
            "resolves 3rd: you take 2 (3 soaked)",
            "if both pass: you 8 - rival 12",
            "YOU",
            "rival may still respond",
            "1 BURY",
            "2 TURN",
            "3 STRIKE",
        ],
    );

    // Aim Turn: only the rival's payloads light — Strike (A) and Haymaker (C).
    stage.press(Key::Digit2);
    let (aiming, panel) = stage.frame();
    judge(checks, "aiming", &panel, &aiming, stage.font, view());
    let marked = marked_slots(&aiming, stage.font);
    let legal = aim_targets(stage.table());
    checks.require(
        marked == vec![0, 2] && legal == marked,
        "the marked targets are not the legal ones",
        format!("{check}: marked {marked:?}, legal_targets {legal:?}, want [0, 2]"),
    );
    on_screen(checks, "aiming", &aiming, view());

    stage.press(Key::C);
    // The rival reads for 45 ticks — the play's own tick and 44 more — so a
    // person sees it think (main.rs, `RIVAL_THINK`).
    let waited = stage.ticks_to_priority();
    checks.require(
        waited == Some(44),
        "the rival did not take its time to answer",
        format!("{check}: priority came back after {waited:?} ticks, want Some(44)"),
    );
    if waited.is_none() {
        return Some(aiming);
    }
    let (frame, panel) = stage.frame();
    judge(checks, check, &panel, &frame, stage.font, view());
    shows(
        checks,
        check,
        &panel,
        &[
            "A  Turn (you) -> D",
            "resolves 1st: Haymaker now works for you",
            "B  Strike (rival)",
            "resolves 2nd: you take 2",
            "C  Ward (you)",
            "resolves 3rd: you +3 shield",
            "D  Haymaker (rival)",
            "resolves 4th: rival takes 5",
            "if both pass: you 10 - rival 7",
            "rival passed: a pass resolves A",
        ],
    );
    let shown = preview(&stage.table().duel);
    let before = stage.table().duel.resolved.len();
    let finished = stage.pass_it_all();
    let actual = resolved_since(stage.table(), before);
    let got = lives(stage.table());
    checks.require(
        finished && actual == shown.steps && got == (10, 7),
        "the response did not resolve as the panel previewed",
        format!(
            "{check}: {} steps resolved against {} previewed, lives {got:?}, want (10, 7)",
            actual.len(),
            shown.steps.len()
        ),
    );
    checks.note(format!(
        "{check}: 3 items shown with order, Turn aimed at [A, C], resolved to {got:?}"
    ));
    Some(aiming)
}

/// Row two: pass, and the stack — a Bury and an Echo in it — resolves exactly
/// as previewed, step for step.
fn pass_resolves(checks: &mut Checks) {
    let check = "pass-resolves-as-previewed";
    let mut stage = Stage::new(staged(
        &[
            (Card::Haymaker, Side::Rival, None),
            (Card::Echo, Side::You, Some(0)),
            (Card::Bury, Side::Rival, Some(1)),
        ],
        &[Card::Strike],
        0,
        true,
    ));
    let (frame, panel) = stage.frame();
    judge(checks, check, &panel, &frame, stage.font, view());
    shows(
        checks,
        check,
        &panel,
        &[
            "A  Bury (rival) -> B",
            "resolves 1st: sinks Echo to base",
            "B  Echo (you) -> C",
            "resolves 3rd: fizzles: target gone",
            "C  Haymaker (rival)",
            "resolves 2nd: you take 5",
            "if both pass: you 7 - rival 12",
            "rival passed: a pass resolves A",
        ],
    );
    let shown = preview(&stage.table().duel);
    // The first pass is a click on the PASS button: it resolves the top.
    stage.click(crate::screen::PASS_BUTTON.center());
    let clicked = stage.table().duel.resolved.len();
    checks.require(
        clicked == 1,
        "clicking PASS did not pass",
        format!("{check}: {clicked} resolutions after the click, want 1"),
    );
    let finished = stage.pass_it_all();
    let actual = resolved_since(stage.table(), 0);
    let order: Vec<&str> = actual.iter().map(|step| step.item.card.name()).collect();
    let got = lives(stage.table());
    checks.require(
        finished
            && actual == shown.steps
            && order == ["Bury", "Haymaker", "Echo"]
            && got == (7, 12),
        "passing did not resolve the stack as previewed",
        format!(
            "{check}: resolved {order:?}, lives {got:?}; want Bury, Haymaker, Echo and (7, 12)"
        ),
    );
    checks.note(format!("{check}: resolved {order:?} to {got:?}, as shown"));
}

/// Row three, reorder: Bury lights only what it can move; an unlit row is
/// refused; the order shown after the play is the order that resolves.
fn target_reorder(checks: &mut Checks) {
    let check = "target-reorder";
    let mut stage = Stage::new(staged(
        &[
            (Card::Ward, Side::You, None),
            (Card::Haymaker, Side::Rival, None),
        ],
        &[Card::Bury],
        1,
        false,
    ));
    stage.press(Key::Digit1);
    let (frame, _) = stage.frame();
    let marked = marked_slots(&frame, stage.font);
    checks.require(
        marked == vec![0] && aim_targets(stage.table()) == marked,
        "Bury lights a row it cannot move",
        format!("{check}: marked {marked:?}, want [0] (the bottom item is never a Bury target)"),
    );
    stage.press(Key::B);
    let refused = stage.table().duel.stack.len() == 2 && stage.table().choosing == Some(0);
    checks.require(
        refused,
        "an unlit row took the Bury",
        format!(
            "{check}: after B the stack holds {} items, choosing {:?}",
            stage.table().duel.stack.len(),
            stage.table().choosing
        ),
    );
    stage.press(Key::A);
    let came_back = stage.wait_for_priority();
    let (frame, panel) = stage.frame();
    judge(checks, check, &panel, &frame, stage.font, view());
    shows(
        checks,
        check,
        &panel,
        &[
            "A  Bury (you) -> B",
            "resolves 1st: sinks Haymaker to base",
            "B  Haymaker (rival)",
            "resolves 3rd: you take 2 (3 soaked)",
            "C  Ward (you)",
            "resolves 2nd: you +3 shield",
        ],
    );
    let finished = came_back && stage.pass_it_all();
    let actual = resolved_since(stage.table(), 0);
    let order: Vec<&str> = actual.iter().map(|step| step.item.card.name()).collect();
    let got = lives(stage.table());
    checks.require(
        finished && order == ["Bury", "Ward", "Haymaker"] && got == (10, 12),
        "the order shown after the Bury is not the order that resolved",
        format!("{check}: shown Bury, Ward, Haymaker; resolved {order:?}, lives {got:?}"),
    );
    checks.note(format!(
        "{check}: marked [A], refused B, resolved {order:?} to {got:?}"
    ));
}

/// Row three, counter: Cancel lights the Haymaker; it never resolves.
fn target_counter(checks: &mut Checks) {
    let check = "target-counter";
    let mut stage = Stage::new(staged(
        &[(Card::Haymaker, Side::Rival, None)],
        &[Card::Cancel],
        2,
        false,
    ));
    // By pointer this time: the hand card, then the lit row.
    stage.click(crate::screen::hand_card(0).center());
    let (frame, _) = stage.frame();
    let marked = marked_slots(&frame, stage.font);
    stage.click(stack_row(0).center());
    let came_back = stage.wait_for_priority();
    let (frame, panel) = stage.frame();
    judge(checks, check, &panel, &frame, stage.font, view());
    shows(
        checks,
        check,
        &panel,
        &[
            "A  Cancel (you) -> B",
            "resolves 1st: removes Haymaker",
            "B  Haymaker (rival)",
            "does not resolve",
        ],
    );
    let finished = came_back && stage.pass_it_all();
    let actual = resolved_since(stage.table(), 0);
    let order: Vec<&str> = actual.iter().map(|step| step.item.card.name()).collect();
    let got = lives(stage.table());
    checks.require(
        marked == vec![0] && finished && order == ["Cancel"] && got == (12, 12),
        "the counter did not land where it was aimed",
        format!("{check}: marked {marked:?}, resolved {order:?}, lives {got:?}"),
    );
    checks.note(format!(
        "{check}: marked [A], resolved {order:?} to {got:?}"
    ));
}

/// Every decision-row check; hands back the aiming frame for the capture.
pub(crate) fn run_all(checks: &mut Checks) -> Option<FrameRecord> {
    let aiming = respond_window(checks);
    pass_resolves(checks);
    target_reorder(checks);
    target_counter(checks);
    aiming
}
