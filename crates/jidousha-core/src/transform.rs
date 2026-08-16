//! Where a thing is: the component almost every drawn entity carries.
//!
//! Key types: `Transform`.
//! Depends on: `component`, `math`, `visual`.
//! INVARIANT: this is simulation data, not rendering data. It lives in core
//! rather than in the renderer because gameplay reads and writes it constantly
//! — a bullet's position is not a rendering concern (renderer.md §3).

use crate::component::Component;
use crate::math::{Radians, Vec2, rotate};
use crate::visual::Depth;

/// Position, rotation, and scale in world space.
///
/// ```
/// # use jidousha_core::{Transform, math::{Radians, Vec2}};
/// let at_origin = Transform::default();
/// let placed = Transform {
///     pos: Vec2::new(3.0, -2.0),
///     rot: Radians::from_degrees(90.0),
///     ..Transform::default()
/// };
/// assert_eq!(at_origin.pos, Vec2::ZERO);
/// assert_eq!(placed.scale, Vec2::ONE, "scale defaults to natural size");
/// ```
///
/// DELIBERATE: `pos` is a `Vec2` with a separate `z` for draw order, rather
/// than a `Vec3`. This is a 2D engine and a 2D ergonomic choice; the eventual
/// 3D transform is its own type behind its own ADR, not a third field quietly
/// added here (ADR-0001, renderer.md §3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// Where, in world units. Y is down (ADR-0010).
    pub pos: Vec2,
    /// Draw order within the layer. Higher draws on top; not a spatial axis.
    pub z: f32,
    /// Rotation, clockwise on screen (ADR-0010).
    pub rot: Radians,
    /// Size multiplier. `Vec2::ONE` is natural size.
    pub scale: Vec2,
}

impl Component for Transform {}

impl Default for Transform {
    /// The origin, unrotated, at natural size.
    ///
    /// DELIBERATE: a meaningful `Default` (ADR-0012) — "no transform yet" is a
    /// real state a spawning entity is in, and `..Transform::default()` is how
    /// a game states the one field it cares about.
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            z: 0.0,
            rot: Radians::ZERO,
            scale: Vec2::ONE,
        }
    }
}

impl Transform {
    /// A transform at `pos`, unrotated and at natural size.
    #[must_use]
    pub fn at(pos: Vec2) -> Self {
        Self {
            pos,
            ..Self::default()
        }
    }

    /// Take a point in this transform's local frame into world space.
    ///
    /// Scale, then rotate, then translate — the order everything drawn agrees
    /// on, so a rotated sprite and a rotated hitbox land in the same place.
    #[must_use]
    pub fn apply(&self, local: Vec2) -> Vec2 {
        self.pos + rotate(local * self.scale, self.rot)
    }

    /// This transform's draw depth in `layer`.
    #[must_use]
    pub fn depth(&self, layer: i16) -> Depth {
        Depth { layer, z: self.z }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_transform_is_the_origin_at_natural_size() {
        let transform = Transform::default();
        assert_eq!(transform.pos, Vec2::ZERO);
        assert_eq!(transform.scale, Vec2::ONE);
        assert_eq!(transform.rot, Radians::ZERO);
        assert_eq!(transform.z, 0.0);
    }

    #[test]
    fn an_untransformed_point_is_where_it_started() {
        let transform = Transform::at(Vec2::new(5.0, 7.0));
        assert_eq!(transform.apply(Vec2::ZERO), Vec2::new(5.0, 7.0));
        assert_eq!(transform.apply(Vec2::new(1.0, 0.0)), Vec2::new(6.0, 7.0));
    }

    #[test]
    fn scale_applies_before_rotation() {
        // A quarter turn of a point stretched along X must land along Y at the
        // stretched length, not the original one.
        let transform = Transform {
            rot: Radians::from_degrees(90.0),
            scale: Vec2::new(3.0, 1.0),
            ..Transform::default()
        };
        let moved = transform.apply(Vec2::new(1.0, 0.0));
        assert!((moved.x).abs() < 1e-6, "{moved:?}");
        // Y is down and positive rotation is clockwise on screen, so +X turns
        // to +Y (ADR-0010).
        assert!((moved.y - 3.0).abs() < 1e-6, "{moved:?}");
    }

    #[test]
    fn depth_takes_its_z_from_the_transform() {
        let transform = Transform {
            z: 2.5,
            ..Transform::default()
        };
        assert_eq!(transform.depth(4), Depth { layer: 4, z: 2.5 });
    }
}
