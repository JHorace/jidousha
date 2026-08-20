//! Pong. Two paddles, a ball, a score, and a winner at five.
//!
//! W and S move the left paddle. The right paddle is the machine's. First to
//! five wins; space starts the next match.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//!
//! # The layout is in constants, and it is a layout for 16:9
//!
//! `GameConfig::window_size` opens at 1280x720 and the camera is
//! [`VIEW_HEIGHT`] world units tall, so the court is [`HALF_WIDTH`] units
//! either side of the centre and every position below is named rather than
//! computed from `Camera::visible_bounds()`. That buys a headless run with
//! exactly one viewport, so every bounds assertion in `verify.rs` is about the
//! shape a player actually sees. What it costs is the drag: pull the window
//! narrower than 16:9 and the goal lines move inside the paddles, and no check
//! can see it because there is no second viewport to run.
//!
//! # Where the ball is when the paddle is asked about it
//!
//! DELIBERATE, and stated here because nothing enforces it: both paddles move
//! *before* the ball does, and [`move_the_ball`]'s sweep treats each paddle as
//! a plane standing still at its **post-move** position for the whole tick.
//! That is wrong by at most one tick of paddle travel and right about the case
//! that matters — a paddle closing on the ball catches it rather than being
//! passed through. The decision is the sequence of `add_system` calls in
//! [`register`] and a tidy-up that reorders them reverses it silently, so
//! `verify.rs` asserts the order out of `HeadlessSim::schedule_debug`.

use std::process::ExitCode;

use jidousha::prelude::*;

// --- the court ---------------------------------------------------------

/// How many world units the camera spans vertically.
const VIEW_HEIGHT: f32 = 20.0;

/// The window's aspect, which the layout below is written for.
const VIEW_ASPECT: f32 = 16.0 / 9.0;

/// Half the camera's width, in world units — the far edge of the screen.
const HALF_WIDTH: f32 = VIEW_HEIGHT * VIEW_ASPECT * 0.5;

/// Half the court's width: the goal line, and the border drawn on it.
const COURT_HALF_X: f32 = 16.5;

/// Half the court's height: the wall the ball bounces off.
const COURT_HALF_Y: f32 = 7.0;

/// What the court is cleared to.
///
/// Named because `verify.rs` asserts it two ways: against this constant, and
/// against the requirement it exists to meet — a white ball has to read on it.
const COURT: Color = Color::rgb(0.04, 0.06, 0.09);

/// What the court's markings are drawn in.
///
/// Alpha blends in linear light, so this reads far brighter than 0.14 looks.
/// Picked from a capture rather than by arithmetic.
const MARKING: Color = Color::rgba(1.0, 1.0, 1.0, 0.14);

/// How thick the court's border is, in world units.
const BORDER_THICKNESS: f32 = 0.14;

/// How tall one dash of the centre line is, in world units.
const DASH_HEIGHT: f32 = 0.9;

/// How wide one dash of the centre line is, in world units.
const DASH_WIDTH: f32 = 0.22;

/// How many dashes the centre line is made of.
///
/// Odd, so one dash sits on the centre of the court rather than a gap.
const DASH_COUNT: i32 = 9;

// --- the paddles -------------------------------------------------------

/// How big a paddle is, in world units.
///
/// The X component is also the thinnest thing the ball must not step through,
/// which is what [`MAX_SPEED`] is capped against.
const PADDLE_SIZE: Vec2 = Vec2::new(0.9, 3.4);

/// How far from the centre a paddle stands, in world units.
const PADDLE_X: f32 = 15.3;

/// How far from the centre a paddle may travel, in world units.
///
/// Exactly far enough that a paddle's end reaches the wall and no further.
const PADDLE_TRAVEL: f32 = COURT_HALF_Y - PADDLE_SIZE.y * 0.5;

/// What a paddle is drawn in.
const PADDLE_COLOR: Color = Color::rgb(0.85, 0.90, 1.0);

/// How fast the player's paddle moves, in world units per second.
const PLAYER_SPEED: f32 = 18.0;

/// How fast the opponent's paddle moves, in world units per second.
///
/// Slower than the player's on purpose: this is the whole of the difficulty.
const OPPONENT_SPEED: f32 = 15.0;

