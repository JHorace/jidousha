//! Every floor, broken on purpose, and seen to bite from the kit — one staged
//! violation per floor, asked for by name (agent-practices §5.2).
//!
//! Key functions: `screen`, `draw_through`.
//! Depends on: `jidousha-ui`, `jidousha-core`, `jidousha-render-core`,
//! `jidousha-assets` (a handle for the icon test).
//! INVARIANT: a floor is asserted by its `what` string, never by a count a
//! different fault could satisfy.

use jidousha_assets::{Assets, MemorySource, TextureData};
use jidousha_core::{
    Color, Depth, Draw, DrawCtx, GameConfig, PhysicalSize, Rect, headless, math::Vec2,
};
use jidousha_render_core::{Camera, FrameRecord, FrameRecorder, Sprite, TextStyle};
use jidousha_ui::{
    Floors, Icon, IconRun, Mapping, Panel, TextRun, frame_text_floor, judge_frame, judge_panel,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Art {
    Coin,
}

impl Icon for Art {
    fn size_at(self, scale: f32) -> Vec2 {
        Vec2::splat(16.0 * scale)
    }
}

const DESIGN: Vec2 = Vec2::new(960.0, 540.0);

fn floors() -> Floors {
    Floors {
        min_text: 12.0,
        chrome: Rect::from_min_size(Vec2::ZERO, DESIGN),
        world: Rect::from_min_size(Vec2::ZERO, Vec2::new(2000.0, 2000.0)),
    }
}

fn style(size: f32, layer: i16) -> TextStyle {
    TextStyle {
        size,
        color: Color::WHITE,
        depth: Depth::layer(layer),
        ..TextStyle::default()
    }
}

fn names(breaches: &[jidousha_ui::Breach]) -> Vec<&'static str> {
    breaches.iter().map(|breach| breach.what).collect()
}

#[test]
fn a_clean_panel_breaches_no_floor() {
    let mut panel: Panel<Art> = Panel::default();
    panel.text(TextRun::new(
        Vec2::new(10.0, 10.0),
        "idle 3",
        style(12.0, 1),
    ));
    panel.text(TextRun::new(
        Vec2::new(10.0, 30.0),
        "away 1",
        style(12.0, 1),
    ));
    panel.icon(IconRun::new(Vec2::new(200.0, 10.0), Art::Coin, 2.0, -1));
    panel.world_text(TextRun::new(
        Vec2::new(500.0, 500.0),
        "the Deep Cave",
        style(12.0, -5),
    ));
    let judged = judge_panel(&panel, &floors(), &[], &[]);
    assert!(judged.is_empty(), "{judged:?}");
}

#[test]
fn text_below_the_floor_bites() {
    let mut panel: Panel<Art> = Panel::default();
    panel.text(TextRun::new(Vec2::new(10.0, 10.0), "tiny", style(11.9, 1)));
    assert_eq!(
        names(&judge_panel(&panel, &floors(), &[], &[])),
        ["a row of text is smaller than the readability floor allows"]
    );
}

#[test]
fn chrome_off_the_ui_rect_bites() {
    let mut panel: Panel<Art> = Panel::default();
    panel.text(TextRun::new(
        Vec2::new(950.0, 10.0),
        "off the edge",
        style(12.0, 1),
    ));
    assert_eq!(
        names(&judge_panel(&panel, &floors(), &[], &[])),
        ["a row of chrome text runs off the UI rect"]
    );
}

#[test]
fn text_across_a_control_it_does_not_label_bites() {
    let button = Rect::from_min_size(Vec2::new(100.0, 10.0), Vec2::new(48.0, 32.0));
    let controls = vec![("the close".to_owned(), button)];
    let mut panel: Panel<Art> = Panel::default();
    // Starts left of the button and runs into it: not its label.
    panel.text(TextRun::new(
        Vec2::new(60.0, 20.0),
        "a long row of words",
        style(12.0, 1),
    ));
    assert_eq!(
        names(&judge_panel(&panel, &floors(), &controls, &[])),
        ["a row of text lies across a control it is not the label of"]
    );
    // The label itself, inside the button, is fine.
    let mut labelled: Panel<Art> = Panel::default();
    labelled.text(TextRun::new(Vec2::new(110.0, 20.0), "X", style(12.0, 1)));
    assert!(judge_panel(&labelled, &floors(), &controls, &[]).is_empty());
}

