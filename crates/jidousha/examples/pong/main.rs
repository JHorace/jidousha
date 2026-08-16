//! Pong. Two paddles, a ball, a score, and about thirty seconds of fun.
//!
//! W and S move the left paddle (arrow keys work too). The right paddle is an
//! AI that does not commit until the ball is past halfway, which is what makes
//! it beatable: aim for the far corner. First to five; Space starts the next
//! match. Where on the paddle you hit the ball decides the angle it leaves at,
//! and every touch makes it faster.
//!
//! No art files: a paddle is a rectangle, the ball is a square, the score is
//! text. Everything is drawn by the engine, so this runs with nothing on disk.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//!
//! The check lives in `verify.rs` beside this file: the same systems and the
//! same config, driven by a script instead of a person, asserting on what the
//! world did and on what was drawn, with no window anywhere.

use jidousha::math::sin_cos;
use jidousha::prelude::*;

mod verify;

/// How many world units the window spans vertically.
///
/// A little taller than the field, so the border has room to breathe.
const VIEW_HEIGHT: f32 = 20.0;

/// Half the playfield, in world units: 34 wide by 19 tall.
///
/// Fixed rather than read off the camera, because the walls the ball bounces
/// off are part of the *game* — a field that grew with the window would make
/// the same rally play differently on a different monitor.
const FIELD: Vec2 = Vec2::new(17.0, 9.5);

/// How big a paddle is, in world units.
const PADDLE_SIZE: Vec2 = Vec2::new(0.7, 3.0);

/// How far from the centre a paddle stands.
const PADDLE_X: f32 = 15.0;

/// How far from the centre a paddle's centre may travel.
const PADDLE_LIMIT: f32 = FIELD.y - PADDLE_SIZE.y * 0.5;

/// How fast the player's paddle moves, in world units per second.
const PLAYER_SPEED: f32 = 19.0;

/// How fast the opponent's paddle moves.
///
/// Slower than the player's, and slower than a steep ball, so the AI misses
/// the hard ones.
const OPPONENT_SPEED: f32 = 12.0;

/// How far across the field the ball must be before the opponent commits.
///
/// The difficulty dial, and the reason the game is winnable. An opponent that
/// starts moving the instant the ball leaves the player's paddle has the whole
/// crossing to get anywhere on the field, and is unbeatable: two attentive
/// players rally until the heat death of the universe. Waiting for the halfway
/// line halves its time, which is what turns "aim for the far corner" from a
/// gesture into a tactic.
const OPPONENT_COMMITS_AT: f32 = 0.0;

/// How close the opponent needs to be before it stops correcting.
///
/// Without it the paddle judders around the ball's line by a fraction of a
/// unit every tick, which reads as a machine rather than as an opponent.
const OPPONENT_DEADZONE: f32 = 0.25;

/// Half the ball, in world units.
const BALL_HALF: f32 = 0.4;

/// How fast a serve leaves the centre spot, in world units per second.
const SERVE_SPEED: f32 = 19.0;

/// How much faster the ball gets with every paddle it touches.
const RALLY_SPEEDUP: f32 = 1.4;

/// The fastest the ball may ever go.
///
/// Capped below the speed at which the ball would cross a paddle's 0.7 units
/// of thickness inside one tick: at 60 Hz, 34 units per second is 0.57 of a
/// unit per tick. The paddle test below is a swept one and would catch it
/// anyway, so this is belt and braces — and mostly it is a playability
/// number, because a ball that outruns both paddles is not a rally.
const MAX_SPEED: f32 = 34.0;

/// The steepest angle a paddle can put on the ball, in radians from the
/// horizontal. Hitting with the very tip of the paddle gives this.
const MAX_BOUNCE: Radians = Radians(0.95);

/// How long the pause between a point and the next serve lasts, in ticks.
const SERVE_DELAY: u32 = 45;

/// Points needed to win a match.
const TARGET: u32 = 5;

