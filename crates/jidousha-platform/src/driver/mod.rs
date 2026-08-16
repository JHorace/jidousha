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
//!
//! **This half owns the window** and translates winit's events. The frame
//! itself — ticks, assets, drawing — is [`frame`], which names no winit type at
//! all and is tested without a window. The split is by length (CLAUDE.md) and it
//! falls here because this is where the platform actually ends.

use std::sync::Arc;

use jidousha_core::{GameConfig, Simulation};
use jidousha_input::{InputEvent, PointerId, SnapshotBuilder};
use jidousha_render_core::{
    Camera, PhysicalSize, RenderBackend, TextureTable, create_builtin_textures,
};
use jidousha_render_wgpu::WgpuBackend;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::clock::FrameClock;
use crate::error::RunError;
use crate::translate;

mod frame;
#[cfg(test)]
mod testing;

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
    ///
    /// Held behind the trait rather than as a `WgpuBackend`. This crate is still
    /// the composition root that *picks* wgpu — `resumed` names it, and nothing
    /// else does (ADR-0003) — but the frame path below only ever asks for what
    /// the seam offers, and storing it that way is what lets the tests at the
    /// bottom of this file install a `NullBackend` and check the third of this
    /// module that otherwise needs a window and a GPU to reach.
    backend: Option<Box<dyn RenderBackend>>,
    /// Which engine texture id maps to which uploaded one.
    ///
    /// `None` until a backend exists to hold the two built-in textures, since
    /// the table is the thing that names them and inventing ids before they are
    /// created is how a driver ends up drawing whatever was uploaded first.
    textures: Option<TextureTable>,
    /// How big the window is, in pixels — the driver's answer, not the game's.
    ///
    /// Kept here rather than only written onto the camera when a resize event
    /// arrives, because the camera is not there to write to yet: `resumed`
    /// learns the size before the first frame, and a game's `Camera` is
    /// inserted by its Startup system *during* that frame. Writing on resize
    /// alone meant every game drew at `Camera::default()`'s 1280x720 aspect
    /// until the player happened to resize the window (e0-findings.md F-012).
    viewport: PhysicalSize,
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
            textures: None,
            // Until `resumed` measures a real window. Sharing the camera's own
            // default keeps a headless run and a windowed one describing the
            // same screen until the window says otherwise.
            viewport: Camera::default().viewport,
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
        crate::report::problem(&error.to_string());
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.failure = Some(error);
        }
        event_loop.exit();
    }

    /// Translate one key event and give it to the builder.
    ///
    /// Split out from the `WindowEvent::KeyboardInput` arm so that it can be
    /// tested: `winit::event::KeyEvent` has a private field and cannot be built
    /// outside winit, so that one arm is the only part of this module a test
    /// cannot drive with a real event. Taking the fields instead leaves the arm
    /// with nothing in it but the destructuring — and the two `bool`s it could
    /// conceivably swap are filtered identically, so even that has no wrong
    /// version to reach.
    fn record_key(
        &mut self,
        physical_key: winit::keyboard::PhysicalKey,
        state: winit::event::ElementState,
        repeat: bool,
        is_synthetic: bool,
    ) {
        if let Some(translated) = translate::key_event(physical_key, state, repeat, is_synthetic) {
            self.input.record(translated);
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
                    Ok(mut backend) => {
                        // Before anything else is uploaded, so the white texel
                        // and the placeholder are the first two textures and the
                        // table knows which ids they got. The GPU is still on
                        // its way; the backend holds these until it arrives.
                        self.textures = Some(create_builtin_textures(&mut backend));
                        self.backend = Some(Box::new(backend));
                    }
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

            // Keyboard and pointer, translated into the engine's vocabulary and
            // handed to the same builder the focus events above use. Each arm
            // does nothing but destructure and delegate: the decisions — which
            // keys exist, what a repeat means, how many lines a pixel is — are
            // all in `translate`, where they can be tested without a window
            // (input.md §6).
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => self.record_key(event.physical_key, event.state, event.repeat, *is_synthetic),
            WindowEvent::CursorMoved { position, .. } => {
                self.input.record(translate::pointer_moved(*position));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(translated) = translate::button_event(*state, *button) {
                    self.input.record(translated);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.record(InputEvent::Scrolled {
                    id: PointerId::PRIMARY,
                    lines: translate::scroll_lines(*delta),
                });
            }

            // Everything else winit reports is not input: file drops, IME,
            // theme changes, occlusion. Adding one is adding an arm here.
            _ => {}
        }
        Response::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{Seen, driver, frames_worth};
    use super::*;
    use jidousha_core::math::Vec2;
    use jidousha_input::{Input, Key, PointerButton};
    use winit::dpi::PhysicalPosition;
    use winit::event::{DeviceId, ElementState, MouseButton, MouseScrollDelta};
    use winit::keyboard::{KeyCode, PhysicalKey};

    /// What the pointer looked like on the last tick that ran.
    fn pointer(driver: &Driver) -> jidousha_input::PointerState {
        driver
            .simulation
            .world()
            .resource::<Input>()
            .pointer()
            .clone()
    }

    #[test]
    fn a_key_press_reaches_the_next_tick_as_an_edge() {
        // As close to the `KeyboardInput` arm as a test can get: winit's
        // `KeyEvent` cannot be constructed outside winit, so this drives the
        // method that arm delegates to. What is left untested is the
        // destructuring itself.
        let mut driver = driver();
        driver.record_key(
            PhysicalKey::Code(KeyCode::KeyA),
            ElementState::Pressed,
            false,
            false,
        );
        driver.frame(frames_worth(1));
        assert_eq!(
            driver.simulation.world().resource::<Seen>().pressed,
            vec![true],
            "the press reached a tick"
        );

        // And auto-repeat, which the operating system sends while a key is
        // held, must not produce a second edge.
        driver.record_key(
            PhysicalKey::Code(KeyCode::KeyA),
            ElementState::Pressed,
            true,
            false,
        );
        driver.frame(frames_worth(1));
        assert_eq!(
            driver.simulation.world().resource::<Seen>().pressed,
            vec![true, false],
            "held, not pressed again"
        );
    }

    #[test]
    fn a_cursor_move_reaches_the_next_tick_as_a_screen_position() {
        // The wiring, not the translation: `translate` is tested on its own, and
        // this is the arm that has to call it. winit's `DeviceId::dummy` exists
        // for exactly this, so three of the four input arms can be driven with
        // real `WindowEvent`s rather than reached past.
        let mut driver = driver();
        assert_eq!(
            driver.on_window_event(&WindowEvent::CursorMoved {
                device_id: DeviceId::dummy(),
                position: PhysicalPosition::new(120.0, 45.0),
            }),
            Response::Continue
        );
        driver.frame(frames_worth(1));
        assert_eq!(pointer(&driver).screen, Vec2::new(120.0, 45.0));
    }

    #[test]
    fn a_click_reaches_the_next_tick_as_a_button_edge() {
        let mut driver = driver();
        driver.on_window_event(&WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: MouseButton::Left,
        });
        driver.frame(frames_worth(1));
        assert!(pointer(&driver).just_pressed(PointerButton::Primary));
        assert!(pointer(&driver).held(PointerButton::Primary));

        // And the edge is spent: the next tick still holds it, without a new
        // press (input.md §2).
        driver.frame(frames_worth(1));
        assert!(!pointer(&driver).just_pressed(PointerButton::Primary));
        assert!(pointer(&driver).held(PointerButton::Primary));
    }

    #[test]
    fn a_button_winit_reports_and_the_engine_does_not_have_is_dropped_quietly() {
        // Not an error — a documented boundary. What must not happen is a panic
        // or a stray edge on some other button.
        let mut driver = driver();
        driver.on_window_event(&WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: MouseButton::Back,
        });
        driver.frame(frames_worth(1));
        for button in PointerButton::ALL {
            assert!(!pointer(&driver).held(*button), "{button:?}");
        }
    }

    #[test]
    fn a_wheel_notch_reaches_the_next_tick_as_one_line() {
        let mut driver = driver();
        driver.on_window_event(&WindowEvent::MouseWheel {
            device_id: DeviceId::dummy(),
            delta: MouseScrollDelta::LineDelta(0.0, -1.0),
            phase: winit::event::TouchPhase::Moved,
        });
        driver.frame(frames_worth(1));
        assert_eq!(pointer(&driver).scroll, 1.0, "one line, toward the end");

        // Scroll is spent like an edge: a tick that follows sees none of it, or
        // one flick would scroll for as long as the frame rate stayed low.
        driver.frame(frames_worth(1));
        assert_eq!(pointer(&driver).scroll, 0.0);
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
}