#[test]
fn two_rows_of_chrome_on_one_band_bite_and_on_two_bands_do_not() {
    let mut same: Panel<Art> = Panel::default();
    same.text(TextRun::new(
        Vec2::new(10.0, 10.0),
        "seed 0",
        style(12.0, 1),
    ));
    same.text(TextRun::new(
        Vec2::new(14.0, 14.0),
        "point at a constant",
        style(12.0, 1),
    ));
    assert_eq!(
        names(&judge_panel(&same, &floors(), &[], &[])),
        ["two rows of chrome text overlap"]
    );
    // The breakdown band over a drawer's footer: deliberate, and on its own band.
    let mut lifted: Panel<Art> = Panel::default();
    lifted.text(TextRun::new(
        Vec2::new(10.0, 10.0),
        "seed 0",
        style(12.0, 1),
    ));
    lifted.text(TextRun::new(
        Vec2::new(14.0, 14.0),
        "point at a constant",
        style(12.0, 6),
    ));
    assert!(judge_panel(&lifted, &floors(), &[], &[]).is_empty());
}

#[test]
fn two_overlays_in_one_frame_bite() {
    let overlays = [
        ("ROSTER", "ROSTER - everyone"),
        ("TUNE", "TUNE - the constants"),
    ];
    let mut both: Panel<Art> = Panel::default();
    both.text(TextRun::new(
        Vec2::new(10.0, 10.0),
        "ROSTER - everyone",
        style(12.0, 6),
    ));
    both.text(TextRun::new(
        Vec2::new(10.0, 100.0),
        "TUNE - the constants",
        style(12.0, 6),
    ));
    assert_eq!(
        names(&judge_panel(&both, &floors(), &[], &overlays)),
        ["two overlays' content is in one frame"]
    );
    let mut one: Panel<Art> = Panel::default();
    one.text(TextRun::new(
        Vec2::new(10.0, 10.0),
        "ROSTER - everyone",
        style(12.0, 6),
    ));
    assert!(judge_panel(&one, &floors(), &[], &overlays).is_empty());
}

#[test]
fn a_map_label_off_the_world_bites() {
    let mut panel: Panel<Art> = Panel::default();
    panel.world_text(TextRun::new(
        Vec2::new(1990.0, 10.0),
        "beyond",
        style(12.0, -5),
    ));
    assert_eq!(
        names(&judge_panel(&panel, &floors(), &[], &[])),
        ["a map label runs off the world"]
    );
}

#[test]
fn two_map_labels_that_collide_bite() {
    let mut panel: Panel<Art> = Panel::default();
    panel.world_text(TextRun::new(
        Vec2::new(100.0, 100.0),
        "the Deep Cave",
        style(12.0, -5),
    ));
    panel.world_text(TextRun::new(
        Vec2::new(104.0, 104.0),
        "the Old Crypt",
        style(12.0, -5),
    ));
    assert_eq!(
        names(&judge_panel(&panel, &floors(), &[], &[])),
        ["two map labels overlap"]
    );
}

#[test]
fn a_fractional_icon_scale_bites() {
    let mut panel: Panel<Art> = Panel::default();
    panel.icon(IconRun::new(Vec2::new(10.0, 10.0), Art::Coin, 1.5, -1));
    assert_eq!(
        names(&judge_panel(&panel, &floors(), &[], &[])),
        ["a pixel-art icon is drawn at a fractional scale"]
    );
}

#[test]
fn a_chrome_icon_off_the_ui_rect_bites() {
    let mut panel: Panel<Art> = Panel::default();
    panel.icon(IconRun::new(Vec2::new(950.0, 10.0), Art::Coin, 1.0, -1));
    assert_eq!(
        names(&judge_panel(&panel, &floors(), &[], &[])),
        ["a chrome icon runs off the UI rect"]
    );
}

// --- the frame half ----------------------------------------------------------

/// The design rect placed at `origin`, `scale` world units per design unit.
struct Map {
    origin: Vec2,
    scale: f32,
}

impl Mapping for Map {
    fn to_world(&self, ui: Vec2) -> Vec2 {
        self.origin + ui * self.scale
    }
    fn scale(&self) -> f32 {
        self.scale
    }
}

