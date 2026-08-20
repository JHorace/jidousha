//! Pong, written against the engine rather than inside it.
//!
//! Two paddles, a ball, a score. `W` and `S` move the left paddle; the right one
//! is played by `opponent_target` below. First to five wins the match, and
//! `Space` starts another.
//!
//! Everything the game is made of is a shape: a paddle is `ctx.rect`, the ball
//! is `ctx.circle`, the score is `ctx.text`, and the court markings are lines
//! and a column of small rectangles. No asset ever loads, so this runs from the
//! first frame on a machine with no files at all.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//!
//! # The two decisions this file makes on purpose
//!
//! **A collider is stationary at its post-move position.** The paddles move in
//! `drive_the_player` and `drive_the_opponent`, and the ball sweeps against them
//! in `move_the_ball`, which is registered after both. So a paddle closing on
//! the ball is met where it ended the tick, not where it started — wrong by at
//! most one tick of a paddle's travel, and right about the case the game is
//! about. `verify.rs` asserts the registration order, because nothing else can
//! see it.
//!
//! **The ball's step and the opponent's decision are free functions.** `step_ball`
//! and `opponent_target` are called by the systems that act on them rather than
//! written inside them, so the check can roll the game forward without a running
//! world to fork. That is what `controller.rs` does thirteen times a tick.

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod controller;
mod verify;

// ---------------------------------------------------------------------------
// The court
// ---------------------------------------------------------------------------

/// How big the window opens, and the aspect every layout constant below assumes.
///
/// The layout is in constants rather than derived from `visible_bounds()`, which
/// is the prototype trade the API document names: it makes every bounds
/// assertion in `verify.rs` a statement about one known aspect, and it means a
/// player who drags the window narrower than 16:9 pulls the goal lines in past
/// the paddles with no check able to see it. Stated, chosen, and given up.
const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);

/// How many world units the camera spans vertically.
const VIEW_HEIGHT: f32 = 20.0;

/// Half the court: the goal lines stand at `±COURT.x`, the walls at `±COURT.y`.
///
/// Inside the camera on both axes at 16:9 — the view is 35.56 x 20 — so the
/// court has a margin around it and the score has somewhere to sit.
const COURT: Vec2 = Vec2::new(16.6, 9.0);

/// What the court is cleared to.
///
/// Dark on purpose: a white ball has to read against it, which `verify.rs`
/// asserts as a requirement rather than as a comparison with this constant.
const COURT_COLOR: Color = Color::rgb(0.05, 0.07, 0.10);

/// What the court markings are drawn in.
///
/// Alpha blends in linear light, so this reads far brighter than 0.14 suggests.
/// Picked by looking at a capture, which is the only way to pick it.
const FIELD_LINE: Color = Color::rgba(1.0, 1.0, 1.0, 0.14);

/// How thick the court's border lines are, in world units.
const FIELD_THICKNESS: f32 = 0.1;

// ---------------------------------------------------------------------------
// The paddles
// ---------------------------------------------------------------------------

/// How big a paddle is, in world units.
///
/// The `x` is not cosmetic: it is the ceiling on how far the ball may travel in
/// one tick, because nothing sweeps for you and a ball thicker than its target
/// steps clean through. `MAX_BALL_SPEED * fixed_dt` must stay under it, and
/// `verify.rs` asserts exactly that against the `fixed_dt` the engine hands it.
const PADDLE_SIZE: Vec2 = Vec2::new(1.1, 3.2);

/// How far from the centre a paddle's own centre may travel.
///
/// The wall, less half a paddle, so a paddle stops flush against the court.
const PADDLE_LIMIT: f32 = COURT.y - PADDLE_SIZE.y * 0.5;

/// How far in from the goal line a paddle stands.
const PADDLE_INSET: f32 = 1.6;

/// How fast the player's paddle moves, in world units per second.
const PLAYER_SPEED: f32 = 20.0;

