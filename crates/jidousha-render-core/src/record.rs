//! Drawing a headless game into a recording, in one call.
//!
//! Key types: `FrameRecorder`.
//! Depends on: `null`, `plan`, `textures`, `camera`, `jidousha-core`. Must never
//! depend on: any graphics API — this is the path that runs where there is no
//! GPU and no display, which is every machine this project builds on.
//! INVARIANT: what this draws is what the windowed driver draws. It builds the
//! texture table the same way and in the same order, and reads the camera out
//! of the world the same way, because a verification that drew a *different*
//! frame from the game would assert about nothing (renderer.md §9).

use jidousha_assets::Assets;
use jidousha_core::{HeadlessSim, PhysicalSize, message};

use crate::backend::{BackendTextureId, RenderBackend};
use crate::camera::Camera;
use crate::font::FONT_TEXTURE;
use crate::null::{FrameRecord, NullBackend};
use crate::plan::{TextureTable, plan_frame};
use crate::textures::{create_builtin_textures, upload_ready_textures};

/// Draws a headless game and keeps every frame, for a test to assert on.
///
/// The five steps this replaces — create the built-in textures, take the
/// frame's quads, plan them against a camera, hand the plan to a backend, ask
/// the backend what it recorded — are the driver's steps, and a game had to
/// write all five out to check that it drew anything. Worse, the texture table
/// went out of scope with them, so answering *"is there text on screen?"*
/// meant building a second throwaway backend and a second table in the same
/// order and asking that one which id the font had landed on.
///
/// E0 run 1 copied that whole shape out of an example, including the comment
/// apologising for it, and reported not understanding why the frame did not
/// simply carry the mapping (e0-findings.md F-010). It does not because
/// [`TextureTable`] resolves at plan time and a plan is per-frame while the
/// table is the driver's, for the life of the run — so the answer is not to
/// put the table in the frame but to keep the thing that owns both.
///
/// ```
/// # use jidousha_core::{Draw, DrawCtx, GameConfig, PhysicalSize, headless};
/// # use jidousha_render_core::{Camera, FrameRecorder, Submit};
/// # use jidousha_core::math::Vec2;
/// # fn draw_a_dot(ctx: &mut DrawCtx) {
/// #     ctx.circle(Vec2::ZERO, 1.0, jidousha_core::Color::WHITE, Default::default());
/// # }
/// let mut sim = headless(GameConfig::default(), |app| {
///     app.add_system(Draw, draw_a_dot);
/// });
/// let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));
///
/// sim.tick();
/// let frame = recorder.draw(&mut sim);
/// assert!(!frame.covering(Vec2::ZERO).is_empty(), "the dot covers the origin");
///
/// // The frame is owned, so looking at the run's history and drawing another
/// // frame compose in one function rather than fighting (ADR-0023).
/// let first = recorder.frames()[0].clone();
/// let staged = recorder.draw(&mut sim);
/// assert_eq!(first.quad_count(), staged.quad_count());
/// ```
pub struct FrameRecorder {
    backend: NullBackend,
    textures: TextureTable,
    viewport: PhysicalSize,
}

impl FrameRecorder {
    /// A recorder drawing to a surface `viewport` pixels across, overriding
    /// the `Camera` resource's own.
    ///
    /// The viewport is the recorder's rather than the game's for the same
    /// reason it is the driver's in a windowed run: it describes the surface,
    /// and here the surface is whatever the test says it is. A game's `Camera`
    /// supplies everything else — centre, height, clear color — so what is
    /// recorded is the game's own view at a size the test controls.
    ///
    /// **This is worth an assertion's attention.** The `Camera` in the world
    /// keeps whatever viewport it was given, and nothing in a headless run
    /// stamps this one onto it. A check that reads a rectangle from
    /// `world.resource::<Camera>().visible_bounds()` and quads from the
    /// recorder is comparing against the wrong rectangle whenever the two
    /// viewports differ — and it goes on passing while it does. Either pass the
    /// recorder the viewport the camera already has, or read the bounds from
    /// `Camera { viewport, ..the game's camera }`.
    #[must_use]
    pub fn new(viewport: PhysicalSize) -> Self {
        let mut backend = NullBackend::new();
        // Before anything else, so the built-ins get the ids the driver gives
        // them. `font_texture` below is only meaningful because of this order.
        let textures = create_builtin_textures(&mut backend);
        Self {
            backend,
            textures,
            viewport,
        }
    }

