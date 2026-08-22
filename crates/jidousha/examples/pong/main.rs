//! Pong: a paddle, an opponent, and a ball that gets faster every time you hit
//! it.
//!
//! W and S move your paddle, on the left. First to five. Press R when it is
//! over and it starts again.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//! On the web: `tools/serve-web pong`
//!
//! The court, the collision arithmetic and the opponent's decision are all in
//! `rules.rs`, as constants and free functions of their arguments. This file is
//! the part that needs a `World`: what to spawn, what to move, what to draw.
//! The split is not tidiness — a `--verify` mode that plays the game rather than
//! scripting it has to be able to ask "where will the ball be" and "where will
//! the opponent go", and nothing can fork a running simulation to find out. It
//! can only call a function, so the answers are functions.

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod controller;
mod rules;
mod verify;

use rules::{
    BALL_HALF, BALL_SPEED_GAIN, BALL_SPEED_MAX, BALL_SPEED_START, BALL_Y_LIMIT, COURT, HALF_H,
    HALF_W, OPPONENT_SPEED, PADDLE_HALF_X, PADDLE_HALF_Y, PADDLE_Y_LIMIT, PLAYER_SPEED,
    SERVE_SPREAD, SERVE_TICKS, Side, VIEW_HEIGHT, WINNING_SCORE,
};

/// How thick the court's border is, in world units.
const BORDER_THICKNESS: f32 = 0.14;

/// Draw bands, named once so that no `layer: 2` ever appears at a call site.
pub(crate) mod layers {
    /// The court and its markings.
    pub(crate) const COURT: i16 = -1;
    /// Paddles and ball.
    pub(crate) const PLAY: i16 = 0;
    /// Score and prompts, over everything.
    pub(crate) const UI: i16 = 1;
}

// --- what the world holds ----------------------------------------------

/// A paddle, and who moves it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Paddle {
    /// Which end of the court this paddle defends.
    pub(crate) side: Side,
    /// How fast it may move, in world units per second.
    pub(crate) speed: f32,
}
impl Component for Paddle {}

/// The ball.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ball {
    /// Where it is going, in world units per second. Zero between points.
    pub(crate) velocity: Vec2,
    /// How fast it will leave the next paddle it touches.
    pub(crate) speed: f32,
}
impl Component for Ball {}

/// Which screen the game is on.
///
/// A game's own `Phase`-shaped enum. The engine's `Phase` is a bound on
/// `add_system` and is not a name a game can collide with, so this one is free
/// to be called whatever it should be called.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Screen {
    /// The ball is sitting at the centre, counting down to a serve.
    Serving,
    /// The ball is in play.
    Rally,
    /// Somebody reached [`WINNING_SCORE`].
    Over,
}

/// The score, the screen, and whose turn it is to receive.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Round {
    /// Which screen the game is on.
    pub(crate) screen: Screen,
    /// The player's score.
    pub(crate) left: u32,
    /// The opponent's score.
    pub(crate) right: u32,
    /// Ticks left before the ball is served.
    pub(crate) countdown: u32,
    /// Which side the ball is served *towards* — the side that just conceded.
    pub(crate) serve_to: Side,
}
impl Resource for Round {}

impl Round {
    /// A fresh game: nil-nil, serving to the player.
    pub(crate) const fn new() -> Round {
        Round {
            screen: Screen::Serving,
            left: 0,
            right: 0,
            countdown: SERVE_TICKS,
            serve_to: Side::Left,
        }
    }

    /// Whoever has reached [`WINNING_SCORE`], if anybody has.
    pub(crate) fn winner(&self) -> Option<Side> {
        match (self.left >= WINNING_SCORE, self.right >= WINNING_SCORE) {
            (true, _) => Some(Side::Left),
            (_, true) => Some(Side::Right),
            _ => None,
        }
    }
}

// --- wiring ------------------------------------------------------------

/// The game's configuration, shared by the window and the verify run, so that
/// what is checked is what a person plays.
pub(crate) fn config() -> GameConfig {
    GameConfig {
        title: "jidousha — pong",
        window_size: rules::WINDOW,
        ..GameConfig::default()
    }
}

