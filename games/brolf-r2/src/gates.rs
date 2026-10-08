//! The gates: every claim the verify run makes, each against the runs in
//! `verify.rs`, each reporting the numbers it judged.
//!
//! Expectations are shipped literals wherever a constant is under test —
//! `5.0s`, `185`, `13.5`, `11` — never arithmetic over the constant itself, so
//! the mutation round (`mutants/r1.txt`) can see the constant move.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, find_bounds};
use jidousha::ui::{Panel, TextRun, frame_text_floor, judge_frame, judge_panel};

mod whole_runs;
use whole_runs::{contracts, players, screens};

use crate::checks::{Checks, greater, near};
use crate::rules::{
    BALL_RADIUS, COURSE_MAX, Fate, Item, PAD_OPENS_TICK, effects, landing_of, next_zone, polar,
    roll_distance, roll_ticks, rolled, zone_at,
};
use crate::screen::{
    Art, FLOORS, LANDING_IN, LINE, ORANGE, OVERLAY_DIM, SEAT_COLORS, UiMap, ZONE_NEXT, ZONE_NOW,
    aim_preview, small, whole,
};
use crate::sim::{Match, Snapshot, read_snapshot};
use crate::verify::{HEADLESS_VIEWPORT, MATCH_TICKS, Runs, Session, with_golfer};
use crate::{TURF, config, register};

/// How far run A's first strike rolled, recorded from the first run and kept.
const FIRST_ROLL: f32 = 11.2775;
/// The reach the panel prints for that strike, recorded from the first run.
const REACH_LINE: &str = "reach 11.3 lands IN";

/// The fixed timestep every closed form here is asked at.
fn dt() -> Seconds {
    config().fixed_dt
}

/// The camera every recorded frame was drawn with.
fn camera_of(snap_world_camera: Camera) -> Camera {
    Camera {
        viewport: HEADLESS_VIEWPORT,
        ..snap_world_camera
    }
}

fn the_camera() -> Camera {
    camera_of(Camera {
        center: Vec2::ZERO,
        height: crate::VIEW_HEIGHT,
        clear_color: TURF,
        ..Camera::default()
    })
}

fn has(panel: &[String], line: &str) -> bool {
    panel.iter().any(|row| row == line)
}

fn starts(panel: &[String], prefix: &str) -> bool {
    panel.iter().any(|row| row.starts_with(prefix))
}

/// The box around the quads tinted `tint`.
fn tinted(frame: &FrameRecord, tint: Color) -> Option<Rect> {
    find_bounds(frame.quads().into_iter().filter(|q| q.tint == tint))
}

/// The box around the quads tinted `tint` that cover `at`.
fn tinted_at(frame: &FrameRecord, tint: Color, at: Vec2) -> Option<Rect> {
    find_bounds(frame.covering(at).into_iter().filter(|q| q.tint == tint))
}

fn square(rect: Option<Rect>, side: f32, slack: f32) -> bool {
    rect.is_some_and(|r| {
        greater(slack, (r.size().x - side).abs()) && greater(slack, (r.size().y - side).abs())
    })
}

/// Every gate, in the order the design lists them; returns the summary lines.
pub fn judge(checks: &mut Checks, runs: &Runs) -> Vec<String> {
    let mut summary = Vec::new();
    zone_row(checks, &runs.a);
    contact_row(checks, &runs.a);
    equipment_row(checks, &runs.a, &runs.b);
    extract_row(checks, &runs.b);
    players(checks, runs, &mut summary);
    contracts(checks, &runs.a);
    screens(checks, runs, &mut summary);
    summary
}

// --- decision row 1: the zone ---------------------------------------------------

