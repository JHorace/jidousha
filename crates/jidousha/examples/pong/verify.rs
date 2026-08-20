//! The check: three players, a headless match each, assertions about what the
//! world did and what was drawn, three staged screens the run never reaches, one
//! contract the run cannot exercise, and a picture.
//!
//! `cargo run -p jidousha --example pong -- --verify`.
//!
//! It runs the *same* systems and the same config the window does. What differs
//! is only what a person would otherwise supply: the keys come from
//! `controller.rs` instead of a keyboard, so the run is the same on every
//! machine and on every day.
//!
//! Nothing here exits on the first bad reading. An instrument that stops at the
//! first fault costs a whole cycle per fault, and one broken constant in this
//! game produces half a dozen readings whose *diagnostic* one is rarely first.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, DrawnQuad, FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder,
    find_bounds,
};

use crate::checks::{Checks, fail, greater, near, sizes_covering};
use crate::controller::{Mode, Player, Report};
use crate::{
    BALL_COLOR, BALL_RADIUS, Ball, COURT, COURT_COLOR, FIELD_LINE, MATCH_POINT, MAX_BALL_SPEED,
    OPPONENT_COLOR, PADDLE_SIZE, PLAYER_COLOR, Paddle, Round, Side, Stage, Tally, banner_lines,
    config, crossing, face_of, hint_text, paddle_home, register,
};

/// How many ticks a session runs before it is called a draw.
///
/// Fifty seconds of play at the engine's default timestep. A rollout match ends
/// in about two thousand; the extra thousand is headroom for the chaser, which
/// plays longer rallies and needs the room to finish rather than be cut off
/// mid-match with a score nothing can conclude from.
const TICKS: u64 = 3000;

/// The viewport the headless run draws at.
///
/// The same size the window opens at, so the recorder's viewport and the game's
/// camera agree and every assertion about *where* a quad is means something.
const HEADLESS_VIEWPORT: PhysicalSize = crate::WINDOW;

/// How long the ball must sit at the centre spot before a serve, in ticks.
///
/// A third of a second, which is about what it takes to look at a score and
/// read it. Not `SERVE_PAUSE`: a check written against the game's own constant
/// follows it to zero and passes for a game that re-serves on the tick the
/// point lands.
const READABLE_PAUSE: u64 = 20;

/// The shortest a won match may be, in ticks.
///
/// Fifteen seconds. Below it the game is not something anybody plays, it is
/// something that happens to them.
const SHORTEST_MATCH: u64 = 900;

/// What one session did.
pub(super) struct Session {
    /// Points, by `Side::index`.
    points: [u32; 2],
    /// What the match was doing when the session ended.
    stage: Stage,
    /// The rally figures at the end.
    tally: Tally,
    /// What the player had to say about itself.
    report: Report,
    /// How many ticks it took to reach `stage`.
    ticks: u64,
    /// How many frames were recorded.
    frames: usize,
    /// The furthest the ball moved in any one tick, in world units.
    longest_step: f32,
    /// The furthest from the centre line the ball's centre ever got, vertically.
    highest: f32,
    /// The shortest run of ticks the ball spent parked at the centre spot
    /// before a serve, over the whole match.
    shortest_pause: u64,
    /// The last frame drawn while the ball was live.
    live: Option<FrameRecord>,
    /// Where the ball was on that frame.
    live_ball: Vec2,
    /// Where each paddle was on that frame, by `Side::index`.
    live_paddles: [Vec2; 2],
    /// The score on that frame.
    live_points: [u32; 2],
    /// The rally figures on that frame.
    live_tally: Tally,
}

