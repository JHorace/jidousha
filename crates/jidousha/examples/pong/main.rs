//! Pong: two paddles, a ball, and a score, written against the jidousha API.
//!
//! W and S move the left paddle. The right paddle is played by the machine.
//! First to five wins; Space serves the next match once one is over.
//!
//! The whole game is shapes and text, so it loads no assets and needs no
//! `Assets` resource at all.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//!
//! The check lives in `verify.rs` beside this file: the same systems and the
//! same config, driven by a controller instead of by hands, asserting on what
//! the world did and on what was drawn. `checks.rs` is the instrument it
//! reports through and `capture.rs` takes the picture.
//!
//! # What this file owes the check
//!
//! Three things here are deliberately pure functions rather than code buried
//! inside a system: `travel_one_tick`, `paddle_contact` and `bounce_off`. The
//! game uses them, and so does the controller in `verify.rs` — which is what
//! lets the controller work out where a ball is going by *replaying the game's
//! own model* rather than by keeping a second copy of it in step by hand.

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod verify;

// --- the court ---------------------------------------------------------

/// How many world units the camera shows from top to bottom.
///
/// The court is smaller than this, so there is a margin of clear space around
/// the playfield at the window's default 16:9.
const VIEW_HEIGHT: f32 = 20.0;

/// The window's opening size, which is also the camera's opening viewport.
const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);

/// Half the playfield, in world units: the ball turns around at these.
///
/// Stated once, because the verify run asserts the ball stays inside it and a
/// check carrying its own copy of the number would keep passing after the
/// court changed shape.
const COURT: Vec2 = Vec2::new(16.0, 9.0);

/// Half a paddle, in world units.
const PADDLE: Vec2 = Vec2::new(0.35, 1.5);

/// How far from the centre line a paddle's own centre sits.
const PADDLE_X: f32 = 14.5;

/// How far a paddle's centre may travel from the middle, so it stays on court.
const PADDLE_LIMIT: f32 = COURT.y - PADDLE.y;

/// The ball's radius, in world units.
const BALL_RADIUS: f32 = 0.45;

// --- how it plays ------------------------------------------------------

/// How fast the ball leaves a serve, in world units per second.
const SERVE_SPEED: f32 = 17.0;

/// How much faster the ball gets with every paddle it touches.
///
/// This is the pace of the whole game and not a cosmetic figure: a rally is
/// only winnable once the ball outruns the opponent, so how fast it climbs is
/// how long a point takes. At 1.1 a point took about thirty seconds of play.
const SPEED_PER_TOUCH: f32 = 2.2;

/// The fastest the ball is ever allowed to go.
///
/// One tick of travel at this speed is 0.45 units against a paddle 0.7 thick,
/// so the ball cannot step through a paddle even without the swept test in
/// `paddle_contact`. The swept test is there anyway, and `verify.rs` asks it
/// its contract directly rather than hoping a rally reaches the case.
const TOP_SPEED: f32 = 27.0;

/// The widest a paddle can throw the ball off the straight.
///
/// Written in degrees because `Radians::from_degrees` is a `const fn` and a
/// hand-typed float near a fraction of pi is a clippy failure.
const MAX_BOUNCE: Radians = Radians::from_degrees(60.0);

/// How fast the player's paddle moves, in world units per second.
///
/// Faster than the steepest ball the game can produce climbs
/// (`TOP_SPEED * sin(MAX_BOUNCE)` is about 23.4), so a good player can always
/// reach the ball and a miss is a decision rather than arithmetic.
const PLAYER_SPEED: f32 = 24.0;

/// How fast the opponent's paddle moves.
///
/// Slow enough that it cannot get from the middle of the court to either end
/// while a ball crosses the half it watches — see `OPPONENT_WATCHES_FROM`,
/// which is the other half of that arithmetic, and the check in `verify.rs`
/// that states it as a requirement rather than as these two numbers.
///
/// Three numbers were tried here and the first two were arrived at by getting
/// the arithmetic wrong, which is worth writing down because the check in
/// `verify.rs` now states it properly.
///
/// 13.0 covered its whole half during any crossing slower than 25.9 units/s —
/// a speed a rally only reaches at the very end of one — so a 3,600-tick match
/// finished 0-0 with a 54-touch rally. 10.0 was worked out against the wrong
/// distance: a defending paddle does not have to reach the ball's line, only to
/// within its own half-height plus the ball's radius of it, which is a third of
/// the court it defends without moving.
const OPPONENT_SPEED: f32 = 7.5;

