//! Pong. Two paddles, a ball, a score, and a winner.
//!
//! W and S move the left paddle. The right paddle plays itself. First to five
//! wins; Enter starts the next match.
//!
//! Everything here is drawn by the engine — a paddle is a rectangle, the ball
//! is a circle, the score is text — so there is no art to load and no asset
//! story to tell.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//!
//! The check lives in `verify.rs` beside this file: the same systems and the
//! same config, driven by a controller instead of a person, asserting on what
//! the world did and on what was drawn, with no window anywhere.

use std::process::ExitCode;

use jidousha::prelude::*;

mod verify;

// --- the field ------------------------------------------------------------

/// How many world units the camera spans vertically.
const VIEW_HEIGHT: f32 = 20.0;

/// The top wall the ball bounces off. Y is down, so this is the smaller number.
const FIELD_TOP: f32 = -9.0;

/// The bottom wall the ball bounces off.
const FIELD_BOTTOM: f32 = 9.0;

/// How far past the centre a ball has to get before it counts as a point.
///
/// Inside the camera's horizontal half-width (17.7 at 16:9 and this view
/// height), so the ball is never drawn off the edge of the screen on its way
/// out.
const FIELD_EDGE: f32 = 17.0;

// --- the pieces -----------------------------------------------------------

/// How big a paddle is, in world units.
const PADDLE_SIZE: Vec2 = Vec2::new(0.8, 3.6);

/// How far from the centre line each paddle stands.
const PADDLE_X: f32 = 15.0;

/// How far from the centre a paddle may travel before the wall stops it.
const PADDLE_LIMIT: f32 = FIELD_BOTTOM - PADDLE_SIZE.y * 0.5;

/// The ball's radius, in world units.
const BALL_RADIUS: f32 = 0.45;

// --- how it plays ---------------------------------------------------------

/// The player's paddle speed, in world units per second.
const PLAYER_SPEED: f32 = 24.0;

/// The opponent's paddle speed, in world units per second.
///
/// Slower than the player's on purpose: it is the whole of the difficulty
/// setting, and the number the game was tuned on.
const AI_SPEED: f32 = 11.0;

/// How fast the opponent drifts back to the middle when the ball is elsewhere.
const AI_RECOVER_SPEED: f32 = 6.0;

/// The opponent does not commit to a ball until it is this far across.
///
/// Y is the same either way, so a machine that starts tracking at the moment of
/// the return is a machine that cannot be beaten. Making it wait is what leaves
/// room for a shot to the far corner.
const AI_REACT_X: f32 = 2.0;

/// How fast a serve leaves the centre, in world units per second.
const SERVE_SPEED: f32 = 19.0;

/// How fast the ball is ever allowed to travel, in world units per second.
///
/// Collisions are tested at tick boundaries and nothing sweeps the ball's own
/// shape against the walls, so this is also the tunnelling budget: at 60 ticks
/// a second this is half a world unit per tick, comfortably inside a paddle's
/// 0.8-unit thickness. `assert_the_ball_cannot_tunnel` checks it against the
/// timestep the engine actually hands us rather than against the 1/60 assumed
/// here.
const MAX_BALL_SPEED: f32 = 34.0;

/// How much faster the ball gets with every paddle it touches.
const SPEED_GAIN: f32 = 1.08;

/// How far off straight a paddle can send the ball, hit right at its tip.
const MAX_BOUNCE: Radians = Radians(0.95);

/// How far off straight a serve can leave the centre.
const SERVE_SPREAD: Radians = Radians(0.45);

/// How long the ball waits at the centre between points, in ticks.
///
/// Ticks, not seconds: the tick is the canonical timeline, and three quarters
/// of a second is what forty-five of them are.
const SERVE_TICKS: u32 = 45;

/// How many points win a match.
const WIN_SCORE: u32 = 5;

/// Draw bands, named once so no number is guessed at a call site.
mod layers {
    /// The field and its markings, behind everything.
    pub const FIELD: i16 = -1;
    /// The paddles and the ball.
    pub const PLAY: i16 = 0;
    /// Score, hints and banners, over everything.
    pub const UI: i16 = 2;
}

// --- what the world is made of --------------------------------------------

/// Which end of the field something belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// The player's end.
    Left,
    /// The opponent's end.
    Right,
}

