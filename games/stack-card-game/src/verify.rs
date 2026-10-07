//! `--verify`: the same systems and config as the window, driven headless by
//! scripted key presses, asserting on the rules, on the screen model the
//! player reads, and on what was drawn. `tools/verify stack-card-game` runs it.
//!
//! The three decision rows of the spec each have their scenario below
//! (`decision_stack`, `decision_pass`, `decision_target`); the rules, three
//! players and determinism checks sit beside them. Every scripted action goes
//! in through the keyboard path a person uses.

use crate::cards::{Card, Side};
use crate::checks::{Checks, fail, greater};
use crate::game::{Game, SLOT_KEYS};
use crate::players;
use crate::rowcheck::{Expect, check_rows, glyphs_in};
use crate::rulechecks::{self, SWEEP_SEEDS};
use crate::rules::{Action, Core, preview};
use crate::ui::{self, Mark};
use crate::{FIRST_SEED, config, register};
use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, InputScript, InputSnapshot};
use std::process::ExitCode;

pub(super) const VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// A headless run of the game, a recorder, and the keys to drive it with.
struct Session {
    sim: HeadlessSim,
    recorder: FrameRecorder,
    ticks: u64,
}

impl Session {
    fn new() -> Session {
        let mut sim = headless(config(), register);
        // Startup runs inside the first tick.
        sim.world_mut()
            .insert_resource(Input::new(InputSnapshot::new()));
        sim.tick();
        Session {
            sim,
            recorder: FrameRecorder::new(VIEWPORT),
            ticks: 1,
        }
    }

    fn game(&self) -> &Game {
        self.sim.world().resource::<Game>()
    }

    fn stage(&mut self, change: impl FnOnce(&mut Core)) {
        let game = self.sim.world_mut().resource_mut::<Game>();
        game.ui = Default::default();
        change(&mut game.core);
    }

    fn step(&mut self, key: Option<Key>) {
        let snapshot = match key {
            Some(key) => InputScript::new().press(key, 1).snapshot_at(1),
            None => InputSnapshot::new(),
        };
        self.sim.world_mut().insert_resource(Input::new(snapshot));
        self.sim.tick();
        self.ticks += 1;
    }

    fn tap(&mut self, keys: &[Key]) {
        for &key in keys {
            self.step(Some(key));
        }
    }

    /// Tick until the player holds priority (or the match is over).
    fn until_your_priority(&mut self) {
        for _ in 0..400 {
            let game = self.game();
            if game.core.priority == Side::You || game.core.outcome.is_some() {
                return;
            }
            self.step(None);
        }
        fail(
            "the NPC never handed priority back",
            "400 ticks passed with the NPC holding priority; it acts after NPC_DELAY ticks",
        );
    }

    /// Pass until the stack is empty, as a patient player would.
    fn pass_until_empty(&mut self) {
        for _ in 0..2000 {
            if self.game().core.stack.is_empty() {
                return;
            }
            if self.game().core.priority == Side::You {
                self.tap(&[Key::Space]);
            } else {
                self.step(None);
            }
        }
        fail(
            "the stack never emptied",
            "2000 ticks of passing left items on it",
        );
    }

    fn view(&self) -> Rect {
        Camera {
            viewport: VIEWPORT,
            ..*self.sim.world().resource::<Camera>()
        }
        .visible_bounds()
    }

    fn frame(&mut self) -> FrameRecord {
        self.recorder.draw(&mut self.sim)
    }
}

/// The keys that make `action` happen, as a person would press them.
fn keys_for(core: &Core, action: Action) -> Vec<Key> {
    match action {
        Action::Pass => vec![Key::Space],
        Action::Play { hand_index, target } => {
            let mut keys = vec![SLOT_KEYS[hand_index], Key::Enter];
            if let Some(target) = target {
                let mut legal = core
                    .options(Side::You, hand_index)
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                legal.reverse();
                let at = legal.iter().position(|&id| id == target).unwrap_or(0);
                keys.extend(std::iter::repeat_n(Key::ArrowDown, at));
                keys.push(Key::Enter);
            }
            keys
        }
    }
}

