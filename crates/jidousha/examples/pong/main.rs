//! Pong: two paddles, a ball, a score, and thirty seconds of a game.
//!
//! W/S or the up/down arrows move the left paddle. The right one is an
//! opponent that chases the ball and drifts back to the middle between shots.
//! First to five. Space serves early, and starts the next match once one is
//! over.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//!
//! # Where the arithmetic lives, and why it is out here
//!
//! [`advance`] and [`opponent_push`] are plain functions rather than the bodies
//! of the systems that call them. That is not tidiness: a check that wants to
//! know whether this game is *playable* has to drive the left paddle well
//! enough to win a point, and to do that it has to be able to run the ball and
//! the opponent forward and look at where they end up. A decision buried inside
//! `fn move_the_ball(&mut World)` cannot be asked anything. `verify.rs` calls
//! both of these directly, some tens of thousands of times a run.

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod controller;
mod verify;

// --- the court ---------------------------------------------------------
//
// Every number below is in world units, and the camera is `VIEW_HEIGHT` of
// them tall. The layout is stated as constants rather than derived from
// `Camera::visible_bounds()`, because a viewport read on tick 1 is whatever the
// game put there rather than the window's true shape.

/// How many world units the camera spans vertically.
pub(crate) const VIEW_HEIGHT: f32 = 20.0;

/// Half the court's height: the inner face of the top and bottom walls.
pub(crate) const COURT_HALF_HEIGHT: f32 = 9.0;

/// Half the court's width, as the walls are drawn.
///
/// Inside the camera at 16:9, which spans `VIEW_HEIGHT * 16 / 9 / 2` = 17.78
/// units either side of the centre.
pub(crate) const COURT_HALF_WIDTH: f32 = 17.0;

/// Past this much X, the ball is out and somebody has scored.
pub(crate) const GOAL_LINE: f32 = 16.4;

/// How far from the centre each paddle stands.
pub(crate) const PADDLE_X: f32 = 14.0;

/// How big a paddle is, in world units.
pub(crate) const PADDLE_SIZE: Vec2 = Vec2::new(0.7, 2.0);

/// How far a paddle's centre may travel from the middle.
pub(crate) const PADDLE_LIMIT: f32 = COURT_HALF_HEIGHT - PADDLE_SIZE.y * 0.5;

/// How big the ball is.
pub(crate) const BALL_RADIUS: f32 = 0.35;

/// The plane the ball's *centre* crosses when it meets a paddle.
///
/// The paddle's inner face, moved out by the ball's radius, so the sweep in
/// [`advance`] can treat the ball as a point.
pub(crate) const CONTACT_X: f32 = PADDLE_X - PADDLE_SIZE.x * 0.5 - BALL_RADIUS;

/// How far either side of a paddle's centre the ball's centre still counts as
/// a hit.
pub(crate) const CONTACT_REACH: f32 = PADDLE_SIZE.y * 0.5 + BALL_RADIUS;

/// How far from the middle the ball's centre may get before the wall stops it.
pub(crate) const BALL_LIMIT: f32 = COURT_HALF_HEIGHT - BALL_RADIUS;

// --- the game ----------------------------------------------------------

/// How fast the player's paddle moves, in world units per second.
pub(crate) const PLAYER_SPEED: f32 = 20.0;

/// How fast the opponent's paddle moves, in world units per second.
///
/// Slower than the vertical speed a steeply struck ball has (a 60-degree
/// bounce at the serve speed is already 14.7 units/s downrange), which is what
/// makes the opponent beatable at all: it chases where the ball *is*, so a shot
/// steep enough outruns it.
pub(crate) const OPPONENT_SPEED: f32 = 15.5;

/// How close is close enough for the opponent, so it does not jitter.
pub(crate) const OPPONENT_DEAD_BAND: f32 = 0.3;

/// How far off its own centre the opponent tries to meet the ball, in world
/// units.
///
/// Without this the opponent centres on the ball, returns it dead flat, and a
/// player who also centres on the ball returns it dead flat back: both sides
/// hold a groove neither can lose and the rally never ends. A headless run of
/// a paddle that only chases the ball scored **one point in seven thousand
/// ticks** before this constant existed, which is a game nobody would sit
/// through and not something any check on the ball's arithmetic could see.
///
/// So the opponent plays a shot instead: it meets a descending ball above its
/// own centre and a climbing one below, which sends the ball back the way it
/// came from and keeps both players moving. `verify.rs` runs the chasing player
/// every time for exactly this reason.
pub(crate) const OPPONENT_AIM: f32 = PADDLE_SIZE.y * 0.5 * 0.55;