impl Side {
    /// Which way along X this side's paddle sends the ball.
    fn facing(self) -> f32 {
        match self {
            Side::Left => 1.0,
            Side::Right => -1.0,
        }
    }

    /// The side's name, for banners and for assertion messages.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Side::Left => "LEFT",
            Side::Right => "RIGHT",
        }
    }
}

/// A paddle: which end it defends, and how fast it can move.
#[derive(Clone, Copy)]
pub(crate) struct Paddle {
    /// Which end of the field it stands at.
    pub(crate) side: Side,
    /// World units per second.
    speed: f32,
}
impl Component for Paddle {}

/// The ball, and where it is going.
#[derive(Clone, Copy)]
pub(crate) struct Ball {
    /// World units per second, as a vector.
    pub(crate) velocity: Vec2,
}
impl Component for Ball {}

/// Which screen the game is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// The ball is waiting at the centre for this many more ticks.
    Serving {
        /// Ticks still to wait.
        ticks_left: u32,
        /// Who the serve will head towards.
        toward: Side,
    },
    /// The ball is in play.
    Rally,
    /// Somebody reached `WIN_SCORE`.
    Over {
        /// Who won.
        winner: Side,
    },
}

/// The match: the score, the screen, and the numbers a check wants to read.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MatchState {
    /// Which screen the game is on.
    pub(crate) stage: Stage,
    /// The player's points.
    pub(crate) left: u32,
    /// The opponent's points.
    pub(crate) right: u32,
    /// Paddle touches in the rally currently being played.
    pub(crate) rally: u32,
    /// The most touches any one rally has had.
    pub(crate) longest_rally: u32,
    /// The fastest the ball has ever gone, in world units per second.
    pub(crate) top_speed: f32,
}
impl Resource for MatchState {}

impl MatchState {
    /// A match nobody has scored in yet, about to serve towards `toward`.
    fn fresh(toward: Side) -> Self {
        MatchState {
            stage: Stage::Serving {
                ticks_left: SERVE_TICKS,
                toward,
            },
            left: 0,
            right: 0,
            rally: 0,
            longest_rally: 0,
            top_speed: 0.0,
        }
    }

    /// This side's points.
    pub(crate) fn points(&self, side: Side) -> u32 {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
        }
    }
}

// --- setup ----------------------------------------------------------------

/// The game's configuration, shared by the window and the check, so that what
/// is verified is what a person plays.
pub(crate) fn config() -> GameConfig {
    GameConfig {
        title: "jidousha — pong",
        ..GameConfig::default()
    }
}

/// Every system this game has, in one place.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, steer_the_paddles);
    app.add_system(Update, move_the_ball);
    app.add_system(Update, keep_score);
    app.add_system(Draw, draw_the_field);
    app.add_system(Draw, draw_the_play);
    app.add_system(Draw, draw_the_readout);
}

fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        clear_color: Color::rgb(0.04, 0.05, 0.08),
        height: VIEW_HEIGHT,
        ..Camera::default()
    });
    world.insert_resource(MatchState::fresh(Side::Right));

    for side in [Side::Left, Side::Right] {
        let paddle = world.spawn();
        let x = match side {
            Side::Left => -PADDLE_X,
            Side::Right => PADDLE_X,
        };
        world.insert(paddle, Transform::at(Vec2::new(x, 0.0)));
        world.insert(
            paddle,
            Paddle {
                side,
                speed: match side {
                    Side::Left => PLAYER_SPEED,
                    Side::Right => AI_SPEED,
                },
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

    assert_the_ball_cannot_tunnel(world.resource::<Time>().fixed_dt);
}

/// The one thing about this game that a change to `fixed_dt` could break.
///
/// Nothing in v1 sweeps, so a ball that moves further in one tick than a paddle
/// is thick steps clean through it. The margin is checked against the timestep
/// the engine handed us, not against the 1/60 the constants were picked for.
fn assert_the_ball_cannot_tunnel(fixed_dt: Seconds) {
    let step = MAX_BALL_SPEED * fixed_dt.as_f32();
    assert!(
        step < PADDLE_SIZE.x,
        "{}",
        message(
            "the ball can move further in one tick than a paddle is thick",
            &format!(
                "{MAX_BALL_SPEED} units/s at a {} s timestep is {step:.3} units per tick, \
                 against a paddle {} units thick",
                fixed_dt.as_f32(),
                PADDLE_SIZE.x
            ),
            "GameConfig::fixed_dt got longer, or MAX_BALL_SPEED got bigger",
            "lower MAX_BALL_SPEED, or make PADDLE_SIZE.x wider than one tick of travel",
        )
    );
}

// --- simulation -----------------------------------------------------------

/// Move both paddles: the left one from the keyboard, the right one by itself.
fn steer_the_paddles(world: &mut World) {
    // Read everything first, write second: a query that borrows the world
    // mutably holds it for as long as it is iterated.
    let player = match world.find_resource::<Input>() {
        // The first tick of a run happens before any input exists.
        None => 0.0,
        Some(input) => f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    };
    let ball = find_ball(world).map(|(_, pos, velocity)| (pos, velocity));
    let dt = world.resource::<Time>().fixed_dt.as_f32();

    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        let reach = paddle.speed * dt;
        let step = match paddle.side {
            Side::Left => player * reach,
            Side::Right => match ball {
                // Chase the ball once it is on its way over; otherwise ease
                // back towards the middle, slowly.
                Some((pos, velocity)) if velocity.x > 0.0 && pos.x > AI_REACT_X => {
                    (pos.y - transform.pos.y).clamp(-reach, reach)
                }
                _ => {
                    let recover = AI_RECOVER_SPEED * dt;
                    (-transform.pos.y).clamp(-recover, recover)
                }
            },
        };
        transform.pos.y = (transform.pos.y + step).clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
    }
}

/// Advance the ball by one tick, bouncing it off the walls and the paddles.
fn move_the_ball(world: &mut World) {
    if world.resource::<MatchState>().stage != Stage::Rally {
        return;
    }
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    let paddles: Vec<(Side, Vec2)> = world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (paddle.side, transform.pos))
        .collect();

    let mut touched = None;
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        let step = advance(transform.pos, ball.velocity, dt, &paddles);
        transform.pos = step.pos;
        ball.velocity = step.velocity;
        touched = step.touched;
    }

    let state = world.resource_mut::<MatchState>();
    if touched.is_some() {
        state.rally += 1;
        state.longest_rally = state.longest_rally.max(state.rally);
    }
}

/// Where one tick of travel puts the ball, and how fast it is going afterwards.
struct Step {
    /// The new position.
    pos: Vec2,
    /// The new velocity.
    velocity: Vec2,
    /// Which paddle it bounced off, if any.
    touched: Option<Side>,
}

