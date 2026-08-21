//! `--verify`: three matches with nobody watching, then every assertion this
//! game can make about what its world did and what it drew.
//!
//! Run it: `cargo run -p jidousha --example pong -- --verify`
//!
//! The shape is the convention's: script or drive the input, tick a fixed
//! number of times, assert, print a verdict beginning with `verified `, and
//! then the transcript of one frame as evidence. Failures are collected rather
//! than exited on, because an instrument that stops at the first bad reading
//! costs a whole cycle per fault.
//!
//! Three matches, because one controller cannot measure a game's difficulty:
//! the rollout player says the game can be won, the do-nothing player says it
//! can be lost, and the chaser — the paddle a person plays on their first try —
//! is the only one of the three that says whether it is worth playing.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, DrawnQuad, FrameRecord, FrameRecorder, InputSnapshot, find_bounds,
};

use crate::checks::{Checks, fail, greater, near, near_within, sizes_covering};
use crate::controller::{Chaser, Report, Rollout};
use crate::{
    BALL_COLOR, BALL_RADIUS, DASH_COUNT, DASH_FILL, DASH_WIDTH, GOAL_X, HALF_W, HINT, MARKING,
    MAX_SPEED, Match, OPPONENT_BIAS, PADDLE_SIZE, PADDLE_X, Paddle, SCORE_GAP, SCORE_SIZE,
    SERVE_SPEED, Side, WALL_Y, WIN_SCORE, banner_lines, config, contact_span, face_contact,
    face_gap, opponent_target, rebound, register, score_text,
};

/// How long each headless match runs.
///
/// About sixty-six seconds of game at the engine's default timestep — long
/// enough for a match to seven to finish with its serve pauses, and the run
/// reports how many ticks it actually took so a shortfall reads as a number
/// rather than as a timeout.
const TICKS: u64 = 4000;

/// The surface the headless runs draw to.
///
/// The same size the window opens at, which is what makes every bounds
/// assertion below about the aspect the layout was stated for. The recorder's
/// viewport overrides the `Camera` resource's, so giving it the one the game
/// already has is what stops the question existing.
const HEADLESS_VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// Which of the three players is at the keyboard.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Player {
    /// Predicts, constrains, optimises and enumerates.
    Rollout,
    /// Follows the ball's current height.
    Chaser,
    /// Does nothing at all.
    Idle,
}

/// Everything the world was doing on one tick.
#[derive(Clone, Copy)]
struct Look {
    ball: Vec2,
    velocity: Vec2,
    speed: f32,
    player: f32,
    opponent: f32,
    state: Match,
}

/// The last frame drawn while play was live, and the world that produced it.
///
/// The frame a match *ends* on carries a banner rather than a picture of the
/// game being played, so the ordinary-layout assertions read this instead.
struct Live {
    frame: FrameRecord,
    look: Look,
    tick: u64,
}

/// What one headless match did.
struct Session {
    /// The score and the state at the end.
    outcome: Match,
    /// What the controller learned about itself.
    report: Report,
    /// The last frame drawn while play was live, if any was.
    live: Option<Live>,
    /// How many frames were recorded.
    frames: usize,
    /// The tick the match was decided on, if it was.
    decided_at: Option<u64>,
    /// The box the ball's centre stayed inside all match.
    ball_extent: Option<Rect>,
    /// How many times the ball arrived at the opponent's face.
    opponent_approaches: u32,
    /// How many of those the opponent returned.
    opponent_returns: u32,
    /// The camera the frames were drawn with.
    camera: Camera,
    /// The timestep the engine actually ran, rather than the one assumed.
    fixed_dt: f32,
    /// Every phase and its systems, in run order.
    schedule: String,
}

/// Read the world, or `None` on the way into tick 1 when it is still empty.
fn look(sim: &HeadlessSim) -> Option<Look> {
    let world = sim.world();
    let state = *world.find_resource::<Match>()?;
    let (_, ball_at, ball) = world.query::<(&Transform, &crate::Ball)>().next()?;
    let mut player = 0.0;
    let mut opponent = 0.0;
    for (_, transform, paddle) in world.query::<(&Transform, &Paddle)>() {
        match paddle.side {
            Side::Player => player = transform.pos.y,
            Side::Opponent => opponent = transform.pos.y,
        }
    }
    Some(Look {
        ball: ball_at.pos,
        velocity: ball.velocity,
        speed: ball.speed,
        player,
        opponent,
        state,
    })
}

