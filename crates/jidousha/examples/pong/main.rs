//! Pong: two paddles, a ball that bounces, a score, and an opponent you can
//! actually beat.
//!
//! W/S or the up/down arrows move the left paddle. The right one is played by
//! the machine, badly on purpose — it only wakes up once the ball crosses the
//! halfway line, and it tracks the ball's *current* height rather than where
//! the ball is going, so a steep return outruns it. First to seven wins;
//! space plays again.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//!
//! # The two decisions this file makes on purpose
//!
//! **A paddle counts as standing still at its post-move position.** Both
//! paddles move earlier in the same tick than the ball does, and the ball's
//! sweep is written against a plane that does not move — so the plane it
//! sweeps against is where the paddle ended up, not where it started. That is
//! wrong by at most one tick of paddle travel and it is right about the case
//! that matters, a paddle closing on the ball. The order *is* the sequence of
//! `add_system` calls in `register`, and `verify.rs` asserts on
//! `schedule_debug()` so a tidy-up cannot reverse it silently.
//!
//! **The layout is constants, for one aspect.** `WINDOW` is 16:9, every
//! position below is stated for that shape, and the check gives its recorder
//! the same size. Dragging the window narrower than 16:9 moves the side walls
//! in and the paddles would sit off the edges, with no check able to see it —
//! that is what the constants cost. Deriving the layout from `visible_bounds()`
//! in `Draw` would survive the drag; it would also mean every number here is
//! computed rather than named, and a prototype takes the constants.

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod controller;
mod verify;

// --- the court ---------------------------------------------------------
//
// Y is down: `-y` is the top of the screen, `+y` the bottom.

/// How big the window opens, and the one aspect this layout is stated for.
const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);

/// How many world units the camera spans vertically.
const VIEW_HEIGHT: f32 = 20.0;

/// Half the camera's height, in world units.
const HALF_H: f32 = VIEW_HEIGHT / 2.0;

/// Half the camera's width, in world units.
///
/// `PhysicalSize::aspect` is the number this is, and it is a `const fn`, so
/// this is derived from `WINDOW` rather than from a hand-typed ratio. The check
/// still asserts it against what the camera actually reports: the derivation
/// says the two constants agree with each other, not that the camera agrees
/// with either.
const HALF_W: f32 = HALF_H * WINDOW.aspect();

/// Where the top and bottom walls are, as a distance from the centre line.
const WALL_Y: f32 = 6.8;

/// How thick a wall is drawn, in world units.
const WALL_THICKNESS: f32 = 0.2;

/// How far the walls reach either side of the centre, in world units.
///
/// Short of the camera's own edge, so a wall quad is never flush with it.
const WALL_HALF_SPAN: f32 = HALF_W - 0.2;

/// What the court is cleared to.
///
/// Dark, because the ball and the markings are white and have to read against
/// it. The check asserts that requirement as well as this constant.
const COURT: Color = Color::rgb(0.04, 0.05, 0.07);

/// What the walls and the centre marking are drawn in.
const MARKING: Color = Color::rgba(1.0, 1.0, 1.0, 0.30);

/// How many dashes the centre marking is made of.
const DASH_COUNT: usize = 11;

/// How wide one centre-line dash is, in world units.
const DASH_WIDTH: f32 = 0.16;

/// What fraction of its pitch a centre-line dash fills.
const DASH_FILL: f32 = 0.55;

// --- the pieces --------------------------------------------------------

/// How big a paddle is, in world units.
///
/// The X here is not cosmetic: it is the thinnest thing the ball must not miss,
/// and therefore the ceiling on `MAX_SPEED * fixed_dt`. The check states that
/// pairing as an assertion rather than leaving it to this comment.
const PADDLE_SIZE: Vec2 = Vec2::new(1.0, 3.2);

/// How far a paddle sits from the centre line, in world units.
const PADDLE_X: f32 = 15.6;