/// Every system this game has, in the order they run.
///
/// **The order of the four Update systems is a decision, not a listing.** Both
/// paddles move before the ball does, so `rules::paddle_contact` can treat a
/// paddle as standing still at its post-move position for the whole tick —
/// which is the difference between a paddle that catches a ball closing on it
/// and one the ball goes through. Move `advance_the_ball` above the paddles and
/// the game still compiles, still runs, and starts leaking balls through
/// paddles in the exact case a player notices.
/// `checks::the_paddles_move_before_the_ball` asserts this against
/// `HeadlessSim::schedule_debug`, which is the only instrument that sees it.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, run_the_round);
    app.add_system(Update, drive_the_player);
    app.add_system(Update, drive_the_opponent);
    app.add_system(Update, advance_the_ball);
    // Drawn back to front by *band*, and submitted in the opposite order on
    // purpose. Where the bands already agree with the submission sequence they
    // are invisible: `quads()` comes back in the depth sort, so a court marking
    // submitted before the ball is behind it either way and no assertion can
    // tell whether `mod layers` did anything. Submitted after, its position in
    // the sort is the band's doing and one index comparison tests it. Same for
    // the score, submitted first and drawn last.
    app.add_system(Draw, draw_the_score);
    app.add_system(Draw, draw_the_play);
    app.add_system(Draw, draw_the_court);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("W and S move your paddle. first to {WINNING_SCORE}. close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        // Print it rather than returning it: `RunError`'s `Display` is the
        // engine's four-part message, and its `Debug` is a struct dump.
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

// --- systems -----------------------------------------------------------

/// The camera, the round, the two paddles and the ball.
fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        center: Vec2::ZERO,
        height: VIEW_HEIGHT,
        clear_color: COURT,
        ..Camera::default()
    });
    world.insert_resource(Round::new());

    for (side, speed) in [(Side::Left, PLAYER_SPEED), (Side::Right, OPPONENT_SPEED)] {
        let paddle = world.spawn();
        world.insert(paddle, Transform::at(Vec2::new(side.paddle_x(), 0.0)));
        world.insert(paddle, Paddle { side, speed });
    }

    let ball = world.spawn();
    world.insert(ball, Transform::at(Vec2::ZERO));
    world.insert(
        ball,
        Ball {
            velocity: Vec2::ZERO,
            speed: BALL_SPEED_START,
        },
    );
}

/// Count down to the serve, launch it, and restart the game when asked.
fn run_the_round(world: &mut World) {
    let restart = world
        .find_resource::<Input>()
        .is_some_and(|input| input.just_pressed(Key::R));
    let round = *world.resource::<Round>();

    match round.screen {
        Screen::Over => {
            if restart {
                *world.resource_mut::<Round>() = Round::new();
                park_the_ball(world);
            }
        }
        Screen::Rally => {}
        Screen::Serving => {
            if round.countdown > 0 {
                world.resource_mut::<Round>().countdown -= 1;
                return;
            }
            // A serve angle from the seeded RNG, so the same run plays the same
            // game every time — which is what lets the verify run replay it.
            let spread = world.resource_mut::<Rng>().next_f32() * 2.0 - 1.0;
            let (sine, cosine) = sin_cos(Radians(SERVE_SPREAD.as_f32() * spread));
            let velocity = Vec2::new(round.serve_to.sign() * cosine, sine) * BALL_SPEED_START;
            for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
                transform.pos = Vec2::ZERO;
                ball.velocity = velocity;
                ball.speed = BALL_SPEED_START;
            }
            world.resource_mut::<Round>().screen = Screen::Rally;
        }
    }
}

/// Put the ball back on the centre spot, stopped.
fn park_the_ball(world: &mut World) {
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        transform.pos = Vec2::ZERO;
        ball.velocity = Vec2::ZERO;
        ball.speed = BALL_SPEED_START;
    }
}

/// W and S move the left paddle.
fn drive_the_player(world: &mut World) {
    let lean = match world.find_resource::<Input>() {
        // The first tick of a run happens before any input exists, and under
        // `headless` there is never one unless a check inserts it.
        None => return,
        Some(input) => f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    };
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.side == Side::Left {
            // Per second, times the timestep, so the paddle keeps its speed the
            // day `GameConfig::fixed_dt` changes.
            let step = lean * paddle.speed * dt;
            transform.pos.y = (transform.pos.y + step).clamp(-PADDLE_Y_LIMIT, PADDLE_Y_LIMIT);
        }
    }
}

