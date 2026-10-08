//! `--verify`: the duel played headless by three players, the decision-surface
//! checks, the floors on every screen, and one captured picture.
//!
//! What `tools/verify stack_card_game_r3v1` runs. Same systems, same config as
//! the window; input comes from the players in `players.rs` and the staged
//! stacks in `scenarios.rs`.
//!
//! Key functions: `run`, `sweep`, `rules_contracts`, `staged_results`.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::FrameRecorder;

use crate::checks::{Checks, clearance, judge, on_screen, panel_has};
use crate::players::{MATCH_TICKS, Player, play_match};
use crate::rules::{Card, Outcome, Side, pass};
use crate::scenarios::{Stage, staged};
use crate::screen::screen;
use crate::{WINDOW, camera, config, palette, register};

/// Seeds every player plays a whole match on.
const SEEDS: u64 = 12;
/// Of those, how many the reader must win, and the most raw power may: the
/// design's claim that sequencing beats raw card power, as numbers.
const READER_WINS_AT_LEAST: usize = 9;
const RAW_WINS_AT_MOST: usize = 3;

pub(crate) fn run() -> ExitCode {
    let mut checks = Checks::default();
    let view = camera().visible_bounds();

    schedule(&mut checks);

    // The shipped match: the reader on the game's own seed, recorded.
    let main = play_match(Player::Reader, config().seed, true);
    let again = play_match(Player::Reader, config().seed, false);
    checks.require(
        main.log_digest == again.log_digest && main.resolved == again.resolved,
        "the same seed played a different match",
        format!(
            "{} then {} resolutions, digests {:x} and {:x}",
            main.resolved, again.resolved, main.log_digest, again.log_digest
        ),
    );
    checks.require(
        main.outcome == Some(Outcome::Won(Side::You)),
        "the reader did not win the shipped match",
        format!(
            "outcome {:?} after {} rounds, {} ticks, life {} - {}",
            main.outcome, main.rounds, main.ticks, main.you_life, main.rival_life
        ),
    );
    let mut transcript = String::new();
    let mut margin = f32::MAX;
    match &main.busy_frame {
        Some((frame, table)) => {
            judge(
                &mut checks,
                "mid-match",
                &screen(table),
                frame,
                font(),
                view,
            );
            on_screen(&mut checks, "mid-match", frame, view);
            margin = margin.min(clearance(frame, view));
            let cleared = frame.plan.clear_color;
            let brightest = cleared.r.max(cleared.g).max(cleared.b);
            checks.require(
                cleared == palette::TABLE && brightest < 0.2,
                "the table is not dark enough for light text",
                format!("clear colour {cleared:?}, brightest channel {brightest:.3}"),
            );
            transcript = frame.transcript();
        }
        None => checks.require(
            false,
            "the shipped match never put three items on the stack",
            format!("{} resolutions in {} rounds", main.resolved, main.rounds),
        ),
    }
    match &main.result_frame {
        Some((frame, table)) => {
            let panel = screen(table);
            judge(&mut checks, "result", &panel, frame, font(), view);
            on_screen(&mut checks, "result", frame, view);
            checks.require(
                panel_has(&panel, "YOU WIN") && panel_has(&panel, "press ENTER for a rematch"),
                "the reader's result screen does not say it won",
                format!("outcome {:?}", table.duel.outcome),
            );
        }
        None => checks.require(
            false,
            "the shipped match never reached its result screen",
            format!("{} ticks of {MATCH_TICKS}", main.ticks),
        ),
    }
    checks.note(format!(
        "shipped match (seed {}): {:?} in {} rounds, {} ticks, life {} - {}",
        config().seed,
        main.outcome,
        main.rounds,
        main.ticks,
        main.you_life,
        main.rival_life
    ));

    sweep(&mut checks);
    let aiming = crate::scenarios::run_all(&mut checks);
    staged_results(&mut checks);
    rules_contracts(&mut checks);

    let picture = match aiming {
        Some(frame) => {
            margin = margin.min(clearance(&frame, view));
            crate::capture::capture_a_frame(&mut checks, &frame, font())
        }
        None => "skipped, no aiming frame was recorded".to_owned(),
    };

    checks.note(format!(
        "closest quad to the edge (play screens; the result dimmer is flush by design): \
         {margin:.2} world units"
    ));
    let verdict = checks.verdict();
    if checks.failed() == 0 {
        println!("verified stack card game: a full match, three players, four decision checks");
        for line in &checks.summary {
            println!("  {line}");
        }
        println!("  capture: {picture}");
        println!();
        println!("{transcript}");
    }
    verdict
}

/// The font's backend id, as every recorder of this game puts it.
fn font() -> jidousha::testing::BackendTextureId {
    FrameRecorder::new(WINDOW).font_texture()
}

