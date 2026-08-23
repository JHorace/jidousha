//! The scaling contract: uniform fit, letterboxed, symmetric, with a floor
//! (UI.md §6; DESIGN §7a).
//!
//! **What the engine gives, and what it leaves to the game.** `Camera::height`
//! is the world span the surface is *tall*, and the width follows the window's
//! aspect; the driver stamps the real `viewport` into the camera every frame
//! and never touches `height` (jidousha-api.md, "The camera is the game's; its
//! `viewport` is the driver's"). So a game that names one `height` and leaves
//! it there scales uniformly when the window shrinks *vertically* and does not
//! scale at all when it shrinks *horizontally* — the same world-units-per-pixel
//! on both axes, with the extra width simply cut off. That is exactly UI.md
//! §6's reported defect, and it is a game-side one: nothing in the engine could
//! know which of the two behaviours giri wanted.
//!
//! **What this does.** Every frame, before anything reads the camera, `fit`
//! recomputes `height` so the whole 960x540 design rect stays on screen at a
//! uniform scale:
//!
//! ```text
//! s = max(MIN_SCALE, min(viewport.w / 960, viewport.h / 540))
//! height = DESIGN_H * viewport.h / (s * 540)
//! ```
//!
//! At the reference size `s` is 1 and `height` is `DESIGN_H`. Narrower, `s`
//! falls with the width and `height` grows, so world-units-per-pixel rises on
//! *both* axes and the picture shrinks whole; the spare span above and below is
//! the letterbox. Wider or shorter, the same arithmetic leaves the spare span
//! at the sides. The camera stays centred on the design rect, so the spare span
//! is split evenly — "symmetric in both axes", which is the half of the
//! contract a fit that pinned a corner would fail.
//!
//! Below `MIN_SCALE` the scale clamps and the view stops shrinking: a window
//! small enough to make the text unreadable gets a readable picture with its
//! edges off-screen instead, which is the trade UI.md §6 asks for.

use jidousha::prelude::*;

use crate::layout::{DESIGN_H, DESIGN_W, REFERENCE};
use crate::theme;

/// The smallest uniform scale the view will shrink to, as a multiple of
/// reference scale. The mockup's own floor, demonstrated by its outer frame.
pub const MIN_SCALE: f32 = 0.3;

/// The uniform scale a viewport of this size gets: 1.0 at the reference size.
///
/// A free function because three readers want it — the camera fit below, the
/// verify run's expectations, and the readability floors, which are stated at
/// reference scale and have to know what scale a capture was taken at.
pub fn scale_for(viewport: PhysicalSize) -> f32 {
    if viewport.width == 0 || viewport.height == 0 {
        return MIN_SCALE;
    }
    let horizontal = viewport.width as f32 / REFERENCE.width as f32;
    let vertical = viewport.height as f32 / REFERENCE.height as f32;
    horizontal.min(vertical).max(MIN_SCALE)
}

/// The camera height that puts `scale_for(viewport)` on the screen.
pub fn height_for(viewport: PhysicalSize) -> f32 {
    if viewport.height == 0 {
        return DESIGN_H;
    }
    DESIGN_H * viewport.height as f32 / (scale_for(viewport) * REFERENCE.height as f32)
}

/// Where the camera sits: the design rect's centre, so the letterbox is
/// symmetric on whichever axis has the spare span.
pub fn center() -> Vec2 {
    Vec2::new(DESIGN_W * 0.5, DESIGN_H * 0.5)
}

/// The surface a run draws to, when nothing measures a window.
///
/// A simulation input like `Tuning` and `StartAt`: the windowed game inserts
/// none and the driver stamps the real window into `Camera::viewport` every
/// frame, while `headless` stamps nothing at all — so a check that draws to a
/// surface has to say which one, before Startup, or the camera it clicks
/// through and the camera the game hit-tests with are two different cameras and
/// every pointer position is wrong (jidousha-testing.md's viewport trap).
#[derive(Clone, Copy, Debug)]
pub struct Surface(pub PhysicalSize);
impl Resource for Surface {}

impl Default for Surface {
    fn default() -> Self {
        Self(crate::WINDOW)
    }
}

/// The camera the game runs with, for a given surface.
///
/// The one builder, so the windowed game, the headless run and the capture
/// path cannot disagree about where a click lands (`jidousha-testing.md`'s
/// viewport trap: a check that builds its camera differently converts every
/// pointer position to the wrong pixel).
pub fn camera_for(viewport: PhysicalSize) -> Camera {
    Camera {
        center: center(),
        height: height_for(viewport),
        clear_color: theme::VOID,
        viewport,
    }
}

/// Keep the camera fitted to whatever surface the driver last stamped.
///
/// Registered **first** in Update, so the click handler that follows converts
/// pointer pixels through the same camera the frame the player clicked on was
/// drawn with. Reversed, a resize would send one frame's clicks to the previous
/// frame's rectangles; `verify.rs` asserts the order out of `schedule_debug`.
pub fn fit(world: &mut World) {
    let camera = world.resource_mut::<Camera>();
    camera.center = center();
    camera.height = height_for(camera.viewport);
}
