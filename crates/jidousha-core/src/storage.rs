//! Component storage: the single table every live entity occupies in M1.
//!
//! Key types: `Table`, `Column`, `TypedColumn`.
//! Depends on: `component`, `entity`. Must never depend on: `world` — the table
//! knows rows, not handles-to-rows.
//! INVARIANT: every column has exactly one slot per row, so a row index is
//! valid in all columns at once. Despawn swap-removes the same row from every
//! column, which keeps rows dense and structural ops O(1).
//! DELIBERATE: one table for all entities, with an absent-slot `Option` per
//! row, rather than one table per component set (see ADR-0006). The archetype
//! graph lands in M2; M1 exists to pin the observable semantics — handles,
//! liveness, and per-entity component state — against the reference model
//! before storage gets clever.

use core::any::{Any, TypeId};

use crate::component::Component;
use crate::entity::Entity;

/// A row in the table. Rows are dense: removing one moves the last row into
/// the hole, so a row index is stable only until the next despawn.
pub(crate) type Row = usize;

/// Panic text for the one way the column bookkeeping could be wrong.
const MISKEYED_COLUMN: &str = "[jidousha] engine bug: a component column is stored under a TypeId that is not its own\n  \
     likely cause: Table::set built a column with a key from a different type\n  \
     fix: report this with the reproduction — game code cannot cause it";

/// Type-erased access to one component column.
///
/// Erasure is by `Any` rather than raw pointers: this crate denies `unsafe`,
/// and downcasting costs a type-id comparison on operations that already touch
/// a `Vec`.
trait Column: Any + Send + Sync {
    /// Append one absent slot, keeping the column's length equal to the table's.
    fn push_absent(&mut self);

    /// Remove `row`, moving the last row into its place.
    fn swap_remove(&mut self, row: Row);

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// The column holding every `T` in the table, indexed by row.
struct TypedColumn<T: Component> {
    values: Vec<Option<T>>,
}

impl<T: Component> Column for TypedColumn<T> {
    fn push_absent(&mut self) {
        self.values.push(None);
    }

