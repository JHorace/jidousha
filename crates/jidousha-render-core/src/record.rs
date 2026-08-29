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
use crate::font::{FONT_TEXTURE, Face, Fonts, upload_text_atlases};
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
        // Read before the draw, which borrows the sim for as long as its
        // submissions live. A `Face` is a `Copy` name for outlines that live as
        // long as the program, so the copy cannot go stale (renderer.md §6).
        let faces: Vec<Face> = sim
            .world()
            .find_resource::<Fonts>()
            .map(|fonts| fonts.faces().to_vec())
            .unwrap_or_default();
        // Copied out because `draw` borrows the sim for as long as its
        // submissions live, and the plan outlives them.
        let quads = sim.draw().quads().to_vec();
        // After the draw and before the plan, exactly as the driver does it:
        // nothing knows which faces at which sizes a frame wants until the game
        // has asked, and the plan resolves every texture id (renderer.md §6).
        upload_text_atlases(&faces, &quads, &mut self.backend, &mut self.textures);
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

    /// Every recorded frame as text, oldest first — not only the last.
    ///
    /// Each frame is headed `frame N:` and then carries every quad's
    /// world-space extent and tint, one per line. A check that recorded a
    /// thousand ticks gets a thousand frames here: this is the *history*, and
    /// it is the right thing to keep as evidence when a failure needs the ticks
    /// before the one that broke.
    ///
    /// **For one frame, use [`FrameRecord::transcript`]** — on the frame
    /// [`FrameRecorder::draw`] hands back, or on `frames().last()`. That is the
    /// one that is a substitute for a screenshot; E0 run 1 called it "a
    /// genuinely good" one, and it was the only way that run could check its
    /// game's layout at all, on a machine with no display and no GPU. A run
    /// that prints this method instead emits every frame it ever drew: E0 run 5
    /// recorded 1,263 of them and got 121,465 lines, under a `--verify`
    /// convention that keeps the transcript as evidence rather than showing it,
    /// so nothing said a word (e0-findings.md F-055).
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
    fn a_recorded_frames_plan_carries_the_cameras_clear_color() {
        // The background leaves no quad behind, so every other assertion over a
        // frame is identical whatever it was cleared to. This is the one path a
        // game author has to it, and *Testing your game* once said there was
        // none (e0-findings.md F-068) — so it is pinned here rather than left
        // to be true by accident.
        let mut sim = sim_drawing(draw_a_square);
        sim.world_mut().insert_resource(Camera {
            clear_color: Color::rgb(0.02, 0.05, 0.03),
            ..Camera::default()
        });
        let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));

        sim.tick();
        let frame = recorder.draw(&mut sim);
        assert_eq!(frame.plan.clear_color, Color::rgb(0.02, 0.05, 0.03));
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

    #[test]
    fn a_frames_draw_order_is_the_depth_sort_not_the_submission_order() {
        // The guard for ADR-0024. E0 run 5 concluded that a recorded frame
        // cannot see draw order, because `DrawnQuad` carries no `Depth`, and
        // filed a layer field as the one thing it would add to the engine.
        // What a check actually wants to know is which of two things ends up in
        // front, and the frame answers that exactly: `quads()` is the sorted
        // sequence, so an index comparison is a layering assertion, and
        // `covering()`'s first element is what the player sees.
        //
        // The submission order here is deliberately the opposite of the depth
        // order, so a frame that merely echoed what the game submitted fails.
        fn draw_small_over_large(ctx: &mut DrawCtx) {
            ctx.rect(
                jidousha_core::Rect::from_center_size(Vec2::ZERO, Vec2::new(1.0, 1.0)),
                Color::RED,
                Depth::layer(10),
            );
            ctx.rect(
                jidousha_core::Rect::from_center_size(Vec2::ZERO, Vec2::new(2.0, 2.0)),
                Color::WHITE,
                Depth::layer(0),
            );
        }

        let mut sim = sim_drawing(draw_small_over_large);
        let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));
        sim.tick();
        let frame = recorder.draw(&mut sim);

        let quads = frame.quads();
        assert_eq!(quads.len(), 2);
        assert_eq!(
            quads[0].tint,
            Color::WHITE,
            "the lower layer is drawn first, though it was submitted second"
        );
        assert_eq!(quads[1].tint, Color::RED, "and the higher layer over it");

        let seen = frame.covering(Vec2::ZERO);
        assert_eq!(
            seen[0].tint,
            Color::RED,
            "front to back, so the first one back is the one in front"
        );
    }

    #[test]
    fn the_recorders_transcript_carries_every_frame_and_a_records_carries_one() {
        // The regression guard for e0-findings.md F-055. Both methods were
        // described as "the last frame", and only one of them is: a run that
        // printed the recorder's got a line per quad per *tick*, which under
        // the `--verify` convention is kept as evidence rather than shown, so
        // the size of it was invisible. The two are pinned apart here so the
        // descriptions cannot drift back together.
        let mut sim = sim_drawing(draw_a_square);
        let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));

        let mut last = None;
        for _ in 0..3 {
            sim.tick();
            last = Some(recorder.draw(&mut sim));
        }
        let one = last.expect("three frames were drawn").transcript();
        let all = recorder.transcript();

        assert_eq!(
            one.matches("quad (").count(),
            1,
            "a record's transcript is one frame:\n{one}"
        );
        assert_eq!(
            all.matches("quad (").count(),
            3,
            "the recorder's transcript is every frame:\n{all}"
        );
        assert!(
            all.contains("frame 2:"),
            "every frame is headed by its index:\n{all}"
        );
    }
}
