//! Text in a loaded face, from `ctx.text` to the texels on the backend.
//!
//! What the unit tests in `font/` cannot reach: that a face loaded from a real
//! file lays out through the ordinary draw path, that its atlas reaches a
//! backend through the ordinary texture path, and that the built-in font still
//! does exactly what it did (renderer.md §6, ADR-0042).

use jidousha_core::math::Vec2;
use jidousha_core::{Draw, DrawCtx, GameConfig, Resource, TextureId, headless};
use jidousha_render_core::{
    Face, Fonts, NullBackend, Submit, TextStyle, TextureTable, create_builtin_textures,
    upload_text_atlases,
};

/// The family this repository ships, from the asset root it ships in.
///
/// Read from disk rather than compiled in, because that is how a game gets it —
/// and a test that reads the file is a test that fails if the file stops being
/// there, which is the point of committing it (ADR-0042).
const REGULAR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/FiraSans-Regular.ttf"
);
/// The bold weight, which is a face of its own rather than a flag on one.
const BOLD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fonts/FiraSans-Bold.ttf"
);

/// What the draw system below draws.
struct Sample {
    /// In this style.
    style: TextStyle,
    /// This string.
    text: String,
}

impl Resource for Sample {}

fn draw_sample(ctx: &mut DrawCtx) {
    let sample = ctx.world.resource::<Sample>();
    let (at, text, style) = (Vec2::ZERO, sample.text.clone(), sample.style);
    ctx.text(at, &text, style);
}

/// Draw `text` in `style`, and hand back the quads with a backend they were
/// uploaded against.
fn draw(
    style: TextStyle,
    text: &str,
    fonts: &Fonts,
) -> (Vec<jidousha_core::Quad>, NullBackend, TextureTable) {
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Draw, draw_sample);
    });
    sim.world_mut().insert_resource(Sample {
        style,
        text: text.to_owned(),
    });
    sim.tick();
    let quads = sim.draw().quads().to_vec();
    let mut backend = NullBackend::new();
    let mut textures = create_builtin_textures(&mut backend);
    upload_text_atlases(fonts.faces(), &quads, &mut backend, &mut textures);
    (quads, backend, textures)
}

fn fira() -> (Fonts, Face) {
    let bytes = std::fs::read(REGULAR).expect("the committed regular weight is readable");
    let mut fonts = Fonts::new();
    let face = match fonts.try_create_face("Fira Sans", &bytes) {
        Ok(face) => face,
        Err(error) => panic!("{error}"),
    };
    (fonts, face)
}

#[test]
fn a_face_loads_from_the_family_this_repository_ships() {
    // Both weights, because a family with one is not a family — and because a
    // file that stopped being committed is exactly the failure this catches
    // (ADR-0042; `CREDITS.md` names them).
    let mut fonts = Fonts::new();
    for (name, path) in [("Fira Sans", REGULAR), ("Fira Sans Bold", BOLD)] {
        let bytes = std::fs::read(path).expect("committed and readable");
        match fonts.try_create_face(name, &bytes) {
            Ok(face) => assert_eq!(face.name(), name),
            Err(error) => panic!("{error}"),
        }
    }
    assert_eq!(fonts.faces().len(), 2);
    assert_ne!(
        fonts.faces()[0],
        fonts.faces()[1],
        "two faces, not one twice"
    );
}

#[test]
fn text_in_a_loaded_face_is_one_quad_per_character_on_that_faces_atlas() {
    let (fonts, face) = fira();
    let style = TextStyle {
        face,
        size: 24.0,
        ..TextStyle::default()
    };
    let (quads, _, _) = draw(style, "Hej, Wörld!", &fonts);
    assert_eq!(
        quads.len(),
        "Hej, Wörld!".chars().count(),
        "one quad per character, spaces included, exactly as the built-in font"
    );
    let atlas = face.atlas_texture(24.0);
    assert!(
        quads.iter().all(|quad| quad.texture == atlas),
        "and all of them on this face's atlas at this size"
    );
    assert_ne!(atlas, TextureId::WHITE);
    assert_ne!(
        atlas,
        jidousha_render_core::FONT_TEXTURE,
        "not the built-in font's"
    );
}

