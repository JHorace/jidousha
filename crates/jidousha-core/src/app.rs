//! The app lifecycle: configuring a game, registering its systems, and driving
//! it without a window.
//!
//! Key types: `GameConfig`, `App`, `HeadlessSim`; `headless`.
//! Depends on: `schedule`, `simulation`, `world`. Must never depend on: any
//! platform crate — `run`, the windowed driver, lands with the platform in M5
//! and wraps the very same [`Simulation`](crate::Simulation).
//! INVARIANT: headless and windowed execution share one loop implementation, so
//! a game that replays correctly headless replays correctly on screen
//! (core.md §8 CONTRACT).

use crate::draw::Submissions;
use crate::panic_hook;
use crate::schedule::{IntoSystem, Phase};
use crate::simulation::Simulation;
use crate::units::Seconds;
use crate::visual::PhysicalSize;
use crate::world::World;

/// How a game is configured at startup.
///
/// ```
/// use jidousha_core::{GameConfig, PhysicalSize, Seconds};
///
/// let config = GameConfig {
///     title: "asteroids",
///     seed: 42,
///     window_size: PhysicalSize::new(1600, 900),
///     ..GameConfig::default()
/// };
/// assert_eq!(config.fixed_dt, Seconds(1.0 / 60.0));
/// ```
///
/// Fields for subsystems that do not exist yet — asset root, camera height —
/// arrive with those subsystems (public-api.md §2). Because games write
/// `..GameConfig::default()`, adding them later does not disturb anything
/// already written.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameConfig {
    /// The window's title. Games set it once and never think about it again.
    pub title: &'static str,
    /// Fixes every random draw of the run. Same seed, same game.
    pub seed: u64,
    /// How much simulated time one Update tick covers.
    ///
    /// One sixtieth of a second by default. A game counting in ticks — a serve
    /// pause, a coyote-time window, an invulnerability period — is counting in
    /// these.
    pub fixed_dt: Seconds,
    /// How big the window opens, in pixels. Ignored by a headless run.
    ///
    /// A suggestion, not a demand: a tiling window manager or a full-screen
    /// session will give you what it gives you, and the camera is what decides
    /// how much *world* is on screen either way. It is worth setting when a
    /// game's playing field has a shape — a 16:9 field cropped by a tall narrow
    /// window is the case this exists for.
    pub window_size: PhysicalSize,
}

impl Default for GameConfig {
    /// DELIBERATE: a `Default` impl, where ADR-0012 bans most of them — the
    /// default *value* is meaningful here (an untitled game at sixty ticks a
    /// second), and struct-update syntax is how a game states only what it
    /// cares about.
    fn default() -> Self {
        Self {
            title: "jidousha game",
            seed: 0,
            fixed_dt: Seconds(1.0 / 60.0),
            // 16:9, which is the shape most games are drawn for and most
            // screens are. It matches `Camera::default()`'s viewport, so a
            // headless transcript and a windowed run describe the same screen
            // until a game says otherwise.
            window_size: PhysicalSize::new(1280, 720),
        }
    }
}

/// What a game's setup closure is handed: the place to register systems.
///
/// Everything else a game does happens inside those systems — spawning the
/// level belongs in a Startup system, not here (core.md §8).
pub struct App {
    simulation: Simulation,
}

impl App {
    fn new(config: GameConfig) -> Self {
        Self {
            simulation: Simulation::new(config.seed, config.fixed_dt),
        }
    }

    /// Append a system to a phase. Systems run in registration order.
    ///
    /// ```
    /// # use jidousha_core::{DrawCtx, Draw, GameConfig, Startup, Update, World, headless};
    /// # fn spawn_level(_world: &mut World) {}
    /// # fn physics(_world: &mut World) {}
    /// # fn draw_sprites(_ctx: &mut DrawCtx) {}
    /// let sim = headless(GameConfig::default(), |app| {
    ///     app.add_system(Startup, spawn_level);
    ///     app.add_system(Update, physics);
    ///     app.add_system(Draw, draw_sprites);
    /// });
    /// ```
    pub fn add_system<P, F>(&mut self, phase: P, system: F)
    where
        P: Phase,
        F: IntoSystem<P>,
    {
        self.simulation.add_system(phase, system);
    }
}

