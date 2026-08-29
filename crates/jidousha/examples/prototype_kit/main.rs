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

use std::process::ExitCode;

use jidousha::prelude::*;

/// Where the art lives, relative to the workspace root (assets.md §2).
const ASSET_ROOT: &str = "assets";

/// The world is twenty units tall; everything below is in those units.
const VIEW_HEIGHT: f32 = 20.0;

/// How big the art is drawn, in world units.
const ART_SIZE: Vec2 = Vec2::new(3.0, 3.0);

/// What a hitbox outline is drawn in.
const HITBOX_LINE: Color = Color::rgba(0.2, 1.0, 0.4, 0.9);

/// How thick a hitbox outline's lines are, in world units.
const HITBOX_THICKNESS: f32 = 0.08;

/// How big the centre marking is, as a radius in world units.
const CENTRE_RADIUS: f32 = 3.0;

/// What the field markings are drawn in.
const FIELD_LINE: Color = Color::rgba(1.0, 1.0, 1.0, 0.18);

/// The dot a hitbox puts on the transform's own position.
const HITBOX_DOT: Color = Color::rgb(1.0, 0.3, 0.3);

/// What the court is cleared to.
///
/// Named because the verify run asserts it two ways: against this constant, and
/// against the requirement it exists to meet — the field markings are white and
/// have to read against it.
const COURT: Color = Color::rgb(0.07, 0.09, 0.13);

/// How big a paddle is drawn, in world units.
///
/// Stated once because the verify run asserts against it: a check that carried
/// its own copy of the number would keep passing after the paddle changed size.
const PADDLE_SIZE: Vec2 = Vec2::new(0.5, 4.0);

/// The score line.
const SCORE_TEXT: &str = "3 - 2";

/// The whole printable range, so the font is inspectable at a glance.
const FONT_SAMPLE: &str = " !\"#$%&'()*+,-./0123456789:;<=>?\n\
     @ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_\n\
     `abcdefghijklmnopqrstuvwxyz{|}~";

/// The debug readout, as one string.
///
/// A function rather than a `format!` inside the draw system so that a check can
/// ask the game for the exact text it draws. No assertion over drawn quads can
/// see a wrong *character* — the font draws an identically sized box for one —
/// so the only instrument is the string itself, and a check that cannot reach
/// the string has nothing to look at.
fn readout_text(tick: u64, elapsed: f32, alpha: f32) -> String {
    format!("tick {tick}\nelapsed {elapsed:.1}s\nalpha {alpha:.2}")
}

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
    /// World units per second.
    speed: f32,
    /// Half the travel allowed either side of the centre.
    limit: f32,
}
impl Component for Paddle {}

/// Where a drawn thing stood at the start of the tick it is now past.
///
/// **The other half of `Time::alpha`, and the game owns it.** The simulation
/// steps sixty times a second whatever the display does; a browser or a monitor
/// that does not present frames on that cadence shows one tick twice and skips
/// the next, and the eye reads that as a jump. `Draw` submits
/// `previous.lerp(current, alpha)` instead of `current`, and the motion is
/// smooth at the cost of one tick of latency.
///
/// The engine supplies the fraction and nothing else — no lerp helper, no
/// engine-side previous transform, because that would be retained render state
/// (renderer.md §2, e0-findings.md F-048). `examples/pong` is the same idiom
/// with more moving things, including the teleport rule this scene has no need
/// of: nothing here ever jumps, so nothing here ever has to snap `Previous`.
///
/// **The bouncing sprite deliberately does not have one**, and the difference
/// is visible on the same screen. It is drawn by
/// `jidousha::systems::draw_sprites`, an engine system that submits committed
/// state — so it steps at the tick rate while the paddle glides. A game that
/// wants its sprites interpolated writes its own `ctx.sprite` loop; this one
/// keeps the contrast, because seeing both is worth more here than smoothing
/// one.
#[derive(Clone, Copy)]
struct Previous(Vec2);
impl Component for Previous {}

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
    // First, and it has to be: it copies where things are into where they
    // *were*, so everything below moves away from a remembered position and
    // `Draw` has two ends to interpolate between.
    app.add_system(Update, remember_where_things_were);
    app.add_system(Update, drive_the_paddle);
    app.add_system(Update, bounce);
    app.add_system(Update, turn);
    // The hitboxes go down *first* and are drawn *last*, and the field goes down
    // after the art and is drawn behind it. Both orders are the bands' doing
    // rather than the submission sequence's, which is the claim `Depth` makes —
    // and it is also the only arrangement in which a check can see a band at
    // all: where submission order already agrees with the layers, swapping two
    // constants in `mod layers` changes nothing a recorded frame can show.
    app.add_system(Draw, draw_the_hitboxes);
    app.add_system(Draw, draw_sprites);
    app.add_system(Draw, draw_the_field);
    app.add_system(Draw, draw_the_readout);
}

mod capture;
mod checks;
mod verify;