/// Where the ball has to be before the opponent starts tracking it.
///
/// Until then it drifts back to the middle. A machine that tracked from the
/// moment the ball left the player's paddle would have the length of the court
/// to line itself up and would never miss.
const OPPONENT_WAKES_AT: f32 = 2.0;

/// How far off its centre the opponent tries to meet the ball, in world units.
///
/// Not zero, and that is the whole difference between a match and a groove. An
/// opponent that centres on the ball returns it dead flat down the middle, and
/// against anybody who also centres — which is what a person does on their
/// first try — the rally has nowhere to go and neither side can ever score.
/// Meeting the ball off-centre is the opponent playing to win, and it is what
/// makes the game a game.
const OPPONENT_AIM: f32 = 1.2;

// --- the ball ----------------------------------------------------------

/// How big the ball is, as a radius in world units.
const BALL_RADIUS: f32 = 0.42;

/// What the ball is drawn in.
const BALL_COLOR: Color = Color::rgb(1.0, 0.95, 0.6);

/// How fast a serve leaves the centre, in world units per second.
const SERVE_SPEED: f32 = 20.0;

/// How much faster the ball gets with every paddle touch, in units per second.
const SPEED_GAIN: f32 = 3.5;

/// The fastest the ball may ever go, in world units per second.
///
/// The cap exists so the ball cannot step clean through a paddle: nothing in
/// v1 sweeps for you, and one tick of travel at this speed is 0.83 units
/// against a paddle 0.9 thick. `verify.rs` asserts that against the `fixed_dt`
/// the engine actually hands the game rather than against the 1/60 assumed
/// here.
const MAX_SPEED: f32 = 50.0;

/// How far off straight a paddle can send the ball, hit at its very end.
const MAX_BOUNCE: Radians = Radians::from_degrees(58.0);

/// How far off straight a serve can leave the centre.
const SERVE_SPREAD: Radians = Radians::from_degrees(20.0);

/// How long the ball sits at the centre before a serve, in ticks.
///
/// Ticks rather than seconds: the tick is the canonical timeline, and 45 of
/// them is three quarters of a second at the default timestep.
const SERVE_PAUSE: u32 = 45;

/// How many points win the match.
const WIN_SCORE: u32 = 5;

// --- the words on the screen -------------------------------------------

/// How tall the score is, in world units.
const SCORE_SIZE: f32 = 1.8;

/// Where the top of the score sits, in world units.
///
/// In the top third of what the camera shows, which is the requirement rather
/// than the constant — `verify.rs` states it the first way.
const SCORE_TOP: f32 = -9.2;

/// How far either side of the centre line a score digit is set.
const SCORE_INSET: f32 = 2.2;

/// How tall the hint at the foot of the screen is, in world units.
const HINT_SIZE: f32 = 0.62;

/// Where the top of the hint sits, in world units.
const HINT_TOP: f32 = 8.7;

/// The hint at the foot of the screen.
const HINT: &str = "w and s move the left paddle";

/// How tall the winner's banner is, in world units.
const BANNER_SIZE: f32 = 1.5;

/// Where the top of the winner's banner sits, in world units.
const BANNER_TOP: f32 = -1.6;

/// The line under the winner's banner.
const BANNER_HINT: &str = "press space to play again";

/// How tall the line under the winner's banner is, in world units.
const BANNER_HINT_SIZE: f32 = 0.8;

/// Where the top of the line under the winner's banner sits.
const BANNER_HINT_TOP: f32 = 0.4;

/// Draw bands, named once so no `layer: 2` appears anywhere below.
mod layers {
    /// The court and its markings, behind the game.
    pub(crate) const FIELD: i16 = -1;
    /// The paddles and the ball: the things the game is about.
    pub(crate) const PLAY: i16 = 0;
    /// The score, the hint and the winner's banner, over everything.
    pub(crate) const UI: i16 = 2;
}

// --- the world ---------------------------------------------------------

/// Which end of the court something belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    /// The end the player defends.
    Left,
    /// The end the machine defends.
    Right,
}

impl Side {
    /// The sign of this side's X: `-1.0` on the left, `+1.0` on the right.
    pub(crate) const fn sign(self) -> f32 {
        match self {
            Side::Left => -1.0,
            Side::Right => 1.0,
        }
    }

