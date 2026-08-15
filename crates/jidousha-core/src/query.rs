//! Queries: iterating the entities that carry a given set of components.
//!
//! Key types: `Query`, `ReadOnlyQuery`, `With`, `Without`, `QueryIter`,
//! `QueryIterMut`.
//! Depends on: `archetype`, `component`, `entity`, `error`. Must never depend
//! on: `world` — a query is built *from* archetypes, and `World` owns the entry
//! points.
//! INVARIANT: archetypes are visited in creation order and rows in row order,
//! both pure functions of the world's operation history, so two identical
//! histories iterate identically (core.md §4 CONTRACT).
//! DELIBERATE: mutating queries take `&mut World` rather than `&World` plus
//! runtime borrow flags (see ADR-0013).

use core::any::{TypeId, type_name};
use core::marker::PhantomData;
use core::slice;

use crate::archetype::{Archetype, Column, typed, typed_mut};
use crate::component::Component;
use crate::entity::Entity;
use crate::error::message;

/// Match only entities that carry `T`, without reading it.
///
/// A filter yields `()`, so it still occupies a position in the item tuple:
///
/// ```
/// # use jidousha_core::{Component, With, World};
/// # #[derive(Debug)] struct Position(i32);
/// # impl Component for Position {}
/// # struct Player;
/// # impl Component for Player {}
/// # let mut world = World::new();
/// for (entity, position, _) in world.query::<(&Position, With<Player>)>() {
///     println!("{entity:?} is at {position:?}");
/// }
/// ```
pub struct With<T: Component>(PhantomData<fn() -> T>);

/// Match only entities that do **not** carry `T`. Yields `()`, like [`With`].
pub struct Without<T: Component>(PhantomData<fn() -> T>);

/// What a query needs from an archetype, declared by its parts.
///
/// Each part records the components it reads, writes, or merely filters on;
/// the world then decides which archetypes match. Keeping the access set a
/// value — rather than a predicate buried in the trait — is also what a future
/// parallel scheduler needs to build conflict graphs (ADR-0007).
pub struct QueryAccess {
    borrowed: Vec<TypeId>,
    with: Vec<TypeId>,
    without: Vec<TypeId>,
}

impl QueryAccess {
    pub(crate) fn new() -> Self {
        Self {
            borrowed: Vec::new(),
            with: Vec::new(),
            without: Vec::new(),
        }
    }

    /// Declare that the query borrows `T`'s column, to read or to write.
    pub fn borrows<T: Component>(&mut self) {
        self.borrowed.push(TypeId::of::<T>());
    }

    /// Declare that only entities carrying `T` match.
    pub fn requires<T: Component>(&mut self) {
        self.with.push(TypeId::of::<T>());
    }

    /// Declare that only entities *not* carrying `T` match.
    pub fn excludes<T: Component>(&mut self) {
        self.without.push(TypeId::of::<T>());
    }

    pub(crate) fn borrowed_type_ids(&self) -> &[TypeId] {
        &self.borrowed
    }

    pub(crate) fn matches(&self, archetype: &Archetype) -> bool {
        self.borrowed
            .iter()
            .chain(&self.with)
            .all(|type_id| archetype.contains(*type_id))
            && !self
                .without
                .iter()
                .any(|type_id| archetype.contains(*type_id))
    }
}

/// The columns one archetype lends to one query, exclusively.
///
/// Each column can be taken exactly once: naming the same component twice in a
/// query would otherwise alias it.
pub struct ColumnsMut<'w> {
    columns: Vec<Option<(TypeId, &'w mut dyn Column)>>,
}

impl<'w> ColumnsMut<'w> {
    pub(crate) fn new(columns: Vec<(TypeId, &'w mut dyn Column)>) -> Self {
        Self {
            columns: columns.into_iter().map(Some).collect(),
        }
    }

    /// Take exclusive access to the values of `T` in this archetype.
    ///
    /// # Panics
    ///
    /// If `T` was already taken — the query names it more than once.
    pub fn take<T: Component>(&mut self) -> &'w mut Vec<T> {
        match take_column(&mut self.columns, TypeId::of::<T>()) {
            Some(column) => &mut typed_mut::<T>(column).values,
            None => panic!("{}", repeated_component_message::<T>()),
        }
    }
}

