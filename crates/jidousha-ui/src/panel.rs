//! A screen as data: every string and every icon it puts on the frame, with
//! its position — handed back first, drawn second.
//!
//! Key types: `Panel`, `TextRun`, `IconRun`, `Icon`, `Mapping`.
//! Depends on: `jidousha-core`, `jidousha-render-core`.
//! INVARIANT: a panel is read by three readers off one layout — the draw
//! system turns it into quads, `floors` judges what was meant, and
//! `judge_frame` finds it on the recorded frame. Nothing drawn through
//! [`Panel::draw`] is drawn past the panel, so the three cannot disagree about
//! what a screen contains.

use jidousha_core::{Color, DrawCtx, Rect, Transform, math::Vec2};
use jidousha_render_core::{Camera, Sprite, Submit, TextStyle};

use crate::text::wrap;

/// A game's own picture vocabulary, as the kit sees it: a role that knows how
/// big it is drawn at a scale.
///
/// The kit never names a texture. A game's icon type — an enum of roles, one
/// per slot in its art library — implements this, and `IconRun::bounds` and
/// the floors read it.
///
/// ```
/// use jidousha_core::math::Vec2;
/// use jidousha_ui::Icon;
///
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Art { Coin, Flame }
/// impl Icon for Art {
///     fn size_at(self, scale: f32) -> Vec2 {
///         // Every role here is a 16x16 picture.
///         Vec2::splat(16.0 * scale)
///     }
/// }
/// assert_eq!(Art::Coin.size_at(2.0), Vec2::splat(32.0));
/// ```
pub trait Icon: Copy {
    /// How big a quad drawing this at `scale` texels-per-texel is.
    fn size_at(self, scale: f32) -> Vec2;
}

/// One row of text a screen draws: where, what, and in which style.
///
/// The style carries the size, the colour, the face and the band
/// (`TextStyle::depth`), exactly as `ctx.text` takes it, so a row is drawn
/// with the style it was measured with.
///
/// ```
/// use jidousha_core::{Color, Depth, math::Vec2};
/// use jidousha_render_core::TextStyle;
/// use jidousha_ui::TextRun;
///
/// let style = TextStyle { size: 12.0, color: Color::WHITE, depth: Depth::layer(1), ..TextStyle::default() };
/// let row = TextRun::new(Vec2::new(10.0, 20.0), "idle 3", style);
/// assert_eq!(row.bounds().min, Vec2::new(10.0, 20.0));
/// assert_eq!(row.bounds().size().y, 12.0);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    /// The top-left of the first character's cell.
    pub at: Vec2,
    /// What it says. One run is one row: no line breaks.
    pub text: String,
    /// How it is drawn — size, colour, face and band.
    pub style: TextStyle,
}

impl TextRun {
    /// A row at `at`, in `style`.
    pub fn new(at: Vec2, text: impl Into<String>, style: TextStyle) -> Self {
        Self {
            at,
            text: text.into(),
            style,
        }
    }

    /// The rectangle this row's glyphs occupy, measured by its own style.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        Rect::from_min_size(
            self.at,
            Vec2::new(self.style.width_of(&self.text), self.style.size),
        )
    }
}

/// One icon a screen draws: where, which role, how big, in what tint, on
/// which band.
///
/// ```
/// use jidousha_core::{Color, math::Vec2};
/// use jidousha_ui::{Icon, IconRun};
///
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Art { Coin }
/// impl Icon for Art {
///     fn size_at(self, scale: f32) -> Vec2 { Vec2::splat(16.0 * scale) }
/// }
/// let icon = IconRun::new(Vec2::new(4.0, 4.0), Art::Coin, 2.0, -1);
/// assert_eq!(icon.bounds().size(), Vec2::splat(32.0));
/// assert_eq!(icon.tint, Color::WHITE);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconRun<I> {
    /// The top-left corner.
    pub at: Vec2,
    /// Which role.
    pub art: I,
    /// Texels per texel. The floors insist it is a whole number: the engine
    /// samples nearest, and a fractional scale puts a wobble in pixel art.
    pub scale: f32,
    /// Multiplied into the picture. White is untinted.
    pub tint: Color,
    /// Which band it is drawn on.
    pub layer: i16,
}