/// How far a paddle's centre may travel from the centre line.
///
/// So a paddle's end stops flush against a wall rather than through it.
const PADDLE_LIMIT: f32 = WALL_Y - PADDLE_SIZE.y / 2.0;

/// What the player's paddle is drawn in.
const PLAYER_COLOR: Color = Color::rgb(0.35, 0.95, 1.0);

/// What the opponent's paddle is drawn in.
const OPPONENT_COLOR: Color = Color::rgb(1.0, 0.55, 0.35);

/// How fast the player's paddle travels, in world units per second.
const PLAYER_SPEED: f32 = 22.0;

/// How fast the opponent's paddle travels, in world units per second.
///
/// Below the vertical speed a steeply angled ball reaches, which is the whole
/// of what makes the opponent beatable: a flat return is tracked perfectly and
/// a steep one is not.
const OPPONENT_SPEED: f32 = 17.0;

/// How far off its own centre the opponent tries to meet the ball.
///
/// **Not zero, and that is a decision about whether the game is playable at
/// all.** An opponent that centres exactly on the ball returns it dead flat,
/// straight back down the middle — and against anyone who also centres on the
/// ball, which is what a person does on their first try, the rally has nowhere
/// to go and the match sits at nil-nil for ever. Meeting the ball off-centre
/// puts an angle on every return, so the court gets used. The check's chaser
/// run is the instrument that sees this, and it is the reason there are three
/// players in it rather than one.
const OPPONENT_BIAS: f32 = 1.1;

/// How far across the court the ball must come before the opponent reacts.
///
/// A handicap with a reason: it gives the player a window in which an angled
/// shot cannot be answered, so the game is winnable by aiming rather than by
/// waiting for the machine to make a mistake.
const OPPONENT_WAKES_AT: f32 = -2.0;

/// How big the ball is, in world units.
const BALL_RADIUS: f32 = 0.45;

/// What the ball is drawn in.
const BALL_COLOR: Color = Color::WHITE;

/// How fast the ball leaves a serve, in world units per second.
const SERVE_SPEED: f32 = 24.0;

/// How much faster the ball gets with every paddle it touches.
const SPEED_RAMP: f32 = 1.05;

/// The fastest the ball may ever travel, in world units per second.
///
/// Chosen against `PADDLE_SIZE.x`: at the engine's 1/60 timestep this is 0.733
/// world units of travel in one tick against a paddle 1.0 thick, so the sweep
/// below never has to reach across more than one paddle. The check asserts that
/// against the `fixed_dt` the engine actually hands it rather than against the
/// 1/60 assumed here.
///
/// It is also the number that makes the game a game. The ball's vertical speed
/// at the steepest bounce is `MAX_SPEED * sin(60 degrees)`, which is well above
/// either paddle's own speed — so neither side can defend by following the ball,
/// and both have to guess where it is going. Lower it under `OPPONENT_SPEED` and
/// the opponent returns everything; lower it under `PLAYER_SPEED` as well and
/// both sides track perfectly, the rally has nowhere to go, and the match sits
/// at nil-nil for ever.
const MAX_SPEED: f32 = 44.0;

/// The steepest a paddle may send the ball away, measured off the horizontal.
const MAX_BOUNCE: Radians = Radians::from_degrees(60.0);

/// How far off the horizontal a serve may wander.
const SERVE_SPREAD: Radians = Radians::from_degrees(25.0);

/// How many ticks the ball waits at the centre before a serve.
///
/// Ticks rather than seconds: the tick is the canonical timeline, and 45 of
/// them is three quarters of a second at the engine's default timestep.
const SERVE_TICKS: u32 = 45;

/// How much clear air is left between the ball at its furthest and the camera's
/// own edge.
///
/// Not decoration. Without it the goal line sits exactly `BALL_RADIUS` short of
/// the edge, the ball is drawn flush against it on the tick before a point, and
/// the "nothing off screen" check passes by six hundredths of a unit — right,
/// still passing, and one nudge from failing. The run prints its closest quad
/// for the same reason.
const GOAL_MARGIN: f32 = 0.3;

