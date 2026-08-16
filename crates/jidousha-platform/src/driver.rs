//! The windowed driver: winit's event loop on one side, the engine's frame on
//! the other.
//!
//! Key types: `Driver`.
//! Depends on: `winit`, `jidousha-core`, `jidousha-input`, `clock`, `error`.
//! INVARIANT (ADR-0004, CONTRACT): no `winit` type appears in anything this
//! module makes public. Every winit value is translated here or discarded here.
//! INVARIANT: the accumulator lives in `Simulation`, not here. This module
//! decides *when* a frame happens and hands over how long it was; the loop that
//! turns that into ticks is the same one `headless` runs (core.md §8 CONTRACT).

use std::sync::Arc;

use jidousha_core::{GameConfig, Seconds, Simulation, World};
use jidousha_input::{Input, InputEvent, SnapshotBuilder};
use jidousha_render_core::{
    BackendTextureId, Camera, PhysicalSize, RenderBackend, TextureTable, plan_frame,
};
use jidousha_render_wgpu::WgpuBackend;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

/// The two built-in textures every backend has, by convention: a white texel
/// for untextured shapes, and the checkered placeholder for anything not ready
/// (renderer.md §5). Uploading them is R2's, when there is art to be not-ready.
const WHITE: BackendTextureId = BackendTextureId(0);
const PLACEHOLDER: BackendTextureId = BackendTextureId(1);

use crate::clock::FrameClock;
use crate::error::RunError;

/// Drives a game from winit's callbacks.
///
/// Owned by [`run`](crate::run) for the life of the program. winit calls into
/// it; it never calls out except to create the window and to ask for the next
/// frame.
pub(crate) struct Driver {
    config: GameConfig,
    simulation: Simulation,
    /// `None` until `resumed` — on Android the window comes and goes, and the
    /// same shape is what winit asks for everywhere (ADR-0005's headroom).
    ///
    /// An `Arc` because the render surface borrows the window for as long as it
    /// lives, and wgpu wants that ownership shared rather than promised.
    window: Option<Arc<Window>>,
    /// `None` until there is a window to draw into.
    backend: Option<WgpuBackend>,
    /// Which engine texture id maps to which uploaded one. Empty until R2:
    /// every id resolves to the placeholder, which is the correct answer while
    /// nothing has been uploaded.
    textures: TextureTable,
    clock: FrameClock,
    input: SnapshotBuilder,
    /// Set when something went wrong badly enough to stop; `run` returns it.
    ///
    /// Native only: on the web `run` has already returned by the time anything
    /// can go wrong, so a failure there is printed and nothing more.
    #[cfg(not(target_arch = "wasm32"))]
    failure: Option<RunError>,
}

impl Driver {
    pub(crate) fn new(config: GameConfig, simulation: Simulation) -> Self {
        Self {
            config,
            simulation,
            window: None,
            backend: None,
            textures: TextureTable::new(WHITE, PLACEHOLDER),
            clock: FrameClock::new(),
            input: SnapshotBuilder::new(),
            #[cfg(not(target_arch = "wasm32"))]
            failure: None,
        }
    }

