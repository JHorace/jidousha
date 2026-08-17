//! Everything a prototype needs, on one screen (renderer.md §11, R3).
//!
//! Sprites, rectangles, lines, circles and text — the whole drawing vocabulary
//! v1 has. After this, "can an agent make Pong?" is a question about the API
//! rather than about what the engine can draw: a paddle is a rectangle, a ball
//! is a circle, a score is text, and a hitbox outline is four lines.
//!
//! Everything here goes through one submission stream, so the debug outlines
//! interleave with the art by depth rather than being stapled on afterwards.
//! The hitbox sits on the DEBUG layer, above the sprite; move that constant
//! below PLAY and it goes behind instead. That choice belongs to the game
//! precisely because there is no separate debug pass to overrule it.
//!
//! Run it: `cargo run -p jidousha --example prototype_kit`
//! On the web: `tools/serve-web prototype_kit`
//! Check it:  `tools/verify prototype_kit`
//!
//! The last of those is I2's exit criterion and the engine's thesis in one
//! command: the same systems and the same config as the window, driven by a
//! script instead of a person, asserting on world state and on the draw
//! transcript, with no display anywhere. It lives in `verify.rs` beside this
//! file — the first example to be a directory rather than one file, because the
//! game and the check on the game are two things to read.

use jidousha::math::sin_cos;
use std::process::ExitCode;

use jidousha::prelude::*;

/// Where the art lives, relative to the workspace root (assets.md §2).
const ASSET_ROOT: &str = "assets";

/// The world is twenty units tall; everything below is in those units.
const VIEW_HEIGHT: f32 = 20.0;

/// How big a paddle is drawn, in world units.
///
/// Stated once because the verify run asserts against it: a check that carried
/// its own copy of the number would keep passing after the paddle changed size.
const PADDLE_SIZE: Vec2 = Vec2::new(0.5, 4.0);

/// Draw bands, so the ordering is stated once rather than guessed at each site.
///
/// This is the layering convention a real game would put in its own module —
/// naming the bands is what stops `z: 3.0` appearing in forty places.
mod layers {
    /// Behind everything: the field and its markings.
    pub const FIELD: i16 = -1;
    /// The things the game is about.
    pub const PLAY: i16 = 0;
    /// Hitboxes and other things only a developer looks at.
    pub const DEBUG: i16 = 1;
    /// Score and readouts, over everything.
    pub const UI: i16 = 2;
}

/// A thing that bounces between two X positions.
#[derive(Clone, Copy)]
struct Bounce {
    /// Half the distance travelled, in world units.
    reach: f32,
    /// Full cycles per second.
    rate: f32,
}
impl Component for Bounce {}

/// A thing that turns.
#[derive(Clone, Copy)]
struct Spin(f32);
impl Component for Spin {}

/// A paddle the player moves, and how far it may travel.
#[derive(Clone, Copy)]
struct Paddle {
    /// World units per tick.
    speed: f32,
    /// Half the travel allowed either side of the centre.
    limit: f32,
}
impl Component for Paddle {}

/// The game's configuration, shared by the window and the verify run so that
/// what is verified is what a person sees.
fn config() -> GameConfig {
    GameConfig {
        title: "jidousha — prototype kit",
        ..GameConfig::default()
    }
}

/// Every system this game has, in one place.
///
/// Named rather than written inline so `tools/verify` runs the *same* game the
/// window does. A verify run that registered a different set of systems would
/// be verifying a different program.
fn register(app: &mut App) {
    app.add_system(Startup, set_the_scene);
    app.add_system(Update, drive_the_paddle);
    app.add_system(Update, bounce);
    app.add_system(Update, turn);
    app.add_system(Draw, draw_sprites);
    app.add_system(Draw, draw_the_field);
    app.add_system(Draw, draw_the_hitboxes);
    app.add_system(Draw, draw_the_readout);
}

mod capture;
mod verify;