/// How fast the opponent's paddle moves, in world units per second.
///
/// Slower than the player's on purpose, and picked by arithmetic rather than by
/// what looked fair: a return struck at `MAX_BOUNCE` off a serve carries
/// `23 * sin 50` = 17.6 units per second of vertical, so a paddle that chases at
/// 18 is beaten by a steep shot from part-way through a rally rather than only
/// at its very end. Every number in this block was picked by playing the game
/// three ways headless and reading the scores, not by looking fair — looking
/// fair is not the test, and the first set that looked fair produced a
/// forty-three-touch rally that never ended.
const OPPONENT_SPEED: f32 = 18.0;

/// How far off target the opponent will sit rather than chase.
///
/// Without it the paddle judders around the ball's line every tick.
const OPPONENT_DEADZONE: f32 = 0.25;

/// How far off its own centre the opponent tries to meet the ball, as a
/// fraction of its reach.
///
/// Not a flourish. An opponent that centres perfectly on the ball returns it
/// dead flat, and a player who also centres returns it dead flat back: both
/// hold a groove neither can lose and the match ends 0-0 after one enormous
/// rally. Leaning into the shot — striking low on the paddle when the ball is
/// already falling — means no return this game plays is ever flat.
const OPPONENT_AIM: f32 = 0.75;

/// What the player's paddle is drawn in.
const PLAYER_COLOR: Color = Color::rgb(0.45, 0.95, 1.0);

/// What the opponent's paddle is drawn in.
const OPPONENT_COLOR: Color = Color::rgb(1.0, 0.6, 0.45);

// ---------------------------------------------------------------------------
// The ball
// ---------------------------------------------------------------------------

/// How big the ball is, as a radius in world units.
const BALL_RADIUS: f32 = 0.36;

/// What the ball is drawn in.
const BALL_COLOR: Color = Color::rgb(1.0, 1.0, 1.0);

/// How fast the ball leaves a serve, in world units per second.
const SERVE_SPEED: f32 = 23.0;

/// What each return multiplies the ball's speed by.
const SPEEDUP: f32 = 1.12;

/// The fastest the ball may ever go, in world units per second.
///
/// Chosen against `PADDLE_SIZE.x`, not against how the game feels: at the
/// engine's default sixtieth of a second, 42 units per second is 0.7 of a unit
/// of travel per tick against a paddle 1.1 thick, so the ball cannot cross a
/// paddle inside one tick however long the rally runs. The two numbers were
/// raised together and in that order: the paddle first, then this. Raising this
/// alone is how a ball starts passing through paddles, and no played session
/// can see it — `verify.rs` asks the sweep about its contract directly instead.
const MAX_BALL_SPEED: f32 = 42.0;

/// The steepest a return may leave a paddle, measured from straight across.
///
/// Written in degrees because `Radians::from_degrees` is a `const fn` and a
/// hand-typed float near a fraction of pi is a clippy error.
const MAX_BOUNCE: Radians = Radians::from_degrees(50.0);

/// The steepest a serve may leave the centre spot.
const MAX_SERVE_ANGLE: Radians = Radians::from_degrees(30.0);

// ---------------------------------------------------------------------------
// The match
// ---------------------------------------------------------------------------

/// How many points win a match.
const MATCH_POINT: u32 = 5;

/// How many ticks the ball waits at the centre spot between points.
///
/// Ticks rather than seconds: the tick is the canonical timeline, and sixty of
/// them is a second, so this is three quarters of one.
const SERVE_PAUSE: u64 = 45;

/// Draw bands, named once so no number appears at a call site.
mod layers {
    /// The court and its markings, behind everything.
    pub const FIELD: i16 = -1;
    /// The paddles and the ball.
    pub const PLAY: i16 = 0;
    /// Score, hint and banner, over everything.
    pub const UI: i16 = 1;
}

// ---------------------------------------------------------------------------
// The world
// ---------------------------------------------------------------------------