/// Play one headless match, with `player` at the keyboard.
///
/// The recorder is fed only for the run that is going to be looked at: a match
/// of four thousand ticks holds four thousand frames, which is affordable once
/// and not three times.
fn play(player: Player, record: bool) -> (Session, HeadlessSim, FrameRecorder) {
    let mut sim = headless(config(), register);
    let mut recorder = FrameRecorder::new(HEADLESS_VIEWPORT);
    let mut rollout = Rollout::default();
    let mut chaser = Chaser::default();
    let fixed_dt = sim.world().resource::<Time>().fixed_dt.as_f32();

    let mut live: Option<Live> = None;
    let mut frames = 0;
    let mut decided_at = None;
    let mut ball_extent: Option<Rect> = None;
    let mut opponent_approaches = 0;
    let mut opponent_returns = 0;
    let mut previous: Option<Look> = None;

    for tick in 1..=TICKS {
        // On the way into tick 1 there is nothing to look at: `Startup` runs
        // *inside* that tick, so this read happens once against an empty world.
        let seen = look(&sim);
        let snapshot = match (player, seen) {
            (Player::Idle, _) | (_, None) => InputSnapshot::new(),
            (Player::Rollout, Some(now)) => rollout.decide(
                now.ball,
                now.velocity,
                now.speed,
                now.player,
                now.opponent,
                fixed_dt,
            ),
            (Player::Chaser, Some(now)) => chaser.decide(now.ball, now.player, fixed_dt),
        };
        sim.world_mut().insert_resource(Input::new(snapshot));
        sim.tick();

        let Some(now) = look(&sim) else {
            fail(
                "the world is empty after a tick",
                "Startup spawns two paddles, one ball and a Match resource",
            );
        };
        // The ball turning round at the far end: an approach the opponent met.
        if let Some(was) = previous {
            if was.velocity.x > 0.0 && now.velocity.x < 0.0 && greater(now.ball.x, 0.0) {
                opponent_approaches += 1;
                opponent_returns += 1;
            }
            // A point for the player: the ball reached the far end and was not
            // returned.
            if now.state.left > was.state.left {
                opponent_approaches += 1;
            }
        }
        previous = Some(now);

        if now.state.winner.is_some() && decided_at.is_none() {
            decided_at = Some(tick);
        }
        let playing = now.state.winner.is_none() && now.state.countdown == 0;
        if playing {
            let point = Rect::from_center_size(now.ball, Vec2::ZERO);
            ball_extent = Some(match ball_extent {
                None => point,
                Some(seen) => Rect {
                    min: seen.min.min(point.min),
                    max: seen.max.max(point.max),
                },
            });
        }
        if record {
            let frame = recorder.draw(&mut sim);
            frames += 1;
            if playing {
                live = Some(Live {
                    frame,
                    look: now,
                    tick,
                });
            }
        }
    }

    let outcome = *sim.world().resource::<Match>();
    let camera = Camera {
        viewport: HEADLESS_VIEWPORT,
        ..*sim.world().resource::<Camera>()
    };
    let schedule = sim.schedule_debug();
    let session = Session {
        outcome,
        report: rollout.report,
        live,
        frames,
        decided_at,
        ball_extent,
        opponent_approaches,
        opponent_returns,
        camera,
        fixed_dt,
        schedule,
    };
    (session, sim, recorder)
}

/// The quads a disc of `radius` at `at` is made of, as one box.
///
/// `ctx.circle` submits sixteen wedges rather than one square, so nothing the
/// size of the ball is drawn anywhere — what is true is that all sixteen share
/// the centre as a corner and all sixteen fit inside the circle's bounding box,
/// so the box around the ones covering the centre is exactly `2r` square.
fn disc_bounds(frame: &FrameRecord, at: Vec2, radius: f32, font: BackendTextureId) -> Option<Rect> {
    let box_of_it = Rect::from_center_size(at, Vec2::splat(radius * 2.0));
    find_bounds(frame.covering(at).into_iter().filter(|quad| {
        // Written out rather than as `Rect::contains`, which is half-open and
        // would throw away the wedge reaching the far edge. A filter, because
        // a centre-line dash covers the same point and is not the ball.
        let drawn = quad.bounds();
        quad.texture != font
            && quad.tint == BALL_COLOR
            && greater(drawn.min.x, box_of_it.min.x - 0.001)
            && greater(drawn.min.y, box_of_it.min.y - 0.001)
            && greater(box_of_it.max.x + 0.001, drawn.max.x)
            && greater(box_of_it.max.y + 0.001, drawn.max.y)
    }))
}

/// Whether a quad is one of the centre marking's dashes.
///
/// By size: the walls carry the same tint and are the width of the court.
fn is_dash(quad: &DrawnQuad) -> bool {
    let size = quad.bounds().size();
    let pitch = WALL_Y * 2.0 / DASH_COUNT as f32;
    quad.tint == MARKING && near(size.x, DASH_WIDTH) && near(size.y, pitch * DASH_FILL)
}

