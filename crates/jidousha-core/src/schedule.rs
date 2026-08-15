//! The schedule: which systems run, in which phase, in which order.
//!
//! Key types: `Phase`, `Startup`, `Update`, `Draw`, `IntoSystem`, `Schedule`.
//! Depends on: `draw`, `world`. Must never depend on: `simulation` — the
//! schedule holds systems; the simulation decides when to run them.
//! INVARIANT: within a phase, systems run in registration order, and commands
//! recorded by one system apply before the next one starts (core.md §7
//! CONTRACT).
//! DELIBERATE: systems are plain functions taking the phase's context and
//! nothing else — no parameter extraction (see ADR-0007).

use core::any::type_name;

use crate::draw::DrawCtx;
use crate::world::World;

/// A system stored for a phase whose systems take `&mut World`.
type WorldSystem = Box<dyn Fn(&mut World) + Send + Sync>;

/// A system stored for the Draw phase, which is handed a context borrowing the
/// world for the length of the call.
type DrawSystem = Box<dyn for<'w> Fn(&mut DrawCtx<'w>) + Send + Sync>;

/// A stage of the frame that systems can be registered into.
///
/// Phases are types, not variants of an enum, because each names the context
/// its systems are called with. Registering an Update-shaped function into
/// Draw will be a compile error once Draw lands with `DrawCtx` (ADR-0008, M4).
pub trait Phase: 'static {
    /// How a system of this phase is stored once registered.
    type Stored;

    /// The phase's name, for the schedule listing.
    const NAME: &'static str;

    /// Append one system to this phase's list.
    fn register(schedule: &mut Schedule, system: RegisteredSystem<Self::Stored>);

    /// This phase's systems, in registration order.
    fn systems(schedule: &Schedule) -> &[RegisteredSystem<Self::Stored>];
}

/// A function that can serve as a system in phase `P`.
///
/// This is where a phase's signature is enforced. Update systems take
/// `&mut World`; Draw systems take `&mut DrawCtx`, which can only read
/// (ADR-0008). Registering one shape where the other belongs fails here, with
/// the message below rather than a wall of trait errors.
#[diagnostic::on_unimplemented(
    message = "[jidousha] this function is not shaped like a {P} system",
    label = "wrong signature for {P}",
    note = "Startup and Update systems take `&mut World`; Draw systems take `&mut DrawCtx`",
    note = "likely cause: a Draw system was registered in Update, or an Update system in Draw",
    note = "fix: give the function the phase's signature — `fn name(world: &mut World)` for \
            Startup and Update, `fn name(ctx: &mut DrawCtx)` for Draw. Draw cannot write the \
            world: move mutation to an Update system."
)]
pub trait IntoSystem<P: Phase>: 'static {
    /// Box the function up in the shape its phase stores.
    fn into_stored(self) -> P::Stored;

    /// The function's own name, for the schedule listing.
    fn name() -> &'static str {
        short_name(type_name::<Self>())
    }
}

impl<F: Fn(&mut World) + Send + Sync + 'static> IntoSystem<Startup> for F {
    fn into_stored(self) -> WorldSystem {
        Box::new(self)
    }
}

impl<F: Fn(&mut World) + Send + Sync + 'static> IntoSystem<Update> for F {
    fn into_stored(self) -> WorldSystem {
        Box::new(self)
    }
}

impl<F: for<'w> Fn(&mut DrawCtx<'w>) + Send + Sync + 'static> IntoSystem<Draw> for F {
    fn into_stored(self) -> DrawSystem {
        Box::new(self)
    }
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

/// Once per rendered frame, after Update has caught up.
///
/// Draw systems take `&mut DrawCtx` and cannot write the world: frames happen
/// at whatever rate the machine manages, so a write here would make the
/// simulation depend on the player's hardware (ADR-0008).
#[derive(Clone, Copy, Debug)]
pub struct Draw;

impl Phase for Startup {
    type Stored = WorldSystem;
    const NAME: &'static str = "Startup";