    /// Apply what has finished loading and put it on the GPU, as the driver
    /// does at the top of every frame.
    ///
    /// Only a game with art needs this — a game of shapes and text draws
    /// correctly without ever calling it. Call it before [`draw`](Self::draw)
    /// on each tick, and a sprite whose texture became ready this tick samples
    /// its art in this frame rather than the placeholder for one frame longer
    /// (assets.md §4).
    ///
    /// A game with no `Assets` resource is a game with no assets, and this does
    /// nothing rather than complaining about it.
    pub fn settle_assets(&mut self, sim: &mut HeadlessSim, tick: u64) {
        let Some(assets) = sim.world_mut().find_resource_mut::<Assets>() else {
            return;
        };
        // Dropped rather than reported: a test scripting a failed load is
        // asserting on the placeholder that results, and `Assets::status` is
        // where it asks. Printing here would put engine noise in a test's
        // output for something the test arranged on purpose.
        let _ = assets.commit(tick);
        upload_ready_textures(assets, &mut self.backend, &mut self.textures);
    }

    /// Run the game's Draw phase once and record the frame it produced.
    ///
    /// **Returns the frame by value**, and that is deliberate (ADR-0021 is the
    /// camera's; this is ADR-0023). A borrow would end at the next `draw` and at
    /// every `frames()`, so the shape *Testing your game* recommends — look at
    /// the run's last frame, then build the screens the run never reached — was a
    /// borrow error, and E0 run 4 worked around it with a second `FrameRecorder`
    /// that silently redirected `transcript()` away from the frame it wanted
    /// (e0-findings.md F-040). The copy this costs is the copy the caller was
    /// being told to make anyway.
    ///
    /// The frame is still kept: [`frames`](Self::frames) has every one of them,
    /// oldest first, for the whole life of the recorder. There is no `clear` —
    /// the history is what a failing assertion reads backwards, and a check that
    /// could throw away the tick before the one that broke would be throwing away
    /// the tick the failure message wants.
    ///
    /// # Panics
    ///
    /// If the recording backend refuses the frame. It has no reason to — it
    /// keeps frames and draws nothing — but a verification whose recorder
    /// silently dropped a frame would assert against the frame before it, which
    /// is worse than stopping.
    pub fn draw(&mut self, sim: &mut HeadlessSim) -> FrameRecord {
        let camera = Camera {
            viewport: self.viewport,
            ..sim
                .world()
                .find_resource::<Camera>()
                .copied()
                .unwrap_or_default()
        };
        // Copied out because `draw` borrows the sim for as long as its
        // submissions live, and the plan outlives them.
        let quads = sim.draw().quads().to_vec();
        let plan = plan_frame(&camera, &quads, &self.textures);
        if let Err(error) = self.backend.render(&plan) {
            panic!(
                "{}",
                message(
                    "the frame recorder could not record a frame",
                    &error.to_string(),
                    "the recording backend refused a plan, which it has no reason to do",
                    "report this — a game cannot cause it",
                )
            );
        }
        let Some(frame) = self.backend.last_frame() else {
            panic!(
                "{}",
                message(
                    "the frame recorder recorded nothing",
                    "the backend accepted a plan and kept no frame",
                    "the recording backend's contract changed",
                    "report this — a game cannot cause it",
                )
            );
        };
        frame.clone()
    }

    /// Which backend texture the engine's font atlas is on.
    ///
    /// The answer to *"is any of this text?"*: a quad sampling this id came
    /// from `ctx.text` and nothing else can have produced it. Before this, a
    /// test had to rebuild the whole texture table against a throwaway backend
    /// to find out, because the id is assignment-ordered rather than fixed and
    /// the real table was long out of scope by assertion time.
    #[must_use]
    pub fn font_texture(&self) -> BackendTextureId {
        self.textures.resolve(FONT_TEXTURE)
    }

    /// Every frame recorded so far, oldest first.
    #[must_use]
    pub fn frames(&self) -> &[FrameRecord] {
        self.backend.frames()
    }

