//! `--verify`: the gates of the run folder's DESIGN.md, run headless.
//!
//! G1 determinism, G2 the arithmetic as shipped literals, G5 the three players,
//! G7 the schedule and G11 exact zero live here; the screen gates (G3, G4, G6,
//! G8, G9) are in `verify_screens.rs`, and G10 is `capture.rs`. Failures are
//! collected, never exited on, and every message prints what it judged.

use std::process::ExitCode;

use crate::beings::BeingId;
use crate::checks::Checks;
use crate::flow::{Choice, Game, Screen, call_options, step};
use crate::players::{Player, Run, play, sim_for};
use crate::rules::{AnswerKind, Outcome, answer_outcome, rotation, tonight};
use crate::screen::Art;
use crate::{WINDOW, camera, verify_screens};
use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder};
use jidousha::ui::Panel;

/// The seeds every gate runs on: the shipped one, and one more.
pub(crate) const SEEDS: [u64; 2] = [1928, 7];

/// Every frame a run recorded, with the panel it was drawn from.
pub(crate) type Frames = Vec<(Panel<Art>, FrameRecord)>;

/// The transcript since its last `==` marker: the screen on show.
pub(crate) fn current_block(transcript: &[String]) -> Vec<String> {
    let start = transcript
        .iter()
        .rposition(|line| line.starts_with("== "))
        .map_or(0, |at| at + 1);
    transcript[start..].to_vec()
}

/// A block's rows joined back into the prose they were wrapped from.
pub(crate) fn joined(block: &[String]) -> String {
    block.join(" ")
}

/// The view every recorded frame is drawn with.
pub(crate) fn view() -> Rect {
    Camera {
        viewport: WINDOW,
        ..camera()
    }
    .visible_bounds()
}

pub(crate) fn run() -> ExitCode {
    let mut checks = Checks::default();
    let mut recorder = FrameRecorder::new(WINDOW);
    let mut frames: Frames = Vec::new();
    let mut summary: Vec<String> = Vec::new();

    determinism(&mut checks);
    arithmetic(&mut checks);
    let scholar = players(&mut checks, &mut recorder, &mut frames, &mut summary);
    verify_screens::call_screen(&mut checks, &mut recorder, &mut frames);
    verify_screens::morning_screen(&mut checks);
    schedule(&mut checks);
    exact_zero(&mut checks);
    let clearance = verify_screens::floors(&mut checks, &mut recorder, &frames);
    verify_screens::sanity_bar(&mut checks, &mut recorder);
    verify_screens::clear_colour(&mut checks, &frames);

    let capture = match scholar.live.as_ref() {
        Some(frame) => crate::capture::capture_a_frame(&mut checks, frame, recorder.font_texture()),
        None => {
            checks.require(
                false,
                "the scholar's run drew no frame with a call on the line",
                format!("{} frames recorded in all", frames.len()),
            );
            "skipped, no live call frame to draw".to_owned()
        }
    };

    let (held, failed) = checks.counts();
    println!("verified call_of_cthulhu_r3v2: {held} checks held, {failed} failed");
    for line in &summary {
        println!("  {line}");
    }
    println!(
        "  {} frames judged; nearest quad to the view's edge: {clearance}",
        frames.len()
    );
    println!("  capture: {capture}");
    println!();
    if let Some(frame) = scholar.live.as_ref() {
        println!("last live frame (scholar, seed 1928):");
        println!("{}", frame.transcript());
    }
    println!("game transcript (scholar, seed 1928):");
    for line in &scholar.game.transcript {
        println!("{line}");
    }
    checks.verdict()
}

