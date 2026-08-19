//! Pong. Two paddles, a ball, a score, and a match that ends.
//!
//! W and S move the left paddle. The right one plays itself. First to five
//! wins; Space plays again. The ball leaves the paddle at an angle set by where
//! along it you struck, so the whole game is standing somewhere useful and then
//! choosing which part of the paddle to meet the ball with.
//!
//! Run it:   `cargo run -p jidousha --example pong`
//! Check it: `cargo run -p jidousha --example pong -- --verify`
//!
//! The check is `verify.rs` beside this file: the same systems and the same
//! config, driven by a controller that plays to win instead of by a person,
//! asserting on what the world did and on what was drawn, with no window
//! anywhere. `rules.rs` holds the arithmetic both of them use, so the check
//! cannot quietly grade the game against a second copy of its own rules.

use std::process::ExitCode;

use jidousha::prelude::*;

mod capture;
mod checks;
mod rules;
mod verify;

use rules::{
    BALL_RADIUS, FIELD_HALF, MAX_BALL_SPEED, OPPONENT_SPEED, PADDLE_SIZE, PLAYER_SPEED,
    SERVE_SPEED, SPEEDUP, Side, bounce_velocity, contact_offset, opponent_step, paddle_step,
    reflect_walls, sweep_contact,
};

/// How tall the camera is, in world units. The field is 18 tall, so this
/// leaves a unit of margin above and below the walls.
pub(crate) const VIEW_HEIGHT: f32 = 20.0;

/// How big the window opens, and the shape everything is framed at.
pub(crate) const VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// How long the ball sits still between a point and the next serve, in ticks.
///
/// Ticks rather than seconds: the tick is the canonical timeline, and 45 of
/// them is three quarters of a second at the default timestep.
pub(crate) const SERVE_PAUSE: u32 = 45;

/// How many points win the match.
pub(crate) const WINNING_SCORE: u32 = 5;

/// Draw bands. Named once here rather than as numbers at forty call sites.
///
/// `SCORE` sits below `PLAY` deliberately: the score is painted on the court,
/// the way it is in the arcade, so the ball passes in *front* of it. Move the
/// constant above `PLAY` and the score paints over the ball, in the right place
/// and at the right size, with every geometric assertion still passing —
/// which is why `verify.rs` checks the order rather than only the geometry.
pub(crate) mod layers {
    /// The court: walls, the halfway line.
    pub const COURT: i16 = -2;
    /// The score, painted on the court behind the play.
    pub const SCORE: i16 = -1;
    /// Paddles and ball.
    pub const PLAY: i16 = 0;
    /// Banners and the hint line, over everything.
    pub const UI: i16 = 2;
}

/// The colours, in one place so the picture can be changed without hunting.
mod palette {
    use jidousha::prelude::Color;

    pub const COURT: Color = Color::rgb(0.05, 0.07, 0.10);
    pub const WALL: Color = Color::rgba(1.0, 1.0, 1.0, 0.35);
    pub const HALFWAY: Color = Color::rgba(1.0, 1.0, 1.0, 0.10);
    pub const SCORE: Color = Color::rgba(1.0, 1.0, 1.0, 0.13);
    pub const PLAYER: Color = Color::rgb(0.40, 0.95, 0.75);
    pub const OPPONENT: Color = Color::rgb(0.95, 0.55, 0.40);
    pub const BALL: Color = Color::rgb(1.0, 1.0, 1.0);
    pub const HINT: Color = Color::rgba(0.75, 0.85, 1.0, 0.75);
    pub const DIM: Color = Color::rgba(0.0, 0.0, 0.0, 0.55);
}

/// A paddle: which end it defends, how fast it moves, and which way it has been
/// told to go this tick.
#[derive(Clone, Copy)]
pub(crate) struct Paddle {
    pub(crate) side: Side,
    pub(crate) speed: f32,
    /// `-1.0` up, `+1.0` down, `0.0` still. Written by whatever steers this
    /// paddle and read by `move_the_paddles`, so a hand and an opponent are the
    /// same thing to everything downstream.
    pub(crate) intent: f32,
}
impl Component for Paddle {}

