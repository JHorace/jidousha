//! Sessions B and C: the contact row and the equipment row.
//!
//! Key functions: `session_b`, `session_c`.
//! Depends on: `gates` (staging), `verify`, `checks`, `draw`, `items`, `world`. Never
//! depended on outside the game's check.
//! INVARIANT: every other rival is frozen, and each number a check expects is a literal
//! written in the check, never arithmetic over the constant under test.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecorder, InputScript};

use crate::checks::{Checks, dots_at_radius, font_quads_in, within};
use crate::draw::TARGET_RING;
use crate::gates::{flight, place, player, positions, set_kit, side, staged, stun, thaw};
use crate::items::{Item, Kit, Slot};
use crate::verify::Gallery;
use crate::world::{Pickup, Stats};
use crate::{LINE_Y, TEXT_SIZE};

/// Session B: the human clubs `P1`, then strikes its ball.
pub fn session_b(checks: &mut Checks, gallery: &mut Gallery) {
    let mut run = staged(&[1]);
    let m = side(run.world());
    let human = Vec2::new(-8.0 * m, 0.0);
    place(run.sim.world_mut(), 0, human, human + Vec2::new(0.0, 0.8));
    let p1_ball = Vec2::new(2.0 * m, 0.0);
    place(
        run.sim.world_mut(),
        1,
        p1_ball + Vec2::new(0.8 * m, 0.0),
        p1_ball,
    );
    stun(run.sim.world_mut(), 1);
    let kit = Kit {
        ball: Some(Item::HeavyBall),
        body: None,
        last_taken: Some(Slot::Ball),
    };
    set_kit(run.sim.world_mut(), 1, kit);
    run.script = InputScript::new().press(Key::Space, 200);
    let mut recorder = FrameRecorder::new(crate::verify::HEADLESS_VIEWPORT);
    run.to(198);
    // One unit from the human, on the side the hole is on; a golfer walking to its ball
    // moves 0.07 a tick, so it is still within 1.5 on ticks 199 and 200.
    place(
        run.sim.world_mut(),
        1,
        human + Vec2::new(1.0 * m, 0.0),
        p1_ball,
    );
    thaw(run.sim.world_mut(), 1);
    run.step();
    let at = gallery.snap(
        &mut recorder,
        &mut run,
        "clubbing, the reach cue on tick 199",
    );
    let shot = &gallery.shots[at];
    let cue = shot.strings[2].clone();
    let quads = font_quads_in(&shot.frame, shot.font, LINE_Y[2], LINE_Y[2] + TEXT_SIZE);
    let p1_at = positions(run.world(), 1).map_or(Vec2::ZERO, |(body, _)| body);
    let ring = dots_at_radius(&shot.frame, TARGET_RING, 0.1, p1_at, 0.5, 0.02);
    checks.require(
        cue == "SPACE: club P1 (dazes 3.0s, drops heavy ball)",
        "the reach cue does not say what the club will do",
        format!("cue on tick 199: {cue:?}"),
    );
    checks.require(
        quads == cue.chars().count() && ring == 12,
        "the cue is not drawn in the bottom band, or the target is not ringed",
        format!(
            "{} characters, {quads} font quads in the line; {ring} orange dots round {p1_at:?}",
            cue.chars().count()
        ),
    );
    run.step();
    let p1 = player(run.world(), 1);
    let me = player(run.world(), 0);
    checks.require(
        p1.is_some_and(|p| p.dazed_until == 380 && p.kit.ball.is_none()),
        "a club did not daze P1 until tick 380 and rob it",
        format!(
            "P1 after tick 200: dazed_until {:?}, ball slot {:?}",
            p1.map(|p| p.dazed_until),
            p1.map(|p| p.kit.ball)
        ),
    );
    let pickups = run.world().query::<&Pickup>().count();
    checks.require(
        me.is_some_and(|p| p.kit.ball == Some(Item::HeavyBall)) && pickups == 8,
        "the robbed item did not land in the clubber's hands",
        format!(
            "human ball slot {:?}; {pickups} pickups on the course (8 dealt)",
            me.map(|p| p.kit.ball)
        ),
    );
    let at200 = positions(run.world(), 1);
    run.to(379);
    let at379 = positions(run.world(), 1);
    checks.require(
        at200.is_some() && at200 == at379,
        "a dazed golfer moved",
        format!("P1 at the end of tick 200 {at200:?}, of tick 379 {at379:?}"),
    );
    run.to(420);
    let at420 = positions(run.world(), 1);
    checks.require(
        at420.is_some() && at420 != at379,
        "a golfer is still dazed after the daze should have ended",
        format!("P1 at tick 379 {at379:?}, at tick 420 {at420:?}"),
    );
    run.to(1200);
    checks.require(
        player(run.world(), 1).is_some() && positions(run.world(), 1).is_some(),
        "a club removed P1 from the match",
        "P1 has no body on tick 1200".to_owned(),
    );
    gallery.say(format!(
        "contact: clubbed P1 on tick 200 (dazed until 380, robbed of its heavy ball); P1 moved again by tick 420 and was alive on tick 1200; cue {cue:?}"
    ));

    // The strike half: P1 stands three units off, only its ball is in reach.
    let mut run = staged(&[1]);
    let m = side(run.world());
    let human = Vec2::new(-8.0 * m, 0.0);
    place(run.sim.world_mut(), 0, human, human + Vec2::new(0.0, 0.8));
    let ball = human + Vec2::new(1.0 * m, 0.0);
    place(
        run.sim.world_mut(),
        1,
        human + Vec2::new(3.0 * m, 0.0),
        ball,
    );
    stun(run.sim.world_mut(), 1);
    run.script = InputScript::new().press(Key::Space, 5);
    run.to(5);
    let struck = flight(run.world(), 1);
    let want_to = ball + Vec2::new(4.0 * m, 0.0);
    checks.require(
        struck.is_some_and(|f| {
            (f.from - ball).length() < 1e-3 && (f.to - want_to).length() < 1e-3 && f.start == 5
        }),
        "a strike did not send the ball four units straight away from the striker",
        format!("flight {struck:?}; wanted {ball:?} -> {want_to:?} starting on tick 5"),
    );
    run.to(5 + 20);
    let rest = positions(run.world(), 1).map(|(_, b)| b);
    checks.require(
        flight(run.world(), 1).is_none() && rest.is_some_and(|b| (b - want_to).length() < 1e-3),
        "the struck ball did not come to rest where the strike sent it",
        format!("ball at {rest:?} on tick 25, wanted {want_to:?}"),
    );
    let struck_count = run.world().resource::<Stats>().npc_balls_struck;
    checks.require(
        struck_count == 1,
        "the strike was not counted",
        format!("{struck_count} rival balls struck"),
    );
    gallery.say(format!(
        "contact: struck P1's ball from {ball:?} to {want_to:?}, at rest on tick 25"
    ));
}

