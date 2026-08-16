//! What rectangles, lines, circles and text actually put in a frame
//! (renderer.md §11, R3).
//!
//! The unit tests beside each expansion check its geometry. These check the
//! thing that only shows up once a whole frame exists: that a shape and a
//! sprite are the *same kind of thing* by the time anything is sorted, so they
//! interleave by depth instead of one class always winning — which is the
//! difference between a debug overlay and a debug primitive.

use jidousha_core::math::Vec2;
use jidousha_core::{Color, Depth, Draw, DrawCtx, GameConfig, Rect, Transform, headless};
use jidousha_render_core::{
    Camera, FrameRecord, NullBackend, RenderBackend, Sprite, Submit, TextStyle,
    create_builtin_textures, plan_frame,
};

/// Record one frame drawn by `draw`, with the real built-in textures.
fn record(draw: fn(&mut DrawCtx)) -> (FrameRecord, NullBackend) {
    let mut backend = NullBackend::new();
    let textures = create_builtin_textures(&mut backend);
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Draw, draw);
    });
    let camera = Camera::default();
    sim.world_mut().insert_resource(camera);
    sim.tick();

    let quads = sim.draw().quads().to_vec();
    let plan = plan_frame(&camera, &quads, &textures);
    let Ok(()) = backend.render(&plan) else {
        panic!("the null backend cannot fail to render");
    };
    match backend.last_frame() {
        Some(frame) => (frame.clone(), backend),
        None => panic!("a frame was recorded"),
    }
}

#[test]
fn a_rectangle_lands_where_it_was_asked_for() {
    fn draw(ctx: &mut DrawCtx) {
        ctx.rect(
            Rect::from_center_size(Vec2::new(2.0, -1.0), Vec2::new(4.0, 2.0)),
            Color::RED,
            Depth::default(),
        );
    }
    let (frame, _) = record(draw);
    assert_eq!(frame.quad_count(), 1);
    let hits = frame.covering(Vec2::new(2.0, -1.0));
    assert_eq!(hits.len(), 1, "the middle of it");
    assert_eq!(hits[0].tint, Color::RED);
    assert!(
        frame.covering(Vec2::new(5.0, -1.0)).is_empty(),
        "just outside the right edge"
    );
}

#[test]
fn a_circle_covers_a_disc_and_not_its_bounding_box() {
    // The reason circles expand into a fan of convex quads rather than one big
    // quad: "did the player click the ball" has to be answerable exactly, and a
    // bounding box claims four corners the ball does not cover.
    fn draw(ctx: &mut DrawCtx) {
        ctx.circle(Vec2::ZERO, 3.0, Color::GREEN, Depth::default());
    }
    let (frame, _) = record(draw);
    assert_eq!(frame.quad_count(), 16, "thirty-two segments, two per quad");

    // A point strictly inside one wedge is claimed by exactly that wedge.
    assert_eq!(frame.covering(Vec2::new(1.5, 0.2)).len(), 1);
    // The centre is a corner of all sixteen, and a point on a shared edge
    // belongs to both quads that meet there. That is the containment rule doing
    // what it should — a game asks "is anything here", not "how many".
    assert_eq!(
        frame.covering(Vec2::ZERO).len(),
        16,
        "every wedge meets here"
    );
    assert!(!frame.covering(Vec2::new(2.9, 0.0)).is_empty(), "inside");
    assert!(
        frame.covering(Vec2::new(3.2, 0.0)).is_empty(),
        "just outside the rim"
    );
    assert!(
        frame.covering(Vec2::new(2.9, 2.9)).is_empty(),
        "inside the bounding box, outside the disc"
    );
}

#[test]
fn a_line_covers_the_band_between_its_ends() {
    fn draw(ctx: &mut DrawCtx) {
        ctx.line(
            Vec2::new(-5.0, 0.0),
            Vec2::new(5.0, 0.0),
            1.0,
            Color::WHITE,
            Depth::default(),
        );
    }
    let (frame, _) = record(draw);
    assert_eq!(frame.covering(Vec2::new(0.0, 0.4)).len(), 1, "inside");
    assert!(
        frame.covering(Vec2::new(0.0, 0.6)).is_empty(),
        "past the half-thickness"
    );
    assert!(
        frame.covering(Vec2::new(6.0, 0.0)).is_empty(),
        "past the end"
    );
}

#[test]
fn shapes_and_sprites_interleave_by_depth() {
    // The whole point of one submission stream: a debug rectangle can go *under*
    // a sprite. An engine with a separate debug pass cannot do this, and every
    // hitbox it draws sits on top of everything whether that helps or not.
    fn draw(ctx: &mut DrawCtx) {
        ctx.rect(Rect::UNIT, Color::RED, Depth { layer: 0, z: 2.0 });
        ctx.circle(Vec2::ZERO, 1.0, Color::GREEN, Depth { layer: 0, z: 0.0 });
        ctx.rect(Rect::UNIT, Color::BLUE, Depth { layer: 0, z: 1.0 });
    }
    let (frame, _) = record(draw);
    let quads = frame.quads();
    // The circle is 16 quads at z 0, then blue at z 1, then red at z 2.
    assert_eq!(quads[0].tint, Color::GREEN, "lowest z draws first");
    assert_eq!(quads[16].tint, Color::BLUE);
    assert_eq!(quads[17].tint, Color::RED, "highest z draws last");
}