/// Stage a stack: NPC active, plays `npc_cards`; you then pass so the NPC may
/// play the next, as a real exchange would. Returns with you holding priority.
fn npc_opens(core: &mut Core, npc_cards: &[Card]) {
    core.active = Side::Npc;
    core.priority = Side::Npc;
    core.passes = 0;
    core.hands[1] = npc_cards.to_vec();
    core.mana[1] = 6;
    for index in 0..npc_cards.len() {
        if core.try_play(Side::Npc, 0, None).is_err() {
            fail(
                "the staged NPC play was refused",
                "the staging hand and mana are the check's own",
            );
        }
        if index + 1 < npc_cards.len() {
            let _ = core.try_pass(Side::You);
        }
    }
}

fn log_tail(core: &Core, n: usize) -> Vec<String> {
    core.log[core.log.len() - n..].to_vec()
}

/// Decision row 1: whether to respond, and with which card — three items on
/// the stack at a priority window, then the resolution matches the preview.
fn decision_stack(session: &mut Session, checks: &mut Checks) -> FrameRecord {
    session.stage(|core| {
        *core = Core::new(FIRST_SEED);
        npc_opens(core, &[Card::Cleaver]);
        // A full-looking hand, so the captured picture is of a game in play.
        core.hands[0] = vec![
            Card::Mend,
            Card::Ember,
            Card::Flip,
            Card::Negate,
            Card::Redirect,
            Card::Bury,
            Card::Echo,
        ];
        core.mana[0] = 6;
    });
    // Mend, then (once the NPC has passed) Ember — through the keyboard.
    session.tap(&[Key::Digit1, Key::Enter]);
    session.until_your_priority();
    session.tap(&[Key::Digit1, Key::Enter]);
    session.until_your_priority();

    let frame = session.frame();
    let font = session.recorder.font_texture();
    let screen = ui::build(session.game(), session.view());
    // Shipped literals: top first, owner, effect, resolution order, life after.
    check_rows(
        checks,
        "decision row 1 (respond to the stack)",
        &screen,
        &[
            Expect::row("@3 YOU Ember 2>NPC", "#1 20-18", Mark::None),
            Expect::row("@2 YOU Mend +4 YOU", "#2 24-18", Mark::None),
            Expect::row("@1 NPC Cleaver 6>YOU", "#3 18-18", Mark::None),
        ],
    );
    checks.require(
        screen.priority_line == "YOUR PRIORITY: respond, or Space to resolve",
        "the priority window does not say you may respond",
        format!("{:?}", screen.priority_line),
    );
    for row in &screen.stack_rows {
        let drawn = glyphs_in(&frame, row.rect, font);
        let wanted = row.left.chars().filter(|c| !c.is_whitespace()).count();
        checks.require(
            drawn >= wanted,
            "a stack row's text is not drawn inside its row",
            format!(
                "{:?}: {drawn} glyph quads inside the row, {wanted} characters to show",
                row.left
            ),
        );
    }
    let predicted: Vec<String> = preview(&session.game().core)
        .iter()
        .map(|s| s.text.clone())
        .collect();
    session.pass_until_empty();
    let core = &session.game().core;
    let actual = log_tail(core, 3);
    checks.require(
        actual == predicted,
        "resolution did not match the preview shown at the priority window",
        format!("preview {predicted:?}, resolved {actual:?}"),
    );
    checks.require(
        actual
            == [
                "YOU Ember: 2 damage to NPC (now 18)",
                "YOU Mend: YOU gains 4 (now 24)",
                "NPC Cleaver: 6 damage to YOU (now 18)",
            ]
            && core.life == [18, 18],
        "the three-item stack resolved to the wrong outcome",
        format!("{actual:?}, life {:?}", core.life),
    );
    frame
}

