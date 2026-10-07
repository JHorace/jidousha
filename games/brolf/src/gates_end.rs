//! Sessions D and E: the extract row, and the three players.
//!
//! Key functions: `session_d`, `session_e`.
//! Depends on: `gates` (staging), `players`, `verify`, `checks`, `outcome`, `world`.
//! Never depended on outside the game's check.
//! INVARIANT: Session D runs with every rival frozen; only Session E runs the live population.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecorder, InputScript, SnapshotBuilder};

use crate::checks::{Checks, font_bounds_in, font_quads_in, within};
use crate::gates::{place, player, set_kit, staged};
use crate::items::{Item, Kit, Slot};
use crate::outcome::{Outcome, PadState, Placing};
use crate::players::{Controller, Kind, Report};
use crate::text::result_lines;
use crate::verify::{Gallery, MAX_TICKS, Run};
use crate::world::{Ball, Course, MatchState, Stats};
use crate::{COURSE_HALF, LINE_Y, TEXT_SIZE};

/// The kit Session D stages.
fn kit() -> Kit {
    Kit {
        ball: Some(Item::HeavyBall),
        body: Some(Item::Helmet),
        last_taken: Some(Slot::Body),
    }
}

/// Session D: the human holds two items, stands on the open pad, presses E; the result
/// screen keeps exactly what the status promised. Then the same kit, eliminated.
pub fn session_d(checks: &mut Checks, gallery: &mut Gallery) {
    let mut run = staged(&[]);
    let pad = run.world().resource::<Course>().pad;
    set_kit(run.sim.world_mut(), 0, kit());
    place(
        run.sim.world_mut(),
        0,
        pad + Vec2::new(1.1, 0.0),
        pad - Vec2::new(1.1, 0.0),
    );
    run.script = InputScript::new().press(Key::E, 2800);
    let mut recorder = FrameRecorder::new(crate::verify::HEADLESS_VIEWPORT);
    run.to(2799);
    let state = run.world().resource::<Course>().pad_state(2800);
    checks.require(
        matches!(state, PadState::Open { .. }),
        "the pad is not open on tick 2800",
        format!("pad state at 2800: {state:?}"),
    );
    let at = gallery.snap(
        &mut recorder,
        &mut run,
        "extract, the status that promises what is kept",
    );
    let shot = &gallery.shots[at];
    let line = shot.strings[1].clone();
    let quads = font_quads_in(&shot.frame, shot.font, LINE_Y[1], LINE_Y[1] + TEXT_SIZE);
    checks.require(
        line.contains("EXTRACT open")
            && line.contains("keeps heavy ball, helmet")
            && line.contains("HOLD heavy ball, helmet")
            && line.contains("RIVALS 5")
            && line.contains("PIN #"),
        "the status line does not promise what extracting keeps",
        format!("status line 2 on tick 2799: {line:?}"),
    );
    checks.require(
        quads == line.chars().count(),
        "the status line is not drawn in the top band's second line",
        format!(
            "{} characters, {quads} font quads in the line",
            line.chars().count()
        ),
    );
    run.step();
    let since = player(run.world(), 0).and_then(|p| p.extracting_since);
    checks.require(
        since == Some(2800),
        "pressing E on the pad did not start extracting on tick 2800",
        format!("extracting_since {since:?}"),
    );
    run.to(2919);
    checks.require(
        *run.world().resource::<MatchState>() == MatchState::Playing,
        "the match ended before the two seconds on the pad were up",
        format!(
            "state on tick 2919: {:?}",
            run.world().resource::<MatchState>()
        ),
    );
    run.step();
    let want = MatchState::Over {
        outcome: Outcome::Extracted,
        kept: kit(),
        placing: Placing::Left { still_in: 5 },
    };
    let got = *run.world().resource::<MatchState>();
    checks.require(
        got == want,
        "extracting did not end the match on tick 2920 with the promised kit",
        format!("state on tick 2920: {got:?}"),
    );
    let lines = result_lines(&run.world().view());
    checks.require(
        lines
            == [
                "EXTRACTED",
                "kept: heavy ball, helmet",
                "left with 5 still in",
            ],
        "the result screen does not say what the status promised",
        format!("result lines: {lines:?}"),
    );
    // D3: the result is centred and on top.
    let at = gallery.snap(&mut recorder, &mut run, "extract, the result screen");
    let shot = &gallery.shots[at];
    let course = Rect::from_center_size(Vec2::ZERO, COURSE_HALF * 2.0);
    let mut centred = true;
    let mut report = Vec::new();
    for k in 0..3 {
        let y0 = -1.35 + 0.9 * k as f32;
        let b = font_bounds_in(&shot.frame, shot.font, 0.9, y0, y0 + 0.9);
        let ok = b.is_some_and(|b| within(b.center().x, 0.0, 0.05) && course.contains_rect(b));
        centred &= ok;
        report.push(format!("{b:?}"));
    }
    checks.require(
        centred,
        "a line of the result is off centre or off the course",
        format!("line boxes: {}", report.join("; ")),
    );
    let top = shot.frame.covering(Vec2::ZERO);
    let tint = top.first().map(|q| q.tint);
    checks.require(
        tint == Some(Color::WHITE) || tint == Some(crate::COURT),
        "something is drawn over the result at the course's centre",
        format!("front quad at the centre: {tint:?}"),
    );
    gallery.say(format!(
        "extract: E on tick 2800 on the pad, extracted on tick 2920 keeping {:?}; result {lines:?}",
        kit()
    ));

    // D4: the same kit, eliminated by the zone, keeps nothing.
    let mut run = staged(&[]);
    set_kit(run.sim.world_mut(), 0, kit());
    while run.tick < MAX_TICKS && *run.world().resource::<MatchState>() == MatchState::Playing {
        run.step();
    }
    let lines = result_lines(&run.world().view());
    checks.require(
        lines[0] == "ELIMINATED" && lines[1] == "kept: nothing" && lines[2] == "placed 6 of 6",
        "an eliminated golfer's result does not keep nothing",
        format!("result lines {lines:?} on tick {}", run.tick),
    );
    gallery.say(format!(
        "extract: the same kit, eliminated on tick {}, keeps {:?}",
        run.tick, lines[1]
    ));
}

