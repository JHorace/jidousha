//! Pong. Two paddles, a ball, a score, and a winner.
//!
//! W/S or Up/Down move the left paddle. The right paddle plays itself. First to
//! five wins; Space starts the next game.
//!
//! Written against `docs/api/jidousha-api.md` and the examples beside this one.
//! Every shape here is drawn by the engine, so there are no asset files and no
//! loading to wait for.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//!
//! The check lives in `verify.rs` next to this file: the same systems and the
//! same config the window runs, driven by a script instead of a person, with
//! assertions about what the world did and what was drawn.

use jidousha::math::sin_cos;
use jidousha::prelude::*;

/// How many world units the screen spans vertically.
const VIEW_HEIGHT: f32 = 20.0;

/// Half the playfield, in world units: the ball scores past `x`, and bounces
/// off `y`.
///
/// Fixed rather than read off the camera so the game is the same shape in the
/// window and in the headless check. The window opens at 16:9 (`config`), which
/// is 17.8 world units either side of centre — wider than this field, so the
/// whole thing is on screen with room to spare.
const FIELD: Vec2 = Vec2::new(16.0, 9.0);

/// How big a paddle is, in world units.
///
/// The width matters. Collision is an overlap test once a tick, so the ball can
/// only be caught while its box and the paddle's box meet — a window
/// `PADDLE_SIZE.x + BALL_SIZE` wide. The ball moves `BALL_MAX_SPEED / 60` units
/// in a tick, and that has to stay comfortably under the window or a fast ball
/// steps straight through the bat.
const PADDLE_SIZE: Vec2 = Vec2::new(0.6, 3.4);

/// How far a paddle's centre sits inside the field's edge.
const PADDLE_INSET: f32 = 1.4;

/// The side of the square ball, in world units.
const BALL_SIZE: f32 = 0.55;

/// How fast the player's paddle travels, in world units per second.
const PLAYER_SPEED: f32 = 24.0;

/// How fast the computer's paddle travels.
///
/// This one number is the whole difficulty setting, and what it is measured
/// against is the ball's *crossing time*. A serve at `BALL_START_SPEED` takes
/// long enough to cross the field that the computer can get anywhere in time,
/// so early exchanges are comfortable; a ball wound up to `BALL_MAX_SPEED`
/// crosses in about 0.9 seconds, in which this covers 13 of the 16 units it
/// might need. That gap is where a rally ends.
const CPU_SPEED: f32 = 15.5;

/// How far off-centre the ball may be before the computer bothers to move.
///
/// Without this the paddle jitters either side of the ball forever, which
/// looks like a bug even though it plays fine.
const CPU_DEADZONE: f32 = 0.5;

/// How fast the ball leaves a serve, in world units per second.
const BALL_START_SPEED: f32 = 20.0;

/// How much faster the ball gets with every paddle it touches.
///
/// This is what ends a rally. Two players who can both reach everything will
/// keep a slow ball up forever; the ramp is what eventually puts a shot out of
/// one of their reach, and it is the reason a long exchange gets tense instead
/// of getting boring.
const BALL_SPEED_GAIN: f32 = 2.0;

/// As fast as the ball is ever allowed to get.
///
/// Capped so the per-tick step stays well inside the collision window described
/// on `PADDLE_SIZE`: 40/60 is 0.67 units against a window of 1.15.
const BALL_MAX_SPEED: f32 = 40.0;

/// The steepest angle a paddle can put on the ball, measured from the X axis.
///
/// Hitting with the end of the paddle rather than the middle is the only aiming
/// the game has, and this is how much it is worth.
const MAX_BOUNCE: Radians = Radians(0.95);

/// How far a serve may wander off straight.
const SERVE_SPREAD: Radians = Radians(0.35);

/// How long the pause before a serve lasts, in ticks.
///
/// The timestep is 1/60 of a second, so this is four fifths of one.
const SERVE_TICKS: u32 = 48;

/// How many points win a game.
const WINNING_SCORE: u32 = 5;