/// Which end of the court something belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// The end the player defends, at negative X.
    Left,
    /// The end the opponent defends, at positive X.
    Right,
}

impl Side {
    /// The other end.
    pub(crate) fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// `-1.0` for the left end, `1.0` for the right one.
    pub(crate) fn sign(self) -> f32 {
        match self {
            Side::Left => -1.0,
            Side::Right => 1.0,
        }
    }

    /// This side's slot in the pairs the scoreboard and the tally keep.
    pub(crate) fn index(self) -> usize {
        match self {
            Side::Left => 0,
            Side::Right => 1,
        }
    }

    /// How the end screen says this side won.
    ///
    /// Two spellings rather than one with an `S` bolted on: "YOU WINS" is what
    /// the single spelling produces, and it is a fault no assertion over drawn
    /// quads can see — right glyph count, right width, correctly centred, every
    /// character printable. Only the captured picture showed it.
    pub(crate) fn victory(self) -> &'static str {
        match self {
            Side::Left => "YOU WIN",
            Side::Right => "CPU WINS",
        }
    }
}

/// Who moves a paddle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    /// The person at the keyboard.
    Keys,
    /// `opponent_target`.
    Computer,
}

/// A paddle: which end it defends, who moves it, and how fast.
///
/// One component carrying an enum rather than two marker components and a
/// `With<>` filter — it keeps the query tuples short and it gives the check
/// something to sort both paddles by that is the game's own, which query
/// iteration order is explicitly not.
#[derive(Clone, Copy)]
pub(crate) struct Paddle {
    /// Which end this paddle defends.
    pub(crate) side: Side,
    /// Who moves it.
    pub(crate) control: Control,
    /// World units per second.
    pub(crate) speed: f32,
}
impl Component for Paddle {}

/// The ball, and where it is going.
#[derive(Clone, Copy)]
pub(crate) struct Ball {
    /// World units per second, direction and magnitude in one.
    pub(crate) vel: Vec2,
}
impl Component for Ball {}

/// What the match is doing right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// The ball is parked at the centre spot, counting down to a serve.
    Serving {
        /// Ticks still to wait.
        ticks_left: u64,
        /// Which end the serve will travel towards.
        toward: Side,
    },
    /// The ball is live.
    Rally,
    /// Somebody reached `MATCH_POINT`.
    Over {
        /// Who won.
        winner: Side,
    },
}

/// The score and what the match is doing — the one resource that selects a
/// screen, so a check can stage any of them by setting it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Round {
    /// Points, by `Side::index`.
    pub(crate) points: [u32; 2],
    /// What the match is doing.
    pub(crate) stage: Stage,
}
impl Resource for Round {}

impl Round {
    /// A fresh match, serving towards the player.
    pub(crate) fn new() -> Round {
        Round {
            points: [0, 0],
            stage: Stage::Serving {
                ticks_left: SERVE_PAUSE,
                toward: Side::Left,
            },
        }
    }
}

/// What the rally has done: the numbers the game shows and the check reports.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Tally {
    /// Returns in the rally being played now.
    pub(crate) touches: u32,
    /// The longest rally of the match, in returns.
    pub(crate) longest: u32,
    /// The fastest the ball has been, in world units per second.
    pub(crate) fastest: f32,
    /// Returns made by each side all match, by `Side::index`.
    pub(crate) returns: [u32; 2],
}
impl Resource for Tally {}

// ---------------------------------------------------------------------------
// The decisions, as functions a check can call
// ---------------------------------------------------------------------------