/// How fast the ball leaves a serve, in world units per second.
pub(crate) const BALL_SPEED_START: f32 = 19.0;

/// What the ball's speed is multiplied by on each paddle touch.
pub(crate) const BALL_SPEED_GAIN: f32 = 1.12;

/// The fastest the ball may ever go, in world units per second.
///
/// CONTRACT: `BALL_SPEED_MAX * fixed_dt` must stay under `PADDLE_SIZE.x`, or a
/// tick's travel can step clean through a paddle. The sweep in [`advance`]
/// makes that survivable rather than fatal, but the cap is the reason a ball
/// never has to be caught by two mechanisms at once. `verify.rs` asserts it
/// against the timestep the engine actually hands the game, not against 1/60.
pub(crate) const BALL_SPEED_MAX: f32 = 31.0;

/// The widest angle a paddle can send the ball off at, measured from straight
/// across the court.
pub(crate) const MAX_BOUNCE: Radians = Radians::from_degrees(60.0);

/// The widest angle a serve leaves the centre at.
pub(crate) const SERVE_SPREAD: Radians = Radians::from_degrees(35.0);

/// How long the ball sits at the centre between points, in ticks.
///
/// Ticks rather than seconds: the tick is the canonical timeline, and 45 of
/// them is three quarters of a second at the default timestep.
pub(crate) const SERVE_PAUSE: u32 = 32;

/// How many points win a match.
pub(crate) const WINNING_SCORE: u32 = 5;

// --- what things look like ---------------------------------------------

/// What the court is cleared to.
///
/// Dark on purpose: the ball and the markings are near-white, and `verify.rs`
/// asserts both that this is the colour the frame cleared to *and* that it is
/// dark enough for a white ball to read against — the second of which survives
/// somebody changing this constant.
pub(crate) const COURT: Color = Color::rgb(0.05, 0.07, 0.09);

/// The top and bottom walls.
pub(crate) const WALL: Color = Color::rgba(1.0, 1.0, 1.0, 0.45);

/// The dashes down the middle.
pub(crate) const NET: Color = Color::rgba(1.0, 1.0, 1.0, 0.22);

/// The ball.
pub(crate) const BALL_COLOR: Color = Color::rgb(1.0, 0.96, 0.75);

/// The paddle the player moves.
pub(crate) const PLAYER_COLOR: Color = Color::rgb(0.42, 0.88, 1.0);

/// The paddle the game moves.
pub(crate) const OPPONENT_COLOR: Color = Color::rgb(1.0, 0.55, 0.45);

/// How thick the walls are drawn.
pub(crate) const WALL_THICKNESS: f32 = 0.2;

/// How big one dash of the centre line is.
pub(crate) const DASH_SIZE: Vec2 = Vec2::new(0.18, 0.8);

/// How far apart the centre line's dashes are, centre to centre.
pub(crate) const DASH_PITCH: f32 = 1.5;

/// How tall the score digits are.
pub(crate) const SCORE_SIZE: f32 = 2.4;

/// Where the top of the score sits.
pub(crate) const SCORE_TOP: f32 = -8.3;

/// How far either side of the centre line a score digit is set.
pub(crate) const SCORE_INSET: f32 = 2.6;

/// How tall the hint line at the bottom is.
pub(crate) const HINT_SIZE: f32 = 0.62;

/// The controls, said once where a player can read them.
pub(crate) const HINT: &str = "W/S move your paddle    space serves    first to 5";

/// How tall the winner's banner is.
pub(crate) const BANNER_SIZE: f32 = 2.0;

/// How tall the line under the winner's banner is.
pub(crate) const BANNER_SUB_SIZE: f32 = 0.9;

/// What the banner says when the player wins.
pub(crate) const BANNER_WON: &str = "YOU WIN";

/// What the banner says when the opponent wins.
pub(crate) const BANNER_LOST: &str = "THE OPPONENT WINS";