/// Draw bands, so the ordering is stated once rather than guessed at each site.
mod layers {
    /// The field and its markings.
    pub const FIELD: i16 = -1;
    /// Paddles and ball.
    pub const PLAY: i16 = 0;
    /// Score and banners.
    pub const UI: i16 = 1;
}

/// Which end of the field something belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

impl Side {
    /// The other one.
    fn opposite(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// Which way along X this side's paddle hits the ball.
    fn outward(self) -> f32 {
        match self {
            Side::Left => 1.0,
            Side::Right => -1.0,
        }
    }
}

/// Who is moving a paddle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    /// The keyboard.
    Keys,
    /// The game itself.
    Cpu,
}

/// A bat at one end of the field.
#[derive(Clone, Copy)]
struct Paddle {
    side: Side,
    control: Control,
    /// World units per second.
    speed: f32,
}
impl Component for Paddle {}

/// The ball, and where it is going.
#[derive(Clone, Copy)]
struct Ball {
    /// World units per second. Zero between points.
    vel: Vec2,
}
impl Component for Ball {}

/// The score.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Score {
    left: u32,
    right: u32,
}
impl Resource for Score {}

/// What the game is doing right now.
///
/// Named `Round` rather than `Phase` because `Phase` is the engine's word for
/// Startup/Update/Draw and shadowing it in a game would read badly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Round {
    /// Waiting to put the ball back in play, `ticks` from now, travelling
    /// `toward` that side.
    Serving { ticks: u32, toward: Side },
    /// The ball is live.
    Rally,
    /// Somebody reached `WINNING_SCORE`.
    Over { winner: Side },
}
impl Resource for Round {}

/// How many paddles the ball has touched since the serve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Volley(u32);
impl Resource for Volley {}

/// The game's configuration, shared by the window and the check, so that what
/// is verified is what a person sees.
fn config() -> GameConfig {
    GameConfig {
        title: "jidousha — pong",
        ..GameConfig::default()
    }
}

/// Every system this game has, in one place.
///
/// Named rather than written inline so `--verify` runs the *same* game the
/// window does.
fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, steer_the_paddles);
    app.add_system(Update, count_down_the_serve);
    app.add_system(Update, carry_the_ball);
    app.add_system(Update, bounce_off_the_walls);
    app.add_system(Update, bounce_off_the_paddles);
    app.add_system(Update, award_the_point);
    app.add_system(Update, start_another_game);
    app.add_system(Draw, draw_the_field);
    app.add_system(Draw, draw_the_play);
    app.add_system(Draw, draw_the_readout);
}

mod verify;

fn main() -> Result<(), RunError> {
    if std::env::args().any(|argument| argument == "--verify") {
        verify::run();
        return Ok(());
    }
    println!("W/S or Up/Down move the left paddle. first to {WINNING_SCORE} wins.");
    run(config(), register)
}

fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        clear_color: Color::rgb(0.04, 0.05, 0.07),
        height: VIEW_HEIGHT,
        ..Camera::default()
    });
    world.insert_resource(Score::default());
    world.insert_resource(Volley::default());
    world.insert_resource(Round::Serving {
        ticks: SERVE_TICKS,
        toward: Side::Right,
    });

    for (side, control) in [(Side::Left, Control::Keys), (Side::Right, Control::Cpu)] {
        let paddle = world.spawn();
        let x = side.outward() * -(FIELD.x - PADDLE_INSET);
        world.insert(paddle, Transform::at(Vec2::new(x, 0.0)));
        world.insert(
            paddle,
            Paddle {
                side,
                control,
                speed: match control {
                    Control::Keys => PLAYER_SPEED,
                    Control::Cpu => CPU_SPEED,
                },
            },
        );
    }

    let ball = world.spawn();
    world.insert(ball, Transform::at(Vec2::ZERO));
    world.insert(ball, Ball { vel: Vec2::ZERO });
}