/// Where the ball has to get to before the opponent starts chasing it, as an X.
///
/// The centre line: until the ball is on its own half the opponent drifts back
/// to the middle instead of tracking. This is what gives it a bounded amount of
/// time to answer a shot, and therefore what makes a shot aimed at the far
/// corner a threat rather than a formality.
const OPPONENT_WATCHES_FROM: f32 = 0.0;

/// How close to the ball's line the opponent tries to sit before it stops
/// correcting, so it does not jitter on the spot.
const OPPONENT_DEADZONE: f32 = 0.25;

/// How many points win a match.
const WIN_SCORE: u32 = 5;

/// How long the ball sits at the centre before a serve, in ticks.
///
/// Ticks rather than seconds: the tick is the canonical timeline, so this is
/// three quarters of a second at the default timestep and stays three quarters
/// of a second's worth of *reading time* whatever the timestep becomes.
const SERVE_PAUSE: u32 = 45;

/// The steepest a serve leaves the centre.
const SERVE_SPREAD: Radians = Radians::from_degrees(35.0);

// --- the look ----------------------------------------------------------

/// The court's colours, named once.
mod palette {
    use jidousha::prelude::Color;

    /// The floor, and the camera's clear colour. Dark, so a white ball reads.
    pub(crate) const COURT: Color = Color::rgb(0.04, 0.06, 0.09);
    /// The border and the centre line.
    pub(crate) const MARKING: Color = Color::rgba(1.0, 1.0, 1.0, 0.10);
    /// The player's paddle.
    pub(crate) const PLAYER: Color = Color::rgb(0.42, 0.92, 1.0);
    /// The opponent's paddle.
    pub(crate) const OPPONENT: Color = Color::rgb(1.0, 0.55, 0.42);
    /// The ball.
    pub(crate) const BALL: Color = Color::rgb(1.0, 1.0, 1.0);
    /// The score.
    pub(crate) const SCORE: Color = Color::rgba(1.0, 1.0, 1.0, 0.85);
    /// The hint line along the bottom.
    pub(crate) const HINT: Color = Color::rgba(0.7, 0.8, 0.9, 0.55);
}

/// The draw bands, so no `layer: 2` is ever typed at a call site.
mod layers {
    /// The court and its markings.
    pub(crate) const FIELD: i16 = -1;
    /// The paddles and the ball.
    pub(crate) const PLAY: i16 = 0;
    /// The score, the hint and the banner, over everything.
    pub(crate) const UI: i16 = 2;
}

/// The score's height in world units.
const SCORE_SIZE: f32 = 2.4;
/// How far the score's centre sits from the centre line.
const SCORE_X: f32 = 6.0;
/// The hint line's height in world units.
const HINT_SIZE: f32 = 0.62;
/// The banner's height in world units.
const BANNER_SIZE: f32 = 1.5;

/// What the bottom of the screen says while a match is being played.
const HINT: &str = "W and S move the left paddle";
/// What it says once one is over.
const HINT_OVER: &str = "press SPACE for another match";

// --- the world ---------------------------------------------------------

/// Which end of the court something belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// The player's end, at negative X.
    Left,
    /// The opponent's end, at positive X.
    Right,
}

impl Side {
    /// Which way this end lies from the centre: -1 for left, +1 for right.
    const fn sign(self) -> f32 {
        match self {
            Side::Left => -1.0,
            Side::Right => 1.0,
        }
    }

    /// The other end.
    const fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// This end's name, for the banner and for a failing check's message.
    const fn name(self) -> &'static str {
        match self {
            Side::Left => "LEFT",
            Side::Right => "RIGHT",
        }
    }
}

/// Who is moving a paddle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    /// The keyboard.
    Keys,
    /// The machine.
    Machine,
}

