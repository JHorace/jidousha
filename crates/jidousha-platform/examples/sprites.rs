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
//! Run it: `cargo run -p jidousha-platform --example sprites`
//! On the web: `tools/serve-web sprites`
//!
//! DELIBERATE: built but not run by `tools/test`, like the other windowed
//! examples — it opens a window and waits for a person (tooling.md).

use jidousha_assets::Assets;
use jidousha_core::{
    Color, Component, GameConfig, Rect, Startup, Time, Transform, Update, World,
    math::{Radians, Vec2, sin_cos},
};
use jidousha_render_core::{Camera, Sprite, draw_sprites};

/// Where the art lives, relative to the workspace root (assets.md §2).
#[cfg(not(target_arch = "wasm32"))]
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

fn main() -> Result<(), jidousha_platform::RunError> {
    jidousha_platform::run(
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
            app.add_system(jidousha_core::Draw, draw_sprites);
        },
    )
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
#[cfg(not(target_arch = "wasm32"))]
fn art() -> Assets {
    Assets::new(jidousha_platform::FileSource::new(ASSET_ROOT))
}

/// The web has no filesystem and no fetch source until A2, so the same three
/// PNGs are compiled in and decoded at startup.
///
/// DELIBERATE and temporary. It exists so this example can be checked in a real
/// browser today — `tools/serve-web sprites --check` is how "sprites are
/// visible on all targets" gets verified — rather than waiting for the loader
/// that will replace it. A2 deletes this function and the `cfg` above it; the
/// rest of the example does not change, which is the point of the `ByteSource`
/// seam (assets.md §5).
#[cfg(target_arch = "wasm32")]
fn art() -> Assets {
    use jidousha_assets::{AssetKind, MemorySource, decode_png};

    let mut source = MemorySource::new();
    for (path, bytes) in [
        (
            "sprites/hero.png",
            include_bytes!("../../../assets/sprites/hero.png").as_slice(),
        ),
        (
            "sprites/glow.png",
            include_bytes!("../../../assets/sprites/glow.png").as_slice(),
        ),
        (
            "sprites/atlas.png",
            include_bytes!("../../../assets/sprites/atlas.png").as_slice(),
        ),
    ] {
        match decode_png(bytes) {
            Ok(texture) => source.insert_texture(path, texture),
            Err(error) => panic!(
                "{}",
                error.message(path, AssetKind::Texture, "examples/sprites.rs")
            ),
        }
    }
    // `sprites/not_here.png` is absent from this source too, so the placeholder
    // appears on the web for the same reason it does natively.
    Assets::new(source)
}
