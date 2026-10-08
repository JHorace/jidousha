//! The `--verify` mode: four decision gates, the match with three players,
//! the layout of a live frame, the staged end screens, the contracts, and the
//! picture. `tools/verify brolf_r3v2` runs it; the summary carries one line per
//! gate, the decision rows named `decision 1 zone` to `decision 4 extraction`.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord};

use crate::checks::{Checks, Driver, camera};
use crate::hud::{LEGEND, aiming, hud_lines_of, result_lines};
use crate::model::*;
use crate::players::Player;
use crate::rules::*;
use crate::world::Snap;

/// One played match.
struct Played {
    result: Option<Outcome>,
    tick: u64,
    live: Option<(FrameRecord, Snap, BackendTextureId)>,
    camera: Camera,
    windows: u32,
    shots: u32,
    planned_cup: Vec<f32>,
    plan_error: Vec<f32>,
    schedule: String,
}

/// Play a whole match with `player` at the keyboard against the three NPCs.
fn play_match(player: Player, record: bool) -> Played {
    let mut driver = Driver::new();
    let (mut windows, mut shots) = (0, 0);
    let (mut planned_cup, mut plan_error) = (Vec::new(), Vec::new());
    let mut in_window = false;
    let mut pending: Option<Vec2> = None;
    let mut live = None;
    while driver.tick <= MATCH_END_TICK && driver.snap().result.is_none() {
        let view = driver.snap();
        let me = view.golfer(0).copied();
        let aim = me.and_then(|me| aiming(&view, &me));
        if aim.is_some() && !in_window {
            windows += 1;
        }
        in_window = aim.is_some();
        if record && driver.tick.is_multiple_of(10) && driver.tick > 0 {
            live = Some((driver.draw(), view.clone(), driver.font()));
        }
        let tapped = driver.play(player);
        let after = driver.snap();
        let moving = after.ball(0).is_some_and(|b| b.vel != Vec2::ZERO);
        if tapped
            && moving
            && let Some(aim) = aim
        {
            shots += 1;
            in_window = false;
            planned_cup.push(aim.roll.rest.distance(CUP));
            pending = Some(aim.roll.rest);
        }
        if let Some(plan) = pending
            && let Some(ball) = after.ball(0)
            && ball.vel == Vec2::ZERO
        {
            plan_error.push(ball.pos.distance(plan));
            pending = None;
        }
    }
    Played {
        result: driver.snap().result,
        tick: driver.tick,
        live,
        camera: camera(&driver),
        windows,
        shots,
        planned_cup,
        plan_error,
        schedule: driver.schedule(),
    }
}

/// The mean of `values`, or 0.
fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

/// A result, for the summary.
fn told(played: &Played) -> String {
    let kind = played
        .result
        .as_ref()
        .map_or("none".to_owned(), |o| format!("{:?}", o.kind));
    format!("{kind} t={}", played.tick)
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let mut frames: Vec<(String, FrameRecord)> = Vec::new();
    let mut summary = vec![
        crate::gates::zone(&mut checks, &mut frames),
        crate::gates::contact(&mut checks),
        crate::gates_more::equipment(&mut checks),
        crate::gates_more::extraction(&mut checks, &mut frames),
    ];
    let good = play_match(Player::Good, true);
    let chaser = play_match(Player::Chaser, false);
    let idle = play_match(Player::Idle, false);
    checks.require(
        good.result.is_some() && chaser.result.is_some(),
        "the match: a match did not reach its result screen",
        format!("good {}; chaser {}", told(&good), told(&chaser)),
    );
    checks.require(
        idle.result
            .as_ref()
            .is_some_and(|o| o.kind == EndKind::Eliminated),
        "the match: a player who does nothing was not eliminated",
        format!("idle {}", told(&idle)),
    );
    summary.push(format!(
        "players: good {} | chaser {} | idle {}",
        told(&good),
        told(&chaser),
        told(&idle)
    ));
    summary.push(format!(
        "good player: met {} of {} shot opportunities; planned rests {:.2} from the cup; shots \
         rested {:.2} from where planned",
        good.shots,
        good.windows,
        mean(&good.planned_cup),
        mean(&good.plan_error)
    ));
    checks.require(
        mean(&good.plan_error) < 0.005,
        "the match: shots did not rest where the aim line said",
        format!(
            "mean error {:.4} over {} shots",
            mean(&good.plan_error),
            good.plan_error.len()
        ),
    );
    summary.push(layout(&mut checks, &good));
    summary.push(crate::gates_more::staged_screens(&mut checks, &mut frames));
    summary.push(crate::gates_more::contracts(&mut checks));
    summary.push(bounds(&mut checks, &good, &frames));
    let captured = match &good.live {
        Some((frame, _, font)) => crate::capture::capture_a_frame(&mut checks, frame, *font),
        None => "skipped, no live frame was recorded".to_owned(),
    };
    let verdict = checks.verdict();
    println!("verified brolf_r3v2: {} checks failed", checks.failed());
    for line in summary {
        println!("  {line}");
    }
    println!("  capture: {captured}");
    if let Some((frame, _, _)) = &good.live {
        print!("{}", frame.transcript());
    }
    verdict
}