/// A paddle, and everything about how it moves.
#[derive(Clone, Copy)]
struct Paddle {
    /// Which end it defends.
    side: Side,
    /// Who moves it.
    control: Control,
    /// World units per second.
    speed: f32,
    /// Which way it is being pushed this tick: -1 up, 0 still, +1 down.
    ///
    /// Written by `steer_the_paddles` and read by `move_the_paddles`, so the
    /// decision and the motion are separate systems and the opponent goes
    /// through exactly the same integrator the player does.
    push: f32,
}
impl Component for Paddle {}

/// The ball.
#[derive(Clone, Copy)]
struct Ball {
    /// Where it is going, in world units per second.
    velocity: Vec2,
    /// How fast, kept separately so a bounce can rebuild the direction without
    /// having to recover the magnitude from a vector that has just been
    /// replaced.
    speed: f32,
}
impl Component for Ball {}

/// Where the match has got to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// The ball is waiting at the centre. Counts down and then serves.
    Serving {
        /// Ticks left before the serve.
        ticks_left: u32,
        /// Which end the ball will be served towards.
        toward: Side,
    },
    /// The ball is live.
    Rally,
    /// Somebody reached `WIN_SCORE`.
    Over {
        /// Who did.
        winner: Side,
    },
}

/// The score, the phase, and the numbers a failing check wants to quote.
#[derive(Clone, Copy, Debug)]
struct Scoreboard {
    /// The player's points.
    left: u32,
    /// The opponent's points.
    right: u32,
    /// What the match is doing.
    phase: Phase,
    /// Paddle touches in the rally being played now.
    touches: u32,
    /// The most touches any one rally has had.
    longest_rally: u32,
    /// The fastest the ball has been, in world units per second.
    top_speed: f32,
}
impl Resource for Scoreboard {}

impl Default for Scoreboard {
    fn default() -> Self {
        Scoreboard {
            left: 0,
            right: 0,
            phase: Phase::Serving {
                ticks_left: SERVE_PAUSE,
                toward: Side::Right,
            },
            touches: 0,
            longest_rally: 0,
            top_speed: 0.0,
        }
    }
}

impl Scoreboard {
    /// This side's points.
    const fn points(&self, side: Side) -> u32 {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
        }
    }
}

// --- the model, as functions the check can call ------------------------

/// One tick of a ball's travel with the top and bottom walls, and nothing else.
///
/// Returns where it ends up and which way it is then going. The walls are a
/// reflection rather than a clamp, so a ball that would have gone past one
/// comes back the distance it overshot and the speed is conserved — which is
/// what makes replaying this function forwards a faithful prediction.
///
/// The game steps the ball with this, and so does the controller in `verify.rs`
/// when it works out where a shot will land. One model, two callers.
fn travel_one_tick(position: Vec2, velocity: Vec2, dt: f32) -> (Vec2, Vec2) {
    let moved = position + velocity * dt;
    let (y, turned) = fold_into_court(moved.y);
    let going = if turned {
        Vec2::new(velocity.x, -velocity.y)
    } else {
        velocity
    };
    (Vec2::new(moved.x, y), going)
}

/// Reflect a Y that has gone past a wall back into the court, and say whether
/// it had to.
///
/// Separate from `travel_one_tick` because the controller in `verify.rs` needs
/// it on its own: when it interpolates a ball's position part way through a
/// tick it has to fold that partial Y the same way the game would, or its
/// prediction and the game disagree exactly on the ticks a wall is involved.
fn fold_into_court(y: f32) -> (f32, bool) {
    let wall = COURT.y - BALL_RADIUS;
    if y > wall {
        (2.0 * wall - y, true)
    } else if y < -wall {
        (-2.0 * wall - y, true)
    } else {
        (y, false)
    }
}

/// Where a paddle's face is: the X the ball's leading edge has to reach.
const fn face_of(side: Side) -> f32 {
    side.sign() * (PADDLE_X - PADDLE.x)
}

/// A ball meeting a paddle, part way through a tick.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Contact {
    /// How far into the tick it happened, 0.0 to 1.0.
    at_fraction: f32,
    /// Where the ball's centre was when it happened.
    centre: Vec2,
    /// Where on the paddle it landed: -1.0 at the top end, +1.0 at the bottom.
    offset: f32,
}