fn zone_row(checks: &mut Checks, a: &Session) {
    let panel = a.panel(49);
    for line in ["BROLF 0:00 zone 1/4", "holds 19.2s", "zone: IN", REACH_LINE] {
        checks.require(
            has(panel, line),
            "the status does not show the zone and the reach while aiming",
            format!("run A tick 49: want {line:?} among {panel:?}"),
        );
    }
    checks.require(
        starts(panel, "aim:"),
        "the status shows no aim row while a ball is in reach",
        format!("run A tick 49: {panel:?}"),
    );
    let snap = a.at(49);
    if let Some(frame) = a.frames.get(&49) {
        let now = tinted(frame, ZONE_NOW);
        let next = tinted(frame, ZONE_NEXT);
        let center_of = |r: Option<Rect>| r.map(|r| r.center());
        checks.require(
            square(now, 2.0 * 7.8 + 0.08, 0.02)
                && center_of(now).is_some_and(|c| c.distance(snap.zone.center) < 0.02),
            "the current zone ring is not the circle zone_at reports",
            format!(
                "ring box {now:?}, want {:.2} square about {:?}",
                2.0 * 7.8 + 0.08,
                snap.zone.center
            ),
        );
        checks.require(
            square(next, 2.0 * 5.0 + 0.06, 0.02)
                && center_of(next).is_some_and(|c| c.distance(snap.course.zones[1].center) < 0.02),
            "the next zone ring is not zone 1",
            format!(
                "ring box {next:?}, want {:.2} square about {:?}",
                2.0 * 5.0 + 0.06,
                snap.course.zones[1].center
            ),
        );
        checks.require(
            now.zip(next).is_some_and(|(n, x)| n.contains_rect(x)),
            "the next zone ring is not inside the current one",
            format!("current {now:?}, next {next:?}"),
        );
    }
    // The landing disc and where the ball stops.
    let preview = aim_preview(snap, 0);
    match (preview, a.frames.get(&49)) {
        (Some(preview), Some(frame)) => {
            checks.require(
                square(
                    tinted_at(frame, LANDING_IN, preview.landing.at),
                    0.44,
                    0.002,
                ),
                "no green landing disc where the preview says the ball stops",
                format!(
                    "at {:?}: {}",
                    preview.landing.at,
                    crate::checks::sizes_covering(frame, preview.landing.at)
                ),
            );
            checks.require(
                preview.in_zone_at_arrival,
                "the preview says the first strike lands outside the zone",
                format!("{preview:?}"),
            );
            checks.require(
                preview.landing.ticks == 101,
                "the first strike does not roll the 101 ticks a 13.5 strike rolls",
                format!("{preview:?}"),
            );
            let rest = a.at(151).ball(0);
            checks.require(
                rest.at_rest && rest.pos == preview.landing.at,
                "the ball did not stop exactly where the preview said",
                format!(
                    "tick 151: ball at {:?} (at rest {}), preview {:?}",
                    rest.pos, rest.at_rest, preview.landing.at
                ),
            );
            let moving = a.at(150).ball(0);
            checks.require(
                !moving.at_rest,
                "the ball stopped before the tick the roll says",
                format!("tick 150: {moving:?}"),
            );
        }
        (preview, _) => checks.require(
            false,
            "run A has no aim preview or no frame at tick 49",
            format!("{preview:?}"),
        ),
    }
    // The grace countdown and the elimination tick.
    let panel = a.panel(460);
    checks.require(
        has(panel, "zone: OUT 5.0s"),
        "the grace countdown does not read 5.0s with 300 ticks of grace left",
        format!("run A tick 460: {panel:?}"),
    );
    let fate = |tick: u64| a.at(tick).golfer(0).fate;
    checks.require(
        fate(759) == Fate::Playing && fate(760) == Fate::Eliminated { at: 760 },
        "the player was not eliminated on tick 760",
        format!("tick 759 {:?}, tick 760 {:?}", fate(759), fate(760)),
    );
    let first_out = first_outside(a);
    checks.require(
        first_out.is_some_and(|t| t + 359 == 760),
        "elimination is not the grace period after the first tick outside",
        format!("first tick outside {first_out:?}, eliminated on 760"),
    );
}

/// The first tick `zone_at` leaves the player or its ball outside, walking
/// the recorded positions: a tick judges where things stood after the one
/// before it (a stage lands between ticks), and nothing here moves in between.
fn first_outside(s: &Session) -> Option<u64> {
    (2..=s.ticks()).find(|tick| {
        let before = s.at(tick - 1);
        let zone = zone_at(&before.course, *tick);
        !zone.contains(before.golfer(0).pos) || !zone.contains(before.ball(0).pos)
    })
}

