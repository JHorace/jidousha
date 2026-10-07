//! The one conversion between the 960x540 design space screens are laid out in and the
//! world the camera looks at: the design rectangle fitted inside the view, centred.

use jidousha::prelude::*;
use jidousha::ui::Mapping;

use crate::screens::{DESIGN_H, DESIGN_W};

/// The design rectangle placed in the view, uniform and letterboxed.
pub struct UiMap {
    origin: Vec2,
    scale: f32,
}

impl UiMap {
    /// Fit the design space inside what `camera` shows.
    pub fn for_camera(camera: &Camera) -> Self {
        let view = camera.visible_bounds();
        let scale = (view.size().x / DESIGN_W).min(view.size().y / DESIGN_H);
        Self {
            origin: view.center() - Vec2::new(DESIGN_W, DESIGN_H) * (scale * 0.5),
            scale,
        }
    }

    /// Where a world point is in the design space: the inverse of `to_world`.
    pub fn to_design(&self, world: Vec2) -> Vec2 {
        (world - self.origin) / self.scale
    }

    /// A design rectangle in the world.
    pub fn rect_to_world(&self, rect: Rect) -> Rect {
        Rect::from_min_size(self.to_world(rect.min), rect.size() * self.scale)
    }
}

impl Mapping for UiMap {
    fn to_world(&self, ui: Vec2) -> Vec2 {
        self.origin + ui * self.scale
    }

    fn scale(&self) -> f32 {
        self.scale
    }
}