#[test]
fn every_untextured_shape_shares_one_batch() {
    // They all sample the white texel, so a frame full of debug shapes is one
    // draw call however many shapes it has.
    fn draw(ctx: &mut DrawCtx) {
        ctx.rect(Rect::UNIT, Color::RED, Depth::default());
        ctx.line(Vec2::ZERO, Vec2::X, 1.0, Color::WHITE, Depth::default());
        ctx.circle(Vec2::ZERO, 1.0, Color::GREEN, Depth::default());
    }
    let (frame, _) = record(draw);
    assert_eq!(frame.plan.batches.len(), 1, "one draw call for all of it");
    assert_eq!(frame.quad_count(), 18, "a rect, a line, and sixteen wedges");
}

#[test]
fn text_becomes_one_quad_per_character_from_the_font_atlas() {
    fn draw(ctx: &mut DrawCtx) {
        ctx.text(Vec2::new(-4.0, -3.0), "hi", TextStyle::default());
    }
    let (frame, backend) = record(draw);
    assert_eq!(frame.quad_count(), 2, "two characters, two quads");

    // The font is a texture like any other, so it gets its own batch — and the
    // texels behind it are the atlas, not the placeholder.
    let batch = frame.plan.batches[0].texture;
    let Some((desc, texels)) = backend.uploaded(batch) else {
        panic!("the font atlas was uploaded");
    };
    assert_eq!(desc.size.width, 112, "sixteen cells of seven texels");
    assert!(
        texels.chunks_exact(4).any(|texel| texel[3] == 255),
        "the atlas has ink in it"
    );
}

#[test]
fn text_sits_where_it_says_it_does_and_measures_what_it_occupies() {
    // A game centres a score by measuring it, so the measurement has to agree
    // with where the glyphs actually land rather than approximate it.
    const STYLE: TextStyle = TextStyle {
        size: 2.0,
        color: Color::WHITE,
        depth: Depth { layer: 0, z: 0.0 },
    };
    fn draw(ctx: &mut DrawCtx) {
        ctx.text(Vec2::new(-3.0, 1.0), "abc", STYLE);
    }
    let (frame, _) = record(draw);
    let quads = frame.quads();
    assert_eq!(quads.len(), 3);

    let first = quads[0].bounds();
    assert_eq!(first.min, Vec2::new(-3.0, 1.0), "the top-left of the first");
    let last = quads[2].bounds();
    assert!(
        (last.max.x - (-3.0 + STYLE.width_of("abc"))).abs() < 1e-5,
        "measured {} but drew to {}",
        STYLE.width_of("abc"),
        last.max.x
    );
}

#[test]
fn text_is_tinted_by_its_style() {
    // The glyphs are white in the atlas and the colour arrives as a tint, which
    // is the same mechanism a tinted sprite uses.
    fn draw(ctx: &mut DrawCtx) {
        ctx.text(
            Vec2::ZERO,
            "x",
            TextStyle {
                color: Color::RED,
                ..TextStyle::default()
            },
        );
    }
    let (frame, _) = record(draw);
    assert_eq!(frame.quads()[0].tint, Color::RED);
}

#[test]
fn a_sprite_and_a_line_of_text_are_two_batches_in_depth_order() {
    // Text obeys layers like everything else: a score on layer 1 draws over a
    // sprite on layer 0, and the batch order says so.
    fn draw(ctx: &mut DrawCtx) {
        let sprite = Sprite::new(jidousha_assets::Assets::new(memory()).load_texture("a.png"));
        ctx.sprite(&Transform::default(), &sprite);
        ctx.text(
            Vec2::ZERO,
            "hp",
            TextStyle {
                depth: Depth::layer(1),
                ..TextStyle::default()
            },
        );
    }
    fn memory() -> jidousha_assets::MemorySource {
        let mut source = jidousha_assets::MemorySource::new();
        source.insert("a.png", vec![0]);
        source
    }
    let (frame, _) = record(draw);
    assert_eq!(frame.plan.batches.len(), 2);
    assert_eq!(
        frame.plan.batches[0].quad_count(),
        1,
        "the sprite, on layer 0, first"
    );
    assert_eq!(frame.plan.batches[1].quad_count(), 2, "then the two glyphs");
}

#[test]
fn the_transcript_of_a_small_scene_is_stable_text() {
    // The snapshot R3's exit criterion asks for: a frame of primitives, as
    // diffable text. Positions to three decimals, so a sprite moving by a pixel
    // shows and the last bit of a rotation does not.
    fn draw(ctx: &mut DrawCtx) {
        ctx.rect(
            Rect::from_min_size(Vec2::new(-2.0, -1.0), Vec2::new(4.0, 2.0)),
            Color::rgba(1.0, 0.0, 0.0, 0.5),
            Depth::default(),
        );
        ctx.line(
            Vec2::new(-2.0, 0.0),
            Vec2::new(2.0, 0.0),
            0.5,
            Color::WHITE,
            Depth { layer: 0, z: 1.0 },
        );
    }
    let (frame, _) = record(draw);
    assert_eq!(
        frame.transcript(),
        "clear #000000ff\n\
         batch 0: texture 0 (2 quads)\n  \
         quad (-2.000, -1.000) (2.000, 1.000) tint #ff000080\n  \
         quad (-2.000, -0.250) (2.000, 0.250) tint #ffffffff\n"
    );
}