#[test]
fn a_loaded_faces_atlas_reaches_the_backend_through_the_ordinary_texture_path() {
    // The decision under test (ADR-0042 §1): no second pipeline and no second
    // upload path — an atlas is a texture the backend was handed, registered in
    // the same table a loaded sprite goes into.
    let (fonts, face) = fira();
    let style = TextStyle {
        face,
        size: 20.0,
        ..TextStyle::default()
    };
    let (_, backend, textures) = draw(style, "atlas", &fonts);
    let atlas = face.atlas_texture(20.0);
    assert!(textures.is_ready(atlas), "the atlas is registered");
    let Some((desc, texels)) = backend.uploaded(textures.resolve(atlas)) else {
        panic!("the atlas was uploaded");
    };
    assert_eq!(
        texels.len(),
        (desc.size.width * desc.size.height * 4) as usize,
        "RGBA8, row-major, like every other texture"
    );
    assert!(
        texels
            .chunks_exact(4)
            .all(|texel| texel[0..3] == [255, 255, 255]),
        "white with the shape in the alpha, so nothing dark bleeds into an edge"
    );
    assert!(
        texels.chunks_exact(4).any(|texel| texel[3] == 255),
        "and there is ink in it"
    );
}

#[test]
fn one_atlas_per_size_and_the_same_one_twice() {
    let (fonts, face) = fira();
    let mut sizes = Vec::new();
    for size in [12.0_f32, 12.0, 24.0, 48.0] {
        let style = TextStyle {
            face,
            size,
            ..TextStyle::default()
        };
        let (quads, _, _) = draw(style, "Aa", &fonts);
        sizes.push(quads[0].texture);
    }
    assert_eq!(sizes[0], sizes[1], "the same size is the same atlas");
    assert_ne!(sizes[0], sizes[2], "a different size is a different one");
    assert_ne!(sizes[2], sizes[3]);
}

#[test]
fn an_atlas_nobody_rasterized_draws_the_placeholder_rather_than_nothing() {
    // renderer.md §5's policy, reached by a route it did not have before: a
    // quad can name an atlas the frame has not built yet, because the id is
    // arithmetic rather than an allocation. The answer is the one every
    // not-yet-there texture gets.
    let (fonts, face) = fira();
    let style = TextStyle {
        face,
        size: 18.0,
        ..TextStyle::default()
    };
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Draw, draw_sample);
    });
    sim.world_mut().insert_resource(Sample {
        style,
        text: "unbuilt".to_owned(),
    });
    sim.tick();
    let quads = sim.draw().quads().to_vec();
    let mut backend = NullBackend::new();
    let textures = create_builtin_textures(&mut backend);
    // Deliberately not calling `upload_text_atlases`.
    assert_eq!(
        textures.resolve(quads[0].texture),
        textures.placeholder(),
        "loud, deterministic, and not a panic"
    );
    let _ = fonts;
}

#[test]
fn a_measured_line_is_the_line_the_glyphs_draw() {
    // The claim `measure` makes, checked against the quads rather than against
    // itself: a game centres a heading on this number and asserts the result is
    // inside a rectangle, and the two have to be the same measurement.
    let (fonts, face) = fira();
    let style = TextStyle {
        face,
        size: 24.0,
        ..TextStyle::default()
    };
    let line = "Measured";
    let (quads, _, _) = draw(style, line, &fonts);
    let (mut min, mut max) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    for quad in &quads {
        for corner in quad.corners {
            min = min.min(corner);
            max = max.max(corner);
        }
    }
    let bounds = jidousha_core::Rect::from_min_size(min, max - min);
    let measured = style.width_of(line);
    // The ink may lean a hair outside the advance box — that is what a
    // proportional face's side bearings are — so the check is that the two
    // agree to within one glyph's border rather than exactly.
    assert!(
        (bounds.size().x - measured).abs() < style.size * 0.25,
        "measured {measured} but drew {} wide",
        bounds.size().x
    );
    assert!(
        bounds.size().y <= style.size * 1.5,
        "and a line is about one line tall, not two"
    );
}

#[test]
fn a_proportional_face_measures_narrow_characters_narrowly() {
    // The reason the measurement API is part of this feature at all
    // (ADR-0042 §3): the built-in font's fixed advance is an assumption a
    // proportional face breaks, and a floor asserted with the old arithmetic
    // would be asserting about a number that no longer describes anything.
    let (_, face) = fira();
    let style = TextStyle {
        face,
        size: 20.0,
        ..TextStyle::default()
    };
    assert!(style.width_of("iiii") < style.width_of("WWWW"));
    // And the built-in font still does not, which is the half that must not
    // regress: every existing game measures with it.
    let built_in = TextStyle {
        size: 20.0,
        ..TextStyle::default()
    };
    assert_eq!(built_in.width_of("iiii"), built_in.width_of("WWWW"));
    assert_eq!(built_in.face, Face::BUILT_IN);
}