/// Draw bands, named once so no `layer: 2` ever appears inline.
mod layers {
    /// The field and its markings.
    pub const FIELD: i16 = -1;
    /// Paddles and ball.
    pub const PLAY: i16 = 0;
    /// Score and prompts, over everything.
    pub const UI: i16 = 1;
}

/// Which end of the field something belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    /// The player's end, at negative X.
    Left,
    /// The opponent's end, at positive X.
    Right,
}

/// A paddle: which end it defends and how fast it may travel.
#[derive(Clone, Copy)]
struct Paddle {
    side: Side,
    /// World units per second.
    speed: f32,
}
impl Component for Paddle {}

/// The ball, and where it is going, in world units per second.
#[derive(Clone, Copy)]
struct Ball {
    velocity: Vec2,
}
impl Component for Ball {}

/// The score, and nothing else — the number on the wall.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Score {
    left: u32,
    right: u32,
}
impl Resource for Score {}

/// Where the match is up to: serving, rallying, or over.
#[derive(Clone, Copy, Debug)]
struct Bout {
    /// Ticks until the next serve. Zero means the ball is live.
    serve_in: u32,
    /// Which way the next serve goes: `-1.0` left, `+1.0` right.
    serve_toward: f32,
    /// How many paddles this rally has touched.
    rally: u32,
    /// Set once someone reaches `TARGET`.
    winner: Option<Side>,
}
impl Resource for Bout {}

/// Things that happened, counted.
///
/// The game shows the rally length off it; the verify run asserts on the rest.
/// Counting in the simulation rather than inferring it afterwards is what lets
/// a check say "the ball bounced off a wall eleven times" rather than "the
/// ball is still on the field, so presumably it did".
#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    wall_bounces: u32,
    paddle_hits: u32,
    points: u32,
}
impl Resource for Tally {}

/// The game's configuration, shared by the window and the verify run so that
/// what is checked is what a person sees.
fn config() -> GameConfig {
    GameConfig {
        title: "jidousha — pong",
        ..GameConfig::default()
    }
}

/// Every system this game has, in one place.
///
/// Named rather than written inline so the verify run drives the *same* game
/// the window does.
fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, drive_the_player);
    app.add_system(Update, drive_the_opponent);
    app.add_system(Update, move_the_ball);
    app.add_system(Update, start_the_next_match);
    app.add_system(Draw, draw_the_field);
    app.add_system(Draw, draw_the_players);
    app.add_system(Draw, draw_the_score);
}

fn main() -> Result<(), RunError> {
    if std::env::args().any(|argument| argument == "--verify") {
        verify::run();
        return Ok(());
    }
    println!("pong — W/S or up/down move the left paddle. first to {TARGET}.");
    println!("close the window to quit");
    run(config(), register)
}

fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        clear_color: Color::rgb(0.04, 0.05, 0.08),
        height: VIEW_HEIGHT,
        ..Camera::default()
    });
    world.insert_resource(Score::default());
    world.insert_resource(Tally::default());
    world.insert_resource(Bout {
        serve_in: SERVE_DELAY,
        serve_toward: 1.0,
        rally: 0,
        winner: None,
    });

    for (side, x, speed) in [
        (Side::Left, -PADDLE_X, PLAYER_SPEED),
        (Side::Right, PADDLE_X, OPPONENT_SPEED),
    ] {
        let paddle = world.spawn();
        world.insert(paddle, Transform::at(Vec2::new(x, 0.0)));
        world.insert(paddle, Paddle { side, speed });
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

/// W/S or the arrow keys, clamped to the field.
///
/// The one system that reads input, and therefore the one a script drives.
fn drive_the_player(world: &mut World) {
    let step = {
        // The first tick of a run can happen before any input is set, and a
        // game that assumed otherwise would panic on startup.
        let Some(input) = world.find_resource::<Input>() else {
            return;
        };
        let down = input.held(Key::S) || input.held(Key::ArrowDown);
        let up = input.held(Key::W) || input.held(Key::ArrowUp);
        f32::from(down) - f32::from(up)
    };
    let dt = world.resource::<Time>().fixed_dt.as_f32();

    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.side != Side::Left {
            continue;
        }
        transform.pos.y =
            (transform.pos.y + step * paddle.speed * dt).clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
    }
}