    /// The other end of the court.
    const fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// What this side is called on the winner's banner.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Side::Left => "you win",
            Side::Right => "the machine wins",
        }
    }
}

/// Who moves a paddle.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    /// The keyboard.
    Player,
    /// [`drive_the_opponent`].
    Opponent,
}

/// A paddle: which end it defends, who moves it, and how fast.
#[derive(Clone, Copy)]
pub(crate) struct Paddle {
    /// Which end of the court it stands at.
    pub(crate) side: Side,
    /// Who moves it.
    control: Control,
    /// How fast it moves, in world units per second.
    speed: f32,
}
impl Component for Paddle {}

/// The ball, so a query can name it.
#[derive(Clone, Copy)]
pub(crate) struct Ball;
impl Component for Ball {}

/// How fast something is moving, in world units per second.
#[derive(Clone, Copy)]
pub(crate) struct Velocity(pub(crate) Vec2);
impl Component for Velocity {}

/// The match: the score, the serve timer, and what the run is worth reporting.
pub(crate) struct Scoreboard {
    /// The player's points.
    pub(crate) left: u32,
    /// The machine's points.
    pub(crate) right: u32,
    /// Ticks left before the next serve; zero while the ball is live.
    pub(crate) serve_in: u32,
    /// Which way the next serve travels.
    serve_to: Side,
    /// Who has won, once anybody has.
    pub(crate) winner: Option<Side>,
    /// Paddle touches in the rally being played.
    touches: u32,
    /// The most paddle touches any one rally has had.
    pub(crate) longest_rally: u32,
    /// The fastest the ball has gone, in world units per second.
    pub(crate) top_speed: f32,
    /// How many balls the opponent sent back.
    pub(crate) returned_by_opponent: u32,
    /// How many balls got past the opponent.
    pub(crate) missed_by_opponent: u32,
}
impl Resource for Scoreboard {}

impl Scoreboard {
    /// A match nobody has scored in, with the first serve on its way out.
    fn new() -> Self {
        Scoreboard {
            left: 0,
            right: 0,
            serve_in: SERVE_PAUSE,
            serve_to: Side::Right,
            winner: None,
            touches: 0,
            longest_rally: 0,
            top_speed: 0.0,
            returned_by_opponent: 0,
            missed_by_opponent: 0,
        }
    }

    /// This side's points.
    pub(crate) const fn points(&self, side: Side) -> u32 {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
        }
    }
}

/// How a match is configured, shared by the window and the verify run so that
/// what is checked is what a person plays.
pub(crate) fn config() -> GameConfig {
    GameConfig {
        title: "jidousha — pong",
        ..GameConfig::default()
    }
}

/// Every system this game has, in one place and in one order.
///
/// The order is a decision, not an accident — see the file header. `verify.rs`
/// asserts it, because nothing else can see a swap of two of these lines.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, restart_the_match);
    // Both paddles move first, so the sweep below meets them where they ended
    // up rather than where they started.
    app.add_system(Update, drive_the_player);
    app.add_system(Update, drive_the_opponent);
    app.add_system(Update, serve_the_ball);
    app.add_system(Update, move_the_ball);
    app.add_system(Update, score_the_point);
    // The play goes down *first* and the court *after* it, so the court's
    // sorting behind is the bands' doing rather than the submission order's —
    // which is the only arrangement in which a recorded frame can see a layer
    // at all.
    app.add_system(Draw, draw_the_play);
    app.add_system(Draw, draw_the_court);
    app.add_system(Draw, draw_the_words);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("{HINT}. first to {WIN_SCORE}. close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        // Display, not Debug: `RunError`'s `Display` is the engine's four-part
        // message, and `fn main() -> Result<_, RunError>` would print a struct
        // dump instead.
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

mod capture;
mod checks;
mod controller;
mod verify;

// --- startup -----------------------------------------------------------

/// Put the camera, the scoreboard, two paddles and a ball into the world.
fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        center: Vec2::ZERO,
        height: VIEW_HEIGHT,
        clear_color: COURT,
        // The driver stamps the real window size in before every frame, so
        // setting it here would be overwritten and reading it here would be a
        // lie. Leave it to the default.
        ..Camera::default()
    });
    world.insert_resource(Scoreboard::new());

    for (side, control, speed) in [
        (Side::Left, Control::Player, PLAYER_SPEED),
        (Side::Right, Control::Opponent, OPPONENT_SPEED),
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
            },
        );
    }

    let ball = world.spawn();
    world.insert(ball, Transform::at(Vec2::ZERO));
    world.insert(ball, Velocity(Vec2::ZERO));
    world.insert(ball, Ball);
}

