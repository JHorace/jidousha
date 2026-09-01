//! One frame: settle the assets, run the ticks the elapsed time bought, draw.
//!
//! Key types: none — this is `Driver`'s frame half, split from `mod.rs` because
//! together they are twice the length a file should be (CLAUDE.md).
//! Depends on: `jidousha-core`, `jidousha-assets`, `jidousha-render-core`.
//! INVARIANT: no `winit` type appears here at all. That is not an accident of
//! the split — it is why the split falls where it does, and it is what lets the
//! tests below run a whole frame on a machine with no display and no GPU.

use jidousha_assets::Assets;
use jidousha_core::{Quad, Seconds, Time, World};
use jidousha_input::Input;
use jidousha_render_core::{
    Camera, Fonts, PhysicalSize, overlay::draw_readout, plan_frame, upload_ready_textures,
    upload_text_atlases,
};

use super::Driver;
use super::overlay::{Engine, Phase, Spans};

impl Driver {
    /// One rendered frame: catch simulation up to `elapsed` of real time, then
    /// draw once.
    ///
    /// This is core.md §7's loop shape, and the only place it is spelled out
    /// for the windowed path — the accumulator itself belongs to `Simulation`.
    ///
    /// The duration is an argument rather than read from the clock here, which
    /// separates *when* a frame happens from *how long* it was. winit decides
    /// the first; the clock answers the second; and the tests below can answer
    /// it themselves, which is how this logic is checked without a window.
    pub(super) fn frame(&mut self, elapsed: Seconds) {
        // The **previous** frame's breakdown, closed by this frame arriving:
        // `elapsed` runs from that frame's start to now, so it is that frame's
        // whole duration and the only total its four measured spans can be
        // subtracted from without inventing anything. The breakdown is one
        // frame behind the pacing readings beside it, which at four repaints a
        // second nobody can see (frame-pacing.md §7).
        let spans = core::mem::replace(&mut self.spans, Spans::new());
        self.overlay.close_frame(elapsed, spans);

        self.settle_assets();

        // The frame's events belong to its first tick; the catch-up ticks
        // behind it see the state those events left, with no edges (input.md
        // §2).
        //
        // The snapshot is taken *inside* the callback, which matters more than
        // it looks: `first_tick_snapshot` spends the builder's edges, and a
        // frame that runs no ticks must not spend them. A machine drawing
        // faster than it ticks has such frames constantly, and taking the
        // snapshot before `advance` decided anything dropped every press that
        // landed in one. Found by the test below, which is why it is there.
        // DELIBERATE: the `index == 0` split below is not currently observable
        // — `first_tick_snapshot` spends the builder's edges, so calling it on
        // every tick would produce what `catch_up_snapshot` produces from the
        // second tick on. Mutation testing says so, and the honest reading is
        // that the code states the contract the *builder* guarantees rather
        // than one this function enforces. It is written out anyway: the day a
        // snapshot depends on anything beyond the drained edges, this is the
        // line that was already right (input.md §2).
        let Self {
            simulation,
            input,
            backend,
            textures,
            viewport,
            faces,
            pacing,
            overlay,
            clock,
            spans,
            ..
        } = self;
        // Four extra clock reads a frame, and only when the sections that use
        // them are on: `since_frame` does not move the frame's mark, so
        // splitting a frame this way cannot spend it (clock.rs). The asset
        // commit and the texture uploads it does are `Encode` — building this
        // frame's picture — even though they ran first; a span is a sum, not a
        // position in the frame.
        let timed = overlay.wants_phases();
        let mark = |phase: Phase, spans: &mut Spans| {
            if timed {
                spans.spent(phase, clock.since_frame());
            }
        };
        mark(Phase::Encode, spans);

        let ticks = simulation.advance(elapsed, |world: &mut World, index| {
            let snapshot = if index == 0 {
                input.first_tick_snapshot()
            } else {
                input.catch_up_snapshot()
            };
            world.insert_resource(Input::new(snapshot));
        });
        mark(Phase::Sim, spans);

        // Read before drawing, which is both what the borrow checker wants and
        // what the phase means: Draw cannot change the world (ADR-0008), so the
        // camera it draws with is the one the last Update tick left.
        //
        // The camera is the game's to set; a game that never inserts one gets
        // the default rather than a panic, because "I have not thought about
        // the camera yet" is a real state for a prototype to be in.
        //
        // The *viewport* is not the game's, though, and is stamped on here
        // every frame rather than only when a resize arrives. It describes the
        // window, so the driver is the only thing that knows it — and every
        // route by which a game ends up holding a stale one is ordinary:
        // `resumed` measures the window before Startup has inserted a camera to
        // write it to, and a game that builds its camera with
        // `..Camera::default()` overwrites whatever was written with 1280x720.
        // Both left games drawing at the wrong aspect ratio until the player
        // resized the window, and neither said anything (e0-findings.md F-012).
        let world = simulation.world_mut();
        if world.find_resource::<Camera>().is_none() {
            world.insert_resource(Camera::default());
        }
        let camera = {
            let camera = world.resource_mut::<Camera>();
            camera.viewport = *viewport;
            *camera
        };

        // Which faces a game has is world state, so it is read here — after
        // the ticks that could have created one, and before the draw that
        // borrows the simulation. Copied rather than borrowed, and only when
        // the count changes: `Face` is a name for outlines that live as long as
        // the program (renderer.md §6), so a stale copy cannot dangle and a
        // frame that loaded nothing new allocates nothing.
        if let Some(fonts) = simulation.world().find_resource::<Fonts>()
            && fonts.faces().len() != faces.len()
        {
            faces.clear();
            faces.extend_from_slice(fonts.faces());
        }

        // The world's own counters, read here rather than after the draw for
        // one reason the borrow checker only echoes: the Draw phase cannot
        // change the world (ADR-0008), so this reading and one taken after
        // `draw` are the same numbers — and `draw` borrows the simulation the
        // world lives in. Read-only, through `World`'s ordinary read paths, and
        // never written back (frame-pacing.md §7).
        let world_counters = if overlay.wants_phases() {
            let world = simulation.world();
            (world.entity_count(), world.component_count())
        } else {
            (0, 0)
        };

        // Draw once per frame, however many ticks ran — including none
        // (core.md §7).
        let submissions = simulation.draw();
        mark(Phase::Draw, spans);

        let (Some(backend), Some(textures)) = (backend, textures.as_mut()) else {
            return;
        };

        // How this frame reached the display, asked of the backend rather than
        // assumed: it is what paces the *next* frame (frame-pacing.md §6) and
        // what the overlay's pacing line prints. A backend with no device yet
        // says `Offscreen`, which caps nothing — a startup polling for a GPU
        // must not be slowed down.
        let presentation = backend.presentation();
        pacing.observe(presentation);
        // What the engine is holding, from the one place that can answer each
        // half: the backend's own running totals across the seam, and the
        // world's counters read a moment ago. Every one of them is a total
        // something already maintains, so this is a read rather than a walk
        // (renderer.md §12a).
        overlay.observe(Engine {
            backend: backend.stats(),
            entities: world_counters.0,
            components: world_counters.1,
            quads: submissions.quads().len(),
        });
        // The frame's reading, taken from numbers the driver already had: the
        // duration the accumulator was given and the tick count it returned.
        // No clock is read for this, so an overlay that is off costs a branch.
        overlay.record(elapsed, ticks, presentation);

        // After the draw and before the plan, which is the only window there
        // is: nothing knows which faces at which sizes a frame wants until the
        // game has asked for them, and `plan_frame` resolves every texture id
        // to a backend id, so an atlas uploaded after it would be resolved to
        // the placeholder for a frame (renderer.md §5, §6).
        upload_text_atlases(faces, submissions.quads(), backend.as_mut(), textures);

        // The overlay is appended *after* the Draw phase closed, to a copy the
        // world never sees. That is the whole of its presentation-only promise:
        // a game's submissions are what the game submitted, byte for byte,
        // whether the overlay is on or off — so a recorded transcript and a
        // replay are identical either way (frame-pacing.md §6). The copy is
        // made only on the frames that draw one.
        let with_overlay: Option<Vec<Quad>> = overlay.is_on().then(|| {
            let mut quads = submissions.quads().to_vec();
            draw_readout(&camera, overlay.readout(), &mut quads);
            quads
        });
        let quads = match &with_overlay {
            Some(quads) => quads.as_slice(),
            None => submissions.quads(),
        };
        let plan = plan_frame(&camera, quads, textures);
        mark(Phase::Encode, spans);
        if let Err(error) = backend.render(&plan) {
            // A frame that cannot be drawn is not a reason to stop: the surface
            // usually comes back. Saying so once per occurrence beats silence
            // and beats quitting.
            crate::report::problem(&error.to_string());
        }
        // `render` is where the loop waits for the display, and the seam is
        // where the driver's reach stops — so this one span is the encode and
        // the present-wait together, and the panel's doc section says which
        // dominates it and when (phases.rs, frame-pacing.md §7).
        mark(Phase::Present, spans);
    }