/// The ball: where it is going, and how fast it is going there.
#[derive(Clone, Copy)]
pub(crate) struct Ball {
    pub(crate) vel: Vec2,
    /// Kept beside the velocity because a rally ramps it, and reading it back
    /// out of the velocity's length would drift a little every bounce.
    pub(crate) speed: f32,
}
impl Component for Ball {}

/// What the match is doing right now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Stage {
    /// The ball is on the spot, waiting. `toward` is which way it will go.
    Serving { ticks_left: u32, toward: f32 },
    /// The ball is live.
    Rally,
    /// Somebody won. Space plays again.
    Over { winner: Side },
}

/// The score, the stage, and the few numbers a failing check wants to quote.
#[derive(Clone, Debug)]
pub(crate) struct Scoreboard {
    pub(crate) left: u32,
    pub(crate) right: u32,
    pub(crate) stage: Stage,
    /// Paddle touches in the rally now being played.
    pub(crate) touches: u32,
    /// The most touches any one rally has had.
    pub(crate) longest_rally: u32,
    /// The fastest the ball has been, in world units per second.
    pub(crate) top_speed: f32,
    /// How many times a paddle has sent the ball back, all match.
    pub(crate) returns: u32,
}
impl Resource for Scoreboard {}

impl Scoreboard {
    fn new() -> Self {
        Scoreboard {
            left: 0,
            right: 0,
            stage: Stage::Serving {
                ticks_left: SERVE_PAUSE,
                toward: 1.0,
            },
            touches: 0,
            longest_rally: 0,
            top_speed: 0.0,
            returns: 0,
        }
    }
}

/// The game's configuration, shared by the window and the check, so what is
/// verified is what a person plays.
pub(crate) fn config() -> GameConfig {
    GameConfig {
        title: "jidousha - pong",
        seed: 7,
        window_size: VIEWPORT,
        ..GameConfig::default()
    }
}

/// Every system, in one place and in one order, for the same reason.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, set_the_court);
    app.add_system(Update, steer_the_player);
    app.add_system(Update, steer_the_opponent);
    app.add_system(Update, move_the_paddles);
    app.add_system(Update, move_the_ball);
    app.add_system(Update, play_again);
    app.add_system(Draw, draw_the_court);
    app.add_system(Draw, draw_the_score);
    app.add_system(Draw, draw_the_play);
    app.add_system(Draw, draw_the_banner);
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("W and S move the left paddle. first to {WINNING_SCORE}; space plays again.");
    println!("close the window to quit.");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

// --- the world ------------------------------------------------------------

fn set_the_court(world: &mut World) {
    world.insert_resource(Camera {
        center: Vec2::ZERO,
        height: VIEW_HEIGHT,
        clear_color: palette::COURT,
        viewport: VIEWPORT,
    });
    world.insert_resource(Scoreboard::new());

    for (side, speed) in [(Side::Left, PLAYER_SPEED), (Side::Right, OPPONENT_SPEED)] {
        let paddle = world.spawn();
        world.insert(paddle, Transform::at(Vec2::new(side.paddle_x(), 0.0)));
        world.insert(
            paddle,
            Paddle {
                side,
                speed,
                intent: 0.0,
            },
        );
    }

    let ball = world.spawn();
    world.insert(ball, Transform::at(Vec2::ZERO));
    world.insert(
        ball,
        Ball {
            vel: Vec2::ZERO,
            speed: SERVE_SPEED,
        },
    );
}

/// W and S, into the left paddle's `intent`.
fn steer_the_player(world: &mut World) {
    // Not before the first tick, and never at all under `headless` unless
    // something puts one in — so this is the resource to ask about rather than
    // to demand.
    let intent = match world.find_resource::<Input>() {
        None => return,
        Some(input) => f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    };
    for (_, paddle) in world.query_mut::<&mut Paddle>() {
        if paddle.side == Side::Left {
            paddle.intent = intent;
        }
    }
}

