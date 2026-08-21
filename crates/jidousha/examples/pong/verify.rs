//! `--verify`: the game, played headlessly by three controllers, asserted on.
//!
//! The shape is the closing convention of `docs/api/jidousha-testing.md`: a
//! `verified ` verdict line, failures collected rather than exited on, and every
//! failure reporting the numbers it judged.
//!
//! Run it: `cargo run -p jidousha --example pong -- --verify`
//! Or:     `tools/verify pong`

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, Input};

use crate::checks;
use crate::controller::{Controller, Report, Style};
use crate::rules::{self, HALF_H, PADDLE_Y_LIMIT, Side, WINNING_SCORE};
use crate::{Ball, Paddle, Round, Screen, config, register};

/// The surface a headless frame is drawn to.
///
/// The same size the game's camera already has, which is the first of the two
/// ways out of the recorder-overrides-the-viewport trap and the one to take: the
/// question of which rectangle a bounds check is measured against stops existing.
const HEADLESS_VIEWPORT: PhysicalSize = rules::WINDOW;

/// How long a match is given before it is called unfinished, in ticks.
///
/// A tick is the systems and nothing else — no frame to wait for, no vsync — so
/// this is about a second of wall clock, not a budget to husband.
const TICK_LIMIT: u64 = 5_400;

/// Every string the game draws that is not a number.
const BANNERS: [&str; 3] = [
    "YOU WIN  -  PRESS R",
    "OPPONENT WINS  -  PRESS R",
    "W AND S TO MOVE",
];

/// What one played match came back with.
struct Session {
    /// The scoreboard at the end.
    round: Round,
    /// What the controller measured about itself.
    report: Report,
    /// How many ticks it took.
    ticks: u64,
    /// The longest rally, in touches by either paddle.
    longest_rally: u32,
    /// The fastest the ball ever got, in world units per second.
    top_speed: f32,
    /// How many balls reached the opponent, and how many it sent back.
    opponent_approaches: u32,
    /// How many of those it returned.
    opponent_returns: u32,
    /// The last frame drawn while the ball was actually in play.
    live: Option<Live>,
}

/// One frame of live play, and the world it was drawn from.
struct Live {
    /// The frame.
    frame: FrameRecord,
    /// Where the ball was on the tick it was drawn.
    ball: Vec2,
    /// Where the player's paddle was.
    left: f32,
    /// Where the opponent's paddle was.
    right: f32,
}

/// A finished match, with the simulation left standing so screens can be staged.
struct Played {
    /// The simulation, at the tick the match ended on.
    sim: HeadlessSim,
    /// The recorder, for staging frames the run never reached.
    recorder: FrameRecorder,
    /// What the match came to.
    session: Session,
}

/// Play one match with one controller.
///
/// `record` is off for the two supporting players: they are there for their
/// verdict lines, and drawing five thousand frames nothing reads is pure cost.
fn play(style: Style, record: bool) -> Played {
    let mut sim = headless(config(), register);
    let mut recorder = FrameRecorder::new(HEADLESS_VIEWPORT);
    let mut controller = Controller::new(style);

    let mut session = Session {
        round: Round::new(),
        report: Report::default(),
        ticks: 0,
        longest_rally: 0,
        top_speed: 0.0,
        opponent_approaches: 0,
        opponent_returns: 0,
        live: None,
    };
    let mut rally = 0_u32;
    let mut heading_right = false;
    let mut opponent_pending = false;

    for tick in 1..=TICK_LIMIT {
        let snapshot = controller.decide(&sim);
        sim.world_mut().insert_resource(Input::new(snapshot));
        sim.tick();
        controller.observe(&sim);
        session.ticks = tick;

        let Some(round) = sim.world().find_resource::<Round>().copied() else {
            continue;
        };
        session.round = round;

        let ball = sim
            .world()
            .query::<(&Transform, &Ball)>()
            .next()
            .map(|(_, transform, ball)| (transform.pos, ball.velocity, ball.speed));
        if let Some((at, velocity, speed)) = ball {
            session.top_speed = session.top_speed.max(speed);
            if round.screen == Screen::Rally {
                // A touch is the tick the ball turned around.
                let now_right = velocity.x > 0.0;
                if now_right != heading_right {
                    rally += 1;
                    session.longest_rally = session.longest_rally.max(rally);
                }
                if now_right && !opponent_pending {
                    session.opponent_approaches += 1;
                    opponent_pending = true;
                } else if !now_right && opponent_pending && at.x > 0.0 {
                    session.opponent_returns += 1;
                    opponent_pending = false;
                }
                heading_right = now_right;
            } else {
                rally = 0;
                opponent_pending = false;
            }

            if record && round.screen == Screen::Rally {
                let frame = recorder.draw(&mut sim);
                let (mut left, mut right) = (0.0, 0.0);
                for (_, transform, paddle) in sim.world().query::<(&Transform, &Paddle)>() {
                    match paddle.side {
                        Side::Left => left = transform.pos.y,
                        Side::Right => right = transform.pos.y,
                    }
                }
                // The last frame of *live* play, kept with the positions from
                // the same tick. The frame a match ends on carries the end
                // screen rather than the layout, and is the wrong thing to
                // assert the ordinary layout against.
                session.live = Some(Live {
                    frame,
                    ball: at,
                    left,
                    right,
                });
            }
        }
        if round.screen == Screen::Over {
            break;
        }
    }
    session.report = controller.report();
    Played {
        sim,
        recorder,
        session,
    }
}

