//! Archetype storage: every entity lives in the archetype of its exact
//! component set, with components in dense parallel columns.
//!
//! Key types: `Archetype`, `Archetypes`, `Column`, `TypedColumn`.
//! Depends on: `component`, `entity`, `error`. Must never depend on: `world`,
//! `query` — the archetype knows rows, not handles-to-rows or query shapes.
//! INVARIANT: within an archetype, `entities` and every column have the same
//! length, and row `r` of every column belongs to `entities[r]`.
//! INVARIANT: `type_ids` is sorted and `columns` is parallel to it, so two
//! entities with the same component set always land in the same archetype
//! whatever order their components were inserted in (core.md §4).

use core::any::{Any, TypeId};

use crate::component::Component;
use crate::entity::Entity;

/// A row within one archetype. Rows are dense: removing one moves the last row
/// into the hole, so a row index is stable only until the next structural op.
pub(crate) type Row = usize;

/// Where an entity's components live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Location {
    pub(crate) archetype: usize,
    pub(crate) row: Row,
}

/// Panic text for the one way the column bookkeeping could be wrong.
const MISKEYED_COLUMN: &str = "[jidousha] engine bug: a component column does not hold the type it is keyed by\n  \
     likely cause: an archetype was built with a column and type id that disagree\n  \
     fix: report this with the reproduction — game code cannot cause it";

/// Type-erased access to one component column.
///
/// Erasure is by `Any` rather than raw pointers: this crate denies `unsafe`,
/// and a type-id comparison is cheap next to the `Vec` work it guards.
pub(crate) trait Column: Any + Send + Sync {
    /// Remove `row`, moving the last row into its place.
    fn swap_remove(&mut self, row: Row);

    /// Move `row`'s value into `dest`, appending it there and swap-removing it
    /// here. Used when an entity changes archetype.
    fn move_row_to(&mut self, row: Row, dest: &mut dyn Column);

    /// An empty column of the same component type — how a new archetype gets
    /// its columns without knowing the types statically.
    fn empty_clone(&self) -> Box<dyn Column>;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// The column holding every `T` in one archetype, indexed by row.
pub(crate) struct TypedColumn<T: Component> {
    pub(crate) values: Vec<T>,
}

impl<T: Component> TypedColumn<T> {
    pub(crate) fn new() -> Self {
        Self { values: Vec::new() }
    }
}

impl<T: Component> Column for TypedColumn<T> {
    fn swap_remove(&mut self, row: Row) {
        self.values.swap_remove(row);
    }

    fn move_row_to(&mut self, row: Row, dest: &mut dyn Column) {
        let value = self.values.swap_remove(row);
        let Some(dest) = dest.as_any_mut().downcast_mut::<TypedColumn<T>>() else {
            unreachable!("{}", MISKEYED_COLUMN);
        };
        dest.values.push(value);
    }

