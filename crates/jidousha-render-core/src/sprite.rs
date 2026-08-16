//! The sprite component, and how one becomes a quad.
//!
//! Key types: `Sprite`.
//! Depends on: `jidousha-core`, `jidousha-assets`.
//! INVARIANT: expansion happens here, above the backend seam, on the CPU. A
//! backend receives vertices and samples them; it never learns what an anchor
//! is (renderer.md §1, §7).

use jidousha_assets::TextureHandle;
use jidousha_core::math::Vec2;
use jidousha_core::{Color, Component, Quad, Rect, Transform};

/// A picture attached to an entity.
///
/// ```
/// # use jidousha_render_core::Sprite;
/// # use jidousha_assets::{Assets, MemorySource};
/// # use jidousha_core::math::Vec2;
/// # let mut source = MemorySource::new();
/// # source.insert("ship.png", vec![0]);
/// # let mut assets = Assets::new(source);
/// let ship = Sprite {
///     texture: assets.load_texture("ship.png"),
///     size: Vec2::new(2.0, 2.0),
///     ..Sprite::new(assets.load_texture("ship.png"))
/// };
/// ```
///
/// `size` is in **world units**, not texels: swapping the art for a
/// higher-resolution version never changes gameplay geometry. CONTRACT: nothing
/// in simulation may read texture dimensions (renderer.md §3), which is what
/// keeps a game's behaviour independent of the pixels that happen to be on disk.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    /// What to draw.
    pub texture: TextureHandle,
    /// Which part of the texture, in normalized 0..1 coordinates. `None` is the
    /// whole thing.
    pub region: Option<Rect>,
    /// How big the quad is, in world units.
    pub size: Vec2,
    /// Where the transform's position sits on the quad: `(0, 0)` is the center,
    /// `(-0.5, -0.5)` the top-left corner, `(0.5, 0.5)` the bottom-right.
    pub anchor: Vec2,
    /// Multiplied into the texture's color.
    pub tint: Color,
    /// Mirror horizontally.
    pub flip_x: bool,
    /// Mirror vertically.
    pub flip_y: bool,
    /// The coarse draw band. The fine order comes from `Transform::z`.
    pub layer: i16,
}

impl Component for Sprite {}

impl Sprite {
    /// A one-unit sprite, centered, untinted, on layer zero.
    ///
    /// DELIBERATE: no `Default` impl, because there is no meaningful default
    /// texture — a sprite is *about* its texture (ADR-0012). This is the one
    /// constructor, and `..Sprite::new(handle)` is how a game states the rest.
    #[must_use]
    pub fn new(texture: TextureHandle) -> Self {
        Self {
            texture,
            region: None,
            size: Vec2::ONE,
            anchor: Vec2::ZERO,
            tint: Color::WHITE,
            flip_x: false,
            flip_y: false,
            layer: 0,
        }
    }

