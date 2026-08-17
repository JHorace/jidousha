//! The world: entities, their components, and every structural operation.
//!
//! Key types: `World`.
//! Depends on: `archetype`, `command`, `component`, `entity`, `error`, `query`,
//! `resource`. Must never depend on: any other jidousha crate (core.md §1
//! CONTRACT).
//! The world's *resource* API lives in `resource.rs`, beside the store it
//! reaches — this file is entities, components, and queries.
//! INVARIANT: `locations` holds a location for exactly the live entities, and
//! every location names the archetype whose component set the entity actually
//! has. Every structural operation restores both before returning.

use core::cell::RefCell;

use crate::archetype::{Archetypes, Location};
use crate::command::{CommandBuffer, Commands};
use crate::component::Component;
use crate::entity::{Entity, EntityAllocator};
use crate::error::{EntityDeadError, message};
use crate::query::{Query, QueryIter, QueryIterMut, ReadOnlyQuery};
use crate::resource::Resources;

/// Everything the simulation can see: the entities that exist and the
/// components they carry.
///
/// Handles come from [`World::spawn`] and stay valid until the entity is
/// despawned. Using a despawned handle for a structural operation is a
/// contract violation and panics with a message naming the handle; the
/// `try_*` operations return [`EntityDeadError`] instead, for the rare call
/// site where the entity may legitimately be gone (core.md §9).
///
/// ```
/// use jidousha_core::{Component, World};
///
/// #[derive(Debug, PartialEq)]
/// struct Position(i32);
/// impl Component for Position {}
/// #[derive(Debug, PartialEq)]
/// struct Velocity(i32);
/// impl Component for Velocity {}
///
/// let mut world = World::new();
/// let entity = world.spawn();
/// world.insert(entity, Position(0));
/// world.insert(entity, Velocity(3));
///
/// for (_entity, position, velocity) in world.query_mut::<(&mut Position, &Velocity)>() {
///     position.0 += velocity.0;
/// }
///
/// assert_eq!(world.component::<Position>(entity), &Position(3));
/// ```
pub struct World {
    entities: EntityAllocator,
    /// Slot index → where its components live, `None` for slots holding no
    /// live entity.
    locations: Vec<Option<Location>>,
    archetypes: Archetypes,
    /// Read through the `impl World` block in `resource.rs`.
    pub(crate) resources: Resources,
    /// Structural changes recorded by the running system.
    ///
    /// DELIBERATE: interior mutability, so a system can record commands while a
    /// read-only query holds the world (core.md §6). The cell guards the buffer
    /// only — never component or resource data — so it cannot be used to reach
    /// around the aliasing rules of ADR-0013.
    commands: RefCell<CommandBuffer>,
}