/// The opponent: chase the ball.
///
/// One `Update` system and one line of decision, and it moves the paddle
/// itself rather than writing an intent — the opponent is not a hand on a
/// keyboard, so routing it through `Paddle::intent` would only be a longer way
/// to say `opponent_step`, and would put a second copy of its speed and its
/// dead zone in a second place. The player's paddle still goes through
/// `intent`, because that is what a hand produces.
fn steer_the_opponent(world: &mut World) {
    let step = world.resource::<Time>().fixed_dt.as_f32();
    // Read pass: the ball is one entity and the paddle another, so the write
    // below cannot borrow the world while this is still reading it.
    let Some(ball_y) = world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, _)| transform.pos.y)
        .next()
    else {
        return;
    };
    // Write pass.
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.side == Side::Right {
            transform.pos.y = opponent_step(transform.pos.y, ball_y, step);
        }
    }
}

/// One step of whatever the player's paddle was told to do.
fn move_the_paddles(world: &mut World) {
    let step = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        if paddle.side == Side::Left {
            transform.pos.y = paddle_step(transform.pos.y, paddle.intent, paddle.speed, step);
        }
    }
}

/// The whole of the ball: the serve clock, one tick of travel, the paddles, the
/// walls, and the goal lines.
fn move_the_ball(world: &mut World) {
    match world.resource::<Scoreboard>().stage {
        Stage::Over { .. } => return,
        Stage::Serving { ticks_left, toward } => {
            let next = ticks_left.saturating_sub(1);
            if next > 0 {
                world.resource_mut::<Scoreboard>().stage = Stage::Serving {
                    ticks_left: next,
                    toward,
                };
                return;
            }
            // The serve's angle is the only random thing in the game, and it
            // comes from the seeded generator, so the same seed plays the same
            // match every time.
            let spread = world.resource_mut::<Rng>().next_f32() * 2.0 - 1.0;
            let vel = bounce_velocity(spread * 0.6, SERVE_SPEED, toward);
            for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
                transform.pos = Vec2::ZERO;
                ball.vel = vel;
                ball.speed = SERVE_SPEED;
            }
            let board = world.resource_mut::<Scoreboard>();
            board.stage = Stage::Rally;
            board.touches = 0;
            return;
        }
        Stage::Rally => {}
    }

    let step = world.resource::<Time>().fixed_dt.as_f32();
    // Read pass: where the paddles are, after they have moved this tick.
    let paddles: Vec<(Side, f32)> = world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (paddle.side, transform.pos.y))
        .collect();
    let Some((entity, from, mut vel, mut speed)) = world
        .query::<(&Transform, &Ball)>()
        .map(|(entity, transform, ball)| (entity, transform.pos, ball.vel, ball.speed))
        .next()
    else {
        return;
    };

    let to = from + vel * step;
    let mut pos = to;
    let mut struck = None;

    // A swept test against each paddle's face, earliest first. Positions are
    // only ever compared at tick boundaries, so a ball moving further in one
    // tick than a paddle is thick would step straight through a test that only
    // asked where it ended up.
    let mut earliest: Option<(f32, Side, f32)> = None;
    for (side, paddle_y) in &paddles {
        let hit = sweep_contact(
            from,
            to,
            side.contact_plane(),
            -side.hits_toward(),
            *paddle_y,
            PADDLE_SIZE.y * 0.5 + BALL_RADIUS,
        );
        if let Some(t) = hit
            && earliest.is_none_or(|(best, _, _)| t < best)
        {
            earliest = Some((t, *side, *paddle_y));
        }
    }
    if let Some((t, side, paddle_y)) = earliest {
        let contact = from + (to - from) * t;
        let offset = contact_offset(contact.y, paddle_y);
        speed = (speed + SPEEDUP).min(MAX_BALL_SPEED);
        vel = bounce_velocity(offset, speed, side.hits_toward());
        // The rest of the tick, spent going the new way.
        pos = contact + vel * step * (1.0 - t);
        struck = Some(side);
    }

    // The walls. The same function the controller in `verify.rs` rolls its
    // candidate shots through.
    let bounced = reflect_walls(pos, vel);
    pos = bounced.0;
    vel = bounced.1;

    let scored = if pos.x < -FIELD_HALF.x {
        Some(Side::Right)
    } else if pos.x > FIELD_HALF.x {
        Some(Side::Left)
    } else {
        None
    };

    // Write pass.
    if let Some(ball) = world.find_component_mut::<Ball>(entity) {
        ball.vel = vel;
        ball.speed = speed;
    }
    if let Some(transform) = world.find_component_mut::<Transform>(entity) {
        transform.pos = pos;
    }

    let board = world.resource_mut::<Scoreboard>();
    if struck.is_some() {
        board.touches += 1;
        board.returns += 1;
        board.longest_rally = board.longest_rally.max(board.touches);
        board.top_speed = board.top_speed.max(speed);
    }
    if let Some(winner_of_the_point) = scored {
        match winner_of_the_point {
            Side::Left => board.left += 1,
            Side::Right => board.right += 1,
        }
        let reached = match winner_of_the_point {
            Side::Left => board.left,
            Side::Right => board.right,
        };
        board.stage = if reached >= WINNING_SCORE {
            Stage::Over {
                winner: winner_of_the_point,
            }
        } else {
            Stage::Serving {
                ticks_left: SERVE_PAUSE,
                // Towards whoever just conceded, which is the arcade's rule and
                // the one that keeps a run of points from feeling like a
                // punishment.
                toward: winner_of_the_point.other().goal_direction(),
            }
        };
        board.touches = 0;
        // Park the ball on the spot for the pause.
        if let Some(transform) = world.find_component_mut::<Transform>(entity) {
            transform.pos = Vec2::ZERO;
        }
        if let Some(ball) = world.find_component_mut::<Ball>(entity) {
            ball.vel = Vec2::ZERO;
        }
    }
}

