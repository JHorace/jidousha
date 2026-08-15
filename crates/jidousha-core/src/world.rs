//! The world: entities, their components, and every structural operation.
//!
//! Key types: `World`.
//! Depends on: `entity`, `storage`, `component`, `error`. Must never depend on:
//! any other jidousha crate (core.md §1 CONTRACT).
//! INVARIANT: `rows` maps an entity's slot to its row in the table for exactly
//! the live entities, and `table.row_count()` equals the number of live
//! entities. Every structural operation restores both before returning.

use crate::component::Component;
use crate::entity::{Entity, EntityAllocator};
use crate::error::{EntityDeadError, message};
use crate::storage::{Row, Table};

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
/// struct Health(u32);
/// impl Component for Health {}
///
/// let mut world = World::new();
/// let player = world.spawn();
/// world.insert(player, Health(100));
///
/// assert_eq!(world.component::<Health>(player), &Health(100));
/// assert!(world.is_alive(player));
///
/// world.despawn(player);
/// assert!(!world.is_alive(player));
/// ```
pub struct World {
    entities: EntityAllocator,
    /// Slot index → row, `None` for slots holding no live entity.
    rows: Vec<Option<Row>>,
    table: Table,
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
            rows: Vec::new(),
            table: Table::new(),
        }
    }

    /// Create an entity with no components.
    ///
    /// The handle is a pure function of this world's operation history: the
    /// same sequence of spawns and despawns yields the same handles on every
    /// platform and every run (core.md §2).
    pub fn spawn(&mut self) -> Entity {
        let entity = self.entities.create();
        let row = self.table.push_row(entity);
        let slot = entity.index();
        if slot >= self.rows.len() {
            self.rows.resize(slot + 1, None);
        }
        self.rows[slot] = Some(row);
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
    /// # Panics
    ///
    /// If `entity` is not alive — a contract violation. Use
    /// [`World::try_insert`] where absence is expected.
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) {
        self.expect_alive("insert", entity);
        let row = self.row_of(entity);
        self.table.set(row, value);
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
        let row = self.row_of(entity);
        self.table.set(row, value);
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
        let row = self.row_of(entity);
        self.table.clear::<T>(row);
    }

    /// [`World::remove`], reporting a dead entity instead of panicking.
    ///
    /// # Errors
    ///
    /// [`EntityDeadError`] if `entity` is not alive.
    pub fn try_remove<T: Component>(&mut self, entity: Entity) -> Result<(), EntityDeadError> {
        self.check_alive("remove", entity)?;
        let row = self.row_of(entity);
        self.table.clear::<T>(row);
        Ok(())
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
        self.table.row_count()
    }

    /// The `T` on `entity`.
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

    /// The `T` on `entity`, for modification.
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
        let row = self.find_row(entity)?;
        self.table.find::<T>(row)
    }

    /// The `T` on `entity` for modification, or `None` if it has none — or is
    /// not alive.
    #[must_use]
    pub fn find_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let row = self.find_row(entity)?;
        self.table.find_mut::<T>(row)
    }

    fn despawn_unchecked(&mut self, entity: Entity) {
        let row = self.row_of(entity);
        self.rows[entity.index()] = None;
        if let Some(moved) = self.table.swap_remove_row(row) {
            // Swap-remove filled the hole with the last row; that entity's
            // mapping is now stale and is the only one that can be.
            self.rows[moved.index()] = Some(row);
        }
        self.entities.destroy(entity);
    }

    fn find_row(&self, entity: Entity) -> Option<Row> {
        if !self.entities.is_alive(entity) {
            return None;
        }
        self.rows.get(entity.index()).copied().flatten()
    }

    /// INVARIANT: every live entity has a row. Callers check liveness first.
    fn row_of(&self, entity: Entity) -> Row {
        match self.find_row(entity) {
            Some(row) => row,
            None => unreachable!(
                "[jidousha] engine bug: live {entity:?} has no row in the table\n  \
                 likely cause: a structural operation returned without restoring the row mapping\n  \
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Position(i32);
    impl Component for Position {}

    #[derive(Debug, PartialEq)]
    struct Velocity(i32);
    impl Component for Velocity {}

    #[test]
    fn a_spawned_entity_is_alive_and_counted() {
        let mut world = World::new();
        let entity = world.spawn();
        assert!(world.is_alive(entity));
        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn a_despawned_entity_is_no_longer_alive_or_counted() {
        let mut world = World::new();
        let entity = world.spawn();
        world.despawn(entity);
        assert!(!world.is_alive(entity));
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn despawning_one_entity_leaves_the_others_components_intact() {
        let mut world = World::new();
        let first = world.spawn();
        let second = world.spawn();
        let third = world.spawn();
        world.insert(first, Position(1));
        world.insert(second, Position(2));
        world.insert(third, Position(3));

        world.despawn(first);

        assert_eq!(world.find_component::<Position>(second), Some(&Position(2)));
        assert_eq!(world.find_component::<Position>(third), Some(&Position(3)));
    }

    #[test]
    fn inserting_a_component_twice_keeps_the_second_value() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position(1));
        world.insert(entity, Position(2));
        assert_eq!(world.component::<Position>(entity), &Position(2));
    }

    #[test]
    fn removing_a_component_the_entity_never_had_is_not_a_failure() {
        let mut world = World::new();
        let entity = world.spawn();
        world.remove::<Position>(entity);
        assert_eq!(world.find_component::<Position>(entity), None);
    }

    #[test]
    fn components_are_independent_of_each_other() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position(1));
        world.insert(entity, Velocity(2));
        world.remove::<Position>(entity);
        assert_eq!(world.find_component::<Position>(entity), None);
        assert_eq!(world.find_component::<Velocity>(entity), Some(&Velocity(2)));
    }

    #[test]
    fn a_component_can_be_changed_through_component_mut() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, Position(1));
        world.component_mut::<Position>(entity).0 = 9;
        assert_eq!(world.component::<Position>(entity), &Position(9));
    }

    #[test]
    fn a_reused_slot_does_not_inherit_the_previous_entitys_components() {
        let mut world = World::new();
        let first = world.spawn();
        world.insert(first, Position(1));
        world.despawn(first);
        let reused = world.spawn();
        assert_eq!(world.find_component::<Position>(reused), None);
    }

    #[test]
    fn a_dead_handle_finds_no_components_even_after_its_slot_is_reused() {
        let mut world = World::new();
        let dead = world.spawn();
        world.despawn(dead);
        let reused = world.spawn();
        world.insert(reused, Position(5));
        assert_eq!(world.find_component::<Position>(dead), None);
    }

    #[test]
    fn try_despawn_reports_a_dead_entity_instead_of_panicking() {
        let mut world = World::new();
        let entity = world.spawn();
        world.despawn(entity);
        let error = world.try_despawn(entity);
        assert_eq!(error.map_err(|error| error.entity()), Err(entity));
    }

    #[test]
    fn try_insert_reports_a_dead_entity_instead_of_panicking() {
        let mut world = World::new();
        let entity = world.spawn();
        world.despawn(entity);
        assert!(world.try_insert(entity, Position(1)).is_err());
    }

    #[test]
    fn try_remove_reports_a_dead_entity_instead_of_panicking() {
        let mut world = World::new();
        let entity = world.spawn();
        world.despawn(entity);
        assert!(world.try_remove::<Position>(entity).is_err());
    }

    #[test]
    fn try_operations_succeed_on_a_live_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        assert!(world.try_insert(entity, Position(1)).is_ok());
        assert!(world.try_remove::<Position>(entity).is_ok());
        assert!(world.try_despawn(entity).is_ok());
    }

    #[test]
    #[should_panic(expected = "despawn failed")]
    fn despawning_a_dead_entity_panics() {
        let mut world = World::new();
        let entity = world.spawn();
        world.despawn(entity);
        world.despawn(entity);
    }

    #[test]
    #[should_panic(expected = "insert failed")]
    fn inserting_on_a_dead_entity_panics() {
        let mut world = World::new();
        let entity = world.spawn();
        world.despawn(entity);
        world.insert(entity, Position(1));
    }

    #[test]
    #[should_panic(expected = "remove failed")]
    fn removing_from_a_dead_entity_panics() {
        let mut world = World::new();
        let entity = world.spawn();
        world.despawn(entity);
        world.remove::<Position>(entity);
    }

    #[test]
    #[should_panic(expected = "component access failed")]
    fn reading_a_component_the_entity_lacks_panics() {
        let mut world = World::new();
        let entity = world.spawn();
        let _ = world.component::<Position>(entity);
    }

    #[test]
    #[should_panic(expected = "component access failed")]
    fn modifying_a_component_the_entity_lacks_panics() {
        let mut world = World::new();
        let entity = world.spawn();
        let _ = world.component_mut::<Position>(entity);
    }

    #[test]
    fn an_entity_from_another_world_is_not_alive_here() {
        let mut other = World::new();
        let stranger = other.spawn();
        let world = World::new();
        assert!(!world.is_alive(stranger));
        assert_eq!(world.find_component::<Position>(stranger), None);
    }
}
