//! The loop: one implementation of Startup and Update, driven by whoever owns
//! the frame.
//!
//! Key types: `Simulation`.
//! Depends on: `schedule`, `time`, `rng`, `world`, `units`. Must never depend
//! on: any platform crate — real frame time arrives as an argument, never from
//! a clock this crate reads (ADR-0005).
//! INVARIANT: `Simulation::tick` is the only path that advances the world, so
//! the windowed driver (M4's `run`) and the headless one execute Startup and
//! Update identically — one loop implementation, two drivers (core.md §8
//! CONTRACT).

use crate::draw::Submissions;
use crate::rng::Rng;
use crate::schedule::{IntoSystem, Phase, Schedule, Startup, Update};
use crate::time::Time;
use crate::units::Seconds;
use crate::world::World;

/// How much real time one frame may contribute before the loop stops trying to
/// catch up.
///
/// Without a ceiling, a machine that stalls for ten seconds would run six
/// hundred ticks at once — the "spiral of death", where catching up takes
/// longer than the time it is catching up on. Simulation time simply falls
/// behind instead, which is visible and recoverable (core.md §7).
const MAX_FRAME: Seconds = Seconds(0.25);

/// A world, its schedule, and the fixed-timestep clock that drives them.
///
/// This is the substrate for headless runs, replay tests, and `tools/verify`.
/// M4 wraps it in `run`/`headless` with a `GameConfig`; nothing in the loop
/// changes when it does.
///
/// ```
/// use jidousha_core::{Component, Seconds, Simulation, Update, World};
///
/// #[derive(Debug, PartialEq)]
/// struct Position(i32);
/// impl Component for Position {}
///
/// fn drift(world: &mut World) {
///     for (_, position) in world.query_mut::<&mut Position>() {
///         position.0 += 1;
///     }
/// }
///
/// let mut simulation = Simulation::new(42, Seconds(1.0 / 60.0));
/// simulation.add_system(Update, drift);
///
/// let entity = simulation.world_mut().spawn();
/// simulation.world_mut().insert(entity, Position(0));
///
/// simulation.tick();
/// simulation.tick();
/// assert_eq!(simulation.world().component::<Position>(entity), &Position(2));
/// ```
pub struct Simulation {
    world: World,
    schedule: Schedule,
    /// Real time carried over from previous frames, not yet spent on a tick.
    accumulator: Seconds,
    started: bool,
    /// The current frame's draw submissions, reused across frames.
    submissions: Submissions,
}

impl Simulation {
    /// Create a simulation with a seeded [`Rng`] and a [`Time`] resource.
    ///
    /// `seed` fixes every random draw for the whole run; `fixed_dt` fixes what
    /// one tick means. Both are part of the determinism contract: the same seed
    /// and the same inputs replay to the same state (core.md §7).
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new(seed: u64, fixed_dt: Seconds) -> Self {
        let mut world = World::new();
        world.insert_resource(Time::new(fixed_dt));
        world.insert_resource(Rng::from_seed(seed));
        Self {
            world,
            schedule: Schedule::new(),
            accumulator: Seconds::ZERO,
            started: false,
            submissions: Submissions::new(),
        }
    }

    /// Append a system to a phase. Systems run in registration order.
    ///
    /// ```
    /// # use jidousha_core::{Seconds, Simulation, Startup, Update, World};
    /// # fn spawn_level(_world: &mut World) {}
    /// # fn physics(_world: &mut World) {}
    /// let mut simulation = Simulation::new(1, Seconds(1.0 / 60.0));
    /// simulation.add_system(Startup, spawn_level);
    /// simulation.add_system(Update, physics);
    /// ```
    pub fn add_system<P, F>(&mut self, phase: P, system: F)
    where
        P: Phase,
        F: IntoSystem<P>,
    {
        self.schedule.add_system(phase, system);
    }

    /// Run the Startup phase, once.
    ///
    /// [`Simulation::tick`] and [`Simulation::advance`] call this themselves if
    /// it has not run yet, so a driver never has to remember to.
    pub fn start(&mut self) {
        if self.started {
            return;
        }
        self.started = true;
        self.schedule.run::<Startup>(&mut self.world);
    }

    /// Run exactly one Update tick.
    ///
    /// The clock advances first, so a system reading [`Time`] sees the tick it
    /// is part of, counting from one.
    pub fn tick(&mut self) {
        self.start();
        match self.world.find_resource_mut::<Time>() {
            Some(time) => time.advance(),
            None => panic!("{}", MISSING_TIME),
        }
        self.schedule.run::<Update>(&mut self.world);
    }