fn main() -> ExitCode {
    // `tools/verify` runs this same binary with `--verify`: same systems, same
    // config, no window, scripted input, and assertions instead of a person.
    if std::env::args().any(|argument| argument == "--verify") {
        verify::run();
        return ExitCode::SUCCESS;
    }
    println!("W and S move the left paddle. close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        clear_color: Color::rgb(0.07, 0.09, 0.13),
        height: VIEW_HEIGHT,
        ..Camera::default()
    });
    // Only if nothing has installed one already. A verify run puts a scripted
    // store in before Startup so that *when* the art arrives is part of the
    // script rather than a question about the disk (assets.md §7).
    if world.find_resource::<Assets>().is_none() {
        world.insert_resource(art());
    }

    let hero = world
        .resource_mut::<Assets>()
        .load_texture("sprites/hero.png");

    let ball = world.spawn();
    world.insert(ball, Transform::at(Vec2::new(0.0, -4.0)));
    world.insert(
        ball,
        Bounce {
            reach: 9.0,
            rate: 0.25,
        },
    );
    world.insert(ball, Spin(1.2));
    world.insert(
        ball,
        Sprite {
            size: Vec2::new(3.0, 3.0),
            layer: layers::PLAY,
            ..Sprite::new(hero)
        },
    );

    // The player's paddle, on the left.
    let paddle = world.spawn();
    world.insert(paddle, Transform::at(Vec2::new(-14.0, 0.0)));
    world.insert(
        paddle,
        Paddle {
            speed: 0.25,
            limit: 7.0,
        },
    );
}

/// Move the paddle with W and S, clamped to the field.
///
/// The one system that reads input, and therefore the one a script can drive.
fn drive_the_paddle(world: &mut World) {
    let step = match world.find_resource::<Input>() {
        // The first tick of a run can happen before any input is set, and a
        // game that assumed otherwise would panic on startup.
        None => return,
        Some(input) => f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    };
    for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>() {
        transform.pos.y =
            (transform.pos.y + step * paddle.speed).clamp(-paddle.limit, paddle.limit);
    }
}

/// Slide the bouncers along X, on simulated time.
fn bounce(world: &mut World) {
    let elapsed = world.resource::<Time>().elapsed.as_f32();
    for (_, transform, bounce) in world.query_mut::<(&mut Transform, &Bounce)>() {
        // The engine's own trigonometry, so the ball is in the same place at
        // tick 600 on every machine (ADR-0009, core.md §7).
        let (sine, _) = sin_cos(Radians(elapsed * bounce.rate * core::f32::consts::TAU));
        transform.pos.x = sine * bounce.reach;
    }
}

fn turn(world: &mut World) {
    let elapsed = world.resource::<Time>().elapsed.as_f32();
    for (_, transform, spin) in world.query_mut::<(&mut Transform, &Spin)>() {
        transform.rot = Radians(elapsed * spin.0);
    }
}

/// The playfield: a border, a halfway line, and a centre circle.
///
/// Every Pong-shaped game draws roughly this, and none of it is art.
fn draw_the_field(ctx: &mut DrawCtx) {
    let (top_left, bottom_right) = ctx.world.resource::<Camera>().visible_bounds();
    let inset = 0.6;
    let field = Rect {
        min: top_left + Vec2::splat(inset),
        max: bottom_right - Vec2::splat(inset),
    };
    let depth = Depth::layer(layers::FIELD);
    let line = Color::rgba(1.0, 1.0, 1.0, 0.18);

    // A border, as four lines rather than a filled rectangle — an outline is
    // what a border is, and there is no "stroke" mode to remember.
    for (from, to) in [
        (field.min, Vec2::new(field.max.x, field.min.y)),
        (Vec2::new(field.max.x, field.min.y), field.max),
        (field.max, Vec2::new(field.min.x, field.max.y)),
        (Vec2::new(field.min.x, field.max.y), field.min),
    ] {
        ctx.line(from, to, 0.15, line, depth);
    }

    ctx.line(
        Vec2::new(field.center().x, field.min.y),
        Vec2::new(field.center().x, field.max.y),
        0.1,
        line,
        depth,
    );
    // Alpha blends in *linear* light, because the surface is sRGB and that is
    // where blending is physically right. The practical consequence is that a
    // small alpha over a dark background reads brighter than the number
    // suggests — 0.06 white here looked like a solid grey disc, and this is the
    // value that actually reads as a field marking.
    ctx.circle(
        field.center(),
        3.0,
        Color::rgba(1.0, 1.0, 1.0, 0.015),
        depth,
    );

    // The right-hand paddle is scenery; the left one is an entity the player
    // moves, drawn below from its `Transform`.
    ctx.rect(
        Rect::from_center_size(Vec2::new(field.max.x - 1.2, 0.0), PADDLE_SIZE),
        Color::rgb(0.85, 0.85, 0.9),
        Depth::layer(layers::PLAY),
    );
    let paddles: Vec<Vec2> = ctx
        .world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, _)| transform.pos)
        .collect();
    for at in paddles {
        ctx.rect(
            Rect::from_center_size(at, PADDLE_SIZE),
            Color::rgb(0.4, 1.0, 0.7),
            Depth::layer(layers::PLAY),
        );
    }
}

