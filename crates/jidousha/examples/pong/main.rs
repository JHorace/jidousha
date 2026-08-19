//! Pong, written against the engine's public API and nothing else.
//!
//! W and S move the left paddle. The right one plays itself. First to five
//! wins; after that, Space starts a fresh match.
//!
//! Every shape here is drawn by the engine — two rectangles, a disc, some lines
//! and some text — so there is no `Assets` resource anywhere in this game and
//! nothing to wait for before the first frame.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//!
//! `verify.rs` beside this file is the second half: the same systems and the
//! same config, driven by a controller instead of a person, asserting on what
//! the world did and on what was drawn, with no window anywhere.

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod controller;
mod draw;
mod field;
mod verify;

pub(crate) use field::*;

/// Which end of the table something belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Left,
    Right,
}

impl Side {
    /// Where this side's paddle sits.
    pub(crate) fn paddle_x(self) -> f32 {
        match self {
            Side::Left => -PADDLE_X,
            Side::Right => PADDLE_X,
        }
    }

    /// `+1.0` for the left side, `-1.0` for the right: the direction a ball
    /// travels when it leaves this side's paddle.
    pub(crate) fn outward(self) -> f32 {
        match self {
            Side::Left => 1.0,
            Side::Right => -1.0,
        }
    }

    pub(crate) fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }
}

/// Who drives a paddle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    Keys,
    Machine,
}

/// A paddle. The enum inside the component is why no `With<Player>` filter
/// appears anywhere below: one query over `&Paddle` reaches both of them, and
/// the tuple stays short.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Paddle {
    pub(crate) side: Side,
    pub(crate) control: Control,
    /// World units per second.
    pub(crate) speed: f32,
    /// Where a machine paddle last saw the ball, refreshed every
    /// `MACHINE_REACTION` ticks. A keyboard paddle ignores it.
    pub(crate) aim: f32,
}
impl Component for Paddle {}

/// The ball. Its speed lives in the length of its velocity.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ball {
    pub(crate) velocity: Vec2,
}
impl Component for Ball {}

/// What the match is doing right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Play {
    /// The ball is parked at the middle, about to be sent at `to`.
    Serving { to: Side, ticks_left: u32 },
    /// The ball is live.
    Rally,
    /// Somebody reached `WINNING_SCORE`.
    Over { winner: Side },
}

/// The score, and everything the match has been up to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Scoreboard {
    pub(crate) left: u32,
    pub(crate) right: u32,
    pub(crate) play: Play,
    /// Paddle touches since the last serve.
    pub(crate) rally: u32,
    /// The longest rally of the match, in touches.
    pub(crate) longest_rally: u32,
    /// The fastest the ball has gone this match, in world units per second.
    pub(crate) top_speed: f32,
}
impl Resource for Scoreboard {}

impl Scoreboard {
    /// Nil-nil, with the first serve going at the right-hand paddle.
    pub(crate) fn new() -> Self {
        Scoreboard {
            left: 0,
            right: 0,
            play: Play::Serving {
                to: Side::Right,
                ticks_left: SERVE_PAUSE,
            },
            rally: 0,
            longest_rally: 0,
            top_speed: 0.0,
        }
    }

    pub(crate) fn points(&self, side: Side) -> u32 {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
        }
    }
}

/// The configuration the window and the verification share, so what is checked
/// is what a person plays.
pub(crate) fn config() -> GameConfig {
    GameConfig {
        title: "jidousha - pong",
        ..GameConfig::default()
    }
}

/// Every system this game has, in one place, in run order.
///
/// Named rather than written inline at each call site so the verification runs
/// the *same* game the window does.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, set_the_table);
    app.add_system(Update, drive_the_paddles);
    app.add_system(Update, start_a_fresh_match);
    app.add_system(Update, serve);
    app.add_system(Update, move_the_ball);
    app.add_system(Update, bounce_off_the_walls);
    app.add_system(Update, bounce_off_the_paddles);
    app.add_system(Update, score_a_point);
    app.add_system(Draw, draw::the_table);
    app.add_system(Draw, draw::the_play);
    app.add_system(Draw, draw::the_readout);
}

