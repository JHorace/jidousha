//! `--verify`: the game played headless, judged, and photographed.
//!
//! Every session here runs the same systems the window does (`register`),
//! with keys pressed by one of the three players (`players`). What it holds
//! the game to: the two decision rows of the handoff (`rows`), a win and a
//! sanity loss reached by play, the three players' lines over forty seeds,
//! every screen against the kit's floors and found on its frame, the screens
//! play does not reach staged and judged, and one captured picture.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord, FrameRecorder, InputScript};
use jidousha::ui::*;

use crate::checks::Checks;
use crate::lore::{self, Being, questions_of};
use crate::play::{Call, Command, Ending, Fate, Run, Screen};
use crate::players::{Player, decide, play};
use crate::rules::{self, CallPlan, Night};
use crate::screen::{DESIGN, SMALL, UiMap, WINDOW, layout};
use crate::{Game, config, register};

mod rows;

/// The seed the played sessions use.
pub(super) const SEED: u64 = 1926;
/// How many seeds the players' lines are measured over.
const SWEEP: u64 = 40;
/// The most ticks a played session may take.
const TICK_CAP: u64 = 600;

/// The floors every screen is held to, in design units.
pub(super) const FLOORS: Floors = Floors {
    min_text: SMALL,
    chrome: Rect {
        min: Vec2::ZERO,
        max: DESIGN,
    },
    world: Rect {
        min: Vec2::ZERO,
        max: DESIGN,
    },
};

/// The digit key that chooses row `row`.
pub(super) fn digit(row: usize) -> Key {
    [
        Key::Digit1,
        Key::Digit2,
        Key::Digit3,
        Key::Digit4,
        Key::Digit5,
        Key::Digit6,
    ][row.min(5)]
}

/// The key a command is pressed with.
pub(super) fn key_for(command: Command) -> Key {
    match command {
        Command::Choose(row) => digit(row),
        Command::Continue => Key::Space,
    }
}

/// A headless game on `seed`, ticked once so Startup has run.
pub(super) fn start(seed: u64) -> HeadlessSim {
    let mut sim = headless(config(seed), register);
    sim.world_mut().insert_resource(Game(Run::new(seed)));
    sim.world_mut()
        .insert_resource(Input::new(jidousha::testing::InputSnapshot::new()));
    sim.tick();
    sim
}

/// The run inside a sim.
pub(super) fn run_of(sim: &HeadlessSim) -> Run {
    sim.world().resource::<Game>().0.clone()
}

/// Press `key` on the next tick, and tick.
pub(super) fn press(sim: &mut HeadlessSim, key: Key) {
    let tick = sim.world().resource::<Time>().tick + 1;
    let script = InputScript::new().press(key, tick);
    sim.world_mut()
        .insert_resource(Input::new(script.snapshot_at(tick)));
    sim.tick();
}

/// The camera a recorded frame was drawn with.
pub(super) fn drawn_camera(sim: &HeadlessSim) -> Camera {
    Camera {
        viewport: WINDOW,
        ..*sim.world().resource::<Camera>()
    }
}

/// Judge one recorded frame of `run`: the floors, the panel found on the
/// frame, nothing off camera. Returns the closest any quad came to the edge.
pub(super) fn judge(
    checks: &mut Checks,
    what: &str,
    run: &Run,
    frame: &FrameRecord,
    font: BackendTextureId,
    camera: &Camera,
) -> f32 {
    let panel = layout(run).panel;
    for breach in judge_panel(&panel, &FLOORS, &[], &[]) {
        checks.require(
            false,
            "a screen breaks a readability floor",
            format!("{what}: {} - {}", breach.what, breach.detail),
        );
    }
    let view = camera.visible_bounds();
    for breach in judge_frame(&panel, frame, font, &UiMap::for_camera(camera), view) {
        checks.require(
            false,
            "a screen's rows are not on its frame",
            format!("{what}: {} - {}", breach.what, breach.detail),
        );
    }
    for breach in frame_text_floor(frame, font, SMALL) {
        checks.require(
            false,
            "a glyph is drawn below the floor",
            format!("{what}: {} - {}", breach.what, breach.detail),
        );
    }
    for text in panel.all_strings() {
        checks.require(
            text.chars().all(|c| (' '..='~').contains(&c)),
            "a screen draws a character the font has no glyph for",
            format!("{what}: {text:?}"),
        );
    }
    let mut clearance = f32::MAX;
    for quad in frame.quads() {
        let bounds = quad.bounds();
        checks.require(
            view.contains_rect(bounds),
            "something is drawn off screen",
            format!("{what}: {bounds:?} against {view:?}"),
        );
        let gap = (bounds.min - view.min).min(view.max - bounds.max);
        clearance = clearance.min(gap.x.min(gap.y));
    }
    clearance
}