/// Play one headless match with `mode` at the keyboard.
///
/// The sim and the recorder come back with it: the staged screens below are
/// drawn from the same world the session left, which is the only way to build a
/// frame of a state a played session never reaches.
fn play(mode: Mode, record: bool) -> (Session, HeadlessSim, FrameRecorder) {
    let mut sim = headless(config(), register);
    let mut recorder = FrameRecorder::new(HEADLESS_VIEWPORT);
    let mut player = Player::new(mode);
    // The driver's own accumulator, so a controller goes through the same edge
    // rules a real keyboard does. Events, not states: a key held for a hundred
    // ticks presses exactly once.
    let mut keyboard = SnapshotBuilder::new();
    let mut holding: Option<Key> = None;

    let mut session = Session {
        points: [0, 0],
        stage: Stage::Rally,
        tally: Tally::default(),
        report: Report::default(),
        ticks: 0,
        frames: 0,
        longest_step: 0.0,
        highest: 0.0,
        shortest_pause: u64::MAX,
        live: None,
        live_ball: Vec2::ZERO,
        live_paddles: [Vec2::ZERO; 2],
        live_points: [0, 0],
        live_tally: Tally::default(),
    };
    let mut previous = Vec2::ZERO;
    let mut parked = 0u64;

    for tick in 1..=TICKS {
        // On the way into tick 1 there is nothing to look at: `Startup` runs
        // inside this call, so the world is still empty when the player reads
        // it. `Player::decide` answers `None` there rather than indexing into
        // an empty query.
        let want = player.decide(sim.world());
        if want != holding {
            if let Some(key) = holding {
                keyboard.record(InputEvent::KeyReleased(key));
            }
            if let Some(key) = want {
                keyboard.record(InputEvent::KeyPressed(key));
            }
            holding = want;
        }
        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();
        session.ticks = tick;

        let Some(round) = sim.world().find_resource::<Round>().copied() else {
            fail(
                "the match state is gone",
                "Startup inserts exactly one Round",
            );
        };
        let Some(tally) = sim.world().find_resource::<Tally>().copied() else {
            fail("the tally is gone", "Startup inserts exactly one Tally");
        };
        session.points = round.points;
        session.stage = round.stage;
        session.tally = tally;

        // How long the ball sat still at the centre before this serve. Counted
        // rather than read off the constant, so it is about what the match did.
        match round.stage {
            Stage::Serving { .. } => parked += 1,
            _ => {
                if parked > 0 {
                    session.shortest_pause = session.shortest_pause.min(parked);
                    parked = 0;
                }
            }
        }

        let ball = ball_at(&sim);
        if round.stage == Stage::Rally {
            session.longest_step = session.longest_step.max((ball - previous).length());
            session.highest = session.highest.max(ball.y.abs());
        }
        previous = ball;

        if record {
            let frame = recorder.draw(&mut sim);
            session.frames += 1;
            // The frame a match *ends* on carries the banner rather than the
            // game, so the ordinary layout is asserted against the last frame
            // drawn while play was live, with the positions from that same tick.
            if round.stage == Stage::Rally {
                session.live_ball = ball;
                session.live_paddles = paddles_at(&sim);
                session.live_points = round.points;
                session.live_tally = tally;
                session.live = Some(frame);
            }
        }
        if let Stage::Over { .. } = round.stage {
            break;
        }
    }
    session.report = player.report();
    (session, sim, recorder)
}

/// Where the ball is.
fn ball_at(sim: &HeadlessSim) -> Vec2 {
    match sim.world().query::<(&Transform, &Ball)>().next() {
        Some((_, transform, _)) => transform.pos,
        None => fail("the ball is gone", "Startup spawns exactly one"),
    }
}

/// Where both paddles are, by `Side::index`.
///
/// Sorted by the side they play rather than by the order the query yielded them
/// in: iteration order is deterministic, which is not the same as being the
/// order they were spawned in.
fn paddles_at(sim: &HeadlessSim) -> [Vec2; 2] {
    let mut found = [None, None];
    for (_, transform, paddle) in sim.world().query::<(&Transform, &Paddle)>() {
        found[paddle.side.index()] = Some(transform.pos);
    }
    match (found[0], found[1]) {
        (Some(left), Some(right)) => [left, right],
        _ => fail("a paddle is gone", "Startup spawns exactly two, one a side"),
    }
}

/// How many balls reached `side`'s paddle at all: the ones it sent back, plus
/// the ones that went past it for a point.
fn approached(session: &Session, side: Side) -> u32 {
    session.tally.returns[side.index()] + session.points[side.other().index()]
}

