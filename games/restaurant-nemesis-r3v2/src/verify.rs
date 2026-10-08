//! `--verify`: the schedule, three players over whole runs, the staged screens
//! and their floors, the four decision gates, and one captured picture
//! (DESIGN.md §Gates, G1-G10).
//!
//! What `tools/verify restaurant_nemesis_r3v2` runs: the same systems and
//! config as the window, keys from the players in `players.rs`.
//!
//! Key functions: `run`, `schedule`, `three_players`, `staged_screens`.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::FrameRecord;
use jidousha::ui::{Panel, TextRun, judge_panel};

use crate::checks::{Checks, FLOORS, OVERLAYS, look, panel_has};
use crate::players::{Policy, Session, greedy, order_taker, play, sleeper};
use crate::screen::Art;
use crate::sim::*;
use crate::{config, register};

/// The first seed the greedy chef wins on.
const SEED_WIN: u64 = 0;
/// The first seed the order-taker has two or more nemeses active on one day.
const SEED_TWO: u64 = 0;
/// The sleeper's seed.
const SEED_SLEEP: u64 = 0;

pub(crate) fn run() -> ExitCode {
    let mut checks = Checks::new();
    schedule(&mut checks);
    three_players(&mut checks);
    let card = staged_screens(&mut checks);
    crate::gates::spawn_seed_holds_now(&mut checks);
    crate::gates::spawn_row(&mut checks);
    crate::gates::card_runs(&mut checks);
    crate::gates::ledger_runs(&mut checks);
    crate::gates::at_the_cap(&mut checks);

    let (picture, transcript) = match card {
        Some(frame) => (
            crate::capture::capture_a_frame(&mut checks, &frame),
            frame.transcript(),
        ),
        None => ("skipped, no card frame".to_owned(), String::new()),
    };
    checks.note(format!(
        "closest quad to the edge: {:.2} world units",
        checks.clearance
    ));
    let verdict = checks.verdict();
    if checks.failed() == 0 {
        println!(
            "verified restaurant_nemesis_r3v2: {} checks, 3 players, {} frames",
            checks.count, checks.frames
        );
        for line in &checks.summary {
            println!("  {line}");
        }
        println!("  capture: {picture}");
        println!();
        println!("{transcript}");
    }
    verdict
}

/// G1: the one Update system and the one Draw system are where they belong.
fn schedule(checks: &mut Checks) {
    let order = headless(config(0), register).schedule_debug();
    let update = order.find("Update");
    let draw_phase = order.find("Draw");
    let advance = order.find("advance");
    let draw = order.rfind("draw");
    checks.require(
        update.is_some()
            && draw_phase.is_some()
            && advance.is_some()
            && draw.is_some()
            && update < advance
            && advance < draw_phase
            && draw_phase < draw,
        "the systems are not in the phases the game runs them in",
        format!(
            "Update at {update:?}, advance at {advance:?}, Draw at {draw_phase:?}, draw at {draw:?}"
        ),
    );
}

/// G4: three players over whole runs.
fn three_players(checks: &mut Checks) {
    let runs: [(&str, u64, Policy); 3] = [
        ("sleeper", SEED_SLEEP, &sleeper),
        ("greedy chef", SEED_WIN, &greedy),
        ("order-taker", SEED_TWO, &order_taker),
    ];
    for (name, seed, policy) in runs {
        let report = play(seed, policy);
        let game = &report.game;
        let s = report.stats;
        checks.require(
            report.finished,
            "a run stalled",
            format!(
                "{name}: {:?} on day {} after {} ticks",
                game.phase, game.day, report.ticks
            ),
        );
        match name {
            "sleeper" => checks.require(
                game.phase
                    == (Phase::Over {
                        won: false,
                        reason: "money",
                    })
                    && game.day == 1
                    && game.money < 0,
                "the sleeper did not close down on day 1",
                format!("{:?} on day {}, money {}", game.phase, game.day, game.money),
            ),
            "greedy chef" => checks.require(
                matches!(game.phase, Phase::Over { won: true, .. }) && game.day == DAYS,
                "the greedy chef did not survive the week",
                format!(
                    "{:?} on day {}, money {}, rep {}",
                    game.phase, game.day, game.money, game.rep
                ),
            ),
            _ => checks.require(
                game.peak.0 >= 2,
                "no run had two nemeses at once",
                format!(
                    "{name}: at most {} active, first on day {}",
                    game.peak.0, game.peak.1
                ),
            ),
        }
        checks.note(format!(
            "{name} (seed {seed}): {:?} on day {}, money ${}, rep {}; most nemeses at once {} \
             (day {}); served {} of {} orders; spawn-risk orders left unmet {}; capacity used \
             {} of {}",
            game.phase,
            game.day,
            game.money,
            game.rep,
            game.peak.0,
            game.peak.1,
            s.served,
            s.orders,
            s.risky_unmet,
            s.used,
            s.capacity
        ));
    }
}