fn main() -> ExitCode {
    // `--verify` is the headless half: no window, a controller instead of a
    // person, and assertions instead of somebody watching.
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("{HINT}");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        // Display, not Debug. `RunError`'s `Display` is the engine's four-part
        // message; returning it from `main` would print a struct dump instead.
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

// --- setup ---------------------------------------------------------------

fn set_the_table(world: &mut World) {
    world.insert_resource(Camera {
        clear_color: Color::rgb(0.04, 0.06, 0.09),
        height: VIEW_HEIGHT,
        ..Camera::default()
    });
    world.insert_resource(Scoreboard::new());

    for (side, control, speed) in [
        (Side::Left, Control::Keys, PLAYER_SPEED),
        (Side::Right, Control::Machine, MACHINE_SPEED),
    ] {
        let paddle = world.spawn();
        world.insert(paddle, Transform::at(Vec2::new(side.paddle_x(), 0.0)));
        world.insert(
            paddle,
            Paddle {
                side,
                control,
                speed,
                aim: 0.0,
            },
        );
    }

    let ball = world.spawn();
    world.insert(ball, Transform::at(Vec2::ZERO));
    world.insert(
        ball,
        Ball {
            velocity: Vec2::ZERO,
        },
    );
}

// --- simulation ----------------------------------------------------------

/// Move both paddles: the left one from the keyboard, the right one from the
/// ball.
fn drive_the_paddles(world: &mut World) {
    // `Input` is absent on the first tick of a windowed run, and absent for the
    // whole of a headless one unless a driver puts it there. Hence
    // `find_resource`: without input the paddles simply do not move.
    let keys = match world.find_resource::<Input>() {
        None => 0.0,
        Some(input) => f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    };
    let step = world.resource::<Time>().fixed_dt.as_f32();

    // Pass one, read-only. The machine paddle has to look at another entity
    // while moving itself, which is the one shape that cannot be a single
    // `query_mut` — the world is exclusively borrowed for as long as it runs.
    let ball = ball_state(world);
    let live = matches!(world.resource::<Scoreboard>().play, Play::Rally);
    // The machine's eyes open on these ticks and no others. Off the tick count
    // rather than off a wall clock, so the opponent plays the same game on a
    // slow machine as on a fast one.
    let looking = world
        .resource::<Time>()
        .tick
        .is_multiple_of(MACHINE_REACTION);

    // Pass two, write.
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &mut Paddle)>() {
        let direction = match paddle.control {
            Control::Keys => keys,
            Control::Machine => {
                if looking {
                    paddle.aim = machine_target(paddle.side, live, ball);
                }
                let gap = paddle.aim - transform.pos.y;
                if gap.abs() < MACHINE_DEAD_BAND {
                    0.0
                } else {
                    gap.signum()
                }
            }
        };
        transform.pos.y = (transform.pos.y + direction * paddle.speed * step)
            .clamp(-PADDLE_TRAVEL, PADDLE_TRAVEL);
    }
}

/// Where the ball is and how fast, or `None` before `Startup` has spawned it.
pub(crate) fn ball_state(world: &World) -> Option<(Vec2, Vec2)> {
    world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| (transform.pos, ball.velocity))
        .next()
}

/// Where the machine paddle wants to be, sampled the moment it looks.
///
/// It chases the ball's y *as it is now*, not where it will end up: a paddle
/// that solved for the crossing, walls and all, would never be beaten by
/// anything. Between looks it drives at the last thing it saw, which is what
/// makes a fast steep ball beat it — the ball's vertical speed can exceed the
/// paddle's, so twelve ticks of lag is more than a paddle's length of error.
pub(crate) fn machine_target(side: Side, live: bool, ball: Option<(Vec2, Vec2)>) -> f32 {
    let Some((pos, velocity)) = ball else {
        return 0.0;
    };
    // Coming this way? If not, wait in the middle for the next one.
    if !live || velocity.x * side.outward() >= 0.0 {
        return 0.0;
    }
    pos.y
}

/// Space clears a finished match back to nil-nil.
///
/// A game does not close itself — there is no `App::quit` in v1 — so the end of
/// a match is a state to leave rather than a program to exit.
fn start_a_fresh_match(world: &mut World) {
    if !matches!(world.resource::<Scoreboard>().play, Play::Over { .. }) {
        return;
    }
    let restart = world
        .find_resource::<Input>()
        .is_some_and(|input| input.just_pressed(Key::Space));
    if !restart {
        return;
    }
    let carried = *world.resource::<Scoreboard>();
    let mut fresh = Scoreboard::new();
    // The match resets; the run's records do not, so a verification that plays
    // through more than one match still sees the fastest ball it ever hit.
    fresh.longest_rally = carried.longest_rally;
    fresh.top_speed = carried.top_speed;
    world.insert_resource(fresh);
}

