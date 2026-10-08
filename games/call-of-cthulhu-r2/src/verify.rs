//! The `--verify` mode: three players over two dozen seeds, the two decision
//! rows, every screen of a recorded run judged, the staged screens, and a
//! captured picture. `tools/verify call_of_cthulhu_r2` runs it.

use std::process::ExitCode;

use crate::checks::Checks;
use crate::conductor::Conductor;
use crate::lore::{AnswerKind, Being};
use crate::play::{CallState, Phase};
use crate::players::{self, Player, Session, key_for};
use crate::rules;
use crate::screen_checks::{self, Margins};
use crate::screens::{self, Pick};

/// The seeds every player plays.
const SEEDS: [u64; 24] = [
    1890, 1926, 1928, 1931, 1936, 1937, 7, 13, 42, 99, 2024, 31337, 1, 2, 3, 5, 8, 21, 34, 55, 89,
    144, 233, 377,
];
/// A prepared player survives at least this many of the twenty-four.
const SCHOLAR_WINS_AT_LEAST: usize = 20;
/// A first try is neither hopeless nor safe: it wins at least this many and
/// loses at least this many of the twenty-four.
const FIRST_TRY_WINS_AT_LEAST: usize = 3;
const FIRST_TRY_LOSES_AT_LEAST: usize = 3;

pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let mut summary = Vec::new();

    // --- the players ------------------------------------------------------
    for player in [Player::Scholar, Player::FirstTimer, Player::Silent] {
        let sessions: Vec<Session> = SEEDS
            .iter()
            .map(|seed| players::play(player, *seed, false))
            .collect();
        summary.push(population(&mut checks, player, &sessions));
    }

    // --- one recorded run, every screen judged -----------------------------
    let recorded = players::play(Player::Scholar, crate::decisions::SEED, true);
    let mut margins = Margins {
        clearance: f32::MAX,
        screens: 0,
    };
    if let Some(font) = recorded.font {
        for shot in &recorded.shots {
            let panel = screens::screen(&shot.game);
            let name = format!("tick {} {}", shot.tick, phase_name(&shot.game.phase));
            screen_checks::judge(&mut checks, &mut margins, &name, &panel, &shot.frame, font);
        }
    }
    checks.require(
        recorded.shots.len() > 20,
        "the recorded run photographed too few screens to judge the game",
        format!("{} shots", recorded.shots.len()),
    );
    let order = &recorded.schedule;
    let input_at = order.find("take_input");
    let silence_at = order.find("count_the_silence");
    checks.require(
        input_at.is_some() && silence_at.is_some() && input_at < silence_at,
        "the answer is no longer taken before the silence is counted",
        format!("take_input at {input_at:?}, count_the_silence at {silence_at:?} in:\n{order}"),
    );

    // --- determinism --------------------------------------------------------
    let again = players::play(Player::Scholar, crate::decisions::SEED, false);
    let other = players::play(Player::Scholar, crate::decisions::SEED + 1, false);
    checks.require(
        again.game.log == recorded.game.log && other.game.log != recorded.game.log,
        "the same seed and inputs did not replay the same run, or two seeds made one run",
        format!(
            "{} vs {} log lines on one seed (first difference at {:?}); seed+1 identical: {}",
            recorded.game.log.len(),
            again.game.log.len(),
            recorded
                .game
                .log
                .iter()
                .zip(&again.game.log)
                .position(|(a, b)| a != b),
            other.game.log == recorded.game.log
        ),
    );

    // --- the decision rows ---------------------------------------------------
    let call_photo = crate::decisions::answer_row(&mut checks);
    crate::decisions::morning_row(&mut checks);

    // --- silence, a tap, and the staged screens ------------------------------
    silence(&mut checks);
    tap(&mut checks);
    screen_checks::content_is_printable(&mut checks);
    screen_checks::staged(&mut checks, &mut margins);
    screen_checks::floor_bites(&mut checks);

    let captured = match (&call_photo, recorded.font) {
        (Some(photo), Some(font)) => {
            crate::capture::capture(&mut checks, &photo.frame, font, "call_of_cthulhu_r2")
        }
        _ => "skipped, the decision-row photo was not taken".to_owned(),
    };

    let verdict = checks.verdict();
    println!(
        "verified call_of_cthulhu_r2: {} checks passed, {} failed",
        checks.passed(),
        checks.failed()
    );
    for line in &summary {
        println!("  {line}");
    }
    println!(
        "  recorded scholar seed {}: {} screens judged, ending {:?} with sanity {}",
        recorded.seed, margins.screens, recorded.game.phase, recorded.game.run.sanity
    );
    println!(
        "  closest quad to the edge: {:.2} world units",
        margins.clearance
    );
    println!("  capture: {captured}");
    println!();
    println!("transcript of the recorded run:");
    for line in &recorded.game.log {
        println!("{line}");
    }
    if let Some(photo) = call_photo {
        println!();
        println!("the decision-row call screen:");
        for row in photo.strings() {
            println!("{row}");
        }
        print!("{}", photo.frame.transcript());
    }
    verdict
}

fn phase_name(phase: &Phase) -> &'static str {
    match phase {
        Phase::Morning => "morning",
        Phase::Call(_) => "call",
        Phase::Hangup(_) => "hang-up",
        Phase::Ended(_) => "ending",
    }
}

