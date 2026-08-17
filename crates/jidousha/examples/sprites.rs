//! Art on the screen: the sprite pipeline end to end (renderer.md §11, R2).
//!
//! Everything R2 added, in one window. Textures are loaded from `assets/`,
//! uploaded the frame they arrive, and drawn as textured quads: rotating,
//! moving, tinted, and sampling regions of an atlas. One sprite asks for a file
//! that is not there, so the checkered magenta placeholder is on screen too —
//! that is the engine saying "this did not load", and it is worth knowing what
//! it looks like before a real game shows it to you.
//!
//! Nothing here talks to the renderer. A game spawns entities with a
//! `Transform` and a `Sprite`, registers `draw_sprites`, and that is the whole
//! of it (renderer.md §2).
//!
//! Run it: `cargo run -p jidousha --example sprites`
//! On the web: `tools/serve-web sprites`
//!
//! DELIBERATE: built but not run by `tools/test`, like the other windowed
//! examples — it opens a window and waits for a person (tooling.md).

use std::process::ExitCode;

use jidousha::prelude::*;

/// Where the art lives, relative to the workspace root (assets.md §2).
const ASSET_ROOT: &str = "assets";

/// How fast something turns, in radians per second.
#[derive(Clone, Copy)]
struct Spin(f32);
impl Component for Spin {}

/// A circle to walk around, and how fast to walk it.
#[derive(Clone, Copy)]
struct Orbit {
    center: Vec2,
    radius: f32,
    rate: f32,
}
impl Component for Orbit {}

fn main() -> ExitCode {
    match run(
        GameConfig {
            title: "jidousha — sprites",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, set_the_scene);
            app.add_system(Update, turn);
            app.add_system(Update, walk_the_circle);
            // The provided Draw system: every entity with a Transform and a
            // Sprite, submitted in query order (renderer.md §2).
            app.add_system(jidousha::Draw, draw_sprites);
        },
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        // A blue-grey, so the black in the placeholder's checks is visible
        // against it and a fully-transparent sprite is not.
        clear_color: Color::rgb(0.09, 0.11, 0.16),
        height: 20.0,
        ..Camera::default()
    });
    world.insert_resource(art());

    let assets = world.resource_mut::<Assets>();
    let hero = assets.load_texture("sprites/hero.png");
    let glow = assets.load_texture("sprites/glow.png");
    let atlas = assets.load_texture("sprites/atlas.png");
    // Deliberately absent. It draws the placeholder and reports one §9 error at
    // the commit that resolves it (renderer.md §5, assets.md §6).
    // check-assets: deliberately missing
    let missing = assets.load_texture("sprites/not_here.png");

    // The atlas is four tiles in a 2×2 grid; regions are normalized 0..1, so a
    // tile is a quarter of the texture on each axis (renderer.md §3).
    for (index, position) in [-6.0, -2.0, 2.0, 6.0].into_iter().enumerate() {
        let column = (index % 2) as f32 * 0.5;
        let row = (index / 2) as f32 * 0.5;
        let tile = world.spawn();
        world.insert(tile, Transform::at(Vec2::new(position, 6.0)));
        world.insert(
            tile,
            Sprite {
                region: Some(Rect::from_min_size(
                    Vec2::new(column, row),
                    Vec2::new(0.5, 0.5),
                )),
                size: Vec2::new(3.0, 3.0),
                // The last one is mirrored: its white corner notch moves and
                // its shape does not, which is what flipping means.
                flip_x: index == 3,
                ..Sprite::new(atlas)
            },
        );
    }

    // The workhorse: a sprite that turns. Rotation is clockwise on screen
    // (ADR-0010), about the transform's position.
    let spinner = world.spawn();
    world.insert(spinner, Transform::at(Vec2::ZERO));
    world.insert(spinner, Spin(0.6));
    world.insert(
        spinner,
        Sprite {
            size: Vec2::new(6.0, 6.0),
            ..Sprite::new(hero)
        },
    );

    // Tinted and half-transparent, on a layer above everything, orbiting the
    // spinner — so the draw order and the alpha blending are both visible.
    let light = world.spawn();
    world.insert(light, Transform::at(Vec2::ZERO));
    world.insert(
        light,
        Orbit {
            center: Vec2::ZERO,
            radius: 6.0,
            rate: 0.9,
        },
    );
    world.insert(
        light,
        Sprite {
            size: Vec2::new(5.0, 5.0),
            tint: Color::rgba(1.0, 0.85, 0.4, 0.85),
            layer: 1,
            ..Sprite::new(glow)
        },
    );

    // And the one that will not arrive.
    let gap = world.spawn();
    world.insert(gap, Transform::at(Vec2::new(-11.0, -5.0)));
    world.insert(
        gap,
        Sprite {
            size: Vec2::new(4.0, 4.0),
            ..Sprite::new(missing)
        },
    );

    println!("window open — close it to quit");
    println!("the checkered magenta square is a texture that did not load, on purpose");
}

/// Turn everything that spins, by simulated time rather than wall clock.
///
/// The angle at tick 600 is the same angle on every machine, however the frames
/// fell (core.md §7).
fn turn(world: &mut World) {
    let elapsed = world.resource::<Time>().elapsed.as_f32();
    for (_, transform, spin) in world.query_mut::<(&mut Transform, &Spin)>() {
        transform.rot = Radians(elapsed * spin.0);
    }
}

/// Walk the orbiters around their circles.
fn walk_the_circle(world: &mut World) {
    let elapsed = world.resource::<Time>().elapsed.as_f32();
    for (_, transform, orbit) in world.query_mut::<(&mut Transform, &Orbit)>() {
        // The engine's own trigonometry: bit-identical on every platform, which
        // std's is not (ADR-0009).
        let (sine, cosine) = sin_cos(Radians(elapsed * orbit.rate));
        transform.pos = orbit.center + Vec2::new(cosine, sine) * orbit.radius;
    }
}

/// The asset store, reading from wherever this platform keeps files.
///
/// One line, no `cfg`: `asset_source` is the platform crate's job and this is
/// what it exists to absorb. Until A2 this function had a second body that
/// compiled the PNGs in, because the web had no loader; the loader landed and
/// the second body went away without the rest of the example changing, which is
/// the `ByteSource` seam doing exactly what it was for (assets.md §5).
fn art() -> Assets {
    Assets::new(asset_source(ASSET_ROOT))
}
