//! How many device pixels a page renders into: the `?renderscale=` seam.
//!
//! Key types: `RenderScale`; `from_query`, `requested`.
//! Depends on: `jidousha-render-core` (`PhysicalSize`), `web-sys` (web only).
//! Must never be depended on by: `jidousha-core` — this is a presentation
//! setting read off a page URL, and nothing simulation can see may depend on
//! which browser is drawing.
//! INVARIANT: presentation-only, and that is a three-part promise. The scale
//! multiplies the surface size, the camera viewport that follows it, and
//! pointer positions — *together*, so the aspect ratio the letterbox is built
//! on (games/giri/UI.md §6), where a click lands, and every world coordinate
//! are all unchanged. The only thing that moves is the number of device pixels
//! the browser is asked to fill (web-publish.md §2).
//! INVARIANT: web-only in effect, compiled everywhere. A native run always gets
//! [`RenderScale::FULL`]; the parsing and the arithmetic live outside the `cfg`
//! for the same reason `asset_url` and `panic::query_asks_for_panic` do — a
//! function behind a `cfg` is a function no test on this machine can reach.
//!
//! **Why the engine reads this and not the page.** The canvas's backing store
//! is not the template's to set: winit reports the canvas's device-pixel box
//! and wgpu writes `canvas.width`/`canvas.height` from the extent it is
//! configured with, so a page-side write would be overwritten by the next
//! surface configure. The size handed to the surface is the one number that
//! decides how many pixels exist, and it is on this side of the seam.

use jidousha_render_core::PhysicalSize;

use crate::web::query_parameter;

/// The query parameter that asks for it.
const PARAMETER: &str = "renderscale";

/// The smallest scale this build accepts.
///
/// A quarter of the linear resolution is a sixteenth of the pixels, which is
/// past the point where a sprite is still a picture. A floor rather than a free
/// number so "it renders fast now but I cannot read it" is not a state a
/// playtester can reach from a URL.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const FLOOR: f32 = 0.25;

/// The largest scale this build accepts.
///
/// One, not more: rendering *more* pixels than the display has is
/// supersampling, which is a different decision with a different cost, and the
/// WebGL2 envelope (renderer.md §8) is not somewhere to wander into by typing a
/// bigger number into a URL.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const CEILING: f32 = 1.0;

/// What fraction of the display's device pixels to render.
///
/// Constructed only by [`from_query`] or as [`RenderScale::FULL`], so a value
/// outside `FLOOR..=CEILING` cannot exist — which is what makes every call site
/// below free of range checks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RenderScale(f32);

impl RenderScale {
    /// Every device pixel the display has — what every run gets unless a page
    /// URL asks otherwise, and the only value a native run ever has.
    pub(crate) const FULL: Self = Self(1.0);

    /// The multiplier itself, for the one caller that scales a position rather
    /// than a size (`translate::pointer_moved`).
    pub(crate) fn factor(self) -> f32 {
        self.0
    }

    /// The surface size for a window this big.
    ///
    /// Both dimensions by the same factor, which is what keeps
    /// `PhysicalSize::aspect` — and therefore the camera's width, and therefore
    /// the letterbox — the number it would have been at full resolution.
    pub(crate) fn apply(self, size: PhysicalSize) -> PhysicalSize {
        PhysicalSize::new(
            scale_dimension(size.width, self.0),
            scale_dimension(size.height, self.0),
        )
    }
}

/// One dimension, scaled and rounded to whole pixels.
///
/// Zero stays zero: a canvas that is not in the document reports 0×0 (winit's
/// answer for a hidden canvas) and that means "nothing to draw", not "one
/// pixel". Anything else stays at least one pixel, because a surface configured
/// to zero width is an error the browser raises rather than a small picture.
fn scale_dimension(value: u32, factor: f32) -> u32 {
    if value == 0 {
        return 0;
    }
    ((value as f32 * factor).round() as u32).max(1)
}

/// The scale a page query string asks for, and the problem to report if it
/// asked for something impossible.
///
/// A malformed parameter is a **handled** problem (core.md §9): the page says
/// what was wrong and renders at full resolution, because refusing to start a
/// game over a typo in a diagnostic parameter would be the worse failure. It is
/// never silent — that is what the second half of the return value is for.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn from_query(search: &str) -> (RenderScale, Option<String>) {
    let Some(value) = query_parameter(search, PARAMETER) else {
        return (RenderScale::FULL, None);
    };
    let Ok(asked) = value.parse::<f32>() else {
        return (
            RenderScale::FULL,
            Some(format!(
                "[jidousha] ?{PARAMETER}={value} is not a number, so the page is \
                 rendering at full resolution\n  \
                 likely cause: a typo in the page URL\n  \
                 fix: pass a fraction between {FLOOR} and {CEILING}, e.g. \
                 ?{PARAMETER}=0.5"
            )),
        );
    };
    if !asked.is_finite() || asked <= 0.0 {
        return (
            RenderScale::FULL,
            Some(format!(
                "[jidousha] ?{PARAMETER}={value} is not a positive fraction, so the \
                 page is rendering at full resolution\n  \
                 likely cause: a negative, zero or infinite value in the page URL\n  \
                 fix: pass a fraction between {FLOOR} and {CEILING}, e.g. \
                 ?{PARAMETER}=0.5"
            )),
        );
    }
    let clamped = asked.clamp(FLOOR, CEILING);
    if clamped == asked {
        return (RenderScale(clamped), None);
    }
    (
        RenderScale(clamped),
        Some(format!(
            "[jidousha] ?{PARAMETER}={value} is outside the range this build \
             accepts, so the page is rendering at {clamped} instead\n  \
             likely cause: asking for fewer pixels than a picture survives, or \
             for more than the display has\n  \
             fix: pass a fraction between {FLOOR} and {CEILING}, e.g. \
             ?{PARAMETER}=0.5"
        )),
    )
}