    /// What stopped the run, if anything did.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn failure(&mut self) -> Option<RunError> {
        self.failure.take()
    }

    /// Stop the run, saying why.
    ///
    /// The message is printed wherever this happens, because on the web `run`
    /// returned long ago and the value has nowhere to go. On native it is also
    /// kept, so `run` can hand it back.
    fn fail(&mut self, error: RunError, event_loop: &ActiveEventLoop) {
        eprintln!("{error}");
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.failure = Some(error);
        }
        event_loop.exit();
    }

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
    fn frame(&mut self, elapsed: Seconds) {
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

        let Some(backend) = backend else {
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

    /// Tell the world how big the window is now.
    ///
    /// The camera's viewport is driver-maintained (renderer.md §4): it
    /// describes the window, and a game that set it would be lying to itself
    /// about how big the window is.
    fn resize(&mut self, size: PhysicalSize) {
        if let Some(backend) = &mut self.backend {
            backend.resize_surface(size);
        }
        if let Some(camera) = self.simulation.world_mut().find_resource_mut::<Camera>() {
            camera.viewport = size;
        }
    }
}

impl ApplicationHandler for Driver {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            // Resumed again after a suspend: the window already exists, and
            // recreating it would drop the surface the renderer will hold (R1).
            return;
        }
        let attributes = window_attributes(self.config.title);
        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                let size = window_size(&window);
                match WgpuBackend::new(Arc::clone(&window), size) {
                    Ok(backend) => self.backend = Some(backend),
                    // No surface means nothing will ever be drawn, so there is
                    // no point continuing with a window that stays blank.
                    Err(error) => {
                        self.fail(
                            RunError::WindowCreation {
                                detail: error.to_string(),
                            },
                            event_loop,
                        );
                        return;
                    }
                }
                window.request_redraw();
                self.window = Some(window);
                self.resize(size);
                // The gap between program start and the first window is real
                // time that was not gameplay.
                self.clock.skip();
            }
            Err(error) => self.fail(
                RunError::WindowCreation {
                    detail: error.to_string(),
                },
                event_loop,
            ),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Response::Exit = self.on_window_event(&event) {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Ask for the next frame as soon as this one is done: a game runs
        // continuously rather than waiting for something to happen to it.
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// How to ask for a window, per platform.
///
/// DELIBERATE: the second `cfg` branch in the engine, and it is here for the
/// same reason as the first — this is the difference the platform crate exists
/// to absorb. On the web a "window" is a canvas, and winit will not put it on
/// the page unless asked; without `with_append` the program runs correctly and
/// draws to something nobody can see (ADR-0004, ADR-0005).
#[cfg(not(target_arch = "wasm32"))]
fn window_attributes(title: &'static str) -> winit::window::WindowAttributes {
    Window::default_attributes().with_title(title)
}

#[cfg(target_arch = "wasm32")]
fn window_attributes(title: &'static str) -> winit::window::WindowAttributes {
    use winit::platform::web::WindowAttributesExtWebSys;

    Window::default_attributes()
        .with_title(title)
        .with_append(true)
}

/// The window's size in physical pixels, in the engine's vocabulary.
fn window_size(window: &Window) -> PhysicalSize {
    let size = window.inner_size();
    PhysicalSize::new(size.width, size.height)
}

/// Whether an event asked the loop to stop.
///
/// Exists so the event handling below can be tested: an `ActiveEventLoop` can
/// only be obtained from winit, inside a running loop, on a machine with a
/// display — which is none of the places these decisions need checking.
#[derive(Debug, PartialEq, Eq)]
enum Response {
    Continue,
    Exit,
}

impl Driver {
    fn on_window_event(&mut self, event: &WindowEvent) -> Response {
        match event {
            WindowEvent::CloseRequested => return Response::Exit,

            // Focus is input — simulation can observe it, so it is recorded
            // like everything else (input.md §4). Losing it also synthesizes a
            // release for every held key, which is where the stuck-key bug
            // after alt-tab is designed out.
            WindowEvent::Focused(true) => {
                self.input.record(InputEvent::FocusGained);
                // Time passed while the window was in the background; none of
                // it was gameplay.
                self.clock.skip();
            }
            WindowEvent::Focused(false) => self.input.record(InputEvent::FocusLost),

            WindowEvent::RedrawRequested => {
                let elapsed = self.clock.frame();
                self.frame(elapsed);
            }

            // Resize is lifecycle, not input (input.md §4): it goes to the
            // surface and to the camera, never through `Input`.
            WindowEvent::Resized(size) => {
                self.resize(PhysicalSize::new(size.width, size.height));
            }
            // A scale change comes with its own resize event, so there is
            // nothing to do here that the arm above will not do.
            WindowEvent::ScaleFactorChanged { .. } => {}

            // Keyboard and pointer events translate to `InputEvent` at I1,
            // which owns the winit tables (input.md §8). The seam they will
            // arrive through — `self.input` — is already here and already
            // driving the per-tick snapshots above, so I1 adds a translation
            // and nothing else.
            _ => {}
        }
        Response::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jidousha_core::{Draw, DrawCtx, Resource, Update, build};
    use jidousha_input::Key;

    /// What `Input` said, once per tick, in tick order.
    #[derive(Debug, Default)]
    struct Seen {
        pressed: Vec<bool>,
        held: Vec<bool>,
        released: Vec<bool>,
    }
    impl Resource for Seen {}

    /// How many times the Draw phase ran.
    ///
    /// An atomic because Draw systems cannot write the world (ADR-0008), and
    /// resources are `Send + Sync`. This is the interior mutability core's
    /// world-shape check is defense-in-depth against — used here on purpose,
    /// by a test, to count something the world's *shape* does not record.
    #[derive(Debug, Default)]
    struct Frames(std::sync::atomic::AtomicU32);
    impl Resource for Frames {}

    fn count_the_frame(ctx: &mut DrawCtx) {
        ctx.world
            .resource::<Frames>()
            .0
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn frames_drawn(driver: &Driver) -> u32 {
        driver
            .simulation
            .world()
            .resource::<Frames>()
            .0
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    fn watch_the_key(world: &mut World) {
        let (pressed, held, released) = {
            let input = world.resource::<Input>();
            (
                input.just_pressed(Key::A),
                input.held(Key::A),
                input.just_released(Key::A),
            )
        };
        let seen = world.resource_mut::<Seen>();
        seen.pressed.push(pressed);
        seen.held.push(held);
        seen.released.push(released);
    }

    /// A driver with the watcher registered and nothing else.
    fn driver() -> Driver {
        let config = GameConfig::default();
        let simulation = build(config, |app| {
            app.add_system(Update, watch_the_key);
            app.add_system(Draw, count_the_frame);
        });
        let mut driver = Driver::new(config, simulation);
        driver
            .simulation
            .world_mut()
            .insert_resource(Seen::default());
        driver
            .simulation
            .world_mut()
            .insert_resource(Frames::default());
        driver
    }

    /// Long enough for exactly `ticks` ticks at the default timestep.
    ///
    /// With a nudge, because `fixed_dt` is `1.0 / 60.0` rounded to f32 and
    /// `ticks / 60.0` is not: three ticks' worth of the former is a hair more
    /// than the latter, and without the nudge this asks for three and gets two.
    fn frames_worth(ticks: u32) -> Seconds {
        Seconds(ticks as f32 / 60.0 + 1e-4)
    }

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
    fn closing_the_window_stops_the_loop() {
        let mut driver = driver();
        assert_eq!(
            driver.on_window_event(&WindowEvent::CloseRequested),
            Response::Exit
        );
    }

    #[test]
    fn the_focus_events_winit_reports_reach_the_input_builder() {
        // The wiring between winit's vocabulary and the engine's, which the
        // tests above reach past by recording on the builder directly. Without
        // this, the alt-tab release could be perfect and never be triggered.
        let mut driver = driver();
        driver.input.record(InputEvent::KeyPressed(Key::A));
        driver.frame(frames_worth(1));

        assert_eq!(
            driver.on_window_event(&WindowEvent::Focused(false)),
            Response::Continue,
            "losing focus is not a reason to quit"
        );
        driver.frame(frames_worth(1));

        let seen = driver.simulation.world().resource::<Seen>();
        assert_eq!(seen.released, vec![false, true]);
        assert_eq!(seen.held, vec![true, false]);
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