    fn empty_clone(&self) -> Box<dyn Column> {
        Box::new(TypedColumn::<T>::new())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Downcast a type-erased column to its component type.
///
/// INVARIANT: columns are keyed by the `TypeId` of the very type they hold, so
/// this cannot fail. A failure is never quietly reported as "no such
/// component", which would hide an engine bug as missing game data.
pub(crate) fn typed<T: Component>(column: &dyn Column) -> &TypedColumn<T> {
    let Some(column) = column.as_any().downcast_ref::<TypedColumn<T>>() else {
        unreachable!("{}", MISKEYED_COLUMN);
    };
    column
}

/// [`typed`], for exclusive access.
pub(crate) fn typed_mut<T: Component>(column: &mut dyn Column) -> &mut TypedColumn<T> {
    let Some(column) = column.as_any_mut().downcast_mut::<TypedColumn<T>>() else {
        unreachable!("{}", MISKEYED_COLUMN);
    };
    column
}

/// All entities sharing one exact component set.
pub(crate) struct Archetype {
    /// Sorted; the archetype's identity.
    type_ids: Vec<TypeId>,
    entities: Vec<Entity>,
    /// Parallel to `type_ids`.
    columns: Vec<Box<dyn Column>>,
}

impl Archetype {
    fn new(type_ids: Vec<TypeId>, columns: Vec<Box<dyn Column>>) -> Self {
        debug_assert_eq!(type_ids.len(), columns.len());
        debug_assert!(type_ids.windows(2).all(|pair| pair[0] < pair[1]));
        Self {
            type_ids,
            entities: Vec::new(),
            columns,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.entities.len()
    }

    pub(crate) fn contains(&self, type_id: TypeId) -> bool {
        self.type_ids.binary_search(&type_id).is_ok()
    }

    fn column_index(&self, type_id: TypeId) -> Option<usize> {
        self.type_ids.binary_search(&type_id).ok()
    }

    pub(crate) fn column<T: Component>(&self) -> Option<&TypedColumn<T>> {
        let index = self.column_index(TypeId::of::<T>())?;
        Some(typed::<T>(self.columns[index].as_ref()))
    }

    pub(crate) fn column_mut<T: Component>(&mut self) -> Option<&mut TypedColumn<T>> {
        let index = self.column_index(TypeId::of::<T>())?;
        Some(typed_mut::<T>(self.columns[index].as_mut()))
    }

    /// Append a row for `entity`, leaving its columns to the caller to fill.
    fn push_entity(&mut self, entity: Entity) -> Row {
        self.entities.push(entity);
        self.entities.len() - 1
    }

    /// Drop `entity`'s row entirely, returning the entity swapped into its place.
    fn swap_remove(&mut self, row: Row) -> Option<Entity> {
        self.entities.swap_remove(row);
        for column in &mut self.columns {
            column.swap_remove(row);
        }
        self.entities.get(row).copied()
    }

    /// Borrow the entity list and the named columns, one exclusive borrow each.
    ///
    /// A type id that is not in this archetype, or that is asked for twice,
    /// simply does not appear in the result — the query layer turns the second
    /// case into a message naming the repeated component.
    pub(crate) fn borrow_columns_mut<'w>(
        &'w mut self,
        wanted: &[TypeId],
    ) -> (&'w [Entity], Vec<(TypeId, &'w mut dyn Column)>) {
        // Destructured so the entity list and the columns are borrowed as the
        // separate fields they are.
        let Self {
            type_ids,
            entities,
            columns,
        } = self;
        let columns = columns
            .iter_mut()
            .enumerate()
            .filter(|(index, _)| wanted.contains(&type_ids[*index]))
            .map(|(index, column)| (type_ids[index], column.as_mut()))
            .collect();
        (entities, columns)
    }

    /// [`Archetype::borrow_columns_mut`], for shared access.
    pub(crate) fn borrow_columns<'w>(
        &'w self,
        wanted: &[TypeId],
    ) -> (&'w [Entity], Vec<(TypeId, &'w dyn Column)>) {
        let columns = self
            .columns
            .iter()
            .enumerate()
            .filter(|(index, _)| wanted.contains(&self.type_ids[*index]))
            .map(|(index, column)| (self.type_ids[index], column.as_ref()))
            .collect();
        (&self.entities, columns)
    }
}

/// Every archetype in the world, in creation order.
///
/// CONTRACT: creation order is the query visit order, and it is a pure function
/// of the world's operation history — never of type-registration order or of
/// any hash (core.md §4).
pub(crate) struct Archetypes {
    list: Vec<Archetype>,
}

impl Archetypes {
    pub(crate) fn new() -> Self {
        Self { list: Vec::new() }
    }

    pub(crate) fn get(&self, index: usize) -> &Archetype {
        &self.list[index]
    }

    pub(crate) fn get_mut(&mut self, index: usize) -> &mut Archetype {
        &mut self.list[index]
    }

    pub(crate) fn all(&self) -> &[Archetype] {
        &self.list
    }

    pub(crate) fn all_mut(&mut self) -> &mut [Archetype] {
        &mut self.list
    }

    pub(crate) fn entity_count(&self) -> usize {
        self.list.iter().map(Archetype::len).sum()
    }