/// The face a paddle presents to the ball, in the ball's *centre's* terms.
///
/// The ball is a disc, so the plane its centre must not cross is the paddle's
/// near face pushed out by the radius, and the span it may strike is the
/// paddle's half-height grown by the same. Doing it here rather than in the
/// crossing test is what keeps that test about arithmetic.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Face {
    /// The ball-centre X at which contact happens.
    pub(crate) plane: f32,
    /// The sign of the ball's X velocity that can reach this face.
    pub(crate) approach: f32,
    /// The paddle's centre, in world units.
    pub(crate) centre: Vec2,
    /// How far from `centre.y` the ball's centre may be and still touch.
    pub(crate) reach: f32,
}

/// Where a paddle stands, given its side.
pub(crate) fn paddle_home(side: Side) -> f32 {
    side.sign() * (COURT.x - PADDLE_INSET)
}

/// The face `side`'s paddle presents, standing with its centre at `centre`.
pub(crate) fn face_of(centre: Vec2, side: Side) -> Face {
    Face {
        // The left paddle's near face is its right-hand one, and the ball's
        // centre stops a radius short of it.
        plane: centre.x - side.sign() * (PADDLE_SIZE.x * 0.5 + BALL_RADIUS),
        approach: side.sign(),
        centre,
        reach: PADDLE_SIZE.y * 0.5 + BALL_RADIUS,
    }
}

/// How far into a tick's travel from `from` to `to` the ball crossed `face`.
///
/// `None` when it did not: travelling the wrong way, already past the plane,
/// short of it at the end of the tick, or crossing it past the end of the
/// paddle. Written out rather than reached for, because there is no `Rect::sweep`
/// and the eight lines are the game's model rather than the engine's.
pub(crate) fn crossing(from: Vec2, to: Vec2, face: Face) -> Option<f32> {
    let travel = (to.x - from.x) * face.approach;
    let before = (face.plane - from.x) * face.approach;
    let after = (face.plane - to.x) * face.approach;
    // Each condition written the positive way round and the *whole* of it
    // negated once, rather than three negated float comparisons. A negated
    // comparison is true for NaN, so a velocity that went to NaN would report a
    // contact at a NaN fraction of the tick and the ball would leave at a NaN
    // position, silently, for the rest of the run. Written this way a NaN fails
    // every conjunct and the answer is "no contact", which is the safe one.
    // (Clippy also rejects `!(a > b)` outright, which is how this got written
    // twice.)
    let approaching = travel > 0.0; // not standing still or going the other way
    let in_front = before >= 0.0; // not already through, leaving as it came
    let reached = after <= 0.0; // this tick's travel did not stop short
    if !(approaching && in_front && reached) {
        return None;
    }
    let at = before / travel;
    let contact = from.y + (to.y - from.y) * at;
    let on_the_paddle = (contact - face.centre.y).abs() <= face.reach;
    if !on_the_paddle {
        return None;
    }
    Some(at)
}

/// The velocity a ball leaves a paddle with, struck at `contact_y`.
///
/// Where on the paddle it lands is the whole of Pong's skill: the middle sends
/// it straight back, the ends send it away at `MAX_BOUNCE`.
pub(crate) fn rebound(contact_y: f32, face: Face, speed: f32) -> Vec2 {
    let offset = ((contact_y - face.centre.y) / face.reach).clamp(-1.0, 1.0);
    let (sine, cosine) = sin_cos(Radians(MAX_BOUNCE.as_f32() * offset));
    // Away from the paddle: the face that only a ball travelling `approach` can
    // reach sends it back the other way.
    Vec2::new(-face.approach * cosine * speed, sine * speed)
}

/// The ball after bouncing off the court's walls, if it reached one.
pub(crate) fn off_the_walls(pos: Vec2, vel: Vec2) -> (Vec2, Vec2) {
    let limit = COURT.y - BALL_RADIUS;
    if pos.y > limit {
        return (
            Vec2::new(pos.x, limit * 2.0 - pos.y),
            Vec2::new(vel.x, -vel.y.abs()),
        );
    }
    if pos.y < -limit {
        return (
            Vec2::new(pos.x, -limit * 2.0 - pos.y),
            Vec2::new(vel.x, vel.y.abs()),
        );
    }
    (pos, vel)
}