    /// Apply what finished loading, and put it on the GPU.
    ///
    /// CONTRACT (assets.md §4): the commit happens **before the frame's first
    /// Update tick**, so every tick of the frame sees one consistent picture of
    /// what is ready. Load timing is environmental — disk speed, cache, network
    /// — and one commit point per frame is what keeps it off the simulation's
    /// timeline.
    ///
    /// The upload follows immediately, so a sprite drawn this frame samples art
    /// that became ready this frame rather than showing the placeholder for one
    /// frame longer than it has to.
    ///
    /// A game with no `Assets` resource is a game with no assets, which is a
    /// perfectly ordinary thing for a prototype to be.
    pub(super) fn settle_assets(&mut self) {
        let Self {
            simulation,
            backend,
            textures,
            ..
        } = self;
        // Startup first, because that is where a game builds its `Assets` and
        // asks for its art. Without this the first frame would find no store to
        // commit and the first loads would wait a frame for no reason. `start`
        // is idempotent, and `advance` below calls it again to no effect.
        simulation.start();

        // The tick the frame starts from. `commit` moves forward only, and this
        // is the same number the frame's first tick will be counted from.
        let tick = simulation.world().resource::<Time>().tick;
        let Some(assets) = simulation.world_mut().find_resource_mut::<Assets>() else {
            return;
        };
        for failure in assets.commit(tick) {
            // Once per asset, not once per frame: the placeholder does the
            // per-frame signalling from here on (assets.md §6). Through
            // `report` rather than `eprintln!`, because on the web there is no
            // stderr and this is the commonest thing a web build gets wrong.
            crate::report::problem(&failure.message());
        }
        let (Some(backend), Some(textures)) = (backend, textures.as_mut()) else {
            // No GPU yet. The store keeps the texels until there is one, which
            // is why nothing is lost by a window that arrives late.
            return;
        };
        upload_ready_textures(assets, backend.as_mut(), textures);
    }