fn screen() -> Panel<Art> {
    let mut panel = Panel::default();
    panel.text(TextRun::new(
        Vec2::new(10.0, 10.0),
        "idle 3",
        style(12.0, 1),
    ));
    panel.icon(IconRun::new(Vec2::new(200.0, 10.0), Art::Coin, 2.0, -1));
    panel.world_text(TextRun::new(
        Vec2::new(500.0, 500.0),
        "the Deep Cave",
        style(12.0, -5),
    ));
    panel
}

const MAP: Map = Map {
    origin: Vec2::new(100.0, 50.0),
    scale: 2.0,
};

fn draw_screen(ctx: &mut DrawCtx) {
    let handle = ctx.world.resource::<Handle>().0;
    screen().draw(ctx, &MAP, |icon, scale| Sprite {
        size: icon.art.size_at(scale),
        anchor: Vec2::new(-0.5, -0.5),
        tint: icon.tint,
        layer: icon.layer,
        ..Sprite::new(handle)
    });
}

#[derive(Clone, Copy)]
struct Handle(jidousha_assets::TextureHandle);
impl jidousha_core::Resource for Handle {}

/// One frame of `screen`, drawn through `MAP` at a camera that shows all of it.
fn draw_through() -> (FrameRecorder, FrameRecord, Rect) {
    let mut source = MemorySource::new();
    source.insert_texture(
        "coin.png",
        TextureData {
            width: 1,
            height: 1,
            rgba: vec![255; 4],
        },
    );
    let mut assets = Assets::new(source);
    let handle = Handle(assets.load_texture("coin.png"));
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Draw, draw_screen);
    });
    sim.world_mut().insert_resource(handle);
    let camera = Camera {
        center: Vec2::new(1000.0, 600.0),
        height: 2000.0,
        ..Camera::default()
    };
    sim.world_mut().insert_resource(camera);
    let mut recorder = FrameRecorder::new(PhysicalSize::new(1000, 1000));
    let frame = recorder.draw(&mut sim);
    let view = Camera {
        viewport: PhysicalSize::new(1000, 1000),
        ..camera
    }
    .visible_bounds();
    (recorder, frame, view)
}

#[test]
fn a_panel_drawn_through_a_mapping_is_found_on_the_frame_where_it_said() {
    let (recorder, frame, view) = draw_through();
    let judged = judge_frame(&screen(), &frame, recorder.font_texture(), &MAP, view);
    assert!(judged.is_empty(), "{judged:?}");
}

#[test]
fn a_row_the_frame_does_not_carry_bites() {
    let (recorder, frame, view) = draw_through();
    let mut claimed = screen();
    claimed.text(TextRun::new(
        Vec2::new(10.0, 40.0),
        "never drawn",
        style(12.0, 1),
    ));
    claimed.world_text(TextRun::new(
        Vec2::new(600.0, 600.0),
        "nor this",
        style(12.0, -5),
    ));
    claimed.icon(IconRun::new(Vec2::new(300.0, 10.0), Art::Coin, 2.0, -1));
    let judged = judge_frame(&claimed, &frame, recorder.font_texture(), &MAP, view);
    assert_eq!(
        names(&judged),
        [
            "a row of the chrome is not drawn as the string it is",
            "a map label is not drawn as the string it is",
            "an icon the screen says it draws is not on the frame",
        ]
    );
}

#[test]
fn a_map_label_outside_the_view_owes_the_frame_nothing() {
    let (recorder, frame, view) = draw_through();
    let mut claimed = screen();
    claimed.world_text(TextRun::new(
        Vec2::new(-900.0, -900.0),
        "panned off",
        style(12.0, -5),
    ));
    assert!(judge_frame(&claimed, &frame, recorder.font_texture(), &MAP, view).is_empty());
}

#[test]
fn a_glyph_below_the_floor_on_the_frame_bites() {
    let (recorder, frame, _) = draw_through();
    // Chrome at 12 through a mapping of 2 is 24 on the frame; the map label
    // is 12. Nothing is below twelve, and everything is below twenty-five.
    assert!(frame_text_floor(&frame, recorder.font_texture(), 12.0).is_empty());
    assert_eq!(
        names(&frame_text_floor(&frame, recorder.font_texture(), 25.0)),
        ["a glyph was drawn below the readability floor"]
    );
}