// --- the simulation ----------------------------------------------------

/// Start a fresh match when the winner's banner is up and space is pressed.
fn restart_the_match(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    if !input.just_pressed(Key::Space) {
        return;
    }
    if world.resource::<Scoreboard>().winner.is_none() {
        return;
    }
    world.insert_resource(Scoreboard::new());
    let ball: Vec<Entity> = world
        .query::<With<Ball>>()
        .map(|(entity, _)| entity)
        .collect();
    for entity in ball {
        world.component_mut::<Transform>(entity).pos = Vec2::ZERO;
        world.component_mut::<Velocity>(entity).0 = Vec2::ZERO;
    }
}

/// Move the player's paddle with W and S, clamped to the court.
fn drive_the_player(world: &mut World) {
    // `Startup` runs inside the first tick, before that tick's `Input` exists,
    // and a headless run never has one unless a check puts it there.
    let direction = match world.find_resource::<Input>() {
        None => return,
        Some(input) => f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    };
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.control != Control::Player {
            continue;
        }
        transform.pos.y =
            (transform.pos.y + direction * paddle.speed * dt).clamp(-PADDLE_TRAVEL, PADDLE_TRAVEL);
    }
}

/// Where the opponent's paddle wants its centre to be.
///
/// A pure function, called by the system that acts on it rather than being a
/// branch inside it, because a controller can only aim at where the opponent
/// *will* be if the answer is a function of the world it can call. Retrofitting
/// this is expensive; writing it this way costs nothing.
///
/// The rule: drift back to the middle until the ball is both coming and past
/// [`OPPONENT_WAKES_AT`], then stand [`OPPONENT_AIM`] off the ball so the
/// return leaves at an angle — away from whichever half of the court the
/// player's paddle is in.
pub(crate) fn opponent_target(ball_pos: Vec2, ball_velocity: Vec2, player_y: f32) -> f32 {
    if ball_velocity.x <= 0.0 || ball_pos.x <= OPPONENT_WAKES_AT {
        return 0.0;
    }
    // Y is down, so a positive offset sends the ball downwards: aim down when
    // the player is in the upper half of the court, and up when it is not.
    let away = if player_y > 0.0 { -1.0 } else { 1.0 };
    (ball_pos.y - away * OPPONENT_AIM).clamp(-PADDLE_TRAVEL, PADDLE_TRAVEL)
}

/// How far a paddle moving at `speed` gets in one tick.
pub(crate) fn paddle_step(speed: f32, dt: f32) -> f32 {
    speed * dt
}

/// Move a paddle one tick towards `target`, clamped to the court.
///
/// The other half of the pure pair above: a controller that has to know where
/// the opponent will be in forty ticks calls this forty times.
pub(crate) fn paddle_towards(from: f32, target: f32, step: f32) -> f32 {
    (from + (target - from).clamp(-step, step)).clamp(-PADDLE_TRAVEL, PADDLE_TRAVEL)
}

/// Move the machine's paddle towards [`opponent_target`].
///
/// Two passes, because the paddles are written while the ball and the player's
/// paddle are read.
fn drive_the_opponent(world: &mut World) {
    let ball = world
        .query::<(&Transform, &Velocity, With<Ball>)>()
        .map(|(_, transform, velocity, _)| (transform.pos, velocity.0))
        .next();
    let Some((ball_pos, ball_velocity)) = ball else {
        return;
    };
    let player_y = world
        .query::<(&Transform, &Paddle)>()
        .find(|(_, _, paddle)| paddle.control == Control::Player)
        .map_or(0.0, |(_, transform, _)| transform.pos.y);
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    let target = opponent_target(ball_pos, ball_velocity, player_y);

    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.control != Control::Opponent {
            continue;
        }
        transform.pos.y = paddle_towards(transform.pos.y, target, paddle_step(paddle.speed, dt));
    }
}