    /// Tell the driver how big the window is now.
    ///
    /// The camera's viewport is driver-maintained (renderer.md §4): it
    /// describes the window, and a game that set it would be lying to itself
    /// about how big the window is.
    ///
    /// This records the size and reconfigures the surface; the camera is
    /// stamped in `frame`, because that is the only moment a camera is
    /// guaranteed to exist to stamp. Writing it here as well would be a second
    /// way to do one thing, and the one that has already been observed to miss.
    pub(super) fn resize(&mut self, size: PhysicalSize) {
        self.viewport = size;
        if let Some(backend) = &mut self.backend {
            backend.resize_surface(size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::overlay::{Level, Overlay};
    use super::super::testing::*;
    use super::*;
    use jidousha_assets::{AssetStatus, MemorySource};
    use jidousha_core::{Color, Depth, Draw, DrawCtx, Rect, Transform, math::Vec2};
    use jidousha_input::{InputEvent, Key};
    use jidousha_render_core::{
        FONT_TEXTURE, NullBackend, Presentation, Sprite, Submit, draw_sprites,
    };

    #[test]
    fn a_frames_edges_go_to_its_first_tick_and_no_other() {
        // input.md §2's CONTRACT, where the driver is the thing that could
        // break it: three catch-up ticks must not fire one jump three times.
        let mut driver = driver();
        driver.input.record(InputEvent::KeyPressed(Key::A));
        driver.frame(frames_worth(3));

        let seen = driver.simulation.world().resource::<Seen>();
        assert_eq!(seen.pressed, vec![true, false, false], "one press edge");
        assert_eq!(seen.held, vec![true, true, true], "still down throughout");
    }

    #[test]
    fn edges_arriving_between_frames_wait_for_the_next_one() {
        let mut driver = driver();
        driver.frame(frames_worth(1));
        driver.input.record(InputEvent::KeyPressed(Key::A));
        driver.frame(frames_worth(1));

        let seen = driver.simulation.world().resource::<Seen>();
        assert_eq!(seen.pressed, vec![false, true]);
    }

    #[test]
    fn a_frame_too_short_for_a_tick_runs_none_and_keeps_the_edges() {
        // A fast machine draws more often than it ticks. The press must not be
        // spent on a frame that ran no ticks, or it would never be seen at all.
        let mut driver = driver();
        driver.input.record(InputEvent::KeyPressed(Key::A));
        driver.frame(Seconds(0.001));
        assert!(
            driver
                .simulation
                .world()
                .resource::<Seen>()
                .pressed
                .is_empty(),
            "no ticks ran"
        );

        driver.frame(frames_worth(1));
        assert_eq!(
            driver.simulation.world().resource::<Seen>().pressed,
            vec![true],
            "the edge survived to the first tick that ran"
        );
    }

    #[test]
    fn losing_focus_releases_what_was_held() {
        // The alt-tab bug, from the driver's side: the release the builder
        // synthesizes has to reach a tick, or the character keeps running.
        let mut driver = driver();
        driver.input.record(InputEvent::KeyPressed(Key::A));
        driver.frame(frames_worth(1));
        driver.input.record(InputEvent::FocusLost);
        driver.frame(frames_worth(1));

        let seen = driver.simulation.world().resource::<Seen>();
        assert_eq!(seen.released, vec![false, true]);
        assert_eq!(seen.held, vec![true, false], "not still holding it");
    }

    #[test]
    fn a_frame_draws_once_however_many_ticks_ran() {
        // Draw is per frame, not per tick (core.md §7).
        let mut driver = driver();
        driver.frame(frames_worth(4));
        assert_eq!(
            driver.simulation.world().resource::<Seen>().pressed.len(),
            4,
            "four ticks"
        );
        assert_eq!(frames_drawn(&driver), 1, "one frame");
    }

    #[test]
    fn a_frame_that_runs_no_ticks_still_draws() {
        // A machine drawing faster than it ticks must still put a picture up,
        // or it would show the same one until the next tick boundary.
        let mut driver = driver();
        driver.frame(Seconds(0.001));
        assert!(
            driver
                .simulation
                .world()
                .resource::<Seen>()
                .pressed
                .is_empty(),
            "no ticks"
        );
        assert_eq!(frames_drawn(&driver), 1, "drew anyway");
    }

    #[test]
    fn assets_commit_once_a_frame_and_before_the_frames_ticks() {
        // assets.md §4's CONTRACT, where the driver is the thing that could
        // break it. The store is scripted to complete at tick 2, so a tick that
        // saw it Ready would have to have run after the commit that readied it.
        let mut driver = driver();
        let mut source = MemorySource::new();
        source.insert_texture("hero.png", one_texel());
        source.complete_at("hero.png", 2);
        let mut assets = Assets::new(source);
        let hero = assets.load_texture("hero.png");
        driver.simulation.world_mut().insert_resource(assets);

        driver.frame(frames_worth(1)); // commits tick 0, ticks to 1
        assert_eq!(
            driver.simulation.world().resource::<Assets>().status(hero),
            AssetStatus::Loading,
            "tick 0 is before the scripted tick"
        );

        driver.frame(frames_worth(1)); // commits tick 1
        assert_eq!(
            driver.simulation.world().resource::<Assets>().status(hero),
            AssetStatus::Loading
        );

        driver.frame(frames_worth(1)); // commits tick 2, which is when it lands
        assert_eq!(
            driver.simulation.world().resource::<Assets>().status(hero),
            AssetStatus::Ready
        );
    }

    #[test]
    fn a_game_with_no_assets_still_draws() {
        // The store is a resource a game inserts. Most first prototypes have
        // none, and a driver that assumed one would panic on the first frame.
        let mut driver = driver();
        driver.frame(frames_worth(1));
        assert_eq!(frames_drawn(&driver), 1);
    }

    #[test]
    fn texels_wait_in_the_store_until_there_is_a_renderer_to_take_them() {
        // A window arrives a few frames after the program starts, and these
        // tests never get one at all. Nothing may be dropped in the meantime, or
        // art that loaded early would never reach the GPU (renderer.md §5).
        let mut driver = driver();
        let mut source = MemorySource::new();
        source.insert_texture("hero.png", one_texel());
        let mut assets = Assets::new(source);
        let hero = assets.load_texture("hero.png");
        driver.simulation.world_mut().insert_resource(assets);

        for _ in 0..3 {
            driver.frame(frames_worth(1));
        }
        let assets = driver.simulation.world().resource::<Assets>();
        assert_eq!(assets.status(hero), AssetStatus::Ready);
        assert!(
            assets.texture_of(hero).is_some(),
            "still here, waiting for a backend"
        );
    }

    #[test]
    fn a_failed_asset_is_reported_once_and_the_run_goes_on() {
        // A missing file is a fact about the world, not a reason to quit: the
        // renderer draws the placeholder and the game keeps running.
        let mut driver = driver();
        let mut assets = Assets::new(MemorySource::new());
        let missing = assets.load_texture("nowhere.png");
        driver.simulation.world_mut().insert_resource(assets);

        for _ in 0..3 {
            driver.frame(frames_worth(1));
        }
        assert_eq!(
            driver
                .simulation
                .world()
                .resource::<Assets>()
                .status(missing),
            AssetStatus::Failed
        );
        assert_eq!(frames_drawn(&driver), 3, "three frames, none of them fatal");
    }

    #[test]
    fn what_loaded_reaches_the_backend_and_the_frame_that_draws_it() {
        // The whole R2 wiring in one test: commit, upload, register, draw. Every
        // step of it is skippable without any other test noticing, because every
        // other test in this file runs without a backend.
        let (mut driver, backend) = driver_with_a_backend();
        let built_in = backend.read(NullBackend::texture_count);

        let mut source = MemorySource::new();
        source.insert_texture("hero.png", one_texel());
        source.complete_at("hero.png", 2);
        let mut assets = Assets::new(source);
        let hero = assets.load_texture("hero.png");
        driver.simulation.world_mut().insert_resource(assets);
        driver.simulation.add_system(Draw, draw_sprites);

        let entity = driver.simulation.world_mut().spawn();
        driver
            .simulation
            .world_mut()
            .insert(entity, Transform::at(Vec2::ZERO));
        driver
            .simulation
            .world_mut()
            .insert(entity, Sprite::new(hero));

        driver.frame(frames_worth(1));
        assert_eq!(
            backend.read(NullBackend::texture_count),
            built_in,
            "nothing has loaded yet"
        );
        let placeholder = match &driver.textures {
            Some(textures) => textures.placeholder(),
            None => panic!("the table was built with the backend"),
        };
        assert_eq!(drawn_texture(&backend), placeholder);

        driver.frame(frames_worth(1));
        driver.frame(frames_worth(1));
        assert_eq!(
            backend.read(NullBackend::texture_count),
            built_in + 1,
            "the texture reached the GPU"
        );
        assert_ne!(
            drawn_texture(&backend),
            placeholder,
            "and the sprite draws it, not the placeholder"
        );
    }

    /// Which backend texture the one quad of the last recorded frame sampled.
    fn drawn_texture(backend: &SharedBackend) -> jidousha_render_core::BackendTextureId {
        backend.read(|backend| {
            let Some(frame) = backend.last_frame() else {
                panic!("a frame was drawn");
            };
            let quads = frame.quads();
            assert_eq!(quads.len(), 1, "one sprite, one quad");
            quads[0].texture
        })
    }

    #[test]
    fn a_resize_reaches_both_the_surface_and_the_camera() {
        // Two things have to hear about it and one of them is the game's view of
        // the world (renderer.md §4). The surface hears immediately; the camera
        // hears on the next frame, which is the only moment one is sure to
        // exist to be told.
        let (mut driver, backend) = driver_with_a_backend();
        driver
            .simulation
            .world_mut()
            .insert_resource(Camera::default());

        driver.resize(PhysicalSize::new(640, 480));
        assert_eq!(
            backend.read(NullBackend::surface),
            PhysicalSize::new(640, 480)
        );

        driver.frame(frames_worth(1));
        assert_eq!(
            driver.simulation.world().resource::<Camera>().viewport,
            PhysicalSize::new(640, 480)
        );
    }

    #[test]
    fn a_camera_built_in_startup_still_gets_the_windows_real_size() {
        // The bug this replaced a resize-time write to fix. `resumed` measures
        // the window before Startup has run, so there is no camera to write to
        // yet; then the game inserts one with `..Camera::default()`, which
        // carries 1280x720. Nothing failed, nothing warned, and the game drew
        // at the wrong aspect ratio until the player resized the window
        // (e0-findings.md F-012).
        let (mut driver, _backend) = driver_with_a_backend();
        driver.resize(PhysicalSize::new(800, 600));

        // What a game's Startup does, arriving after the size was measured.
        driver
            .simulation
            .world_mut()
            .insert_resource(Camera::default());

        driver.frame(frames_worth(1));
        assert_eq!(
            driver.simulation.world().resource::<Camera>().viewport,
            PhysicalSize::new(800, 600),
            "the driver owns the viewport and says so every frame"
        );
    }

    #[test]
    fn a_game_that_inserts_no_camera_still_draws_at_the_windows_size() {
        // `quickstart.rs` is such a game, and it is the one every author starts
        // as a copy of. The camera was defaulted per frame and thrown away, so
        // the resize had nowhere to land at all.
        let (mut driver, _backend) = driver_with_a_backend();
        driver.resize(PhysicalSize::new(1024, 768));
        driver.frame(frames_worth(1));

        let camera = driver.simulation.world().resource::<Camera>();
        assert_eq!(camera.viewport, PhysicalSize::new(1024, 768));
        assert_eq!(
            camera.height,
            Camera::default().height,
            "everything the game did not ask about is still the default"
        );
    }

    /// A Draw system that puts one recognisable thing on screen.
    fn draw_one_rectangle(ctx: &mut DrawCtx) {
        ctx.rect(
            Rect::from_center_size(Vec2::ZERO, Vec2::splat(2.0)),
            Color::WHITE,
            Depth::layer(0),
        );
    }

    /// What the last frame drew, as `(texture, corners)` pairs in draw order.
    fn drawn(backend: &SharedBackend) -> Vec<(jidousha_render_core::BackendTextureId, [Vec2; 4])> {
        backend.read(|backend| {
            let Some(frame) = backend.last_frame() else {
                panic!("a frame was drawn");
            };
            frame
                .quads()
                .into_iter()
                .map(|quad| (quad.texture, quad.corners))
                .collect()
        })
    }

    #[test]
    fn a_run_that_did_not_ask_for_the_overlay_draws_exactly_what_the_game_submitted() {
        // Off by default, checked where it matters — not "the switch defaults
        // to false" but "the frame is the game's frame". This is the half of
        // the promise a screenshot shows.
        let (mut driver, backend) = driver_with_a_backend();
        driver.simulation.add_system(Draw, draw_one_rectangle);
        driver.frame(frames_worth(1));

        let quads = drawn(&backend);
        assert_eq!(quads.len(), 1, "one rectangle and nothing else: {quads:?}");
        assert!(!driver.overlay.is_on());
    }

    #[test]
    fn the_overlay_draws_over_the_frame_and_leaves_every_quad_under_it_alone() {
        // The other half: switched on, the instrument is *added*. The game's
        // own quad has to come through untouched and in the same place, because
        // an overlay that moved the picture would be diagnosing itself.
        let (mut off, off_backend) = driver_with_a_backend();
        off.simulation.add_system(Draw, draw_one_rectangle);
        off.frame(frames_worth(1));

        let (mut on, on_backend) = driver_with_a_backend();
        on.overlay = Overlay::new(Level::Pacing);
        on.simulation.add_system(Draw, draw_one_rectangle);
        on.frame(frames_worth(1));

        let plain = drawn(&off_backend);
        let with_overlay = drawn(&on_backend);
        assert_eq!(
            with_overlay[..plain.len()],
            plain[..],
            "the game's frame changed when the overlay came on"
        );
        assert!(
            with_overlay.len() > plain.len(),
            "the overlay drew nothing at all"
        );
        let glyphs = on
            .textures
            .as_ref()
            .map(|textures| textures.resolve(FONT_TEXTURE));
        assert!(
            with_overlay[plain.len()..]
                .iter()
                .any(|(texture, _)| Some(*texture) == glyphs),
            "the overlay drew a backdrop and no text"
        );
    }

    #[test]
    fn every_level_of_the_overlay_leaves_the_games_own_frame_byte_identical() {
        // The whole promise, at every level of the switch: a transcript, a
        // replay and a `--verify` run see the same submissions whether the
        // panel is off, showing the pacing readings, or showing the performance
        // sections (frame-pacing.md §7). Level 2 reads the world's counters and
        // times four spans, and neither may move a quad.
        let frame_of = |level| {
            let (mut driver, backend) = driver_with_a_backend();
            driver.overlay = Overlay::new(level);
            driver.simulation.add_system(Draw, draw_one_rectangle);
            for _ in 0..5 {
                driver.frame(frames_worth(1));
            }
            (drawn(&backend), driver)
        };
        let (off, _) = frame_of(Level::Off);
        let (pacing, _) = frame_of(Level::Pacing);
        let (perf, perf_driver) = frame_of(Level::Perf);

        assert_eq!(off.len(), 1, "one rectangle and nothing else: {off:?}");
        assert_eq!(
            pacing[..off.len()],
            off[..],
            "the game's frame changed at level 1"
        );
        assert_eq!(
            perf[..off.len()],
            off[..],
            "the game's frame changed at level 2"
        );
        assert!(
            perf.len() > pacing.len(),
            "level 2 drew no more of a panel than level 1: {} vs {}",
            perf.len(),
            pacing.len()
        );
        assert!(
            perf_driver.overlay.readout().contains("frame breakdown"),
            "{}",
            perf_driver.overlay.readout()
        );
    }

    #[test]
    fn the_performance_panel_reports_the_world_the_frame_actually_drew() {
        // The accounting seam, through the driver: the counters on the panel
        // are this world's, read at draw time, and they move when the world
        // does. A panel wired to the wrong world would look perfectly plausible.
        let (mut driver, _backend) = driver_with_a_backend();
        driver.overlay = Overlay::new(Level::Perf);
        driver.simulation.add_system(Draw, draw_one_rectangle);
        for index in 0..4 {
            let entity = driver.simulation.world_mut().spawn();
            driver
                .simulation
                .world_mut()
                .insert(entity, Transform::at(Vec2::splat(index as f32)));
        }
        // Past the panel's repaint period, which is a quarter second of frames:
        // the numbers are for a person to read, so they are not rebuilt sixty
        // times a second (overlay/mod.rs).
        for _ in 0..20 {
            driver.frame(frames_worth(1));
        }
        let readout = driver.overlay.readout();
        assert!(readout.contains("4 entities"), "{readout}");
        assert!(readout.contains("4 components"), "{readout}");
        // One rectangle is one quad, and the panel's own quads are appended to
        // a copy the world never sees — so they must not be counted here.
        assert!(readout.contains("1 quads drawn"), "{readout}");
    }

    #[test]
    fn the_frame_breakdown_accounts_for_the_whole_frame_it_describes() {
        // The derived bucket, through the driver rather than in isolation: the
        // four measured spans plus sleep are the frame, so a mark the driver
        // forgot to take shows up as sleep rather than as time that vanished.
        let (mut driver, _backend) = driver_with_a_backend();
        driver.overlay = Overlay::new(Level::Perf);
        for _ in 0..8 {
            driver.frame(frames_worth(1));
        }
        let readout = driver.overlay.readout();
        for bucket in ["sim", "draw", "encode", "present", "sleep", "busy"] {
            assert!(readout.contains(bucket), "no {bucket} row: {readout}");
        }
    }

    #[test]
    fn the_overlay_reads_the_tick_count_off_the_frame_it_is_describing() {
        // The reading the web overlay has to model from deltas alone
        // (frame-pacing.md §4). Here it comes back from `Simulation::advance`,
        // and this is the wiring that carries it — a driver that passed the
        // wrong number would produce a plausible, wrong panel.
        let (mut driver, _backend) = driver_with_a_backend();
        driver.overlay = Overlay::new(Level::Pacing);
        driver.frame(frames_worth(3));
        assert!(
            driver.overlay.readout().contains("3+:1"),
            "{}",
            driver.overlay.readout()
        );
    }

    #[test]
    fn the_frame_tells_the_pacer_how_this_frame_reached_the_display() {
        // The wiring the whole fix hangs off: without it the pacer keeps
        // answering `Offscreen`, and a surface that never waits is never
        // capped (frame-pacing.md §6).
        for presentation in [
            Presentation::Vsync,
            Presentation::Mailbox,
            Presentation::Immediate,
        ] {
            let (mut driver, _backend) = driver_presenting(presentation);
            driver.frame(frames_worth(1));
            assert_eq!(
                driver.pacing.schedule(Seconds(0.0)) == super::super::pacing::Schedule::Now,
                !presentation.needs_a_cap(),
                "{presentation} was paced as if it were something else"
            );
        }
    }

    #[test]
    fn the_panel_is_rebuilt_four_times_a_second_rather_than_once_a_frame() {
        // The instrument-perturbation rule, as behaviour rather than as a
        // timing. Composing the panel is string formatting, a sort of the
        // window, and a percentile per bucket; doing it sixty times a second
        // would put all of that inside every frame — and the numbers would
        // strobe too fast to read (overlay/mod.rs's REPAINT_PERIOD).
        let (mut driver, _backend) = driver_with_a_backend();
        driver.overlay = Overlay::new(Level::Perf);
        // A tenth of a second of frames, after the first one, which always
        // repaints so a screenshot taken at launch says something.
        driver.frame(frames_worth(1));
        let first = driver.overlay.readout().to_owned();
        for _ in 0..6 {
            driver.frame(frames_worth(1));
        }
        assert_eq!(
            driver.overlay.readout(),
            first,
            "the panel was rebuilt inside a quarter second"
        );
        for _ in 0..10 {
            driver.frame(frames_worth(1));
        }
        assert_ne!(
            driver.overlay.readout(),
            first,
            "a quarter second went by and the panel never caught up"
        );
    }

    #[test]
    fn the_performance_panel_costs_the_frame_it_measures_almost_nothing() {
        // "An instrument that perturbs is the failure mode." This is the
        // measurement, taken the only way it can be taken honestly: the same
        // driver, the same scene, the same number of frames, run with the panel
        // off and with it at level 2, on a backend that draws nothing so that
        // what is left is the instrument.
        //
        // The bound is deliberately loose — twenty-five times what this costs
        // in practice — because a threshold tight enough to be interesting on
        // one machine is a flake on a loaded runner. What it catches is a
        // *structural* mistake rather than a slow afternoon: sampling `/proc`
        // every frame, or composing the panel every frame, would each land here.
        let cost_of = |level| {
            let (mut driver, _backend) = driver_with_a_backend();
            driver.overlay = Overlay::new(level);
            driver.simulation.add_system(Draw, draw_one_rectangle);
            // Warm the allocations up, so the measurement is of steady state
            // rather than of the first `String` the panel ever built.
            for _ in 0..200 {
                driver.frame(frames_worth(1));
            }
            let started = web_time::Instant::now();
            for _ in 0..FRAMES_MEASURED {
                driver.frame(frames_worth(1));
            }
            started.elapsed().as_secs_f64() / f64::from(FRAMES_MEASURED)
        };
        let off = cost_of(Level::Off);
        let pacing = cost_of(Level::Pacing);
        let perf = cost_of(Level::Perf);
        let added = (perf - off) * 1e6;
        println!(
            "overlay overhead: off {:.1}us, level 1 {:.1}us (+{:.1}), level 2 {:.1}us (+{added:.1}) a frame",
            off * 1e6,
            pacing * 1e6,
            (pacing - off) * 1e6,
            perf * 1e6
        );
        // A tenth of the tick period the engine's picture actually changes at,
        // stated against `fixed_dt` rather than as a bare number so the bound
        // moves with the thing it is a share of.
        let tenth_of_a_tick =
            f64::from(jidousha_core::GameConfig::default().fixed_dt.as_f32()) * 1e6 / 10.0;
        assert!(
            added < tenth_of_a_tick,
            "level 2 added {added:.1}us a frame against a {tenth_of_a_tick:.1}us bound, which is not an instrument that leaves what it measures alone"
        );
    }

    /// How many frames the overhead measurement above times.
    ///
    /// Enough that the panel is rebuilt several times inside the window — a
    /// measurement that never crossed a repaint would leave out the one part of
    /// the instrument that is not a handful of additions.
    const FRAMES_MEASURED: u32 = 2_000;

    #[test]
    fn a_long_stall_does_not_run_hundreds_of_ticks() {
        // The spiral of death: a ten-second hitch must leave the simulation
        // behind, not try to catch up all at once (core.md §7).
        let mut driver = driver();
        driver.frame(Seconds(10.0));
        let ticks = driver.simulation.world().resource::<Seen>().pressed.len();
        assert!(ticks <= 16, "{ticks} ticks from one stalled frame");
    }
}