/// The columns one archetype lends to a read-only query.
pub struct ColumnsRef<'w> {
    columns: Vec<Option<(TypeId, &'w dyn Column)>>,
}

impl<'w> ColumnsRef<'w> {
    pub(crate) fn new(columns: Vec<(TypeId, &'w dyn Column)>) -> Self {
        Self {
            columns: columns.into_iter().map(Some).collect(),
        }
    }

    /// Take shared access to the values of `T` in this archetype.
    ///
    /// # Panics
    ///
    /// If `T` was already taken — the query names it more than once.
    pub fn take<T: Component>(&mut self) -> &'w Vec<T> {
        match take_column(&mut self.columns, TypeId::of::<T>()) {
            Some(column) => &typed::<T>(column).values,
            None => panic!("{}", repeated_component_message::<T>()),
        }
    }
}

fn take_column<C>(columns: &mut [Option<(TypeId, C)>], wanted: TypeId) -> Option<C> {
    columns
        .iter_mut()
        .find(|slot| slot.as_ref().is_some_and(|(type_id, _)| *type_id == wanted))
        .and_then(Option::take)
        .map(|(_, column)| column)
}

fn repeated_component_message<T: Component>() -> String {
    let name = type_name::<T>();
    message(
        &format!("query names the component {name} more than once"),
        "a component can be accessed once per query, or its two accesses could alias",
        "the query tuple lists the same type twice, such as (&mut Position, &Position)",
        &format!(
            "keep one access to {name} in this query — a `&mut` access already lets you read it"
        ),
    )
}

/// What a query asks of each entity: a component access, a filter, or a tuple
/// of them.
///
/// Implemented for `&T`, `&mut T`, [`With`], [`Without`], and tuples of up to
/// six of those. Not implemented by game code.
pub trait Query<'w>: Sized {
    /// What this part of the query yields for one entity.
    type Item;

    /// Iteration state over one archetype's rows.
    type Cursor;

    /// The flat item the iterator yields: the entity, then every part.
    type Yield;

    /// Declare what this part of the query needs from an archetype.
    fn access(access: &mut QueryAccess);

    /// Build the cursor from columns borrowed exclusively.
    fn cursor(columns: &mut ColumnsMut<'w>) -> Self::Cursor;

    /// Advance one row.
    fn next(cursor: &mut Self::Cursor) -> Option<Self::Item>;

    /// Assemble the yielded item from the entity and this query's parts.
    fn yield_item(entity: Entity, item: Self::Item) -> Self::Yield;
}

/// A query that reads and never writes, so it can run against a shared world.
///
/// This is what makes [`World::query`](crate::World::query) safe to call while
/// other parts of the world are being read, and is the bound the read-only
/// Draw-phase world view will use (ADR-0008).
#[diagnostic::on_unimplemented(
    message = "[jidousha] `{Self}` writes components, so it cannot be used in a read-only query",
    label = "this query needs exclusive access",
    note = "likely cause: `&mut T` appears in a `world.query::<...>()` call",
    note = "fix: use `world.query_mut::<...>()` for `&mut T` access, or drop the `mut` to read"
)]
pub trait ReadOnlyQuery<'w>: Query<'w> {
    /// Build the cursor from columns borrowed shared.
    fn cursor_shared(columns: &mut ColumnsRef<'w>) -> Self::Cursor;
}

impl<'w, T: Component> Query<'w> for &'w T {
    type Item = &'w T;
    type Cursor = slice::Iter<'w, T>;
    type Yield = (Entity, &'w T);

    fn access(access: &mut QueryAccess) {
        access.borrows::<T>();
    }

    fn cursor(columns: &mut ColumnsMut<'w>) -> Self::Cursor {
        columns.take::<T>().iter()
    }

    fn next(cursor: &mut Self::Cursor) -> Option<Self::Item> {
        cursor.next()
    }

    fn yield_item(entity: Entity, item: Self::Item) -> Self::Yield {
        (entity, item)
    }
}

impl<'w, T: Component> ReadOnlyQuery<'w> for &'w T {
    fn cursor_shared(columns: &mut ColumnsRef<'w>) -> Self::Cursor {
        columns.take::<T>().iter()
    }
}