/// Space, after somebody has won.
fn play_again(world: &mut World) {
    if !matches!(world.resource::<Scoreboard>().stage, Stage::Over { .. }) {
        return;
    }
    let restart = world
        .find_resource::<Input>()
        .is_some_and(|input| input.just_pressed(Key::Space));
    if !restart {
        return;
    }
    world.insert_resource(Scoreboard::new());
    for (_, transform, ball) in world.query_mut::<(&mut Transform, &mut Ball)>() {
        transform.pos = Vec2::ZERO;
        ball.vel = Vec2::ZERO;
        ball.speed = SERVE_SPEED;
    }
}

// --- the picture ----------------------------------------------------------

/// The walls and the halfway line.
fn draw_the_court(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::COURT);
    let thickness = 0.3;
    for wall in [-FIELD_HALF.y, FIELD_HALF.y] {
        ctx.rect(
            Rect::from_center_size(
                Vec2::new(0.0, wall),
                Vec2::new(FIELD_HALF.x * 2.0, thickness),
            ),
            palette::WALL,
            depth,
        );
    }
    // A dashed halfway line, which is what makes it read as a court rather
    // than as a divider.
    let dashes = 13;
    let pitch = FIELD_HALF.y * 2.0 / dashes as f32;
    for index in 0..dashes {
        let y = -FIELD_HALF.y + pitch * (index as f32 + 0.5);
        ctx.rect(
            Rect::from_center_size(Vec2::new(0.0, y), Vec2::new(0.16, pitch * 0.55)),
            palette::HALFWAY,
            depth,
        );
    }
}

/// The score's text height, in world units.
pub(crate) const SCORE_SIZE: f32 = 3.4;
/// Where the top of the score's glyphs sits, in world units.
pub(crate) const SCORE_TOP: f32 = -8.2;
/// How far either side of the halfway line each score is centred.
pub(crate) const SCORE_OFFSET: f32 = 5.0;

/// The two numbers, painted on the court behind the play.
fn draw_the_score(ctx: &mut DrawCtx) {
    let board = ctx.world.resource::<Scoreboard>();
    let style = TextStyle {
        size: SCORE_SIZE,
        color: palette::SCORE,
        depth: Depth::layer(layers::SCORE),
    };
    for (score, centre) in [(board.left, -SCORE_OFFSET), (board.right, SCORE_OFFSET)] {
        let text = score.to_string();
        ctx.text(
            Vec2::new(centre - style.width_of(&text) * 0.5, SCORE_TOP),
            &text,
            style,
        );
    }
}

