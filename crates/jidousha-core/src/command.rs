//! Commands: structural changes recorded now and applied at the end of the
//! system that recorded them.
//!
//! Key types: `Commands`, `Bundle`, `CommandBuffer`.
//! Depends on: `component`, `entity`, `world`. Must never depend on: `query`.
//! INVARIANT: commands apply in recording order, and application is itself part
//! of the world's operation history — two runs that record the same commands in
//! the same order produce the same world (core.md §4, §6).

use core::fmt;

use crate::component::Component;
use crate::entity::Entity;
use crate::world::World;

/// A set of components to give a new entity in one go.
///
/// Implemented for tuples of components, up to six:
///
/// ```
/// # use jidousha_core::{Component, Seconds, Simulation};
/// # #[derive(Debug)] struct Position(i32);
/// # impl Component for Position {}
/// # #[derive(Debug)] struct Velocity(i32);
/// # impl Component for Velocity {}
/// # let mut simulation = Simulation::new(1, Seconds(1.0 / 60.0));
/// # let world = simulation.world_mut();
/// world.commands().spawn((Position(0), Velocity(1)));
/// ```
///
/// A single component is a one-element tuple: `spawn((Position(0),))`. That is
/// one character of ceremony in exchange for one rule instead of two.
pub trait Bundle: 'static + Send + Sync {
    /// Give every component in this bundle to `entity`.
    fn insert_into(self, world: &mut World, entity: Entity);
}

/// What a recorded command does, for reading a buffer back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    /// Create an entity carrying a bundle.
    Spawn,
    /// Destroy an entity.
    Despawn,
    /// Give an entity a component.
    Insert,
    /// Take a component away from an entity.
    Remove,
}

/// One recorded change, plus enough description to print the buffer.
pub(crate) struct Recorded {
    kind: CommandKind,
    /// The entity the command names, if it names an existing one.
    entity: Option<Entity>,
    apply: Box<dyn FnOnce(&mut World) + Send + Sync>,
}

/// Structural changes waiting to be applied.
///
/// INVARIANT: recording never touches the world, which is what lets a system
/// record while a read-only query holds it (ADR-0013).
pub(crate) struct CommandBuffer {
    recorded: Vec<Recorded>,
}

impl CommandBuffer {
    pub(crate) fn new() -> Self {
        Self {
            recorded: Vec::new(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.recorded.is_empty()
    }

    /// Take everything recorded so far, leaving the buffer empty.
    pub(crate) fn drain(&mut self) -> Vec<Recorded> {
        core::mem::take(&mut self.recorded)
    }

    /// Apply one drained batch, in recording order.
    ///
    /// CONTRACT: a command naming an entity that is no longer alive is a
    /// no-op, not a failure. Deferral exists precisely because the world moves
    /// between recording and application: another system may legitimately have
    /// despawned the entity first, and the command's intent — "this entity
    /// should be gone", "this entity should carry Frozen" — is then either
    /// already satisfied or moot (core.md §6).
    pub(crate) fn apply(world: &mut World, batch: Vec<Recorded>) {
        for command in batch {
            (command.apply)(world);
        }
    }

    fn record(
        &mut self,
        kind: CommandKind,
        entity: Option<Entity>,
        apply: impl FnOnce(&mut World) + Send + Sync + 'static,
    ) {
        self.recorded.push(Recorded {
            kind,
            entity,
            apply: Box::new(apply),
        });
    }

    fn pending(&self) -> Vec<(CommandKind, Option<Entity>)> {
        self.recorded
            .iter()
            .map(|command| (command.kind, command.entity))
            .collect()
    }
}

/// Records structural changes to apply at the end of the current system.
///
/// Direct structural operations on `&mut World` exist for setup code. During a
/// query the world is borrowed, so systems record instead:
///
/// ```
/// # use jidousha_core::{Component, Seconds, Simulation, World};
/// # #[derive(Debug)] struct Health(i32);
/// # impl Component for Health {}
/// fn reap_the_dead(world: &mut World) {
///     let mut commands = world.commands();
///     for (entity, health) in world.query::<&Health>() {
///         if health.0 <= 0 {
///             commands.despawn(entity);
///         }
///     }
/// }
/// # let mut simulation = Simulation::new(1, Seconds(1.0 / 60.0));
/// # simulation.add_system(jidousha_core::Update, reap_the_dead);
/// ```
///
/// The recorder holds no borrow on the world's entities or components, so
/// reading and recording interleave freely. Everything recorded applies when
/// the system returns, in recording order, before the next system runs
/// (core.md §6 CONTRACT).
pub struct Commands<'w> {
    buffer: core::cell::RefMut<'w, CommandBuffer>,
}

impl<'w> Commands<'w> {
    pub(crate) fn new(buffer: core::cell::RefMut<'w, CommandBuffer>) -> Self {
        Self { buffer }
    }

    /// Create an entity carrying `bundle`.
    ///
    /// The handle is allocated at application time, so it is not available
    /// here. Give the entity everything it needs through the bundle; a system
    /// that must hold the handle spawns directly on `&mut World` instead.
    pub fn spawn<B: Bundle>(&mut self, bundle: B) {
        self.buffer.record(CommandKind::Spawn, None, move |world| {
            let entity = world.spawn();
            bundle.insert_into(world, entity);
        });
    }

    /// Destroy `entity`.
    pub fn despawn(&mut self, entity: Entity) {
        self.buffer
            .record(CommandKind::Despawn, Some(entity), move |world| {
                let _ = world.try_despawn(entity);
            });
    }

    /// Give `entity` a component.
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) {
        self.buffer
            .record(CommandKind::Insert, Some(entity), move |world| {
                let _ = world.try_insert(entity, value);
            });
    }

    /// Take a component away from `entity`.
    pub fn remove<T: Component>(&mut self, entity: Entity) {
        self.buffer
            .record(CommandKind::Remove, Some(entity), move |world| {
                let _ = world.try_remove::<T>(entity);
            });
    }

    /// What is queued, in recording order — for debugging a system that is not
    /// changing what you expect.
    #[must_use]
    pub fn pending(&self) -> Vec<(CommandKind, Option<Entity>)> {
        self.buffer.pending()
    }
}

impl fmt::Debug for Commands<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Commands")
            .field("pending", &self.buffer.pending())
            .finish()
    }
}

/// Implement `Bundle` for one tuple arity.
macro_rules! impl_bundle_tuple {
    ($($part:ident $field:ident),+) => {
        impl<$($part: Component),+> Bundle for ($($part,)+) {
            fn insert_into(self, world: &mut World, entity: Entity) {
                let ($($field,)+) = self;
                $(world.insert(entity, $field);)+
            }
        }
    };
}

impl_bundle_tuple!(A a);
impl_bundle_tuple!(A a, B b);
impl_bundle_tuple!(A a, B b, C c);
impl_bundle_tuple!(A a, B b, C c, D d);
impl_bundle_tuple!(A a, B b, C c, D d, E e);
impl_bundle_tuple!(A a, B b, C c, D d, E e, F f);