impl World {
    /// Create an empty world.
    ///
    /// DELIBERATE: no `Default` impl, despite `clippy::new_without_default`
    /// (see ADR-0012) — one way to do everything, and `new` is that way.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            entities: EntityAllocator::new(),
            locations: Vec::new(),
            archetypes: Archetypes::new(),
            resources: Resources::new(),
            commands: RefCell::new(CommandBuffer::new()),
        }
    }

    /// Create an entity with no components.
    ///
    /// The handle is a pure function of this world's operation history: the
    /// same sequence of spawns and despawns yields the same handles on every
    /// platform and every run (core.md §2).
    pub fn spawn(&mut self) -> Entity {
        let entity = self.entities.create();
        let location = self.archetypes.push_new(entity);
        self.set_location(entity, Some(location));
        entity
    }

    /// Destroy `entity` and everything it carries.
    ///
    /// # Panics
    ///
    /// If `entity` is not alive — a contract violation. Use
    /// [`World::try_despawn`] where absence is expected.
    pub fn despawn(&mut self, entity: Entity) {
        self.expect_alive("despawn", entity);
        self.despawn_unchecked(entity);
    }

    /// [`World::despawn`], reporting a dead entity instead of panicking.
    ///
    /// # Errors
    ///
    /// [`EntityDeadError`] if `entity` is not alive.
    pub fn try_despawn(&mut self, entity: Entity) -> Result<(), EntityDeadError> {
        self.check_alive("despawn", entity)?;
        self.despawn_unchecked(entity);
        Ok(())
    }

    /// Give `entity` a `T`, replacing any `T` it already had.
    ///
    /// Adding a component moves the entity to the archetype for its new
    /// component set, which invalidates row order in both archetypes — never
    /// entity handles.
    ///
    /// # Panics
    ///
    /// If `entity` is not alive — a contract violation. Use
    /// [`World::try_insert`] where absence is expected.
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) {
        self.expect_alive("insert", entity);
        self.insert_unchecked(entity, value);
    }

    /// [`World::insert`], reporting a dead entity instead of panicking.
    ///
    /// # Errors
    ///
    /// [`EntityDeadError`] if `entity` is not alive.
    pub fn try_insert<T: Component>(
        &mut self,
        entity: Entity,
        value: T,
    ) -> Result<(), EntityDeadError> {
        self.check_alive("insert", entity)?;
        self.insert_unchecked(entity, value);
        Ok(())
    }

    /// Take `T` away from `entity`.
    ///
    /// CONTRACT: this states an end state — `entity` has no `T` afterwards —
    /// so it is idempotent, and removing a component the entity never had is
    /// not a failure. Only the entity being dead is.
    ///
    /// # Panics
    ///
    /// If `entity` is not alive — a contract violation. Use
    /// [`World::try_remove`] where absence is expected.
    pub fn remove<T: Component>(&mut self, entity: Entity) {
        self.expect_alive("remove", entity);
        self.remove_unchecked::<T>(entity);
    }

    /// [`World::remove`], reporting a dead entity instead of panicking.
    ///
    /// # Errors
    ///
    /// [`EntityDeadError`] if `entity` is not alive.
    pub fn try_remove<T: Component>(&mut self, entity: Entity) -> Result<(), EntityDeadError> {
        self.check_alive("remove", entity)?;
        self.remove_unchecked::<T>(entity);
        Ok(())
    }

    /// Iterate every entity matching a read-only query.
    ///
    /// ```
    /// # use jidousha_core::{Component, With, World};
    /// # #[derive(Debug)] struct Position(i32);
    /// # impl Component for Position {}
    /// # struct Player;
    /// # impl Component for Player {}
    /// # let mut world = World::new();
    /// for (entity, position) in world.query::<&Position>() {
    ///     println!("{entity:?} {position:?}");
    /// }
    /// for (entity, position, _) in world.query::<(&Position, With<Player>)>() {
    ///     println!("player {entity:?} {position:?}");
    /// }
    /// ```
    ///
    /// Because the query only reads, the rest of the world stays readable while
    /// it runs — point lookups on other entities included. Writing needs
    /// [`World::query_mut`].
    pub fn query<'w, Q: ReadOnlyQuery<'w>>(&'w self) -> QueryIter<'w, Q> {
        QueryIter::new(self.archetypes.all().iter())
    }

    /// Iterate every entity matching a query, with `&mut T` access where asked.
    ///
    /// ```
    /// # use jidousha_core::{Component, World};
    /// # struct Position(i32);
    /// # impl Component for Position {}
    /// # struct Velocity(i32);
    /// # impl Component for Velocity {}
    /// # let mut world = World::new();
    /// for (_entity, position, velocity) in world.query_mut::<(&mut Position, &Velocity)>() {
    ///     position.0 += velocity.0;
    /// }
    /// ```
    ///
    /// A mutable query borrows the whole world, so nothing else can be read
    /// while it iterates: to use one entity's data when writing another, take a
    /// read pass with [`World::query`] that collects what you need, then a write
    /// pass that consumes it — the read-pass/write-pass pattern, spelled out in
    /// `docs/internal/core.md` §5 and shown working in
    /// `crates/jidousha-core/examples/homing.rs` (ADR-0013).
    ///
    /// # Panics
    ///
    /// When the query is constructed, if it names one component type twice with
    /// at least one `&mut` — such as `(&mut Position, &Position)` — since the
    /// two accesses would alias. Two shared reads of one component are allowed.
    pub fn query_mut<'w, Q: Query<'w>>(&'w mut self) -> QueryIterMut<'w, Q> {
        QueryIterMut::new(self.archetypes.all_mut().iter_mut())
    }

    /// Whether `entity` is still live in this world.
    ///
    /// False for a despawned entity, for a handle whose slot has since been
    /// reused, and for a handle from another world.
    #[must_use]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }

    /// How many entities are alive.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.archetypes.entity_count()
    }

    /// The `T` on `entity`, panicking if it has none.
    ///
    /// # Panics
    ///
    /// If `entity` is not alive or carries no `T` — both contract violations.
    /// Use [`World::find_component`] where absence is expected.
    #[must_use]
    pub fn component<T: Component>(&self, entity: Entity) -> &T {
        match self.find_component::<T>(entity) {
            Some(value) => value,
            None => panic!("{}", self.missing_component_message::<T>(entity)),
        }
    }

    /// The `T` on `entity` for modification, panicking if it has none.
    ///
    /// # Panics
    ///
    /// If `entity` is not alive or carries no `T` — both contract violations.
    /// Use [`World::find_component_mut`] where absence is expected.
    #[must_use]
    pub fn component_mut<T: Component>(&mut self, entity: Entity) -> &mut T {
        // The failure is detected through a shared borrow first: building the
        // message needs to read the world, which the returned `&mut` would
        // still be holding under the current borrow checker.
        if self.find_component::<T>(entity).is_none() {
            panic!("{}", self.missing_component_message::<T>(entity));
        }
        match self.find_component_mut::<T>(entity) {
            Some(value) => value,
            None => unreachable!(
                "[jidousha] engine bug: a component present through a shared borrow was absent \
                 through a mutable one\n  \
                 likely cause: storage lookup disagrees with itself between borrows\n  \
                 fix: report this with the reproduction — game code cannot cause it"
            ),
        }
    }

    /// The `T` on `entity`, or `None` if it has none — or is not alive.
    #[must_use]
    pub fn find_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        let location = self.find_location(entity)?;
        let column = self.archetypes.get(location.archetype).column::<T>()?;
        column.values.get(location.row)
    }

    /// The `T` on `entity` for modification, or `None` if it has none — or is
    /// not alive.
    #[must_use]
    pub fn find_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let location = self.find_location(entity)?;
        let column = self
            .archetypes
            .get_mut(location.archetype)
            .column_mut::<T>()?;
        column.values.get_mut(location.row)
    }

    /// Record structural changes to apply at the end of the running system.
    ///
    /// The recorder holds no borrow on entities or components, so a system can
    /// read the world — queries included — while recording (core.md §6).
    ///
    /// # Panics
    ///
    /// If another recorder from this world is still alive. Take one at a time:
    /// a system needs only one.
    #[must_use]
    pub fn commands(&self) -> Commands<'_> {
        match self.commands.try_borrow_mut() {
            Ok(buffer) => Commands::new(buffer),
            Err(_) => panic!("{}", SECOND_RECORDER),
        }
    }

    /// A cheap summary of the world's structure, for the Draw-phase check.
    ///
    /// Structural only — entity count and archetype layout. A component's
    /// *value* changing through interior mutability escapes this, which is why
    /// ADR-0008 puts the real enforcement in the type system and calls this
    /// defense in depth.
    pub(crate) fn shape(&self) -> (usize, usize, usize) {
        (
            self.entity_count(),
            self.archetypes.all().len(),
            self.locations.iter().flatten().count(),
        )
    }

    /// Apply everything recorded since the last application, in order.
    ///
    /// The schedule calls this after every system, which is what makes the
    /// "applied when the system returns" contract true (core.md §6). Nothing in
    /// game code needs to call it.
    pub(crate) fn apply_commands(&mut self) {
        loop {
            // Applying a command may record more (a spawned entity's setup, a
            // despawn cascade); those apply in this same flush, still in order.
            let batch = match self.commands.try_borrow_mut() {
                Ok(mut buffer) => {
                    if buffer.is_empty() {
                        return;
                    }
                    buffer.drain()
                }
                Err(_) => panic!("{}", SECOND_RECORDER),
            };
            CommandBuffer::apply(self, batch);
        }
    }

    fn insert_unchecked<T: Component>(&mut self, entity: Entity, value: T) {
        let location = self.location_of(entity);
        let target = self.archetypes.with_component::<T>(location.archetype);
        if target == location.archetype {
            // Already in this archetype: the value replaces the old one in place.
            let Some(column) = self.archetypes.get_mut(target).column_mut::<T>() else {
                unreachable!("{}", MISSING_COLUMN);
            };
            column.values[location.row] = value;
            return;
        }
        let location = self.move_entity(entity, location, target);
        let Some(column) = self.archetypes.get_mut(target).column_mut::<T>() else {
            unreachable!("{}", MISSING_COLUMN);
        };
        column.values.push(value);
        debug_assert_eq!(column.values.len() - 1, location.row);
    }

    fn remove_unchecked<T: Component>(&mut self, entity: Entity) {
        let location = self.location_of(entity);
        let target = self.archetypes.without_component::<T>(location.archetype);
        if target == location.archetype {
            // The entity never had a `T`; the end state already holds.
            return;
        }
        self.move_entity(entity, location, target);
    }

    /// Move `entity` into archetype `target`, repairing both its location and
    /// that of whatever entity backfilled its old row.
    fn move_entity(&mut self, entity: Entity, from: Location, target: usize) -> Location {
        let (row, swapped) = self.archetypes.move_entity(entity, from, target);
        let location = Location {
            archetype: target,
            row,
        };
        self.set_location(entity, Some(location));
        if let Some(swapped) = swapped {
            self.set_location(swapped, Some(from));
        }
        location
    }

    fn despawn_unchecked(&mut self, entity: Entity) {
        let location = self.location_of(entity);
        self.set_location(entity, None);
        if let Some(swapped) = self.archetypes.remove(location) {
            self.set_location(swapped, Some(location));
        }
        self.entities.destroy(entity);
    }

    fn set_location(&mut self, entity: Entity, location: Option<Location>) {
        let slot = entity.index();
        if slot >= self.locations.len() {
            self.locations.resize(slot + 1, None);
        }
        self.locations[slot] = location;
    }

    fn find_location(&self, entity: Entity) -> Option<Location> {
        if !self.entities.is_alive(entity) {
            return None;
        }
        self.locations.get(entity.index()).copied().flatten()
    }

    /// INVARIANT: every live entity has a location. Callers check liveness first.
    fn location_of(&self, entity: Entity) -> Location {
        match self.find_location(entity) {
            Some(location) => location,
            None => unreachable!(
                "[jidousha] engine bug: live {entity:?} has no location\n  \
                 likely cause: a structural operation returned without restoring the location \
                 map\n  \
                 fix: report this with the reproduction — game code cannot cause it"
            ),
        }
    }

    fn check_alive(&self, operation: &'static str, entity: Entity) -> Result<(), EntityDeadError> {
        if self.entities.is_alive(entity) {
            return Ok(());
        }
        Err(EntityDeadError::new(
            operation,
            entity,
            self.entities.slot_generation(entity),
        ))
    }

    fn expect_alive(&self, operation: &'static str, entity: Entity) {
        if let Err(error) = self.check_alive(operation, entity) {
            panic!("{error}");
        }
    }

    fn missing_component_message<T: Component>(&self, entity: Entity) -> String {
        if let Err(error) = self.check_alive("component access", entity) {
            return error.to_string();
        }
        message(
            &format!(
                "component access failed: {} not present on {entity:?}",
                core::any::type_name::<T>()
            ),
            "the entity is alive but carries no component of that type",
            "the entity was spawned without it, or a previous operation removed it",
            "use world.find_component::<T>(entity) if absence is expected here, or insert the \
             component at spawn",
        )
    }
}

/// Panic text for a second live command recorder.
const SECOND_RECORDER: &str = "[jidousha] this world already has a command recorder in use\n  \
     world.commands() was called while an earlier recorder is still alive\n  \
     likely cause: two `let mut commands = world.commands()` bindings in one system\n  \
     fix: take one recorder per system and record everything through it";

/// Panic text for an archetype that lacks a column its component set promises.
const MISSING_COLUMN: &str = "[jidousha] engine bug: an archetype is missing a column for a type in its component set\n  \
     likely cause: the archetype was created without a column for every type id\n  \
     fix: report this with the reproduction — game code cannot cause it";
