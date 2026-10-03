//! The readability floors, as a list of breaches rather than as advice:
//! what a panel says it draws, judged against the numbers a game states.
//!
//! Key types: `Floors`, `Breach`. Key functions: `judge_panel`,
//! `judge_frame`, `frame_text_floor`, `glyph_run`, `inside`.
//! Depends on: `panel`, `jidousha-core`, `jidousha-render-core`.
//! INVARIANT: a floor's threshold is the game's (`Floors`) and never a number
//! written here. A floor passes silently and fails as a `Breach` naming the
//! numbers it judged, so a game's own accumulator can keep every breach and
//! print them together — an instrument that halts at the first bad reading
//! costs a cycle per fault.

use std::fmt::Debug;

use jidousha_core::{Rect, math::Vec2};
use jidousha_render_core::{BackendTextureId, FrameRecord, TextStyle};

use crate::panel::{Icon, IconRun, Mapping, Panel};

/// One floor, failed: the claim by name, and the numbers behind it.
///
/// `what` is a sentence stable enough to assert on — a check that stages a
/// broken screen asks "did *this* floor bite" by name rather than counting
/// problems that any other fault could satisfy.
///
/// ```
/// use jidousha_ui::Breach;
///
/// let breach = Breach { what: "two rows of chrome text overlap", detail: "x at .. and y at ..".to_owned() };
/// assert!(breach.what.starts_with("two rows"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breach {
    /// The claim that failed, by name.
    pub what: &'static str,
    /// The numbers it judged.
    pub detail: String,
}

/// The numbers a game's floors are stated against.
///
/// ```
/// use jidousha_core::{Rect, math::Vec2};
/// use jidousha_ui::Floors;
///
/// let floors = Floors {
///     min_text: 12.0,
///     chrome: Rect::from_min_size(Vec2::ZERO, Vec2::new(960.0, 540.0)),
///     world: Rect::from_min_size(Vec2::ZERO, Vec2::new(4000.0, 4000.0)),
/// };
/// assert_eq!(floors.chrome.size().x, 960.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Floors {
    /// No row of text smaller than this, in the units text is sized in.
    pub min_text: f32,
    /// The rectangle every piece of chrome lies inside — the design rect.
    pub chrome: Rect,
    /// The rectangle every world-space label lies inside — the map.
    pub world: Rect,
}

/// `a > b`, and false when either is NaN.
///
/// Spelled out rather than `!(a <= b)`: the negation of a float comparison
/// silently means something else, and a NaN that crept into a layout would
/// satisfy every plain `<=` and pass.
fn greater(a: f32, b: f32) -> bool {
    matches!(a.partial_cmp(&b), Some(std::cmp::Ordering::Greater))
}

/// Within a thousandth, and false when either is NaN.
fn near(a: f32, b: f32) -> bool {
    greater(0.001, (a - b).abs())
}

/// Whether `bounds` sits inside `area`, to within a hundredth of a unit.
///
/// ```
/// use jidousha_core::{Rect, math::Vec2};
/// use jidousha_ui::inside;
///
/// let screen = Rect::from_min_size(Vec2::ZERO, Vec2::new(960.0, 540.0));
/// assert!(inside(screen, Rect::from_min_size(Vec2::new(950.0, 0.0), Vec2::new(10.0, 10.0))));
/// assert!(!inside(screen, Rect::from_min_size(Vec2::new(955.0, 0.0), Vec2::new(10.0, 10.0))));
/// ```
#[must_use]
pub fn inside(area: Rect, bounds: Rect) -> bool {
    const SLACK: f32 = 0.01;
    bounds.min.x >= area.min.x - SLACK
        && bounds.min.y >= area.min.y - SLACK
        && bounds.max.x <= area.max.x + SLACK
        && bounds.max.y <= area.max.y + SLACK
}