#[test]
fn the_column_count_never_overruns_and_the_line_fit_is_tight() {
    let (_, face) = fira();
    let style = TextStyle {
        face,
        size: 16.0,
        ..TextStyle::default()
    };
    for width in [0.0_f32, 5.0, 17.0, 60.0, 250.0] {
        // The pessimistic count holds for the worst string there is.
        let columns = style.columns_in(width);
        assert!(
            style.width_of(&"W".repeat(columns)) <= width,
            "{columns} of the widest character must fit in {width}"
        );
        // And the tight count is at least as generous for a real string.
        let line = "the quick brown fox jumps";
        let fitted = style.fits_in(line, width);
        assert!(fitted >= columns.min(line.chars().count()));
        let head: String = line.chars().take(fitted).collect();
        assert!(style.width_of(&head) <= width);
        if fitted < line.chars().count() {
            let more: String = line.chars().take(fitted + 1).collect();
            assert!(style.width_of(&more) > width, "and one more does not fit");
        }
    }
}

#[test]
fn every_latin_1_character_draws_and_a_stray_codepoint_draws_a_box() {
    // The promise ADR-0042 makes about coverage, and the failure mode it names:
    // a Latin-1 promise broken by a panic on a stray codepoint. Nothing here
    // may panic, and nothing may silently draw nothing.
    let (fonts, face) = fira();
    let style = TextStyle {
        face,
        size: 18.0,
        ..TextStyle::default()
    };
    let latin1: String = (0x20..=0x7E_u32)
        .chain(0xA0..=0xFF)
        .filter_map(char::from_u32)
        .collect();
    let (quads, _, _) = draw(style, &latin1, &fonts);
    assert_eq!(quads.len(), latin1.chars().count(), "all of them drew");

    // Outside Latin-1: still one quad each, and all of them the same cell.
    let strays = "\u{2603}\u{4e2d}\u{1F600}";
    let (boxes, _, _) = draw(style, strays, &fonts);
    assert_eq!(boxes.len(), strays.chars().count(), "drawn, not skipped");
    assert_eq!(boxes[0].uvs, boxes[1].uvs, "and all the same box");
    assert_eq!(boxes[1].uvs, boxes[2].uvs);
    let (real, _, _) = draw(style, "A", &fonts);
    assert_ne!(boxes[0].uvs, real[0].uvs, "which is not a letter");
}

#[test]
fn a_glyph_samples_inside_its_own_atlas() {
    let (fonts, face) = fira();
    let style = TextStyle {
        face,
        size: 30.0,
        ..TextStyle::default()
    };
    let (quads, _, _) = draw(style, "Aä!W ij", &fonts);
    for quad in &quads {
        for uv in quad.uvs {
            assert!(
                (0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y),
                "a UV outside the atlas samples whatever the driver clamps to: {uv:?}"
            );
        }
    }
}

#[test]
fn the_built_in_font_draws_exactly_what_it_drew_before() {
    // The other half of ADR-0042 §5's no-flag-day: every existing game measures
    // and draws with this face, so its numbers are frozen. Seven ninths of the
    // line, per character, one quad each, on `FONT_TEXTURE`.
    let mut fonts = Fonts::new();
    let style = TextStyle {
        size: 9.0,
        ..TextStyle::default()
    };
    let (quads, _, _) = draw(style, "abc", &fonts);
    assert_eq!(quads.len(), 3);
    assert_eq!(quads[0].corners[0], Vec2::ZERO);
    assert_eq!(quads[1].corners[0], Vec2::new(7.0, 0.0));
    assert_eq!(quads[2].corners[0], Vec2::new(14.0, 0.0));
    assert_eq!(quads[0].corners[2], Vec2::new(7.0, 9.0), "a cell is 7 by 9");
    assert!(
        quads
            .iter()
            .all(|quad| quad.texture == jidousha_render_core::FONT_TEXTURE)
    );
    assert_eq!(style.width_of("abc"), 21.0);
    assert_eq!(style.columns_in(21.0), 3);
    assert!(fonts.try_create_face("nope", b"not a font").is_err());
}