impl<'w, T: Component> Query<'w> for &'w mut T {
    type Item = &'w mut T;
    type Cursor = slice::IterMut<'w, T>;
    type Yield = (Entity, &'w mut T);

    fn access(access: &mut QueryAccess) {
        access.borrows::<T>();
    }

    fn cursor(columns: &mut ColumnsMut<'w>) -> Self::Cursor {
        columns.take::<T>().iter_mut()
    }

    fn next(cursor: &mut Self::Cursor) -> Option<Self::Item> {
        cursor.next()
    }

    fn yield_item(entity: Entity, item: Self::Item) -> Self::Yield {
        (entity, item)
    }
}

impl<'w, T: Component> Query<'w> for With<T> {
    type Item = ();
    type Cursor = ();
    type Yield = (Entity, ());

    fn access(access: &mut QueryAccess) {
        access.requires::<T>();
    }

    fn cursor(_columns: &mut ColumnsMut<'w>) -> Self::Cursor {}

    fn next(_cursor: &mut Self::Cursor) -> Option<Self::Item> {
        Some(())
    }

    fn yield_item(entity: Entity, item: Self::Item) -> Self::Yield {
        (entity, item)
    }
}

impl<'w, T: Component> ReadOnlyQuery<'w> for With<T> {
    fn cursor_shared(_columns: &mut ColumnsRef<'w>) -> Self::Cursor {}
}

impl<'w, T: Component> Query<'w> for Without<T> {
    type Item = ();
    type Cursor = ();
    type Yield = (Entity, ());

    fn access(access: &mut QueryAccess) {
        access.excludes::<T>();
    }

    fn cursor(_columns: &mut ColumnsMut<'w>) -> Self::Cursor {}

    fn next(_cursor: &mut Self::Cursor) -> Option<Self::Item> {
        Some(())
    }

    fn yield_item(entity: Entity, item: Self::Item) -> Self::Yield {
        (entity, item)
    }
}

impl<'w, T: Component> ReadOnlyQuery<'w> for Without<T> {
    fn cursor_shared(_columns: &mut ColumnsRef<'w>) -> Self::Cursor {}
}

/// Implement `Query`/`ReadOnlyQuery` for one tuple arity.
///
/// DELIBERATE: a macro, but one that generates no public symbols — the trait
/// and its methods are written out above, and only the per-arity impls repeat
/// (agent-practices §5.4).
macro_rules! impl_query_tuple {
    ($($part:ident $field:ident),+) => {
        impl<'w, $($part: Query<'w>),+> Query<'w> for ($($part,)+) {
            type Item = ($($part::Item,)+);
            type Cursor = ($($part::Cursor,)+);
            type Yield = (Entity, $($part::Item,)+);

            fn access(access: &mut QueryAccess) {
                $($part::access(access);)+
            }

            fn cursor(columns: &mut ColumnsMut<'w>) -> Self::Cursor {
                ($($part::cursor(columns),)+)
            }

            fn next(cursor: &mut Self::Cursor) -> Option<Self::Item> {
                let ($($field,)+) = cursor;
                Some(($($part::next($field)?,)+))
            }

            fn yield_item(entity: Entity, item: Self::Item) -> Self::Yield {
                let ($($field,)+) = item;
                (entity, $($field,)+)
            }
        }

        impl<'w, $($part: ReadOnlyQuery<'w>),+> ReadOnlyQuery<'w> for ($($part,)+) {
            fn cursor_shared(columns: &mut ColumnsRef<'w>) -> Self::Cursor {
                ($($part::cursor_shared(columns),)+)
            }
        }
    };
}