/// Count the serve pause down and launch the ball when it runs out.
fn serve_the_ball(world: &mut World) {
    let board = world.resource::<Scoreboard>();
    if board.winner.is_some() || board.serve_in == 0 {
        return;
    }
    let remaining = board.serve_in - 1;
    world.resource_mut::<Scoreboard>().serve_in = remaining;
    if remaining > 0 {
        return;
    }

    let towards = world.resource::<Scoreboard>().serve_to;
    // The engine's seeded generator, so the same run serves the same way every
    // time — which is what lets `verify.rs` replay a match.
    let spread = world.resource_mut::<Rng>().next_f32() * 2.0 - 1.0;
    let (sine, cosine) = sin_cos(Radians(spread * SERVE_SPREAD.as_f32()));
    let launch = Vec2::new(towards.sign() * cosine, sine) * SERVE_SPEED;

    for (_, transform, velocity, _) in
        world.query_mut::<(&mut Transform, &mut Velocity, With<Ball>)>()
    {
        transform.pos = Vec2::ZERO;
        velocity.0 = launch;
    }
}

/// One paddle's face, as the ball meets it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Face {
    /// Where the face is, in world X.
    pub(crate) plane_x: f32,
    /// Which way the ball has to be travelling to touch it: `-1.0` for the
    /// left paddle's inward face, `+1.0` for the right paddle's.
    pub(crate) approach: f32,
    /// The middle of the paddle, in world Y.
    pub(crate) centre_y: f32,
    /// How far from `centre_y` the face catches, in world units.
    pub(crate) reach: f32,
}

/// How far along this tick's travel the ball's leading edge first touches
/// `face`, as a fraction of the tick, or `None` if it did not.
///
/// The eight lines the engine deliberately does not have: the plane the
/// leading edge touches, whether the ball was approaching, whether this tick's
/// travel crossed it, and where along the tick. `face` stands still — see the
/// file header for which position that is and why.
///
/// A ball already past the plane at the start of the tick, or leaving through
/// the same face, is not a touch: both are `None`, which is what stops a ball
/// resting against a paddle from being reflected on every tick.
pub(crate) fn face_crossing(from: Vec2, to: Vec2, radius: f32, face: Face) -> Option<f32> {
    let lead_from = from.x + face.approach * radius;
    let lead_to = to.x + face.approach * radius;
    let travel = lead_to - lead_from;
    if travel * face.approach <= 0.0 {
        return None; // standing still, or leaving through this same face
    }
    if (lead_from - face.plane_x) * face.approach > 0.0 {
        return None; // already through it when the tick began
    }
    if (lead_to - face.plane_x) * face.approach < 0.0 {
        return None; // did not reach it by the end of the tick
    }
    let at = (face.plane_x - lead_from) / travel;
    let contact = from.lerp(to, at);
    if (contact.y - face.centre_y).abs() > face.reach {
        return None; // crossed the plane past the end of the paddle
    }
    Some(at)
}

/// Where a paddle standing at `pos` catches the ball.
pub(crate) fn face_of(side: Side, pos: Vec2) -> Face {
    Face {
        // The inward face: the side of the paddle the ball arrives at.
        plane_x: pos.x - side.sign() * PADDLE_SIZE.x * 0.5,
        // The ball travels towards the paddle, which is away from its side.
        approach: side.sign(),
        centre_y: pos.y,
        // A ball whose centre is level with the paddle's very end is caught:
        // the paddle's own half-height, plus the ball's radius.
        reach: PADDLE_SIZE.y * 0.5 + BALL_RADIUS,
    }
}

/// The velocity a ball leaves a paddle with.
///
/// A pure function for the same reason [`opponent_target`] is one: the check's
/// controller has to know what a candidate contact point would *do* before it
/// stands there, and a second copy of this arithmetic would drift away from
/// this one without anything noticing.
///
/// `contact_y` is where the ball's centre was when it touched, `paddle_y` the
/// middle of the paddle it touched; the further from the middle, the sharper
/// the angle, up to [`MAX_BOUNCE`] at the very end.
pub(crate) fn rebound(side: Side, contact_y: f32, paddle_y: f32, speed_in: f32) -> Vec2 {
    let offset = ((contact_y - paddle_y) / (PADDLE_SIZE.y * 0.5)).clamp(-1.0, 1.0);
    let (sine, cosine) = sin_cos(Radians(offset * MAX_BOUNCE.as_f32()));
    let speed = (speed_in + SPEED_GAIN).min(MAX_SPEED);
    // Away from the paddle that was hit. `cosine` is positive for every angle
    // inside the bounce limit, so the sign is the whole of the direction.
    Vec2::new(-side.sign() * cosine, sine) * speed
}