/// One panel against the floors.
///
/// Every row of text clears `min_text`; every piece of chrome lies inside
/// the chrome rect and every map label inside the world; no row lies across
/// a `control` it is not the label of; no two rows of chrome on one band
/// collide, and no two map labels; every icon is drawn at a whole scale; and
/// at most one of `overlays` — `(label, title)` pairs, counted by the title
/// row each draws — is in the frame.
///
/// The last is the one-value floor: with a game's open overlay held as one
/// `Option` the state is unrepresentable, and the floor says it anyway, so
/// the next surface to grow an open-flag of its own fails here rather than
/// in a screenshot.
///
/// ```
/// use jidousha_core::{Color, Depth, Rect, math::Vec2};
/// use jidousha_render_core::TextStyle;
/// use jidousha_ui::{Floors, Icon, Panel, TextRun, judge_panel};
///
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Art {}
/// impl Icon for Art {
///     fn size_at(self, _scale: f32) -> Vec2 { match self {} }
/// }
/// let floors = Floors {
///     min_text: 12.0,
///     chrome: Rect::from_min_size(Vec2::ZERO, Vec2::new(960.0, 540.0)),
///     world: Rect::from_min_size(Vec2::ZERO, Vec2::new(4000.0, 4000.0)),
/// };
/// let tiny = TextStyle { size: 9.0, color: Color::WHITE, depth: Depth::layer(1), ..TextStyle::default() };
/// let mut panel: Panel<Art> = Panel::default();
/// panel.text(TextRun::new(Vec2::new(10.0, 10.0), "too small", tiny));
/// let breaches = judge_panel(&panel, &floors, &[], &[]);
/// assert_eq!(breaches.len(), 1);
/// assert_eq!(breaches[0].what, "a row of text is smaller than the readability floor allows");
/// ```
pub fn judge_panel<I: Icon + Debug>(
    panel: &Panel<I>,
    floors: &Floors,
    controls: &[(String, Rect)],
    overlays: &[(&str, &str)],
) -> Vec<Breach> {
    let mut out = Vec::new();
    for text in panel.runs.iter().chain(panel.world_runs.iter()) {
        if greater(floors.min_text, text.style.size) {
            out.push(Breach {
                what: "a row of text is smaller than the readability floor allows",
                detail: format!(
                    "{:?} is set at {:.1} and the floor is {:.0}",
                    text.text, text.style.size, floors.min_text
                ),
            });
        }
    }
    for text in &panel.runs {
        if !inside(floors.chrome, text.bounds()) {
            out.push(Breach {
                what: "a row of chrome text runs off the UI rect",
                detail: format!("{:?} occupies {:?}", text.text, text.bounds()),
            });
        }
        // Nothing lies across a control it is not the label of.
        for (control, target) in controls {
            if text.bounds().overlaps(*target) && !inside(*target, text.bounds()) {
                out.push(Breach {
                    what: "a row of text lies across a control it is not the label of",
                    detail: format!(
                        "{:?} at {:?} crosses {control} at {target:?}",
                        text.text,
                        text.bounds()
                    ),
                });
            }
        }
    }
    for text in &panel.world_runs {
        if !inside(floors.world, text.bounds()) {
            out.push(Breach {
                what: "a map label runs off the world",
                detail: format!("{:?} occupies {:?}", text.text, text.bounds()),
            });
        }
    }
    // No two rows of chrome on one band collide. **On one band**, because the
    // layers are what make an overlay legitimate: a band drawn over a
    // drawer's footer with its own ground behind it is deliberate, while two
    // rows on the same band are two rows drawn through each other.
    let mut chrome: Vec<&crate::panel::TextRun> = panel.runs.iter().collect();
    chrome.sort_by_key(|run| run.style.depth.layer);
    for (index, text) in chrome.iter().enumerate() {
        for other in chrome.iter().skip(index + 1) {
            if other.style.depth.layer != text.style.depth.layer {
                continue;
            }
            if text.bounds().overlaps(other.bounds()) {
                out.push(Breach {
                    what: "two rows of chrome text overlap",
                    detail: format!(
                        "{:?} at {:?} and {:?} at {:?}, both on band {}",
                        text.text,
                        text.bounds(),
                        other.text,
                        other.bounds(),
                        text.style.depth.layer
                    ),
                });
            }
        }
    }
    // At most one overlay's content in the frame, counted by the title row
    // each one draws — the one string the overlay prints and this reads, so
    // the two cannot drift apart into a floor that sees nothing.
    let drawing: Vec<&str> = overlays
        .iter()
        .filter(|(_, title)| panel.runs.iter().any(|run| run.text == *title))
        .map(|(label, _)| *label)
        .collect();
    if drawing.len() > 1 {
        out.push(Breach {
            what: "two overlays' content is in one frame",
            detail: format!("{drawing:?} are all drawing, and an overlay covers the screen"),
        });
    }
    for (index, text) in panel.world_runs.iter().enumerate() {
        for other in panel.world_runs.iter().skip(index + 1) {
            if text.bounds().overlaps(other.bounds()) {
                out.push(Breach {
                    what: "two map labels overlap",
                    detail: format!(
                        "{:?} at {:?} and {:?} at {:?}",
                        text.text,
                        text.bounds(),
                        other.text,
                        other.bounds()
                    ),
                });
            }
        }
    }
    for icon in panel.icons.iter().chain(panel.world_icons.iter()) {
        if !near(icon.scale, icon.scale.round()) {
            out.push(Breach {
                what: "a pixel-art icon is drawn at a fractional scale",
                detail: format!(
                    "{:?} is drawn at {:.2}x, and the engine samples nearest - a fraction puts \
                     a wobble in it",
                    icon.art, icon.scale
                ),
            });
        }
    }
    for icon in &panel.icons {
        if !inside(floors.chrome, icon.bounds()) {
            out.push(Breach {
                what: "a chrome icon runs off the UI rect",
                detail: format!("{:?} occupies {:?}", icon.art, icon.bounds()),
            });
        }
    }
    out
}