/// The line under either banner.
pub(crate) const BANNER_SUB: &str = "press space for another match";

/// Draw bands, named once so no `layer: 1` appears at a call site.
pub(crate) mod layers {
    /// The walls and the centre line, behind everything.
    pub(crate) const FIELD: i16 = -1;
    /// The paddles and the ball.
    pub(crate) const PLAY: i16 = 0;
    /// The score, the hint and the banner, over everything.
    pub(crate) const UI: i16 = 1;
}

// --- components and resources -------------------------------------------

/// Which end of the court something belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// The player's end, at negative X.
    Left,
    /// The opponent's end, at positive X.
    Right,
}

impl Side {
    /// Which way this side's paddle sends the ball: -1 for the right paddle,
    /// +1 for the left.
    pub(crate) const fn outward(self) -> f32 {
        match self {
            Side::Left => 1.0,
            Side::Right => -1.0,
        }
    }

    /// Where this side's paddle stands, in X.
    pub(crate) const fn paddle_x(self) -> f32 {
        match self {
            Side::Left => -PADDLE_X,
            Side::Right => PADDLE_X,
        }
    }

    /// This side's slot in the two-element arrays [`advance`] takes.
    pub(crate) const fn index(self) -> usize {
        match self {
            Side::Left => 0,
            Side::Right => 1,
        }
    }

    /// The other one.
    pub(crate) const fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// The side's name, for a message.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Side::Left => "you",
            Side::Right => "the opponent",
        }
    }
}

/// Who moves a paddle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    /// The keyboard.
    Player,
    /// [`opponent_push`].
    Opponent,
}

/// A paddle: which end it defends, and who moves it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Paddle {
    /// Which end of the court.
    pub(crate) side: Side,
    /// Who moves it.
    pub(crate) control: Control,
}
impl Component for Paddle {}

/// The ball. Its position lives in its `Transform`, like everything else's.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ball {
    /// World units per second.
    pub(crate) vel: Vec2,
}
impl Component for Ball {}

/// What the match is doing right now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Stage {
    /// The ball is parked at the centre, waiting to be served.
    Serving {
        /// How many more ticks before it goes.
        ticks_left: u32,
        /// Which side it will be served at.
        toward: Side,
    },
    /// The ball is live.
    Rally,
    /// Somebody reached [`WINNING_SCORE`].
    Over {
        /// Who did.
        winner: Side,
    },
}

/// The score, and what the match is doing about it.
///
/// One resource rather than two, so a check can stage a screen the run never
/// reached — the losing banner, say — by inserting exactly one value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Scoreboard {
    /// The player's points.
    pub(crate) left: u32,
    /// The opponent's points.
    pub(crate) right: u32,
    /// What the match is doing.
    pub(crate) stage: Stage,
}
impl Resource for Scoreboard {}