/// One player over every seed: its verdict line, and the checks it owes.
fn population(checks: &mut Checks, player: Player, sessions: &[Session]) -> String {
    let wins: Vec<i32> = sessions
        .iter()
        .filter_map(|session| match session.game.phase {
            Phase::Ended(crate::play::Ending::Won { sanity }) => Some(sanity),
            _ => None,
        })
        .collect();
    let lost_days: Vec<u32> = sessions
        .iter()
        .filter_map(|session| match session.game.phase {
            Phase::Ended(crate::play::Ending::Lost { day, .. }) => Some(day),
            _ => None,
        })
        .collect();
    let unfinished = sessions.len() - wins.len() - lost_days.len();
    let answers: [u32; 3] = sessions.iter().fold([0; 3], |sum, session| {
        [
            sum[0] + session.answers[0],
            sum[1] + session.answers[1],
            sum[2] + session.answers[2],
        ]
    });
    let hinted: u32 = sessions.iter().map(|session| session.hinted).sum();
    let aim: i32 = sessions.iter().map(|session| session.aim_error.abs()).sum();
    let total: u32 = answers.iter().sum();
    checks.require(
        unfinished == 0,
        "a session never ended",
        format!(
            "{player:?}: {unfinished} of {} stuck; the longest ran {} ticks",
            sessions.len(),
            sessions
                .iter()
                .map(|session| session.ticks)
                .max()
                .unwrap_or(0)
        ),
    );
    checks.require(
        aim == 0,
        "what the hint said an answer costs is not what it cost",
        format!("{player:?}: summed |predicted - applied| sanity over {total} answers is {aim}"),
    );
    match player {
        Player::Scholar => checks.require(
            wins.len() >= SCHOLAR_WINS_AT_LEAST && answers[2] == 0,
            "a prepared player does not reliably survive five nights",
            format!(
                "won {} of {}, lost on days {lost_days:?}; answers lore/wrong/insult {answers:?}",
                wins.len(),
                sessions.len()
            ),
        ),
        Player::FirstTimer => checks.require(
            wins.len() >= FIRST_TRY_WINS_AT_LEAST && lost_days.len() >= FIRST_TRY_LOSES_AT_LEAST,
            "a first try is either hopeless or safe, and should be neither",
            format!(
                "won {} of {}, lost on days {lost_days:?}",
                wins.len(),
                sessions.len()
            ),
        ),
        Player::Silent => checks.require(
            wins.is_empty() && lost_days.iter().all(|day| *day == 1) && total == 0,
            "silence on the line does not cost the run",
            format!(
                "won {} of {}, lost on days {lost_days:?}, {total} answers given",
                wins.len(),
                sessions.len()
            ),
        ),
    }
    let mean = |values: &[i32]| {
        if values.is_empty() {
            0.0
        } else {
            values.iter().sum::<i32>() as f32 / values.len() as f32
        }
    };
    format!(
        "{:<10} won {:>2} of {} (mean sanity at the end {:.0}), lost on days {lost_days:?}; answered {total}: \
         lore {}, wrong {}, insult {}; {hinted} with the hint showing; aim error {aim}",
        format!("{player:?}"),
        wins.len(),
        sessions.len(),
        mean(&wins),
        answers[0],
        answers[1],
        answers[2],
    )
}

/// Silence: one tick short of the threshold costs nothing, the threshold
/// costs one drain — the shipped literal for Yog-Sothoth at temper 0.
fn silence(checks: &mut Checks) {
    let mut game = crate::play::Game::new(crate::decisions::SEED);
    game.night.calls = vec![rules::PlannedCall {
        being: Being::YogSothoth,
        questions: vec![0, 1, 2, 3],
    }];
    game.phase = Phase::Call(CallState {
        slot: 0,
        being: Being::YogSothoth,
        interest: 5,
        asked: 0,
        quiet: 0,
        spent: 0,
        exchanges: 0,
        last: None,
    });
    for _ in 1..480 {
        game.listen();
    }
    let early = game.run.sanity;
    game.listen();
    let late = game.run.sanity;
    checks.require(
        early == 100 && late == 96,
        "silence on the line does not cost one drain every eight seconds",
        format!("after 479 quiet ticks sanity {early} (want 100); after 480, {late} (want 96: Yog-Sothoth's drain 4)"),
    );
}

/// A tap on an answer row answers that row: the sanity spent is that row's
/// outcome.
fn tap(checks: &mut Checks) {
    let mut run = Conductor::new(crate::decisions::SEED);
    run.press(key_for(Pick::Morning(0)));
    let game = run.game().clone();
    let Some(asking) = game.asking() else {
        checks.require(
            false,
            "a tap check found no call after the morning",
            format!("{:?}", game.phase),
        );
        return;
    };
    let row = asking
        .order
        .iter()
        .position(|kind| *kind == AnswerKind::Wrong)
        .unwrap_or(0);
    let Some((_, outcome)) = game.preview(row) else {
        return;
    };
    run.tap(screens::answer_row(row).center());
    let after = run.game();
    let interest = match &after.phase {
        Phase::Call(call) => call.interest,
        _ => -1,
    };
    checks.require(
        game.run.sanity - after.run.sanity == outcome.sanity_cost && interest == 6,
        "a tap on an answer row did not answer that row",
        format!(
            "tapped row {} (the wrong answer): sanity {} -> {} (want -{}), interest 5 -> {interest} (want 6)",
            row + 1,
            game.run.sanity,
            after.run.sanity,
            outcome.sanity_cost
        ),
    );
}
