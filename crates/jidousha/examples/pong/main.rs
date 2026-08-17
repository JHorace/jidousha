//! Pong, written against the engine the way a game is meant to be.
//!
//! Two paddles, a ball, a score. W and S move the left paddle; the right one is
//! played by the machine. First to five wins, then Space starts another match.
//!
//! Everything here is a rectangle, a circle, a line or a line of text, so the
//! game names no asset at all and there is no loading story to tell.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//!
//! Four files, because they are four things to read: the rules of the game are
//! here, what it looks like is in `draw.rs`, a played session is in
//! `session.rs`, and what that session is judged against is in `verify.rs`.

use std::process::ExitCode;

use jidousha::math::sin_cos;
use jidousha::prelude::*;

mod draw;
mod session;
mod verify;

/// How many world units the camera spans vertically.
///
/// At 16:9 that is 35.55 units across, which is what leaves a margin around the
/// field below rather than cropping it.
const VIEW_HEIGHT: f32 = 20.0;

/// Half the playfield, in world units: the goal lines are at `±x`, the walls at
/// `±y`. Stated once because the verify run asserts the ball stays inside it.
const FIELD: Vec2 = Vec2::new(16.0, 8.0);

/// How big a paddle is drawn and how big it is for collision — one number, so
/// the thing you see is the thing you hit.
const PADDLE_SIZE: Vec2 = Vec2::new(0.8, 3.4);

/// How far from the centre a paddle sits, in world units.
const PADDLE_X: f32 = 14.8;

/// How far a paddle's centre may travel from the middle of the field.
const PADDLE_LIMIT: f32 = FIELD.y - PADDLE_SIZE.y * 0.5;

/// How fast the player's paddle moves, in world units per second.
const PLAYER_SPEED: f32 = 27.0;

/// How fast the opponent's paddle moves. Slower than the player's on purpose:
/// this is the whole difficulty knob, and a game you cannot win is not fun.
const OPPONENT_SPEED: f32 = 12.0;

/// The opponent ignores the ball until it has crossed this X, which is what
/// gives a human time to aim at the corner it has left open.
const OPPONENT_WAKES_AT: f32 = 8.0;

/// How close counts as lined up, so the opponent does not jitter on the spot.
const OPPONENT_DEADZONE: f32 = 0.35;

/// How big the ball is, in world units.
const BALL_RADIUS: f32 = 0.35;

/// How fast the ball leaves a serve, in world units per second.
///
/// INVARIANT: one tick of travel at `MAX_SPEED` has to be shorter than half a
/// paddle plus the ball's radius, or a fast ball steps straight through a
/// paddle without ever overlapping it. The verify run checks this against the
/// timestep the engine actually hands the game, rather than against 1/60
/// written down here.
const SERVE_SPEED: f32 = 18.0;

/// What each paddle touch multiplies the ball's speed by, and where that stops.
const SPEEDUP: f32 = 1.07;
const MAX_SPEED: f32 = 30.0;

/// The steepest angle a paddle's edge can send the ball off at, measured from
/// the horizontal. This is the whole of Pong's control scheme.
const MAX_BOUNCE: f32 = 52.0;

/// The shallowest angle a return may leave at.
///
/// Without it the game has a dead end: two players who both centre the ball
/// return it perfectly horizontally, and the rally runs in a flat groove that
/// neither can ever lose. A floor on the angle means every exchange moves the
/// ball a little, so a rally always goes somewhere.
const MIN_BOUNCE: f32 = 9.0;

/// The widest a serve may leave the centre spot at.
const MAX_SERVE_ANGLE: f32 = 28.0;

/// How long the ball sits at the centre before a serve, in ticks. The engine's
/// timestep is 60 ticks a second unless a game says otherwise, so this is
/// three quarters of a second.
const SERVE_PAUSE: u32 = 36;

/// How many points wins a match.
const WINNING_SCORE: u32 = 5;

/// How tall the score is, and where its top edge sits — above the field, so it
/// never overlaps play. Stated here because the check looks for a glyph at the
/// middle of it, and a check carrying its own copy of the number would keep
/// passing after the score moved.
const SCORE_SIZE: f32 = 1.5;
const SCORE_TOP: f32 = -FIELD.y - 1.6;