/// What one played session through the systems came to.
pub(super) struct Session {
    pub(super) run: Run,
    pub(super) ticks: u64,
    pub(super) clearance: f32,
}

/// Play `player` on `seed` through the systems, key by key, judging every frame.
fn session(checks: &mut Checks, player: Player, seed: u64) -> Session {
    let mut sim = start(seed);
    let mut recorder = FrameRecorder::new(WINDOW);
    let mut clearance = f32::MAX;
    let mut ticks = 1;
    while ticks < TICK_CAP {
        let run = run_of(&sim);
        if matches!(run.screen, Screen::End(_)) {
            break;
        }
        press(&mut sim, key_for(decide(player, &run)));
        ticks += 1;
        let frame = recorder.draw(&mut sim);
        let camera = drawn_camera(&sim);
        let what = format!("{} tick {ticks}", player.name());
        clearance = clearance.min(judge(
            checks,
            &what,
            &run_of(&sim),
            &frame,
            recorder.font_texture(),
            &camera,
        ));
    }
    Session {
        run: run_of(&sim),
        ticks,
        clearance,
    }
}

/// The screens play does not reach on the sessions above, staged whole and judged.
fn staged_screens(checks: &mut Checks) -> usize {
    let mut staged = Vec::new();
    let mut base = Run::new(SEED);
    staged.push(("the morning with every option", base.clone()));
    base.state.known = [3; 3];
    base.state.composure = rules::MAX_COMPOSURE;
    for being in Being::ALL {
        for id in questions_of(being) {
            let mut run = base.clone();
            let plan = CallPlan {
                being,
                questions: vec![id],
                drain: rules::drain(being, 0),
            };
            run.night = Night {
                calls: vec![plan.clone(), plan],
            };
            let call = Call {
                slot: 0,
                being,
                line: being.spec().line,
                anger: being.spec().temper - 1,
                asked: 0,
                cost: 0,
            };
            run.screen = Screen::Calling(call);
            staged.push(("a call with all its lore known", run.clone()));
            run.screen = Screen::CallOver {
                call,
                ending: Ending::Wrath,
                last: rules::answer_outcome(id, 0, 2, 0),
            };
            staged.push(("a call ending in wrath", run));
        }
    }
    for fate in [
        Fate::Won { sanity: 64 },
        Fate::Won { sanity: 4 },
        Fate::Lost { day: 3 },
    ] {
        let mut run = base.clone();
        run.screen = Screen::End(fate);
        staged.push(("an end screen", run));
    }
    let mut sim = start(SEED);
    let mut recorder = FrameRecorder::new(WINDOW);
    for (what, run) in &staged {
        sim.world_mut().insert_resource(Game(run.clone()));
        let frame = recorder.draw(&mut sim);
        judge(
            checks,
            what,
            run,
            &frame,
            recorder.font_texture(),
            &drawn_camera(&sim),
        );
    }
    staged.len()
}