impl Scoreboard {
    /// A match nobody has scored in yet, about to serve at the player.
    pub(crate) const fn fresh() -> Self {
        Scoreboard {
            left: 0,
            right: 0,
            stage: Stage::Serving {
                ticks_left: SERVE_PAUSE,
                toward: Side::Left,
            },
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

// --- the arithmetic a check can call ------------------------------------

/// A ball's position and velocity: the whole of what the flight arithmetic
/// needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Flight {
    /// Where the ball's centre is.
    pub(crate) pos: Vec2,
    /// Where it is going, in world units per second.
    pub(crate) vel: Vec2,
}

/// What one tick did to a ball.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Step {
    /// Where the ball ended up, and where it is going now.
    pub(crate) flight: Flight,
    /// Which paddle it touched on the way, if either.
    pub(crate) touched: Option<Side>,
    /// Whether it bounced off a wall.
    pub(crate) walled: bool,
}

/// Where in a tick's travel the ball's centre crosses a paddle's contact
/// plane, as a fraction of the tick.
///
/// There is no `Rect::sweep` in v1 and this is the eight lines the API document
/// says to write instead. `face` is the plane the centre must cross — the
/// paddle's inner face already moved out by the ball's radius — and `toward` is
/// the sign of the X travel that counts as approaching it: `-1.0` for the
/// paddle on the left, which the ball reaches while moving in `-X`. `reach` is
/// the interval of Y the paddle covers, already widened by the ball's radius.
///
/// CONTRACT: `None` unless all of these hold — the ball is moving toward
/// `face`, this tick's travel reaches it, the crossing happens *within* this
/// tick, and the crossing point falls inside `reach`. The three interesting
/// `None`s are a ball moving away, a ball that is already through (so it is
/// *leaving* by the same face and must not be batted back), and a ball that
/// crosses the plane past the end of the paddle.
///
/// The second of those has no test of its own on purpose. A ball already past
/// the plane and still travelling that way puts the numerator and the
/// denominator of `fraction` on opposite signs, so the `0.0..=1.0` test rejects
/// it — and an explicit `already past the plane` guard above that one is a
/// branch nothing can enter. It was written, and a deliberately broken build
/// with it deleted passed every check in `verify.rs`, which is how it was found
/// to be dead rather than untested.
pub(crate) fn crossing(
    from: Vec2,
    to: Vec2,
    face: f32,
    toward: f32,
    reach: (f32, f32),
) -> Option<f32> {
    let travel = to.x - from.x;
    if travel * toward <= 0.0 {
        return None; // going the other way, or not moving in X at all
    }
    if (to.x - face) * toward < 0.0 {
        return None; // this tick's travel does not get there
    }
    let fraction = (face - from.x) / travel;
    if !(0.0..=1.0).contains(&fraction) {
        // Not inside this tick: either the ball is already through the plane
        // and leaving by it, or the arithmetic disagrees with the tests above,
        // or something upstream is NaN.
        return None;
    }
    let at = from.y + (to.y - from.y) * fraction;
    if at < reach.0 || at > reach.1 {
        return None; // past the end of the paddle
    }
    Some(fraction)
}

/// One tick of the ball's flight, given where the two paddles now stand.
///
/// `paddles` is indexed by [`Side::index`]: the Y of each paddle's centre.
///
/// DELIBERATE: the paddles are treated as **stationary at their post-move
/// position** for the whole tick. They are not — a system earlier in this same
/// tick moved them — and nothing in the engine offers a sub-tick to appeal to,
/// so this game picks the end of the tick and says so here. It is wrong by at
/// most one tick of a paddle's travel and it is right about the case that
/// matters, which is a paddle moving to meet the ball; the alternative lets a
/// ball through a paddle that was closing on it, and no assertion about where
/// things ended up would see that happen.
///
/// The wall bounce is applied after the paddle sweep, by reflecting whatever
/// overshoot is left. For a plane that is exact rather than approximate, and
/// the corner — where a tick's travel could cross a wall *and* a paddle plane —
/// resolves paddle-first. At the speeds this game caps at, a tick's travel is
/// half a unit and the corner case is a millimetre of geometry, so it is a
/// known wrong answer in a place nobody can see rather than an oversight.
pub(crate) fn advance(flight: Flight, paddles: [f32; 2], dt: f32) -> Step {
    let mut pos = flight.pos;
    let mut vel = flight.vel;
    let mut remaining = 1.0_f32;
    let mut touched = None;

    // Twice, not once: a ball struck at the very tip of one paddle with almost
    // no tick left could in principle reach nothing else, but the loop costs
    // nothing and a single pass would silently swallow the second contact.
    for _ in 0..2 {
        let to = pos + vel * dt * remaining;
        let mut first: Option<(Side, f32)> = None;
        for side in [Side::Left, Side::Right] {
            let face = side.side_contact_x();
            let paddle_y = paddles[side.index()];
            let reach = (paddle_y - CONTACT_REACH, paddle_y + CONTACT_REACH);
            let Some(fraction) = crossing(pos, to, face, -side.outward(), reach) else {
                continue;
            };
            if first.is_none_or(|(_, best)| fraction < best) {
                first = Some((side, fraction));
            }
        }
        let Some((side, fraction)) = first else {
            pos = to;
            break;
        };

        let at = pos.lerp(to, fraction);
        // Where on the paddle it landed, -1 at the top edge and +1 at the
        // bottom, is the whole of this game's shot-making: the bounce angle is
        // that offset times MAX_BOUNCE, so the middle of the paddle sends the
        // ball straight back and the ends send it away steeply.
        let offset = ((at.y - paddles[side.index()]) / (PADDLE_SIZE.y * 0.5)).clamp(-1.0, 1.0);
        let (sine, cosine) = sin_cos(Radians(offset * MAX_BOUNCE.as_f32()));
        let speed = (vel.length() * BALL_SPEED_GAIN).min(BALL_SPEED_MAX);
        pos = at;
        vel = Vec2::new(side.outward() * cosine * speed, sine * speed);
        touched = Some(side);
        remaining *= 1.0 - fraction;
        if remaining <= 0.0 {
            break;
        }
    }

    // The walls. Reflecting the overshoot is the exact swept answer for a
    // plane, as long as only one of them is crossed in a tick — which the speed
    // cap guarantees on an 18-unit court.
    let mut walled = false;
    if pos.y > BALL_LIMIT {
        pos.y = 2.0 * BALL_LIMIT - pos.y;
        vel.y = -vel.y;
        walled = true;
    } else if pos.y < -BALL_LIMIT {
        pos.y = -2.0 * BALL_LIMIT - pos.y;
        vel.y = -vel.y;
        walled = true;
    }
    pos.y = pos.y.clamp(-BALL_LIMIT, BALL_LIMIT);

    Step {
        flight: Flight { pos, vel },
        touched,
        walled,
    }
}

impl Side {
    /// The contact plane this side's paddle presents to the ball's centre.
    pub(crate) const fn side_contact_x(self) -> f32 {
        match self {
            Side::Left => -CONTACT_X,
            Side::Right => CONTACT_X,
        }
    }
}

/// Which way the opponent leans this tick: -1 up, 0 still, +1 down.
///
/// A pure function of the ball and the paddle, deliberately, so a check can run
/// it forward beside the ball and ask where the opponent *will be* rather than
/// where it is standing. A branch inside `drive_the_paddles` would be the same
/// arithmetic and unaskable.
///
/// It chases where the ball *is* rather than where it will arrive, and only
/// while the ball is coming at it; the rest of the time it drifts back to the
/// middle. Both halves are what make it beatable: a shot steeper than
/// [`OPPONENT_SPEED`] outruns a chaser, and a chaser that has gone home has
/// ground to make up.
///
/// It does not centre on the ball, though — see [`OPPONENT_AIM`]. Y is down, so
/// a ball with a positive `vel.y` is descending and the paddle wants to be
/// *above* it, which is the smaller number.
pub(crate) fn opponent_push(ball: Flight, paddle_y: f32) -> f32 {
    let target = if ball.vel.x > 0.0 {
        let aim = if ball.vel.y > 0.0 {
            -OPPONENT_AIM
        } else {
            OPPONENT_AIM
        };
        (ball.pos.y + aim).clamp(-PADDLE_LIMIT, PADDLE_LIMIT)
    } else {
        0.0
    };
    let gap = target - paddle_y;
    if gap.abs() < OPPONENT_DEAD_BAND {
        return 0.0;
    }
    gap.signum()
}

// --- the game, as systems ------------------------------------------------

/// The game's configuration, shared by the window and the verify run so that
/// what is checked is what a person plays.
pub(crate) fn config() -> GameConfig {
    GameConfig {
        title: "jidousha — pong",
        seed: 20_250_819,
        ..GameConfig::default()
    }
}

/// Every system this game has, in one place, in the order they run.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, run_the_clock);
    // Before the ball moves: `advance` reads the paddles where this system has
    // just put them, which is the DELIBERATE choice documented on it.
    app.add_system(Update, drive_the_paddles);
    app.add_system(Update, move_the_ball);
    app.add_system(Update, keep_score);
    // The play goes down first and the court after it, so the bands have to
    // sort them the other way round. Where a game's submission order already
    // agrees with its layers, no assertion over drawn quads can see a band at
    // all — so the disagreement is arranged rather than hoped for.
    app.add_system(Draw, draw_the_play);
    app.add_system(Draw, draw_the_court);
    app.add_system(Draw, draw_the_furniture);
}

fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        clear_color: COURT,
        height: VIEW_HEIGHT,
        ..Camera::default()
    });
    world.insert_resource(Scoreboard::fresh());

    for (side, control) in [
        (Side::Left, Control::Player),
        (Side::Right, Control::Opponent),
    ] {
        let paddle = world.spawn();
        world.insert(paddle, Transform::at(Vec2::new(side.paddle_x(), 0.0)));
        world.insert(paddle, Paddle { side, control });
    }

    let ball = world.spawn();
    world.insert(ball, Transform::at(Vec2::ZERO));
    world.insert(ball, Ball { vel: Vec2::ZERO });
}

/// Count the serve down, put the ball back in play, and take Space for a
/// rematch once a match is over.
fn run_the_clock(world: &mut World) {
    let space = world
        .find_resource::<Input>()
        .is_some_and(|input| input.just_pressed(Key::Space));
    match world.resource::<Scoreboard>().stage {
        Stage::Rally => {}
        Stage::Over { .. } => {
            if space {
                world.insert_resource(Scoreboard::fresh());
                park_everything(world);
            }
        }
        Stage::Serving { ticks_left, toward } => {
            if ticks_left > 1 && !space {
                world.resource_mut::<Scoreboard>().stage = Stage::Serving {
                    ticks_left: ticks_left - 1,
                    toward,
                };
                return;
            }
            // A serve wants a little variety and the same variety every run:
            // the engine's Rng is seeded from GameConfig, so a replay of this
            // session serves the same balls.
            let spread = world.resource_mut::<Rng>().next_f32() * 2.0 - 1.0;
            let (sine, cosine) = sin_cos(Radians(spread * SERVE_SPREAD.as_f32()));
            let vel = Vec2::new(toward.other().outward() * cosine, sine) * BALL_SPEED_START;
            for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
                transform.pos = Vec2::ZERO;
                ball.vel = vel;
            }
            world.resource_mut::<Scoreboard>().stage = Stage::Rally;
        }
    }
}