impl<I: Icon> IconRun<I> {
    /// An untinted icon at `at`, drawn at `scale` on `layer`.
    pub fn new(at: Vec2, art: I, scale: f32, layer: i16) -> Self {
        Self {
            at,
            art,
            scale,
            tint: Color::WHITE,
            layer,
        }
    }

    /// The rectangle it covers.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        Rect::from_min_size(self.at, self.art.size_at(self.scale))
    }
}

/// The one conversion between a screen's own units and the world the camera
/// is looking at — what [`Panel::draw`] places chrome through.
///
/// Chrome is laid out in a fixed design space (ninjo's is 960x540 reference
/// pixels) and rides the camera: the design rect is fitted inside whatever
/// the camera shows, so the chrome is a constant size on screen whatever the
/// zoom and the floors stay stated in design units. The game owns the fit;
/// the kit asks only where a design point lands and how many world units one
/// design unit is.
///
/// ```
/// use jidousha_core::math::Vec2;
/// use jidousha_ui::Mapping;
///
/// /// The design rect, placed at `origin` and scaled uniformly.
/// struct UiMap { origin: Vec2, scale: f32 }
/// impl Mapping for UiMap {
///     fn to_world(&self, ui: Vec2) -> Vec2 { self.origin + ui * self.scale }
///     fn scale(&self) -> f32 { self.scale }
/// }
/// let map = UiMap { origin: Vec2::new(100.0, 50.0), scale: 2.0 };
/// assert_eq!(map.to_world(Vec2::new(10.0, 10.0)), Vec2::new(120.0, 70.0));
/// ```
pub trait Mapping {
    /// A design-space point in the world.
    fn to_world(&self, ui: Vec2) -> Vec2;
    /// World units per design unit.
    fn scale(&self) -> f32;
}

/// Everything one screen puts on the frame, as data.
///
/// Two spaces, two lists each: `runs` and `icons` are the chrome, in design
/// units, placed through a [`Mapping`] at draw time; `world_runs` and
/// `world_icons` are in world units — map labels and anything else that pans
/// with the camera — and are culled to the camera's view when drawn.
///
/// ```
/// use jidousha_core::{Color, Depth, math::Vec2};
/// use jidousha_render_core::TextStyle;
/// use jidousha_ui::{Icon, IconRun, Panel, TextRun};
///
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Art { Coin }
/// impl Icon for Art {
///     fn size_at(self, scale: f32) -> Vec2 { Vec2::splat(16.0 * scale) }
/// }
/// let small = TextStyle { size: 12.0, color: Color::WHITE, depth: Depth::layer(1), ..TextStyle::default() };
///
/// let mut panel = Panel::default();
/// panel.icon(IconRun::new(Vec2::new(10.0, 10.0), Art::Coin, 1.0, -1));
/// panel.text(TextRun::new(Vec2::new(30.0, 11.0), "40g", small));
/// // A wrapped block is stored as the rows it is, one run per line.
/// let next_y = panel.block(Vec2::new(10.0, 40.0), "two\nrows", small, 2.0);
/// assert_eq!(panel.runs.len(), 3);
/// assert_eq!(next_y, 40.0 + 2.0 * (12.0 + 2.0));
/// assert_eq!(panel.all_strings().count(), 3);
/// ```
#[derive(Clone, Debug)]
pub struct Panel<I> {
    /// Every row of chrome text, in design units.
    pub runs: Vec<TextRun>,
    /// Every chrome icon, in design units.
    pub icons: Vec<IconRun<I>>,
    /// Every row of world-space text, in world units.
    pub world_runs: Vec<TextRun>,
    /// Every world-space icon, in world units.
    pub world_icons: Vec<IconRun<I>>,
}

impl<I> Default for Panel<I> {
    /// An empty screen — a meaningful default, and the one every screen
    /// function starts from.
    fn default() -> Self {
        Self {
            runs: Vec::new(),
            icons: Vec::new(),
            world_runs: Vec::new(),
            world_icons: Vec::new(),
        }
    }
}

impl<I: Icon> Panel<I> {
    /// Add a row of chrome text.
    pub fn text(&mut self, run: TextRun) -> &mut Self {
        self.runs.push(run);
        self
    }

    /// Add a chrome icon.
    pub fn icon(&mut self, run: IconRun<I>) -> &mut Self {
        self.icons.push(run);
        self
    }