/// The opponent moves towards wherever `rules::opponent_target` says to be.
fn drive_the_opponent(world: &mut World) {
    // Read everything the decision needs, then write: a `query_mut` holds the
    // world for as long as it iterates, so the paddle cannot be moved while the
    // ball is still being looked at.
    let Some((_, ball_transform, ball)) = world.query::<(&Transform, &Ball)>().next() else {
        return;
    };
    let (ball_pos, velocity) = (ball_transform.pos, ball.velocity);
    let player_y = world
        .query::<(&Transform, &Paddle)>()
        .find(|(_, _, paddle)| paddle.side == Side::Left)
        .map_or(0.0, |(_, transform, _)| transform.pos.y);

    let target = rules::opponent_target(ball_pos, velocity, player_y);
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.side == Side::Right {
            // `move_towards` stops exactly on the target rather than jittering
            // across it the way `signum * step` would.
            let step = paddle.speed * dt;
            let moved = transform
                .pos
                .move_towards(Vec2::new(transform.pos.x, target), step);
            transform.pos.y = moved.y.clamp(-PADDLE_Y_LIMIT, PADDLE_Y_LIMIT);
        }
    }
}

/// Move the ball one tick: paddles, then walls, then the goal lines.
fn advance_the_ball(world: &mut World) {
    if world.resource::<Round>().screen != Screen::Rally {
        return;
    }
    let dt = world.resource::<Time>().fixed_dt.as_f32();

    // The paddles as the ball will see them: post-move, standing still. Read
    // out into a plain array before anything is written.
    let mut paddles = [(Side::Left, 0.0_f32), (Side::Right, 0.0_f32)];
    for (_, transform, paddle) in world.query::<(&Transform, &Paddle)>() {
        let slot = match paddle.side {
            Side::Left => 0,
            Side::Right => 1,
        };
        paddles[slot] = (paddle.side, transform.pos.y);
    }

    let Some((entity, transform, ball)) = world.query::<(&Transform, &Ball)>().next() else {
        return;
    };
    let (mut position, mut velocity, mut speed) = (transform.pos, ball.velocity, ball.speed);

    let from = position;
    let to = from + velocity * dt;
    // Only one paddle can be met in a tick — the ball is travelling one way in x
    // and the court is a hundred ticks wide — but ask both and take the earlier,
    // so nothing depends on that staying true.
    let hit = paddles
        .iter()
        .filter_map(|&(side, paddle_y)| {
            rules::paddle_contact(from, to, side, paddle_y).map(|contact| (side, paddle_y, contact))
        })
        .min_by(|left, right| left.2.fraction.total_cmp(&right.2.fraction));

    position = match hit {
        Some((side, paddle_y, contact)) => {
            speed = (speed + BALL_SPEED_GAIN).min(BALL_SPEED_MAX);
            velocity = rules::rebound(contact.at.y, paddle_y, side, speed);
            // The rest of the tick, travelled at the new velocity.
            contact.at + velocity * dt * (1.0 - contact.fraction)
        }
        None => to,
    };

    // The walls. One clamp is enough: a tick's travel is far shorter than the
    // court is tall, which `checks::the_ball_cannot_outrun_the_thinnest_collider`
    // is the general form of.
    if position.y < -BALL_Y_LIMIT {
        position.y = -BALL_Y_LIMIT;
        velocity.y = velocity.y.abs();
    } else if position.y > BALL_Y_LIMIT {
        position.y = BALL_Y_LIMIT;
        velocity.y = -velocity.y.abs();
    }

    {
        let transform = world.component_mut::<Transform>(entity);
        transform.pos = position;
    }
    {
        let ball = world.component_mut::<Ball>(entity);
        ball.velocity = velocity;
        ball.speed = speed;
    }

    // Past a paddle and off the court: a point to the other end.
    let conceded = if position.x < -rules::GOAL_X {
        Some(Side::Left)
    } else if position.x > rules::GOAL_X {
        Some(Side::Right)
    } else {
        None
    };
    let Some(conceded) = conceded else { return };
    let round = world.resource_mut::<Round>();
    match conceded.other() {
        Side::Left => round.left += 1,
        Side::Right => round.right += 1,
    }
    round.serve_to = conceded;
    round.countdown = SERVE_TICKS;
    round.screen = if round.winner().is_some() {
        Screen::Over
    } else {
        Screen::Serving
    };
    park_the_ball(world);
}

// --- drawing -----------------------------------------------------------