    fn find(&self, type_ids: &[TypeId]) -> Option<usize> {
        self.list
            .iter()
            .position(|archetype| archetype.type_ids == type_ids)
    }

    /// The archetype for the empty component set, creating it if this is the
    /// first spawn. It is always archetype 0, since every entity starts there.
    pub(crate) fn empty_set(&mut self) -> usize {
        match self.find(&[]) {
            Some(index) => index,
            None => {
                self.list.push(Archetype::new(Vec::new(), Vec::new()));
                self.list.len() - 1
            }
        }
    }

    /// The archetype holding `source`'s component set plus `T`, created from
    /// `source`'s columns if it does not exist yet.
    pub(crate) fn with_component<T: Component>(&mut self, source: usize) -> usize {
        let added = TypeId::of::<T>();
        let mut type_ids = self.list[source].type_ids.clone();
        let Err(position) = type_ids.binary_search(&added) else {
            return source;
        };
        type_ids.insert(position, added);
        if let Some(index) = self.find(&type_ids) {
            return index;
        }
        let mut columns: Vec<Box<dyn Column>> = self.list[source]
            .columns
            .iter()
            .map(|column| column.empty_clone())
            .collect();
        columns.insert(position, Box::new(TypedColumn::<T>::new()));
        self.list.push(Archetype::new(type_ids, columns));
        self.list.len() - 1
    }

    /// The archetype holding `source`'s component set minus `T`.
    pub(crate) fn without_component<T: Component>(&mut self, source: usize) -> usize {
        let removed = TypeId::of::<T>();
        let mut type_ids = self.list[source].type_ids.clone();
        let Ok(position) = type_ids.binary_search(&removed) else {
            return source;
        };
        type_ids.remove(position);
        if let Some(index) = self.find(&type_ids) {
            return index;
        }
        let columns: Vec<Box<dyn Column>> = self.list[source]
            .columns
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != position)
            .map(|(_, column)| column.empty_clone())
            .collect();
        self.list.push(Archetype::new(type_ids, columns));
        self.list.len() - 1
    }

    /// Move `entity`'s row from `from` to `to`, carrying every component the
    /// two archetypes share and dropping the rest.
    ///
    /// Returns the new row and the entity that was swapped into the vacated
    /// row, whose location the caller owes an update.
    pub(crate) fn move_entity(
        &mut self,
        entity: Entity,
        from: Location,
        to: usize,
    ) -> (Row, Option<Entity>) {
        debug_assert_ne!(from.archetype, to);
        let Ok([source, target]) = self.list.get_disjoint_mut([from.archetype, to]) else {
            unreachable!(
                "[jidousha] engine bug: an entity was moved between overlapping archetype \
                 indices\n  likely cause: the source and target archetype resolved to the same \
                 slot\n  fix: report this with the reproduction — game code cannot cause it"
            );
        };
        let row = target.push_entity(entity);
        // Source columns are visited in order; each either has a home in the
        // target or is dropped with the row.
        for index in 0..source.type_ids.len() {
            let type_id = source.type_ids[index];
            match target.column_index(type_id) {
                Some(target_index) => {
                    let (source_column, target_column) = (
                        &mut source.columns[index],
                        &mut target.columns[target_index],
                    );
                    source_column.move_row_to(from.row, target_column.as_mut());
                }
                None => source.columns[index].swap_remove(from.row),
            }
        }
        source.entities.swap_remove(from.row);
        (row, source.entities.get(from.row).copied())
    }

    /// Place a freshly spawned `entity` in the empty archetype.
    pub(crate) fn push_new(&mut self, entity: Entity) -> Location {
        let archetype = self.empty_set();
        let row = self.list[archetype].push_entity(entity);
        Location { archetype, row }
    }

    /// Remove `entity`'s row, returning the entity swapped into its place.
    pub(crate) fn remove(&mut self, location: Location) -> Option<Entity> {
        self.list[location.archetype].swap_remove(location.row)
    }
}