/// The last live frame of the match: the HUD above the course, the legend
/// below it, the background, every string printable, the system order.
fn layout(checks: &mut Checks, played: &Played) -> String {
    let Some((frame, view, font)) = &played.live else {
        checks.require(
            false,
            "layout: the match recorded no live frame",
            String::new(),
        );
        return "layout: not run".to_owned();
    };
    let glyphs: Vec<Rect> = frame
        .quads()
        .iter()
        .filter(|q| q.texture == *font)
        .map(|q| q.bounds())
        .collect();
    let hud_low = glyphs
        .iter()
        .filter(|b| b.max.y <= 0.0 && b.min.y < COURSE.min.y)
        .map(|b| b.max.y)
        .fold(f32::MIN, f32::max);
    let legend_high = glyphs
        .iter()
        .filter(|b| b.min.y > COURSE.max.y - 0.5)
        .map(|b| b.min.y)
        .fold(f32::MAX, f32::min);
    checks.require(
        hud_low <= COURSE.min.y + 1e-4 && legend_high >= COURSE.max.y - 1e-4,
        "layout: the HUD or the legend overlaps the course",
        format!("lowest HUD glyph ends at {hud_low:.2}, legend starts at {legend_high:.2}; course {COURSE:?}"),
    );
    let cleared = frame.plan.clear_color;
    let brightest = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        cleared == palette::CLEAR && brightest < 0.25,
        "layout: the background is not the dark clear colour",
        format!("cleared {cleared:?}"),
    );
    let lines = hud_lines_of(view);
    let mut strings: Vec<String> = lines.to_vec();
    for kind in [
        EndKind::Holed,
        EndKind::LastStanding,
        EndKind::Extracted,
        EndKind::Eliminated,
        EndKind::Lost { by: 2 },
    ] {
        strings.extend(result_lines(&outcome_of(
            kind,
            &Kit::of(&[Item::LongClub, Item::Helmet]),
        )));
    }
    strings.push(LEGEND.to_owned());
    strings.extend([Item::Driver, Item::LongClub, Item::LeadBall, Item::Helmet].map(describe));
    let stray: Vec<&String> = strings
        .iter()
        .filter(|s| !s.chars().all(|c| (' '..='~').contains(&c)))
        .collect();
    let long: Vec<&String> = lines
        .iter()
        .filter(|line| line.chars().count() > 80)
        .collect();
    checks.require(
        stray.is_empty() && long.is_empty(),
        "layout: a string is unprintable, or a HUD line is over 80 characters",
        format!("unprintable {stray:?}; long {long:?}"),
    );
    let names = [
        "player_intent",
        "npc_intents",
        "apply_contacts",
        "apply_walks",
        "apply_shots_takes_extracts",
        "move_balls",
        "zone_grace",
        "end_match",
    ];
    let found: Vec<Option<usize>> = names
        .iter()
        .map(|name| played.schedule.find(&format!(". {name}\n")))
        .collect();
    let ascending =
        found.iter().all(Option::is_some) && found.windows(2).all(|pair| pair[0] < pair[1]);
    checks.require(
        ascending,
        "layout: the Update systems are not all present in their decided order",
        format!("positions {found:?} in {:?}", played.schedule),
    );
    format!(
        "layout: HUD ends at y {hud_low:.2}, legend from {legend_high:.2}; Update order as decided"
    )
}

/// Nothing drawn outside the camera, on any frame the run kept.
fn bounds(checks: &mut Checks, played: &Played, frames: &[(String, FrameRecord)]) -> String {
    let view = played.camera.visible_bounds();
    checks.require(
        (view.size().x - 2.0 * HALF_W).abs() < 1e-3,
        "bounds: the camera is not as wide as the window's shape makes it",
        format!("{view:?} against a half-width of {HALF_W}"),
    );
    let mut clearance = f32::MAX;
    let live = played
        .live
        .iter()
        .map(|(frame, _, _)| ("the match, live".to_owned(), frame.clone()));
    let all: Vec<(String, FrameRecord)> = frames.iter().cloned().chain(live).collect();
    for (label, frame) in &all {
        let quads = frame.quads();
        let outside: Vec<Rect> = quads
            .iter()
            .map(|q| q.bounds())
            .filter(|b| !view.contains_rect(*b))
            .collect();
        checks.require(
            outside.is_empty(),
            "bounds: something was drawn outside the camera",
            format!(
                "{label}: {} quads outside {view:?}, first {:?}",
                outside.len(),
                outside.first()
            ),
        );
        for quad in &quads {
            let b = quad.bounds();
            let gap = (b.min - view.min).min(view.max - b.max);
            clearance = clearance.min(gap.x.min(gap.y));
        }
    }
    format!(
        "bounds: {} frames; closest quad to the edge: {clearance:.2} world units",
        all.len()
    )
}