    /// Spend `frame` of real time on ticks, returning how many ran.
    ///
    /// This is the accumulator loop of core.md §7: real time goes in, whole
    /// ticks come out, and the remainder is carried to the next frame. Frames
    /// longer than a quarter second are clamped.
    ///
    /// `before_tick` runs immediately before each tick, with the world and the
    /// tick's index within this frame — zero for the first. That index is what
    /// a driver needs to honor the input contract: a frame's events belong to
    /// its first tick, and the catch-up ticks behind it see state without edges
    /// (input.md §2). The callback exists because core cannot name an
    /// `InputSnapshot` — it depends on no other jidousha crate (§1, CONTRACT) —
    /// so the driver reaches in rather than core reaching out.
    ///
    /// DELIBERATE: this is the *only* place real time enters the engine, and it
    /// arrives as an argument. Nothing in `jidousha-core` reads a clock
    /// (ADR-0005). It is also the only accumulator: the windowed driver calls
    /// this rather than keeping its own, which is what makes §8's one-loop
    /// CONTRACT true.
    pub fn advance(&mut self, frame: Seconds, mut before_tick: impl FnMut(&mut World, u32)) -> u32 {
        self.start();
        let frame = if frame > MAX_FRAME { MAX_FRAME } else { frame };
        self.accumulator += frame;
        let fixed_dt = self.fixed_dt();
        let mut steps = 0;
        while self.accumulator >= fixed_dt {
            self.accumulator -= fixed_dt;
            before_tick(&mut self.world, steps);
            self.tick();
            steps += 1;
        }
        let alpha = self.accumulator.as_f32() / fixed_dt.as_f32();
        if let Some(time) = self.world.find_resource_mut::<Time>() {
            time.alpha = alpha;
        }
        steps
    }

    /// Run the Draw phase once.
    ///
    /// Draw reads the world and cannot write it, so unlike [`Simulation::tick`]
    /// this changes nothing. In debug builds the world's shape is compared
    /// before and after as defense-in-depth against interior-mutability
    /// escapes that no type can see (core.md §7, ADR-0008).
    ///
    /// # Panics
    ///
    /// In debug builds, if the world's shape changed across the phase.
    pub fn draw(&mut self) -> &Submissions {
        self.start();
        let before = self.world.shape();
        // Each frame starts empty: submissions are immediate-mode, and nothing
        // is retained across frames at the API level (renderer.md §2).
        self.submissions.clear();
        self.schedule.run_draw(&self.world, &mut self.submissions);
        let after = self.world.shape();
        debug_assert_eq!(
            before, after,
            "[jidousha] a Draw system changed the world\n  \
             Draw runs once per rendered frame, so a change here makes the simulation depend on \
             frame rate\n  \
             likely cause: a component or resource holding a Cell, RefCell, atomic, or other \
             interior mutability, written from a Draw system\n  \
             fix: move the change to an Update system (ADR-0008)"
        );
        &self.submissions
    }

    /// What the last [`draw`](Simulation::draw) submitted.
    #[must_use]
    pub fn submissions(&self) -> &Submissions {
        &self.submissions
    }

    /// The world, for reading state.
    #[must_use]
    pub fn world(&self) -> &World {
        &self.world
    }

    /// The world, for setting it up or inspecting it in a test.
    ///
    /// Game code changes the world through systems; this is for the driver and
    /// for tests.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Every phase and its systems, in run order.
    ///
    /// ```
    /// # use jidousha_core::{Seconds, Simulation, Update, World};
    /// # fn physics(_world: &mut World) {}
    /// # let mut simulation = Simulation::new(1, Seconds(1.0 / 60.0));
    /// simulation.add_system(Update, physics);
    /// assert!(simulation.schedule_debug().contains("0. physics"));
    /// ```
    #[must_use]
    pub fn schedule_debug(&self) -> String {
        self.schedule.debug_text()
    }

    fn fixed_dt(&self) -> Seconds {
        match self.world.find_resource::<Time>() {
            Some(time) => time.fixed_dt,
            None => panic!("{}", MISSING_TIME),
        }
    }
}

/// Panic text for a world whose clock was removed.
const MISSING_TIME: &str = "[jidousha] the Time resource is missing from the world\n  \
     the simulation inserts it at construction and the loop needs it every tick\n  \
     likely cause: game code called world.remove_resource::<Time>()\n  \
     fix: leave Time in the world — read it freely, but let the loop own it";