/// Draw bands, named once so `z: 3.0` never appears at a call site.
pub(crate) mod layers {
    /// The field and its markings.
    pub const FIELD: i16 = -1;
    /// Paddles and ball.
    pub const PLAY: i16 = 0;
    /// Score, banners, the hint along the bottom.
    pub const UI: i16 = 2;
}

/// `a > b`, and false when either is NaN.
///
/// Spelled out rather than written as the negation of a `<=`, because the
/// negation of a float comparison silently means something else: a NaN that
/// crept into a position would satisfy every plain `<=` in this file.
fn greater(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(core::cmp::Ordering::Greater))
}

/// Which end of the field something belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

impl Side {
    /// The direction, along X, that this side's paddle hits the ball.
    fn outward(self) -> f32 {
        match self {
            Side::Left => 1.0,
            Side::Right => -1.0,
        }
    }

    /// The other one.
    fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// A paddle, and who moves it.
#[derive(Clone, Copy)]
struct Paddle {
    side: Side,
    /// Whether a person drives this one or the machine does.
    played_by: Control,
}
impl Component for Paddle {}

/// Who is moving a paddle.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    Keyboard,
    Machine,
}

/// The ball. Carries no data: where it is lives in `Transform`, how fast it is
/// going lives in `Velocity`.
#[derive(Clone, Copy)]
struct Ball;
impl Component for Ball {}

/// World units per second.
#[derive(Clone, Copy)]
struct Velocity(Vec2);
impl Component for Velocity {}

/// What the match is doing right now.
#[derive(Clone, Copy, PartialEq)]
enum Round {
    /// The ball is on the centre spot, about to go towards `toward`.
    Serving { ticks_left: u32, toward: Side },
    /// The ball is in play.
    Rallying,
    /// Somebody reached `WINNING_SCORE`; Space starts another match.
    Over { winner: Side },
}

/// The score, the state of the match, and the numbers a check wants to read.
///
/// One resource rather than three, because every one of them changes on the
/// tick a point is scored and splitting them only spreads that out.
struct Scoreboard {
    left: u32,
    right: u32,
    round: Round,
    /// How many times each paddle has returned the ball, all match.
    left_hits: u32,
    right_hits: u32,
    /// The fastest the ball has gone, in world units per second.
    top_speed: f32,
    /// Paddle touches in the rally that is running now, and the longest so far.
    rally: u32,
    longest_rally: u32,
}
impl Resource for Scoreboard {}

impl Scoreboard {
    /// A fresh match, with the first serve going towards `toward`.
    fn new(toward: Side) -> Self {
        Scoreboard {
            left: 0,
            right: 0,
            round: Round::Serving {
                ticks_left: SERVE_PAUSE,
                toward,
            },
            left_hits: 0,
            right_hits: 0,
            top_speed: 0.0,
            rally: 0,
            longest_rally: 0,
        }
    }

    /// The score as it is drawn, and as the check looks for it.
    fn text(&self) -> String {
        format!("{} : {}", self.left, self.right)
    }
}

/// The game's configuration, shared by the window and the verify run so that
/// what is checked is what a person plays.
fn config() -> GameConfig {
    GameConfig {
        title: "jidousha — pong",
        ..GameConfig::default()
    }
}

/// Every system this game has, in one place, in the order they run.
fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, run_the_clock);
    app.add_system(Update, steer_the_player);
    app.add_system(Update, steer_the_opponent);
    app.add_system(Update, advance_the_ball);
    app.add_system(Draw, draw::draw_the_field);
    app.add_system(Draw, draw::draw_the_play);
    app.add_system(Draw, draw::draw_the_readout);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        verify::run();
        return ExitCode::SUCCESS;
    }
    println!("W and S move the left paddle. first to {WINNING_SCORE} wins; Space plays again");
    println!("close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        // Print it, do not return it: `RunError`'s `Display` is the engine's
        // four-part message, and `Debug` is a struct dump.
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        height: VIEW_HEIGHT,
        clear_color: Color::rgb(0.04, 0.06, 0.09),
        ..Camera::default()
    });

    let first = coin(world);
    world.insert_resource(Scoreboard::new(first));

    for (side, played_by) in [
        (Side::Left, Control::Keyboard),
        (Side::Right, Control::Machine),
    ] {
        let paddle = world.spawn();
        world.insert(
            paddle,
            Transform::at(Vec2::new(PADDLE_X * -side.outward(), 0.0)),
        );
        world.insert(paddle, Paddle { side, played_by });
    }

    let ball = world.spawn();
    world.insert(ball, Transform::at(Vec2::ZERO));
    world.insert(ball, Velocity(Vec2::ZERO));
    world.insert(ball, Ball);
}

