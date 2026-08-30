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
mod report;
mod translate;
mod web;

pub use clock::FrameClock;
pub use error::RunError;
#[cfg(not(target_arch = "wasm32"))]
pub use files::FileSource;
#[cfg(target_arch = "wasm32")]
pub use web::WebSource;

use jidousha_core::{App, GameConfig};

use crate::driver::Driver;

/// The asset source this platform reads with — `root` is a path from the top of
/// the repository, and the web build stages it under your page, so the same
/// string works on both.
///
/// **This is what a game calls.** `FileSource` and `WebSource` are both public
/// and both real, but which one a game wants is never a question it should have
/// to answer — a game that wrote the `cfg` itself would be doing the platform
/// crate's job, and would get it wrong the first time it was ported.
///
/// `root` names a directory **from the top of the repository**, and the same
/// string works on both platforms: native reads it from there, and on the web
/// the build stages that directory under the page at the same path, so the
/// relative URL and the relative path are one string (assets.md §2 CONTRACT).
/// Everything a game loads is relative to it, with forward slashes everywhere.
///
/// **There are two roots, and where your code lives picks which one is yours**
/// (ADR-0040):
///
/// - an engine example loads from the repository's shared `"assets"`;
/// - a game crate loads from its own — `"games/giri/assets"` for a game at
///   `games/giri/` — so its art travels with it and two games' `icon_coin.png`
///   cannot collide.
///
/// `tools/build-web` stages exactly those two under a page and
/// `tools/check-assets` refuses any other, which is what makes "it works on the
/// web too" something you can rely on rather than something to find out after
/// deploying.
///
/// Bytes this store resolves for a `load_texture` are decoded by the engine, so
/// a picture is a picture on every platform and a file that is not one resolves
/// `Failed` with a message naming your line (assets.md §3, §6).
///
/// ```no_run
/// use jidousha_assets::Assets;
///
/// // In a game at `games/giri/`, this would be "games/giri/assets".
/// let mut assets = Assets::new(jidousha_platform::asset_source("assets"));
/// let hero = assets.load_texture("sprites/hero.png");
/// # let _ = hero;
/// ```
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn asset_source(root: &str) -> impl jidousha_assets::ByteSource {
    FileSource::new(root)
}

/// The asset source this platform reads with — `fetch`, on the web.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn asset_source(root: &str) -> impl jidousha_assets::ByteSource {
    WebSource::new(root)
}

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
    // On the web, panics must reach the page, not just the (invisible) stderr —
    // the playtest overlay is built on this hook (web-publish.md §2 CONTRACT).
    // Installed before anything that can panic, and before the forced test
    // panic below, which exists so the overlay itself is checkable: loading any
    // game with `?panic=1` proves the §9 text arrives where a playtester looks.
    #[cfg(target_arch = "wasm32")]
    {
        web::panic::install();
        if web::panic::forced_panic_requested() {
            panic!(
                "{}",
                jidousha_core::message(
                    "forced test panic",
                    "the page URL asked for it with ?panic=1",
                    "someone is checking the panic overlay (web-publish.md §2)",
                    "remove ?panic=1 from the URL to run the game normally",
                )
            );
        }
    }

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
    // The value the loop starts on, and the only one it has until there is a
    // window: `Wait` — winit's default — would idle until something happened to
    // the window, and a game draws continuously.
    //
    // From the first window onwards the driver sets this every iteration, out
    // of what the surface says about how its frames reach the display: `Poll`
    // while something else is doing the waiting, and a real sleep when nothing
    // is (driver/pacing.rs, frame-pacing.md §6).
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