/// How many of a row's glyphs were drawn, counted inside the row's own
/// world-space box: `at` and `size` as the row was drawn, `width` as its
/// style measures it.
///
/// ```
/// # use jidousha_core::{Draw, DrawCtx, GameConfig, PhysicalSize, headless, math::Vec2};
/// # use jidousha_render_core::{FrameRecorder, Submit, TextStyle};
/// # use jidousha_ui::glyph_run;
/// # fn draw_it(ctx: &mut DrawCtx) {
/// #     ctx.text(Vec2::new(10.0, 10.0), "hello", TextStyle { size: 12.0, ..TextStyle::default() });
/// # }
/// # let mut sim = headless(GameConfig::default(), |app| { app.add_system(Draw, draw_it); });
/// # let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));
/// # let frame = recorder.draw(&mut sim);
/// let style = TextStyle { size: 12.0, ..TextStyle::default() };
/// let drawn = glyph_run(&frame, recorder.font_texture(), Vec2::new(10.0, 10.0), 12.0, style.width_of("hello"));
/// assert_eq!(drawn, 5);
/// ```
#[must_use]
pub fn glyph_run(
    frame: &FrameRecord,
    font: BackendTextureId,
    at: Vec2,
    size: f32,
    width: f32,
) -> usize {
    frame
        .quads()
        .iter()
        .filter(|quad| {
            quad.texture == font
                && near(quad.bounds().min.y, at.y)
                && quad.bounds().min.x >= at.x - 0.5
                && quad.bounds().max.x <= at.x + width + 0.5 + size * 0.01
        })
        .count()
}