/// Put the world into a state a played session never reaches, completely.
///
/// A staged frame is not staged until all of it is: this recipe is corrective
/// rather than additive, because whatever the run left behind is still set.
fn stage(sim: &mut HeadlessSim, state: Match, ball: Vec2, paddles: (f32, f32)) {
    sim.world_mut().insert_resource(state);
    let balls: Vec<Entity> = sim
        .world()
        .query::<(&Transform, &crate::Ball)>()
        .map(|(entity, _, _)| entity)
        .collect();
    for entity in balls {
        if let Some(transform) = sim.world_mut().find_component_mut::<Transform>(entity) {
            transform.pos = ball;
        }
    }
    for (_, transform, paddle) in sim.world_mut().query_mut::<(&mut Transform, &Paddle)>() {
        transform.pos.y = match paddle.side {
            Side::Player => paddles.0,
            Side::Opponent => paddles.1,
        };
    }
}

/// Every string the game can draw, and where it comes from.
fn every_string() -> Vec<(&'static str, String)> {
    let mut out = vec![
        ("the hint line", HINT.to_owned()),
        ("a score digit", score_text(WIN_SCORE)),
    ];
    for side in [Side::Player, Side::Opponent] {
        for line in banner_lines(side) {
            out.push(("a banner line", line.to_owned()));
        }
    }
    out
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let (session, mut sim, mut recorder) = play(Player::Rollout, true);
    let (chased, _, _) = play(Player::Chaser, false);
    let (idle, _, _) = play(Player::Idle, false);
    let font = recorder.font_texture();
    let view = session.camera.visible_bounds();

    // --- the contracts a played session never exercises --------------------
    //
    // A run only tests the states it reaches, and the safety margins a game is
    // built on are exactly the states a correct game never reaches: capped at
    // half a paddle's thickness of travel per tick, the ball *cannot* tunnel,
    // so replacing the sweep with a position test would pass the entire match.
    // Ask the function its contract directly instead.
    let span = contact_span();
    let plane = -PADDLE_X;
    let gap = |x: f32| face_gap(x, plane, Side::Player);
    // One tick of travel eight units long, straight across a paddle 0.7 thick.
    let across = face_contact(gap(plane + 4.0), gap(plane - 4.0));
    let contact_x = across.map(|fraction| (plane + 4.0) + (-8.0) * fraction);
    checks.require(
        contact_x.is_some_and(|x| near(x - BALL_RADIUS, plane + PADDLE_SIZE.x / 2.0)),
        "the sweep does not find a contact for travel that crosses a whole paddle",
        format!(
            "eight units of travel from {:.2} to {:.2} across a face at {:.2} reported \
             {across:?}, putting the ball's leading edge at {:?}; a position test would \
             have missed this entirely and the match would still be 7-0",
            plane + 4.0,
            plane - 4.0,
            plane + PADDLE_SIZE.x / 2.0,
            contact_x.map(|x| x - BALL_RADIUS),
        ),
    );
    // The three negatives, which is where a naive sweep goes wrong.
    let short = face_contact(gap(plane + 4.0), gap(plane + 3.0));
    let behind = face_contact(gap(plane), gap(plane - 4.0));
    let away = face_contact(gap(plane - 4.0), gap(plane + 4.0));
    checks.require(
        short.is_none() && behind.is_none() && away.is_none(),
        "the sweep reports contact for travel that never touches the face",
        format!(
            "stopping short gave {short:?}, leaving through the same face gave {behind:?}, \
             and travelling away gave {away:?}; all three must be None"
        ),
    );
    // NaN, which is the half of this that clippy's `neg_cmp_op_on_partial_ord`
    // invites you to get wrong: `a <= b` is false for NaN where `!(a > b)` is
    // true, and the ball then sits at a NaN position for the rest of the run.
    let nan_before = face_contact(f32::NAN, -1.0);
    let nan_after = face_contact(1.0, f32::NAN);
    checks.require(
        nan_before.is_none() && nan_after.is_none(),
        "the sweep takes a contact at a NaN fraction of the tick",
        format!(
            "a NaN gap before gave {nan_before:?} and a NaN gap after gave {nan_after:?}; a \
             ball that takes one leaves at a NaN position and stays there, silently"
        ),
    );

    // The rebound: speed is conserved, the ball leaves away from the paddle it
    // struck, and it leaves towards the side of the paddle it struck.
    let flat = rebound(0.0, SERVE_SPEED, Side::Player);
    let high = rebound(-span, SERVE_SPEED, Side::Player);
    let low = rebound(span, SERVE_SPEED, Side::Player);
    let far = rebound(0.0, SERVE_SPEED, Side::Opponent);
    checks.require(
        near(flat.length(), SERVE_SPEED)
            && near(high.length(), SERVE_SPEED)
            && near(low.length(), SERVE_SPEED),
        "a rebound does not conserve the ball's speed",
        format!(
            "a {SERVE_SPEED:.1} ball came off at {:.3} flat, {:.3} off the top and {:.3} off \
             the bottom",
            flat.length(),
            high.length(),
            low.length()
        ),
    );
    checks.require(
        greater(flat.x, 0.0) && greater(0.0, far.x) && greater(0.0, high.y) && greater(low.y, 0.0),
        "a rebound sends the ball the wrong way",
        format!(
            "off the player's paddle a flat hit went {flat:?} and off the opponent's {far:?}; \
             a hit above centre went {high:?} and below centre {low:?}. Y is down, so a hit \
             above centre must leave with a negative Y"
        ),
    );

    // The opponent's rule, asked directly rather than inferred from a rally.
    let asleep = opponent_target(Vec2::new(10.0, 4.0), Vec2::new(-20.0, 0.0));
    let waiting = opponent_target(Vec2::new(-9.0, 4.0), Vec2::new(20.0, 0.0));
    let low = opponent_target(Vec2::new(9.0, 4.0), Vec2::new(20.0, 0.0));
    let high = opponent_target(Vec2::new(9.0, -4.0), Vec2::new(20.0, 0.0));
    checks.require(
        near(asleep, 0.0)
            && near(waiting, 0.0)
            && near(low, 4.0 + OPPONENT_BIAS)
            && near(high, -4.0 - OPPONENT_BIAS),
        "the opponent does not follow the handicap the game is balanced around",
        format!(
            "with the ball going away it aimed at {asleep:.2}, with the ball coming but \
             still behind the handicap line at {waiting:.2}, and with the ball at heights \
             4.00 and -4.00 past it at {low:.2} and {high:.2}"
        ),
    );
    // And the requirement that constant exists to meet, which it cannot move:
    // an opponent that meets the ball on its own centre line returns it dead
    // flat, and against anyone who does the same the rally has nowhere to go.
    // Stated as "off-centre enough to matter" rather than as the constant.
    let bias = (low - 4.0).abs();
    checks.require(
        greater(bias, span * 0.25) && greater(span, bias),
        "the opponent meets the ball too near its own centre to put an angle on it",
        format!(
            "it aims {bias:.2} off centre on a paddle that can return the ball from \
             {span:.2} either side, which is {:.0}% of its reach. Too little and every \
             return goes straight back down the middle; more than the reach and it misses \
             on purpose",
            bias / span * 100.0
        ),
    );

    // --- the two constants that are secretly one -------------------------
    //
    // A paddle's thickness reads as cosmetic and is not: it is the largest
    // `speed * fixed_dt` the game may reach. Asserted against the timestep the
    // engine actually handed this run rather than against the 1/60 assumed
    // when the constants were picked.
    let travel = MAX_SPEED * session.fixed_dt;
    checks.require(
        greater(PADDLE_SIZE.x, travel),
        "the ball can travel further in one tick than a paddle is thick",
        format!(
            "{MAX_SPEED:.1} units a second at a {:.5}s timestep is {travel:.3} units of \
             travel against a paddle {:.2} thick; nothing in v1 sweeps for you, so raise \
             the paddle's thickness before the speed",
            session.fixed_dt, PADDLE_SIZE.x
        ),
    );

    // The layout is stated for one aspect, and this is the assertion that says
    // the aspect it was stated for is the one the camera actually has.
    checks.require(
        near_within(view.max.x, HALF_W, 0.01) && near_within(view.max.y, crate::HALF_H, 0.01),
        "the camera does not show the court this layout was written for",
        format!(
            "the camera shows {view:?}; the constants say half-width {HALF_W:.4} and \
             half-height {:.4}, and every position in the game is stated against those",
            crate::HALF_H
        ),
    );

    // --- the order the systems run in --------------------------------------
    //
    // The game decides that a paddle counts as standing still at its post-move
    // position, which is only true while both paddles move before the ball.
    // Nothing else in this surface sees a swap of two `add_system` calls: the
    // world ends up legal either way, one tick of paddle travel apart, and
    // every assertion about where things ended up passes.
    let order = &session.schedule;
    let at = |name: &str| order.find(name);
    let (player_at, opponent_at, ball_at) = (
        at("drive_the_player"),
        at("drive_the_opponent"),
        at("move_the_ball"),
    );
    checks.require(
        player_at.is_some() && opponent_at.is_some() && ball_at.is_some(),
        "a system the schedule assertion names is not in the schedule",
        format!(
            "drive_the_player {player_at:?}, drive_the_opponent {opponent_at:?}, \
             move_the_ball {ball_at:?}; two renamed systems give two Nones, which compare \
             equal, and the order check below would then pass while seeing nothing"
        ),
    );
    if let (Some(one), Some(two), Some(ball)) = (player_at, opponent_at, ball_at) {
        checks.require(
            one < ball && two < ball,
            "the ball moves before the paddles do",
            format!(
                "in run order: drive_the_player at {one}, drive_the_opponent at {two}, \
                 move_the_ball at {ball}. The sweep treats a paddle as a plane that stands \
                 still, so a paddle that has not moved yet is a plane in last tick's place \
                 and a ball passes through a paddle closing on it"
            ),
        );
    }

    // --- three players, three verdicts -------------------------------------
    let Session {
        outcome,
        ref report,
        ref live,
        frames,
        decided_at,
        ball_extent,
        opponent_approaches,
        opponent_returns,
        ..
    } = session;

    checks.require(
        outcome.winner == Some(Side::Player) && outcome.left >= WIN_SCORE,
        "the rollout player did not win the match",
        format!(
            "it finished {}-{} after {TICKS} ticks, met {} of {} approaches, aimed its \
             returns {:.2} units from the opponent and landed them {:.2} from where it \
             planned. All three healthy and still not winning means the game is the half to \
             open, not the controller",
            outcome.left,
            outcome.right,
            report.met,
            report.approaches,
            report.mean_planned_gap(),
            report.mean_aim_error(),
        ),
    );
    checks.require(
        idle.outcome.winner == Some(Side::Opponent) && idle.outcome.left == 0,
        "a player who does nothing at all is not beaten nil",
        format!(
            "the do-nothing run finished {}-{}; a game that cannot be lost by standing \
             still is not being defended by anything",
            idle.outcome.left, idle.outcome.right
        ),
    );
    // The one that measures the *game* rather than a controller. A Pong only a
    // rollout player can win is a Pong nobody will enjoy, and a Pong the first
    // thing anyone writes wins 7-0 has no opponent in it.
    let chaser_won = chased.outcome.left;
    let chaser_lost = chased.outcome.right;
    checks.require(
        chaser_won >= 1 && chaser_lost >= 1,
        "chasing the ball is not a way to play this game",
        format!(
            "a paddle that simply follows the ball finished {chaser_won}-{chaser_lost}. \
             Both sides centring on the ball is the degenerate groove: nobody can score and \
             the rally has nowhere to go. A person's first try has to be able to win a \
             point and lose one"
        ),
    );
    // And the requirement stated where the game actually operates rather than
    // at its most favourable point: the opponent has to be scoreable at the
    // speeds a rally really reaches, which is a number this run already has.
    let returned = if opponent_approaches == 0 {
        0.0
    } else {
        opponent_returns as f32 / opponent_approaches as f32
    };
    checks.require(
        greater(returned, 0.15) && greater(0.95, returned),
        "the opponent is either a wall or a sieve",
        format!(
            "it returned {opponent_returns} of {opponent_approaches} balls that reached it, \
             {:.0}%. Under 15% it is not defending; over 95% no shot exists and the match is \
             decided by how long anyone is willing to wait",
            returned * 100.0
        ),
    );
    checks.require(
        decided_at.is_some(),
        "no one won the match",
        format!(
            "after {TICKS} ticks it was {}-{}, longest rally {} touches, top ball speed \
             {:.1} units a second; the rollout player met {} of {} approaches",
            outcome.left,
            outcome.right,
            outcome.longest_rally,
            outcome.top_speed,
            report.met,
            report.approaches,
        ),
    );

    // --- the ball stayed in the court --------------------------------------
    let Some(extent) = ball_extent else {
        fail(
            "the ball was never in play",
            "every tick of the match was a serve pause or an ended match",
        );
    };
    let ball_limit = WALL_Y - BALL_RADIUS;
    checks.require(
        greater(ball_limit + 0.001, extent.max.y)
            && greater(extent.min.y, -ball_limit - 0.001)
            && greater(GOAL_X + 0.001, extent.max.x)
            && greater(extent.min.x, -GOAL_X - 0.001),
        "the ball left the court",
        format!(
            "over the match its centre ranged {extent:?}; the walls hold it to +/-\
             {ball_limit:.2} vertically and a goal is scored at +/-{GOAL_X:.2}, so anything \
             outside that is a ball drawn off the side of the screen"
        ),
    );

    // --- what was drawn, on the last frame play was live on ----------------
    let Some(live) = live.as_ref() else {
        fail(
            "no frame was recorded while play was live",
            "the loop draws every tick and the match is live for most of them",
        );
    };
    let frame = &live.frame;
    let quads = frame.quads();
    checks.require(
        frames == TICKS as usize,
        "one frame per tick was expected",
        format!("{frames} frames for {TICKS} ticks"),
    );

    // Both paddles, by bounds rather than by "something is there". A
    // paddle-sized quad covers its own centre even when it is drawn a long way
    // out of position, so covering a point says a quad is nearby and only its
    // bounds say where it is.
    for (name, side, y) in [
        ("the player's", Side::Player, live.look.player),
        ("the opponent's", Side::Opponent, live.look.opponent),
    ] {
        let at = Vec2::new(side.sign() * PADDLE_X, y);
        let placed = frame.covering(at).into_iter().any(|quad| {
            let bounds = quad.bounds();
            near(bounds.size().x, PADDLE_SIZE.x)
                && near(bounds.size().y, PADDLE_SIZE.y)
                && near(bounds.center().x, at.x)
                && near(bounds.center().y, at.y)
        });
        checks.require(
            placed,
            "a paddle is not drawn where the world puts it",
            format!(
                "{name} paddle is at ({:.2}, {:.2}) and is {}x{}; what covers that point is \
                 {}",
                at.x,
                at.y,
                PADDLE_SIZE.x,
                PADDLE_SIZE.y,
                sizes_covering(frame, at)
            ),
        );
    }

    // The ball, which is sixteen wedges and not one quad.
    let disc = disc_bounds(frame, live.look.ball, BALL_RADIUS, font);
    let disc_size = disc.map_or(Vec2::ZERO, |rect| rect.size());
    checks.require(
        near(disc_size.x, BALL_RADIUS * 2.0) && near(disc_size.y, BALL_RADIUS * 2.0),
        "there is no ball-sized disc where the ball is",
        format!(
            "the world has it at ({:.2}, {:.2}) with radius {BALL_RADIUS:.2}; the wedges \
             covering that point span {:.3}x{:.3} and want {:.2} square. What covers it is {}",
            live.look.ball.x,
            live.look.ball.y,
            disc_size.x,
            disc_size.y,
            BALL_RADIUS * 2.0,
            sizes_covering(frame, live.look.ball),
        ),
    );

    // --- the score: where it is, stated as the requirement -----------------
    //
    // Not `quad.min.y < SCORE_TOP + margin`, which moves with its constant —
    // put SCORE_TOP in the middle of the court and that check follows it down,
    // passes, and leaves the score across the play. The requirement names no
    // constant the game owns: the score sits in the top third of what the
    // camera shows, one number either side of the centre line, evenly set.
    let top_third = view.min.y + view.size().y / 3.0;
    let glyphs: Vec<DrawnQuad> = quads
        .iter()
        .copied()
        .filter(|quad| quad.texture == font)
        .collect();
    let score_glyphs: Vec<DrawnQuad> = glyphs
        .iter()
        .copied()
        .filter(|quad| greater(top_third, quad.bounds().max.y))
        .collect();
    checks.require(
        score_glyphs.len() == 2,
        "the score is not two digits in the top third of the court",
        format!(
            "{} glyphs sit above y={top_third:.2}, out of {} drawn in all; the score is \
             {}-{} and each side is one digit",
            score_glyphs.len(),
            glyphs.len(),
            live.look.state.left,
            live.look.state.right,
        ),
    );
    let left_edge = score_glyphs
        .iter()
        .filter(|quad| greater(0.0, quad.bounds().max.x))
        .map(|quad| quad.bounds().max.x)
        .fold(f32::MIN, f32::max);
    let right_edge = score_glyphs
        .iter()
        .filter(|quad| greater(quad.bounds().min.x, 0.0))
        .map(|quad| quad.bounds().min.x)
        .fold(f32::MAX, f32::min);
    checks.require(
        near_within(-left_edge, right_edge, 0.01) && greater(right_edge, 0.0),
        "the score is not set evenly either side of the centre line",
        format!(
            "the left number ends {:.3} before the line and the right one starts {right_edge:.3} \
             after it",
            -left_edge
        ),
    );
    // And that the digits say what the world says. No assertion over drawn
    // quads can read a digit, so this asks the *width* — which catches a score
    // drawn from the wrong number only when the digit count changes, and is
    // the honest limit of what a frame can be asked here.
    checks.require(
        near(right_edge, SCORE_GAP) && near(-left_edge, SCORE_GAP),
        "the score is not set against the gap the layout states",
        format!(
            "it is set {:.3} and {right_edge:.3} either side; the layout says {SCORE_GAP:.2}",
            -left_edge
        ),
    );

    // --- the bands ---------------------------------------------------------
    //
    // A frame carries the order quads were drawn in, not the `Depth` that
    // produced it, so a band is only visible where it *changes* that order.
    // `register` submits the play before the court for exactly this reason:
    // both come back sorted the other way round, so swapping two constants in
    // `mod layers` moves them and a check can say so.
    let dash_at = quads.iter().position(is_dash);
    let ball_at = quads.iter().position(|quad| {
        quad.texture != font
            && quad.tint == BALL_COLOR
            && quad.bounds().size().x < BALL_RADIUS * 2.1
    });
    let hint_at = glyphs
        .iter()
        .position(|quad| greater(quad.bounds().min.y, 0.0));
    checks.require(
        dash_at.is_some() && ball_at.is_some() && hint_at.is_some(),
        "one of the three bands drew nothing in the last live frame",
        format!(
            "as indices into the draw order: a centre dash {dash_at:?}, a ball wedge \
             {ball_at:?}, a hint glyph {hint_at:?}; None means that band drew nothing where \
             it was looked for"
        ),
    );
    if let (Some(dash), Some(ball)) = (dash_at, ball_at) {
        checks.require(
            dash < ball,
            "the centre marking is drawn over the ball instead of under it",
            format!(
                "the dash is at index {dash} in the draw order and a ball wedge at {ball}; \
                 the game submits the court *after* the play, so only FIELD sorting under \
                 PLAY can put the dash first"
            ),
        );
    }

    // --- nothing off screen, and by how much -------------------------------
    let off_screen: Vec<Rect> = quads
        .iter()
        .map(|quad| quad.bounds())
        .filter(|bounds| !view.contains_rect(*bounds))
        .collect();
    checks.require(
        off_screen.is_empty(),
        "something was drawn outside what the camera shows",
        format!(
            "{} of {} quads fall outside {view:?}; the first is {:?} — text centred by \
             TextStyle::width_of is the usual culprit",
            off_screen.len(),
            quads.len(),
            off_screen.first(),
        ),
    );
    let clearance = quads
        .iter()
        .map(|quad| {
            let bounds = quad.bounds();
            let closest = (bounds.min - view.min).min(view.max - bounds.max);
            closest.x.min(closest.y)
        })
        .fold(f32::MAX, f32::min);

    // --- the background, which leaves no quad behind ------------------------
    let cleared = frame.plan.clear_color;
    checks.require(
        cleared == crate::COURT,
        "the court was cleared to a colour the game does not name",
        format!(
            "the frame cleared to {cleared:?}; the game's constant is {:?}",
            crate::COURT
        ),
    );
    // And the requirement the colour exists to meet, which the constant cannot
    // move: the ball is white and has to read against the court.
    let brightness = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        greater(0.25, brightness) && greater(cleared.a, 0.99),
        "the court is not dark enough for a white ball to read against",
        format!(
            "its brightest channel is {brightness:.3} at alpha {:.2}",
            cleared.a
        ),
    );

    // --- the strings themselves ---------------------------------------------
    //
    // No assertion over drawn quads can see a wrong character: the font draws
    // an unknown one as a box at exactly a letter's advance, so a stray em
    // dash, curly quote or middle dot passes the glyph count, the centring and
    // the bounds check alike.
    for (name, text) in every_string() {
        let stray = text.chars().find(|glyph| !(' '..='~').contains(glyph));
        checks.require(
            stray.is_none(),
            "a string the game draws has a character the font cannot draw",
            format!(
                "{name}, {text:?}, contains {stray:?}, which draws as a box at exactly a \
                 letter's width - no assertion over what was drawn can tell the difference"
            ),
        );
    }

    // --- the screens this run never reached ---------------------------------
    //
    // The bounds assertion above only judges frames that were drawn, and a
    // controller good enough to win is a controller that never loses: the
    // losing banner is the longest string in the game and nothing has measured
    // it. Build it by hand — and correctively, setting every piece of state the
    // frame depends on, including the state nothing is being asked about.
    let mut staged: Vec<(&str, FrameRecord)> = Vec::new();
    for (name, winner, score) in [
        ("the winning screen", Side::Player, (WIN_SCORE, 3)),
        ("the losing screen", Side::Opponent, (2, WIN_SCORE)),
    ] {
        stage(
            &mut sim,
            Match {
                left: score.0,
                right: score.1,
                countdown: 0,
                serve_towards: Side::Player,
                winner: Some(winner),
                rally: 0,
                longest_rally: 9,
                top_speed: MAX_SPEED,
            },
            Vec2::ZERO,
            (0.0, 0.0),
        );
        staged.push((name, recorder.draw(&mut sim)));
    }
    for (name, screen) in &staged {
        let strays: Vec<Rect> = screen
            .quads()
            .iter()
            .map(|quad| quad.bounds())
            .filter(|bounds| !view.contains_rect(*bounds))
            .collect();
        checks.require(
            strays.is_empty(),
            "a screen the match never reached draws outside the camera",
            format!(
                "{name} puts {} of {} quads outside {view:?}; the first is {:?}. Each banner \
                 line is centred by its own width_of, and a line one character too long runs \
                 off both edges in silence",
                strays.len(),
                screen.quad_count(),
                strays.first(),
            ),
        );
        // Each line centred by *its own* width, not the block's: centring a
        // two-line block by one width_of centres the longer line and hangs the
        // shorter one off to the left, visibly crooked, at the right size,
        // with every other assertion passing.
        let lines = banner_line_bounds(screen, font);
        let crooked: Vec<f32> = lines
            .iter()
            .map(|bounds| bounds.center().x)
            .filter(|middle| !near_within(*middle, 0.0, 0.05))
            .collect();
        checks.require(
            lines.len() == 2 && crooked.is_empty(),
            "a banner line is not centred on the court",
            format!(
                "{name} drew {} banner lines with off-centre middles at {crooked:?}; a block \
                 centred by one width_of centres its longest line and hangs the rest to the \
                 left",
                lines.len()
            ),
        );
    }

    // And one overlap a played session does produce but never at rest: the
    // ball on a centre-line dash. `covering(p)[0]` is the depth sort read
    // backwards, so its first entry is what a player actually sees.
    stage(
        &mut sim,
        Match {
            countdown: 30,
            ..Match::new()
        },
        Vec2::ZERO,
        (0.0, 0.0),
    );
    let parked = recorder.draw(&mut sim);
    let front = parked.covering(Vec2::ZERO).into_iter().next();
    checks.require(
        front.is_some_and(|quad| quad.texture != font && quad.tint == BALL_COLOR),
        "the centre marking is painted over the ball on the centre spot",
        format!(
            "the front-most quad at the centre spot, where a dash and the parked ball both \
             sit, is {:?} rather than a ball wedge",
            front.map(|quad| (quad.tint, quad.bounds().size())),
        ),
    );
    // And the other way round: the score has to be in front of a ball that
    // reaches it, which a match never produces because the ball is confined
    // below the walls.
    let in_the_score = Vec2::new(-SCORE_GAP - SCORE_SIZE * 7.0 / 18.0, crate::SCORE_TOP + 1.0);
    stage(
        &mut sim,
        Match {
            countdown: 30,
            ..Match::new()
        },
        in_the_score,
        (0.0, 0.0),
    );
    let behind = recorder.draw(&mut sim);
    let front = behind.covering(in_the_score).into_iter().next();
    checks.require(
        front.is_some_and(|quad| quad.texture == font),
        "the ball is painted over the score",
        format!(
            "the front-most quad at ({:.2}, {:.2}), inside the left score digit with the \
             ball parked on it, is {:?} rather than a glyph",
            in_the_score.x,
            in_the_score.y,
            front.map(|quad| (quad.tint, quad.bounds().size())),
        ),
    );

    let captured = crate::capture::capture_a_frame(&mut checks, frame, font);
    let verdict = checks.verdict();

    println!("verified pong over {TICKS} ticks, three players");
    println!(
        "  rollout: {}-{} won on tick {}; longest rally {} touches, top speed {:.1} u/s",
        outcome.left,
        outcome.right,
        decided_at.map_or("never".to_owned(), |tick| tick.to_string()),
        outcome.longest_rally,
        outcome.top_speed,
    );
    println!(
        "  controller: met {} of {} approaches; planned returns aimed to land {:.2} from the \
         opponent; shots landed {:.2} from where they were planned to",
        report.met,
        report.approaches,
        report.mean_planned_gap(),
        report.mean_aim_error(),
    );
    println!(
        "  chaser: {}-{}   do-nothing: {}-{}",
        chased.outcome.left, chased.outcome.right, idle.outcome.left, idle.outcome.right,
    );
    println!(
        "  opponent returned {opponent_returns} of {opponent_approaches} balls ({:.0}%)",
        returned * 100.0
    );
    println!(
        "  last live frame: tick {}, {} quads, {} glyphs, {} batches",
        live.tick,
        quads.len(),
        glyphs.len(),
        frame.plan.batches.len(),
    );
    println!("  closest quad to the edge: {clearance:.2} world units");
    println!("  ball stayed inside {extent:?}");
    println!("  capture: {captured}");
    println!("  {} checks failed", checks.failures());
    print!("{}", frame.transcript());
    verdict
}

/// The bounds of each line of the banner, top line first.
///
/// Grouped by row: `ctx.text` lays a line out from one top-left corner, and no
/// two lines of this banner share a row.
fn banner_line_bounds(frame: &FrameRecord, font: BackendTextureId) -> Vec<Rect> {
    let mut rows: Vec<Rect> = Vec::new();
    for quad in frame.quads() {
        if quad.texture != font {
            continue;
        }
        let bounds = quad.bounds();
        // The score and the hint sit outside the banner's band.
        if !near_within(bounds.center().y, crate::BANNER_TOP + 1.5, 1.6) {
            continue;
        }
        match rows
            .iter_mut()
            .find(|row| near_within(row.min.y, bounds.min.y, 0.01))
        {
            Some(row) => {
                row.min = row.min.min(bounds.min);
                row.max = row.max.max(bounds.max);
            }
            None => rows.push(bounds),
        }
    }
    rows.sort_by(|a, b| a.min.y.total_cmp(&b.min.y));
    rows
}