/// What one tick does to the ball.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Step {
    /// Where it ends the tick.
    pub(crate) pos: Vec2,
    /// How it is travelling at the end of the tick.
    pub(crate) vel: Vec2,
    /// Which paddle returned it this tick, if either did.
    pub(crate) hit: Option<Side>,
}

/// One tick of the ball, as a function of everything that can move it.
///
/// Free rather than buried in `move_the_ball` because a check cannot fork a
/// running simulation: this function *is* the model a controller rolls forward
/// to work out where the ball is going. `paddles` are taken where they stand at
/// the moment of the call, which — given the registration order this game
/// chose — means where they finished this tick's move.
pub(crate) fn step_ball(pos: Vec2, vel: Vec2, paddles: &[(Vec2, Side)], dt: f32) -> Step {
    let target = pos + vel * dt;
    let mut first: Option<(f32, Side, Face)> = None;
    for (centre, side) in paddles {
        let face = face_of(*centre, *side);
        if let Some(at) = crossing(pos, target, face)
            && first.is_none_or(|(best, _, _)| at < best)
        {
            first = Some((at, *side, face));
        }
    }
    let (moved, going, hit) = match first {
        None => (target, vel, None),
        Some((at, side, face)) => {
            let contact = pos.lerp(target, at);
            let speed = (vel.length() * SPEEDUP).min(MAX_BALL_SPEED);
            let away = rebound(contact.y, face, speed);
            // The rest of the tick, travelling the new way.
            (contact + away * dt * (1.0 - at), away, Some(side))
        }
    };
    let (pos, vel) = off_the_walls(moved, going);
    Step { pos, vel, hit }
}

/// Where the opponent's paddle wants its centre to be.
///
/// A ball heading the other way is somebody else's problem, so the paddle
/// returns to the middle — which is what makes it beatable rather than a wall,
/// and it is the one line of this game a check can ask about directly.
pub(crate) fn opponent_target(ball_pos: Vec2, ball_vel: Vec2, side: Side) -> f32 {
    let coming = (paddle_home(side) - ball_pos.x) * ball_vel.x > 0.0;
    if !coming {
        return 0.0;
    }
    // Lean into the shot: stand `OPPONENT_AIM` of a reach on the near side of
    // the ball, so it strikes off-centre and leaves at an angle. `signum`
    // answers 1.0 for zero, which is what makes a dead-flat ball get leaned on
    // too — and a sign that changes only at a wall or a paddle is a sign that
    // does not judder from one tick to the next.
    let reach = PADDLE_SIZE.y * 0.5 + BALL_RADIUS;
    ball_pos.y - OPPONENT_AIM * reach * ball_vel.y.signum()
}

/// A paddle's centre after one tick of chasing `target` at `speed`.
pub(crate) fn chase(from: f32, target: f32, speed: f32, dt: f32) -> f32 {
    if (target - from).abs() < OPPONENT_DEADZONE {
        return from;
    }
    let step = (target - from).signum() * speed * dt;
    let moved = if (target - from).abs() < step.abs() {
        target
    } else {
        from + step
    };
    moved.clamp(-PADDLE_LIMIT, PADDLE_LIMIT)
}

// ---------------------------------------------------------------------------
// The strings
// ---------------------------------------------------------------------------

/// What the bottom of the screen says while a match is being played.
///
/// A function rather than a literal inside the draw system so that a check can
/// ask the game for the exact characters it draws: the font draws an unknown
/// character as a box at exactly a letter's width, so no assertion over what was
/// drawn can tell a curly quote from a straight one.
pub(crate) fn hint_text(tally: &Tally) -> String {
    format!(
        "W and S to move   first to {MATCH_POINT}   rally {}   best {}",
        tally.touches, tally.longest
    )
}