/// Put the world into a named state, all of it, so a staged frame is staged.
///
/// The recipe in the document is additive — tick, insert, draw — and what a game
/// needs is corrective: whatever the match left behind is still set. So this
/// takes every piece of state the frame depends on, including the pieces the
/// check is not asking about.
fn stage(sim: &mut HeadlessSim, round: Round, ball: Vec2, left: f32, right: f32) {
    sim.world_mut().insert_resource(round);
    let ball_entity = sim
        .world()
        .query::<(&Transform, &Ball)>()
        .next()
        .map(|(entity, _, _)| entity);
    if let Some(entity) = ball_entity {
        sim.world_mut().component_mut::<Transform>(entity).pos = ball;
    }
    let paddles: Vec<(Entity, Side)> = sim
        .world()
        .query::<(&Transform, &Paddle)>()
        .map(|(entity, _, paddle)| (entity, paddle.side))
        .collect();
    for (entity, side) in paddles {
        let y = match side {
            Side::Left => left,
            Side::Right => right,
        };
        sim.world_mut().component_mut::<Transform>(entity).pos.y = y;
    }
}

/// The whole `--verify` mode.
#[expect(
    clippy::too_many_lines,
    reason = "the check list is a list; splitting it hides the order it runs in"
)]
pub(crate) fn run() -> ExitCode {
    let mut failures: Vec<String> = Vec::new();
    let mut summary: Vec<String> = Vec::new();

    // --- the contracts a played match never exercises ---------------------
    //
    // First, because they need no match at all and because a failure in one of
    // them explains everything that follows.
    for (name, outcome) in [
        (
            "the swept contact answers its contract",
            checks::the_swept_contact_answers_its_contract(),
        ),
        (
            "a steep return outruns the opponent",
            checks::a_steep_return_outruns_the_opponent(),
        ),
        (
            "every drawn string is printable",
            checks::every_drawn_string_is_printable(&BANNERS),
        ),
        (
            "there is room behind each paddle",
            checks::there_is_room_behind_each_paddle(),
        ),
        (
            "a paddle at its limit stays on the court",
            checks::a_paddle_at_its_limit_stays_on_the_court(),
        ),
    ] {
        if let Err(why) = outcome {
            failures.push(format!("{name}: {why}"));
        }
    }

    // --- the match -------------------------------------------------------

    let mut played = play(Style::Rollout, true);
    let session = &played.session;
    let dt = played.sim.world().resource::<Time>().fixed_dt;
    if let Err(why) = checks::the_ball_cannot_outrun_the_thinnest_collider(dt) {
        failures.push(format!("the ball outruns the thinnest collider: {why}"));
    }
    if let Err(why) = checks::the_paddles_move_before_the_ball(&played.sim.schedule_debug()) {
        failures.push(format!("the schedule is in the wrong order: {why}"));
    }

    summary.push(format!(
        "    rollout: {}-{} in {} ticks; longest rally {} touches; top ball speed {:.1} \
         units/s; the opponent returned {} of {} balls",
        session.round.left,
        session.round.right,
        session.ticks,
        session.longest_rally,
        session.top_speed,
        session.opponent_returns,
        session.opponent_approaches,
    ));
    summary.push(session.report.lines(Style::Rollout));

    if session.round.winner() != Some(Side::Left) {
        failures.push(format!(
            "the rollout controller did not win: {}-{} after {} ticks, longest rally {} \
             touches, top ball speed {:.1} units/s, met {} of {} approaches — a game the \
             planner cannot win is not a game a person can",
            session.round.left,
            session.round.right,
            session.ticks,
            session.longest_rally,
            session.top_speed,
            session.report.met,
            session.report.approaches,
        ));
    }
    // Stated where the game operates rather than at its most favourable point,
    // and measured rather than derived: this is the opponent as played.
    if session.opponent_approaches > 0 {
        let returned = f64::from(session.opponent_returns) / f64::from(session.opponent_approaches);
        if returned < 0.5 {
            failures.push(format!(
                "the opponent is not an opponent: it returned {} of {} balls ({:.0}%) — the \
                 rollout's win says nothing about a game whose other paddle is a wall",
                session.opponent_returns,
                session.opponent_approaches,
                returned * 100.0,
            ));
        }
    }

    // --- what a live frame shows -----------------------------------------

    let Some(live) = played.session.live.as_ref() else {
        // Nothing left to measure, so stop here rather than reporting a hundred
        // consequences of one missing reading.
        eprintln!("[pong] no frame was ever drawn during live play; there is nothing to check");
        return ExitCode::FAILURE;
    };
    // The recorder's viewport, the game's everything else. Read back after the
    // ticks rather than before, because a game may move its camera as it plays.
    let camera = Camera {
        viewport: HEADLESS_VIEWPORT,
        ..*played.sim.world().resource::<Camera>()
    };

    match checks::nothing_is_drawn_outside_the_camera(&live.frame, &camera) {
        Ok(clearance) => summary.push(format!(
            "    closest quad to the edge: {clearance:.2} world units"
        )),
        Err(why) => failures.push(format!("something is drawn off screen: {why}")),
    }
    for (name, outcome) in [
        (
            "the court is cleared to its colour",
            checks::the_court_is_cleared_to_its_colour(&live.frame),
        ),
        (
            "the court is dark enough for a white ball",
            checks::the_court_is_dark_enough_for_a_white_ball(&live.frame),
        ),
        (
            "the score sits in the top third",
            checks::the_score_sits_in_the_top_third(
                &live.frame,
                &camera,
                played.recorder.font_texture(),
            ),
        ),
        (
            "each paddle is drawn where it stands",
            checks::each_paddle_is_drawn_where_it_stands(&live.frame, live.left, live.right),
        ),
        (
            "the ball is drawn at its collider",
            checks::the_ball_is_drawn_at_its_collider(&live.frame, live.ball),
        ),
        (
            "the court markings stay behind the play",
            checks::the_court_markings_stay_behind_the_play(&live.frame),
        ),
    ] {
        if let Err(why) = outcome {
            failures.push(format!("{name}: {why}"));
        }
    }

    let live_transcript = live.frame.transcript();

    // --- the screens the run never reached --------------------------------

    let ball_colour = Color::WHITE;
    let banner_colour = Color::rgba(0.85, 0.93, 1.0, 0.85);

    // The ball parked on a centre-line dash: the court band has to lose, though
    // it was submitted last.
    stage(
        &mut played.sim,
        Round {
            screen: Screen::Rally,
            left: 3,
            right: 2,
            countdown: 0,
            serve_to: Side::Left,
        },
        Vec2::ZERO,
        0.0,
        0.0,
    );
    let on_the_dash = played.recorder.draw(&mut played.sim);
    if let Err(why) =
        checks::the_front_most_thing_at(&on_the_dash, Vec2::ZERO, ball_colour, "the ball")
    {
        failures.push(format!("the ball is behind a centre-line dash: {why}"));
    }

    // The losing screen, which a controller good enough to finish the game never
    // draws — so the longest string in the game is the one string nothing has
    // measured.
    let losing = Round {
        screen: Screen::Over,
        left: 2,
        right: WINNING_SCORE,
        countdown: 0,
        serve_to: Side::Left,
    };
    // Under the banner, so the UI band has to win though it was submitted first.
    let under_banner = Vec2::new(0.0, HALF_H - 2.0 - 1.0 + 0.5);
    stage(
        &mut played.sim,
        losing,
        under_banner,
        PADDLE_Y_LIMIT,
        -PADDLE_Y_LIMIT,
    );
    let losing_frame = played.recorder.draw(&mut played.sim);
    if let Err(why) = checks::the_front_most_thing_at(
        &losing_frame,
        under_banner,
        banner_colour,
        "the end-screen banner",
    ) {
        failures.push(format!("the banner is behind the ball: {why}"));
    }
    for (name, outcome) in [
        (
            "the losing screen draws off camera",
            checks::nothing_is_drawn_outside_the_camera(&losing_frame, &camera).map(|_| ()),
        ),
        (
            "the losing screen's paddles",
            checks::each_paddle_is_drawn_where_it_stands(
                &losing_frame,
                PADDLE_Y_LIMIT,
                -PADDLE_Y_LIMIT,
            ),
        ),
    ] {
        if let Err(why) = outcome {
            failures.push(format!("{name}: {why}"));
        }
    }

    let winning = Round {
        screen: Screen::Over,
        left: WINNING_SCORE,
        right: 2,
        countdown: 0,
        serve_to: Side::Right,
    };
    stage(&mut played.sim, winning, Vec2::ZERO, 0.0, 0.0);
    let winning_frame = played.recorder.draw(&mut played.sim);
    if let Err(why) = checks::nothing_is_drawn_outside_the_camera(&winning_frame, &camera) {
        failures.push(format!("the winning screen draws off camera: {why}"));
    }
    if let Err(why) = checks::the_two_end_screens_differ(
        &winning_frame,
        &losing_frame,
        played.recorder.font_texture(),
    ) {
        failures.push(format!("the end screens are one screen: {why}"));
    }
    // Counted in `chars()`, never `len()`: `ctx.text` submits one quad per
    // character and `str::len` is bytes, so the two disagree on exactly the
    // string the printable check exists for.
    let font = played.recorder.font_texture();
    let glyphs = winning_frame
        .quads()
        .into_iter()
        .filter(|quad| quad.texture == font)
        .count();
    let expected = BANNERS[0].chars().count()
        + format!("{}", winning.left).chars().count()
        + format!("{}", winning.right).chars().count();
    if glyphs != expected {
        failures.push(format!(
            "the winning screen drew {glyphs} glyphs, wanting {expected}: {:?} plus a \
             {}-{} score",
            BANNERS[0], winning.left, winning.right
        ));
    }

    // --- the other two players -------------------------------------------
    //
    // The rollout's win clears the mechanics and says nothing about whether the
    // game is worth playing. Only the chaser can say that, and only the idle
    // player proves the game can be lost.

    let chaser = play(Style::Chaser, false).session;
    summary.push(format!(
        "    chaser: {}-{} in {} ticks; longest rally {} touches",
        chaser.round.left, chaser.round.right, chaser.ticks, chaser.longest_rally,
    ));
    summary.push(chaser.report.lines(Style::Chaser));
    if chaser.round.left == 0 {
        failures.push(format!(
            "the chaser never scored: {}-{} in {} ticks, met {} of {} approaches, longest \
             rally {} touches — a game only a planner can score in has a rally with nowhere \
             to go, and the opponent's placement constant is what to look at before any speed",
            chaser.round.left,
            chaser.round.right,
            chaser.ticks,
            chaser.report.met,
            chaser.report.approaches,
            chaser.longest_rally,
        ));
    }
    if chaser.round.winner() == Some(Side::Left) {
        failures.push(format!(
            "the chaser won {}-{}: steering at where the ball is should not be enough, or \
             there is nothing to plan and the game is a groove",
            chaser.round.left, chaser.round.right,
        ));
    }

    let idle = play(Style::Idle, false).session;
    summary.push(format!(
        "    idle: {}-{} in {} ticks",
        idle.round.left, idle.round.right, idle.ticks,
    ));
    if idle.round.winner() != Some(Side::Right) {
        failures.push(format!(
            "a player doing nothing was not beaten: {}-{} after {} ticks — the game cannot \
             be lost, so winning it means nothing",
            idle.round.left, idle.round.right, idle.ticks,
        ));
    }

    // --- the picture ------------------------------------------------------

    match crate::capture::write_the_picture(&live.frame, &camera) {
        Ok(path) => summary.push(format!("    capture: {path}")),
        Err(why) => failures.push(format!("no picture was taken: {why}")),
    }

    // --- the verdict ------------------------------------------------------

    if failures.is_empty() {
        println!(
            "verified pong: rollout {}-{}, chaser {}-{}, idle {}-{}",
            played.session.round.left,
            played.session.round.right,
            chaser.round.left,
            chaser.round.right,
            idle.round.left,
            idle.round.right,
        );
        for line in summary {
            println!("{line}");
        }
        // Kept as evidence rather than reprinted: one frame, not the history.
        println!("{live_transcript}");
        return ExitCode::SUCCESS;
    }

    eprintln!("[pong] {} check(s) failed", failures.len());
    for line in &summary {
        eprintln!("{line}");
    }
    for (index, failure) in failures.iter().enumerate() {
        eprintln!("  {}. {failure}", index + 1);
    }
    ExitCode::FAILURE
}