/// How far out the ball has to get before it counts as a goal.
///
/// Short of the camera's edge by its own radius and `GOAL_MARGIN`, so the ball
/// is reset on the tick it crosses and is never *drawn* hanging off the side.
const GOAL_X: f32 = HALF_W - BALL_RADIUS - GOAL_MARGIN;

/// How many points win the match.
const WIN_SCORE: u32 = 7;

// --- the writing on the screen -----------------------------------------

/// How tall a score digit is, in world units.
const SCORE_SIZE: f32 = 2.0;

/// Where the top of the score sits, in world units.
///
/// Inside the top third of the court, which is the requirement the check states
/// — rather than "wherever this constant says", which would move with it.
const SCORE_TOP: f32 = -9.4;

/// How far either side of the centre line a score number is set.
const SCORE_GAP: f32 = 1.6;

/// How tall the hint line at the bottom is, in world units.
const HINT_SIZE: f32 = 0.75;

/// Where the top of the hint line sits, in world units.
const HINT_TOP: f32 = 8.4;

/// What the hint line says.
const HINT: &str = "W/S or up/down to move - first to 7 wins";

/// How tall a line of the end banner is, in world units.
const BANNER_SIZE: f32 = 1.8;

/// Where the top of the banner's first line sits, in world units.
const BANNER_TOP: f32 = -1.9;

/// What the banner says, one entry per line.
///
/// A function rather than a `format!` inside the draw system, because no
/// assertion over drawn quads can see a wrong *character* — the font draws an
/// unknown one as a box at exactly a letter's advance — so the string itself is
/// the only instrument, and a check has to be able to reach it.
fn banner_lines(winner: Side) -> [&'static str; 2] {
    match winner {
        Side::Player => ["YOU WIN", "space to play again"],
        Side::Opponent => ["YOU LOSE", "space to play again"],
    }
}

/// The score line for one side, as it is drawn.
fn score_text(points: u32) -> String {
    format!("{points}")
}

/// Draw bands, named once so no `layer: 2` appears at a call site.
mod layers {
    /// The court itself: walls and centre marking.
    pub const FIELD: i16 = -1;
    /// The things the game is about: paddles and ball.
    pub const PLAY: i16 = 0;
    /// Score, hint and banner, over everything.
    pub const UI: i16 = 1;
}

// --- state --------------------------------------------------------------

/// Which end of the court something belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// The left-hand end, played from the keyboard.
    Player,
    /// The right-hand end, played by the machine.
    Opponent,
}

impl Side {
    /// Which way this side lies from the centre: `-1.0` left, `+1.0` right.
    const fn sign(self) -> f32 {
        match self {
            Side::Player => -1.0,
            Side::Opponent => 1.0,
        }
    }

    /// The other one.
    const fn other(self) -> Side {
        match self {
            Side::Player => Side::Opponent,
            Side::Opponent => Side::Player,
        }
    }
}

/// A paddle, and which end it plays.
#[derive(Clone, Copy)]
struct Paddle {
    /// Which end of the court this paddle defends.
    side: Side,
}
impl Component for Paddle {}

/// The ball, and how it is travelling.
#[derive(Clone, Copy)]
struct Ball {
    /// World units per second, as a vector.
    velocity: Vec2,
    /// World units per second, as a scalar — the authority, so a bounce cannot
    /// leak or gain speed through repeated normalisation.
    speed: f32,
}
impl Component for Ball {}

/// The score, and what the court is doing right now.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Match {
    /// The player's points.
    left: u32,
    /// The opponent's points.
    right: u32,
    /// Ticks left before the ball is released; zero means play is live.
    countdown: u32,
    /// Which end the next serve travels towards.
    serve_towards: Side,
    /// Who won, once somebody reached `WIN_SCORE`.
    winner: Option<Side>,
    /// How many paddles the ball has touched since the last serve.
    rally: u32,
    /// The longest rally of the match so far, in paddle touches.
    longest_rally: u32,
    /// The fastest the ball has travelled this match, in world units per second.
    top_speed: f32,
}
impl Resource for Match {}