/// The two lines the end screen shows.
///
/// Two, and drawn as two `ctx.text` calls each centred by its own width:
/// `width_of` measures the widest line only, so centring a block by it hangs
/// every shorter line off to the left, visibly crooked and silently on screen.
pub(crate) fn banner_lines(winner: Side, points: [u32; 2]) -> [String; 2] {
    [
        format!(
            "{} {} - {}",
            winner.victory(),
            points[winner.index()],
            points[winner.other().index()]
        ),
        "press space for a new match".to_owned(),
    ]
}

// ---------------------------------------------------------------------------
// Wiring
// ---------------------------------------------------------------------------

/// The game's configuration, shared by the window and the verify run, so that
/// what is checked is what a person plays.
pub(crate) fn config() -> GameConfig {
    GameConfig {
        title: "jidousha — pong",
        seed: 7,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// Every system this game has, in the order they run.
///
/// The order is the decision, and nothing but this list holds it: both paddles
/// move before the ball sweeps against them, so a paddle closing on the ball
/// meets it. `verify.rs` asserts that out of `schedule_debug()`, which is the
/// only instrument that can see a tidy-up swapping two of these lines.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, run_the_match);
    app.add_system(Update, drive_the_player);
    app.add_system(Update, drive_the_opponent);
    app.add_system(Update, move_the_ball);
    app.add_system(Update, score_the_point);
    // The play goes down first and the court after it, so the court's markings
    // sort *behind* the ball that was submitted before them. That disagreement
    // between submission order and band is the only arrangement in which a
    // check can see a layer at all.
    app.add_system(Draw, draw_the_play);
    app.add_system(Draw, draw_the_court);
    app.add_system(Draw, draw_the_score);
    app.add_system(Draw, draw_the_words);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("W and S move the left paddle. first to {MATCH_POINT}; close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        // Display, not Debug: `RunError`'s Display is the engine's four-part
        // message and its Debug is a struct dump.
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Put the court, the paddles and the ball in the world.
fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        center: Vec2::ZERO,
        height: VIEW_HEIGHT,
        clear_color: COURT_COLOR,
        ..Camera::default()
    });
    world.insert_resource(Round::new());
    world.insert_resource(Tally::default());

    for (side, control, speed) in [
        (Side::Left, Control::Keys, PLAYER_SPEED),
        (Side::Right, Control::Computer, OPPONENT_SPEED),
    ] {
        let paddle = world.spawn();
        world.insert(paddle, Transform::at(Vec2::new(paddle_home(side), 0.0)));
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
    world.insert(ball, Ball { vel: Vec2::ZERO });
}

/// Count down a serve, launch it, and start a new match when one is asked for.
fn run_the_match(world: &mut World) {
    let restart = world
        .find_resource::<Input>()
        .is_some_and(|input| input.just_pressed(Key::Space));
    let round = *world.resource::<Round>();

    if let Stage::Over { .. } = round.stage {
        if restart {
            world.insert_resource(Round::new());
            world.insert_resource(Tally::default());
            park_the_ball(world);
        }
        return;
    }

    let Stage::Serving { ticks_left, toward } = round.stage else {
        return;
    };
    if ticks_left > 0 {
        park_the_ball(world);
        world.resource_mut::<Round>().stage = Stage::Serving {
            ticks_left: ticks_left - 1,
            toward,
        };
        return;
    }

    // A seeded draw, so the same match plays the same way every run.
    let roll = world.resource_mut::<Rng>().next_f32();
    let angle = Radians(MAX_SERVE_ANGLE.as_f32() * (roll * 2.0 - 1.0));
    let (sine, cosine) = sin_cos(angle);
    let vel = Vec2::new(toward.sign() * cosine * SERVE_SPEED, sine * SERVE_SPEED);
    let ball = world
        .query::<(&Ball, &Transform)>()
        .map(|(entity, _, _)| entity)
        .next();
    if let Some(ball) = ball {
        world.component_mut::<Transform>(ball).pos = Vec2::ZERO;
        world.component_mut::<Ball>(ball).vel = vel;
    }
    world.resource_mut::<Round>().stage = Stage::Rally;
    world.resource_mut::<Tally>().touches = 0;
}

/// Hold the ball on the centre spot, still.
fn park_the_ball(world: &mut World) {
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        transform.pos = Vec2::ZERO;
        ball.vel = Vec2::ZERO;
    }
}