/// The three players' lines over the sweep: wins, and what the wins left.
fn players_line(checks: &mut Checks, summary: &mut Vec<String>) {
    for player in Player::ALL {
        let reports: Vec<_> = (0..SWEEP).filter_map(|seed| play(player, seed)).collect();
        checks.require(
            reports.len() as u64 == SWEEP,
            "a player chose a row the screen did not draw, or never finished",
            format!(
                "{}: {} of {SWEEP} runs finished",
                player.name(),
                reports.len()
            ),
        );
        let wins: Vec<i32> = reports
            .iter()
            .filter_map(|report| match report.fate {
                Fate::Won { sanity } => Some(sanity),
                Fate::Lost { .. } => None,
            })
            .collect();
        let informed: usize = reports.iter().map(|report| report.informed).sum();
        let answers: usize = reports
            .iter()
            .map(|report| report.answers)
            .sum::<usize>()
            .max(1);
        let wraths: usize = reports.iter().map(|report| report.wraths).sum();
        let left = wins.iter().sum::<i32>() / i32::try_from(wins.len().max(1)).unwrap_or(1);
        summary.push(format!(
            "{}: won {} of {SWEEP} (mean sanity left {left}), {}% of answers from lore, {wraths} wraths",
            player.name(),
            wins.len(),
            informed * 100 / answers
        ));
        // Shipped bands: the scholar nearly always wins, the mute never does,
        // and a first try lands between.
        let (low, high) = match player {
            Player::Scholar => (34, 40),
            Player::Novice => (4, 30),
            Player::Mute => (0, 0),
        };
        checks.require(
            (low..=high).contains(&wins.len()),
            "a player's line left its band",
            format!(
                "{} won {} of {SWEEP}, want {low}..={high}",
                player.name(),
                wins.len()
            ),
        );
    }
}

/// Run every check and print the verdict.
pub(crate) fn run() -> ExitCode {
    let mut checks = Checks::default();
    let mut summary = Vec::new();

    for text in lore::all_strings() {
        checks.require(
            text.chars().all(|c| (' '..='~').contains(&c)),
            "the lore has a character the font cannot draw",
            format!("{text:?}"),
        );
    }

    let scholar = session(&mut checks, Player::Scholar, SEED);
    let mute = session(&mut checks, Player::Mute, SEED);
    checks.require(
        matches!(scholar.run.screen, Screen::End(Fate::Won { .. })),
        "the scholar did not survive the fifth night by play",
        format!(
            "seed {SEED}: ended on {:?} after {} ticks, sanity {}",
            scholar.run.screen, scholar.ticks, scholar.run.sanity
        ),
    );
    checks.require(
        matches!(mute.run.screen, Screen::End(Fate::Lost { .. })),
        "the mute did not lose its sanity by play",
        format!(
            "seed {SEED}: ended on {:?} after {} ticks, sanity {}",
            mute.run.screen, mute.ticks, mute.run.sanity
        ),
    );
    // The keys and the state machine are one game: the session played
    // through the systems ends exactly where the pure run does.
    let mut pure = Run::new(SEED);
    while !matches!(pure.screen, Screen::End(_)) && pure.answers.len() < 200 {
        pure.step(decide(Player::Scholar, &pure));
    }
    checks.require(
        pure == scholar.run,
        "the played session and the pure run disagree",
        format!("pure {:?} vs played {:?}", pure.screen, scholar.run.screen),
    );
    summary.push(format!(
        "by play, seed {SEED}: scholar {:?}, mute {:?}",
        scholar.run.screen, mute.run.screen
    ));

    players_line(&mut checks, &mut summary);
    let staged = staged_screens(&mut checks);
    summary.push(format!("{staged} staged screens judged"));
    summary.push(format!(
        "closest quad to the edge: {:.2} design units",
        scholar.clearance.min(mute.clearance)
    ));

    rows::row_answer(&mut checks, &mut summary);
    rows::row_morning(&mut checks, &mut summary);
    rows::taps_and_order(&mut checks, &mut summary);
    rows::floors_bite(&mut checks);
    rows::zero_is_lost(&mut checks);

    let picture = rows::picture(&mut checks);
    summary.push(format!("capture: {picture}"));

    let word = if checks.passed() { "ok" } else { "FAILED" };
    println!("verified call_of_cthulhu_r3v1: {word}");
    for line in &summary {
        println!("  {line}");
    }
    checks.verdict()
}