/// How far up or down a paddle wants to go this tick, as -1, 0 or 1.
///
/// The computer chases the ball only while it is coming this way, and goes back
/// to the middle otherwise — which is both what a person does and what stops it
/// from being unbeatable.
fn cpu_direction(side: Side, paddle_y: f32, ball: Option<(Vec2, Vec2)>) -> f32 {
    let Some((pos, vel)) = ball else {
        return 0.0;
    };
    let incoming = match side {
        Side::Left => vel.x < 0.0,
        Side::Right => vel.x > 0.0,
    };
    let target = if incoming { pos.y } else { 0.0 };
    let delta = target - paddle_y;
    if delta.abs() < CPU_DEADZONE {
        0.0
    } else {
        delta.signum()
    }
}

/// Move both paddles: one from the keyboard, one from the ball.
///
/// The two-pass shape the engine's Concepts section describes — the ball is
/// read into a local before the paddles are borrowed for writing, because one
/// query cannot be open while another is.
fn steer_the_paddles(world: &mut World) {
    // A run's first tick can happen before any input is set.
    let keys = match world.find_resource::<Input>() {
        None => 0.0,
        // Y is down: S and Down move the paddle towards the bottom of the
        // screen, which is the larger number.
        Some(input) => {
            let down = input.held(Key::S) || input.held(Key::ArrowDown);
            let up = input.held(Key::W) || input.held(Key::ArrowUp);
            f32::from(down) - f32::from(up)
        }
    };
    let ball = world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| (transform.pos, ball.vel))
        .next();
    let step = world.resource::<Time>().fixed_dt.as_f32();
    let limit = FIELD.y - PADDLE_SIZE.y * 0.5;

    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        let direction = match paddle.control {
            Control::Keys => keys,
            Control::Cpu => cpu_direction(paddle.side, transform.pos.y, ball),
        };
        transform.pos.y = (transform.pos.y + direction * paddle.speed * step).clamp(-limit, limit);
    }
}

/// Count the serve pause down, then put the ball in play.
fn count_down_the_serve(world: &mut World) {
    let Round::Serving { ticks, toward } = *world.resource::<Round>() else {
        return;
    };
    // The tick that awarded the point is the first of the pause — it ends with
    // the ball already parked — so the countdown runs out one tick early and
    // `SERVE_TICKS` is the number of ticks a watcher actually waits.
    let remaining = ticks.saturating_sub(1);
    if remaining > 0 {
        *world.resource_mut::<Round>() = Round::Serving {
            ticks: remaining,
            toward,
        };
        return;
    }

    // Seeded from `GameConfig`, so the same run serves the same way every time
    // — which is what lets the check replay a session and get one answer.
    let spread = world.resource_mut::<Rng>().next_f32() - 0.5;
    let (sine, cosine) = sin_cos(Radians(spread * 2.0 * SERVE_SPREAD.as_f32()));
    let toward_x = match toward {
        Side::Left => -1.0,
        Side::Right => 1.0,
    };
    let launch = Vec2::new(toward_x * cosine, sine) * BALL_START_SPEED;

    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        transform.pos = Vec2::ZERO;
        ball.vel = launch;
    }
    world.resource_mut::<Volley>().0 = 0;
    *world.resource_mut::<Round>() = Round::Rally;
}

/// Move the ball by one tick's worth of its velocity.
fn carry_the_ball(world: &mut World) {
    let step = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &Ball)>() {
        transform.pos += ball.vel * step;
    }
}

/// Reflect the ball off the top and bottom of the field.
///
/// The overshoot is folded back rather than the position being clamped, so a
/// fast ball keeps the distance it travelled and the bounce stays symmetric.
fn bounce_off_the_walls(world: &mut World) {
    let edge = FIELD.y - BALL_SIZE * 0.5;
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        if transform.pos.y < -edge {
            transform.pos.y = -edge - (transform.pos.y + edge);
            ball.vel.y = ball.vel.y.abs();
        } else if transform.pos.y > edge {
            transform.pos.y = edge - (transform.pos.y - edge);
            ball.vel.y = -ball.vel.y.abs();
        }
    }
}