pub(crate) fn run() -> ExitCode {
    let mut checks = Checks::default();

    // One recorded session and two more for the scoreline. Recording every frame
    // of one match is thousands of frames, which the recorder is built to hold.
    let (good, mut sim, mut recorder) = play(Mode::Rollout, true);
    let (chaser, _, _) = play(Mode::Chaser, false);
    let (idle, _, _) = play(Mode::Idle, false);

    let font = recorder.font_texture();
    // The game's camera with this run's viewport stamped on, read back *after*
    // the ticks. The pair has to agree before any assertion about where a quad
    // is means anything.
    let camera = Camera {
        viewport: HEADLESS_VIEWPORT,
        ..*sim.world().resource::<Camera>()
    };
    let view = camera.visible_bounds();
    let dt = sim.world().resource::<Time>().fixed_dt.as_f32();

    // --- the order the systems run in ---------------------------------------
    //
    // The one instrument that can see it. The game decided that a collider is
    // stationary at its *post-move* position, and the whole of that decision is
    // the sequence of `add_system` calls in `register`: a tidy-up that moves one
    // of them reverses it silently, the ball starts passing through a paddle
    // closing on it, and every assertion about where things ended up still
    // passes.
    let order = sim.schedule_debug();
    for mover in ["drive_the_player", "drive_the_opponent"] {
        let (at_mover, at_ball) = (order.find(mover), order.find("move_the_ball"));
        checks.require(
            // Both have to be *found*: two renamed systems give two `None`s,
            // which compare equal, and the check then passes while seeing
            // nothing at all.
            matches!((at_mover, at_ball), (Some(a), Some(b)) if a < b),
            "a paddle is moved after the ball sweeps against it, not before",
            format!(
                "in the schedule, {mover} is at {at_mover:?} and move_the_ball at {at_ball:?}; \
                 the game treats a paddle as stationary at its post-move position, which is \
                 only true if it has already moved. None means the system was renamed and this \
                 check is looking at nothing.\n{order}"
            ),
        );
    }

    // --- the margin the game is built on, which no played session can see ----
    //
    // A fixed timestep only tests collisions at tick boundaries, so a ball that
    // travels further in one tick than its target is thick steps clean through
    // it. The cap is real and correct play never reaches it, so ask the numbers
    // directly — and against the `fixed_dt` the engine actually handed over
    // rather than the sixtieth of a second the constants were picked against.
    let per_tick = MAX_BALL_SPEED * dt;
    checks.require(
        greater(PADDLE_SIZE.x, per_tick),
        "the ball can cross a paddle inside one tick",
        format!(
            "at its top speed of {MAX_BALL_SPEED} units a second and a timestep of {dt:.5}s the \
             ball moves {per_tick:.3} per tick, against a paddle {:.2} thick; nothing sweeps \
             between ticks, so it would pass straight through",
            PADDLE_SIZE.x
        ),
    );
    // And what the session actually did, which is a different claim: the cap
    // above is about the constants, this is about the code that applies them.
    checks.require(
        greater(PADDLE_SIZE.x, good.longest_step) && greater(good.longest_step, 0.0),
        "the ball moved further in one tick than a paddle is thick",
        format!(
            "its longest single tick was {:.3} against a paddle {:.2} thick",
            good.longest_step, PADDLE_SIZE.x
        ),
    );

    // --- the swept contact test, asked its contract rather than played into ---
    //
    // Three calls and no match at all. A correct game never produces a tick of
    // travel eight units long, so replacing the sweep with a position test
    // passes an entire session — every assertion, the same 5-0, every frame.
    let stationary = Vec2::new(paddle_home(Side::Left), 0.0);
    let face = face_of(stationary, Side::Left);
    let across = crossing(
        Vec2::new(face.plane + 4.0, 0.0),
        Vec2::new(face.plane - 4.0, 0.0),
        face,
    );
    checks.require(
        across.is_some_and(|at| near(at, 0.5)),
        "a tick of travel straight across a paddle was not seen as a contact",
        format!(
            "eight units of travel through the plane at x={:.2}, crossing it exactly halfway, \
             reported {across:?}",
            face.plane
        ),
    );
    let past_the_end = crossing(
        Vec2::new(face.plane + 4.0, COURT.y),
        Vec2::new(face.plane - 4.0, COURT.y),
        face,
    );
    checks.require(
        past_the_end.is_none(),
        "a ball crossing the plane past the end of the paddle was called a contact",
        format!(
            "the same travel at y={:.1}, {:.1} beyond a reach of {:.2}, reported {past_the_end:?}",
            COURT.y,
            COURT.y - face.reach,
            face.reach
        ),
    );
    let leaving = crossing(
        Vec2::new(face.plane - 0.1, 0.0),
        Vec2::new(face.plane + 4.0, 0.0),
        face,
    );
    checks.require(
        leaving.is_none(),
        "a ball leaving through the face it came in by was called a contact",
        format!("travel from behind the plane back out through it reported {leaving:?}"),
    );

    // --- what the world did -------------------------------------------------
    checks.require(
        good.stage == Stage::Over { winner: Side::Left } && good.points[0] == MATCH_POINT,
        "the game cannot be won",
        format!(
            "a controller that rolls the game forward and plays the shot landing furthest from \
             the opponent finished {} - {} after {} ticks, in stage {:?}",
            good.points[0], good.points[1], good.ticks, good.stage
        ),
    );
    checks.require(
        idle.stage
            == Stage::Over {
                winner: Side::Right,
            }
            && idle.points[1] == MATCH_POINT,
        "the game cannot be lost",
        format!(
            "a player that never touches a key finished {} - {} after {} ticks, in stage {:?}",
            idle.points[0], idle.points[1], idle.ticks, idle.stage
        ),
    );
    // The one that says whether the game is worth playing. A rollout controller
    // winning proves the mechanics work and says nothing about that: a game
    // whose two sides both centre on the ball holds one rally for the whole
    // session, and only a paddle that simply chases the ball can see it.
    checks.require(
        matches!(chaser.stage, Stage::Over { .. })
            && chaser.points[0] >= 1
            && chaser.points[1] >= 1,
        "a player who simply chases the ball cannot play this game",
        format!(
            "chasing finished {} - {} after {} ticks in stage {:?}, with a longest rally of {} \
             touches; a match that neither side can score in is a game with a groove in it, not \
             a hard game",
            chaser.points[0], chaser.points[1], chaser.ticks, chaser.stage, chaser.tally.longest
        ),
    );
    // Stated where the game operates rather than at its most favourable point:
    // measured over the match as played, not derived at the top speed a rally
    // touches only at its very end.
    let reached = approached(&good, Side::Right);
    checks.require(
        reached > 0 && good.tally.returns[Side::Right.index()] * 2 >= reached,
        "the opponent is not really playing",
        format!(
            "{} balls reached it and it returned {}; an opponent that sends back fewer than half \
             is scenery, and one that sends back all of them cannot be scored against",
            reached,
            good.tally.returns[Side::Right.index()]
        ),
    );
    // A point has to be readable. The score changes on the tick the ball goes
    // past a paddle, and if the next serve leaves on that same tick nobody can
    // see what happened — the requirement is a person's eye, not the game's own
    // `SERVE_PAUSE`, which a check comparing against it would follow to zero.
    checks.require(
        good.shortest_pause >= READABLE_PAUSE && good.shortest_pause < u64::MAX,
        "a point goes by too fast to see",
        format!(
            "the shortest pause between a point and the serve after it was {} ticks ({:.2}s at \
             {dt:.5}s a tick); the score changes on that boundary and a person has to be able to \
             read it, which takes about a third of a second",
            good.shortest_pause,
            good.shortest_pause as f32 * dt,
        ),
    );
    checks.require(
        good.ticks >= SHORTEST_MATCH && good.ticks <= TICKS,
        "a won match is not the length of a game somebody plays",
        format!(
            "it took {} ticks, which is {:.1} seconds at {dt:.5}s a tick; the game is written to \
             be worth about half a minute",
            good.ticks,
            good.ticks as f32 * dt
        ),
    );
    // The controller's own contract, checked on the numbers it actually picked.
    // Read together these say which half of the program to open; no one of them
    // can. All three healthy and still 0-0 would mean the game is wrong.
    checks.require(
        good.report.approaches > 0 && good.report.met == good.report.approaches,
        "the rollout controller did not return every ball that came to it",
        format!(
            "met {} of {} approaches, so what the rest of this run measures is partly the \
             controller rather than the game",
            good.report.met, good.report.approaches
        ),
    );
    checks.require(
        greater(0.5, good.report.aim_error()) && good.report.aimed > 0,
        "the rollout controller's shots are not the shots it planned",
        format!(
            "{} shots landed {:.2} from where they were planned to, on a court {:.1} tall; a \
             controller aiming at noise reports a plausible wrong number about the game",
            good.report.aimed,
            good.report.aim_error(),
            view.size().y
        ),
    );
    checks.require(
        greater(good.report.threat(), 0.5) && good.report.planned > 0,
        "the rollout controller's shots are not threats",
        format!(
            "{} planned returns aimed to land {:.2} from the opponent's paddle centre, and its \
             reach is {:.2}; shots aimed inside that are shots aimed at the opponent",
            good.report.planned,
            good.report.threat(),
            PADDLE_SIZE.y * 0.5 + BALL_RADIUS
        ),
    );
    // The ball stayed in the court while it was live.
    checks.require(
        greater(COURT.y - BALL_RADIUS + 0.01, good.highest),
        "the ball went through a wall",
        format!(
            "its centre reached {:.3} from the centre line; the wall is at {:.2} and the ball's \
             radius is {BALL_RADIUS}, so {:.3} is as far as its centre may go",
            good.highest,
            COURT.y,
            COURT.y - BALL_RADIUS
        ),
    );

    // --- what was drawn -----------------------------------------------------
    let Some(live) = good.live.clone() else {
        fail(
            "no frame was recorded while the ball was live",
            "the recorded session draws one frame a tick and plays for thousands",
        );
    };
    checks.require(
        good.frames == good.ticks as usize,
        "one frame per tick was expected",
        format!("{} frames for {} ticks", good.frames, good.ticks),
    );

    // Both paddles, by bounds rather than by "something is drawn there". A
    // paddle-sized quad covers its own centre even when it is drawn a long way
    // out of position, so covering a point says a quad is nearby and only its
    // bounds say where it is.
    for side in [Side::Left, Side::Right] {
        let at = good.live_paddles[side.index()];
        let found = live.covering(at).into_iter().any(|quad| {
            let bounds = quad.bounds();
            near(bounds.size().x, PADDLE_SIZE.x)
                && near(bounds.size().y, PADDLE_SIZE.y)
                && near(bounds.center().x, at.x)
                && near(bounds.center().y, at.y)
        });
        checks.require(
            found,
            "no paddle-shaped quad was drawn where a paddle is",
            format!(
                "the {side:?} paddle is at ({:.2}, {:.2}) and is {:.2} by {:.2}; what covers that \
                 point is {}",
                at.x,
                at.y,
                PADDLE_SIZE.x,
                PADDLE_SIZE.y,
                sizes_covering(&live, at)
            ),
        );
    }

    // The ball is a circle, so "a quad the size of the thing is at the thing's
    // position" is the wrong question: sixteen wedges are submitted and nothing
    // the size of the disc is drawn anywhere. What is true is that all sixteen
    // share the centre as a corner and all sixteen fit inside its bounding box.
    let at = good.live_ball;
    let box_of_it = Rect::from_center_size(at, Vec2::splat(BALL_RADIUS * 2.0));
    let disc = find_bounds(live.covering(at).into_iter().filter(|quad| {
        // Spelled out rather than as `Rect::contains`, which is half-open and
        // would throw away the wedges reaching the far edge. Filtered, because a
        // centre-line dash under the ball covers the same point and is not the
        // ball.
        let drawn = quad.bounds();
        greater(drawn.min.x, box_of_it.min.x - 0.001)
            && greater(drawn.min.y, box_of_it.min.y - 0.001)
            && greater(box_of_it.max.x + 0.001, drawn.max.x)
            && greater(box_of_it.max.y + 0.001, drawn.max.y)
    }));
    let disc_size = disc.map(|rect| rect.size()).unwrap_or(Vec2::ZERO);
    checks.require(
        near(disc_size.x, BALL_RADIUS * 2.0) && near(disc_size.y, BALL_RADIUS * 2.0),
        "no ball-sized disc is drawn where the ball is",
        format!(
            "the world has it at ({:.2}, {:.2}); the wedges covering that point span \
             {:.3}x{:.3}, and a radius of {BALL_RADIUS} is {:.3} square. Everything covering it \
             is {}",
            at.x,
            at.y,
            disc_size.x,
            disc_size.y,
            BALL_RADIUS * 2.0,
            sizes_covering(&live, at)
        ),
    );
    // And a second one the constant cannot move with: a ball has to be small
    // against the paddle it gets past and big enough to see.
    checks.require(
        greater(PADDLE_SIZE.y * 0.5, disc_size.y) && greater(disc_size.y, view.size().y * 0.01),
        "the ball is not a readable size against the court",
        format!(
            "it is {:.2} across on a court {:.1} tall, against paddles {:.2} long",
            disc_size.y,
            view.size().y,
            PADDLE_SIZE.y
        ),
    );

    // --- the score, against the requirement rather than against its constant --
    //
    // A score drawn at SCORE_TOP and checked against SCORE_TOP moves with its
    // constant: put that constant in the middle of the court and the check
    // follows it down, passes, and leaves the score across the play. The
    // requirement names nothing the game owns.
    let glyphs: Vec<DrawnQuad> = live
        .quads()
        .into_iter()
        .filter(|quad| quad.texture == font)
        .collect();
    let top_third = view.min.y + view.size().y / 3.0;
    let score: Vec<DrawnQuad> = glyphs
        .iter()
        .copied()
        .filter(|quad| greater(top_third, quad.bounds().max.y))
        .collect();
    let (left_half, right_half): (Vec<DrawnQuad>, Vec<DrawnQuad>) = score
        .iter()
        .copied()
        .partition(|quad| greater(0.0, quad.bounds().center().x));
    let placed =
        find_bounds(left_half.iter().copied()).zip(find_bounds(right_half.iter().copied()));
    match placed {
        None => checks.require(
            false,
            "the score is not one number either side of the centre line, in the top third",
            format!(
                "of {} glyphs drawn, {} are in the top third of a view spanning {:.1} to {:.1} \
                 vertically; {} of those are left of the centre line and {} right of it",
                glyphs.len(),
                score.len(),
                view.min.y,
                view.max.y,
                left_half.len(),
                right_half.len()
            ),
        ),
        Some((left_box, right_box)) => {
            let gap_left = -left_box.max.x;
            let gap_right = right_box.min.x;
            checks.require(
                greater(gap_left, 0.0) && greater(gap_right, 0.0),
                "a score number is on the wrong side of the centre line",
                format!(
                    "the left number ends at x={:.2} and the right one starts at x={:.2}",
                    left_box.max.x, right_box.min.x
                ),
            );
            checks.require(
                greater(0.05, (gap_left - gap_right).abs()),
                "the two score numbers are not evenly set about the centre line",
                format!(
                    "the left one stands {gap_left:.3} from it and the right one {gap_right:.3}"
                ),
            );
            checks.require(
                greater(left_box.size().y, 0.0) && near(left_box.size().y, right_box.size().y),
                "the two score numbers are not the same size",
                format!(
                    "the left one is {:.3} tall and the right one {:.3}",
                    left_box.size().y,
                    right_box.size().y
                ),
            );
        }
    }
    // One quad per *character*, spaces included, and counted in `chars` rather
    // than `len`: `str::len` is bytes, which is right for pure ASCII and wrong
    // for exactly the input the printable check below exists for.
    let hint = hint_text(&good.live_tally);
    let expected: usize = hint.chars().filter(|glyph| *glyph != '\n').count()
        + good.live_points[0].to_string().chars().count()
        + good.live_points[1].to_string().chars().count();
    checks.require(
        glyphs.len() == expected,
        "the frame does not hold one quad per character the game draws",
        format!(
            "{} glyph quads for a hint of {} characters and a score of {} - {}, which is {expected}",
            glyphs.len(),
            hint.chars().count(),
            good.live_points[0],
            good.live_points[1],
        ),
    );

    // --- the bands, where the sort disagrees with the submission order -------
    //
    // A frame carries the order quads were drawn in, not the `Depth` that
    // produced it, so a band is only visible where it *changes* that order.
    // `register` submits the court *after* the play for exactly this reason:
    // where a game's submission order already agrees with its bands, no
    // assertion over drawn quads can see a layer at all.
    let quads = live.quads();
    let last_field = quads.iter().rposition(|quad| quad.tint == FIELD_LINE);
    let first_play = quads.iter().position(|quad| {
        quad.tint == BALL_COLOR || quad.tint == PLAYER_COLOR || quad.tint == OPPONENT_COLOR
    });
    checks.require(
        matches!((last_field, first_play), (Some(field), Some(play)) if field < play),
        "the court is drawn over the play instead of behind it",
        format!(
            "as indices into the draw order: the last court marking is at {last_field:?} and the \
             first paddle or ball quad at {first_play:?}. The game submits the court *after* the \
             play, so only FIELD sorting under PLAY can put it first — None means that band drew \
             nothing and the comparison is about nothing"
        ),
    );

    // --- nothing off screen, in every frame ---------------------------------
    //
    // The highest-value check a game of shapes and text can write. Over every
    // recorded frame rather than only the last: the hint line grows a digit
    // when a rally passes nine, and the frame that first ran off the edge would
    // otherwise be one nothing looked at.
    let mut strays: Vec<(usize, Rect)> = Vec::new();
    for (index, frame) in recorder.frames().iter().enumerate() {
        for quad in frame.quads() {
            let bounds = quad.bounds();
            // `contains_rect` is closed on all four sides, because a quad flush
            // against the camera's edge is on screen. `Rect::contains` takes a
            // point and is half-open, which is a different question.
            if !view.contains_rect(bounds) {
                strays.push((index, bounds));
            }
        }
    }
    checks.require(
        strays.is_empty(),
        "something was drawn outside what the camera shows",
        format!(
            "{} quads across {} frames fall outside {view:?}; the first is in frame {:?} — text \
             centred by TextStyle::width_of is the usual culprit",
            strays.len(),
            recorder.frames().len(),
            strays.first(),
        ),
    );

    // --- the background, which leaves no quad behind ------------------------
    let cleared = live.plan.clear_color;
    checks.require(
        cleared == COURT_COLOR,
        "the court was cleared to a colour the game does not name",
        format!("the frame cleared to {cleared:?}; the game's constant is {COURT_COLOR:?}"),
    );
    // And the requirement, which that constant cannot move: a white ball has to
    // read against it. Written the first way alone, the clear colour is exactly
    // the fault a mutation walks straight through.
    let brightness = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        greater(0.25, brightness) && greater(cleared.a, 0.99),
        "the court is not dark enough to see a white ball on",
        format!(
            "its brightest channel is {brightness:.3} at alpha {:.2}",
            cleared.a
        ),
    );

    // --- the strings themselves ---------------------------------------------
    //
    // No assertion over drawn quads can see a wrong character: the font draws an
    // unknown one as a box at exactly a letter's advance, so a stray em dash or
    // curly quote passes the glyph count, the centring and the bounds check
    // alike. The string is the only instrument, which is why the game hands its
    // hint and its banner back as strings rather than formatting them inside the
    // draw system where nothing could reach them.
    let banners = [
        banner_lines(Side::Left, [MATCH_POINT, 2]),
        banner_lines(Side::Right, [1, MATCH_POINT]),
    ];
    let mut drawn_strings: Vec<(String, String)> = vec![("the hint".to_owned(), hint.clone())];
    for (side, lines) in [Side::Left, Side::Right].iter().zip(banners.iter()) {
        for (index, line) in lines.iter().enumerate() {
            drawn_strings.push((
                format!("the {side:?} banner, line {}", index + 1),
                line.clone(),
            ));
        }
    }
    for (name, text) in &drawn_strings {
        let stray = text
            .chars()
            .find(|glyph| *glyph != '\n' && !(' '..='~').contains(glyph));
        checks.require(
            stray.is_none(),
            "a string the game draws has a character the font cannot draw",
            format!(
                "{name} is {text:?} and contains {stray:?}, which draws as a box at exactly a \
                 letter's width — no assertion over what was drawn can tell the difference"
            ),
        );
    }

    // --- the screens this run never reached ---------------------------------
    //
    // A controller good enough to finish the game is a controller that never
    // loses it, so the losing banner is drawn zero times in a session that ends
    // 5-0 and the longest string in the game is the one nothing measured. Build
    // them by hand, and set *every* piece of state the frame depends on: this
    // recipe is additive while the frames are corrective, and whatever the run
    // left behind is still set.
    let mut staged: Vec<(String, FrameRecord)> = Vec::new();
    for (winner, points) in [
        (Side::Left, [MATCH_POINT, 2]),
        (Side::Right, [1, MATCH_POINT]),
    ] {
        sim.world_mut().insert_resource(Round {
            points,
            stage: Stage::Over { winner },
        });
        staged.push((format!("{winner:?} wins"), recorder.draw(&mut sim)));
    }
    for ((name, frame), points) in staged.iter().zip([[MATCH_POINT, 2], [1, MATCH_POINT]]) {
        let outside: Vec<Rect> = frame
            .quads()
            .iter()
            .map(|quad| quad.bounds())
            .filter(|bounds| !view.contains_rect(*bounds))
            .collect();
        checks.require(
            outside.is_empty(),
            "an end screen draws outside what the camera shows",
            format!(
                "on the '{name}' screen, {} quads fall outside {view:?}; the first is {:?}",
                outside.len(),
                outside.first()
            ),
        );
        // Two lines, each centred by its own width. `width_of` measures the
        // widest line only, so one call for the block would hang the shorter
        // line off to the left — on screen, right size, visibly crooked, and
        // indistinguishable from a layout that meant it by any check over
        // drawn quads except this one.
        // Each banner line found by the number of glyphs it drew rather than by
        // where it sits, so the check survives the layout moving — an earlier
        // spelling picked the bands out by height and started failing the
        // moment the banner was nudged up the court, which is a check about
        // itself rather than about the game.
        //
        // Three bands: the score, still drawn on an end screen, and the two
        // lines of the banner. Counting only "at least two" would pass for a
        // block drawn in one `ctx.text` call, which is the fault this exists to
        // catch: `width_of` measures the widest line only, so one call for the
        // block centres the long line and hangs the short one off to the left,
        // on screen, at the right size, and visibly crooked.
        let bands = banner_glyph_lines(frame, font);
        checks.require(
            bands.len() == 3,
            "the end screen does not draw a score and two banner lines",
            format!(
                "the '{name}' screen drew {} bands of glyphs, at {:?}",
                bands.len(),
                bands
                    .iter()
                    .map(|(row, count)| (row.min.y, *count))
                    .collect::<Vec<_>>()
            ),
        );
        let winner = if points[0] > points[1] {
            Side::Left
        } else {
            Side::Right
        };
        for line in banner_lines(winner, points) {
            let glyphs = line.chars().count();
            let matching: Vec<Rect> = bands
                .iter()
                .filter(|(_, count)| *count == glyphs)
                .map(|(row, _)| *row)
                .collect();
            checks.require(
                matching.len() == 1
                    && matching
                        .iter()
                        .all(|row| greater(0.05, row.center().x.abs())),
                "a line of the end screen was not drawn centred, on its own",
                format!(
                    "on the '{name}' screen, {line:?} is {glyphs} characters and {} band(s) of \
                     that many glyphs were drawn, centred at {:?}",
                    matching.len(),
                    matching
                        .iter()
                        .map(|row| row.center().x)
                        .collect::<Vec<_>>()
                ),
            );
        }
    }

    // --- the band boundary a played session never produces ------------------
    //
    // FIELD under PLAY is checked above by index, because the game submits them
    // the other way round. PLAY under UI is not: the hint line is submitted last
    // *and* sorts last, so no comparison over a played frame can see that band
    // at all. Arrange the disagreement instead — park the ball under the hint
    // and ask what a player looking at that point actually sees.
    let hint_box = find_bounds(
        live.quads()
            .into_iter()
            .filter(|quad| quad.texture == font && greater(quad.bounds().min.y, 0.0)),
    );
    match hint_box {
        None => checks.require(
            false,
            "the hint line was not drawn",
            "no glyph was drawn below the centre line in a live frame".to_owned(),
        ),
        Some(hint_box) => {
            let under = hint_box.center();
            // Corrective, not additive: the staged end screen above is still
            // set, and a frame drawn now would carry the banner rather than the
            // hint. Put the match back into play before asking.
            sim.world_mut().insert_resource(Round {
                points: [1, 1],
                stage: Stage::Rally,
            });
            let ball = sim
                .world()
                .query::<(&Transform, &Ball)>()
                .map(|(entity, _, _)| entity)
                .next();
            match ball {
                None => fail("the ball is gone", "Startup spawns exactly one"),
                Some(ball) => sim.world_mut().component_mut::<Transform>(ball).pos = under,
            }
            let overlap = recorder.draw(&mut sim);
            let front = overlap.covering(under).into_iter().next();
            checks.require(
                front.is_some_and(|quad| quad.texture == font),
                "the hint line is not drawn over the ball",
                format!(
                    "with the ball parked at ({:.2}, {:.2}), under the middle of the hint, the \
                     front-most quad there is {:?} rather than a glyph; {} quads cover the point",
                    under.x,
                    under.y,
                    front.map(|quad| quad.tint),
                    overlap.covering(under).len()
                ),
            );
        }
    }

    // --- a picture ----------------------------------------------------------
    let captured = crate::capture::capture_a_frame(&mut checks, &live, font, "pong");
    let end_screen = staged
        .first()
        .map(|(_, frame)| crate::capture::capture_a_frame(&mut checks, frame, font, "pong-over"))
        .unwrap_or_else(|| "skipped, no end screen was staged".to_owned());

    let failures = checks.failures();
    let verdict = checks.verdict();

    println!(
        "verified pong over {} ticks of play and {} recorded frames, {failures} problems",
        good.ticks + chaser.ticks + idle.ticks,
        recorder.frames().len(),
    );
    println!(
        "  rollout: {} - {} in {} ticks ({:.1}s), longest rally {}, top speed {:.1}",
        good.points[0],
        good.points[1],
        good.ticks,
        good.ticks as f32 * dt,
        good.tally.longest,
        good.tally.fastest,
    );
    for line in good.report.lines("rollout") {
        println!("{line}");
    }
    println!(
        "  chaser: {} - {} in {} ticks, longest rally {} — the one that says it is playable",
        chaser.points[0], chaser.points[1], chaser.ticks, chaser.tally.longest,
    );
    println!(
        "  chaser: met {} of {} approaches",
        chaser.report.met, chaser.report.approaches
    );
    println!(
        "  idle: {} - {} in {} ticks — the game can be lost",
        idle.points[0], idle.points[1], idle.ticks,
    );
    println!(
        "  opponent: returned {} of the {} balls that reached it",
        good.tally.returns[Side::Right.index()],
        approached(&good, Side::Right),
    );
    println!(
        "  ball: longest tick {:.3} against a paddle {:.2} thick, shortest pause between points \
         {} ticks",
        good.longest_step, PADDLE_SIZE.x, good.shortest_pause,
    );
    println!("  capture: {captured}");
    println!("  capture: {end_screen}");
    print!("{}", live.transcript());
    verdict
}

/// The bounding box of each horizontal band of glyphs in `frame`.
///
/// One band per `ctx.text` call that drew a line, found by grouping glyph quads
/// on the row they sit in — which is how a check asks whether two lines of a
/// banner were each centred, rather than whether the block was.
fn banner_glyph_lines(frame: &FrameRecord, font: BackendTextureId) -> Vec<(Rect, usize)> {
    let mut rows: Vec<(Rect, usize)> = Vec::new();
    for quad in frame.quads() {
        if quad.texture != font {
            continue;
        }
        let bounds = quad.bounds();
        // Grouped on the top of the glyph cell, which `ctx.text` puts at exactly
        // the `at` it was given: two lines of different sizes are two rows even
        // where they nearly touch, and one line is never split.
        let row = rows
            .iter_mut()
            .find(|(row, _)| greater(0.001, (row.min.y - bounds.min.y).abs()));
        match row {
            Some((row, count)) => {
                row.min = row.min.min(bounds.min);
                row.max = row.max.max(bounds.max);
                *count += 1;
            }
            None => rows.push((bounds, 1)),
        }
    }
    rows
}