/// Did the ball cross this paddle's face during a tick, and did it land on it?
///
/// This is the swept test Concepts asks a game to write for itself: the plane
/// the leading edge touches, whether the ball was approaching it, whether this
/// tick's travel crossed it, and the fraction of the tick at which it did.
///
/// `from` and `to` are the ball's centre before and after a tick of free
/// travel. `paddle_y` is the paddle's centre *after* it moved this tick — the
/// paddle is treated as stationary within the tick, which is the usual
/// prototype simplification and is why a paddle slamming into a ball does not
/// impart anything.
///
/// Returns `None` for all three ways this is not a hit: the ball is going away
/// from this face, the travel did not reach it, or it crossed past the end of
/// the paddle.
fn paddle_contact(from: Vec2, to: Vec2, side: Side, paddle_y: f32) -> Option<Contact> {
    let sign = side.sign();
    let travel = to.x - from.x;
    // Going away from this face, or not going anywhere in X at all.
    if travel * sign <= 0.0 {
        return None;
    }
    let face = face_of(side);
    // The edge of the ball that arrives first.
    let (lead_from, lead_to) = (from.x + sign * BALL_RADIUS, to.x + sign * BALL_RADIUS);
    // Already through it at the start of the tick, or still short at the end.
    if (lead_from - face) * sign > 0.0 || (lead_to - face) * sign < 0.0 {
        return None;
    }
    let at_fraction = ((face - lead_from) / travel).clamp(0.0, 1.0);
    let centre = Vec2::new(
        face - sign * BALL_RADIUS,
        from.y + (to.y - from.y) * at_fraction,
    );
    // Past the end of the paddle: the ball went by, and this is a point.
    let reach = PADDLE.y + BALL_RADIUS;
    let offset = (centre.y - paddle_y) / reach;
    if offset.abs() > 1.0 || offset.is_nan() {
        return None;
    }
    Some(Contact {
        at_fraction,
        centre,
        offset,
    })
}

/// Which way a ball leaves a paddle it struck `offset` of the way along.
///
/// The angle comes from *where* on the paddle it landed rather than from the
/// angle it arrived at, which is Pong's whole control scheme: the middle sends
/// it back flat, the ends throw it away at `MAX_BOUNCE`. `side` is the paddle
/// it bounced off, so the ball leaves in the other direction.
fn bounce_off(side: Side, offset: f32, speed: f32) -> Vec2 {
    let angle = Radians(offset.clamp(-1.0, 1.0) * MAX_BOUNCE.as_f32());
    let (sine, cosine) = sin_cos(angle);
    Vec2::new(-side.sign() * cosine, sine) * speed
}

// --- the game ----------------------------------------------------------

