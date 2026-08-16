//! What a game looks like while its art is still arriving (renderer.md §5,
//! assets.md §7).
//!
//! A0 wrote the readiness half of these transcripts and said the placeholder
//! half needed a renderer. This is that half: a sprite drawn every frame from
//! the first, whose texture is scripted to arrive at a known tick, asserted on
//! both sides of that tick. No GPU — the null backend records what a real one
//! would have been told, which is exactly where the placeholder policy lives.

use jidousha_assets::{Assets, MemorySource, TextureData};
use jidousha_core::math::Vec2;
use jidousha_core::{Draw, GameConfig, Transform, headless};
use jidousha_render_core::{
    Camera, NullBackend, RenderBackend, Sprite, TextureTable, create_builtin_textures,
    draw_sprites, plan_frame, upload_ready_textures,
};

/// A texture of one flat value, big enough to tell two of them apart by size.
fn texture(width: u32, height: u32, fill: u8) -> TextureData {
    TextureData {
        width,
        height,
        rgba: vec![fill; (width * height * 4) as usize],
    }
}

/// One frame of a game whose art is loading: commit, upload, tick, draw.
///
/// The order is the driver's (`jidousha-platform`'s `settle_assets` then
/// `frame`), written out here so a test can step it by hand.
struct Game {
    sim: jidousha_core::HeadlessSim,
    backend: NullBackend,
    textures: TextureTable,
    tick: u64,
    /// How many textures existed before any asset loaded.
    ///
    /// Counted rather than written down: the built-in set has grown once
    /// already, when R3 added the font, and a test asserting "and nothing else"
    /// should be about the asset that did not load rather than about how many
    /// textures the renderer happens to ship with.
    built_in: usize,
}

impl Game {
    fn new() -> Self {
        let mut backend = NullBackend::new();
        let textures = create_builtin_textures(&mut backend);
        let sim = headless(GameConfig::default(), |app| {
            app.add_system(Draw, draw_sprites);
        });
        let built_in = backend.texture_count();
        Self {
            sim,
            backend,
            textures,
            tick: 0,
            built_in,
        }
    }

    fn frame(&mut self) {
        let Some(assets) = self.sim.world_mut().find_resource_mut::<Assets>() else {
            panic!("this test always has a store");
        };
        assets.commit(self.tick);
        upload_ready_textures(assets, &mut self.backend, &mut self.textures);
        self.sim.tick();
        self.tick += 1;

        let camera = *self.sim.world().resource::<Camera>();
        let quads = self.sim.draw().quads().to_vec();
        let plan = plan_frame(&camera, &quads, &self.textures);
        let Ok(()) = self.backend.render(&plan) else {
            panic!("the null backend cannot fail to render");
        };
    }

    /// Which backend texture the single quad of the last frame sampled.
    fn drawn_texture(&self) -> jidousha_render_core::BackendTextureId {
        let Some(frame) = self.backend.last_frame() else {
            panic!("a frame was drawn");
        };
        let quads = frame.quads();
        assert_eq!(quads.len(), 1, "one sprite, one quad");
        quads[0].texture
    }
}

#[test]
fn a_sprite_draws_the_placeholder_until_its_texture_arrives_and_itself_after() {
    let mut game = Game::new();
    let mut source = MemorySource::new();
    source.insert_texture("hero.png", texture(2, 2, 200));
    source.complete_at("hero.png", 3);
    let mut assets = Assets::new(source);
    let hero = assets.load_texture("hero.png");
    game.sim.world_mut().insert_resource(assets);
    game.sim.world_mut().insert_resource(Camera::default());

    let entity = game.sim.world_mut().spawn();
    game.sim.world_mut().insert(entity, Transform::default());
    game.sim.world_mut().insert(
        entity,
        Sprite {
            size: Vec2::new(4.0, 4.0),
            ..Sprite::new(hero)
        },
    );

    // Frames committing ticks 0, 1 and 2: nothing has arrived, and the sprite
    // draws anyway. Drawing anyway is the whole point of the policy — a game
    // whose first frames were blank or fatal would be unusable while loading.
    for tick in 0..3 {
        game.frame();
        assert_eq!(
            game.drawn_texture(),
            game.textures.placeholder(),
            "tick {tick} is before the texture arrives"
        );
    }

    // The frame that commits tick 3 is the frame it lands on, and it is drawn
    // with the real texture in that same frame rather than the next.
    game.frame();
    let drawn = game.drawn_texture();
    assert_ne!(drawn, game.textures.placeholder());
    assert_eq!(drawn, game.textures.resolve(hero.texture_id()));

    let Some((desc, texels)) = game.backend.uploaded(drawn) else {
        panic!("the texture was uploaded");
    };
    assert_eq!(desc.size.width, 2);
    assert_eq!(
        texels[0], 200,
        "the texels the store decoded, not a stand-in"
    );
}

#[test]
fn a_sprite_whose_texture_never_arrives_draws_the_placeholder_forever() {
    // A failed load is not a state a game recovers from, and it is not a state
    // that stops it either: the placeholder keeps saying so, every frame.
    let mut game = Game::new();
    let mut assets = Assets::new(MemorySource::new());
    let missing = assets.load_texture("nowhere.png");
    game.sim.world_mut().insert_resource(assets);
    game.sim.world_mut().insert_resource(Camera::default());

    let entity = game.sim.world_mut().spawn();
    game.sim.world_mut().insert(entity, Transform::default());
    game.sim.world_mut().insert(entity, Sprite::new(missing));

    for _ in 0..5 {
        game.frame();
        assert_eq!(game.drawn_texture(), game.textures.placeholder());
    }
    assert_eq!(
        game.backend.texture_count(),
        game.built_in,
        "the built-ins and nothing else"
    );
}

#[test]
fn two_sprites_waiting_on_different_textures_merge_and_then_split() {
    // While they are both placeholders they share a batch, which is why a
    // loading screen is cheap. When the art arrives they stop sharing, and a
    // game can see the batch count change — that is the same transcript that
    // tells it whether its atlas is doing any good.
    let mut game = Game::new();
    let mut source = MemorySource::new();
    source.insert_texture("a.png", texture(1, 1, 1));
    source.insert_texture("b.png", texture(1, 1, 2));
    source.complete_at("a.png", 2);
    source.complete_at("b.png", 2);
    let mut assets = Assets::new(source);
    let (a, b) = (assets.load_texture("a.png"), assets.load_texture("b.png"));
    game.sim.world_mut().insert_resource(assets);
    game.sim.world_mut().insert_resource(Camera::default());

    for (index, handle) in [a, b].into_iter().enumerate() {
        let entity = game.sim.world_mut().spawn();
        game.sim
            .world_mut()
            .insert(entity, Transform::at(Vec2::new(index as f32 * 2.0, 0.0)));
        game.sim.world_mut().insert(entity, Sprite::new(handle));
    }

    game.frame();
    let Some(frame) = game.backend.last_frame() else {
        panic!("a frame was drawn");
    };
    assert_eq!(frame.plan.batches.len(), 1, "both are the placeholder");

    game.frame();
    game.frame();
    let Some(frame) = game.backend.last_frame() else {
        panic!("a frame was drawn");
    };
    assert_eq!(frame.plan.batches.len(), 2, "two textures, two draw calls");
}