    /// The quad this sprite draws at `transform`.
    ///
    /// Corners wind top-left, top-right, bottom-right, bottom-left in the
    /// sprite's own frame, before the transform rotates them. Flips permute the
    /// texture coordinates rather than the corners, so a flipped sprite keeps
    /// its winding and its rotation still turns the way the transform says.
    #[must_use]
    pub fn quad(&self, transform: &Transform) -> Quad {
        // The anchor names which point of the quad sits at the transform's
        // position, so the quad's own frame is offset by the opposite of it.
        let center = -self.anchor * self.size;
        let half = self.size * 0.5;
        let local = [
            center + Vec2::new(-half.x, -half.y),
            center + Vec2::new(half.x, -half.y),
            center + Vec2::new(half.x, half.y),
            center + Vec2::new(-half.x, half.y),
        ];
        let region = self.region.unwrap_or(Rect::UNIT);
        let mut uvs = [
            Vec2::new(region.min.x, region.min.y),
            Vec2::new(region.max.x, region.min.y),
            Vec2::new(region.max.x, region.max.y),
            Vec2::new(region.min.x, region.max.y),
        ];
        if self.flip_x {
            uvs.swap(0, 1);
            uvs.swap(2, 3);
        }
        if self.flip_y {
            uvs.swap(0, 3);
            uvs.swap(1, 2);
        }
        Quad {
            corners: local.map(|point| transform.apply(point)),
            uvs,
            tint: self.tint,
            texture: self.texture.texture_id(),
            depth: transform.depth(self.layer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jidousha_assets::{Assets, MemorySource};
    use jidousha_core::math::Radians;

    fn a_handle() -> TextureHandle {
        let mut source = MemorySource::new();
        source.insert("a.png", vec![0]);
        Assets::new(source).load_texture("a.png")
    }

    fn unit_sprite() -> Sprite {
        Sprite {
            size: Vec2::new(2.0, 4.0),
            ..Sprite::new(a_handle())
        }
    }

    #[test]
    fn a_centered_sprite_straddles_its_position() {
        let quad = unit_sprite().quad(&Transform::at(Vec2::new(10.0, 20.0)));
        assert_eq!(quad.corners[0], Vec2::new(9.0, 18.0), "top-left");
        assert_eq!(quad.corners[2], Vec2::new(11.0, 22.0), "bottom-right");
    }

    #[test]
    fn a_top_left_anchor_puts_the_position_at_the_corner() {
        let sprite = Sprite {
            anchor: Vec2::new(-0.5, -0.5),
            ..unit_sprite()
        };
        let quad = sprite.quad(&Transform::at(Vec2::new(10.0, 20.0)));
        assert_eq!(quad.corners[0], Vec2::new(10.0, 20.0));
        assert_eq!(quad.corners[2], Vec2::new(12.0, 24.0));
    }

    #[test]
    fn scale_multiplies_the_world_size() {
        let transform = Transform {
            scale: Vec2::new(3.0, 0.5),
            ..Transform::default()
        };
        let quad = unit_sprite().quad(&transform);
        assert_eq!(quad.corners[0], Vec2::new(-3.0, -1.0));
        assert_eq!(quad.corners[2], Vec2::new(3.0, 1.0));
    }

    #[test]
    fn the_whole_texture_is_the_default_region() {
        let quad = unit_sprite().quad(&Transform::default());
        assert_eq!(quad.uvs[0], Vec2::ZERO);
        assert_eq!(quad.uvs[2], Vec2::ONE);
    }

    #[test]
    fn an_atlas_region_samples_only_its_part() {
        let sprite = Sprite {
            region: Some(Rect::from_min_size(
                Vec2::new(0.25, 0.5),
                Vec2::new(0.25, 0.5),
            )),
            ..unit_sprite()
        };
        let quad = sprite.quad(&Transform::default());
        assert_eq!(quad.uvs[0], Vec2::new(0.25, 0.5));
        assert_eq!(quad.uvs[2], Vec2::new(0.5, 1.0));
    }

    #[test]
    fn flipping_moves_the_texture_and_leaves_the_geometry_alone() {
        let plain = unit_sprite().quad(&Transform::default());
        let flipped = Sprite {
            flip_x: true,
            ..unit_sprite()
        }
        .quad(&Transform::default());
        assert_eq!(
            flipped.corners, plain.corners,
            "same quad, mirrored texture"
        );
        assert_eq!(flipped.uvs[0], plain.uvs[1]);
        assert_eq!(flipped.uvs[1], plain.uvs[0]);
    }

    #[test]
    fn flipping_both_ways_is_a_half_turn_of_the_texture() {
        let plain = unit_sprite().quad(&Transform::default());
        let flipped = Sprite {
            flip_x: true,
            flip_y: true,
            ..unit_sprite()
        }
        .quad(&Transform::default());
        assert_eq!(flipped.uvs[0], plain.uvs[2]);
        assert_eq!(flipped.uvs[2], plain.uvs[0]);
    }

    #[test]
    fn rotation_turns_the_quad_about_its_anchor() {
        let transform = Transform {
            rot: Radians::from_degrees(90.0),
            ..Transform::default()
        };
        let quad = unit_sprite().quad(&transform);
        // The top-left corner (-1, -2) turns clockwise on screen to (2, -1).
        assert!(
            (quad.corners[0] - Vec2::new(2.0, -1.0)).length() < 1e-5,
            "{:?}",
            quad.corners
        );
    }

    #[test]
    fn the_layer_comes_from_the_sprite_and_the_z_from_the_transform() {
        // Two sources, deliberately: layer is a property of what the thing *is*,
        // z is a property of where it is (renderer.md §3).
        let sprite = Sprite {
            layer: 5,
            ..unit_sprite()
        };
        let transform = Transform {
            z: 1.5,
            ..Transform::default()
        };
        let quad = sprite.quad(&transform);
        assert_eq!(quad.depth.layer, 5);
        assert_eq!(quad.depth.z, 1.5);
    }
}
