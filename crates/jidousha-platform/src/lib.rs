//! Window, event pump and the real-time loop driver — the engine's only platform seam.
//!
//! Key types: `run`, `RunError`, `FrameClock`.
//! Depends on: `jidousha-core`, `jidousha-input`, `winit`. Must never be depended on by:
//! `jidousha-core`.
//! INVARIANT: the ONLY crate that may depend on `winit` and the ONLY crate that may
//! read a wall clock; neither may appear in its public API (ADR-0004, ADR-0005).
//!
//! Built so far (`docs/internal/core.md` §11): M5 — the window, the event loop,
//! and the real-time driver. Keyboard and pointer translation is I1's; the
//! surface and anything drawn on it is R1's.
//!
//! ```no_run
//! use jidousha_core::{GameConfig, Update, World};
//!
//! fn physics(_world: &mut World) {}
//!
//! # fn main() -> Result<(), jidousha_platform::RunError> {
//! jidousha_platform::run(
//!     GameConfig {
//!         title: "asteroids",
//!         ..GameConfig::default()
//!     },
//!     |app| {
//!         app.add_system(Update, physics);
//!     },
//! )
//! # }
//! ```

mod clock;
mod driver;
mod error;
#[cfg(not(target_arch = "wasm32"))]
mod files;

pub use clock::FrameClock;
pub use error::RunError;
#[cfg(not(target_arch = "wasm32"))]
pub use files::FileSource;

use jidousha_core::{App, GameConfig};

use crate::driver::Driver;

/// Run a game in a window, forever.
///
/// The same setup closure `headless` takes, driven by real time instead of by
/// hand. CONTRACT: both paths build the simulation through [`jidousha_core::build`]
/// and run Startup and Update identically, so a game that replays correctly
/// headless replays correctly on screen (core.md §8).
///
/// # Errors
///
/// If there is no display, if the window cannot be created, or if the event
/// loop stops with a fault. All three are facts about the machine rather than
/// about the game — a headless CI runner gets the first one, and its message
/// says to use `headless` instead (core.md §9).
///
/// # Platform notes
///
/// On native this returns when the window closes. On the web it returns
/// immediately and the game keeps running: the browser owns the loop, and
/// nothing after this call will execute in the way a native program expects
/// (ADR-0005's callback-shaped lifecycle, which is why the API takes a closure
/// rather than handing back a loop to run).
pub fn run(config: GameConfig, setup: impl FnOnce(&mut App)) -> Result<(), RunError> {
    let simulation = jidousha_core::build(config, setup);
    let driver = Driver::new(config, simulation);

    let event_loop = winit::event_loop::EventLoop::new().map_err(|error| {
        // winit reports a missing display as an OS error, which is the case a
        // headless runner and an SSH session both hit. Naming it precisely is
        // worth more than passing the raw text through, because the fix is
        // different from every other startup failure.
        RunError::NoDisplay {
            detail: error.to_string(),
        }
    })?;
    // Poll rather than Wait: a game draws continuously, and `Wait` would idle
    // until something happened to the window.
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    run_app(event_loop, driver)
}

/// Hand the loop to winit, in whichever way this platform hands it over.
///
/// DELIBERATE: the one `cfg` branch in the engine, and it is here because this
/// is the difference the platform crate exists to absorb. On the web the loop
/// belongs to the browser and never gives control back; on native it returns
/// when the window closes (ADR-0005).
#[cfg(not(target_arch = "wasm32"))]
fn run_app(
    event_loop: winit::event_loop::EventLoop<()>,
    mut driver: Driver,
) -> Result<(), RunError> {
    event_loop
        .run_app(&mut driver)
        .map_err(|error| RunError::EventLoop {
            detail: error.to_string(),
        })?;
    // A failure the driver recorded mid-run outranks a clean loop exit: winit
    // returns `Ok` from `run_app` when the loop was asked to stop, and being
    // asked to stop is exactly what the driver does when it cannot go on.
    match driver.failure() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(target_arch = "wasm32")]
fn run_app(event_loop: winit::event_loop::EventLoop<()>, driver: Driver) -> Result<(), RunError> {
    use winit::platform::web::EventLoopExtWebSys;

    // `spawn_app` takes the driver and returns immediately; the browser calls
    // back into it from then on. Nothing that happens after this point can be
    // reported through the return value — on the web there is no "after", which
    // is the whole reason the lifecycle is callback-shaped (ADR-0005).
    event_loop.spawn_app(driver);
    Ok(())
}