impl Match {
    /// A fresh match: nil-nil, serving towards the player.
    const fn new() -> Self {
        Match {
            left: 0,
            right: 0,
            countdown: SERVE_TICKS,
            serve_towards: Side::Player,
            winner: None,
            rally: 0,
            longest_rally: 0,
            top_speed: 0.0,
        }
    }

    /// This side's points.
    const fn points(&self, side: Side) -> u32 {
        match side {
            Side::Player => self.left,
            Side::Opponent => self.right,
        }
    }
}

// --- the arithmetic, as functions a check can call ----------------------

/// How far the ball's leading edge still is from `side`'s paddle face.
///
/// Positive while the ball is in front of the face, negative once it is
/// through. One function for both ends: the sign flip is `side`, so there is
/// no mirrored copy of this to keep in step.
fn face_gap(ball_x: f32, paddle_x: f32, side: Side) -> f32 {
    let sign = side.sign();
    let face = paddle_x - sign * PADDLE_SIZE.x / 2.0;
    let leading_edge = ball_x + sign * BALL_RADIUS;
    sign * (face - leading_edge)
}

/// How far through this tick's travel the ball reached a paddle's face, as a
/// fraction in `0.0..=1.0`, or `None` if it did not reach it at all.
///
/// The eight lines the engine deliberately does not have. Each condition is
/// named the positive way round and the whole conjunction is negated once —
/// which is both what clippy's `neg_cmp_op_on_partial_ord` wants and what is
/// right about NaN: a velocity that has gone to NaN fails every conjunct, so
/// the answer is "no contact" rather than a contact at a NaN fraction that the
/// ball then sits at, silently, for the rest of the run.
fn face_contact(before: f32, after: f32) -> Option<f32> {
    let travel = before - after;
    let approaching = travel > 0.0; // not standing still, not going the other way
    let in_front = before >= 0.0; // not already through it
    let reached = after <= 0.0; // this tick's travel did not stop short
    if !(approaching && in_front && reached) {
        return None;
    }
    Some(before / travel)
}

/// Where a paddle sends the ball, given how far off its centre the ball struck.
///
/// `offset` is the contact point's height minus the paddle's, so the middle of
/// the paddle returns the ball flat and the ends return it at `MAX_BOUNCE`.
/// Built from `sin_cos` rather than from `rotate`, because the two ends want
/// opposite X and the same Y — which is one expression this way and a sign
/// puzzle the other.
fn rebound(offset: f32, speed: f32, struck: Side) -> Vec2 {
    let span = PADDLE_SIZE.y / 2.0 + BALL_RADIUS;
    let lean = (offset / span).clamp(-1.0, 1.0);
    let (sine, cosine) = sin_cos(Radians(lean * MAX_BOUNCE.as_f32()));
    // Away from the paddle that was struck, and towards the side of it that
    // was struck: Y is down, so a hit above the centre leaves upwards.
    Vec2::new(-struck.sign() * cosine, sine) * speed
}

/// The height the opponent's paddle is trying to reach this tick.
///
/// A free function, called by the system that acts on it, so a check can ask
/// the game where the opponent will go without ticking anything.
fn opponent_target(ball_pos: Vec2, ball_velocity: Vec2) -> f32 {
    // Asleep until the ball is both coming this way and past the halfway
    // handicap. Returning to the centre while asleep is what makes the
    // handicap cost something: the opponent starts every chase from the middle.
    if ball_velocity.x <= 0.0 || ball_pos.x < OPPONENT_WAKES_AT {
        return 0.0;
    }
    // Off-centre, on the side that sends the ball back across the court rather
    // than straight down the middle. `signum` answers 1.0 for zero, so a ball
    // arriving exactly on the centre line picks a side rather than juddering
    // between two.
    ball_pos.y + OPPONENT_BIAS * ball_pos.y.signum()
}