    fn swap_remove(&mut self, row: Row) {
        self.values.swap_remove(row);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Every live entity, one row each, with one column per component type in use.
///
/// INVARIANT: `entities.len()` is the row count, and every column holds exactly
/// that many slots.
pub(crate) struct Table {
    entities: Vec<Entity>,
    /// Columns in the order their component type was first used.
    ///
    /// DELIBERATE: a `Vec` of pairs rather than a `HashMap` keyed by `TypeId`
    /// (see ADR-0006). Hash iteration order varies between runs and platforms,
    /// which would leak into query order in M2 and break the determinism
    /// contract (core.md §4). A linear scan over a handful of component types
    /// is also faster than hashing.
    columns: Vec<(TypeId, Box<dyn Column>)>,
}

impl Table {
    pub(crate) fn new() -> Self {
        Self {
            entities: Vec::new(),
            columns: Vec::new(),
        }
    }

    pub(crate) fn row_count(&self) -> usize {
        self.entities.len()
    }

    /// Append a row for `entity` and return its index.
    pub(crate) fn push_row(&mut self, entity: Entity) -> Row {
        self.entities.push(entity);
        for (_, column) in &mut self.columns {
            column.push_absent();
        }
        self.entities.len() - 1
    }

    /// Remove `row` from every column, returning the entity that was moved into
    /// the hole — the caller owes that entity's row mapping an update.
    pub(crate) fn swap_remove_row(&mut self, row: Row) -> Option<Entity> {
        self.entities.swap_remove(row);
        for (_, column) in &mut self.columns {
            column.swap_remove(row);
        }
        self.entities.get(row).copied()
    }

    /// Store `value` at `row`, replacing whatever `T` was there.
    pub(crate) fn set<T: Component>(&mut self, row: Row, value: T) {
        let index = match self.column_index::<T>() {
            Some(index) => index,
            None => {
                let mut values = Vec::with_capacity(self.row_count());
                values.resize_with(self.row_count(), || None);
                self.columns
                    .push((TypeId::of::<T>(), Box::new(TypedColumn::<T> { values })));
                self.columns.len() - 1
            }
        };
        self.typed_mut::<T>(index).values[row] = Some(value);
    }

    /// Clear any `T` at `row`. Absence afterwards is the whole point, so a row
    /// that had no `T` is left as it is.
    pub(crate) fn clear<T: Component>(&mut self, row: Row) {
        let Some(index) = self.column_index::<T>() else {
            return;
        };
        self.typed_mut::<T>(index).values[row] = None;
    }

    pub(crate) fn find<T: Component>(&self, row: Row) -> Option<&T> {
        let index = self.column_index::<T>()?;
        let Some(column) = self.columns[index]
            .1
            .as_any()
            .downcast_ref::<TypedColumn<T>>()
        else {
            unreachable!("{}", MISKEYED_COLUMN);
        };
        column.values[row].as_ref()
    }

    pub(crate) fn find_mut<T: Component>(&mut self, row: Row) -> Option<&mut T> {
        let index = self.column_index::<T>()?;
        self.typed_mut::<T>(index).values[row].as_mut()
    }

    /// INVARIANT: a column is stored under `TypeId::of::<T>()` of the very type
    /// its `TypedColumn` holds, so this downcast cannot fail. A failure would
    /// mean the column index and the key disagree — never silently treated as
    /// "the component is absent", which would hide the bug as missing data.
    fn typed_mut<T: Component>(&mut self, index: usize) -> &mut TypedColumn<T> {
        let Some(column) = self.columns[index]
            .1
            .as_any_mut()
            .downcast_mut::<TypedColumn<T>>()
        else {
            unreachable!("{}", MISKEYED_COLUMN);
        };
        column
    }

    fn column_index<T: Component>(&self) -> Option<usize> {
        let wanted = TypeId::of::<T>();
        self.columns
            .iter()
            .position(|(type_id, _)| *type_id == wanted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Position(i32);
    impl Component for Position {}

    #[derive(Debug, PartialEq)]
    struct Frozen;
    impl Component for Frozen {}

    fn table_with_rows(count: usize) -> (Table, Vec<Entity>) {
        let mut allocator = crate::entity::EntityAllocator::new();
        let mut table = Table::new();
        let entities: Vec<Entity> = (0..count)
            .map(|_| {
                let entity = allocator.create();
                table.push_row(entity);
                entity
            })
            .collect();
        (table, entities)
    }

    #[test]
    fn a_row_added_after_a_column_exists_starts_without_that_component() {
        let (mut table, _) = table_with_rows(1);
        table.set(0, Position(7));
        let mut allocator = crate::entity::EntityAllocator::new();
        let row = table.push_row(allocator.create());
        assert_eq!(table.find::<Position>(row), None);
    }

    #[test]
    fn a_column_created_after_rows_exist_starts_absent_on_every_row() {
        let (mut table, _) = table_with_rows(3);
        table.set(2, Position(1));
        assert_eq!(table.find::<Position>(0), None);
        assert_eq!(table.find::<Position>(1), None);
        assert_eq!(table.find::<Position>(2), Some(&Position(1)));
    }

    #[test]
    fn setting_a_component_twice_replaces_the_first_value() {
        let (mut table, _) = table_with_rows(1);
        table.set(0, Position(1));
        table.set(0, Position(2));
        assert_eq!(table.find::<Position>(0), Some(&Position(2)));
    }

    #[test]
    fn clearing_a_component_that_was_never_there_leaves_the_row_alone() {
        let (mut table, _) = table_with_rows(1);
        table.clear::<Position>(0);
        assert_eq!(table.find::<Position>(0), None);
    }

    #[test]
    fn removing_a_row_moves_the_last_row_into_the_hole() {
        let (mut table, entities) = table_with_rows(3);
        table.set(0, Position(10));
        table.set(2, Position(30));
        let moved = table.swap_remove_row(0);
        assert_eq!(moved, Some(entities[2]));
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.find::<Position>(0), Some(&Position(30)));
    }

    #[test]
    fn removing_the_last_row_moves_nothing() {
        let (mut table, _) = table_with_rows(2);
        assert_eq!(table.swap_remove_row(1), None);
        assert_eq!(table.row_count(), 1);
    }

    #[test]
    fn zero_sized_components_are_stored_like_any_other() {
        let (mut table, _) = table_with_rows(2);
        table.set(1, Frozen);
        assert_eq!(table.find::<Frozen>(0), None);
        assert_eq!(table.find::<Frozen>(1), Some(&Frozen));
    }

    #[test]
    fn a_component_can_be_changed_through_a_mutable_borrow() {
        let (mut table, _) = table_with_rows(1);
        table.set(0, Position(1));
        if let Some(position) = table.find_mut::<Position>(0) {
            position.0 = 42;
        }
        assert_eq!(table.find::<Position>(0), Some(&Position(42)));
    }
}
