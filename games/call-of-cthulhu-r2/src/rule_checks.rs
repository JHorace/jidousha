//! The contracts a played run never isolates, asked directly — each with its
//! expectation as a shipped literal, so a mutated constant cannot carry the
//! expectation with it. Written after the first mutation round
//! (`mutants/r2.txt`), whose escapes each of these closes.

use crate::checks::Checks;
use crate::conductor::Conductor;
use crate::lore::{AnswerKind, Being};
use crate::play::{CallState, Game, Phase};
use crate::players::key_for;
use crate::rules::{self, MorningAction, PlannedCall, Run};
use crate::screens::{self, Pick};

pub fn all(checks: &mut Checks) {
    composure(checks);
    cults(checks);
    the_run(checks);
    hanging_up(checks);
    silence_resets(checks);
    unknown_is_unmarked(checks);
}

/// Composure trains once, to 1, and is then no longer offered.
fn composure(checks: &mut Checks) {
    let once = rules::apply_morning(&Run::new(1), MorningAction::Train);
    let twice = rules::apply_morning(&once, MorningAction::Train);
    let offered = rules::morning_options(&once).contains(&MorningAction::Train);
    checks.require(
        once.composure == 1 && twice.composure == 1 && !offered,
        "composure does not train once, to 1",
        format!(
            "after one training {}, after two {} (want 1 and 1); still offered after one: {offered}",
            once.composure, twice.composure
        ),
    );
}

/// A cult holds its being back at temper 0 and 1, not at 2.
fn cults(checks: &mut Checks) {
    let mut run = Run::new(1);
    let mut offered = Vec::new();
    for temper in 0..4 {
        run.temper[Being::Dagon.index()] = temper;
        offered.push(rules::morning_options(&run).contains(&MorningAction::Disrupt(Being::Dagon)));
    }
    checks.require(
        offered == [true, true, false, false],
        "a cult is offered for disruption at the wrong tempers",
        format!("Dagon's cult offered at temper 0..4: {offered:?} (want true, true, false, false)"),
    );
}

/// Five days; two callers a night, three on the fifth.
fn the_run(checks: &mut Checks) {
    let counts: Vec<usize> = (1..=5)
        .map(|day| {
            let mut run = Run::new(1890);
            run.day = day;
            rules::tonight(&run).calls.len()
        })
        .collect();
    let mut game = Game::new(1890);
    let mut mornings = 0;
    for _ in 0..10_000 {
        match &game.phase {
            Phase::Morning => {
                mornings += 1;
                game.choose_morning(0);
            }
            Phase::Call(_) => {
                let lore = game
                    .asking()
                    .and_then(|asking| {
                        asking
                            .order
                            .iter()
                            .position(|kind| *kind == AnswerKind::Lore)
                    })
                    .unwrap_or(0);
                game.answer(lore);
            }
            Phase::Hangup(_) => game.go_on(),
            Phase::Ended(_) => break,
        }
    }
    checks.require(
        counts == [2, 2, 2, 2, 3] && mornings == 5 && game.run.day == 5,
        "the run is not five days of two calls, then three on the last night",
        format!(
            "callers by night {counts:?} (want [2, 2, 2, 2, 3]); an all-lore run saw {mornings} mornings \
             and ended on day {} (want 5 and 5) in {:?}",
            game.run.day, game.phase
        ),
    );
}

/// A call on the line with `interest` left, about Dagon's fact 0.
fn staged_call(interest: i32) -> Game {
    let mut game = Game::new(1890);
    game.night.calls = vec![PlannedCall {
        being: Being::Dagon,
        questions: vec![0, 1, 2, 3],
    }];
    game.phase = Phase::Call(CallState {
        slot: 0,
        being: Being::Dagon,
        interest,
        asked: 0,
        quiet: 0,
        spent: 0,
        exchanges: 0,
        last: None,
    });
    game
}

fn position(game: &Game, kind: AnswerKind) -> usize {
    game.asking()
        .and_then(|asking| asking.order.iter().position(|each| *each == kind))
        .unwrap_or(0)
}

/// The line goes dead at interest 0 and not before; each answer moves on to
/// the next question.
fn hanging_up(checks: &mut Checks) {
    let mut ends = staged_call(3);
    ends.answer(position(&ends, AnswerKind::Lore));
    let mut stays = staged_call(4);
    let first = stays.asking().map(|asking| asking.fact);
    stays.answer(position(&stays, AnswerKind::Lore));
    let next = stays.asking().map(|asking| asking.fact);
    let left = match &stays.phase {
        Phase::Call(call) => call.interest,
        _ => -1,
    };
    checks.require(
        matches!(ends.phase, Phase::Hangup(_)) && left == 1 && first == Some(0) && next == Some(1),
        "a call does not end exactly at interest 0, or does not move on to its next question",
        format!(
            "lore at interest 3: {:?} (want the hang-up); lore at 4 leaves {left} (want 1) and the \
             question goes from fact {first:?} to {next:?} (want 0 then 1)",
            ends.phase
        ),
    );
}

/// An answer resets the silence: 479 quiet ticks, an answer, one more tick —
/// only the answer is paid.
fn silence_resets(checks: &mut Checks) {
    let mut game = staged_call(5);
    for _ in 0..479 {
        game.listen();
    }
    game.answer(position(&game, AnswerKind::Wrong));
    game.listen();
    checks.require(
        game.run.sanity == 98,
        "an answer does not reset the silence on the line",
        format!(
            "sanity {} after 479 quiet ticks, a wrong answer to Dagon and one tick (want 98)",
            game.run.sanity
        ),
    );
}

/// A question about a fact you do not know carries no hint: each answer reads
/// "?".
fn unknown_is_unmarked(checks: &mut Checks) {
    let mut run = Conductor::new(crate::decisions::SEED);
    run.press(key_for(Pick::Morning(0)));
    let photo = run.photo();
    let hints: Vec<String> = (0..3)
        .filter_map(|index| photo.game.preview(index))
        .map(|(_, outcome)| screens::hint_line(outcome))
        .filter(|hint| photo.shows(hint))
        .collect();
    let marks = photo.strings().iter().filter(|row| *row == "?").count();
    checks.require(
        hints.is_empty() && marks == 3,
        "a question about an unknown fact gives its answers away",
        format!("{marks} rows read \"?\" (want 3); hints on screen: {hints:?}"),
    );
}