/// Put the ball at the centre and both paddles back in the middle.
fn park_everything(world: &mut World) {
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        transform.pos = Vec2::ZERO;
        ball.vel = Vec2::ZERO;
    }
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        transform.pos = Vec2::new(paddle.side.paddle_x(), 0.0);
    }
}

/// Move the player's paddle from the keyboard and the opponent's from
/// [`opponent_push`].
fn drive_the_paddles(world: &mut World) {
    if matches!(world.resource::<Scoreboard>().stage, Stage::Over { .. }) {
        return;
    }
    // Read what both decisions need, then write: a `query_mut` holds the world
    // for as long as it iterates, so the ball cannot be looked at from inside
    // the loop that moves the paddles.
    let player = match world.find_resource::<Input>() {
        // Startup runs inside the first tick, before that tick's Input exists.
        None => 0.0,
        Some(input) => {
            let down = input.held(Key::S) || input.held(Key::ArrowDown);
            let up = input.held(Key::W) || input.held(Key::ArrowUp);
            f32::from(down) - f32::from(up)
        }
    };
    let Some(ball) = ball_flight(world) else {
        return;
    };
    let step = world.resource::<Time>().fixed_dt.as_f32();

    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        let (push, speed) = match paddle.control {
            Control::Player => (player, PLAYER_SPEED),
            Control::Opponent => (opponent_push(ball, transform.pos.y), OPPONENT_SPEED),
        };
        transform.pos.y =
            (transform.pos.y + push * speed * step).clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
    }
}

/// One tick of the ball, through [`advance`].
fn move_the_ball(world: &mut World) {
    if !matches!(world.resource::<Scoreboard>().stage, Stage::Rally) {
        return;
    }
    let mut paddles = [0.0_f32; 2];
    for (_, transform, paddle) in world.query::<(&Transform, &Paddle)>() {
        paddles[paddle.side.index()] = transform.pos.y;
    }
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        let step = advance(
            Flight {
                pos: transform.pos,
                vel: ball.vel,
            },
            paddles,
            dt,
        );
        transform.pos = step.flight.pos;
        ball.vel = step.flight.vel;
    }
}

/// A ball past a goal line is a point, and a fifth point is a match.
fn keep_score(world: &mut World) {
    if !matches!(world.resource::<Scoreboard>().stage, Stage::Rally) {
        return;
    }
    let Some(ball) = ball_flight(world) else {
        return;
    };
    let scorer = if ball.pos.x > GOAL_LINE {
        Side::Left
    } else if ball.pos.x < -GOAL_LINE {
        Side::Right
    } else {
        return;
    };

    let board = world.resource_mut::<Scoreboard>();
    match scorer {
        Side::Left => board.left += 1,
        Side::Right => board.right += 1,
    }
    board.stage = if board.points(scorer) >= WINNING_SCORE {
        Stage::Over { winner: scorer }
    } else {
        Stage::Serving {
            ticks_left: SERVE_PAUSE,
            // Served at whoever just conceded, so the loser of the point plays
            // the next ball.
            toward: scorer.other(),
        }
    };
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        transform.pos = Vec2::ZERO;
        ball.vel = Vec2::ZERO;
    }
}

