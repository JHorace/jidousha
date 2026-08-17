//! The camera: what part of the world is on screen, and the only sanctioned
//! way to convert between the two spaces.
//!
//! Key types: `Camera`.
//! Depends on: `jidousha-core`, `backend` (for `PhysicalSize`).
//! INVARIANT: world↔screen conversion happens here and nowhere else
//! (conventions). A second conversion elsewhere would be a second source of
//! truth about where things are, and the two would drift.

use jidousha_core::math::{Mat4, Vec2};
use jidousha_core::{Color, Resource};

use jidousha_core::PhysicalSize;

/// What the frame is looking at, held as a world resource.
///
/// A game sets its own in a `Startup` system —
/// `world.insert_resource(Camera { height: 20.0, ..Camera::default() })` — and
/// reads it back with `world.resource::<Camera>()`. A windowed run that never
/// inserts one still has one: `run` installs [`Camera::default`] before the
/// first frame rather than refusing to draw, because "I have not thought about
/// the camera yet" is a real state for a prototype to be in. **A headless run
/// does not**, so a game leaning on that default has no `Camera` resource in a
/// test — `FrameRecorder` draws with the default without inserting it, and
/// `world.resource::<Camera>()` panics.
///
/// One camera in v1; multiple cameras and render-to-texture are deferred
/// together (renderer.md §4).
///
/// ```
/// # use jidousha_render_core::Camera;
/// # use jidousha_core::math::Vec2;
/// let mut camera = Camera::default();
/// camera.center = Vec2::new(10.0, 0.0);
/// camera.height = 20.0;      // 20 world units tall; zoom by changing this
/// ```
///
/// **Width follows from height and the surface's aspect.** A prototype looks
/// right on any screen without letterboxing logic: a wider window shows more
/// world to the sides, never a squashed picture. Letterbox mode is deferred
/// until a game needs it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// The world position at the center of the screen.
    pub center: Vec2,
    /// How many world units the screen spans vertically.
    pub height: f32,
    /// What to fill the screen with before drawing.
    pub clear_color: Color,
    /// The surface this camera is drawing to, in pixels.
    ///
    /// Maintained by the driver — the platform crate writes it on resize, and
    /// a headless run keeps the default. Games read it (through the conversions
    /// below) rather than setting it: it describes the window, and a game that
    /// set it would be lying to itself about how big the window is.
    pub viewport: PhysicalSize,
}

impl Resource for Camera {}

impl Default for Camera {
    /// Centered on the origin, twenty units tall, on a black 1280×720 screen.
    ///
    /// DELIBERATE: a meaningful `Default` (ADR-0012) — "the camera I have not
    /// configured yet" is a real state, and a prototype's first sprite should
    /// be visible without a camera setup step. The default viewport gives a
    /// headless run a definite aspect ratio, which keeps transcripts identical
    /// between a test and a windowed run of the same size.
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            height: 20.0,
            clear_color: Color::BLACK,
            viewport: PhysicalSize::new(1280, 720),
        }
    }
}

impl Camera {
    /// How many world units the screen spans horizontally.
    #[must_use]
    pub fn width(&self) -> f32 {
        self.height * self.viewport.aspect()
    }

    /// The world rectangle currently on screen, as (top-left, bottom-right).
    ///
    /// Y is down, so the first corner is the top-left (ADR-0010).
    #[must_use]
    pub fn visible_bounds(&self) -> (Vec2, Vec2) {
        let half = Vec2::new(self.width(), self.height) * 0.5;
        (self.center - half, self.center + half)
    }

    /// Where a world point lands on screen, in pixels from the top-left.
    ///
    /// The only sanctioned world→screen conversion (conventions).
    #[must_use]
    pub fn world_to_screen(&self, world: Vec2) -> Vec2 {
        let (min, _) = self.visible_bounds();
        let offset = world - min;
        let scale = self.pixels_per_unit();
        Vec2::new(offset.x * scale.x, offset.y * scale.y)
    }

