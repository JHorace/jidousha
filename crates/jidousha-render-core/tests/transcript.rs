//! The R0 exit criterion: what was drawn, where, in what order, in how many
//! batches — asserted without a GPU (renderer.md §9, §11).
//!
//! These are the questions an agent actually has about a frame. Every one of
//! them is answered here by reading a recorded frame, on every target,
//! including wasm CI.

use jidousha_assets::{Assets, MemorySource, TextureHandle};
use jidousha_core::math::{Radians, Vec2};
use jidousha_core::{Color, Draw, GameConfig, Transform, headless};
use jidousha_render_core::{
    BackendTextureId, Camera, FrameRecord, NullBackend, RenderBackend, Sprite, TextureTable,
    draw_sprites, plan_frame,
};

const WHITE: BackendTextureId = BackendTextureId(0);
const PLACEHOLDER: BackendTextureId = BackendTextureId(1);

/// An asset store with a few textures already asked for.
fn assets_with(paths: &[&str]) -> (Assets, Vec<TextureHandle>) {
    let mut source = MemorySource::new();
    for path in paths {
        source.insert(path, vec![0]);
    }
    let mut assets = Assets::new(source);
    let handles = paths.iter().map(|path| assets.load_texture(path)).collect();
    (assets, handles)
}

/// Draw a world of `(Transform, Sprite)` pairs and record the frame.
fn record(sprites: &[(Transform, Sprite)], camera: Camera, textures: &TextureTable) -> FrameRecord {
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Draw, draw_sprites);
    });
    sim.world_mut().insert_resource(camera);
    for (transform, sprite) in sprites {
        let entity = sim.world_mut().spawn();
        sim.world_mut().insert(entity, *transform);
        sim.world_mut().insert(entity, *sprite);
    }
    sim.tick();

    let quads = sim.draw().quads().to_vec();
    let mut backend = NullBackend::new();
    let plan = plan_frame(&camera, &quads, textures);
    let Ok(()) = backend.render(&plan) else {
        panic!("the null backend cannot fail to render");
    };
    match backend.last_frame() {
        Some(frame) => frame.clone(),
        None => panic!("a frame was just recorded"),
    }
}

fn ready_table(handles: &[TextureHandle]) -> TextureTable {
    let mut textures = TextureTable::new(WHITE, PLACEHOLDER);
    for (index, handle) in handles.iter().enumerate() {
        // Standing in for R2's upload: the ids the backend would have returned.
        textures.register(
            handle.texture_id(),
            BackendTextureId(u32::try_from(index).unwrap_or(0) + 2),
        );
    }
    textures
}

#[test]
fn a_sprite_is_drawn_where_the_game_put_it() {
    // The question `tools/verify` exists to answer, and R0's exit criterion:
    // "is there a sprite at world point P?"
    let (_, handles) = assets_with(&["ship.png"]);
    let frame = record(
        &[(
            Transform::at(Vec2::new(3.0, -2.0)),
            Sprite {
                size: Vec2::new(2.0, 2.0),
                ..Sprite::new(handles[0])
            },
        )],
        Camera::default(),
        &ready_table(&handles),
    );

    assert_eq!(frame.covering(Vec2::new(3.0, -2.0)).len(), 1, "dead center");
    assert_eq!(
        frame.covering(Vec2::new(3.9, -1.1)).len(),
        1,
        "inside a corner"
    );
    assert!(frame.covering(Vec2::new(5.0, -2.0)).is_empty(), "outside");
}

#[test]
fn a_rotated_sprite_covers_what_it_actually_covers() {
    // The bounding box would claim the corners; the sprite does not. An agent
    // asking "did the shot hit the ship" needs the exact answer.
    let (_, handles) = assets_with(&["ship.png"]);
    let frame = record(
        &[(
            Transform {
                rot: Radians::from_degrees(45.0),
                ..Transform::default()
            },
            Sprite {
                size: Vec2::new(2.0, 2.0),
                ..Sprite::new(handles[0])
            },
        )],
        Camera::default(),
        &ready_table(&handles),
    );

    assert_eq!(frame.covering(Vec2::ZERO).len(), 1, "the middle");
    assert!(
        frame.covering(Vec2::new(0.99, 0.99)).is_empty(),
        "a corner of the box the turned sprite has vacated"
    );
    assert_eq!(
        frame.covering(Vec2::new(0.0, 1.3)).len(),
        1,
        "and the point the turned sprite now reaches"
    );
}