/// The ball's position and velocity, if there is a ball.
pub(crate) fn ball_flight(world: &World) -> Option<Flight> {
    world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| Flight {
            pos: transform.pos,
            vel: ball.vel,
        })
        .next()
}

// --- drawing --------------------------------------------------------------

/// The paddles and the ball.
fn draw_the_play(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PLAY);
    for (_, transform, paddle) in ctx.world.query::<(&Transform, &Paddle)>() {
        let color = match paddle.control {
            Control::Player => PLAYER_COLOR,
            Control::Opponent => OPPONENT_COLOR,
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

/// The two walls and the dashed centre line.
///
/// Submitted after the play and drawn behind it, which is the band doing its
/// job rather than the submission order.
fn draw_the_court(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::FIELD);
    for wall in [-COURT_HALF_HEIGHT, COURT_HALF_HEIGHT] {
        ctx.line(
            Vec2::new(-COURT_HALF_WIDTH, wall),
            Vec2::new(COURT_HALF_WIDTH, wall),
            WALL_THICKNESS,
            WALL,
            depth,
        );
    }
    for index in 0..dash_count() {
        ctx.rect(
            Rect::from_center_size(Vec2::new(0.0, dash_y(index)), DASH_SIZE),
            NET,
            depth,
        );
    }
}

/// How many dashes the centre line has.
pub(crate) fn dash_count() -> usize {
    // Whole pitches that fit between the walls, leaving the wall lines clear.
    let span = (COURT_HALF_HEIGHT - DASH_SIZE.y) * 2.0;
    (span / DASH_PITCH) as usize + 1
}

/// Where the centre line's `index`th dash sits.
pub(crate) fn dash_y(index: usize) -> f32 {
    let count = dash_count() as f32;
    (index as f32 - (count - 1.0) * 0.5) * DASH_PITCH
}

/// The score, the hint, and the banner when there is one.
fn draw_the_furniture(ctx: &mut DrawCtx) {
    let board = *ctx.world.resource::<Scoreboard>();
    let style = TextStyle {
        size: SCORE_SIZE,
        color: Color::WHITE,
        depth: Depth::layer(layers::UI),
    };
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
        color: Color::rgba(0.72, 0.82, 0.92, 0.85),
        depth: Depth::layer(layers::UI),
    };
    ctx.text(
        Vec2::new(
            -hint.width_of(HINT) * 0.5,
            VIEW_HEIGHT * 0.5 - HINT_SIZE - 0.25,
        ),
        HINT,
        hint,
    );

    let Stage::Over { winner } = board.stage else {
        return;
    };
    let banner = TextStyle {
        size: BANNER_SIZE,
        color: match winner {
            Side::Left => PLAYER_COLOR,
            Side::Right => OPPONENT_COLOR,
        },
        depth: Depth::layer(layers::UI),
    };
    let headline = match winner {
        Side::Left => BANNER_WON,
        Side::Right => BANNER_LOST,
    };
    // One `ctx.text` per line, each centred by its own width. `width_of` is the
    // width of the *widest* line, so centring a two-line block by one call puts
    // the long line in the middle and hangs the short one off to the left.
    ctx.text(
        Vec2::new(-banner.width_of(headline) * 0.5, -1.6),
        headline,
        banner,
    );
    let under = TextStyle {
        size: BANNER_SUB_SIZE,
        color: Color::rgba(0.9, 0.9, 0.95, 0.9),
        depth: Depth::layer(layers::UI),
    };
    ctx.text(
        Vec2::new(-under.width_of(BANNER_SUB) * 0.5, 1.1),
        BANNER_SUB,
        under,
    );
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("W/S (or the arrow keys) move the left paddle. close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        // Display, not Debug: `RunError`'s Display is the engine's four-part
        // message, and returning it from main prints a struct dump instead.
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