impl_query_tuple!(A a);
impl_query_tuple!(A a, B b);
impl_query_tuple!(A a, B b, C c);
impl_query_tuple!(A a, B b, C c, D d);
impl_query_tuple!(A a, B b, C c, D d, E e);
impl_query_tuple!(A a, B b, C c, D d, E e, F f);

/// Iteration state for the archetype a query is currently walking.
struct Walk<'w, Q: Query<'w>> {
    entities: slice::Iter<'w, Entity>,
    cursor: Q::Cursor,
}

impl<'w, Q: Query<'w>> Walk<'w, Q> {
    fn next(&mut self) -> Option<Q::Yield> {
        let entity = *self.entities.next()?;
        let Some(item) = Q::next(&mut self.cursor) else {
            unreachable!(
                "[jidousha] engine bug: an archetype column is shorter than its entity list\n  \
                 likely cause: a structural operation updated the entity list without its \
                 columns\n  \
                 fix: report this with the reproduction — game code cannot cause it"
            );
        };
        Some(Q::yield_item(entity, item))
    }
}

/// The iterator returned by [`World::query`](crate::World::query).
///
/// Archetypes are visited in creation order and rows in row order — the same
/// operation history always iterates the same way (core.md §4).
pub struct QueryIter<'w, Q: ReadOnlyQuery<'w>> {
    archetypes: slice::Iter<'w, Archetype>,
    walk: Option<Walk<'w, Q>>,
    access: QueryAccess,
}

impl<'w, Q: ReadOnlyQuery<'w>> QueryIter<'w, Q> {
    pub(crate) fn new(archetypes: slice::Iter<'w, Archetype>) -> Self {
        let mut access = QueryAccess::new();
        Q::access(&mut access);
        Self {
            archetypes,
            walk: None,
            access,
        }
    }

    fn next_archetype(&mut self) -> bool {
        for archetype in self.archetypes.by_ref() {
            if archetype.len() == 0 || !self.access.matches(archetype) {
                continue;
            }
            let (entities, columns) = archetype.borrow_columns(self.access.borrowed_type_ids());
            let mut columns = ColumnsRef::new(columns);
            self.walk = Some(Walk {
                entities: entities.iter(),
                cursor: Q::cursor_shared(&mut columns),
            });
            return true;
        }
        false
    }
}

impl<'w, Q: ReadOnlyQuery<'w>> Iterator for QueryIter<'w, Q> {
    type Item = Q::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(walk) = self.walk.as_mut()
                && let Some(item) = walk.next()
            {
                return Some(item);
            }
            self.walk = None;
            if !self.next_archetype() {
                return None;
            }
        }
    }
}

/// The iterator returned by [`World::query_mut`](crate::World::query_mut).
///
/// Same visit order as [`QueryIter`]; the difference is exclusive access, which
/// is what lets a part of the query be `&mut T`.
pub struct QueryIterMut<'w, Q: Query<'w>> {
    archetypes: slice::IterMut<'w, Archetype>,
    walk: Option<Walk<'w, Q>>,
    access: QueryAccess,
}

impl<'w, Q: Query<'w>> QueryIterMut<'w, Q> {
    pub(crate) fn new(archetypes: slice::IterMut<'w, Archetype>) -> Self {
        let mut access = QueryAccess::new();
        Q::access(&mut access);
        Self {
            archetypes,
            walk: None,
            access,
        }
    }

    fn next_archetype(&mut self) -> bool {
        for archetype in self.archetypes.by_ref() {
            if archetype.len() == 0 || !self.access.matches(archetype) {
                continue;
            }
            let (entities, columns) = archetype.borrow_columns_mut(self.access.borrowed_type_ids());
            let mut columns = ColumnsMut::new(columns);
            self.walk = Some(Walk {
                entities: entities.iter(),
                cursor: Q::cursor(&mut columns),
            });
            return true;
        }
        false
    }
}

impl<'w, Q: Query<'w>> Iterator for QueryIterMut<'w, Q> {
    type Item = Q::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(walk) = self.walk.as_mut()
                && let Some(item) = walk.next()
            {
                return Some(item);
            }
            self.walk = None;
            if !self.next_archetype() {
                return None;
            }
        }
    }
}