/// Your input before the rival's (main.rs, `register`).
fn schedule(checks: &mut Checks) {
    let order = headless(config(), register).schedule_debug();
    let yours = order.find("take_your_input");
    let rivals = order.find("rival_acts");
    checks.require(
        yours.is_some() && rivals.is_some() && yours < rivals,
        "the rival acts before your input is read",
        format!("take_your_input at {yours:?}, rival_acts at {rivals:?}"),
    );
}

/// Every player, every seed, a whole match: the three lines and their numbers.
fn sweep(checks: &mut Checks) {
    for player in [Player::Reader, Player::RawPower, Player::Idle] {
        let reports: Vec<_> = (1..=SEEDS)
            .map(|seed| play_match(player, seed, false))
            .collect();
        let wins = reports
            .iter()
            .filter(|r| r.outcome == Some(Outcome::Won(Side::You)))
            .count();
        let unfinished = reports.iter().filter(|r| r.outcome.is_none()).count();
        let windows: u32 = reports.iter().map(|r| r.windows).sum();
        let answered: u32 = reports.iter().map(|r| r.answered).sum();
        let planned = reports.iter().map(|r| r.planned).sum::<f32>() / SEEDS as f32;
        let off = reports.iter().map(|r| r.landed_off).sum::<f32>() / SEEDS as f32;
        let rounds = reports.iter().map(|r| r.rounds).sum::<u32>() as f32 / SEEDS as f32;
        checks.require(
            unfinished == 0,
            "a match never ended",
            format!(
                "{}: {unfinished} of {SEEDS} matches ran {MATCH_TICKS} ticks",
                player.name()
            ),
        );
        let (ok, want) = match player {
            Player::Reader => (
                wins >= READER_WINS_AT_LEAST,
                format!("at least {READER_WINS_AT_LEAST}"),
            ),
            Player::RawPower => (
                wins <= RAW_WINS_AT_MOST,
                format!("at most {RAW_WINS_AT_MOST}"),
            ),
            Player::Idle => (wins == 0, "none".to_owned()),
        };
        checks.require(
            ok,
            "a player's results break the design's claim",
            format!("{} won {wins} of {SEEDS}, want {want}", player.name()),
        );
        checks.note(format!(
            "{}: won {wins} of {SEEDS}, {rounds:.1} rounds a match; answered the stack in \
             {answered} of {windows} windows; planned margin {planned:+.1}, landed {off:.1} off plan",
            player.name()
        ));
    }
}

/// The result screens a winning reader never sees.
fn staged_results(checks: &mut Checks) {
    let view = camera().visible_bounds();
    let mut banners = Vec::new();
    for (name, outcome, banner) in [
        ("lost", Outcome::Won(Side::Rival), "THE RIVAL WINS"),
        ("drawn", Outcome::Draw, "A DRAW"),
    ] {
        let mut stage = Stage::new(staged(&[], &[Card::Strike], 3, false));
        let table = stage.table_mut();
        table.duel.outcome = Some(outcome);
        table.duel.you.life = 0;
        let (frame, panel) = stage.frame();
        judge(checks, name, &panel, &frame, stage.font, view);
        // No margin fold here: the dimmer is the whole view, flush by design.
        on_screen(checks, name, &frame, view);
        checks.require(
            panel_has(&panel, banner),
            "a result screen says the wrong thing",
            format!("{name}: no row reads {banner:?}"),
        );
        banners.push(
            panel
                .runs
                .iter()
                .map(|run| run.text.clone())
                .collect::<Vec<_>>(),
        );
    }
    checks.require(
        banners.len() == 2 && banners[0] != banners[1],
        "losing and drawing look the same",
        "the two staged result screens have identical rows".to_owned(),
    );
}

/// What a played match rarely reaches: the round's end with empty decks.
fn rules_contracts(checks: &mut Checks) {
    let mut duel = staged(&[], &[Card::Strike; 7], 1, true);
    duel.you.shield = 2;
    duel.you.deck = vec![Card::Ward];
    duel.rival.deck.clear();
    let _ = pass(&mut duel, Side::You);
    let got = (
        duel.round,
        duel.leader,
        duel.you.life,
        duel.rival.life,
        duel.you.hand.len(),
        duel.you.deck.len(),
        duel.you.shield,
        duel.you.focus,
    );
    let want = (2, Side::You, 11, 10, 7, 0, 0, 3);
    checks.require(
        got == want,
        "the round did not end by the rules",
        format!(
            "(round, leader, your life, rival life, hand, deck, shield, focus) = {got:?}, want \
             {want:?}: a full hand burns its draw, an empty deck costs 1 life a draw"
        ),
    );
}