/// G3: the five screens, judged; one staged breach bites by name. Hands back
/// the card frame for the capture.
fn staged_screens(checks: &mut Checks) -> Option<FrameRecord> {
    let mut session = Session::new(crate::gates::SEED_SPAWN);
    look(checks, "service day 1", &mut session);

    let mut card = None;
    let mut stats = crate::players::Stats::default();
    let policy = crate::players::scenario(Key::S);
    let at_card = |g: &Game| g.day == 3 && matches!(g.phase, Phase::Service { focus: Some(_) });
    if session.run_until(&policy, &at_card, &mut stats) && at_card(session.game()) {
        let (frame, panel) = look(checks, "service with the card", &mut session);
        checks.require(
            panel_has(&panel, "NEMESIS CARD"),
            "the card screen has no card",
            "staged on day 3".to_owned(),
        );
        card = Some(frame);
    } else {
        checks.require(
            false,
            "the card was never opened",
            format!("day {}", session.game().day),
        );
    }

    let mut session = Session::new(crate::gates::SEED_SPAWN);
    let at_ledger = |g: &Game| g.phase == Phase::Ledger;
    session.run_until(&policy, &at_ledger, &mut stats);
    let mut second = session.game().nemeses.first().copied();
    if let Some(n) = second.as_mut() {
        n.id = NemesisId(99);
        n.theme = Theme::Portion;
        n.followers = 65;
        session.game_mut().nemeses.push(*n);
    }
    let (_, panel) = look(checks, "ledger with two nemeses", &mut session);
    checks.require(
        panel_has(
            &panel,
            "[2] THE CRUMB COUNTESS - Trending Terror, 65 fol, 5 spends to ratio",
        ),
        "the ledger does not list the second nemesis",
        "staged with The Crumb Countess at 65 followers".to_owned(),
    );

    for (name, phase, line) in [
        (
            "end won",
            Phase::Over {
                won: true,
                reason: "survived",
            },
            "SURVIVED 7 DAYS - THE REVIEWS ARE MIXED",
        ),
        (
            "end lost",
            Phase::Over {
                won: false,
                reason: "money",
            },
            "CLOSED DOWN - out of money",
        ),
    ] {
        session.game_mut().phase = phase;
        let (_, panel) = look(checks, name, &mut session);
        checks.require(
            panel_has(&panel, line),
            "an end screen says the wrong thing",
            format!("{name}: no row reads {line:?}"),
        );
    }

    let small = TextStyle {
        size: 12.0,
        ..TextStyle::default()
    };
    let mut staged: Panel<Art> = Panel::default();
    staged.text(TextRun::new(Vec2::new(20.0, 300.0), "the soup", small));
    staged.text(TextRun::new(Vec2::new(20.0, 302.0), "is cold", small));
    let bites = judge_panel(&staged, &FLOORS, &[], OVERLAYS)
        .iter()
        .any(|breach| breach.what == "two rows of chrome text overlap");
    checks.require(
        bites,
        "the overlap floor does not bite on the screen it was written for",
        "two rows 2 units apart".to_owned(),
    );
    card
}