// --- decision row 2: contact ------------------------------------------------------

fn contact_row(checks: &mut Checks, a: &Session) {
    let panel = a.panel(2);
    for line in ["F: club N1 (stun 3.0s)", "  drops Spikes"] {
        checks.require(
            has(panel, line),
            "the reach cue does not name the NPC before the club",
            format!("run A tick 2: want {line:?} among {panel:?}"),
        );
    }
    if let Some(frame) = a.frames.get(&2) {
        let n1 = a.at(2).golfer(1).pos;
        let ring = tinted_at(frame, ORANGE, n1);
        checks.require(
            square(ring, 1.04, 0.002),
            "no orange ring under the NPC in club reach",
            format!("at {n1:?}: orange box {ring:?}, want 1.04 square"),
        );
    }
    let n1 = |tick: u64| a.at(tick).golfer(1).fate;
    checks.require(
        n1(4) == Fate::Playing,
        "N1 was not playing before the club",
        format!("tick 4 {:?}", n1(4)),
    );
    let stunned = (5..185).all(|t| n1(t) == Fate::Stunned { until: 185 });
    checks.require(
        stunned && n1(185) == Fate::Playing,
        "the club did not stun N1 for exactly 3.0s",
        format!(
            "tick 5 {:?}, tick 184 {:?}, tick 185 {:?}",
            n1(5),
            n1(184),
            n1(185)
        ),
    );
    let removed = (5..=300).find(|t| !n1(*t).alive());
    checks.require(
        removed.is_none(),
        "the club removed N1 from the match",
        format!("N1 not alive on tick {removed:?}: {:?}", removed.map(n1)),
    );
    let at5 = a.at(5);
    let dropped = at5
        .pickups
        .iter()
        .any(|p| p.item == Item::Spikes && p.pos.distance(at5.golfer(1).pos) <= 0.7);
    checks.require(
        dropped && at5.golfer(1).held.is_empty(),
        "the club did not drop N1's Spikes at its feet",
        format!(
            "tick 5: N1 holds {:?}, pickups {:?}",
            at5.golfer(1).held,
            at5.pickups
        ),
    );
    checks.require(
        at5.golfer(0).cooldown == 90,
        "the club did not start the 1.5s cooldown",
        format!("tick 5 cooldown {}", at5.golfer(0).cooldown),
    );
    // The sledge.
    let panel = a.panel(339);
    checks.require(
        has(panel, "Space: strike N1's ball +2"),
        "the panel does not offer the sledge before it lands",
        format!("run A tick 339: {panel:?}"),
    );
    let ball = a.at(340).ball(1);
    let speed = ball.landing.map(|l| l.at);
    let rolling = !ball.at_rest;
    let points = (a.at(339).golfer(0).points, a.at(340).golfer(0).points);
    checks.require(
        rolling && points == (0, 2),
        "the sledge did not roll N1's ball and pay 2",
        format!("tick 340: N1's ball rolling {rolling} to {speed:?}; points 339/340 {points:?}"),
    );
    let sent = a.at(341).ball(1).pos - a.at(340).ball(1).pos;
    let along = polar(1.0, Radians::ZERO);
    checks.require(
        greater(sent.dot(along), 0.0) && near(sent.y, 0.0) && near(sent.x, rolled(13.5, 1, dt())),
        "the sledged ball did not go where the striker aimed at 13.5",
        format!("tick 340 to 341 it moved {sent:?}"),
    );
}

// --- decision row 3: equipment --------------------------------------------------