/// The serve countdown, and the key that starts a new match.
fn run_the_clock(world: &mut World) {
    let restart = world
        .find_resource::<Input>()
        .is_some_and(|input| input.just_pressed(Key::Space));

    match world.resource::<Scoreboard>().round {
        Round::Serving { ticks_left, toward } if ticks_left > 0 => {
            world.resource_mut::<Scoreboard>().round = Round::Serving {
                ticks_left: ticks_left - 1,
                toward,
            };
        }
        Round::Serving { toward, .. } => {
            let angle = Radians::from_degrees(spread(world, MAX_SERVE_ANGLE));
            let (sine, cosine) = sin_cos(angle);
            let heading = Vec2::new(cosine * toward.outward(), sine) * SERVE_SPEED;
            let ball = world
                .query::<(&Transform, With<Ball>)>()
                .map(|(entity, _, _)| entity)
                .next();
            if let Some(ball) = ball {
                world.component_mut::<Transform>(ball).pos = Vec2::ZERO;
                world.component_mut::<Velocity>(ball).0 = heading;
            }
            let board = world.resource_mut::<Scoreboard>();
            board.round = Round::Rallying;
            board.rally = 0;
            board.top_speed = board.top_speed.max(SERVE_SPEED);
        }
        Round::Over { .. } if restart => {
            let toward = coin(world);
            world.insert_resource(Scoreboard::new(toward));
            let ball = world
                .query::<(&Transform, With<Ball>)>()
                .map(|(entity, _, _)| entity)
                .next();
            if let Some(ball) = ball {
                world.component_mut::<Transform>(ball).pos = Vec2::ZERO;
                world.component_mut::<Velocity>(ball).0 = Vec2::ZERO;
            }
        }
        Round::Over { .. } | Round::Rallying => {}
    }
}

/// Which way the next match's first serve goes, from the run's seeded
/// generator — so the same seed opens the same way every time.
fn coin(world: &mut World) -> Side {
    if world.resource_mut::<Rng>().below(2) == 0 {
        Side::Left
    } else {
        Side::Right
    }
}

/// A number in `-spread..spread`, from the run's seeded generator.
fn spread(world: &mut World, spread: f32) -> f32 {
    (world.resource_mut::<Rng>().next_f32() - 0.5) * 2.0 * spread
}

/// W and S, on whichever paddle a person is playing.
fn steer_the_player(world: &mut World) {
    let step = match world.find_resource::<Input>() {
        // The first tick of a windowed run happens before any input is set.
        None => return,
        Some(input) => f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    };
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.played_by == Control::Keyboard {
            transform.pos.y =
                (transform.pos.y + step * PLAYER_SPEED * dt).clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
        }
    }
}

/// The machine's paddle: chase the ball's Y, but only once the ball is coming,
/// and never faster than `OPPONENT_SPEED`.
///
/// Read first, write second — the ball has to be looked at before the paddles
/// can be moved, and a query that borrows the world mutably holds it.
fn steer_the_opponent(world: &mut World) {
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    let rallying = world.resource::<Scoreboard>().round == Round::Rallying;
    let ball = world
        .query::<(&Transform, With<Ball>)>()
        .map(|(_, transform, _)| transform.pos)
        .next();
    let Some(ball) = ball else { return };

    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.played_by != Control::Machine {
            continue;
        }
        // Between points, and while the ball is still on the far side, it
        // drifts back to the middle instead of camping where it last was.
        let target = if rallying && ball.x > OPPONENT_WAKES_AT {
            ball.y
        } else {
            0.0
        };
        let gap = target - transform.pos.y;
        if gap.abs() <= OPPONENT_DEADZONE {
            continue;
        }
        let reach = OPPONENT_SPEED * dt;
        let step = gap.clamp(-reach, reach);
        transform.pos.y = (transform.pos.y + step).clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
    }
}

