//! The gates: every claim the verify run makes, each against the runs in
//! `verify.rs`, each reporting the numbers it judged.
//!
//! Expectations are shipped literals wherever a constant is under test —
//! `5.0s`, `185`, `13.5`, `11` — never arithmetic over the constant itself, so
//! the mutation round (`mutants/r1.txt`) can see the constant move.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, find_bounds};
use jidousha::ui::{Panel, TextRun, frame_text_floor, judge_frame, judge_panel};

use crate::checks::{Checks, greater, near};
use crate::rules::{
    BALL_RADIUS, COURSE_MAX, Fate, Item, effects, landing_of, next_zone, polar, roll_distance,
    roll_ticks, rolled, zone_at, PAD_OPENS_TICK,
};
use crate::screen::{
    Art, FLOORS, LANDING_IN, LINE, ORANGE, OVERLAY_DIM, SEAT_COLORS, UiMap, ZONE_NEXT, ZONE_NOW,
    aim_preview, result_panel, small, whole,
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
                && center_of(next)
                    .is_some_and(|c| c.distance(snap.course.zones[1].center) < 0.02),
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
                square(tinted_at(frame, LANDING_IN, preview.landing.at), 0.44, 0.002),
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
    let first_out = (1..=RUN_A_LAST).find(|tick| {
        let s = a.at(*tick);
        let zone = zone_at(&s.course, *tick);
        !zone.contains(s.golfer(0).pos) || !zone.contains(s.ball(0).pos)
    });
    checks.require(
        first_out.is_some_and(|t| t + 359 == 760),
        "elimination is not the grace period after the first tick outside",
        format!("first tick outside {first_out:?}, eliminated on 760"),
    );
}

const RUN_A_LAST: u64 = crate::verify::RUN_A_TICKS;

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
        format!("tick 5: N1 holds {:?}, pickups {:?}", at5.golfer(1).held, at5.pickups),
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
        format!("tick 9 {:?}, tick 10 {:?}", a.at(9).golfer(0).held, a.at(10).golfer(0).held),
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
        format!("tick 1934 {:?}, tick 1935 {fate:?}", b.at(1934).golfer(0).fate),
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

// --- the players -----------------------------------------------------------------

fn fates_line(name: &str, s: &Session) -> String {
    let last = s.at(s.ticks());
    let seats: Vec<String> = (0..4)
        .map(|seat| {
            let word = match last.golfer(seat).fate {
                Fate::Extracted { .. } => "extracted",
                Fate::Survived { .. } => "survived",
                Fate::Eliminated { .. } => "eliminated",
                Fate::Playing | Fate::Stunned { .. } => "playing",
            };
            format!("{} {word} {}", crate::screen::NAMES[seat], last.banked(seat))
        })
        .collect();
    format!("{name}: over at {:?}; {}", s.over, seats.join(", "))
}