/// Count the pause between points down, then put the ball in play.
fn serve(world: &mut World) {
    let (to, ticks_left) = match world.resource::<Scoreboard>().play {
        Play::Serving { to, ticks_left } => (to, ticks_left),
        _ => return,
    };
    if ticks_left > 0 {
        world.resource_mut::<Scoreboard>().play = Play::Serving {
            to,
            ticks_left: ticks_left - 1,
        };
        return;
    }

    // The engine's seeded `Rng`, so the same run serves the same ball every
    // time — which is what lets a verification be believed at all.
    let spread = {
        let rng = world.resource_mut::<Rng>();
        rng.next_f32() * 2.0 - 1.0
    };
    let (sin, cos) = sin_cos(Radians(spread * SERVE_SPREAD.as_f32()));
    // `to` is the side being served *at*, so the ball travels against that
    // side's outward direction.
    let velocity = Vec2::new(cos * -to.outward(), sin) * SERVE_SPEED;

    let Some(ball) = world.query::<&Ball>().map(|(entity, _)| entity).next() else {
        return;
    };
    world.component_mut::<Transform>(ball).pos = Vec2::ZERO;
    world.component_mut::<Ball>(ball).velocity = velocity;
    let board = world.resource_mut::<Scoreboard>();
    board.play = Play::Rally;
    board.rally = 0;
    board.top_speed = board.top_speed.max(SERVE_SPEED);
}

fn move_the_ball(world: &mut World) {
    if !matches!(world.resource::<Scoreboard>().play, Play::Rally) {
        return;
    }
    let step = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &Ball)>() {
        transform.pos += ball.velocity * step;
    }
}

/// Reflect the ball off the top and bottom walls.
///
/// Position-based rather than swept, and safe that way because a wall is
/// infinitely thick from the ball's side: the worst a fast tick can do is
/// overshoot into it, and mirroring the overshoot back out leaves the ball
/// exactly where a swept test would have put it.
fn bounce_off_the_walls(world: &mut World) {
    let limit = WALL_Y - BALL_RADIUS;
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        if transform.pos.y < -limit {
            transform.pos.y = -limit - (transform.pos.y + limit);
            ball.velocity.y = ball.velocity.y.abs();
        } else if transform.pos.y > limit {
            transform.pos.y = limit - (transform.pos.y - limit);
            ball.velocity.y = -ball.velocity.y.abs();
        }
    }
}

/// Reflect the ball off a paddle, with the contact point deciding the angle.
fn bounce_off_the_paddles(world: &mut World) {
    if !matches!(world.resource::<Scoreboard>().play, Play::Rally) {
        return;
    }
    let step = world.resource::<Time>().fixed_dt.as_f32();
    let Some((pos, velocity)) = ball_state(world) else {
        return;
    };
    // `move_the_ball` ran earlier this tick, so this is where the ball was when
    // the tick started — reconstructed rather than remembered, because a
    // component holding last tick's position is a second copy of the truth.
    //
    // One approximation lives here: `bounce_off_the_walls` runs between the two,
    // so on a tick where the ball also bounced off a wall this back-projection
    // follows the *outgoing* heading and misplaces `previous.y` by twice the
    // overshoot — at most half a unit at the speed ceiling. A ball that clips a
    // wall and a paddle tip in the same 1/60th of a second can therefore be
    // judged wrongly. Prototype-grade, and stated rather than hidden: fixing it
    // means the wall bounce recording where it happened, which is a second
    // component and a rule about ordering for a case worth about one frame a
    // match.
    let previous = pos - velocity * step;

    let paddles: Vec<(Side, f32)> = world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (paddle.side, transform.pos.y))
        .collect();

    for (side, paddle_y) in paddles {
        let Some(hit) = contact(previous, pos, side, paddle_y, step) else {
            continue;
        };
        let Some(ball) = world.query::<&Ball>().map(|(entity, _)| entity).next() else {
            return;
        };
        world.component_mut::<Transform>(ball).pos = hit.position;
        world.component_mut::<Ball>(ball).velocity = hit.velocity;
        let board = world.resource_mut::<Scoreboard>();
        board.rally += 1;
        board.longest_rally = board.longest_rally.max(board.rally);
        board.top_speed = board.top_speed.max(hit.velocity.length());
        // One paddle per tick. The two are thirty units apart and the ball
        // travels at most 0.55 in a tick, so there is no second one to find.
        return;
    }
}

