//! The gates over whole runs: the three players and the replay, the
//! contracts no played session reaches, and every screen judged against the
//! camera and the floors — staged end screens included.
//!
//! A child of `gates` for the size convention; it shares that file's helpers.

use super::*;

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
            format!(
                "{} {word} {}",
                crate::screen::NAMES[seat],
                last.banked(seat)
            )
        })
        .collect();
    format!("{name}: over at {:?}; {}", s.over, seats.join(", "))
}

pub(super) fn players(checks: &mut Checks, runs: &Runs, summary: &mut Vec<String>) {
    for (name, s) in [("planner", &runs.p), ("chaser", &runs.c), ("idle", &runs.i)] {
        summary.push(fates_line(name, s));
        checks.require(
            s.over.is_some_and(|t| t < MATCH_TICKS),
            "a match against the NPCs did not finish",
            format!("{name}: over at {:?} after {} ticks", s.over, s.ticks()),
        );
        let last = s.at(s.ticks());
        let open = (0..4)
            .filter(|seat| last.golfer(*seat).fate.alive())
            .count();
        checks.require(
            open == 0,
            "a finished match left a golfer without a fate",
            format!("{name}: {open} golfers still alive at the end"),
        );
    }
    summary.extend(runs.reports.iter().cloned());
    // Idle dies on the tick zone_at predicts for where it stands.
    let i = &runs.i;
    let first_out = first_outside(i);
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

pub(super) fn contracts(checks: &mut Checks, a: &Session) {
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
    let found: Vec<Option<usize>> = names
        .iter()
        .map(|n| order.find(&format!(". {n}\n")))
        .collect();
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
    world.insert_resource(Match::Over {
        at: 1,
        winner: None,
    });
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

pub(super) fn screens(checks: &mut Checks, runs: &Runs, summary: &mut Vec<String>) {
    let mut clearance = f32::MAX;
    let mut judged =
        |checks: &mut Checks, name: &str, frame: &FrameRecord, snap: &Snapshot, font| {
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
        let ball = quads.iter().position(|q| {
            q.tint == SEAT_COLORS[0] && q.bounds().size().x <= 2.0 * BALL_RADIUS + 1e-3
        });
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
        (
            "nobody survives",
            everyone_out as fn(&mut World),
            "the zone took everyone",
        ),
        ("N2 survives", n2_survives, "N2 wins with 13"),
    ] {
        let (frame, snap, font) = staged(set);
        let panel = whole(&snap);
        checks.require(
            panel.all_strings().any(|s| s == want),
            "a staged result screen does not say who won",
            format!(
                "{name}: want {want:?} among {:?}",
                panel.all_strings().collect::<Vec<_>>()
            ),
        );
        judged(checks, name, &frame, &snap, font);
    }
    // The floors bite on the screens they were written for.
    let mut overlap: Panel<Art> = Panel::default();
    overlap.text(TextRun::new(
        Vec2::new(700.0, 100.0),
        "zone: IN",
        small(LINE),
    ));
    overlap.text(TextRun::new(
        Vec2::new(700.0, 102.0),
        "pad: OPEN",
        small(LINE),
    ));
    let bites = judge_panel(&overlap, &FLOORS, &[], &[])
        .iter()
        .any(|b| b.what == "two rows of chrome text overlap");
    checks.require(
        bites,
        "the overlap floor does not bite",
        "two rows 2 units apart".to_owned(),
    );
    let snap = runs.b.at(runs.b.ticks());
    // A second overlay drawn over the result: the state the game holds as one
    // `Match` and so cannot reach, staged.
    let mut twice = whole(snap);
    let mut second: Panel<Art> = Panel::default();
    second.text(TextRun::new(Vec2::new(300.0, 300.0), "PAUSED", small(LINE)));
    twice.absorb(second.lifted(crate::layers::OVERLAY));
    let breaches = judge_panel(
        &twice,
        &FLOORS,
        &[],
        &[("RESULT", "RESULT"), ("PAUSED", "PAUSED")],
    );
    let names: Vec<&str> = breaches.iter().map(|b| b.what).collect();
    summary.push(format!("two overlays breach: {names:?}"));
    checks.require(
        names.contains(&"two overlays' content is in one frame"),
        "the two-overlays floor does not bite",
        format!("{names:?}"),
    );
    summary.push(format!(
        "closest quad to the edge: {clearance:.2} world units"
    ));
}