fn players(checks: &mut Checks, runs: &Runs, summary: &mut Vec<String>) {
    for (name, s) in [("planner", &runs.p), ("chaser", &runs.c), ("idle", &runs.i)] {
        summary.push(fates_line(name, s));
        checks.require(
            s.over.is_some_and(|t| t < MATCH_TICKS),
            "a match against the NPCs did not finish",
            format!("{name}: over at {:?} after {} ticks", s.over, s.ticks()),
        );
        let last = s.at(s.ticks());
        let open = (0..4).filter(|seat| last.golfer(*seat).fate.alive()).count();
        checks.require(
            open == 0,
            "a finished match left a golfer without a fate",
            format!("{name}: {open} golfers still alive at the end"),
        );
    }
    summary.extend(runs.reports.iter().cloned());
    // Idle dies on the tick zone_at predicts for where it stands.
    let i = &runs.i;
    let first_out = (1..=i.ticks()).find(|tick| {
        let s = i.at(*tick);
        let zone = zone_at(&s.course, *tick);
        !zone.contains(s.golfer(0).pos) || !zone.contains(s.ball(0).pos)
    });
    let idle_fate = i.at(i.ticks()).golfer(0).fate;
    checks.require(
        first_out.is_some_and(|t| idle_fate == Fate::Eliminated { at: t + 359 }),
        "the idle player was not eliminated when the zone left it",
        format!("first tick outside {first_out:?}, fate {idle_fate:?}"),
    );
    let chaser_best = runs
        .c
        .snaps
        .iter()
        .map(|s| s.golfer(0).points)
        .max()
        .unwrap_or(0);
    summary.push(format!("chaser's best points {chaser_best}"));
    checks.require(
        chaser_best >= 2,
        "the chaser never holed a cup",
        format!("best points {chaser_best}; {}", runs.reports[1]),
    );
    let bank = |s: &Session| s.at(s.ticks()).banked(0);
    checks.require(
        bank(&runs.p) > 0 && bank(&runs.p) >= bank(&runs.c),
        "the planner banked less than the chaser",
        format!("planner {}, chaser {}", bank(&runs.p), bank(&runs.c)),
    );
    let (met, intended) = runs.planner.met();
    let landed = runs.planner.landed_off();
    checks.require(
        intended > 0 && met * 10 >= intended * 8,
        "the planner missed its own ball on too many strikes",
        format!("met {met} of {intended}"),
    );
    checks.require(
        greater(0.6, landed),
        "the planner's strikes land far from where it planned them",
        format!("landed {landed:.3} from planned, want under 0.6"),
    );
    // The same seed replays the same match.
    let end = |s: &Session| {
        let last = s.at(s.ticks());
        (0..4)
            .map(|seat| (last.golfer(seat).fate, last.banked(seat)))
            .collect::<Vec<_>>()
    };
    let transcript = |s: &Session| s.last_live.as_ref().map(|(t, f)| (*t, f.transcript()));
    checks.require(
        end(&runs.p) == end(&runs.p2)
            && runs.p.over == runs.p2.over
            && transcript(&runs.p) == transcript(&runs.p2),
        "the same seed did not replay the same match",
        format!(
            "P {:?} over {:?}; P2 {:?} over {:?}",
            end(&runs.p),
            runs.p.over,
            end(&runs.p2),
            runs.p2.over
        ),
    );
}

// --- the contracts play never exercises -------------------------------------------

fn contracts(checks: &mut Checks, a: &Session) {
    let fence = crate::rules::course();
    let hit = landing_of(Vec2::new(5.0, 0.0), Vec2::X, 18.0, fence, dt());
    checks.require(
        hit.fenced && hit.at.x == COURSE_MAX.x - BALL_RADIUS && hit.ticks < roll_ticks(18.0, dt()),
        "a strike at the fence does not stop dead on it",
        format!("{hit:?}"),
    );
    let from = Vec2::new(-4.5, 0.5);
    let free = landing_of(from, Vec2::X, 6.0, fence, dt());
    checks.require(
        !free.fenced && free.at == from + Vec2::X * roll_distance(6.0, dt()),
        "an unfenced strike does not stop where the roll says",
        format!("{free:?}"),
    );
    checks.require(
        rolled(18.0, roll_ticks(18.0, dt()), dt()) == roll_distance(18.0, dt())
            && rolled(18.0, 10_000, dt()) == roll_distance(18.0, dt()),
        "the roll goes on after it stops",
        format!(
            "{} vs {}",
            rolled(18.0, 10_000, dt()),
            roll_distance(18.0, dt())
        ),
    );
    let course = &a.at(1).course;
    let mid = zone_at(course, 1560);
    checks.require(
        near(mid.radius, 6.4)
            && zone_at(course, 1920) == course.zones[1]
            && zone_at(course, 1200) == course.zones[0]
            && zone_at(course, 5581).radius == 0.0
            && next_zone(course, 1921) == Some(course.zones[2])
            && next_zone(course, 5581).is_none()
            && PAD_OPENS_TICK == 1921,
        "zone_at is not the schedule",
        format!(
            "1560 {mid:?}; 1920 {:?} vs {:?}; 5581 {:?}; next(1921) {:?}",
            zone_at(course, 1920),
            course.zones[1],
            zone_at(course, 5581),
            next_zone(course, 1921)
        ),
    );
    let order = &a.schedule;
    let names = [
        "read_player_intent",
        "decide_npcs",
        "walk",
        "act",
        "roll_balls",
        "settle_balls",
        "judge_zone",
        "end_match",
    ];
    let found: Vec<Option<usize>> = names.iter().map(|n| order.find(&format!(". {n}\n"))).collect();
    let in_order = found.iter().all(Option::is_some) && found.windows(2).all(|w| w[0] < w[1]);
    checks.require(
        in_order,
        "the Update systems do not run in the decided order",
        format!("positions {found:?} in {order}"),
    );
}