/// The whole of the ball's life: move, bounce off the walls, bounce off a
/// paddle, or go past one and become a point.
fn advance_the_ball(world: &mut World) {
    if world.resource::<Scoreboard>().round != Round::Rallying {
        return;
    }
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    let paddles: Vec<(Side, Vec2)> = world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (paddle.side, transform.pos))
        .collect();
    let ball = world
        .query::<(&Transform, &Velocity, With<Ball>)>()
        .map(|(entity, transform, velocity, _)| (entity, transform.pos, velocity.0))
        .next();
    let Some((entity, mut pos, mut velocity)) = ball else {
        return;
    };

    pos += velocity * dt;

    // The walls. Put the ball back where it should have ended up rather than
    // just flipping the sign, so a fast ball cannot settle inside the wall.
    let wall = FIELD.y - BALL_RADIUS;
    if pos.y < -wall {
        pos.y = -wall - (pos.y + wall);
        velocity.y = velocity.y.abs();
    } else if pos.y > wall {
        pos.y = wall - (pos.y - wall);
        velocity.y = -velocity.y.abs();
    }

    // The paddles. A hit only counts when the ball is heading into the paddle,
    // which is what stops a ball that clipped an edge from rattling inside it.
    let mut hit = None;
    let ball_box = Rect::from_center_size(pos, Vec2::splat(BALL_RADIUS * 2.0));
    for (side, at) in &paddles {
        let outward = side.outward();
        if velocity.x * outward >= 0.0 {
            continue;
        }
        if !ball_box.overlaps(Rect::from_center_size(*at, PADDLE_SIZE)) {
            continue;
        }
        // Where on the paddle it landed, -1 at the top edge and 1 at the
        // bottom. This is the aiming: the edges send the ball off steeply.
        let reach = PADDLE_SIZE.y * 0.5 + BALL_RADIUS;
        let offset = ((pos.y - at.y) / reach).clamp(-1.0, 1.0);
        let speed = (velocity.length() * SPEEDUP).min(MAX_SPEED);
        // Which way a return that is too flat to be interesting gets nudged:
        // the side of the paddle it landed on, or the way it was already
        // drifting when it landed exactly in the middle.
        let toward = if offset > 0.0 || (offset == 0.0 && velocity.y >= 0.0) {
            1.0
        } else {
            -1.0
        };
        let angle = offset * MAX_BOUNCE;
        let angle = if greater(MIN_BOUNCE, angle.abs()) {
            toward * MIN_BOUNCE
        } else {
            angle
        };
        let (sine, cosine) = sin_cos(Radians::from_degrees(angle));
        velocity = Vec2::new(cosine * outward, sine) * speed;
        pos.x = at.x + outward * (PADDLE_SIZE.x * 0.5 + BALL_RADIUS);
        hit = Some(*side);
        break;
    }

    world.component_mut::<Transform>(entity).pos = pos;
    world.component_mut::<Velocity>(entity).0 = velocity;

    let board = world.resource_mut::<Scoreboard>();
    board.top_speed = board.top_speed.max(velocity.length());
    if let Some(side) = hit {
        match side {
            Side::Left => board.left_hits += 1,
            Side::Right => board.right_hits += 1,
        }
        board.rally += 1;
        board.longest_rally = board.longest_rally.max(board.rally);
    }

    // Past a goal line: the other side scores, and serves towards whoever was
    // just scored on.
    let scorer = if pos.x < -FIELD.x {
        Some(Side::Right)
    } else if pos.x > FIELD.x {
        Some(Side::Left)
    } else {
        None
    };
    let Some(scorer) = scorer else { return };

    let board = world.resource_mut::<Scoreboard>();
    match scorer {
        Side::Left => board.left += 1,
        Side::Right => board.right += 1,
    }
    let points = match scorer {
        Side::Left => board.left,
        Side::Right => board.right,
    };
    board.round = if points >= WINNING_SCORE {
        Round::Over { winner: scorer }
    } else {
        Round::Serving {
            ticks_left: SERVE_PAUSE,
            toward: scorer.other(),
        }
    };
    world.component_mut::<Transform>(entity).pos = Vec2::ZERO;
    world.component_mut::<Velocity>(entity).0 = Vec2::ZERO;
}