/// Decision row 2: pass priority and let the stack resolve.
fn decision_pass(session: &mut Session, checks: &mut Checks) {
    session.stage(|core| {
        *core = Core::new(FIRST_SEED);
        npc_opens(core, &[Card::Siege]);
        core.hands[0] = vec![];
    });
    let before = ui::build(session.game(), session.view());
    check_rows(
        checks,
        "decision row 2 (pass priority)",
        &before,
        &[Expect::row("@1 NPC Siege 10>YOU", "#1 10-20", Mark::None)],
    );
    let predicted = preview(&session.game().core)[0].text.clone();
    session.tap(&[Key::Space]);
    let waiting = ui::build(session.game(), session.view());
    checks.require(
        waiting.priority_line == "NPC HAS PRIORITY: it may still respond"
            && session.game().core.stack.len() == 1
            && session.game().core.priority == Side::Npc,
        "after one pass the opponent could not still respond",
        format!(
            "{:?}, {} item(s) left",
            waiting.priority_line,
            session.game().core.stack.len()
        ),
    );
    session.pass_until_empty();
    let core = &session.game().core;
    checks.require(
        core.life == [10, 20] && core.log.last() == Some(&predicted),
        "passing did not resolve the stack exactly as previewed",
        format!(
            "life {:?}, last {:?}, preview {predicted:?}",
            core.life,
            core.log.last()
        ),
    );
}

