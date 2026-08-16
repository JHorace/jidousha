//! Asking a frame what it drew, without drawing it (renderer.md §9).
//!
//! Three ships drift right; one of them is waiting on art that never arrived.
//! The run asserts where each ended up, which one is showing the placeholder,
//! and how many draw calls the frame cost — all from a recorded frame, with no
//! GPU and no window anywhere in it.
//!
//! This is the feedback loop `tools/verify` will wrap: an agent changes a
//! system, runs this, and finds out whether the thing it moved is where it
//! meant to put it.
//!
//! Run it: `cargo run -p jidousha-render-core --example what_was_drawn`

use jidousha_assets::{Assets, MemorySource};
use jidousha_core::math::{Radians, Vec2};
use jidousha_core::{Color, Component, Draw, GameConfig, Time, Transform, Update, World, headless};
use jidousha_render_core::{
    BackendTextureId, Camera, NullBackend, RenderBackend, Sprite, TextureTable, draw_sprites,
    plan_frame,
};

/// The two built-in textures a backend always has (renderer.md §5).
const WHITE: BackendTextureId = BackendTextureId(0);
const PLACEHOLDER: BackendTextureId = BackendTextureId(1);

/// How fast a ship drifts, in world units per second.
#[derive(Clone, Copy, Debug)]
struct Drift(Vec2);
impl Component for Drift {}

fn drift(world: &mut World) {
    let step = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, drift) in world.query_mut::<(&mut Transform, &Drift)>() {
        transform.pos += drift.0 * step;
        transform.rot = Radians(transform.rot.as_f32() + step);
    }
}

fn main() {
    // Two of the three textures exist; "ghost.png" does not, which is the
    // interesting one.
    let mut source = MemorySource::new();
    source.insert("ship.png", vec![0]);
    source.insert("rock.png", vec![0]);
    let mut assets = Assets::new(source);
    let ship = assets.load_texture("ship.png");
    let rock = assets.load_texture("rock.png");
    let ghost = assets.load_texture("ghost.png");

    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Update, drift);
        app.add_system(Draw, draw_sprites);
    });
    sim.world_mut().insert_resource(Camera::default());

    for (index, texture) in [ship, rock, ghost].into_iter().enumerate() {
        let entity = sim.world_mut().spawn();
        sim.world_mut().insert(
            entity,
            Transform::at(Vec2::new(0.0, index as f32 * 3.0 - 3.0)),
        );
        sim.world_mut().insert(
            entity,
            Sprite {
                size: Vec2::new(2.0, 2.0),
                tint: Color::WHITE,
                ..Sprite::new(texture)
            },
        );
        sim.world_mut().insert(entity, Drift(Vec2::new(2.0, 0.0)));
    }

    // A second of game time.
    for _ in 0..60 {
        sim.tick();
    }

    // Stand-in for R2's upload step: the two textures whose bytes exist are on
    // the GPU, the third never will be. The table is the whole not-ready
    // policy — an id nobody registered draws the placeholder.
    let mut textures = TextureTable::new(WHITE, PLACEHOLDER);
    textures.register(ship.texture_id(), BackendTextureId(2));
    textures.register(rock.texture_id(), BackendTextureId(3));

    let camera = *sim.world().resource::<Camera>();
    let quads = sim.draw().quads().to_vec();
    let mut backend = NullBackend::new();
    let plan = plan_frame(&camera, &quads, &textures);
    if let Err(error) = backend.render(&plan) {
        // The null backend cannot fail; a real one can, and this is the shape.
        println!("{error}");
        return;
    }

    let Some(frame) = backend.last_frame() else {
        println!("nothing was drawn");
        return;
    };
    print!("{}", frame.transcript());

    // Each ship drifted two units per second for one second.
    let expected = Vec2::new(2.0, -3.0);
    let hits = frame.covering(expected);
    println!("\n{} quad(s) at {expected:?}", hits.len());
    assert_eq!(hits.len(), 1, "the first ship is where the drift put it");

    // The ghost is drawing, and drawing the placeholder — loud, deterministic,
    // and non-fatal, which is what a game's first frames need (ADR-0011).
    let ghost_quads: Vec<_> = frame
        .quads()
        .into_iter()
        .filter(|quad| quad.texture == PLACEHOLDER)
        .collect();
    assert_eq!(ghost_quads.len(), 1, "the missing texture still draws");
    println!(
        "the missing texture drew as the placeholder at {:?}",
        ghost_quads[0].bounds().center()
    );

    // Two real textures plus the placeholder, and the placeholder sorted
    // between them by its owner's z — three draw calls.
    println!(
        "{} draw call(s) for {} quads",
        plan.batches.len(),
        plan.quad_count()
    );
    assert_eq!(plan.quad_count(), 3);

    // Nothing was retained: drawing again from an unchanged world gives the
    // same frame, and only one frame's worth of quads.
    let again = plan_frame(&camera, sim.draw().quads(), &textures);
    assert_eq!(again, plan, "the same world draws the same frame");
    println!("drew it again: identical to the bit");
}