/// Every row and icon of a panel, found on the frame it was drawn into —
/// the other direction from `judge_panel`: not "is what it says legible"
/// but "is what it says what the frame shows".
///
/// Chrome is looked for through the same `map` the frame was drawn with;
/// world-space rows are looked for where they are, and only the ones inside
/// `view` owe the frame their glyphs, because the draw culls the rest.
///
/// ```
/// # use jidousha_core::{Color, Depth, Draw, DrawCtx, GameConfig, PhysicalSize, Rect, headless, math::Vec2};
/// # use jidousha_render_core::{Camera, FrameRecorder, Sprite, Submit, TextStyle};
/// # use jidousha_ui::{Icon, IconRun, Mapping, Panel, TextRun, judge_frame};
/// # #[derive(Clone, Copy, Debug, PartialEq)]
/// # enum Art {}
/// # impl Icon for Art { fn size_at(self, _scale: f32) -> Vec2 { match self {} } }
/// # struct Flat;
/// # impl Mapping for Flat {
/// #     fn to_world(&self, ui: Vec2) -> Vec2 { ui }
/// #     fn scale(&self) -> f32 { 1.0 }
/// # }
/// # fn screen() -> Panel<Art> {
/// #     let small = TextStyle { size: 12.0, color: Color::WHITE, depth: Depth::layer(1), ..TextStyle::default() };
/// #     let mut panel = Panel::default();
/// #     panel.text(TextRun::new(Vec2::new(10.0, 10.0), "idle 3", small));
/// #     panel
/// # }
/// # fn draw_it(ctx: &mut DrawCtx) {
/// #     screen().draw(ctx, &Flat, |_icon, _scale| unreachable!("no icons here"));
/// # }
/// # let mut sim = headless(GameConfig::default(), |app| { app.add_system(Draw, draw_it); });
/// # sim.world_mut().insert_resource(Camera { center: Vec2::new(480.0, 270.0), height: 540.0, ..Camera::default() });
/// # let mut recorder = FrameRecorder::new(PhysicalSize::new(960, 540));
/// # let frame = recorder.draw(&mut sim);
/// let view = Rect::from_center_size(Vec2::new(480.0, 270.0), Vec2::new(960.0, 540.0));
/// let breaches = judge_frame(&screen(), &frame, recorder.font_texture(), &Flat, view);
/// assert!(breaches.is_empty(), "{breaches:?}");
/// ```
pub fn judge_frame<I: Icon + Debug>(
    panel: &Panel<I>,
    frame: &FrameRecord,
    font: BackendTextureId,
    map: &impl Mapping,
    view: Rect,
) -> Vec<Breach> {
    let mut out = Vec::new();
    for run in &panel.runs {
        let at = map.to_world(run.at);
        let style = TextStyle {
            size: run.style.size * map.scale(),
            ..run.style
        };
        let drawn = glyph_run(frame, font, at, style.size, style.width_of(&run.text));
        if drawn != run.text.chars().count() {
            out.push(Breach {
                what: "a row of the chrome is not drawn as the string it is",
                detail: format!(
                    "{:?} at ({:.1}, {:.1}) is {} characters and {drawn} glyphs landed in its box",
                    run.text,
                    at.x,
                    at.y,
                    run.text.chars().count(),
                ),
            });
        }
    }
    for run in &panel.world_runs {
        if !run.bounds().overlaps(view) {
            continue;
        }
        let drawn = glyph_run(
            frame,
            font,
            run.at,
            run.style.size,
            run.style.width_of(&run.text),
        );
        if drawn != run.text.chars().count() {
            out.push(Breach {
                what: "a map label is not drawn as the string it is",
                detail: format!(
                    "{:?} at ({:.1}, {:.1}) is {} characters and {drawn} glyphs landed in its box",
                    run.text,
                    run.at.x,
                    run.at.y,
                    run.text.chars().count(),
                ),
            });
        }
    }
    for icon in &panel.icons {
        judge_icon(&mut out, frame, font, icon, map.to_world(icon.at));
    }
    for icon in &panel.world_icons {
        if !icon.bounds().overlaps(view) {
            continue;
        }
        judge_icon(&mut out, frame, font, icon, icon.at);
    }
    out
}

fn judge_icon<I: Icon + Debug>(
    out: &mut Vec<Breach>,
    frame: &FrameRecord,
    font: BackendTextureId,
    icon: &IconRun<I>,
    at: Vec2,
) {
    let covered = frame.quads().iter().any(|quad| {
        quad.texture != font && near(quad.bounds().min.x, at.x) && near(quad.bounds().min.y, at.y)
    });
    if !covered {
        out.push(Breach {
            what: "an icon the screen says it draws is not on the frame",
            detail: format!("{:?} at ({:.1}, {:.1}) has no quad", icon.art, at.x, at.y),
        });
    }
}

/// The frame floor: no glyph drawn below `min_text`, on a frame where one
/// world unit is one of the units text is sized in.
///
/// ```
/// # use jidousha_core::{Draw, DrawCtx, GameConfig, PhysicalSize, headless, math::Vec2};
/// # use jidousha_render_core::{FrameRecorder, Submit, TextStyle};
/// # use jidousha_ui::frame_text_floor;
/// # fn draw_it(ctx: &mut DrawCtx) {
/// #     ctx.text(Vec2::new(10.0, 10.0), "tiny", TextStyle { size: 9.0, ..TextStyle::default() });
/// # }
/// # let mut sim = headless(GameConfig::default(), |app| { app.add_system(Draw, draw_it); });
/// # let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));
/// # let frame = recorder.draw(&mut sim);
/// let breaches = frame_text_floor(&frame, recorder.font_texture(), 12.0);
/// assert_eq!(breaches.len(), 1);
/// assert_eq!(breaches[0].what, "a glyph was drawn below the readability floor");
/// ```
pub fn frame_text_floor(frame: &FrameRecord, font: BackendTextureId, min_text: f32) -> Vec<Breach> {
    let smallest = frame
        .quads()
        .iter()
        .filter(|quad| quad.texture == font)
        .map(|quad| quad.bounds().size().y)
        .fold(f32::MAX, f32::min);
    if smallest == f32::MAX || !greater(min_text - 0.01, smallest) {
        return Vec::new();
    }
    vec![Breach {
        what: "a glyph was drawn below the readability floor",
        detail: format!("the shortest glyph quad is {smallest:.2} and the floor is {min_text:.0}"),
    }]
}