/// Decision row 3: where to target a reorder, a counter.
fn decision_target(session: &mut Session, checks: &mut Checks) {
    let stage = |session: &mut Session, hand: Vec<Card>| {
        session.stage(|core| {
            *core = Core::new(FIRST_SEED);
            npc_opens(core, &[Card::Cleaver, Card::Ember, Card::Ember]);
            core.hands[0] = hand;
            core.mana[0] = 4;
        });
    };
    // --- reorder: Bury one of the Embers --------------------------------
    stage(session, vec![Card::Bury, Card::Negate]);
    session.tap(&[Key::Digit1, Key::Enter]);
    let marked = ui::build(session.game(), session.view());
    check_rows(
        checks,
        "decision row 3 (targets marked, cursor on the top item)",
        &marked,
        &[
            Expect::row("@3 NPC Ember 2>YOU", "#1 18-20", Mark::Cursor),
            Expect::row("@2 NPC Ember 2>YOU", "#2 16-20", Mark::Legal),
            Expect::row("@1 NPC Cleaver 6>YOU", "#3 10-20", Mark::None),
        ],
    );
    checks.require(
        marked.priority_line == "TARGET: Up/Down, Enter ok, Esc cancel (2 legal)",
        "choosing a target does not say what the keys do",
        format!("{:?}", marked.priority_line),
    );
    session.tap(&[Key::ArrowDown]);
    let moved = ui::build(session.game(), session.view());
    let marks: Vec<Mark> = moved.stack_rows.iter().map(|r| r.mark).collect();
    checks.require(
        marks == [Mark::Legal, Mark::Cursor, Mark::None],
        "the target cursor did not move down to the next legal item",
        format!("{marks:?}"),
    );
    session.tap(&[Key::Enter]);
    session.until_your_priority();
    let queued = ui::build(session.game(), session.view());
    // The order the preview shows once Bury is on the stack.
    check_rows(
        checks,
        "decision row 3 (order shown with Bury queued)",
        &queued,
        &[
            Expect::row("@4 YOU Bury ->@2", "#1 20-20", Mark::None),
            Expect::row("@3 NPC Ember 2>YOU", "#2 18-20", Mark::None),
            Expect::row("@2 NPC Ember 2>YOU", "#4 10-20", Mark::None),
            Expect::row("@1 NPC Cleaver 6>YOU", "#3 12-20", Mark::None),
        ],
    );
    let shown: Vec<u32> = preview(&session.game().core)
        .iter()
        .map(|s| s.item.id.0)
        .collect();
    checks.require(
        shown == [4, 3, 1, 2],
        "the previewed order is not the Bury order",
        format!("{shown:?}"),
    );
    session.tap(&[Key::Space]);
    // Bury resolves (the NPC has already passed); then the new order is on screen.
    let after = ui::build(session.game(), session.view());
    check_rows(
        checks,
        "decision row 3 (order after Bury resolves equals the order shown)",
        &after,
        &[
            Expect::row("@3 NPC Ember 2>YOU", "#1 18-20", Mark::None),
            Expect::row("@1 NPC Cleaver 6>YOU", "#2 12-20", Mark::None),
            Expect::row("@2 NPC Ember 2>YOU", "#3 10-20", Mark::None),
        ],
    );
    let predicted: Vec<String> = preview(&session.game().core)
        .iter()
        .map(|s| s.text.clone())
        .collect();
    session.pass_until_empty();
    let actual = log_tail(&session.game().core, 3);
    checks.require(
        actual == predicted && session.game().core.life == [10, 20],
        "the order that resolved is not the order the panel showed",
        format!(
            "shown {predicted:?}, resolved {actual:?}, life {:?}",
            session.game().core.life
        ),
    );
    // --- counter: Negate the Cleaver ------------------------------------
    stage(session, vec![Card::Negate]);
    session.tap(&[Key::Digit1, Key::Enter]);
    let marked = ui::build(session.game(), session.view());
    let marks: Vec<Mark> = marked.stack_rows.iter().map(|r| r.mark).collect();
    checks.require(
        marks == [Mark::Cursor, Mark::Legal, Mark::Legal],
        "Negate did not mark every stack item as a legal target",
        format!("{marks:?}"),
    );
    session.tap(&[Key::ArrowDown, Key::ArrowDown, Key::Enter]);
    session.until_your_priority();
    let queued = ui::build(session.game(), session.view());
    check_rows(
        checks,
        "decision row 3 (counter shown before it resolves)",
        &queued,
        &[
            Expect::row("@4 YOU Negate ->@1", "#1 20-20", Mark::None),
            Expect::row("@3 NPC Ember 2>YOU", "#2 18-20", Mark::None),
            Expect::row("@2 NPC Ember 2>YOU", "#3 16-20", Mark::None),
            Expect::row("@1 NPC Cleaver 6>YOU", "x by #1", Mark::None),
        ],
    );
    session.pass_until_empty();
    let core = &session.game().core;
    checks.require(
        core.life == [16, 20]
            && core
                .log
                .iter()
                .any(|l| l == "YOU Negate: countered an item"),
        "the countered Cleaver still resolved",
        format!("life {:?}", core.life),
    );
    // Escape backs out of a choice without playing.
    stage(session, vec![Card::Negate]);
    session.tap(&[Key::Digit1, Key::Enter, Key::Escape]);
    checks.require(
        session.game().ui.targeting.is_none() && session.game().core.stack.len() == 3,
        "Escape did not cancel the target choice",
        format!("{} items", session.game().core.stack.len()),
    );
}

/// A whole match through the keyboard against the NPC, to the result screen.
struct Played {
    log: Vec<String>,
    ticks: u64,
    result: Option<&'static str>,
    deepest: usize,
    glyphs_on_result: usize,
}