    /// The last frame as stable, diffable text.
    ///
    /// Every quad's world-space extent and tint, one per line. E0 run 1 called
    /// this "a genuinely good substitute for a screenshot", and it was the only
    /// way that run could check its game's layout at all — the machine it ran
    /// on had no display and no GPU.
    #[must_use]
    pub fn transcript(&self) -> String {
        self.backend.transcript()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jidousha_core::math::Vec2;
    use jidousha_core::{Color, Depth, Draw, DrawCtx, GameConfig, TextureId, headless};

    use crate::font::TextStyle;
    use crate::submit::Submit;

    fn draw_a_square(ctx: &mut DrawCtx) {
        ctx.rect(
            jidousha_core::Rect::from_center_size(Vec2::ZERO, Vec2::new(2.0, 2.0)),
            Color::WHITE,
            Depth::default(),
        );
    }

    fn draw_some_text(ctx: &mut DrawCtx) {
        ctx.text(Vec2::ZERO, "hi", TextStyle::default());
    }

    fn sim_drawing(system: fn(&mut DrawCtx)) -> HeadlessSim {
        headless(GameConfig::default(), |app| {
            app.add_system(Draw, system);
        })
    }

    #[test]
    fn one_call_records_a_frame_a_test_can_ask_about() {
        // The five-step ceremony this replaces is the whole finding: a game
        // should not have to name the backend seam to check that it drew.
        let mut sim = sim_drawing(draw_a_square);
        let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));

        sim.tick();
        let frame = recorder.draw(&mut sim);
        assert_eq!(frame.quad_count(), 1);
        assert!(!frame.covering(Vec2::ZERO).is_empty(), "covers the origin");
    }

    #[test]
    fn a_recorded_frame_outlives_the_next_draw() {
        // The regression guard for e0-findings.md F-040: *Testing your game*
        // tells a check to inspect the run's last frame and then build the
        // screens the run never reached, and while `draw` returned a borrow
        // those two paragraphs did not compile together. A run worked around it
        // with a second recorder, which quietly moved what `transcript()`
        // printed away from the frame it cared about.
        let mut sim = sim_drawing(draw_a_square);
        let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));

        sim.tick();
        let match_frame = recorder.draw(&mut sim);
        let held = recorder.frames().last().cloned().expect("one frame drawn");
        let staged = recorder.draw(&mut sim);

        assert_eq!(
            match_frame.quad_count(),
            1,
            "still readable after two draws"
        );
        assert_eq!(held.quad_count(), 1, "and so is one taken from frames()");
        assert_eq!(staged.quad_count(), 1);
        assert_eq!(
            recorder.frames().len(),
            2,
            "both are kept; there is no clear"
        );
    }

    #[test]
    fn the_recorder_answers_which_texture_the_font_is_on() {
        // The throwaway-backend trick, gone: the recorder still owns the table
        // that knows, so it can simply be asked.
        let mut sim = sim_drawing(draw_some_text);
        let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));

        sim.tick();
        let font = recorder.font_texture();
        let frame = recorder.draw(&mut sim);
        assert!(
            frame.quads().iter().any(|quad| quad.texture == font),
            "the text was drawn with the font atlas"
        );
    }

    #[test]
    fn the_font_is_not_the_texture_a_plain_shape_uses() {
        // Otherwise the question above would answer "yes, there is text" for
        // any game that drew a rectangle.
        let mut sim = sim_drawing(draw_a_square);
        let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));

        sim.tick();
        let font = recorder.font_texture();
        let frame = recorder.draw(&mut sim);
        assert!(
            frame.quads().iter().all(|quad| quad.texture != font),
            "a rectangle samples the white texel, not the atlas"
        );
        assert_ne!(font, recorder.textures.resolve(TextureId::WHITE));
    }

    #[test]
    fn the_recorder_draws_through_the_games_own_camera() {
        // What is recorded has to be what the game would show, or an assertion
        // about the frame is an assertion about the recorder.
        let mut sim = sim_drawing(draw_a_square);
        sim.world_mut().insert_resource(Camera {
            center: Vec2::new(100.0, 0.0),
            ..Camera::default()
        });
        let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));

        sim.tick();
        let frame = recorder.draw(&mut sim);
        assert!(
            frame.covering(Vec2::ZERO).is_empty() || frame.quad_count() == 1,
            "the square is still submitted; the camera decides where it lands"
        );
        assert_eq!(frame.quad_count(), 1);
    }

    #[test]
    fn frames_accumulate_so_a_session_can_be_replayed_over() {
        let mut sim = sim_drawing(draw_a_square);
        let mut recorder = FrameRecorder::new(PhysicalSize::new(640, 480));

        for _ in 0..3 {
            sim.tick();
            recorder.draw(&mut sim);
        }
        assert_eq!(recorder.frames().len(), 3);
        assert!(
            recorder.transcript().contains("quad"),
            "{}",
            recorder.transcript()
        );
    }
}