/// Carry the ball forward for `seconds`, bouncing it off the walls.
///
/// The third pure function, and the one the controller runs forward a few
/// hundred ticks at a time. The walls are *mirrored* rather than clamped, so a
/// ball that would have gone two units past the wall comes two units back off
/// it and keeps its speed.
pub(crate) fn drift(pos: Vec2, velocity: Vec2, seconds: f32) -> (Vec2, Vec2) {
    let mut pos = pos + velocity * seconds;
    let mut velocity = velocity;
    let top = -COURT_HALF_Y + BALL_RADIUS;
    let bottom = COURT_HALF_Y - BALL_RADIUS;
    if pos.y < top {
        pos.y = top + top - pos.y;
        velocity.y = -velocity.y;
    } else if pos.y > bottom {
        pos.y = bottom + bottom - pos.y;
        velocity.y = -velocity.y;
    }
    (pos, velocity)
}

/// Step the ball, bouncing it off whichever paddle it crossed and off the
/// walls.
///
/// Two passes, because the paddles are read while the ball is written.
fn move_the_ball(world: &mut World) {
    let board = world.resource::<Scoreboard>();
    if board.winner.is_some() || board.serve_in > 0 {
        return;
    }
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    let faces: Vec<(Side, Face)> = world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (paddle.side, face_of(paddle.side, transform.pos)))
        .collect();

    let mut struck: Option<Side> = None;
    let mut speed_now = 0.0;
    for (_, transform, velocity, _) in
        world.query_mut::<(&mut Transform, &mut Velocity, With<Ball>)>()
    {
        let from = transform.pos;
        let to = from + velocity.0 * dt;

        // The earliest of the two faces this tick's travel crossed. Only one
        // can ever be crossed — they face each other — but taking the earlier
        // says so rather than assuming it.
        let mut first: Option<(f32, Side, Face)> = None;
        for (side, face) in &faces {
            let Some(at) = face_crossing(from, to, BALL_RADIUS, *face) else {
                continue;
            };
            if first.is_none_or(|(so_far, _, _)| at < so_far) {
                first = Some((at, *side, *face));
            }
        }

        let (start, carry, seconds) = match first {
            Some((at, side, face)) => {
                let contact = from.lerp(to, at);
                let leaving = rebound(side, contact.y, face.centre_y, velocity.0.length());
                struck = Some(side);
                speed_now = leaving.length();
                // The rest of the tick, travelled the new way.
                (contact, leaving, dt * (1.0 - at))
            }
            None => (from, velocity.0, dt),
        };
        let (pos, carried) = drift(start, carry, seconds);
        transform.pos = pos;
        velocity.0 = carried;
    }

    if let Some(side) = struck {
        let board = world.resource_mut::<Scoreboard>();
        board.touches += 1;
        board.longest_rally = board.longest_rally.max(board.touches);
        board.top_speed = board.top_speed.max(speed_now);
        if side == Side::Right {
            board.returned_by_opponent += 1;
        }
    }
}

/// Award a point when the ball passes a goal line, and set up the next serve.
fn score_the_point(world: &mut World) {
    let board = world.resource::<Scoreboard>();
    if board.winner.is_some() || board.serve_in > 0 {
        return;
    }
    let ball = world
        .query::<(&Transform, With<Ball>)>()
        .map(|(_, transform, _)| transform.pos)
        .next();
    let Some(pos) = ball else { return };
    let conceded = if pos.x < -COURT_HALF_X {
        Side::Left
    } else if pos.x > COURT_HALF_X {
        Side::Right
    } else {
        return;
    };

    let board = world.resource_mut::<Scoreboard>();
    let scorer = conceded.other();
    match scorer {
        Side::Left => board.left += 1,
        Side::Right => board.right += 1,
    }
    if conceded == Side::Right {
        board.missed_by_opponent += 1;
    }
    board.touches = 0;
    board.serve_in = SERVE_PAUSE;
    // The next serve goes at whoever just conceded.
    board.serve_to = conceded;
    if board.points(scorer) >= WIN_SCORE {
        board.winner = Some(scorer);
    }

    for (_, transform, velocity, _) in
        world.query_mut::<(&mut Transform, &mut Velocity, With<Ball>)>()
    {
        transform.pos = Vec2::ZERO;
        velocity.0 = Vec2::ZERO;
    }
}