/// One tick of ball travel: a swept test against each paddle, then the walls.
///
/// Swept rather than an overlap test between two rectangles, because an overlap
/// test only sees the ball on ticks where it happens to be *inside* the paddle,
/// and a ball moving half a unit a tick against a paddle 0.8 units thick spends
/// at most one tick there. The crossing test asks the question that actually
/// matters — did the ball pass through the plane the paddle's face lies on, and
/// was the paddle there when it did.
fn advance(from: Vec2, velocity: Vec2, dt: f32, paddles: &[(Side, Vec2)]) -> Step {
    let mut velocity = velocity;
    let mut to = from + velocity * dt;
    let mut touched = None;

    for &(side, centre) in paddles {
        let facing = side.facing();
        // The plane the ball's edge touches, on the side it arrives from.
        let plane = centre.x + facing * (PADDLE_SIZE.x * 0.5 + BALL_RADIUS);
        let approaching = velocity.x * facing < 0.0;
        let crossed = (from.x - plane) * facing >= 0.0 && (to.x - plane) * facing < 0.0;
        if !approaching || !crossed {
            continue;
        }
        let travelled = from.x - to.x;
        let when = if travelled.abs() > f32::EPSILON {
            ((from.x - plane) / travelled).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let where_y = from.y + (to.y - from.y) * when;
        let reach = PADDLE_SIZE.y * 0.5 + BALL_RADIUS;
        let offset = (where_y - centre.y) / reach;
        if offset.abs() > 1.0 {
            continue;
        }

        // Off the tip of the paddle is a steep return; off the middle is flat.
        // That is the whole of Pong's aiming, and the reason a rally can go
        // anywhere.
        let speed = (velocity.length() * SPEED_GAIN).min(MAX_BALL_SPEED);
        let (sine, cosine) = sin_cos(Radians(offset * MAX_BOUNCE.0));
        velocity = Vec2::new(facing * speed * cosine, speed * sine);
        to = Vec2::new(plane, where_y) + velocity * dt * (1.0 - when);
        touched = Some(side);
        break;
    }

    // Walls last, so a ball that hit a paddle in the corner still ends the tick
    // on the field.
    let top = FIELD_TOP + BALL_RADIUS;
    let bottom = FIELD_BOTTOM - BALL_RADIUS;
    if to.y < top {
        to.y = top + (top - to.y);
        velocity.y = -velocity.y;
    } else if to.y > bottom {
        to.y = bottom - (to.y - bottom);
        velocity.y = -velocity.y;
    }
    to.y = to.y.clamp(top, bottom);

    Step {
        pos: to,
        velocity,
        touched,
    }
}

/// Count the points, run the serve clock, and start the next match on Enter.
fn keep_score(world: &mut World) {
    match world.resource::<MatchState>().stage {
        Stage::Serving { ticks_left, toward } => {
            if ticks_left > 1 {
                world.resource_mut::<MatchState>().stage = Stage::Serving {
                    ticks_left: ticks_left - 1,
                    toward,
                };
                return;
            }
            // The engine's RNG is seeded from `GameConfig`, so the same run
            // serves the same way every time and a check can replay it.
            let spread = {
                let rng = world.resource_mut::<Rng>();
                (rng.next_f32() - 0.5) * 2.0 * SERVE_SPREAD.0
            };
            let (sine, cosine) = sin_cos(Radians(spread));
            let facing = match toward {
                Side::Left => -1.0,
                Side::Right => 1.0,
            };
            put_the_ball(
                world,
                Vec2::ZERO,
                Vec2::new(facing * SERVE_SPEED * cosine, SERVE_SPEED * sine),
            );
            let state = world.resource_mut::<MatchState>();
            state.stage = Stage::Rally;
            state.rally = 0;
        }
        Stage::Rally => {
            let Some((_, pos, velocity)) = find_ball(world) else {
                return;
            };
            let speed = velocity.length();
            {
                let state = world.resource_mut::<MatchState>();
                state.top_speed = state.top_speed.max(speed);
            }
            let scorer = if pos.x > FIELD_EDGE {
                Side::Left
            } else if pos.x < -FIELD_EDGE {
                Side::Right
            } else {
                return;
            };
            put_the_ball(world, Vec2::ZERO, Vec2::ZERO);
            let state = world.resource_mut::<MatchState>();
            match scorer {
                Side::Left => state.left += 1,
                Side::Right => state.right += 1,
            }
            state.stage = if state.points(scorer) >= WIN_SCORE {
                Stage::Over { winner: scorer }
            } else {
                // Serve towards whoever just conceded.
                Stage::Serving {
                    ticks_left: SERVE_TICKS,
                    toward: match scorer {
                        Side::Left => Side::Right,
                        Side::Right => Side::Left,
                    },
                }
            };
        }
        Stage::Over { winner } => {
            let again = world
                .find_resource::<Input>()
                .is_some_and(|input| input.just_pressed(Key::Enter));
            if again {
                put_the_ball(world, Vec2::ZERO, Vec2::ZERO);
                let top_speed = world.resource::<MatchState>().top_speed;
                let mut fresh = MatchState::fresh(winner);
                // Carry the fastest ball across matches: it is a fact about the
                // session, not about one match.
                fresh.top_speed = top_speed;
                world.insert_resource(fresh);
            }
        }
    }
}

/// The ball's entity, position and velocity, if there is one.
fn find_ball(world: &World) -> Option<(Entity, Vec2, Vec2)> {
    world
        .query::<(&Transform, &Ball)>()
        .map(|(entity, transform, ball)| (entity, transform.pos, ball.velocity))
        .next()
}

/// Put the ball somewhere, going somewhere.
fn put_the_ball(world: &mut World, pos: Vec2, velocity: Vec2) {
    let Some((ball, _, _)) = find_ball(world) else {
        return;
    };
    world.component_mut::<Transform>(ball).pos = pos;
    world.component_mut::<Ball>(ball).velocity = velocity;
}

// --- drawing --------------------------------------------------------------

/// The walls, the sidelines and the net.
fn draw_the_field(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::FIELD);
    let chalk = Color::rgba(1.0, 1.0, 1.0, 0.12);

    for y in [FIELD_TOP, FIELD_BOTTOM] {
        ctx.line(
            Vec2::new(-FIELD_EDGE, y),
            Vec2::new(FIELD_EDGE, y),
            0.2,
            chalk,
            depth,
        );
    }

    // The net, as a column of dashes — the thing that makes it read as Pong.
    let dash = 0.7;
    let mut y = FIELD_TOP + dash * 0.5;
    while y < FIELD_BOTTOM {
        ctx.rect(
            Rect::from_center_size(Vec2::new(0.0, y), Vec2::new(0.16, dash)),
            chalk,
            depth,
        );
        y += dash * 2.0;
    }
}