/// The opponent: chase the ball while it is coming, drift home while it is not.
///
/// Two passes, because reading the ball while writing the paddles is the one
/// thing a mutable query will not let you do: collect first, then move.
fn drive_the_opponent(world: &mut World) {
    let Some((ball_pos, ball_velocity)) = world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| (transform.pos, ball.velocity))
        .next()
    else {
        return;
    };
    let live = world.resource::<Bout>().serve_in == 0;
    let dt = world.resource::<Time>().fixed_dt.as_f32();

    // Only track a ball that is on its way over *and* already past halfway.
    // Between rallies, and while the player has it, the paddle goes back to
    // the middle — which is both what a person does and what stops the AI
    // from being perfect.
    let goal = if live && ball_velocity.x > 0.0 && ball_pos.x > OPPONENT_COMMITS_AT {
        ball_pos.y
    } else {
        0.0
    };

    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.side != Side::Right {
            continue;
        }
        let error = goal - transform.pos.y;
        if error.abs() < OPPONENT_DEADZONE {
            continue;
        }
        let reach = paddle.speed * dt;
        transform.pos.y =
            (transform.pos.y + error.clamp(-reach, reach)).clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
    }
}

/// The whole of Pong: serve, fly, bounce, score.
///
/// Read pass then write pass, like every system here that needs to see one
/// entity while changing another.
fn move_the_ball(world: &mut World) {
    if world.resource::<Bout>().winner.is_some() {
        return;
    }
    if world.resource::<Bout>().serve_in > 0 {
        let bout = world.resource_mut::<Bout>();
        bout.serve_in -= 1;
        if bout.serve_in == 0 {
            serve(world);
        }
        return;
    }

    let dt = world.resource::<Time>().fixed_dt.as_f32();
    let paddles: Vec<(Side, Vec2)> = world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (paddle.side, transform.pos))
        .collect();
    let Some((entity, before, mut velocity)) = world
        .query::<(&Transform, &Ball)>()
        .map(|(entity, transform, ball)| (entity, transform.pos, ball.velocity))
        .next()
    else {
        return;
    };

    let mut after = before + velocity * dt;
    let mut hits = 0;
    let mut bounces = 0;

    // Paddles first: they stand well clear of the walls, so the two tests
    // cannot both want the same tick.
    for (side, paddle) in paddles {
        // Which way this paddle sends the ball.
        let facing = match side {
            Side::Left => 1.0,
            Side::Right => -1.0,
        };
        // The plane the ball's *centre* is on when its edge touches the
        // paddle's edge — the paddle's own X, moved a half-paddle and a
        // half-ball toward the middle of the field.
        let face = paddle.x + facing * (PADDLE_SIZE.x * 0.5 + BALL_HALF);
        let closing = velocity.x * facing < 0.0;
        let crossed = (before.x - face) * facing >= 0.0 && (after.x - face) * facing <= 0.0;
        if !closing || !crossed {
            continue;
        }

        // Where the ball was when it reached the plane — not where it ended
        // up, which is already past the paddle. A ball moving steeply can
        // clear the paddle's corner between two ticks otherwise.
        let travel = before.x - after.x;
        let along = if travel.abs() > f32::EPSILON {
            (before.x - face) / travel
        } else {
            0.0
        };
        let meeting_y = before.y + (after.y - before.y) * along;
        let overlap = PADDLE_SIZE.y * 0.5 + BALL_HALF;
        if (meeting_y - paddle.y).abs() > overlap {
            continue;
        }

        // Where on the paddle it landed decides the angle: the middle sends
        // it flat, the tip sends it away steeply. This is the one rule that
        // turns Pong from a demo into a game.
        let offset = ((meeting_y - paddle.y) / overlap).clamp(-1.0, 1.0);
        let speed = (velocity.length() + RALLY_SPEEDUP).min(MAX_SPEED);
        let (sine, cosine) = sin_cos(Radians(offset * MAX_BOUNCE.0));
        velocity = Vec2::new(facing * speed * cosine, speed * sine);
        after = Vec2::new(face + (face - after.x), meeting_y);
        hits += 1;
    }

    // Then the walls. Reflecting rather than clamping keeps the ball's
    // distance travelled right, so a steep ball does not lose ground on the
    // tick it bounces.
    let wall = FIELD.y - BALL_HALF;
    if after.y > wall {
        after.y = 2.0 * wall - after.y;
        velocity.y = -velocity.y;
        bounces += 1;
    } else if after.y < -wall {
        after.y = -2.0 * wall - after.y;
        velocity.y = -velocity.y;
        bounces += 1;
    }

    // Past a paddle and off the end of the field: a point.
    let scored = if after.x < -FIELD.x {
        Some(Side::Right)
    } else if after.x > FIELD.x {
        Some(Side::Left)
    } else {
        None
    };

    world.component_mut::<Transform>(entity).pos = after;
    world.component_mut::<Ball>(entity).velocity = velocity;
    world.resource_mut::<Tally>().wall_bounces += bounces;
    world.resource_mut::<Tally>().paddle_hits += hits;
    world.resource_mut::<Bout>().rally += hits;

    let Some(scorer) = scored else {
        return;
    };
    let score = world.resource_mut::<Score>();
    let reached = match scorer {
        Side::Left => {
            score.left += 1;
            score.left
        }
        Side::Right => {
            score.right += 1;
            score.right
        }
    };
    world.resource_mut::<Tally>().points += 1;

    let bout = world.resource_mut::<Bout>();
    bout.rally = 0;
    bout.serve_in = SERVE_DELAY;
    // Serve toward whoever just conceded, which is the courtesy every version
    // of this game has had.
    bout.serve_toward = match scorer {
        Side::Left => 1.0,
        Side::Right => -1.0,
    };
    if reached >= TARGET {
        bout.winner = Some(scorer);
    }

    // Park the ball on the centre spot so the pause looks like a pause.
    world.component_mut::<Transform>(entity).pos = Vec2::ZERO;
    world.component_mut::<Ball>(entity).velocity = Vec2::ZERO;
}