/// G1: the same seed and inputs replay the same run; the rotation puts three
/// distinct beings on nights 1-3 and two distinct beings on nights 4 and 5.
fn determinism(checks: &mut Checks) {
    let first = play(1928, Player::Scholar, None);
    let second = play(1928, Player::Scholar, None);
    checks.require(
        first.game.transcript == second.game.transcript,
        "two scholar runs on one seed told different stories",
        format!(
            "{} and {} transcript lines; first difference at line {:?}",
            first.game.transcript.len(),
            second.game.transcript.len(),
            first
                .game
                .transcript
                .iter()
                .zip(second.game.transcript.iter())
                .position(|(a, b)| a != b)
        ),
    );
    for seed in SEEDS {
        let order = rotation(&mut Rng::from_seed(seed));
        let sim = sim_for(seed);
        let shipped = sim.world().resource::<Game>().rotation;
        let a = tonight(3, shipped, [1, 0, 2], true, None);
        let b = tonight(3, shipped, [1, 0, 2], true, None);
        checks.require(
            a == b,
            "tonight() answered twice differently for the same day",
            format!("{a:?} then {b:?}"),
        );
        let first_callers: Vec<BeingId> = (1..=3)
            .map(|day| tonight(day, shipped, [0; 3], false, None).calls[0].being)
            .collect();
        let distinct = first_callers[0] != first_callers[1]
            && first_callers[1] != first_callers[2]
            && first_callers[0] != first_callers[2];
        checks.require(
            distinct,
            "nights 1-3 do not put each being on the line once",
            format!("seed {seed}: first callers {first_callers:?} (rotation {order:?})"),
        );
        for day in [4, 5] {
            let calls = tonight(day, shipped, [0; 3], false, None).calls;
            checks.require(
                calls.len() == 2 && calls[0].being != calls[1].being,
                "a two-call night does not have two different callers",
                format!("seed {seed} night {day}: {calls:?}"),
            );
        }
    }
}

/// G2: `answer_outcome` against shipped literals, never arithmetic over the
/// constants under test.
fn arithmetic(checks: &mut Checks) {
    let o = |sanity_cost, temper_delta, progress, hangs_up| Outcome {
        sanity_cost,
        temper_delta,
        progress,
        hangs_up,
    };
    let cases = [
        (
            BeingId::Cthulhu,
            0,
            false,
            AnswerKind::Lore,
            o(1, 0, 2, false),
        ),
        (
            BeingId::Cthulhu,
            0,
            false,
            AnswerKind::Wrong,
            o(6, 0, 1, false),
        ),
        (
            BeingId::Cthulhu,
            0,
            false,
            AnswerKind::Anger,
            o(12, 1, 0, false),
        ),
        (
            BeingId::Cthulhu,
            0,
            false,
            AnswerKind::HangUp,
            o(0, 2, 0, true),
        ),
        (
            BeingId::Nyarlathotep,
            0,
            false,
            AnswerKind::Wrong,
            o(4, 0, 1, false),
        ),
        (
            BeingId::Hastur,
            0,
            false,
            AnswerKind::Wrong,
            o(5, 0, 1, false),
        ),
        (
            BeingId::Cthulhu,
            0,
            true,
            AnswerKind::Wrong,
            o(5, 0, 1, false),
        ),
        (
            BeingId::Cthulhu,
            0,
            true,
            AnswerKind::Lore,
            o(1, 0, 2, false),
        ),
        (
            BeingId::Cthulhu,
            3,
            false,
            AnswerKind::Wrong,
            o(12, 0, 1, false),
        ),
        (
            BeingId::Cthulhu,
            3,
            false,
            AnswerKind::Anger,
            o(24, 1, 0, false),
        ),
    ];
    for (being, temper, meditated, kind, want) in cases {
        let got = answer_outcome(being, temper, meditated, kind);
        checks.require(
            got == want,
            "a reply does not cost what the design says",
            format!(
                "{being:?} temper {temper} meditated {meditated} {kind:?}: got {got:?}, want {want:?}"
            ),
        );
    }
}