/// The configuration the window and the check share, so what is verified is
/// what a person sees.
fn config() -> GameConfig {
    GameConfig {
        title: "jidousha - pong",
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// Every system, in one place, for the same reason.
fn register(app: &mut App) {
    app.add_system(Startup, set_the_court);
    app.add_system(Update, steer_the_paddles);
    app.add_system(Update, move_the_paddles);
    app.add_system(Update, move_the_ball);
    app.add_system(Update, advance_the_match);
    app.add_system(Draw, draw_the_court);
    app.add_system(Draw, draw_the_players);
    app.add_system(Draw, draw_the_ball);
    app.add_system(Draw, draw_the_readout);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("pong: W and S move the left paddle. first to {WIN_SCORE}. close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        // Display, not Debug: `RunError`'s Display is the engine's four-part
        // message, and `fn main() -> Result<_, RunError>` would print the
        // struct dump instead.
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// The camera, the paddles, the ball and a fresh scoreboard.
fn set_the_court(world: &mut World) {
    world.insert_resource(Camera {
        center: Vec2::ZERO,
        height: VIEW_HEIGHT,
        clear_color: palette::COURT,
        viewport: WINDOW,
    });
    world.insert_resource(Scoreboard::default());

    for (side, control, speed) in [
        (Side::Left, Control::Keys, PLAYER_SPEED),
        (Side::Right, Control::Machine, OPPONENT_SPEED),
    ] {
        let paddle = world.spawn();
        world.insert(
            paddle,
            Transform::at(Vec2::new(side.sign() * PADDLE_X, 0.0)),
        );
        world.insert(
            paddle,
            Paddle {
                side,
                control,
                speed,
                push: 0.0,
            },
        );
    }

    let ball = world.spawn();
    world.insert(ball, Transform::at(Vec2::ZERO));
    world.insert(
        ball,
        Ball {
            velocity: Vec2::ZERO,
            speed: SERVE_SPEED,
        },
    );
}

/// Decide which way each paddle is being pushed this tick.
///
/// The keyboard for one, the machine for the other, both writing the same
/// field. The two-pass shape is here because the machine's decision needs to
/// look at the ball while the query holds the paddles.
fn steer_the_paddles(world: &mut World) {
    // Read pass. `find_resource` because there is no `Input` on the tick
    // `Startup` runs in, and none at all under `headless` unless a check puts
    // one there.
    let keys = match world.find_resource::<Input>() {
        None => 0.0,
        Some(input) => f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    };
    let ball = world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| (transform.pos, ball.velocity))
        .next();

    // Write pass.
    for (_, transform, paddle) in world.query_mut::<(&Transform, &mut Paddle)>() {
        paddle.push = match paddle.control {
            Control::Keys => keys,
            Control::Machine => match ball {
                None => 0.0,
                Some((position, velocity)) => {
                    // Chase the ball when it is coming; drift back to the
                    // middle when it is not. Aiming away from the middle is
                    // therefore what beats this paddle, which is the same
                    // thing that makes it a game rather than a wall.
                    // Coming *and* already on this end of the court. Both
                    // halves matter: the second is what bounds how long the
                    // opponent has to answer, and without it a paddle this slow
                    // still reaches everything.
                    let sign = paddle.side.sign();
                    let watching =
                        velocity.x * sign > 0.0 && position.x * sign > OPPONENT_WATCHES_FROM * sign;
                    let target = if watching { position.y } else { 0.0 };
                    let gap = target - transform.pos.y;
                    if gap.abs() < OPPONENT_DEADZONE {
                        0.0
                    } else {
                        gap.signum()
                    }
                }
            },
        };
    }
}

/// Which way the machine pushes its paddle, given what the ball is doing.
///
/// A function rather than a branch inside `steer_the_paddles`, because the
/// controller in `verify.rs` has to *predict* this paddle in order to aim at
/// all: a shot is only good if it lands somewhere the opponent will not be, and
/// the only honest way to know where it will be is to run its own rule forward.
/// One model, two callers, same as `travel_one_tick`.
fn machine_push(paddle_y: f32, ball: Vec2, velocity: Vec2, side: Side) -> f32 {
    // Coming *and* already on this end of the court. Both halves matter: the
    // second is what bounds how long the opponent has to answer, and without it
    // a paddle this slow still reaches everything.
    let sign = side.sign();
    let watching = velocity.x * sign > 0.0 && ball.x * sign > OPPONENT_WATCHES_FROM * sign;
    let target = if watching { ball.y } else { 0.0 };
    let gap = target - paddle_y;
    if gap.abs() < OPPONENT_DEADZONE {
        0.0
    } else {
        gap.signum()
    }
}

/// One tick of a paddle's motion, clamped to the court.
///
/// Also shared with the controller's prediction, for the same reason.
fn step_paddle(paddle_y: f32, push: f32, speed: f32, dt: f32) -> f32 {
    // Per second, times the timestep, so a paddle keeps its speed if
    // `GameConfig::fixed_dt` ever changes.
    (paddle_y + push * speed * dt).clamp(-PADDLE_LIMIT, PADDLE_LIMIT)
}

/// Move every paddle by its push, and keep it on the court.
fn move_the_paddles(world: &mut World) {
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        transform.pos.y = step_paddle(transform.pos.y, paddle.push, paddle.speed, dt);
    }
}

/// One tick of ball: travel, walls, paddles, and the point at the end of it.
fn move_the_ball(world: &mut World) {
    if !matches!(world.resource::<Scoreboard>().phase, Phase::Rally) {
        return;
    }
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    // Read pass: the paddles, and where the ball is now.
    let paddles: Vec<(Side, f32)> = world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (paddle.side, transform.pos.y))
        .collect();
    let Some((entity, from, mut velocity, mut speed)) = world
        .query::<(&Transform, &Ball)>()
        .map(|(entity, transform, ball)| (entity, transform.pos, ball.velocity, ball.speed))
        .next()
    else {
        return;
    };

    // Free travel for this tick, then ask each paddle whether the ball crossed
    // it on the way. Only one can be hit: they are at opposite ends and the
    // ball crosses at most one face per tick.
    let (mut position, mut going) = travel_one_tick(from, velocity, dt);
    let mut touched = false;
    for (side, paddle_y) in &paddles {
        let Some(contact) = paddle_contact(from, from + velocity * dt, *side, *paddle_y) else {
            continue;
        };
        speed = (speed + SPEED_PER_TOUCH).min(TOP_SPEED);
        velocity = bounce_off(*side, contact.offset, speed);
        // The rest of the tick, travelled the new way, walls and all.
        let (after, going_after) =
            travel_one_tick(contact.centre, velocity, dt * (1.0 - contact.at_fraction));
        position = after;
        going = going_after;
        touched = true;
        break;
    }
    velocity = going;

    world.component_mut::<Transform>(entity).pos = position;
    let ball = world.component_mut::<Ball>(entity);
    ball.velocity = velocity;
    ball.speed = speed;

    let board = world.resource_mut::<Scoreboard>();
    if touched {
        board.touches += 1;
        board.longest_rally = board.longest_rally.max(board.touches);
        board.top_speed = board.top_speed.max(speed);
    }

    // Past the end of the court is a point for the other side.
    if position.x.abs() <= COURT.x + BALL_RADIUS {
        return;
    }
    let conceded = if position.x < 0.0 {
        Side::Left
    } else {
        Side::Right
    };
    let scorer = conceded.other();
    match scorer {
        Side::Left => board.left += 1,
        Side::Right => board.right += 1,
    }
    board.touches = 0;
    board.phase = if board.points(scorer) >= WIN_SCORE {
        Phase::Over { winner: scorer }
    } else {
        // Served towards whoever just conceded, so the loser of the point gets
        // the ball coming at them and the rally starts from the same shape
        // every time.
        Phase::Serving {
            ticks_left: SERVE_PAUSE,
            toward: conceded,
        }
    };
    world.component_mut::<Transform>(entity).pos = Vec2::ZERO;
    let ball = world.component_mut::<Ball>(entity);
    ball.velocity = Vec2::ZERO;
    ball.speed = SERVE_SPEED;
}

/// Count the serve down, serve it, and start a new match when asked.
fn advance_the_match(world: &mut World) {
    let restart = world
        .find_resource::<Input>()
        .is_some_and(|input| input.just_pressed(Key::Space));
    let phase = world.resource::<Scoreboard>().phase;
    match phase {
        Phase::Rally => {}
        Phase::Over { .. } => {
            if restart {
                world.insert_resource(Scoreboard::default());
                let ball = world
                    .query::<(&Transform, &Ball)>()
                    .map(|(entity, _, _)| entity)
                    .next();
                if let Some(ball) = ball {
                    world.component_mut::<Transform>(ball).pos = Vec2::ZERO;
                }
                for (_, transform, _) in world.query_mut::<(&mut Transform, &Paddle)>() {
                    transform.pos.y = 0.0;
                }
            }
        }
        Phase::Serving { ticks_left, toward } => {
            if ticks_left > 0 {
                world.resource_mut::<Scoreboard>().phase = Phase::Serving {
                    ticks_left: ticks_left - 1,
                    toward,
                };
                return;
            }
            // A serve is a bounce off an imaginary paddle at the other end,
            // struck somewhere across its middle: the same function, so a
            // serve can never produce an angle a rally could not.
            let spread = SERVE_SPREAD.as_f32() / MAX_BOUNCE.as_f32();
            let roll = world.resource_mut::<Rng>().next_f32();
            let offset = (roll * 2.0 - 1.0) * spread;
            let velocity = bounce_off(toward.other(), offset, SERVE_SPEED);
            world.resource_mut::<Scoreboard>().phase = Phase::Rally;
            let ball = world
                .query::<(&Transform, &Ball)>()
                .map(|(entity, _, _)| entity)
                .next();
            let Some(ball) = ball else { return };
            let ball = world.component_mut::<Ball>(ball);
            ball.velocity = velocity;
            ball.speed = SERVE_SPEED;
        }
    }
}

// --- drawing -----------------------------------------------------------

/// The border and the dashed centre line.
fn draw_the_court(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::FIELD);
    let court = Rect::from_center_size(Vec2::ZERO, COURT * 2.0);
    for (from, to) in [
        (court.min, Vec2::new(court.max.x, court.min.y)),
        (Vec2::new(court.min.x, court.max.y), court.max),
    ] {
        ctx.line(from, to, 0.12, palette::MARKING, depth);
    }
    // A dashed centre line, out of rectangles: there is no dashed-line verb and
    // no circle outline, so a Pong centre marking is drawn rather than asked
    // for.
    let dash = Vec2::new(0.14, 0.9);
    let mut y = -COURT.y + 0.55;
    while y < COURT.y {
        ctx.rect(
            Rect::from_center_size(Vec2::new(0.0, y), dash),
            palette::MARKING,
            depth,
        );
        y += 1.6;
    }
}