/// Put the ball on the centre spot and push it, at an angle the seeded
/// generator picks — so the same run serves the same ball every time.
fn serve(world: &mut World) {
    let toward = world.resource::<Bout>().serve_toward;
    let spread = world.resource_mut::<Rng>().next_f32();
    let Some(entity) = world
        .query::<(&Transform, &Ball)>()
        .map(|(entity, _, _)| entity)
        .next()
    else {
        return;
    };
    // Between roughly 25 degrees up and 25 degrees down: never flat, never so
    // steep that the first bounce comes before anyone can move.
    let heading = Vec2::new(toward, (spread - 0.5) * 0.9).normalize_or_zero();
    world.component_mut::<Transform>(entity).pos = Vec2::ZERO;
    world.component_mut::<Ball>(entity).velocity = heading * SERVE_SPEED;
}

/// Space, once someone has won, wipes the score and serves again.
fn start_the_next_match(world: &mut World) {
    if world.resource::<Bout>().winner.is_none() {
        return;
    }
    let restart = match world.find_resource::<Input>() {
        None => return,
        Some(input) => input.just_pressed(Key::Space),
    };
    if !restart {
        return;
    }
    *world.resource_mut::<Score>() = Score::default();
    let bout = world.resource_mut::<Bout>();
    bout.winner = None;
    bout.rally = 0;
    bout.serve_in = SERVE_DELAY;
}