/// The paddles and the ball, read straight out of the world.
fn draw_the_play(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PLAY);
    let paddles: Vec<(Side, Vec2)> = ctx
        .world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (paddle.side, transform.pos))
        .collect();
    for (side, at) in paddles {
        let color = match side {
            Side::Left => Color::rgb(0.45, 0.95, 0.75),
            Side::Right => Color::rgb(0.95, 0.6, 0.45),
        };
        ctx.rect(Rect::from_center_size(at, PADDLE_SIZE), color, depth);
    }

    let balls: Vec<Vec2> = ctx
        .world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, _)| transform.pos)
        .collect();
    for at in balls {
        ctx.circle(at, BALL_RADIUS, Color::WHITE, depth);
    }
}

/// The score, the hint line, and whatever banner the current screen wants.
fn draw_the_readout(ctx: &mut DrawCtx) {
    let state = *ctx.world.resource::<MatchState>();

    let score = TextStyle {
        size: 2.2,
        color: Color::rgba(1.0, 1.0, 1.0, 0.75),
        depth: Depth::layer(layers::UI),
    };
    // Centred by measuring: `width_of` is exact, so this lines up rather than
    // nearly lining up.
    centred(
        ctx,
        &format!("{}   {}", state.left, state.right),
        score,
        FIELD_TOP + 0.5,
    );

    let hint = TextStyle {
        size: 0.7,
        color: Color::rgba(0.7, 0.85, 1.0, 0.55),
        depth: Depth::layer(layers::UI),
    };
    // Inside the bottom wall rather than under it: `bottom_right.y - 1.3` puts
    // the line where the camera ends, which is on top of the wall the field is
    // drawn with. The field's own edge is the thing to hang UI off.
    ctx.text(
        Vec2::new(-FIELD_EDGE + 0.3, FIELD_BOTTOM - 1.05),
        &format!("W / S - move       first to {WIN_SCORE}"),
        hint,
    );

    match state.stage {
        Stage::Serving { ticks_left, toward } => {
            let ready = TextStyle {
                size: 0.9,
                color: Color::rgba(1.0, 1.0, 1.0, 0.5),
                depth: Depth::layer(layers::UI),
            };
            centred(
                ctx,
                &format!("serving {} in {}", toward.name(), ticks_left / 15 + 1),
                ready,
                -3.0,
            );
        }
        Stage::Rally => {}
        Stage::Over { winner } => {
            let banner = TextStyle {
                size: 2.0,
                color: Color::WHITE,
                depth: Depth::layer(layers::UI),
            };
            centred(ctx, &format!("{} WINS", winner.name()), banner, -3.2);
            let again = TextStyle {
                size: 0.9,
                color: Color::rgba(1.0, 1.0, 1.0, 0.6),
                depth: Depth::layer(layers::UI),
            };
            centred(ctx, "press ENTER to play again", again, 1.6);
        }
    }
}

/// Draw one line of text centred on the middle of the field, with its top at
/// `top`.
///
/// `ctx.text` places the first character's *top-left* corner, so centring is
/// half the measured width to the left and nothing at all vertically.
fn centred(ctx: &mut DrawCtx, text: &str, style: TextStyle, top: f32) {
    ctx.text(Vec2::new(-style.width_of(text) * 0.5, top), text, style);
}

// --- entry point ----------------------------------------------------------

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        verify::run();
        return ExitCode::SUCCESS;
    }
    println!("W and S move the left paddle. first to {WIN_SCORE}. close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        // Print the error rather than returning it: `RunError`'s `Display` is
        // the engine's four-part message, and `Debug` is a struct dump.
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