    fn register(schedule: &mut Schedule, system: RegisteredSystem<WorldSystem>) {
        schedule.startup.push(system);
    }

    fn systems(schedule: &Schedule) -> &[RegisteredSystem<WorldSystem>] {
        &schedule.startup
    }
}

impl Phase for Update {
    type Stored = WorldSystem;
    const NAME: &'static str = "Update";

    fn register(schedule: &mut Schedule, system: RegisteredSystem<WorldSystem>) {
        schedule.update.push(system);
    }

    fn systems(schedule: &Schedule) -> &[RegisteredSystem<WorldSystem>] {
        &schedule.update
    }
}

impl Phase for Draw {
    type Stored = DrawSystem;
    const NAME: &'static str = "Draw";

    fn register(schedule: &mut Schedule, system: RegisteredSystem<DrawSystem>) {
        schedule.draw.push(system);
    }

    fn systems(schedule: &Schedule) -> &[RegisteredSystem<DrawSystem>] {
        &schedule.draw
    }
}

/// A system together with the name it was registered under.
pub struct RegisteredSystem<S> {
    stored: S,
    name: &'static str,
}

impl<S> RegisteredSystem<S> {
    /// The system's name, as the schedule listing shows it.
    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn stored(&self) -> &S {
        &self.stored
    }
}

/// Every system the world runs, grouped by phase and kept in registration
/// order.
pub struct Schedule {
    startup: Vec<RegisteredSystem<WorldSystem>>,
    update: Vec<RegisteredSystem<WorldSystem>>,
    draw: Vec<RegisteredSystem<DrawSystem>>,
}

impl Schedule {
    pub(crate) fn new() -> Self {
        Self {
            startup: Vec::new(),
            update: Vec::new(),
            draw: Vec::new(),
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
        F: IntoSystem<P>,
    {
        let name = F::name();
        P::register(
            self,
            RegisteredSystem {
                stored: system.into_stored(),
                name,
            },
        );
    }

    /// Run one phase's systems in order, applying each one's commands before
    /// the next starts.
    pub(crate) fn run<P>(&self, world: &mut World)
    where
        P: Phase<Stored = WorldSystem>,
    {
        for registered in P::systems(self) {
            crate::panic_hook::with_running_system(P::NAME, registered.name(), || {
                (registered.stored())(world);
            });
            world.apply_commands();
        }
    }

    /// Run the Draw systems, each with a fresh read-only context.
    ///
    /// Nothing here can mutate the world — `DrawCtx` has no method that does —
    /// so unlike `run` there are no commands to apply (ADR-0008).
    pub(crate) fn run_draw(&self, world: &World) {
        for registered in Draw::systems(self) {
            let mut context = DrawCtx::new(world);
            crate::panic_hook::with_running_system(Draw::NAME, registered.name(), || {
                (registered.stored())(&mut context);
            });
        }
    }

    /// The schedule as text: every phase, and its systems in run order.
    ///
    /// One call answers "what runs when" for a debugging agent (core.md §7).
    pub(crate) fn debug_text(&self) -> String {
        let mut text = String::from("schedule:\n");
        let mut list = |phase: &str, names: Vec<&'static str>| {
            text.push_str(&format!("  {phase} ({})\n", names.len()));
            for (index, name) in names.iter().enumerate() {
                text.push_str(&format!("    {index}. {name}\n"));
            }
        };
        list(
            Startup::NAME,
            self.startup.iter().map(RegisteredSystem::name).collect(),
        );
        list(
            Update::NAME,
            self.update.iter().map(RegisteredSystem::name).collect(),
        );
        list(
            Draw::NAME,
            self.draw.iter().map(RegisteredSystem::name).collect(),
        );
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
             Update (2)\n    0. second\n    1. first\n  \
             Draw (0)\n"
        );
    }

    #[test]
    fn a_system_name_is_the_functions_own_name() {
        assert_eq!(short_name("my_game::systems::physics"), "physics");
        assert_eq!(short_name("physics"), "physics");
    }
}