/// A paddle's height after moving `speed * dt` towards `target`, clamped to the
/// court.
///
/// Stops exactly on the target rather than overshooting and juddering, which is
/// what `clamp` on the remaining distance buys over a `signum` step.
fn paddle_step(current: f32, target: f32, speed: f32, dt: f32) -> f32 {
    let reach = speed * dt;
    (current + (target - current).clamp(-reach, reach)).clamp(-PADDLE_LIMIT, PADDLE_LIMIT)
}

/// The velocity a serve leaves the centre with.
fn serve_velocity(rng: &mut Rng, towards: Side) -> Vec2 {
    let spread = (rng.next_f32() - 0.5) * 2.0;
    let (sine, cosine) = sin_cos(Radians(spread * SERVE_SPREAD.as_f32()));
    Vec2::new(towards.sign() * cosine, sine) * SERVE_SPEED
}

/// Where the ball's centre may go, vertically.
const fn ball_limit() -> f32 {
    WALL_Y - BALL_RADIUS
}

/// A position and velocity reflected back inside the walls.
///
/// Reflected rather than clamped: a clamped ball slides along the wall for a
/// tick, and reflecting keeps the angle the player aimed.
fn bounce_off_walls(mut pos: Vec2, mut velocity: Vec2) -> (Vec2, Vec2) {
    let limit = ball_limit();
    if pos.y < -limit {
        pos.y = -2.0 * limit - pos.y;
        velocity.y = -velocity.y;
    }
    if pos.y > limit {
        pos.y = 2.0 * limit - pos.y;
        velocity.y = -velocity.y;
    }
    (pos, velocity)
}

/// One tick of a ball nobody touches: where the straight travel took it, where
/// the walls left it, and the velocity it came out with.
///
/// Three values rather than two because a sweep is a question about the
/// *straight* segment and the wall reflection happens after it — that is the
/// order `move_the_ball` applies them in, and a controller rolling the ball
/// forward against the reflected point would disagree with the game on exactly
/// the ticks where the answer matters. One function, so there is only ever one
/// of it to keep in step.
fn free_step(pos: Vec2, velocity: Vec2, dt: f32) -> (Vec2, Vec2, Vec2) {
    let straight = pos + velocity * dt;
    let (settled, reflected) = bounce_off_walls(straight, velocity);
    (straight, settled, reflected)
}

/// How far off a paddle's centre a ball may strike and still be returned.
const fn contact_span() -> f32 {
    PADDLE_SIZE.y / 2.0 + BALL_RADIUS
}

// --- the game ------------------------------------------------------------

