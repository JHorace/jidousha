//! The schedule: which systems run, in which phase, in which order.
//!
//! Key types: `Phase`, `Startup`, `Update`, `Schedule`.
//! Depends on: `world`. Must never depend on: `simulation` — the schedule holds
//! systems; the simulation decides when to run them.
//! INVARIANT: within a phase, systems run in registration order, and commands
//! recorded by one system apply before the next one starts (core.md §7
//! CONTRACT).
//! DELIBERATE: systems are plain functions taking the phase's context and
//! nothing else — no parameter extraction (see ADR-0007).

use core::any::type_name;

use crate::world::World;

/// A stage of the frame that systems can be registered into.
///
/// Phases are types, not variants of an enum, because each names the context
/// its systems are called with. Registering an Update-shaped function into
/// Draw will be a compile error once Draw lands with `DrawCtx` (ADR-0008, M4).
pub trait Phase: 'static {
    /// What a system in this phase is called with.
    type Context;

    /// The phase's name, for the schedule listing.
    const NAME: &'static str;

    /// Append one system to this phase's list.
    fn register(schedule: &mut Schedule, system: RegisteredSystem<Self::Context>);

    /// This phase's systems, in registration order.
    fn systems(schedule: &Schedule) -> &[RegisteredSystem<Self::Context>];
}

/// Runs once, before the first tick.
///
/// Startup is where a game builds its world: spawning the level, inserting
/// resources, loading what it needs.
#[derive(Clone, Copy, Debug)]
pub struct Startup;

/// The simulation, run on the fixed timestep — zero or more times per frame.
#[derive(Clone, Copy, Debug)]
pub struct Update;

impl Phase for Startup {
    type Context = World;
    const NAME: &'static str = "Startup";

    fn register(schedule: &mut Schedule, system: RegisteredSystem<World>) {
        schedule.startup.push(system);
    }

    fn systems(schedule: &Schedule) -> &[RegisteredSystem<World>] {
        &schedule.startup
    }
}

impl Phase for Update {
    type Context = World;
    const NAME: &'static str = "Update";

    fn register(schedule: &mut Schedule, system: RegisteredSystem<World>) {
        schedule.update.push(system);
    }

    fn systems(schedule: &Schedule) -> &[RegisteredSystem<World>] {
        &schedule.update
    }
}

/// A system together with the name it was registered under.
pub struct RegisteredSystem<C> {
    run: Box<dyn Fn(&mut C) + Send + Sync>,
    name: &'static str,
}

impl<C> RegisteredSystem<C> {
    /// The system's name, as the schedule listing shows it.
    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn run(&self, context: &mut C) {
        (self.run)(context);
    }
}

/// Every system the world runs, grouped by phase and kept in registration
/// order.
pub struct Schedule {
    startup: Vec<RegisteredSystem<World>>,
    update: Vec<RegisteredSystem<World>>,
}

impl Schedule {
    pub(crate) fn new() -> Self {
        Self {
            startup: Vec::new(),
            update: Vec::new(),
        }
    }

    /// Append a system to a phase.
    ///
    /// The generic parameter is what makes the system's *name* available:
    /// every function has its own zero-sized type, so the name can be read
    /// here. Pass named functions — a closure registers as `{{closure}}` and
    /// tells a future debugger nothing.
    pub(crate) fn add_system<P, F>(&mut self, _phase: P, system: F)
    where
        P: Phase,
        F: Fn(&mut P::Context) + Send + Sync + 'static,
    {
        P::register(
            self,
            RegisteredSystem {
                run: Box::new(system),
                name: short_name(type_name::<F>()),
            },
        );
    }

    /// Run one phase's systems in order, applying each one's commands before
    /// the next starts.
    pub(crate) fn run<P>(&self, world: &mut World)
    where
        P: Phase<Context = World>,
    {
        for registered in P::systems(self) {
            registered.run(world);
            world.apply_commands();
        }
    }

    /// The schedule as text: every phase, and its systems in run order.
    ///
    /// One call answers "what runs when" for a debugging agent (core.md §7).
    pub(crate) fn debug_text(&self) -> String {
        let mut text = String::from("schedule:\n");
        for (phase, systems) in [(Startup::NAME, &self.startup), (Update::NAME, &self.update)] {
            text.push_str(&format!("  {phase} ({})\n", systems.len()));
            for (index, registered) in systems.iter().enumerate() {
                text.push_str(&format!("    {index}. {}\n", registered.name()));
            }
        }
        text
    }
}

/// Trim a system's type name to something a schedule listing can show.
///
/// `type_name` gives a full path; the last segment is the function's own name,
/// which is what greps and what a stack trace shows.
fn short_name(full: &str) -> &str {
    match full.rsplit_once("::") {
        Some((_, last)) => last,
        None => full,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first(_world: &mut World) {}
    fn second(_world: &mut World) {}

    #[test]
    fn the_schedule_lists_each_phase_and_its_systems_in_order() {
        let mut schedule = Schedule::new();
        schedule.add_system(Startup, first);
        schedule.add_system(Update, second);
        schedule.add_system(Update, first);
        assert_eq!(
            schedule.debug_text(),
            "schedule:\n  \
             Startup (1)\n    0. first\n  \
             Update (2)\n    0. second\n    1. first\n"
        );
    }

    #[test]
    fn a_system_name_is_the_functions_own_name() {
        assert_eq!(short_name("my_game::systems::physics"), "physics");
        assert_eq!(short_name("physics"), "physics");
    }
}