fn full_match(checks: &mut Checks) -> Played {
    let mut session = Session::new();
    let mut deepest = 0;
    for _ in 0..60_000 {
        let game = session.game();
        if game.core.outcome.is_some() {
            break;
        }
        deepest = deepest.max(game.core.stack.len());
        if game.core.priority == Side::You {
            let action = players::skilled(&game.core);
            let keys = keys_for(&game.core, action);
            session.tap(&keys);
        } else {
            session.step(None);
        }
    }
    let frame = session.frame();
    let font = session.recorder.font_texture();
    let result = session.game().result_line();
    let screen = ui::build(session.game(), session.view());
    let shows_result = screen
        .labels
        .iter()
        .any(|l| Some(l.text.as_str()) == result);
    checks.require(
        result.is_some() && shows_result,
        "a full scripted match did not reach a result screen",
        format!("result {result:?}, {} ticks", session.ticks),
    );
    let glyphs_on_result = frame.quads().iter().filter(|q| q.texture == font).count();
    Played {
        log: session.game().core.log.clone(),
        ticks: session.ticks,
        result,
        deepest,
        glyphs_on_result,
    }
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();

    rulechecks::run_all(&mut checks);
    let sweep = rulechecks::sweep(&mut checks);

    let mut session = Session::new();
    let capture_frame = decision_stack(&mut session, &mut checks);
    decision_pass(&mut session, &mut checks);
    decision_target(&mut session, &mut checks);
    let font = session.recorder.font_texture();
    let view = session.view();

    crate::layout::check(&mut checks);

    // --- nothing drawn outside the camera, with the margin printed ------
    let clearance = capture_frame
        .quads()
        .iter()
        .map(|quad| {
            let bounds = quad.bounds();
            let gap = (bounds.min - view.min).min(view.max - bounds.max);
            gap.x.min(gap.y)
        })
        .fold(f32::MAX, f32::min);
    checks.require(
        greater(clearance, -0.001),
        "something was drawn outside the camera",
        format!("closest quad to the edge is {clearance:.3} world units inside"),
    );

    // --- the full match, twice: it ends, and it is deterministic --------
    let first = full_match(&mut checks);
    let second = full_match(&mut checks);
    checks.require(
        first.log == second.log && first.ticks == second.ticks,
        "the same seed and the same keys played two different matches",
        format!(
            "{} vs {} ticks, {} vs {} log lines",
            first.ticks,
            second.ticks,
            first.log.len(),
            second.log.len()
        ),
    );

    // --- three players, and what they say about the game ---------------
    let blind = sweep.blind_wins;
    let power = sweep.power_wins;
    let skilled = sweep.skilled_wins;
    checks.require(
        blind == 0,
        "doing nothing won a match",
        format!("{blind} of {SWEEP_SEEDS}"),
    );
    checks.require(
        power * 4 < SWEEP_SEEDS as u32,
        "raw power alone wins too often for sequencing to be the skill",
        format!("power-only won {power} of {SWEEP_SEEDS}; limit is under a quarter"),
    );
    checks.require(
        skilled * 100 >= SWEEP_SEEDS as u32 * 55,
        "playing the stack does not beat the NPC often enough to be learnable",
        format!("the skilled player won {skilled} of {SWEEP_SEEDS}; expected at least 55%"),
    );
    checks.require(
        sweep.manipulation_plays >= 40 && sweep.windows_with_stack >= 100,
        "the skilled player rarely played the stack, so its win rate says nothing about it",
        format!(
            "{} manipulation plays over {} windows ({} with a non-empty stack)",
            sweep.manipulation_plays, sweep.windows, sweep.windows_with_stack
        ),
    );

    let captured = crate::capture::capture_a_frame(&mut checks, &capture_frame, font);
    let verdict = checks.verdict();

    println!(
        "verified stack-card-game over {} scripted matches, 3 decision scenarios",
        SWEEP_SEEDS
    );
    println!(
        "  full match at seed {FIRST_SEED}: {} in {} ticks, deepest stack {}",
        first.result.unwrap_or("none"),
        first.ticks,
        first.deepest
    );
    println!("  blind player won {blind} of {SWEEP_SEEDS}");
    println!("  power-only player won {power} of {SWEEP_SEEDS}");
    println!("  skilled player won {skilled} of {SWEEP_SEEDS}");
    println!(
        "  skilled player: {} stack-manipulation plays over {} priority windows, {} with a non-empty stack",
        sweep.manipulation_plays, sweep.windows, sweep.windows_with_stack
    );
    println!(
        "  result screen: {} glyph quads drawn",
        first.glyphs_on_result
    );
    println!("  closest quad to the edge: {clearance:.2} world units");
    println!("  capture: {captured}");
    print!("{}", capture_frame.transcript());
    verdict
}