/// The game's configuration, shared by the window and the check, so that what
/// is verified is what a person plays.
fn config() -> GameConfig {
    GameConfig {
        title: "jidousha - pong",
        seed: 7,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// Every system this game has, in the order they run.
///
/// The order is the decision: both paddles move before the ball, so the ball's
/// sweep meets each paddle where it ended up. `verify.rs` asserts on
/// `schedule_debug()` because nothing else in the surface can see a swap here.
fn register(app: &mut App) {
    app.add_system(Startup, set_the_court);
    app.add_system(Update, restart_the_match);
    app.add_system(Update, drive_the_player);
    app.add_system(Update, drive_the_opponent);
    app.add_system(Update, count_down_the_serve);
    app.add_system(Update, move_the_ball);
    // Submitted play-first and field-second on purpose: the sort puts the
    // court behind the ball, which is a pair whose order only the bands can
    // produce. Where submission order already agrees with the bands, no
    // assertion over drawn quads can see a layer at all.
    app.add_system(Draw, draw_the_play);
    app.add_system(Draw, draw_the_court);
    app.add_system(Draw, draw_the_hud);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("W/S or up/down move the left paddle. first to {WIN_SCORE} wins; space plays again");
    println!("close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        // Display, not Debug: `RunError`'s `Display` is the engine's four-part
        // message, and a `fn main() -> Result<_, RunError>` would print a
        // struct dump instead.
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Put the camera, the score, the paddles and the ball into an empty world.
fn set_the_court(world: &mut World) {
    world.insert_resource(Camera {
        center: Vec2::ZERO,
        height: VIEW_HEIGHT,
        clear_color: COURT,
        ..Camera::default()
    });
    world.insert_resource(Match::new());

    for side in [Side::Player, Side::Opponent] {
        let paddle = world.spawn();
        world.insert(
            paddle,
            Transform::at(Vec2::new(side.sign() * PADDLE_X, 0.0)),
        );
        world.insert(paddle, Paddle { side });
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

/// Space starts a new match once one has been won.
fn restart_the_match(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    if !input.just_pressed(Key::Space) {
        return;
    }
    if world.resource::<Match>().winner.is_none() {
        return;
    }
    world.insert_resource(Match::new());
    park_the_ball(world);
    for (_, transform, _) in world.query_mut::<(&mut Transform, &Paddle)>() {
        transform.pos.y = 0.0;
    }
}

/// W/S or the arrows, moving the left paddle.
fn drive_the_player(world: &mut World) {
    let lean = match world.find_resource::<Input>() {
        // The first tick of a run happens before any input exists, and under
        // `headless` there may never be one.
        None => return,
        Some(input) => {
            let down = input.held(Key::S) || input.held(Key::ArrowDown);
            let up = input.held(Key::W) || input.held(Key::ArrowUp);
            f32::from(down) - f32::from(up)
        }
    };
    if world.resource::<Match>().winner.is_some() {
        return;
    }
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.side == Side::Player {
            // A held key is a target one tick's travel away, so the player and
            // the opponent go through the same clamp and the same limit.
            let target = transform.pos.y + lean * PLAYER_SPEED * dt;
            transform.pos.y = paddle_step(transform.pos.y, target, PLAYER_SPEED, dt);
        }
    }
}

/// The machine, moving the right paddle towards where it thinks the ball is.
fn drive_the_opponent(world: &mut World) {
    if world.resource::<Match>().winner.is_some() {
        return;
    }
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    // Read first, write second: the ball's own row is read out before the query
    // that writes the paddles borrows the world.
    let Some((_, ball_at, ball)) = world.query::<(&Transform, &Ball)>().next() else {
        return;
    };
    let target = opponent_target(ball_at.pos, ball.velocity);
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.side == Side::Opponent {
            transform.pos.y = paddle_step(transform.pos.y, target, OPPONENT_SPEED, dt);
        }
    }
}

/// Hold the ball at the centre for the serve pause, then let it go.
fn count_down_the_serve(world: &mut World) {
    let state = *world.resource::<Match>();
    if state.winner.is_some() || state.countdown == 0 {
        return;
    }
    let remaining = state.countdown - 1;
    world.resource_mut::<Match>().countdown = remaining;
    if remaining > 0 {
        return;
    }
    let velocity = serve_velocity(world.resource_mut::<Rng>(), state.serve_towards);
    for (_, ball) in world.query_mut::<&mut Ball>() {
        ball.velocity = velocity;
        ball.speed = SERVE_SPEED;
    }
    world.resource_mut::<Match>().top_speed = world.resource::<Match>().top_speed.max(SERVE_SPEED);
}

/// The whole of the physics: travel, walls, paddles, goals.
fn move_the_ball(world: &mut World) {
    let state = *world.resource::<Match>();
    if state.winner.is_some() || state.countdown > 0 {
        return;
    }
    let dt = world.resource::<Time>().fixed_dt.as_f32();

    // Read: where the ball is, and where both paddles ended up this tick.
    let Some((entity, ball_at, ball)) = world
        .query::<(&Transform, &Ball)>()
        .map(|(entity, transform, ball)| (entity, transform.pos, *ball))
        .next()
    else {
        return;
    };
    let mut paddles: Vec<(Side, f32)> = world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (paddle.side, transform.pos.y))
        .collect();
    // Query order is deterministic but is not spawn order, so sort on something
    // the game owns rather than on the order the rows came out.
    paddles.sort_by_key(|(side, _)| *side == Side::Opponent);

    let before = ball_at;
    let (after, settled, wall_velocity) = free_step(before, ball.velocity, dt);

    // The sweep. Each paddle's face is a plane that stands still — at the
    // position that paddle *ended this tick at*, which is the decision this
    // file's header states.
    let mut hit: Option<(Side, f32, f32)> = None;
    for (side, paddle_y) in &paddles {
        let gap_before = face_gap(before.x, side.sign() * PADDLE_X, *side);
        let gap_after = face_gap(after.x, side.sign() * PADDLE_X, *side);
        let Some(fraction) = face_contact(gap_before, gap_after) else {
            continue;
        };
        let contact_y = before.lerp(after, fraction).y;
        let offset = contact_y - paddle_y;
        if offset.abs() > contact_span() {
            continue;
        }
        // The earliest contact of the two, which cannot happen at this speed
        // but costs one comparison to be right about.
        if hit.is_none_or(|(_, earlier, _)| fraction < earlier) {
            hit = Some((*side, fraction, offset));
        }
    }

    let (pos, velocity, speed, touched) = match hit {
        Some((side, fraction, offset)) => {
            let speed = (ball.speed * SPEED_RAMP).min(MAX_SPEED);
            let velocity = rebound(offset, speed, side);
            let contact = before.lerp(after, fraction);
            // The rest of the tick, travelled the new way, and only then the
            // walls: a ball that clips a corner takes the paddle first.
            let (pos, velocity) =
                bounce_off_walls(contact + velocity * dt * (1.0 - fraction), velocity);
            (pos, velocity, speed, true)
        }
        None => (settled, wall_velocity, ball.speed, false),
    };

    // A goal is the ball past the far edge of the court. It is reset on the
    // same tick, so the ball is never drawn hanging off the side.
    let scored = if pos.x < -GOAL_X {
        Some(Side::Opponent)
    } else if pos.x > GOAL_X {
        Some(Side::Player)
    } else {
        None
    };

    // Write.
    if let Some(scorer) = scored {
        let mut state = *world.resource::<Match>();
        match scorer {
            Side::Player => state.left += 1,
            Side::Opponent => state.right += 1,
        }
        state.countdown = SERVE_TICKS;
        state.serve_towards = scorer.other();
        state.longest_rally = state.longest_rally.max(state.rally);
        state.rally = 0;
        if state.points(scorer) >= WIN_SCORE {
            state.winner = Some(scorer);
        }
        world.insert_resource(state);
        park_the_ball(world);
        return;
    }

    if touched {
        let mut state = *world.resource::<Match>();
        state.rally += 1;
        state.longest_rally = state.longest_rally.max(state.rally);
        state.top_speed = state.top_speed.max(speed);
        world.insert_resource(state);
    }
    if let Some(transform) = world.find_component_mut::<Transform>(entity) {
        transform.pos = pos;
    }
    if let Some(ball) = world.find_component_mut::<Ball>(entity) {
        ball.velocity = velocity;
        ball.speed = speed;
    }
}

/// Put the ball back on the centre spot, stopped.
fn park_the_ball(world: &mut World) {
    let balls: Vec<Entity> = world
        .query::<(&Transform, &Ball)>()
        .map(|(entity, _, _)| entity)
        .collect();
    for entity in balls {
        if let Some(transform) = world.find_component_mut::<Transform>(entity) {
            transform.pos = Vec2::ZERO;
        }
        if let Some(ball) = world.find_component_mut::<Ball>(entity) {
            ball.velocity = Vec2::ZERO;
            ball.speed = SERVE_SPEED;
        }
    }
}

// --- drawing -------------------------------------------------------------

/// Paddles and ball.
fn draw_the_play(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PLAY);
    // Straight out of the query: a Draw system's iterator borrows the world,
    // not the context, so there is no two-pass collect to do here.
    for (_, transform, paddle) in ctx.world.query::<(&Transform, &Paddle)>() {
        let color = match paddle.side {
            Side::Player => PLAYER_COLOR,
            Side::Opponent => OPPONENT_COLOR,
        };
        ctx.rect(
            Rect::from_center_size(transform.pos, PADDLE_SIZE),
            color,
            depth,
        );
    }
    for (_, transform, _) in ctx.world.query::<(&Transform, &Ball)>() {
        ctx.circle(transform.pos, BALL_RADIUS, BALL_COLOR, depth);
    }
}

/// The walls and the centre marking — drawn after the play and behind it.
fn draw_the_court(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::FIELD);
    for wall in [-WALL_Y, WALL_Y] {
        ctx.line(
            Vec2::new(-WALL_HALF_SPAN, wall),
            Vec2::new(WALL_HALF_SPAN, wall),
            WALL_THICKNESS,
            MARKING,
            depth,
        );
    }
    // A dashed centre marking is a column of rectangles: there is no dash
    // pattern to ask for, and this is what one is.
    let pitch = WALL_Y * 2.0 / DASH_COUNT as f32;
    for index in 0..DASH_COUNT {
        let centre = -WALL_Y + pitch * (index as f32 + 0.5);
        ctx.rect(
            Rect::from_center_size(
                Vec2::new(0.0, centre),
                Vec2::new(DASH_WIDTH, pitch * DASH_FILL),
            ),
            MARKING,
            depth,
        );
    }
}

/// The score, the hint, and the banner once somebody has won.
fn draw_the_hud(ctx: &mut DrawCtx) {
    let state = *ctx.world.resource::<Match>();
    let score = TextStyle {
        size: SCORE_SIZE,
        color: Color::WHITE,
        depth: Depth::layer(layers::UI),
    };
    // One number either side of the centre line, each set against the gap: the
    // left one ends `SCORE_GAP` before the line, the right one starts
    // `SCORE_GAP` after it.
    let left = score_text(state.left);
    let right = score_text(state.right);
    ctx.text(
        Vec2::new(-SCORE_GAP - score.width_of(&left), SCORE_TOP),
        &left,
        score,
    );
    ctx.text(Vec2::new(SCORE_GAP, SCORE_TOP), &right, score);

    let hint = TextStyle {
        size: HINT_SIZE,
        color: Color::rgba(1.0, 1.0, 1.0, 0.55),
        depth: Depth::layer(layers::UI),
    };
    ctx.text(Vec2::new(-hint.width_of(HINT) * 0.5, HINT_TOP), HINT, hint);

    let Some(winner) = state.winner else {
        return;
    };
    let banner = TextStyle {
        size: BANNER_SIZE,
        color: Color::WHITE,
        depth: Depth::layer(layers::UI),
    };
    let under = TextStyle {
        size: BANNER_SIZE / 2.0,
        color: Color::rgba(1.0, 1.0, 1.0, 0.8),
        depth: Depth::layer(layers::UI),
    };
    let [headline, subtitle] = banner_lines(winner);
    // One `ctx.text` per line, each centred by its own width. Centring a
    // two-line block by one `width_of` would centre the longer line and hang
    // the shorter one off to the left, visibly crooked and invisible to every
    // assertion over drawn quads.
    ctx.text(
        Vec2::new(-banner.width_of(headline) * 0.5, BANNER_TOP),
        headline,
        banner,
    );
    ctx.text(
        Vec2::new(
            -under.width_of(subtitle) * 0.5,
            BANNER_TOP + BANNER_SIZE + 0.4,
        ),
        subtitle,
        under,
    );
}