/// Where a bounce leaves the ball, and where it is going afterwards.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Contact {
    pub(crate) position: Vec2,
    pub(crate) velocity: Vec2,
}

/// The swept paddle test.
///
/// The API document is explicit that there is no `Rect::sweep` in v1 and that
/// this is the game's own eight lines rather than something to go looking for:
/// the plane the ball's leading edge must cross, whether it was approaching it,
/// whether *this tick's travel* crossed it, and the fraction of the tick at
/// which it did. A position-only test would let a fast ball step clean through
/// a thin paddle between two ticks.
///
/// `previous` and `now` bracket one tick of travel. The verification's shot
/// planner calls this too, so what it predicts is what the game does.
pub(crate) fn contact(
    previous: Vec2,
    now: Vec2,
    side: Side,
    paddle_y: f32,
    step: f32,
) -> Option<Contact> {
    let travel = now - previous;
    // Heading towards this paddle at all?
    if travel.x * side.outward() >= 0.0 {
        return None;
    }
    // The plane the ball's near edge meets: the paddle's inner face, pushed out
    // by the ball's radius, so the test is a point against a plane.
    let face = side.paddle_x() + side.outward() * (PADDLE_SIZE.x * 0.5 + BALL_RADIUS);
    // Distance outward from that plane: positive is in front of the paddle.
    let before = (previous.x - face) * side.outward();
    let after = (now.x - face) * side.outward();
    // In front before, behind now. `>=` on the first so a ball that finished
    // last tick exactly on the face still registers this tick.
    if !(before >= 0.0 && after < 0.0) {
        return None;
    }
    // Positive, because `before >= 0.0 > after`.
    let fraction = before / (before - after);
    let at = previous + travel * fraction;

    let reach = PADDLE_SIZE.y * 0.5 + BALL_RADIUS;
    let offset = at.y - paddle_y;
    if offset.abs() > reach {
        return None;
    }

    // Where along the paddle it landed — `-1.0` at the top edge, `+1.0` at the
    // bottom — sets the outgoing angle. Dead centre sends the ball straight
    // back; the tips send it away at `MAX_BOUNCE`.
    let along = (offset / reach).clamp(-1.0, 1.0);
    let (sin, cos) = sin_cos(Radians(along * MAX_BOUNCE.as_f32()));
    let speed = (travel.length() / step + SPEED_GAIN).min(MAX_BALL_SPEED);
    let velocity = Vec2::new(cos * side.outward(), sin) * speed;

    // Spend the rest of the tick on the new heading, so the ball does not stall
    // against the paddle for a frame.
    Some(Contact {
        position: at + velocity * (1.0 - fraction) * step,
        velocity,
    })
}

/// A ball past a goal line is a point, and `WINNING_SCORE` points is a match.
fn score_a_point(world: &mut World) {
    if !matches!(world.resource::<Scoreboard>().play, Play::Rally) {
        return;
    }
    let Some((pos, _)) = ball_state(world) else {
        return;
    };
    let conceded = if pos.x < -GOAL_X {
        Some(Side::Left)
    } else if pos.x > GOAL_X {
        Some(Side::Right)
    } else {
        None
    };
    let Some(conceded) = conceded else { return };

    let scorer = conceded.other();
    let board = world.resource_mut::<Scoreboard>();
    match scorer {
        Side::Left => board.left += 1,
        Side::Right => board.right += 1,
    }
    board.play = if board.points(scorer) >= WINNING_SCORE {
        Play::Over { winner: scorer }
    } else {
        // The side that conceded receives the next serve.
        Play::Serving {
            to: conceded,
            ticks_left: SERVE_PAUSE,
        }
    };

    // Park the ball in the middle rather than leave it drifting off into
    // nowhere during the pause.
    let Some(ball) = world.query::<&Ball>().map(|(entity, _)| entity).next() else {
        return;
    };
    world.component_mut::<Transform>(ball).pos = Vec2::ZERO;
    world.component_mut::<Ball>(ball).velocity = Vec2::ZERO;
}
