//! The vocabulary a Draw system submits in: colors, rectangles, depth, and the
//! quad everything drawn is made of.
//!
//! Key types: `Color`, `Rect`, `Depth`, `TextureId`, `Quad`.
//! Depends on: `math`. Must never depend on: anything outside this crate.
//! INVARIANT: plain data, no rendering cleverness. Expansion (a sprite, a
//! circle, a line of text into quads), sorting, and batching all live in
//! `jidousha-render-core`, which keeps backends dumb and this crate free of
//! renderer machinery (renderer.md §1).
//!
//! DELIBERATE: a rendering-shaped type in the ECS crate needs a word. `DrawCtx`
//! lives here (ADR-0008), so the sink it writes into lives here too, and the
//! sink has to speak *some* vocabulary. That vocabulary cannot name a texture
//! asset — core depends on no other jidousha crate (core.md §1, CONTRACT) — so
//! it names an opaque [`TextureId`] instead, and the mapping from ids to real
//! textures belongs to the crates that have both. See ADR-0015.

use core::fmt;

use crate::math::Vec2;

/// A color: linear-looking sRGB components, 0.0 to 1.0, straight alpha.
///
/// What an agent or a human means by "half grey" is `rgb(0.5, 0.5, 0.5)`.
/// Linearization happens inside the render backend, where it belongs
/// (conventions).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// Red, 0.0 to 1.0.
    pub r: f32,
    /// Green, 0.0 to 1.0.
    pub g: f32,
    /// Blue, 0.0 to 1.0.
    pub b: f32,
    /// Alpha, 0.0 transparent to 1.0 opaque.
    pub a: f32,
}

impl Color {
    /// An opaque color.
    #[must_use]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// A color with alpha.
    #[must_use]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Opaque white — the tint that changes nothing.
    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);
    /// Opaque black.
    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
    /// Opaque red.
    pub const RED: Color = Color::rgb(1.0, 0.0, 0.0);
    /// Opaque green.
    pub const GREEN: Color = Color::rgb(0.0, 1.0, 0.0);
    /// Opaque blue.
    pub const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);
    /// Opaque magenta — the placeholder's color, and the engine's "look here".
    pub const MAGENTA: Color = Color::rgb(1.0, 0.0, 1.0);
    /// Fully transparent.
    pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

    /// This color multiplied by another, component-wise — how tinting works.
    #[must_use]
    pub fn modulate(self, other: Color) -> Color {
        Color {
            r: self.r * other.r,
            g: self.g * other.g,
            b: self.b * other.b,
            a: self.a * other.a,
        }
    }
}

impl Default for Color {
    /// White: the color that tints nothing and shows a texture as it is.
    ///
    /// DELIBERATE: this is a `Default` that means something (ADR-0012) — a
    /// sprite with no tint stated should look like its art.
    fn default() -> Self {
        Color::WHITE
    }
}

/// An axis-aligned rectangle, in whatever space its user is working in.
///
/// Y is down (ADR-0010), so `min` is the top-left corner and `max` the
/// bottom-right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Top-left.
    pub min: Vec2,
    /// Bottom-right.
    pub max: Vec2,
}

impl Rect {
    /// The rectangle covering `size` from its top-left corner.
    #[must_use]
    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Self {
            min,
            max: min + size,
        }
    }

    /// The rectangle of `size` centered on `center`.
    #[must_use]
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        let half = size * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// The whole of something, in normalized coordinates: (0,0) to (1,1).
    pub const UNIT: Rect = Rect {
        min: Vec2::ZERO,
        max: Vec2::ONE,
    };

    /// Width and height.
    #[must_use]
    pub fn size(self) -> Vec2 {
        self.max - self.min
    }

    /// The point in the middle.
    #[must_use]
    pub fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// Whether `point` is inside, counting the top-left edges and not the
    /// bottom-right ones — so adjacent rectangles never both claim a point.
    #[must_use]
    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x < self.max.x
            && point.y >= self.min.y
            && point.y < self.max.y
    }

    /// Whether any part of `other` is inside this rectangle.
    #[must_use]
    pub fn overlaps(self, other: Rect) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }
}

/// Where something sits in the draw order.
///
/// `layer` is the coarse band — background, world, UI — and `z` orders within
/// it. Higher `z` draws on top. CONTRACT: `z` is a draw-order key, not a
/// position on the spatial +Z axis, which points into the screen (ADR-0010).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Depth {
    /// The coarse band. Higher layers draw over lower ones.
    pub layer: i16,
    /// The fine order within a layer. Higher draws on top.
    pub z: f32,
}

impl Depth {
    /// The front of `layer`'s band.
    #[must_use]
    pub const fn layer(layer: i16) -> Self {
        Self { layer, z: 0.0 }
    }
}

impl Default for Depth {
    /// Layer 0, z 0 — the middle of everything, which is where a prototype
    /// wants its first sprite.
    ///
    /// DELIBERATE: a meaningful `Default` (ADR-0012), not an alias for a
    /// constructor.
    fn default() -> Self {
        Self { layer: 0, z: 0.0 }
    }
}