/// G5: the three players on both seeds, each with its three numbers.
fn players(
    checks: &mut Checks,
    recorder: &mut FrameRecorder,
    frames: &mut Frames,
    summary: &mut Vec<String>,
) -> Run {
    let mut shipped_scholar = None;
    for seed in SEEDS {
        for player in Player::ALL {
            let run = play(seed, player, Some(&mut *recorder));
            frames.extend(run.frames.iter().cloned());
            let game = &run.game;
            summary.push(format!(
                "{} seed {seed}: {} on day {}, sanity {}/90 | answered {} of {} questions asked | lore replies {} of {} | predicted cost {}, paid {}",
                player.name(),
                match game.screen {
                    Screen::End { won: true } => "won",
                    Screen::End { won: false } => "lost",
                    _ => "unfinished",
                },
                game.day,
                game.sanity,
                run.answered,
                run.asked,
                run.lore,
                run.answered,
                run.predicted,
                run.paid
            ));
            checks.require(
                run.predicted == run.paid,
                "a player paid something other than what answer_outcome predicted",
                format!(
                    "{} seed {seed}: predicted {}, paid {}",
                    player.name(),
                    run.predicted,
                    run.paid
                ),
            );
            checks.require(
                run.answered == run.asked,
                "a player was asked a question it could not answer",
                format!(
                    "{} seed {seed}: answered {} of {}",
                    player.name(),
                    run.answered,
                    run.asked
                ),
            );
            let state = format!(
                "{} seed {seed}: screen {:?}, day {}, sanity {}",
                player.name(),
                game.screen,
                game.day,
                game.sanity
            );
            match player {
                Player::Scholar => {
                    checks.require(
                        game.screen == Screen::End { won: true } && game.sanity >= 20,
                        "the scholar did not survive the run with sanity to spare",
                        state.clone(),
                    );
                    checks.require(
                        game.transcript.iter().any(|line| line.contains("DAY 5")),
                        "the scholar's run never reached day 5",
                        state,
                    );
                }
                Player::Guesser => checks.require(
                    game.screen == Screen::End { won: false } && (4..=5).contains(&game.day),
                    "the guesser did not lose during night 4 or 5",
                    state,
                ),
                Player::Blasphemer => checks.require(
                    game.screen == Screen::End { won: false } && game.day <= 2,
                    "the blasphemer did not lose by the end of night 2",
                    state,
                ),
            }
            tonight_row_names_the_caller(checks, &run, player, seed);
            if seed == 1928 && player == Player::Scholar {
                shipped_scholar = Some(run);
            }
        }
    }
    let Some(run) = shipped_scholar else {
        crate::checks::fail(
            "the scholar's seed-1928 run was never played",
            "players() loops over SEEDS and Player::ALL",
        );
    };
    run
}

/// G4's last clause, over every G5 run: the morning's `tonight:` row names the
/// being the night then shows calling first.
fn tonight_row_names_the_caller(checks: &mut Checks, run: &Run, player: Player, seed: u64) {
    let lines = &run.game.transcript;
    let mut previewed: Option<String> = None;
    for (at, line) in lines.iter().enumerate() {
        if line.starts_with("tonight: ") {
            previewed = line
                .trim_start_matches("tonight: ")
                .split(' ')
                .next()
                .map(str::to_owned);
        }
        if line.starts_with("call 1 of ")
            && lines[..at]
                .last()
                .is_some_and(|prev| prev.starts_with("== night"))
            && let Some(name) = previewed.take()
        {
            let shown = line
                .split(" - ")
                .nth(1)
                .and_then(|rest| rest.split(',').next());
            checks.require(
                shown == Some(name.as_str()),
                "the morning previewed one caller and another called",
                format!(
                    "{} seed {seed}: previewed {name}, then {:?} called (line {at})",
                    player.name(),
                    shown
                ),
            );
        }
    }
}

/// G7: the three systems, each found, each in its phase.
fn schedule(checks: &mut Checks) {
    let sim = sim_for(1928);
    let order = sim.schedule_debug();
    let at = |name: &str| order.find(name);
    let (startup, update, draw) = (at("Startup"), at("Update (1)"), at("Draw"));
    let (scene, play, screen) = (at("set_the_scene"), at("play"), at("draw_screen"));
    let found = [startup, update, draw, scene, play, screen]
        .iter()
        .all(Option::is_some);
    checks.require(
        found && startup < scene && scene < update && update < play && play < draw && draw < screen,
        "the schedule is not set_the_scene, then play alone, then draw_screen",
        order,
    );
}

/// G11: sanity reaching exactly zero loses.
fn exact_zero(checks: &mut Checks) {
    let mut game = Game::new([BeingId::Cthulhu, BeingId::Nyarlathotep, BeingId::Hastur]);
    // Study Cthulhu (the first option), which puts Cthulhu on the line with a
    // plain wrong reply costing 6.
    let _ = step(&mut game, Choice::Pick(0));
    game.sanity = 6;
    let wrong = call_options(&game)
        .iter()
        .position(|(kind, _)| *kind == AnswerKind::Wrong);
    if let Some(index) = wrong {
        let _ = step(&mut game, Choice::Pick(index));
    }
    checks.require(
        game.screen == Screen::End { won: false },
        "sanity at exactly zero did not end the run",
        format!(
            "screen {:?}, sanity {}, wrong option {wrong:?}",
            game.screen, game.sanity
        ),
    );
}