#[test]
fn layers_draw_back_to_front_whatever_order_they_were_submitted_in() {
    let (_, handles) = assets_with(&["a.png"]);
    let background = Sprite {
        layer: -1,
        tint: Color::BLUE,
        ..Sprite::new(handles[0])
    };
    let foreground = Sprite {
        layer: 1,
        tint: Color::RED,
        ..Sprite::new(handles[0])
    };
    // Submitted foreground first, on purpose.
    let frame = record(
        &[
            (Transform::default(), foreground),
            (Transform::default(), background),
        ],
        Camera::default(),
        &ready_table(&handles),
    );

    let hits = frame.covering(Vec2::ZERO);
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].tint, Color::RED, "layer 1 is on top of layer -1");
}

#[test]
fn z_orders_within_a_layer() {
    let (_, handles) = assets_with(&["a.png"]);
    let sprite = Sprite::new(handles[0]);
    let frame = record(
        &[
            (
                Transform {
                    z: 5.0,
                    ..Transform::default()
                },
                Sprite {
                    tint: Color::RED,
                    ..sprite
                },
            ),
            (
                Transform {
                    z: 1.0,
                    ..Transform::default()
                },
                Sprite {
                    tint: Color::BLUE,
                    ..sprite
                },
            ),
        ],
        Camera::default(),
        &ready_table(&handles),
    );

    let hits = frame.covering(Vec2::ZERO);
    assert_eq!(hits[0].tint, Color::RED, "higher z draws on top");
}

#[test]
fn one_texture_across_many_sprites_is_one_batch() {
    let (_, handles) = assets_with(&["a.png"]);
    let sprite = Sprite::new(handles[0]);
    let sprites: Vec<(Transform, Sprite)> = (0..10)
        .map(|index| {
            (
                Transform::at(Vec2::new(index as f32, 0.0)),
                Sprite { ..sprite },
            )
        })
        .collect();
    let frame = record(&sprites, Camera::default(), &ready_table(&handles));

    assert_eq!(frame.plan.batches.len(), 1, "ten sprites, one draw call");
    assert_eq!(frame.quad_count(), 10);
}

#[test]
fn interleaved_textures_cost_a_batch_each() {
    // The cost model a game can reason about: batching follows draw order, and
    // draw order is the game's to choose. Sorting to merge more would break the
    // painter's algorithm.
    let (_, handles) = assets_with(&["a.png", "b.png"]);
    let table = ready_table(&handles);
    let sprites: Vec<(Transform, Sprite)> = (0..4)
        .map(|index| {
            (
                Transform {
                    z: index as f32,
                    ..Transform::default()
                },
                Sprite::new(handles[index % 2]),
            )
        })
        .collect();
    let frame = record(&sprites, Camera::default(), &table);
    assert_eq!(frame.plan.batches.len(), 4);

    // Give them all one texture and the same four sprites collapse to one.
    let same: Vec<(Transform, Sprite)> = sprites
        .iter()
        .map(|(transform, sprite)| {
            (
                *transform,
                Sprite {
                    texture: handles[0],
                    ..*sprite
                },
            )
        })
        .collect();
    assert_eq!(
        record(&same, Camera::default(), &table).plan.batches.len(),
        1
    );
}

#[test]
fn a_texture_that_has_not_arrived_draws_the_placeholder() {
    // The not-ready policy (renderer.md §5): loud, deterministic, non-fatal.
    // Assets are legitimately in flight during a game's first frames, and
    // panicking would make every startup a race.
    let (_, handles) = assets_with(&["ship.png"]);
    let nothing_uploaded = TextureTable::new(WHITE, PLACEHOLDER);
    let frame = record(
        &[(Transform::default(), Sprite::new(handles[0]))],
        Camera::default(),
        &nothing_uploaded,
    );

    assert_eq!(frame.quad_count(), 1, "it still draws");
    assert_eq!(frame.plan.batches[0].texture, PLACEHOLDER);
}

#[test]
fn sprites_waiting_on_different_textures_share_the_placeholder_batch() {
    // Which is why a loading screen full of not-yet-loaded art is cheap.
    let (_, handles) = assets_with(&["a.png", "b.png", "c.png"]);
    let nothing_uploaded = TextureTable::new(WHITE, PLACEHOLDER);
    let sprites: Vec<(Transform, Sprite)> = handles
        .iter()
        .map(|handle| (Transform::default(), Sprite::new(*handle)))
        .collect();
    let frame = record(&sprites, Camera::default(), &nothing_uploaded);

    assert_eq!(frame.plan.batches.len(), 1);
    assert_eq!(frame.quad_count(), 3);
}