    /// What world point a screen pixel is over.
    ///
    /// The only sanctioned screen→world conversion. This is how a click becomes
    /// a place in the world.
    #[must_use]
    pub fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        let (min, _) = self.visible_bounds();
        let scale = self.pixels_per_unit();
        // A zero-sized viewport has no meaningful mapping; report the center
        // rather than a NaN that would spread into gameplay.
        if scale.x == 0.0 || scale.y == 0.0 {
            return self.center;
        }
        min + Vec2::new(screen.x / scale.x, screen.y / scale.y)
    }

    /// The matrix that takes world space to clip space.
    ///
    /// Built once per frame and handed to the backend in the [`FramePlan`].
    /// Y is down in world space and up in clip space, so this flips it — the
    /// one place that flip happens.
    ///
    /// [`FramePlan`]: crate::FramePlan
    #[must_use]
    pub fn view_projection(&self) -> Mat4 {
        let half = Vec2::new(self.width(), self.height) * 0.5;
        let left = self.center.x - half.x;
        let right = self.center.x + half.x;
        // Top has the *smaller* world Y, so `top - bottom` is negative and the
        // Y scale below comes out negative — that is the Y flip, and it is the
        // only place it happens.
        let top = self.center.y - half.y;
        let bottom = self.center.y + half.y;
        let (near, far) = (-1.0, 1.0);

        // DELIBERATE: written out rather than calling glam's `orthographic_*`
        // helpers. They are deprecated as of glam 0.33 and moving, and this
        // matrix is on the determinism path — six divisions we own beat an
        // upstream API whose depth convention could change under us (ADR-0009's
        // reasoning, applied to projection rather than trigonometry).
        // Column-major, and depth mapped to 0..1 as wgpu and Vulkan want.
        Mat4::from_cols_array(&[
            2.0 / (right - left),
            0.0,
            0.0,
            0.0,
            //
            0.0,
            2.0 / (top - bottom),
            0.0,
            0.0,
            //
            0.0,
            0.0,
            1.0 / (far - near),
            0.0,
            //
            -(right + left) / (right - left),
            -(top + bottom) / (top - bottom),
            -near / (far - near),
            1.0,
        ])
    }

    /// Screen pixels per world unit, on each axis.
    fn pixels_per_unit(&self) -> Vec2 {
        let width = self.width();
        if width == 0.0 || self.height == 0.0 {
            return Vec2::ZERO;
        }
        Vec2::new(
            self.viewport.width as f32 / width,
            self.viewport.height as f32 / self.height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_camera() -> Camera {
        Camera {
            viewport: PhysicalSize::new(100, 100),
            height: 10.0,
            ..Camera::default()
        }
    }

    #[test]
    fn the_camera_center_is_the_middle_of_the_screen() {
        let camera = square_camera();
        assert_eq!(camera.world_to_screen(Vec2::ZERO), Vec2::new(50.0, 50.0));
    }

    #[test]
    fn width_follows_the_aspect_ratio() {
        // A wider window shows more world, rather than the same world squashed
        // — the whole point of fixed-height framing (renderer.md §4).
        let camera = Camera {
            viewport: PhysicalSize::new(200, 100),
            height: 10.0,
            ..Camera::default()
        };
        assert_eq!(camera.width(), 20.0);
        assert_eq!(camera.height, 10.0);
    }

    #[test]
    fn screen_y_grows_downwards_like_world_y() {
        // Both spaces agree on which way is down (ADR-0010, conventions), so a
        // falling object moves down on screen without anyone negating anything.
        let camera = square_camera();
        let above = camera.world_to_screen(Vec2::new(0.0, -1.0));
        let below = camera.world_to_screen(Vec2::new(0.0, 1.0));
        assert!(above.y < below.y, "{above:?} then {below:?}");
    }

    #[test]
    fn the_two_conversions_are_each_others_inverse() {
        let camera = Camera {
            center: Vec2::new(3.0, -7.0),
            viewport: PhysicalSize::new(800, 600),
            height: 12.0,
            ..Camera::default()
        };
        for world in [
            Vec2::ZERO,
            Vec2::new(3.0, -7.0),
            Vec2::new(-20.0, 15.0),
            Vec2::new(0.5, 0.25),
        ] {
            let round_trip = camera.screen_to_world(camera.world_to_screen(world));
            assert!(
                (round_trip - world).length() < 1e-4,
                "{world:?} became {round_trip:?}"
            );
        }
    }

    #[test]
    fn a_zoomed_camera_shows_less_world() {
        let mut camera = square_camera();
        let wide = camera.visible_bounds();
        camera.height = 5.0;
        let close = camera.visible_bounds();
        assert!(close.1.x - close.0.x < wide.1.x - wide.0.x);
    }

    #[test]
    fn a_minimized_window_does_not_produce_nan() {
        // Zero height would divide by zero and put NaN into gameplay through
        // screen_to_world. A wrong answer nobody can see beats a NaN that
        // spreads (renderer.md §10).
        let camera = Camera {
            viewport: PhysicalSize::new(0, 0),
            ..Camera::default()
        };
        let world = camera.screen_to_world(Vec2::new(10.0, 10.0));
        assert!(world.x.is_finite() && world.y.is_finite(), "{world:?}");
    }

    #[test]
    fn the_projection_puts_the_camera_center_at_the_middle_of_clip_space() {
        let camera = square_camera();
        let clip = camera.view_projection() * camera.center.extend(0.0).extend(1.0);
        assert!(clip.x.abs() < 1e-6 && clip.y.abs() < 1e-6, "{clip:?}");
    }

    #[test]
    fn the_projection_flips_y_for_clip_space() {
        // World +Y is down; clip +Y is up. Something below the camera must land
        // in the lower half of clip space, which is negative Y there.
        let camera = square_camera();
        let below = Vec2::new(0.0, 2.0);
        let clip = camera.view_projection() * below.extend(0.0).extend(1.0);
        assert!(clip.y < 0.0, "{clip:?}");
    }
}