/// The border, and the dashed line down the middle.
fn draw_the_court(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::COURT);
    let line = Color::rgba(0.6, 0.85, 1.0, 0.10);
    // A border is an outline, and there is no stroke mode: four lines.
    // On the court's true boundary, which is what a border means. It fits
    // because the camera shows `rules::MARGIN` more than the court in every
    // direction; drawn against the camera's own edge instead, half this line's
    // thickness would be off screen.
    let court = Rect::from_center_size(Vec2::ZERO, Vec2::new(HALF_W, HALF_H) * 2.0);
    let corners = [
        court.min,
        Vec2::new(court.max.x, court.min.y),
        court.max,
        Vec2::new(court.min.x, court.max.y),
    ];
    for index in 0..4 {
        ctx.line(
            corners[index],
            corners[(index + 1) % 4],
            BORDER_THICKNESS,
            line,
            depth,
        );
    }

    // The centre marking, as a column of rectangles — the dashes are the shape,
    // and there is no dash pattern to ask for.
    let dashes = 13;
    let pitch = (HALF_H * 2.0) / dashes as f32;
    for index in 0..dashes {
        let middle = -HALF_H + pitch * (index as f32 + 0.5);
        ctx.rect(
            Rect::from_center_size(Vec2::new(0.0, middle), Vec2::new(0.12, pitch * 0.5)),
            Color::rgba(0.6, 0.85, 1.0, 0.18),
            depth,
        );
    }
}

/// The two paddles and the ball.
fn draw_the_play(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PLAY);
    let size = Vec2::new(PADDLE_HALF_X, PADDLE_HALF_Y) * 2.0;
    // Straight out of the query: a Draw system's iterator borrows the world, not
    // the context, so there is no `Vec` in between.
    for (_, transform, paddle) in ctx.world.query::<(&Transform, &Paddle)>() {
        let color = match paddle.side {
            Side::Left => Color::rgb(0.45, 0.95, 0.75),
            Side::Right => Color::rgb(0.98, 0.62, 0.45),
        };
        ctx.rect(Rect::from_center_size(transform.pos, size), color, depth);
    }
    // The ball is drawn as the square it collides as.
    for (_, transform, _) in ctx.world.query::<(&Transform, &Ball)>() {
        ctx.rect(
            Rect::from_center_size(transform.pos, Vec2::splat(BALL_HALF * 2.0)),
            Color::WHITE,
            depth,
        );
    }
}

/// What the screen has to say, or nothing while the ball is in play.
///
/// A function rather than a `match` inside the draw system so that a check can
/// ask the game for the exact text it draws. No assertion over drawn quads can
/// see a wrong *character* — the font draws an identically sized box for one —
/// so the only instrument is the string itself, and a check holding its own copy
/// of the literals is inspecting a string the game may no longer draw.
pub(crate) fn banner_for(screen: Screen, winner: Option<Side>) -> Option<&'static str> {
    match (screen, winner) {
        (Screen::Over, Some(Side::Left)) => Some("YOU WIN  -  PRESS R"),
        (Screen::Over, _) => Some("OPPONENT WINS  -  PRESS R"),
        (Screen::Serving, _) => Some("W AND S TO MOVE"),
        (Screen::Rally, _) => None,
    }
}

/// The score at the top, and whatever the screen has to say.
fn draw_the_score(ctx: &mut DrawCtx) {
    let round = ctx.world.resource::<Round>();
    let score = TextStyle {
        size: 2.4,
        color: Color::rgba(0.85, 0.93, 1.0, 0.75),
        depth: Depth::layer(layers::UI),
    };
    // Y is down, so the top of the court is `-HALF_H` and a line drawn at `y`
    // occupies `y .. y + size`.
    let top = -HALF_H + 0.8;
    let left = format!("{}", round.left);
    ctx.text(Vec2::new(-2.0 - score.width_of(&left), top), &left, score);
    ctx.text(Vec2::new(2.0, top), &format!("{}", round.right), score);

    let Some(message) = banner_for(round.screen, round.winner()) else {
        return;
    };
    let banner = TextStyle {
        size: 1.0,
        color: Color::rgba(0.85, 0.93, 1.0, 0.85),
        depth: Depth::layer(layers::UI),
    };
    // The whole of `size` is what clears a bottom edge, not half of it.
    ctx.text(
        Vec2::new(-banner.width_of(message) * 0.5, HALF_H - 2.0 - banner.size),
        message,
        banner,
    );
}