/// Both paddles, from the world rather than from a constant.
fn draw_the_players(ctx: &mut DrawCtx) {
    for (_, transform, paddle) in ctx.world.query::<(&Transform, &Paddle)>() {
        let colour = match paddle.control {
            Control::Keys => palette::PLAYER,
            Control::Machine => palette::OPPONENT,
        };
        ctx.rect(
            Rect::from_center_size(transform.pos, PADDLE * 2.0),
            colour,
            Depth::layer(layers::PLAY),
        );
    }
}

/// The ball, drawn wherever it is — including at rest under the winner's
/// banner, which is where a decided match leaves it.
///
/// Hiding it on that screen would be tidier, and it is drawn there on purpose:
/// the banner over the ball is the only place in the whole game where two draw
/// bands cover the same point. Without that overlap, moving the banner out of
/// the UI band changes nothing any assertion over drawn quads can see — a frame
/// records the order the quads went down in and not the `Depth` that produced
/// it, and the banner is submitted last either way.
fn draw_the_ball(ctx: &mut DrawCtx) {
    for (_, transform, _) in ctx.world.query::<(&Transform, &Ball)>() {
        ctx.circle(
            transform.pos,
            BALL_RADIUS,
            palette::BALL,
            Depth::layer(layers::PLAY),
        );
    }
}