/// Move the player's paddle with W and S, clamped to the court.
fn drive_the_player(world: &mut World) {
    // `Startup` runs inside the first tick, before that tick's `Input` exists,
    // and a headless run has none at all unless a check puts one in.
    let direction = match world.find_resource::<Input>() {
        None => return,
        Some(input) => f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    };
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.control == Control::Keys {
            transform.pos.y = (transform.pos.y + direction * paddle.speed * dt)
                .clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
        }
    }
}

/// Move the opponent's paddle towards `opponent_target`.
fn drive_the_opponent(world: &mut World) {
    // Read first, write second: a `query_mut` holds the world for as long as it
    // iterates, so the ball has to be looked up before the paddles are moved.
    let Some((_, ball_transform, ball)) = world.query::<(&Transform, &Ball)>().next() else {
        return;
    };
    let (ball_pos, ball_vel) = (ball_transform.pos, ball.vel);
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.control == Control::Computer {
            let target = opponent_target(ball_pos, ball_vel, paddle.side);
            transform.pos.y = chase(transform.pos.y, target, paddle.speed, dt);
        }
    }
}

/// Sweep the ball through this tick, off the walls and off the paddles.
fn move_the_ball(world: &mut World) {
    if world.resource::<Round>().stage != Stage::Rally {
        return;
    }
    let paddles: Vec<(Vec2, Side)> = world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (transform.pos, paddle.side))
        .collect();
    let Some((entity, transform, ball)) = world.query::<(&Transform, &Ball)>().next() else {
        return;
    };
    let (pos, vel) = (transform.pos, ball.vel);
    let dt = world.resource::<Time>().fixed_dt.as_f32();

    let step = step_ball(pos, vel, &paddles, dt);
    world.component_mut::<Transform>(entity).pos = step.pos;
    world.component_mut::<Ball>(entity).vel = step.vel;

    let tally = world.resource_mut::<Tally>();
    tally.fastest = tally.fastest.max(step.vel.length());
    if let Some(side) = step.hit {
        tally.touches += 1;
        tally.longest = tally.longest.max(tally.touches);
        tally.returns[side.index()] += 1;
    }
}

