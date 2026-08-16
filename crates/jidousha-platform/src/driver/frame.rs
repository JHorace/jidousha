//! One frame: settle the assets, run the ticks the elapsed time bought, draw.
//!
//! Key types: none — this is `Driver`'s frame half, split from `mod.rs` because
//! together they are twice the length a file should be (CLAUDE.md).
//! Depends on: `jidousha-core`, `jidousha-assets`, `jidousha-render-core`.
//! INVARIANT: no `winit` type appears here at all. That is not an accident of
//! the split — it is why the split falls where it does, and it is what lets the
//! tests below run a whole frame on a machine with no display and no GPU.

use jidousha_assets::Assets;
use jidousha_core::{Seconds, Time, World};
use jidousha_input::Input;
use jidousha_render_core::{Camera, PhysicalSize, plan_frame, upload_ready_textures};

use super::Driver;

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
            ..
        } = self;
        simulation.advance(elapsed, |world: &mut World, index| {
            let snapshot = if index == 0 {
                input.first_tick_snapshot()
            } else {
                input.catch_up_snapshot()
            };
            world.insert_resource(Input::new(snapshot));
        });

        // Read before drawing, which is both what the borrow checker wants and
        // what the phase means: Draw cannot change the world (ADR-0008), so the
        // camera it draws with is the one the last Update tick left.
        //
        // The camera is the game's to set; a game that never inserts one gets
        // the default rather than a panic, because "I have not thought about
        // the camera yet" is a real state for a prototype to be in.
        let camera = simulation
            .world()
            .find_resource::<Camera>()
            .copied()
            .unwrap_or_default();

        // Draw once per frame, however many ticks ran — including none
        // (core.md §7).
        let submissions = simulation.draw();

        let (Some(backend), Some(textures)) = (backend, textures.as_ref()) else {
            return;
        };
        let plan = plan_frame(&camera, submissions.quads(), textures);
        if let Err(error) = backend.render(&plan) {
            // A frame that cannot be drawn is not a reason to stop: the surface
            // usually comes back. Saying so once per occurrence beats silence
            // and beats quitting.
            eprintln!("{error}");
        }
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
            // per-frame signalling from here on (assets.md §6).
            eprintln!("{}", failure.message());
        }
        let (Some(backend), Some(textures)) = (backend, textures.as_mut()) else {
            // No GPU yet. The store keeps the texels until there is one, which
            // is why nothing is lost by a window that arrives late.
            return;
        };
        upload_ready_textures(assets, backend.as_mut(), textures);
    }

    /// Tell the world how big the window is now.
    ///
    /// The camera's viewport is driver-maintained (renderer.md §4): it
    /// describes the window, and a game that set it would be lying to itself
    /// about how big the window is.
    pub(super) fn resize(&mut self, size: PhysicalSize) {
        if let Some(backend) = &mut self.backend {
            backend.resize_surface(size);
        }
        if let Some(camera) = self.simulation.world_mut().find_resource_mut::<Camera>() {
            camera.viewport = size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use super::*;
    use jidousha_assets::{AssetStatus, MemorySource};
    use jidousha_core::{Draw, Transform, math::Vec2};
    use jidousha_input::{InputEvent, Key};
    use jidousha_render_core::{NullBackend, Sprite, draw_sprites};

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
        // the world (renderer.md §4). Before a backend existed this could only
        // check the camera half.
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
        assert_eq!(
            driver.simulation.world().resource::<Camera>().viewport,
            PhysicalSize::new(640, 480)
        );
    }

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