/// The scale this run renders at, read from the page URL once at startup.
///
/// Anything wrong with the parameter is reported where a playtester can see it
/// — `report::problem` is the browser console, and the playtest page puts a
/// `[jidousha] ` line on its status bar (web-publish.md §2).
#[cfg(target_arch = "wasm32")]
pub(crate) fn requested() -> RenderScale {
    let Some(window) = web_sys::window() else {
        return RenderScale::FULL;
    };
    let Ok(search) = window.location().search() else {
        return RenderScale::FULL;
    };
    let (scale, problem) = from_query(&search);
    if let Some(problem) = problem {
        crate::report::problem(&problem);
    }
    scale
}

/// Always full resolution: a native run has no page URL to ask.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn requested() -> RenderScale {
    RenderScale::FULL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_that_asks_for_nothing_renders_every_device_pixel() {
        assert_eq!(from_query(""), (RenderScale::FULL, None));
        assert_eq!(from_query("?frametime=1"), (RenderScale::FULL, None));
    }

    #[test]
    fn half_scale_halves_both_dimensions_of_the_surface() {
        let (scale, problem) = from_query("?frametime=1&renderscale=0.5");
        assert_eq!(problem, None);
        assert_eq!(
            scale.apply(PhysicalSize::new(1920, 1080)),
            PhysicalSize::new(960, 540)
        );
    }

    #[test]
    fn scaling_the_surface_leaves_the_aspect_ratio_the_letterbox_is_built_on() {
        // games/giri/UI.md §6: the view scales uniformly and letterboxes on the
        // short-fall axis. That contract is a function of the aspect ratio, so
        // the aspect ratio is the thing a render scale must not move.
        let full = PhysicalSize::new(1600, 900);
        for query in ["?renderscale=0.25", "?renderscale=0.5", "?renderscale=1"] {
            let (scale, _) = from_query(query);
            let scaled = scale.apply(full);
            assert!(
                (scaled.aspect() - full.aspect()).abs() < 1e-3,
                "{query} moved the aspect ratio: {} vs {}",
                scaled.aspect(),
                full.aspect()
            );
        }
    }

    #[test]
    fn a_pointer_and_the_surface_are_scaled_by_the_same_factor() {
        // The failure this rules out: a click that lands somewhere other than
        // where it looks like it lands. The pointer is read against the
        // viewport (`Camera::screen_to_world`), so the two have to move
        // together or neither.
        let (scale, _) = from_query("?renderscale=0.5");
        let surface = scale.apply(PhysicalSize::new(1920, 1080));
        let corner = 1920.0 * scale.factor();
        assert_eq!(surface.width as f32, corner);
    }

    #[test]
    fn a_hidden_canvas_stays_zero_sized_rather_than_becoming_one_pixel() {
        let (scale, _) = from_query("?renderscale=0.5");
        assert_eq!(
            scale.apply(PhysicalSize::new(0, 0)),
            PhysicalSize::new(0, 0)
        );
    }

    #[test]
    fn a_surface_too_small_to_halve_keeps_at_least_one_pixel() {
        let (scale, _) = from_query("?renderscale=0.25");
        assert_eq!(
            scale.apply(PhysicalSize::new(1, 3)),
            PhysicalSize::new(1, 1)
        );
    }

    #[test]
    fn a_scale_below_the_floor_is_clamped_and_says_so() {
        let (scale, problem) = from_query("?renderscale=0.05");
        assert_eq!(scale, RenderScale(FLOOR));
        let problem = problem.expect("clamping is not allowed to be silent");
        assert!(problem.starts_with("[jidousha] ?renderscale=0.05"));
        assert!(problem.contains("likely cause:"));
        assert!(problem.contains("fix:"));
    }

    #[test]
    fn a_scale_above_one_is_clamped_to_one_and_says_so() {
        let (scale, problem) = from_query("?renderscale=4");
        assert_eq!(scale, RenderScale::FULL);
        assert!(problem.is_some(), "clamping is not allowed to be silent");
    }

    #[test]
    fn a_scale_that_is_not_a_number_renders_full_size_and_says_so() {
        for query in ["?renderscale=banana", "?renderscale="] {
            let (scale, problem) = from_query(query);
            assert_eq!(scale, RenderScale::FULL, "{query}");
            let problem = problem.unwrap_or_else(|| panic!("{query} was refused silently"));
            assert!(problem.contains("fix:"), "{query}");
        }
    }

    #[test]
    fn a_scale_that_is_not_a_positive_fraction_renders_full_size_and_says_so() {
        for query in ["?renderscale=0", "?renderscale=-0.5", "?renderscale=inf"] {
            let (scale, problem) = from_query(query);
            assert_eq!(scale, RenderScale::FULL, "{query}");
            assert!(problem.is_some(), "{query} was refused silently");
        }
    }

    #[test]
    fn only_a_parameter_named_exactly_render_scale_is_read() {
        assert_eq!(from_query("?norenderscale=0.5"), (RenderScale::FULL, None));
        assert_eq!(from_query("?renderscaled=0.5"), (RenderScale::FULL, None));
    }
}