// --- the screens ---------------------------------------------------------------

/// Bounds, printable strings and the floors over one frame and its panel.
fn judge_screen(
    checks: &mut Checks,
    name: &str,
    frame: &FrameRecord,
    panel: &Panel<Art>,
    font: jidousha::testing::BackendTextureId,
) -> f32 {
    let camera = the_camera();
    let view = camera.visible_bounds();
    let mut clearance = f32::MAX;
    for quad in frame.quads() {
        let b = quad.bounds();
        checks.require(
            view.contains_rect(b),
            "drawn off screen",
            format!("{name}: {b:?} against {view:?}"),
        );
        let gap = (b.min - view.min).min(view.max - b.max);
        clearance = clearance.min(gap.x.min(gap.y));
    }
    for text in panel.all_strings() {
        checks.require(
            text.chars().all(|c| (' '..='~').contains(&c) || c == '\n'),
            "a string the font cannot draw",
            format!("{name}: {text:?}"),
        );
    }
    let map = UiMap::for_camera(&camera);
    for breach in judge_panel(panel, &FLOORS, &[], &[("RESULT", "RESULT")]) {
        checks.require(false, breach.what, format!("{name}: {}", breach.detail));
    }
    for breach in judge_frame(panel, frame, font, &map, view) {
        checks.require(false, breach.what, format!("{name}: {}", breach.detail));
    }
    for breach in frame_text_floor(frame, font, 12.0 * jidousha::ui::Mapping::scale(&map)) {
        checks.require(false, breach.what, format!("{name}: {}", breach.detail));
    }
    clearance
}

/// A staged end screen: one tick, every fate set, the match over, one frame.
fn staged(set: fn(&mut World)) -> (FrameRecord, Snapshot, jidousha::testing::BackendTextureId) {
    let mut sim = headless(config(), register);
    let mut recorder = FrameRecorder::new(HEADLESS_VIEWPORT);
    sim.tick();
    set(sim.world_mut());
    let frame = recorder.draw(&mut sim);
    let snap = read_snapshot(&sim.world().view());
    let Some(snap) = snap else {
        crate::checks::fail("a staged screen had no course", "tick 1 ran Startup");
    };
    (frame, snap, recorder.font_texture())
}

fn everyone_out(world: &mut World) {
    for seat in 0..4 {
        with_golfer(world, seat, |g, _| g.fate = Fate::Eliminated { at: 1 });
    }
    world.insert_resource(Match::Over { at: 1, winner: None });
}

fn n2_survives(world: &mut World) {
    for seat in [0, 1, 3] {
        with_golfer(world, seat, |g, _| g.fate = Fate::Eliminated { at: 1 });
    }
    with_golfer(world, 2, |g, _| g.fate = Fate::Survived { banked: 13 });
    world.insert_resource(Match::Over {
        at: 1,
        winner: Some(2),
    });
}