#[test]
fn the_same_scene_transcribes_identically_every_time() {
    // CONTRACT: identical submission streams produce identical plans
    // (renderer.md §2). Every golden image later rests on this.
    let (_, handles) = assets_with(&["a.png", "b.png"]);
    let table = ready_table(&handles);
    let sprites: Vec<(Transform, Sprite)> = (0..6)
        .map(|index| {
            (
                Transform {
                    pos: Vec2::new(index as f32, -(index as f32)),
                    rot: Radians::from_degrees(index as f32 * 15.0),
                    z: (index % 3) as f32,
                    ..Transform::default()
                },
                Sprite::new(handles[index % 2]),
            )
        })
        .collect();

    let first = record(&sprites, Camera::default(), &table);
    let second = record(&sprites, Camera::default(), &table);
    assert_eq!(first.transcript(), second.transcript());
    assert_eq!(
        first.plan, second.plan,
        "identical to the bit, not just as text"
    );
}

#[test]
fn a_frame_transcribes_to_the_text_a_human_can_read_in_a_diff() {
    let (_, handles) = assets_with(&["a.png"]);
    let frame = record(
        &[
            (
                Transform::at(Vec2::new(-2.0, 0.0)),
                Sprite {
                    size: Vec2::new(1.0, 1.0),
                    tint: Color::RED,
                    ..Sprite::new(handles[0])
                },
            ),
            (
                Transform::at(Vec2::new(2.0, 1.0)),
                Sprite {
                    size: Vec2::new(2.0, 2.0),
                    ..Sprite::new(handles[0])
                },
            ),
        ],
        Camera {
            clear_color: Color::rgb(0.0, 0.0, 0.0),
            ..Camera::default()
        },
        &ready_table(&handles),
    );

    assert_eq!(
        frame.transcript(),
        "clear #000000ff\n\
         batch 0: texture 2 (2 quads)\n  \
         quad (-2.500, -0.500) (-1.500, 0.500) tint #ff0000ff\n  \
         quad (1.000, 0.000) (3.000, 2.000) tint #ffffffff\n"
    );
}

#[test]
fn an_empty_world_draws_an_empty_frame() {
    let frame = record(
        &[],
        Camera::default(),
        &TextureTable::new(WHITE, PLACEHOLDER),
    );
    assert_eq!(frame.quad_count(), 0);
    assert!(frame.plan.batches.is_empty());
    assert_eq!(frame.transcript(), "clear #000000ff\n");
}

#[test]
fn the_camera_decides_what_a_screen_pixel_is_over() {
    // Transcripts plus camera math is what turns "the player clicked at pixel
    // (640, 360)" into "they clicked the ship" — with no renderer running.
    let (_, handles) = assets_with(&["ship.png"]);
    let camera = Camera::default();
    let frame = record(
        &[(
            Transform::at(Vec2::new(0.0, 0.0)),
            Sprite {
                size: Vec2::new(4.0, 4.0),
                ..Sprite::new(handles[0])
            },
        )],
        camera,
        &ready_table(&handles),
    );

    let middle = camera.screen_to_world(Vec2::new(640.0, 360.0));
    assert_eq!(frame.covering(middle).len(), 1, "the center of the screen");

    let corner = camera.screen_to_world(Vec2::new(10.0, 10.0));
    assert!(
        frame.covering(corner).is_empty(),
        "the top-left of the screen"
    );
}

#[test]
fn each_frame_starts_empty() {
    // Submissions are immediate-mode: nothing is retained across frames at the
    // API level (renderer.md §2), so a game that stops drawing something stops
    // drawing it.
    let (_, handles) = assets_with(&["a.png"]);
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Draw, draw_sprites);
    });
    sim.world_mut().insert_resource(Camera::default());
    let entity = sim.world_mut().spawn();
    sim.world_mut().insert(entity, Transform::default());
    sim.world_mut().insert(entity, Sprite::new(handles[0]));
    sim.tick();

    assert_eq!(sim.draw().quads().len(), 1);
    assert_eq!(sim.draw().quads().len(), 1, "not two");

    sim.world_mut().despawn(entity);
    assert_eq!(sim.draw().quads().len(), 0);
}