// --- drawing -----------------------------------------------------------

/// The paddles and the ball.
fn draw_the_play(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PLAY);
    for (_, transform, _) in ctx.world.query::<(&Transform, &Paddle)>() {
        ctx.rect(
            Rect::from_center_size(transform.pos, PADDLE_SIZE),
            PADDLE_COLOR,
            depth,
        );
    }
    for (_, transform, _) in ctx.world.query::<(&Transform, With<Ball>)>() {
        ctx.circle(transform.pos, BALL_RADIUS, BALL_COLOR, depth);
    }
}

/// The border and the dashed centre line.
///
/// Submitted *after* the play and drawn *behind* it, which is the bands doing
/// their job rather than the submission order doing it for them.
fn draw_the_court(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::FIELD);
    let court = Rect::from_center_size(
        Vec2::ZERO,
        Vec2::new(COURT_HALF_X * 2.0, COURT_HALF_Y * 2.0),
    );
    // A border is four lines. There is no stroke mode, and this is what one is.
    for (from, to) in [
        (court.min, Vec2::new(court.max.x, court.min.y)),
        (Vec2::new(court.max.x, court.min.y), court.max),
        (court.max, Vec2::new(court.min.x, court.max.y)),
        (Vec2::new(court.min.x, court.max.y), court.min),
    ] {
        ctx.line(from, to, BORDER_THICKNESS, MARKING, depth);
    }

    // The centre marking, as a column of rectangles: a dash pattern is not a
    // thing to ask a line for.
    let spacing = COURT_HALF_Y * 2.0 / DASH_COUNT as f32;
    for index in 0..DASH_COUNT {
        let y = (index as f32 - (DASH_COUNT - 1) as f32 * 0.5) * spacing;
        ctx.rect(
            Rect::from_center_size(Vec2::new(0.0, y), Vec2::new(DASH_WIDTH, DASH_HEIGHT)),
            MARKING,
            depth,
        );
    }
}

/// The score, the hint, and the winner's banner.
fn draw_the_words(ctx: &mut DrawCtx) {
    let board = ctx.world.resource::<Scoreboard>();
    let style = TextStyle {
        size: SCORE_SIZE,
        color: Color::WHITE,
        depth: Depth::layer(layers::UI),
    };
    // One number either side of the centre line, evenly set: the left score
    // ends `SCORE_INSET` short of the centre, the right one begins there.
    let left = board.left.to_string();
    ctx.text(
        Vec2::new(-SCORE_INSET - style.width_of(&left), SCORE_TOP),
        &left,
        style,
    );
    ctx.text(
        Vec2::new(SCORE_INSET, SCORE_TOP),
        &board.right.to_string(),
        style,
    );

    let hint = TextStyle {
        size: HINT_SIZE,
        color: Color::rgba(1.0, 1.0, 1.0, 0.55),
        depth: Depth::layer(layers::UI),
    };
    ctx.text(Vec2::new(-hint.width_of(HINT) * 0.5, HINT_TOP), HINT, hint);

    let Some(winner) = board.winner else { return };
    // One `ctx.text` call per line, each centred by its own width: `width_of`
    // measures the widest line only, so centring a two-line block in one call
    // hangs the shorter line off to the left.
    let banner = TextStyle {
        size: BANNER_SIZE,
        color: Color::WHITE,
        depth: Depth::layer(layers::UI),
    };
    let headline = winner.name();
    ctx.text(
        Vec2::new(-banner.width_of(headline) * 0.5, BANNER_TOP),
        headline,
        banner,
    );
    let under = TextStyle {
        size: BANNER_HINT_SIZE,
        color: Color::rgba(1.0, 1.0, 1.0, 0.75),
        depth: Depth::layer(layers::UI),
    };
    ctx.text(
        Vec2::new(-under.width_of(BANNER_HINT) * 0.5, BANNER_HINT_TOP),
        BANNER_HINT,
        under,
    );
}