/// Award a point to whoever the ball did not go past, and set up the next serve.
fn score_the_point(world: &mut World) {
    if world.resource::<Round>().stage != Stage::Rally {
        return;
    }
    let Some((_, transform, _)) = world.query::<(&Transform, &Ball)>().next() else {
        return;
    };
    let x = transform.pos.x;
    let conceded = if x > COURT.x {
        Side::Right
    } else if x < -COURT.x {
        Side::Left
    } else {
        return;
    };

    let round = world.resource_mut::<Round>();
    round.points[conceded.other().index()] += 1;
    round.stage = if round.points[conceded.other().index()] >= MATCH_POINT {
        Stage::Over {
            winner: conceded.other(),
        }
    } else {
        Stage::Serving {
            ticks_left: SERVE_PAUSE,
            toward: conceded,
        }
    };
    park_the_ball(world);
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// The paddles and the ball.
fn draw_the_play(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PLAY);
    // Straight out of the query: a Draw system's iterator borrows the world, not
    // the context, so there is no two-pass collect to write here.
    for (_, transform, paddle) in ctx.world.query::<(&Transform, &Paddle)>() {
        let color = match paddle.control {
            Control::Keys => PLAYER_COLOR,
            Control::Computer => OPPONENT_COLOR,
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

/// The court: a border and a dashed centre line.
///
/// Submitted after the play and drawn behind it, which is the band doing work
/// the submission order cannot.
fn draw_the_court(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::FIELD);
    let corners = [
        Vec2::new(-COURT.x, -COURT.y),
        Vec2::new(COURT.x, -COURT.y),
        Vec2::new(COURT.x, COURT.y),
        Vec2::new(-COURT.x, COURT.y),
    ];
    for index in 0..4 {
        ctx.line(
            corners[index],
            corners[(index + 1) % 4],
            FIELD_THICKNESS,
            FIELD_LINE,
            depth,
        );
    }
    // A dashed centre marking is a column of rectangles: there is no dash
    // pattern anywhere in the drawing vocabulary, and this is what one is.
    let dash = Vec2::new(0.18, 0.7);
    let mut y = -COURT.y + dash.y;
    while y < COURT.y - dash.y {
        ctx.rect(
            Rect::from_center_size(Vec2::new(0.0, y), dash),
            FIELD_LINE,
            depth,
        );
        y += dash.y * 2.0;
    }
}

/// The score: one number either side of the centre line, evenly set.
fn draw_the_score(ctx: &mut DrawCtx) {
    let round = ctx.world.resource::<Round>();
    let style = TextStyle {
        size: 2.2,
        color: Color::rgba(1.0, 1.0, 1.0, 0.9),
        depth: Depth::layer(layers::UI),
    };
    // Y is down, so the top of the court is the smaller number.
    let top = -COURT.y + 0.6;
    let gap = 1.6;
    let left = format!("{}", round.points[Side::Left.index()]);
    let right = format!("{}", round.points[Side::Right.index()]);
    ctx.text(Vec2::new(-gap - style.width_of(&left), top), &left, style);
    ctx.text(Vec2::new(gap, top), &right, style);
}

/// The hint line, or the end screen when there is one.
fn draw_the_words(ctx: &mut DrawCtx) {
    let round = ctx.world.resource::<Round>();
    let tally = ctx.world.resource::<Tally>();

    if let Stage::Over { winner } = round.stage {
        // Above the score in the same band: a banner covers the court, and the
        // fields are public so that saying so costs one line.
        let over = Depth {
            layer: layers::UI,
            z: 1.0,
        };
        let lines = banner_lines(winner, round.points);
        // One `ctx.text` per line, each centred by its own width, and the two
        // are different sizes. `width_of` measures the widest line only, so one
        // call for the block would hang the short line off to the left — on
        // screen, at the right size, visibly crooked, and indistinguishable
        // from a layout that meant it by anything but a picture.
        //
        // Above the centre spot rather than across it: the ball is parked there
        // between matches, and the UI band draws the banner straight through it.
        for (line, style, top) in [
            (
                &lines[0],
                TextStyle {
                    size: 1.5,
                    color: Color::WHITE,
                    depth: over,
                },
                -4.4,
            ),
            (
                &lines[1],
                TextStyle {
                    size: 0.7,
                    color: Color::rgba(0.75, 0.85, 0.95, 0.9),
                    depth: over,
                },
                -2.5,
            ),
        ] {
            ctx.text(Vec2::new(-style.width_of(line) * 0.5, top), line, style);
        }
        return;
    }

    let style = TextStyle {
        size: 0.55,
        color: Color::rgba(0.75, 0.85, 0.95, 0.9),
        depth: Depth::layer(layers::UI),
    };
    let hint = hint_text(tally);
    // Below the court's floor line and above the camera's edge. `at` is the
    // *top* of the glyph cell and a line is exactly `size` tall, so this band
    // runs to `COURT.y + 0.83` against a camera reaching 10.0 — the margin was
    // 0.03 when the size was 0.62, which is not a margin.
    ctx.text(
        Vec2::new(-style.width_of(&hint) * 0.5, COURT.y + 0.28),
        &hint,
        style,
    );
}