/// A box around every sprite, the way a developer checks their geometry.
///
/// This is the sprite's `size` at its position — *not* the bounds of the
/// rotated quad, which are larger. Watch the spinning sprite and the difference
/// is the point: an axis-aligned box is what most prototype collision uses, and
/// seeing it disagree with the art is exactly the kind of thing debug drawing
/// exists to show.
///
/// On the DEBUG layer, which here is above the play layer — move the constant
/// below PLAY and the outlines go behind instead.
fn draw_the_hitboxes(ctx: &mut DrawCtx) {
    let boxes: Vec<Rect> = ctx
        .world
        .query::<(&Transform, &Sprite)>()
        .map(|(_, transform, sprite)| Rect::from_center_size(transform.pos, sprite.size))
        .collect();
    let depth = Depth::layer(layers::DEBUG);
    for bounds in boxes {
        let corners = [
            bounds.min,
            Vec2::new(bounds.max.x, bounds.min.y),
            bounds.max,
            Vec2::new(bounds.min.x, bounds.max.y),
        ];
        for index in 0..4 {
            ctx.line(
                corners[index],
                corners[(index + 1) % 4],
                0.08,
                Color::rgba(0.2, 1.0, 0.4, 0.9),
                depth,
            );
        }
        // And a dot on the transform's actual position, which is the thing an
        // anchor moves and the thing a bounding box hides.
        ctx.circle(
            bounds.center(),
            0.12,
            Color::rgb(1.0, 0.3, 0.3),
            Depth::layer(layers::DEBUG),
        );
    }
}

/// Score, a clock, and a line of prose — what text is actually for.
fn draw_the_readout(ctx: &mut DrawCtx) {
    let time = ctx.world.resource::<Time>();
    let camera = ctx.world.resource::<Camera>();
    let (top_left, bottom_right) = camera.visible_bounds();

    // Centred, by measuring. `width_of` is exact — the font is monospace with
    // no kerning — so this lines up rather than nearly lines up.
    let score = TextStyle {
        size: 1.6,
        color: Color::WHITE,
        depth: Depth::layer(layers::UI),
    };
    let text = "3 - 2";
    ctx.text(
        Vec2::new(-score.width_of(text) * 0.5, top_left.y + 1.0),
        text,
        score,
    );

    // A debug readout in the corner, one line per fact. Ticks rather than
    // seconds, because ticks are the canonical timeline (core.md §7).
    let readout = TextStyle {
        size: 0.7,
        color: Color::rgba(0.6, 0.9, 1.0, 0.9),
        depth: Depth::layer(layers::UI),
    };
    ctx.text(
        Vec2::new(top_left.x + 1.0, top_left.y + 1.0),
        &format!(
            "tick {}\nelapsed {:.1}s\nalpha {:.2}",
            time.tick,
            time.elapsed.as_f32(),
            time.alpha
        ),
        readout,
    );

    // And the whole printable range, so the font is inspectable at a glance —
    // this is the picture that would show a broken glyph.
    let sample = TextStyle {
        size: 0.6,
        color: Color::rgba(1.0, 1.0, 1.0, 0.55),
        depth: Depth::layer(layers::UI),
    };
    ctx.text(
        Vec2::new(top_left.x + 1.0, bottom_right.y - 2.6),
        " !\"#$%&'()*+,-./0123456789:;<=>?\n\
         @ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_\n\
         `abcdefghijklmnopqrstuvwxyz{|}~",
        sample,
    );
}

/// The asset store, reading from wherever this platform keeps files.
///
/// One line, no `cfg` — see `examples/sprites.rs` for why that is worth
/// remarking on.
fn art() -> Assets {
    Assets::new(asset_source(ASSET_ROOT))
}