/// Build a game and drive it by hand, with no window and no clock.
///
/// This is the substrate for tests, for replay, and for `tools/verify`. It runs
/// Startup and Update exactly as the windowed driver will; Draw runs only when
/// asked, since there are no frames here (core.md §8).
///
/// ```
/// use jidousha_core::{Component, GameConfig, Startup, Update, World, headless};
///
/// #[derive(Debug, PartialEq)]
/// struct Ticks(u32);
/// impl Component for Ticks {}
///
/// fn spawn_counter(world: &mut World) {
///     let entity = world.spawn();
///     world.insert(entity, Ticks(0));
/// }
///
/// fn count(world: &mut World) {
///     for (_, ticks) in world.query_mut::<&mut Ticks>() {
///         ticks.0 += 1;
///     }
/// }
///
/// let mut sim = headless(GameConfig::default(), |app| {
///     app.add_system(Startup, spawn_counter);
///     app.add_system(Update, count);
/// });
///
/// sim.tick();
/// sim.tick();
/// assert_eq!(sim.world().query::<&Ticks>().count(), 1);
/// ```
#[must_use]
pub fn headless(config: GameConfig, setup: impl FnOnce(&mut App)) -> HeadlessSim {
    HeadlessSim {
        simulation: build(config, setup),
    }
}

/// Build the simulation a driver will run.
///
/// Both drivers go through here: [`headless`] wraps it, and the platform
/// crate's `run` wraps it too. That shared path is what makes §8's CONTRACT
/// — that windowed and headless execution run Startup and Update identically —
/// a fact about the code rather than a promise about it (core.md §8).
///
/// Games do not call this. It is public because the windowed driver lives in
/// another crate, and core depends on no other jidousha crate (§1, CONTRACT),
/// so the driver cannot reach in.
#[must_use]
pub fn build(config: GameConfig, setup: impl FnOnce(&mut App)) -> Simulation {
    // Installed here rather than in each driver, so every path that runs a
    // system gets the `in system:` line in its panics (core.md §9).
    panic_hook::install();
    let mut app = App::new(config);
    setup(&mut app);
    app.simulation
}

/// A game running without a window, advanced one tick at a time.
pub struct HeadlessSim {
    simulation: Simulation,
}

impl HeadlessSim {
    /// Run one Update tick, running Startup first if it has not run yet.
    ///
    /// The per-tick input snapshot this will take (public-api.md §2's
    /// `TickInput`) arrives with the input and asset subsystems; until then a
    /// tick has nothing to feed in.
    pub fn tick(&mut self) {
        self.simulation.tick();
    }

    /// Run the Draw phase once, as a rendered frame would, and return what it
    /// submitted.
    ///
    /// Draw cannot change the world (ADR-0008), so this exists for tests that
    /// want to exercise draw systems and for `tools/verify` — a headless run
    /// that never draws is still a correct run. The returned submissions are
    /// this frame's only; the next `draw` starts empty.
    ///
    /// # Panics
    ///
    /// In debug builds, if the world's shape changed across the phase — the
    /// defense-in-depth check core.md §7 asks for, which catches the interior
    /// mutability no type can see.
    pub fn draw(&mut self) -> &Submissions {
        self.simulation.draw()
    }

    /// The world, for asserting on state.
    #[must_use]
    pub fn world(&self) -> &World {
        self.simulation.world()
    }

    /// The world, for arranging a test's starting state.
    ///
    /// Game code changes the world through systems; this is for tests.
    pub fn world_mut(&mut self) -> &mut World {
        self.simulation.world_mut()
    }

    /// Every phase and its systems, in run order.
    #[must_use]
    pub fn schedule_debug(&self) -> String {
        self.simulation.schedule_debug()
    }
}