/// Which texture a quad samples.
///
/// Opaque, and deliberately meaningless here: this crate knows nothing about
/// textures, files, or pixels. `jidousha-assets` mints these from its handles
/// and `jidousha-render-core` maps them to whatever the backend uploaded. An id
/// nobody registered draws the placeholder (renderer.md §5), which is why this
/// carries no "is it valid" question — every id is drawable.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureId(u64);

impl TextureId {
    /// The untextured id: a flat white 1×1, for shapes that carry only a color.
    ///
    /// Reserved, and never minted from an asset handle.
    pub const WHITE: TextureId = TextureId(0);

    /// The id for a raw value. Called by the crates that own the mapping.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// The raw value.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }
}

impl fmt::Debug for TextureId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TextureId::WHITE => formatter.write_str("TextureId(white)"),
            TextureId(bits) => write!(formatter, "TextureId({bits})"),
        }
    }
}

/// One textured, tinted quadrilateral in world space: everything the engine
/// draws, after expansion.
///
/// Sprites, rectangles, lines, circles, and glyphs all arrive here — one
/// vertex format and one pipeline, which is what keeps the backend seam narrow
/// (renderer.md §7).
///
/// INVARIANT: `corners` and `uvs` are in the same order, and that order winds
/// consistently — top-left, top-right, bottom-right, bottom-left, in the
/// quad's own frame before rotation. Two triangles are cut from it the same way
/// every time, so identical submissions produce identical vertices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    /// The four corners, in world space, already rotated and scaled.
    pub corners: [Vec2; 4],
    /// Where each corner samples the texture, normalized 0..1.
    pub uvs: [Vec2; 4],
    /// Multiplied into whatever the texture gives.
    pub tint: Color,
    /// What to sample.
    pub texture: TextureId,
    /// Where in the draw order.
    pub depth: Depth,
}

/// The size of a surface or a texture, in physical pixels.
///
/// Physical, not logical: DPI scaling is the platform's business, and the
/// renderer works in the pixels it is actually given.
///
/// Here rather than in the renderer because three crates need it and one of
/// them is this one: `GameConfig::window_size` asks for a window this big, the
/// renderer's `Camera` records how big the surface turned out, and the platform
/// crate measures it. That is ADR-0015's rule applied to pixels — vocabulary
/// that has to cross the seam lives on the near side of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalSize {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl PhysicalSize {
    /// A size in pixels.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Width divided by height, or 1.0 for a degenerate surface.
    ///
    /// A minimized window reports zero height, and a camera that divided by it
    /// would put NaN into every vertex of the frame.
    #[must_use]
    pub fn aspect(self) -> f32 {
        if self.width == 0 || self.height == 0 {
            return 1.0;
        }
        self.width as f32 / self.height as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tint_of_white_changes_nothing() {
        let color = Color::rgba(0.25, 0.5, 0.75, 0.5);
        assert_eq!(color.modulate(Color::WHITE), color);
    }

    #[test]
    fn rectangles_agree_about_where_they_are() {
        let from_size = Rect::from_min_size(Vec2::new(1.0, 2.0), Vec2::new(4.0, 6.0));
        let from_center = Rect::from_center_size(Vec2::new(3.0, 5.0), Vec2::new(4.0, 6.0));
        assert_eq!(from_size, from_center);
        assert_eq!(from_size.size(), Vec2::new(4.0, 6.0));
        assert_eq!(from_size.center(), Vec2::new(3.0, 5.0));
    }

    #[test]
    fn adjacent_rectangles_never_both_claim_a_point() {
        // Half-open on the bottom-right, so a grid of them tiles without
        // overlap — the property a hit test depends on.
        let left = Rect::from_min_size(Vec2::ZERO, Vec2::new(1.0, 1.0));
        let right = Rect::from_min_size(Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0));
        let shared = Vec2::new(1.0, 0.5);
        assert!(!left.contains(shared));
        assert!(right.contains(shared));
    }

    #[test]
    fn overlap_is_about_area_not_touching() {
        let a = Rect::from_min_size(Vec2::ZERO, Vec2::new(1.0, 1.0));
        let touching = Rect::from_min_size(Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0));
        let overlapping = Rect::from_min_size(Vec2::new(0.5, 0.5), Vec2::new(1.0, 1.0));
        assert!(!a.overlaps(touching), "edge to edge is not overlap");
        assert!(a.overlaps(overlapping));
    }

    #[test]
    fn the_white_texture_prints_as_itself() {
        assert_eq!(format!("{:?}", TextureId::WHITE), "TextureId(white)");
        assert_eq!(format!("{:?}", TextureId::from_bits(7)), "TextureId(7)");
    }

    #[test]
    fn a_degenerate_surface_has_a_usable_aspect() {
        // A minimized window reports zero height. Dividing by it would put NaN
        // into every vertex of the frame, which is a far worse outcome than a
        // frame drawn at the wrong shape and never seen.
        assert_eq!(PhysicalSize::new(0, 0).aspect(), 1.0);
        assert_eq!(PhysicalSize::new(800, 0).aspect(), 1.0);
        assert_eq!(PhysicalSize::new(1600, 900).aspect(), 16.0 / 9.0);
    }

    #[test]
    fn depth_defaults_to_the_middle_of_everything() {
        assert_eq!(Depth::default(), Depth { layer: 0, z: 0.0 });
        assert_eq!(Depth::layer(3), Depth { layer: 3, z: 0.0 });
    }
}