fn main() -> ExitCode {
    // `tools/verify` runs this same binary with `--verify`: same systems, same
    // config, no window, scripted input, and assertions instead of a person.
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
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
        clear_color: COURT,
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
            size: ART_SIZE,
            layer: layers::PLAY,
            ..Sprite::new(hero)
        },
    );

    // The player's paddle, on the left.
    let paddle = world.spawn();
    let at = Vec2::new(-14.0, 0.0);
    world.insert(paddle, Transform::at(at));
    // Starting where it starts: a `Previous` of the origin would draw the first
    // frame with the paddle halfway to its post.
    world.insert(paddle, Previous(at));
    world.insert(
        paddle,
        Paddle {
            speed: 15.0,
            limit: 7.0,
        },
    );
}

/// Copy where everything is into where it was, before anything moves it.
///
/// The Update half of the interpolation idiom — see `Previous`. One loop over
/// everything that carries the component, so adding a moving thing to this
/// scene is one `world.insert` and no change here.
fn remember_where_things_were(world: &mut World) {
    for (_, previous, transform) in world.query_mut::<(&mut Previous, &Transform)>() {
        previous.0 = transform.pos;
    }
}

/// Move the paddle with W and S, clamped to the field.
///
/// The one system that reads input, and therefore the one a script can drive.
fn drive_the_paddle(world: &mut World) {
    let direction = match world.find_resource::<Input>() {
        // The first tick of a run can happen before any input is set, and a
        // game that assumed otherwise would panic on startup.
        None => return,
        Some(input) => f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    };
    // Per second, times the timestep — the same shape every other example uses,
    // so the paddle keeps its speed if `GameConfig::fixed_dt` ever changes
    // (conventions).
    let step = direction * world.resource::<Time>().fixed_dt.as_f32();
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
    let view = ctx.world.resource::<Camera>().visible_bounds();
    let inset = 0.6;
    let field = Rect {
        min: view.min + Vec2::splat(inset),
        max: view.max - Vec2::splat(inset),
    };
    let depth = Depth::layer(layers::FIELD);
    let line = FIELD_LINE;

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
        CENTRE_RADIUS,
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
    // Straight out of the query, with no `Vec` in between. A Draw system never
    // needs the two-pass collect: `query` hands back an iterator borrowed from
    // the *world*, not from `ctx`, so drawing inside the loop is fine. The
    // two-pass pattern belongs to `&mut World` systems, where the query really
    // does hold the thing being written to (`homing.rs` is that one).
    // The Draw half of the interpolation idiom (see `Previous`): where the
    // paddle is drawn is between where it was and where it is, and `alpha` says
    // where. Read once — it is the same number for every submission in a frame.
    let alpha = ctx.world.resource::<Time>().alpha;
    for (_, transform, previous, _) in ctx.world.query::<(&Transform, &Previous, &Paddle)>() {
        ctx.rect(
            Rect::from_center_size(previous.0.lerp(transform.pos, alpha), PADDLE_SIZE),
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
    let depth = Depth::layer(layers::DEBUG);
    for (_, transform, sprite) in ctx.world.query::<(&Transform, &Sprite)>() {
        let bounds = Rect::from_center_size(transform.pos, sprite.size);
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
                HITBOX_THICKNESS,
                HITBOX_LINE,
                depth,
            );
        }
        // And a dot on the transform's actual position, which is the thing an
        // anchor moves and the thing a bounding box hides.
        ctx.circle(
            bounds.center(),
            0.12,
            HITBOX_DOT,
            Depth::layer(layers::DEBUG),
        );
    }
}

/// Score, a clock, and a line of prose — what text is actually for.
fn draw_the_readout(ctx: &mut DrawCtx) {
    let time = ctx.world.resource::<Time>();
    let camera = ctx.world.resource::<Camera>();
    let view = camera.visible_bounds();

    // Centred, by measuring. `width_of` is exact — the font is monospace with
    // no kerning — so this lines up rather than nearly lines up.
    let score = TextStyle {
        face: Face::BUILT_IN,
        size: 1.6,
        color: Color::WHITE,
        depth: Depth::layer(layers::UI),
    };
    let text = SCORE_TEXT;
    ctx.text(
        Vec2::new(-score.width_of(text) * 0.5, view.min.y + 1.0),
        text,
        score,
    );

    // A debug readout in the corner, one line per fact. Ticks rather than
    // seconds, because ticks are the canonical timeline (core.md §7).
    let readout = TextStyle {
        face: Face::BUILT_IN,
        size: 0.7,
        color: Color::rgba(0.6, 0.9, 1.0, 0.9),
        depth: Depth::layer(layers::UI),
    };
    ctx.text(
        Vec2::new(view.min.x + 1.0, view.min.y + 1.0),
        &readout_text(time.tick, time.elapsed.as_f32(), time.alpha),
        readout,
    );

    // And the whole printable range, so the font is inspectable at a glance —
    // this is the picture that would show a broken glyph.
    let sample = TextStyle {
        face: Face::BUILT_IN,
        size: 0.6,
        color: Color::rgba(1.0, 1.0, 1.0, 0.55),
        depth: Depth::layer(layers::UI),
    };
    ctx.text(
        Vec2::new(view.min.x + 1.0, view.max.y - 2.6),
        FONT_SAMPLE,
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