fn equipment_row(checks: &mut Checks, a: &Session, b: &Session) {
    let panel = a.panel(2);
    for line in ["E: take Driver, worth 3", "  strike 1.5x faster"] {
        checks.require(
            has(panel, line),
            "the pickup row does not say what the Driver does",
            format!("run A tick 2: want {line:?} among {panel:?}"),
        );
    }
    checks.require(
        !starts(panel, "  swaps"),
        "the pickup row offers a swap with a free slot",
        format!("run A tick 2: {panel:?}"),
    );
    let panel = b.panel(1931);
    for line in [
        "E: take Helmet, worth 3",
        "  club stuns 1.0s not 3.0s",
        "  swaps out Heavy",
    ] {
        checks.require(
            has(panel, line),
            "the pickup row does not say what the Helmet does and what it swaps",
            format!("run B tick 1931: want {line:?} among {panel:?}"),
        );
    }
    let held_from_10 = (10..=300).all(|t| a.at(t).golfer(0).held == vec![Item::Driver]);
    checks.require(
        held_from_10 && a.at(9).golfer(0).held.is_empty(),
        "E did not take the Driver on tick 10",
        format!(
            "tick 9 {:?}, tick 10 {:?}",
            a.at(9).golfer(0).held,
            a.at(10).golfer(0).held
        ),
    );
    checks.require(
        effects(&[Item::Driver]).shot_scale == 1.5
            && effects(&[Item::Helmet]).stun_ticks == 60
            && effects(&[Item::Spikes]).walk_scale == 1.5
            && effects(&[Item::Heavy]).sledged_scale == 0.5,
        "the equipment effects are not the ones the rows print",
        format!(
            "Driver {:?}; Helmet {:?}",
            effects(&[Item::Driver]),
            effects(&[Item::Helmet])
        ),
    );
    let start = a.at(49).ball(0).pos;
    let rest = a.at(151).ball(0).pos;
    let gone = rest.distance(start);
    checks.require(
        near(gone, FIRST_ROLL),
        "the Driver strike did not roll as far as a 13.5 strike rolls",
        format!("rolled {gone:.4}, want {FIRST_ROLL}"),
    );
    checks.require(
        a.at(50).ball(0).landing.is_some() && !a.at(50).ball(0).at_rest,
        "the Driver strike did not start rolling on the release tick",
        format!("tick 50: {:?}", a.at(50).ball(0)),
    );
}

// --- decision row 4: extraction -----------------------------------------------

fn number_after(panel: &[String], prefix: &str) -> Option<u32> {
    panel
        .iter()
        .find(|row| row.starts_with(prefix))
        .and_then(|row| row.rsplit(' ').next())
        .and_then(|n| n.parse().ok())
}

fn extract_row(checks: &mut Checks, b: &Session) {
    let panel = b.panel(1931);
    checks.require(
        has(panel, "X: extract, keeps 11"),
        "the extract row does not promise points plus the held worth",
        format!("run B tick 1931: {panel:?}"),
    );
    let fate = b.at(1935).golfer(0).fate;
    checks.require(
        fate == Fate::Extracted { banked: 11 } && b.at(1934).golfer(0).fate.alive(),
        "X did not extract with 11 on tick 1935",
        format!(
            "tick 1934 {:?}, tick 1935 {fate:?}",
            b.at(1934).golfer(0).fate
        ),
    );
    let last = b.panel(b.ticks());
    let promised = number_after(panel, "X: extract");
    let kept = number_after(last, "YOU  extracted");
    checks.require(
        has(last, "YOU  extracted  11") && promised.is_some() && promised == kept,
        "the result does not keep what the status promised",
        format!("promised {promised:?}, result {kept:?}, last panel {last:?}"),
    );
    checks.require(
        b.over.is_some_and(|t| t < MATCH_TICKS) && has(last, "RESULT"),
        "run B never reached its result screen",
        format!("over at {:?}, last panel {last:?}", b.over),
    );
    if let Some((_, frame)) = &b.last {
        let map = UiMap::for_camera(&the_camera());
        let p = jidousha::ui::Mapping::to_world(&map, Vec2::new(480.0, 400.0));
        let top = frame.covering(p).first().map(|q| q.tint);
        checks.require(
            top == Some(OVERLAY_DIM),
            "the result overlay is not on top of the course",
            format!("at {p:?} the top quad is tinted {top:?}"),
        );
    }
}