    /// Add a row of world-space text.
    pub fn world_text(&mut self, run: TextRun) -> &mut Self {
        self.world_runs.push(run);
        self
    }

    /// Add a world-space icon.
    pub fn world_icon(&mut self, run: IconRun<I>) -> &mut Self {
        self.world_icons.push(run);
        self
    }

    /// Add a block of chrome text, one row per line, and say where the next
    /// row would go.
    ///
    /// Rows advance by `style.size + leading`. `ctx.text` honours `\n`, but a
    /// block submitted as one call is one row to every reader that counts
    /// glyphs on a row — so a block is stored as the rows it is, and the count
    /// stays exact.
    pub fn block(&mut self, at: Vec2, text: &str, style: TextStyle, leading: f32) -> f32 {
        let mut y = at.y;
        for line in text.lines() {
            self.runs
                .push(TextRun::new(Vec2::new(at.x, y), line, style));
            y += style.size + leading;
        }
        y
    }

    /// Add a band of prose wrapped to `width`, and say where the next row
    /// would go — the hint block every surface with a sentence to say ends
    /// in.
    ///
    /// The column count is the style's own measurement (`columns_in`), so no
    /// advance ratio appears in the game; a floor over the band's tallest
    /// state is the game's to write, because only the game knows which of its
    /// sentences is longest.
    pub fn hint(
        &mut self,
        at: Vec2,
        width: f32,
        text: &str,
        style: TextStyle,
        leading: f32,
    ) -> f32 {
        self.block(at, &wrap(text, style.columns_in(width)), style, leading)
    }

    /// Every string in this panel, both spaces — for a check that every
    /// string is one the font can draw.
    pub fn all_strings(&self) -> impl Iterator<Item = &str> {
        self.runs
            .iter()
            .chain(self.world_runs.iter())
            .map(|run| run.text.as_str())
    }

    /// Everything in `other`, appended.
    pub fn absorb(&mut self, other: Panel<I>) {
        self.runs.extend(other.runs);
        self.icons.extend(other.icons);
        self.world_runs.extend(other.world_runs);
        self.world_icons.extend(other.world_icons);
    }

    /// The same panel with every piece of chrome moved onto `layer` — how a
    /// drawer built out of ordinary rows is lifted onto the overlay's band at
    /// the end, in one place, rather than per row.
    ///
    /// World-space content is untouched: it is the map's, and a drawer lies
    /// over the map rather than in it.
    #[must_use]
    pub fn lifted(mut self, layer: i16) -> Self {
        for run in &mut self.runs {
            run.style.depth.layer = layer;
        }
        for icon in &mut self.icons {
            icon.layer = layer;
        }
        self
    }

    /// Draw the panel: world-space content as it is, culled to the camera;
    /// chrome through `map`, scaled with it.
    ///
    /// `sprite` turns one icon into the `Sprite` that draws it at the scale
    /// given — which is the icon's own scale for world-space icons and the
    /// icon's scale times the mapping's for chrome — because the kit never
    /// names a texture and a game's art library does. One transform, applied
    /// at the last moment, so every other reader of the layout reads it
    /// untransformed.
    pub fn draw(
        &self,
        ctx: &mut DrawCtx,
        map: &impl Mapping,
        sprite: impl Fn(&IconRun<I>, f32) -> Sprite,
    ) {
        // Map-space content is culled to the camera, like terrain: a label
        // panned off the screen submits nothing.
        let view = ctx.world.resource::<Camera>().visible_bounds();
        for icon in &self.world_icons {
            if !icon.bounds().overlaps(view) {
                continue;
            }
            ctx.sprite(&Transform::at(icon.at), &sprite(icon, icon.scale));
        }
        for run in &self.world_runs {
            if !run.bounds().overlaps(view) {
                continue;
            }
            ctx.text(run.at, &run.text, run.style);
        }
        for icon in &self.icons {
            ctx.sprite(
                &Transform::at(map.to_world(icon.at)),
                &sprite(icon, icon.scale * map.scale()),
            );
        }
        for run in &self.runs {
            ctx.text(
                map.to_world(run.at),
                &run.text,
                TextStyle {
                    size: run.style.size * map.scale(),
                    ..run.style
                },
            );
        }
    }
}