fn screens(checks: &mut Checks, runs: &Runs, summary: &mut Vec<String>) {
    let mut clearance = f32::MAX;
    let mut judged = |checks: &mut Checks, name: &str, frame: &FrameRecord, snap: &Snapshot, font| {
        let c = judge_screen(checks, name, frame, &whole(snap), font);
        clearance = clearance.min(c);
    };
    let a = &runs.a;
    if let (Some(frame), Some(font)) = (a.frames.get(&49), a.font) {
        judged(checks, "run A tick 49", frame, a.at(49), font);
        checks.require(
            frame.plan.clear_color == TURF,
            "the frame is not cleared to the turf",
            format!("{:?}", frame.plan.clear_color),
        );
        let c = frame.plan.clear_color;
        let bright = c.r.max(c.g).max(c.b);
        checks.require(
            bright < 0.25 && c.a > 0.99,
            "the turf is not dark enough to read white rings on",
            format!("brightest channel {bright:.3}, alpha {:.2}", c.a),
        );
        // The band: the course is submitted after play, so only FIELD < PLAY
        // can put a fence line before the player's ball in the draw order.
        let quads = frame.quads();
        let fence = quads.iter().position(|q| q.tint == LINE);
        let ball = quads.iter().position(|q| q.tint == SEAT_COLORS[0]
            && q.bounds().size().x <= 2.0 * BALL_RADIUS + 1e-3);
        checks.require(
            fence.zip(ball).is_some_and(|(f, b)| f < b),
            "the course is not drawn behind play",
            format!("fence quad at {fence:?}, ball quad at {ball:?}"),
        );
        let at = a.at(49).ball(0).pos;
        let top = frame.covering(at).first().map(|q| q.tint);
        checks.require(
            top == Some(SEAT_COLORS[0]),
            "something is drawn over the player's ball",
            format!("at {at:?}: {top:?}"),
        );
        let capture = crate::capture::capture_a_frame(checks, frame, font);
        summary.push(format!("capture: {capture}"));
    }
    if let (Some((tick, frame)), Some(font)) = (&runs.b.last, runs.b.font) {
        judged(checks, "run B result", frame, runs.b.at(*tick), font);
    }
    if let (Some((tick, frame)), Some(font)) = (&runs.p.last_live, runs.p.font) {
        judged(checks, "run P last live", frame, runs.p.at(*tick), font);
    }
    for (name, set, want) in [
        ("nobody survives", everyone_out as fn(&mut World), "the zone took everyone"),
        ("N2 survives", n2_survives, "N2 wins with 13"),
    ] {
        let (frame, snap, font) = staged(set);
        let panel = whole(&snap);
        checks.require(
            panel.all_strings().any(|s| s == want),
            "a staged result screen does not say who won",
            format!("{name}: want {want:?} among {:?}", panel.all_strings().collect::<Vec<_>>()),
        );
        judged(checks, name, &frame, &snap, font);
    }
    // The floors bite on the screens they were written for.
    let mut overlap: Panel<Art> = Panel::default();
    overlap.text(TextRun::new(Vec2::new(700.0, 100.0), "zone: IN", small(LINE)));
    overlap.text(TextRun::new(Vec2::new(700.0, 102.0), "pad: OPEN", small(LINE)));
    let bites = judge_panel(&overlap, &FLOORS, &[], &[])
        .iter()
        .any(|b| b.what == "two rows of chrome text overlap");
    checks.require(bites, "the overlap floor does not bite", "two rows 2 units apart".to_owned());
    let snap = runs.b.at(runs.b.ticks());
    let mut twice = whole(snap);
    twice.absorb(result_panel(snap));
    let breaches = judge_panel(&twice, &FLOORS, &[], &[("RESULT", "RESULT")]);
    let names: Vec<&str> = breaches.iter().map(|b| b.what).collect();
    summary.push(format!("two results breach: {names:?}"));
    checks.require(
        names.iter().any(|n| n.contains("overlay")),
        "the two-overlays floor does not bite",
        format!("{names:?}"),
    );
    summary.push(format!("closest quad to the edge: {clearance:.2} world units"));
}