/// The border and the halfway line: everything that is not a moving part.
fn draw_the_field(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::FIELD);
    let chalk = Color::rgba(1.0, 1.0, 1.0, 0.16);
    let corners = [
        Vec2::new(-FIELD.x, -FIELD.y),
        Vec2::new(FIELD.x, -FIELD.y),
        Vec2::new(FIELD.x, FIELD.y),
        Vec2::new(-FIELD.x, FIELD.y),
    ];
    for index in 0..corners.len() {
        ctx.line(
            corners[index],
            corners[(index + 1) % corners.len()],
            0.15,
            chalk,
            depth,
        );
    }

    // The halfway line, dashed, because a solid one reads as a wall.
    let dash = 0.7;
    let mut y = -FIELD.y + dash;
    while y < FIELD.y - dash {
        ctx.line(
            Vec2::new(0.0, y),
            Vec2::new(0.0, y + dash),
            0.12,
            chalk,
            depth,
        );
        y += dash * 2.0;
    }
}

/// The paddles and the ball, from where the world says they are.
fn draw_the_players(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PLAY);
    for (_, transform, paddle) in ctx.world.query::<(&Transform, &Paddle)>() {
        let color = match paddle.side {
            Side::Left => Color::rgb(0.45, 0.95, 1.0),
            Side::Right => Color::rgb(1.0, 0.55, 0.45),
        };
        ctx.rect(
            Rect::from_center_size(transform.pos, PADDLE_SIZE),
            color,
            depth,
        );
    }
    // A square ball, drawn at exactly the size the collision uses — so what
    // the player sees hitting the paddle is what the simulation tested.
    for (_, transform, _) in ctx.world.query::<(&Transform, &Ball)>() {
        ctx.rect(
            Rect::from_center_size(transform.pos, Vec2::splat(BALL_HALF * 2.0)),
            Color::WHITE,
            depth,
        );
    }
}

/// How far out from the halfway line each score sits.
const SCORE_GAP: f32 = 2.0;

/// How far down from the top of the field the score sits.
const SCORE_TOP: f32 = -FIELD.y + 1.0;

/// How the score is drawn.
///
/// A function rather than a literal at the draw site because the verify run
/// asks it where the digits land — a check carrying its own copy of the size
/// would keep passing after the score moved.
fn score_style() -> TextStyle {
    TextStyle {
        size: 2.4,
        color: Color::rgba(1.0, 1.0, 1.0, 0.75),
        depth: Depth::layer(layers::UI),
    }
}

/// The score, the rally counter, and whatever the game wants to say.
fn draw_the_score(ctx: &mut DrawCtx) {
    let score = *ctx.world.resource::<Score>();
    let bout = *ctx.world.resource::<Bout>();

    // Big, and out from the halfway line on each side, the way the cabinet
    // did it. Measured rather than nudged: `width_of` is exact.
    let numbers = score_style();
    let left = format!("{}", score.left);
    ctx.text(
        Vec2::new(-SCORE_GAP - numbers.width_of(&left), SCORE_TOP),
        &left,
        numbers,
    );
    ctx.text(
        Vec2::new(SCORE_GAP, SCORE_TOP),
        &format!("{}", score.right),
        numbers,
    );

    let note = TextStyle {
        size: 0.75,
        color: Color::rgba(0.65, 0.85, 1.0, 0.8),
        depth: Depth::layer(layers::UI),
    };
    let footer = match bout.winner {
        Some(Side::Left) => "you win — space to play again".to_owned(),
        Some(Side::Right) => "you lose — space to play again".to_owned(),
        None if bout.serve_in > 0 => "serving...".to_owned(),
        None => format!("rally {}", bout.rally),
    };
    ctx.text(
        Vec2::new(-note.width_of(&footer) * 0.5, FIELD.y - 1.6),
        &footer,
        note,
    );

    // The winner gets the middle of the screen, over everything.
    if let Some(winner) = bout.winner {
        let banner = TextStyle {
            size: 1.4,
            color: match winner {
                Side::Left => Color::rgb(0.45, 0.95, 1.0),
                Side::Right => Color::rgb(1.0, 0.55, 0.45),
            },
            depth: Depth::layer(layers::UI),
        };
        let text = match winner {
            Side::Left => "PLAYER WINS",
            Side::Right => "OPPONENT WINS",
        };
        ctx.text(
            Vec2::new(-banner.width_of(text) * 0.5, -banner.size * 0.5),
            text,
            banner,
        );
    }
}