/// Reflect the ball off a paddle, steeper the further from the paddle's middle
/// it lands, and a little faster every time.
fn bounce_off_the_paddles(world: &mut World) {
    if *world.resource::<Round>() != Round::Rally {
        return;
    }
    let paddles: Vec<(Side, Vec2)> = world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (paddle.side, transform.pos))
        .collect();
    let Some((entity, pos, vel)) = world
        .query::<(&Transform, &Ball)>()
        .map(|(entity, transform, ball)| (entity, transform.pos, ball.vel))
        .next()
    else {
        return;
    };

    let ball_box = Rect::from_center_size(pos, Vec2::new(BALL_SIZE, BALL_SIZE));
    let hit = paddles.into_iter().find(|(side, at)| {
        // Only a ball travelling towards a paddle can bounce off it. Without
        // this a ball that ends a tick still inside the paddle would flip back
        // and forth and never leave.
        vel.x * side.outward() < 0.0 && Rect::from_center_size(*at, PADDLE_SIZE).overlaps(ball_box)
    });
    let Some((side, at)) = hit else {
        return;
    };

    let offset = ((pos.y - at.y) / (PADDLE_SIZE.y * 0.5)).clamp(-1.0, 1.0);
    let (sine, cosine) = sin_cos(Radians(offset * MAX_BOUNCE.as_f32()));
    let speed = (vel.length() + BALL_SPEED_GAIN).min(BALL_MAX_SPEED);

    let transform = world.component_mut::<Transform>(entity);
    // Put the ball against the paddle's face, so the next tick starts it clear.
    transform.pos.x = at.x + side.outward() * (PADDLE_SIZE.x + BALL_SIZE) * 0.5;
    world.component_mut::<Ball>(entity).vel = Vec2::new(side.outward() * cosine, sine) * speed;
    world.resource_mut::<Volley>().0 += 1;
}

/// A ball past either end is a point, and either a new serve or a winner.
fn award_the_point(world: &mut World) {
    if *world.resource::<Round>() != Round::Rally {
        return;
    }
    let Some(pos) = world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, _)| transform.pos)
        .next()
    else {
        return;
    };
    // Past the right-hand end means the right paddle missed, so the left one
    // scored.
    let scorer = if pos.x > FIELD.x {
        Side::Left
    } else if pos.x < -FIELD.x {
        Side::Right
    } else {
        return;
    };

    let score = world.resource_mut::<Score>();
    match scorer {
        Side::Left => score.left += 1,
        Side::Right => score.right += 1,
    }
    let reached = match scorer {
        Side::Left => score.left,
        Side::Right => score.right,
    };

    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        transform.pos = Vec2::ZERO;
        ball.vel = Vec2::ZERO;
    }
    world.resource_mut::<Volley>().0 = 0;
    *world.resource_mut::<Round>() = if reached >= WINNING_SCORE {
        Round::Over { winner: scorer }
    } else {
        // The ball goes to whoever just conceded.
        Round::Serving {
            ticks: SERVE_TICKS,
            toward: scorer.opposite(),
        }
    };
}

/// Space, after a game is over, starts another one.
fn start_another_game(world: &mut World) {
    if !matches!(*world.resource::<Round>(), Round::Over { .. }) {
        return;
    }
    let pressed = world
        .find_resource::<Input>()
        .is_some_and(|input| input.just_pressed(Key::Space));
    if !pressed {
        return;
    }
    *world.resource_mut::<Score>() = Score::default();
    *world.resource_mut::<Volley>() = Volley::default();
    *world.resource_mut::<Round>() = Round::Serving {
        ticks: SERVE_TICKS,
        toward: Side::Right,
    };
    for (_, transform, _) in world.query_mut::<(&mut Transform, &Paddle)>() {
        transform.pos.y = 0.0;
    }
}