/// The paddles and the ball.
fn draw_the_play(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PLAY);
    for (_, transform, paddle) in ctx.world.query::<(&Transform, &Paddle)>() {
        let colour = match paddle.side {
            Side::Left => palette::PLAYER,
            Side::Right => palette::OPPONENT,
        };
        ctx.rect(
            Rect::from_center_size(transform.pos, PADDLE_SIZE),
            colour,
            depth,
        );
    }
    // The ball is hidden between the point and the serve: it is on the spot,
    // and drawing it there suggests it is live.
    if matches!(ctx.world.resource::<Scoreboard>().stage, Stage::Rally) {
        for (_, transform, _) in ctx.world.query::<(&Transform, &Ball)>() {
            ctx.circle(transform.pos, BALL_RADIUS, palette::BALL, depth);
        }
    }
}

/// Where the hint line's glyphs start, in world units from the top of the text.
pub(crate) const HINT_TOP: f32 = 7.6;
/// The hint line's text height.
pub(crate) const HINT_SIZE: f32 = 0.75;
/// What the hint line says while a match is being played.
pub(crate) const HINT: &str = "W / S to move - first to 5";
/// The banner over a match the player won.
pub(crate) const WIN_HEADLINE: &str = "YOU WIN";
/// The banner over a match the player lost.
pub(crate) const LOSE_HEADLINE: &str = "OPPONENT WINS";
/// What both banners say underneath.
pub(crate) const PLAY_AGAIN: &str = "space to play again";
/// The countdown's words, before its number.
pub(crate) const SERVING: &str = "serving in";

/// Every string this game ever draws that is not a number.
///
/// Collected so the check can look at the *strings* rather than at the quads
/// they produced: the font draws anything outside space-through-`~` as a box at
/// exactly a letter's advance, so a stray dash or curly quote passes every
/// assertion about what was drawn.
pub(crate) const EVERY_LITERAL: &[&str] = &[HINT, WIN_HEADLINE, LOSE_HEADLINE, PLAY_AGAIN, SERVING];

/// The hint line, the countdown, and the two banners.
fn draw_the_banner(ctx: &mut DrawCtx) {
    let board = ctx.world.resource::<Scoreboard>();
    let hint = TextStyle {
        size: HINT_SIZE,
        color: palette::HINT,
        depth: Depth::layer(layers::UI),
    };
    centred_line(ctx, HINT, HINT_TOP, hint);

    match board.stage {
        Stage::Rally => {}
        Stage::Serving { ticks_left, .. } => {
            let count = ticks_left.div_ceil(20);
            centred_line(
                ctx,
                &format!("{SERVING} {count}"),
                -1.0,
                TextStyle {
                    size: 1.1,
                    color: palette::HINT,
                    depth: Depth::layer(layers::UI),
                },
            );
        }
        Stage::Over { winner } => {
            // A dimmer, so the banner reads against a court that is still
            // drawn underneath it. Over the whole camera rather than over the
            // field, or the margins outside the walls stay bright and the
            // screen reads as a vignette.
            ctx.rect(
                ctx.world.resource::<Camera>().visible_bounds(),
                palette::DIM,
                Depth::layer(layers::UI),
            );
            let headline = match winner {
                Side::Left => WIN_HEADLINE,
                Side::Right => LOSE_HEADLINE,
            };
            let big = TextStyle {
                size: 2.0,
                color: Color::WHITE,
                depth: Depth {
                    layer: layers::UI,
                    z: 1.0,
                },
            };
            let small = TextStyle {
                size: 0.9,
                color: palette::HINT,
                depth: Depth {
                    layer: layers::UI,
                    z: 1.0,
                },
            };
            // One call per line, each centred by its own width: `width_of`
            // measures only the widest line, so a single two-line block would
            // hang the shorter line off to the left.
            centred_line(ctx, headline, -2.4, big);
            centred_line(
                ctx,
                &format!("{} - {}", board.left, board.right),
                0.2,
                small,
            );
            centred_line(ctx, PLAY_AGAIN, 1.6, small);
        }
    }
}

/// One line of text, centred on the halfway line, with `top` the top of its
/// glyphs.
fn centred_line(ctx: &mut DrawCtx, text: &str, top: f32, style: TextStyle) {
    ctx.text(Vec2::new(-style.width_of(text) * 0.5, top), text, style);
}
