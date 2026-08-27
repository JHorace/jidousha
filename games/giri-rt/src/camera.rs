//! The map camera and the UI mapping: pan/zoom over the world, chrome that
//! stays put (DESIGN §3; UI.md §5, §6).
//!
//! **The engine Camera is the pan/zoom** — `center` moves over the map and
//! `height` is the zoom, both game state driven by recorded input
//! (`flow.rs`). The driver stamps the real `viewport` every frame and the
//! game never touches it.
//!
//! **The UI rides the camera.** Every piece of chrome is laid out in
//! `layout.rs`'s 960x540 UI space, and [`UiMap`] is the one conversion
//! between that space and the world: the UI rect is fitted uniformly inside
//! whatever the camera currently shows, centred, exactly the scaling contract
//! giri's design rect had — so the chrome is a constant size *on screen*
//! whatever the zoom, the readability floors stay stated in reference pixels,
//! and a pointer hit-test converts through the same map the frame was drawn
//! with. At the default camera on a 16:9 surface the scale is exactly 1 and
//! UI units are world units.
//!
//! The camera is presentation: nothing in the simulation reads it, so panning
//! during a replay cannot move an outcome — but hit-tests do convert through
//! it, which is why `fit` runs before the click handler (`verify.rs` asserts
//! the order).

use jidousha::prelude::*;

use crate::grid::Grid;
use crate::layout::{DESIGN_H, DESIGN_W};
use crate::theme;

/// The camera height the scenario opens at: the whole map in view, with the
/// UI at exactly reference scale on a 16:9 surface.
pub const DEFAULT_H: f32 = 540.0;
/// The closest the zoom goes (world units of height on screen).
pub const MIN_H: f32 = 135.0;
/// And the widest.
pub const MAX_H: f32 = 1080.0;

/// How fast the arrow keys pan, in view-heights per second.
pub const PAN_RATE: f32 = 0.8;
/// How fast the zoom keys move the height, as a factor per second.
pub const ZOOM_RATE: f32 = 1.8;
/// How much one scroll line zooms, as a factor.
pub const SCROLL_STEP: f32 = 1.12;

/// The surface a run draws to, when nothing measures a window.
///
/// A simulation input like `Tuning`: the windowed game inserts none and the
/// driver stamps the real window into `Camera::viewport` every frame, while
/// `headless` stamps nothing at all — so a check that draws to a surface says
/// which one, before Startup, or the camera it clicks through and the camera
/// the game hit-tests with are two different cameras and every pointer
/// position is wrong (jidousha-testing.md's viewport trap).
#[derive(Clone, Copy, Debug)]
pub struct Surface(pub PhysicalSize);
impl Resource for Surface {}

impl Default for Surface {
    fn default() -> Self {
        Self(crate::WINDOW)
    }
}

/// The camera a scenario opens with, for a given surface: centred on the map,
/// at the default zoom.
pub fn camera_for(viewport: PhysicalSize) -> Camera {
    let map = crate::grid::grid().world_rect();
    Camera {
        center: map.center(),
        height: DEFAULT_H,
        clear_color: theme::VOID,
        viewport,
    }
}

/// Keep the camera legal: zoom inside its clamps, centre over the map.
///
/// Registered **first** in Update, so the click handler that follows converts
/// pointer pixels through the same camera the frame the player clicked on was
/// drawn with. The centre clamp keeps the view over the map where the view is
/// smaller than it, and pins the centre to the map's own centre on any axis
/// where the view is wider — panning past the world's edge shows the void,
/// which is a fact, not a picture.
pub fn fit(world: &mut World) {
    let map = world.resource::<Grid>().world_rect();
    let camera = world.resource_mut::<Camera>();
    camera.height = camera.height.clamp(MIN_H, MAX_H);
    let half = Vec2::new(camera.width(), camera.height) * 0.5;
    for axis in 0..2 {
        let (low, high, center) = if axis == 0 {
            (map.min.x + half.x, map.max.x - half.x, &mut camera.center.x)
        } else {
            (map.min.y + half.y, map.max.y - half.y, &mut camera.center.y)
        };
        if low >= high {
            *center = (low + high) * 0.5;
        } else {
            *center = center.clamp(low, high);
        }
    }
}

/// The one conversion between UI space (960x540 reference pixels) and the
/// world the camera is looking at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiMap {
    /// Where UI (0,0) lands in the world.
    pub origin: Vec2,
    /// World units per UI unit.
    pub scale: f32,
}

impl UiMap {
    /// The mapping for what `camera` currently shows: the UI rect fitted
    /// uniformly inside the view, centred — aspect preserved, letterboxed,
    /// symmetric, which is giri's scaling contract restated over a camera
    /// that moves.
    pub fn for_camera(camera: &Camera) -> Self {
        let view = camera.visible_bounds();
        let size = view.size();
        let scale = (size.x / DESIGN_W).min(size.y / DESIGN_H);
        Self {
            origin: view.center() - Vec2::new(DESIGN_W, DESIGN_H) * (scale * 0.5),
            scale,
        }
    }

    /// A UI point in the world.
    pub fn to_world(self, ui: Vec2) -> Vec2 {
        self.origin + ui * self.scale
    }

    /// A UI rectangle in the world.
    pub fn to_world_rect(self, ui: Rect) -> Rect {
        Rect {
            min: self.to_world(ui.min),
            max: self.to_world(ui.max),
        }
    }

    /// A world point in UI units — what a pointer hit-test compares against
    /// `layout.rs`.
    pub fn ui_of(self, world: Vec2) -> Vec2 {
        (world - self.origin) / self.scale
    }
}