/// The border and the halfway line.
///
/// Alpha blends in linear light, so these read brighter than the numbers look —
/// both of these were picked down from something that seemed reasonable on
/// paper and turned out to be a solid white wall.
fn draw_the_field(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::FIELD);
    let ink = Color::rgba(1.0, 1.0, 1.0, 0.05);
    let corners = [
        Vec2::new(-FIELD.x, -FIELD.y),
        Vec2::new(FIELD.x, -FIELD.y),
        Vec2::new(FIELD.x, FIELD.y),
        Vec2::new(-FIELD.x, FIELD.y),
    ];
    for index in 0..4 {
        ctx.line(corners[index], corners[(index + 1) % 4], 0.12, ink, depth);
    }

    // A dashed halfway line, drawn as its dashes: there is no dash pattern to
    // set, and there does not need to be one.
    let dashes = 13;
    let pitch = FIELD.y * 2.0 / dashes as f32;
    for index in 0..dashes {
        let y = -FIELD.y + pitch * (index as f32 + 0.5);
        ctx.rect(
            Rect::from_center_size(Vec2::new(0.0, y), Vec2::new(0.16, pitch * 0.55)),
            ink,
            depth,
        );
    }
}

/// The paddles and the ball.
fn draw_the_play(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PLAY);
    for (_, transform, paddle) in ctx.world.query::<(&Transform, &Paddle)>() {
        let color = match paddle.control {
            Control::Keys => Color::rgb(0.45, 0.95, 0.75),
            Control::Cpu => Color::rgb(0.95, 0.55, 0.45),
        };
        ctx.rect(
            Rect::from_center_size(transform.pos, PADDLE_SIZE),
            color,
            depth,
        );
    }
    for (_, transform, _) in ctx.world.query::<(&Transform, &Ball)>() {
        ctx.rect(
            Rect::from_center_size(transform.pos, Vec2::new(BALL_SIZE, BALL_SIZE)),
            Color::WHITE,
            depth,
        );
    }
}

/// The score, and whatever the game wants to say.
fn draw_the_readout(ctx: &mut DrawCtx) {
    let (top_left, bottom_right) = ctx.world.resource::<Camera>().visible_bounds();
    let score = *ctx.world.resource::<Score>();
    let digits = TextStyle {
        size: 2.6,
        color: Color::rgba(1.0, 1.0, 1.0, 0.85),
        depth: Depth::layer(layers::UI),
    };
    let top = top_left.y + 0.9;

    // Either side of the halfway line, each measured so the pair stays centred
    // however many digits it grows to.
    let left = score.left.to_string();
    ctx.text(Vec2::new(-1.4 - digits.width_of(&left), top), &left, digits);
    ctx.text(Vec2::new(1.4, top), &score.right.to_string(), digits);

    let banner = TextStyle {
        size: 1.3,
        color: Color::rgba(0.75, 0.9, 1.0, 0.9),
        depth: Depth::layer(layers::UI),
    };
    let footnote = TextStyle {
        size: 0.8,
        color: Color::rgba(0.75, 0.9, 1.0, 0.65),
        depth: Depth::layer(layers::UI),
    };
    // Two short centred lines rather than one long one. The first version put
    // the winner and the restart key in a single string, which came to 43
    // characters — 43.5 world units across a screen 35.6 wide, so it ran off
    // both edges. Nothing in the game noticed; the frame transcript did.
    let (headline, note) = match *ctx.world.resource::<Round>() {
        Round::Serving { .. } => (Some("get ready".to_string()), None),
        Round::Rally => (None, None),
        Round::Over { winner } => (
            Some(format!(
                "{} {} - {}",
                match winner {
                    Side::Left => "you win",
                    Side::Right => "computer wins",
                },
                score.left.max(score.right),
                score.left.min(score.right),
            )),
            Some("space to play again"),
        ),
    };
    // Below the middle, because the middle is where the ball is parked while
    // any of this is worth reading.
    if let Some(headline) = headline {
        ctx.text(
            Vec2::new(-banner.width_of(&headline) * 0.5, 2.4),
            &headline,
            banner,
        );
    }
    if let Some(note) = note {
        ctx.text(
            Vec2::new(-footnote.width_of(note) * 0.5, 4.2),
            note,
            footnote,
        );
    }

    let hint = TextStyle {
        size: 0.6,
        color: Color::rgba(1.0, 1.0, 1.0, 0.28),
        depth: Depth::layer(layers::UI),
    };
    let hint_text = "w/s or up/down";
    ctx.text(
        Vec2::new(
            -hint.width_of(hint_text) * 0.5,
            bottom_right.y - hint.size * 1.6,
        ),
        hint_text,
        hint,
    );
}