/// What one full session of a player came to.
struct Played {
    state: MatchState,
    ticks: u64,
    report: Report,
    stats: Stats,
    shots: u32,
    balls: Vec<(usize, Vec2)>,
}

/// Play `kind` against the live population until the match is over or the clock runs out.
fn play(kind: Kind) -> Played {
    let mut run = Run::new();
    let mut controller = Controller::new(kind);
    let mut keyboard = SnapshotBuilder::new();
    let camera = crate::camera();
    while run.tick < MAX_TICKS {
        controller.step(&run.world().view(), &mut keyboard, &camera);
        run.step_with(keyboard.first_tick_snapshot());
        if *run.world().resource::<MatchState>() != MatchState::Playing {
            break;
        }
    }
    let mut balls: Vec<(usize, Vec2)> = run
        .world()
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| (ball.owner, transform.pos))
        .collect();
    balls.sort_by_key(|(owner, _)| *owner);
    Played {
        state: *run.world().resource::<MatchState>(),
        ticks: run.tick,
        report: controller.report,
        stats: *run.world().resource::<Stats>(),
        shots: player(run.world(), 0).map_or(0, |p| p.shots),
        balls,
    }
}

/// A line for a player's verdict and its three numbers.
fn describe(name: &str, played: &Played) -> String {
    let r = played.report;
    format!(
        "{name}: {:?} on tick {}, {} shots ({} counted), {} in zone at arrival, aimed {:.2} from the hole, landed {:.2} from plan (mean shot {:.2}); rivals {:?}",
        played.state,
        played.ticks,
        played.shots,
        r.shots,
        r.in_zone_at_arrival,
        r.mean_aimed_from_hole(),
        r.mean_landed_from_plan(),
        r.mean_length(),
        played.stats,
    )
}

/// Session E: three players, one verdict line each.
pub fn session_e(checks: &mut Checks, gallery: &mut Gallery) {
    let idle = play(Kind::Idle);
    let golfer = play(Kind::Golfer);
    let full = play(Kind::Full);
    for (name, played) in [("idle", &idle), ("golfer", &golfer), ("full", &full)] {
        gallery.say(describe(name, played));
    }
    // E3: the idle player can lose.
    let idle_outcome = match idle.state {
        MatchState::Over { outcome, .. } => Some(outcome),
        MatchState::Playing => None,
    };
    checks.require(
        idle_outcome == Some(Outcome::Eliminated),
        "a player who does nothing was not eliminated by the zone",
        describe("idle", &idle),
    );
    // E1: a first-try player survives the zone and takes a handful of shots.
    let golfer_outcome = match golfer.state {
        MatchState::Over { outcome, .. } => Some(outcome),
        MatchState::Playing => None,
    };
    checks.require(
        golfer_outcome.is_some_and(|o| o != Outcome::Eliminated) && golfer.shots >= 3,
        "a golfer who walks to their ball and hits it where the zone will be did not survive, or barely played",
        describe("golfer", &golfer),
    );
    // E2: the full player finishes, and its three numbers are healthy.
    let r = full.report;
    let finished = matches!(full.state, MatchState::Over { .. });
    checks.require(
        finished
            && r.shots >= 1
            && r.in_zone_at_arrival as f32 >= r.shots as f32 * 0.8
            && r.mean_landed_from_plan() <= 0.12 * r.mean_length() + 0.01,
        "the full player did not finish, or its numbers say the controller is the fault",
        describe("full", &full),
    );
    // E4: determinism.
    let again = play(Kind::Full);
    checks.require(
        again.state == full.state
            && again.shots == full.shots
            && again.balls == full.balls
            && again.stats == full.stats,
        "two runs of the same player on the same seed differ",
        format!(
            "{} versus {}",
            describe("first", &full),
            describe("second", &again)
        ),
    );
    // E5: the rivals make the zone, contact and extraction matter.
    let s = golfer.stats;
    checks.require(
        s.npc_dazed >= 1
            && s.npc_balls_struck >= 1
            && s.npc_extractions + s.npc_zone_eliminations >= 1,
        "the rivals did not make contact and extraction matter in the golfer's whole match",
        format!("{s:?}"),
    );
    // E6: no hole-out settles before the hole opens.
    for (name, played) in [("idle", &idle), ("golfer", &golfer), ("full", &full)] {
        let early = matches!(
            played.state,
            MatchState::Over {
                outcome: Outcome::HoledOut | Outcome::Lost { .. },
                ..
            }
        ) && played.ticks < 5700;
        checks.require(
            !early,
            "a hole-out ended the match before the hole opened",
            describe(name, played),
        );
    }
    gallery.say(format!("rivals in the golfer's whole match: {s:?}"));
}