/// Session C: the human takes a heavy ball, and what its label says is what the sim then does.
pub fn session_c(checks: &mut Checks, gallery: &mut Gallery) {
    let mut run = staged(&[1]);
    stun(run.sim.world_mut(), 1);
    let m = side(run.world());
    let anchor = Vec2::new(-8.0 * m, 0.0);
    place(
        run.sim.world_mut(),
        0,
        anchor,
        anchor + Vec2::new(0.8 * m, 0.0),
    );
    let heavy_at = anchor + Vec2::new(4.0 * m, 0.0);
    let corner = Vec2::new(-13.5 * m, -7.2);
    let mut moved_heavy = false;
    for (_, transform, pickup) in run.sim.world_mut().query_mut::<(&mut Transform, &Pickup)>() {
        if pickup.0 == Item::HeavyBall && !moved_heavy {
            transform.pos = heavy_at;
            moved_heavy = true;
        } else {
            transform.pos = corner;
        }
    }
    let key = if m > 0.0 { Key::D } else { Key::A };
    run.script = InputScript::new().hold(key, 2..200);
    let mut recorder = FrameRecorder::new(crate::verify::HEADLESS_VIEWPORT);
    let mut near_at: Option<usize> = None;
    let mut taken_at: Option<u64> = None;
    let mut first = true;
    while run.tick < 200 {
        run.step();
        if first {
            let at = gallery.snap(&mut recorder, &mut run, "equipment, the first walk tick");
            let line = &gallery.shots[at].strings[3];
            checks.require(
                line.is_empty(),
                "a pickup label is showing with the nearest pickup four units away",
                format!("pickup line on tick {}: {line:?}", run.tick),
            );
            first = false;
        }
        let Some((body, _)) = positions(run.world(), 0) else {
            break;
        };
        let distance = (body - heavy_at).length();
        if near_at.is_none() && distance <= 2.5 {
            near_at = Some(gallery.snap(&mut recorder, &mut run, "equipment, the label in reach"));
        }
        if taken_at.is_none() && distance <= 0.6 {
            taken_at = Some(run.tick);
            let me = player(run.world(), 0);
            let left = run.world().query::<&Pickup>().count();
            checks.require(
                me.is_some_and(|p| p.kit.ball == Some(Item::HeavyBall)) && left == 7,
                "walking onto the heavy ball did not take it",
                format!(
                    "tick {}: ball slot {:?}, {left} pickups left (7 expected)",
                    run.tick,
                    me.map(|p| p.kit.ball)
                ),
            );
        }
    }
    match near_at {
        Some(at) => {
            let shot = &gallery.shots[at];
            let line = shot.strings[3].clone();
            let quads = font_quads_in(&shot.frame, shot.font, LINE_Y[3], LINE_Y[3] + TEXT_SIZE);
            checks.require(
                line == "TAKE heavy ball: strikes on it x0.5, your reach x0.75",
                "the label does not say what the heavy ball does",
                format!("pickup line when first within 2.5: {line:?}"),
            );
            checks.require(
                quads == line.chars().count(),
                "the label is not drawn in the bottom band's second line",
                format!(
                    "{} characters, {quads} font quads in the line",
                    line.chars().count()
                ),
            );
        }
        None => checks.require(
            false,
            "the human never came within reach of the label",
            format!("heavy ball at {heavy_at:?}"),
        ),
    }
    checks.require(
        taken_at.is_some(),
        "the human never walked onto the heavy ball",
        format!("heavy ball at {heavy_at:?}"),
    );

    // Part one of the effect: a rival's strike on the human's ball travels 4.0 x 0.5.
    run.to(220);
    let ball = Vec2::new(-4.0 * m, 3.0);
    place(run.sim.world_mut(), 0, Vec2::new(-9.0 * m, -5.0), ball);
    place(
        run.sim.world_mut(),
        1,
        Vec2::new(-5.0 * m, 3.0),
        Vec2::new(-13.0 * m, -7.0),
    );
    thaw(run.sim.world_mut(), 1);
    let shoot_from = Vec2::new(-8.0 * m, 3.0);
    let pointer = Vec2::new(shoot_from.x + 20.0 * m, 3.0);
    let screen = run.camera().world_to_screen(pointer);
    run.script = InputScript::new()
        .pointer_at(391, screen)
        .click(PointerButton::Primary, 400);
    run.step();
    run.step();
    let struck = flight(run.world(), 0);
    checks.require(
        struck.is_some_and(|f| {
            within((f.to - f.from).length(), 2.0, 1e-3) && (f.from - ball).length() < 1e-3
        }),
        "a strike on a heavy ball did not travel half of four units",
        format!("flight of the human's ball on tick 222: {struck:?}"),
    );
    run.to(222 + 30);
    let rest = flight(run.world(), 0).is_none();
    checks.require(
        rest,
        "the struck heavy ball never came to rest",
        format!("flight on tick 252: {:?}", flight(run.world(), 0)),
    );

    // Part two: the human's own shot is limited to 7.0 x 0.75.
    run.to(390);
    place(
        run.sim.world_mut(),
        1,
        Vec2::new(-13.0 * m, -7.0),
        Vec2::new(-13.0 * m, -7.0),
    );
    stun(run.sim.world_mut(), 1);
    place(
        run.sim.world_mut(),
        0,
        shoot_from + Vec2::new(0.5 * m, 0.0),
        shoot_from,
    );
    run.to(400);
    let shot_flight = flight(run.world(), 0);
    let length = shot_flight.map(|f| (f.to - f.from).length());
    checks.require(
        length.is_some_and(|l| (4.62..=5.88).contains(&l)),
        "a heavy ball's shot was not limited to three quarters of the longest shot",
        format!("length of the shot on tick 400: {length:?}, wanted 5.25 within 12%"),
    );
    gallery.say(format!(
        "equipment: heavy ball taken on tick {taken_at:?}; a strike on it flew {:?}; a shot with it flew {length:?} (reach 5.25)",
        struck.map(|f| (f.to - f.from).length()),
    ));
}