/// What the banner says when a match is over.
///
/// A function rather than a `format!` inside the Draw system, because the check
/// asserts every string this game draws is printable ASCII — the font draws a
/// box for anything else, at exactly a letter's width, so no assertion over
/// drawn quads can tell a curly quote from a letter.
fn banner_text(winner: Side, left: u32, right: u32) -> String {
    format!("{} WINS {left}-{right}", winner.name())
}

/// The score, the hint, and the banner when there is one.
///
/// Every line is centred by its own `width_of` rather than one call with a
/// `\n` in it: `width_of` measures only the widest line of a block, so a
/// two-line banner centred in one call hangs its shorter line off to the left.
fn draw_the_readout(ctx: &mut DrawCtx) {
    let board = *ctx.world.resource::<Scoreboard>();
    let score = TextStyle {
        size: SCORE_SIZE,
        color: palette::SCORE,
        depth: Depth::layer(layers::UI),
    };
    for (side, points) in [(Side::Left, board.left), (Side::Right, board.right)] {
        let text = format!("{points}");
        ctx.text(
            Vec2::new(
                side.sign() * SCORE_X - score.width_of(&text) * 0.5,
                -COURT.y + 0.7,
            ),
            &text,
            score,
        );
    }

    let hint = TextStyle {
        size: HINT_SIZE,
        color: palette::HINT,
        depth: Depth::layer(layers::UI),
    };
    let line = match board.phase {
        Phase::Over { .. } => HINT_OVER,
        _ => HINT,
    };
    ctx.text(
        Vec2::new(-hint.width_of(line) * 0.5, COURT.y - 1.35),
        line,
        hint,
    );

    let Phase::Over { winner } = board.phase else {
        return;
    };
    let banner = TextStyle {
        size: BANNER_SIZE,
        color: palette::BALL,
        depth: Depth::layer(layers::UI),
    };
    let headline = banner_text(winner, board.left, board.right);
    ctx.text(
        Vec2::new(-banner.width_of(&headline) * 0.5, -BANNER_SIZE * 0.5),
        &headline,
        banner,
    );
}
