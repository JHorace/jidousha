//! What a query declares it needs, and what an archetype lends it.
//!
//! Key types: `QueryAccess`, `ColumnsMut`, `ColumnsRef`.
//! Depends on: `archetype`, `component`, `error`. Must never depend on:
//! `query` — this side describes access; `query.rs` builds iterators from it.
//! INVARIANT: a component is lent exclusively at most once per archetype per
//! query. `QueryAccess::validate` rejects the query shapes that would break
//! that before any column is touched (ADR-0013).

use core::any::{TypeId, type_name};

use crate::archetype::{Archetype, Column, typed, typed_mut};
use crate::component::Component;
use crate::error::message;

/// One component access a query part declared.
struct Borrow {
    type_id: TypeId,
    name: &'static str,
    /// Which part of the query tuple asked for it, counting from zero.
    position: usize,
    /// `&mut T` rather than `&T`.
    exclusive: bool,
}

impl Borrow {
    /// How the part spells the access, for the error message.
    fn spelling(&self) -> String {
        if self.exclusive {
            format!("&mut {}", self.name)
        } else {
            format!("&{}", self.name)
        }
    }
}

/// What a query needs from an archetype, declared by its parts.
///
/// Each part records the components it reads, writes, or merely filters on;
/// the world then decides which archetypes match. Keeping the access set a
/// value — rather than a predicate buried in the trait — is what makes the
/// conflict check possible before iteration starts, and is what a future
/// parallel scheduler needs to build conflict graphs (ADR-0007).
pub struct QueryAccess {
    borrowed: Vec<Borrow>,
    borrowed_type_ids: Vec<TypeId>,
    with: Vec<TypeId>,
    without: Vec<TypeId>,
    /// The part currently declaring; tuples set it before each part.
    position: usize,
}

impl QueryAccess {
    pub(crate) fn new() -> Self {
        Self {
            borrowed: Vec::new(),
            borrowed_type_ids: Vec::new(),
            with: Vec::new(),
            without: Vec::new(),
            position: 0,
        }
    }

    /// Set which part of the query tuple is declaring next.
    pub fn at(&mut self, position: usize) -> &mut Self {
        self.position = position;
        self
    }

    /// Declare that the query reads `T`.
    pub fn reads<T: Component>(&mut self) {
        self.borrow::<T>(false);
    }

    /// Declare that the query writes `T`.
    pub fn writes<T: Component>(&mut self) {
        self.borrow::<T>(true);
    }

    /// Declare that only entities carrying `T` match.
    pub fn requires<T: Component>(&mut self) {
        self.with.push(TypeId::of::<T>());
    }

    /// Declare that only entities *not* carrying `T` match.
    pub fn excludes<T: Component>(&mut self) {
        self.without.push(TypeId::of::<T>());
    }

    fn borrow<T: Component>(&mut self, exclusive: bool) {
        self.borrowed.push(Borrow {
            type_id: TypeId::of::<T>(),
            name: type_name::<T>(),
            position: self.position,
            exclusive,
        });
        self.borrowed_type_ids.push(TypeId::of::<T>());
    }

    pub(crate) fn borrowed_type_ids(&self) -> &[TypeId] {
        &self.borrowed_type_ids
    }

    /// Reject a query whose parts would alias each other.
    ///
    /// CONTRACT: this runs when the query is *constructed*, not when it first
    /// yields, so an empty world reports the mistake exactly like a full one —
    /// the first test run of a new system surfaces it whether or not anything
    /// has been spawned yet (ADR-0013).
    ///
    /// Two shared reads of one component are fine; anything involving a write
    /// is not.
    ///
    /// # Panics
    ///
    /// If two parts access the same component and either one writes.
    pub(crate) fn validate(&self) {
        for (index, part) in self.borrowed.iter().enumerate() {
            for other in &self.borrowed[index + 1..] {
                if part.type_id == other.type_id && (part.exclusive || other.exclusive) {
                    panic!("{}", conflicting_access_message(part, other));
                }
            }
        }
    }

    pub(crate) fn matches(&self, archetype: &Archetype) -> bool {
        self.borrowed_type_ids
            .iter()
            .chain(&self.with)
            .all(|type_id| archetype.contains(*type_id))
            && !self
                .without
                .iter()
                .any(|type_id| archetype.contains(*type_id))
    }
}

fn conflicting_access_message(part: &Borrow, other: &Borrow) -> String {
    message(
        &format!(
            "query accesses the component {} twice: parts {} and {}",
            part.name, part.position, other.position
        ),
        &format!(
            "part {} takes {}, part {} takes {} — the two accesses would alias",
            part.position,
            part.spelling(),
            other.position,
            other.spelling()
        ),
        "the query tuple lists the same component type more than once, such as \
         (&mut Position, &Position)",
        &format!(
            "keep one access to {} in this query — a `&mut` access already lets you read it",
            part.name
        ),
    )
}

/// How much of a column a query part still holds.
enum Lent<'w> {
    Exclusive(&'w mut dyn Column),
    /// Downgraded by the first `&T` part; further `&T` parts copy it.
    Shared(&'w dyn Column),
}

/// The columns one archetype lends to a query that may write.
///
/// A column is lent exclusively once, or shared any number of times.
/// [`QueryAccess::validate`] has already rejected the combinations that would
/// alias, so the panics here report engine bugs, not game mistakes.
pub struct ColumnsMut<'w> {
    columns: Vec<Option<(TypeId, Lent<'w>)>>,
}

impl<'w> ColumnsMut<'w> {
    pub(crate) fn new(columns: Vec<(TypeId, &'w mut dyn Column)>) -> Self {
        Self {
            columns: columns
                .into_iter()
                .map(|(type_id, column)| Some((type_id, Lent::Exclusive(column))))
                .collect(),
        }
    }

    fn slot(&mut self, wanted: TypeId) -> &mut Option<(TypeId, Lent<'w>)> {
        let index = self
            .columns
            .iter()
            .position(|slot| slot.as_ref().is_some_and(|(type_id, _)| *type_id == wanted));
        match index {
            Some(index) => &mut self.columns[index],
            None => unreachable!("{}", UNAVAILABLE_COLUMN),
        }
    }

    /// Take exclusive access to the values of `T` in this archetype.
    pub fn take_mut<T: Component>(&mut self) -> &'w mut Vec<T> {
        let slot = self.slot(TypeId::of::<T>());
        match slot.take() {
            Some((_, Lent::Exclusive(column))) => &mut typed_mut::<T>(column).values,
            _ => unreachable!("{}", UNAVAILABLE_COLUMN),
        }
    }

    /// Take shared access to the values of `T` in this archetype.
    ///
    /// The first caller downgrades the exclusive borrow to a shared one, which
    /// later callers copy — that is what makes `(&T, &T)` legal.
    pub fn take_ref<T: Component>(&mut self) -> &'w Vec<T> {
        let slot = self.slot(TypeId::of::<T>());
        let shared = match slot.take() {
            Some((type_id, Lent::Exclusive(column))) => {
                let shared: &'w dyn Column = column;
                *slot = Some((type_id, Lent::Shared(shared)));
                shared
            }
            Some((type_id, Lent::Shared(shared))) => {
                *slot = Some((type_id, Lent::Shared(shared)));
                shared
            }
            None => unreachable!("{}", UNAVAILABLE_COLUMN),
        };
        &typed::<T>(shared).values
    }
}

/// The columns one archetype lends to a read-only query.
///
/// Every borrow here is shared, so any number of parts may take the same one.
pub struct ColumnsRef<'w> {
    columns: Vec<(TypeId, &'w dyn Column)>,
}

impl<'w> ColumnsRef<'w> {
    pub(crate) fn new(columns: Vec<(TypeId, &'w dyn Column)>) -> Self {
        Self { columns }
    }

    /// Take shared access to the values of `T` in this archetype.
    pub fn take<T: Component>(&mut self) -> &'w Vec<T> {
        let wanted = TypeId::of::<T>();
        match self.columns.iter().find(|(type_id, _)| *type_id == wanted) {
            Some((_, column)) => &typed::<T>(*column).values,
            None => unreachable!("{}", UNAVAILABLE_COLUMN),
        }
    }
}

/// Panic text for a column the query layer asked for and cannot have.
const UNAVAILABLE_COLUMN: &str = "[jidousha] engine bug: a query part asked for a column that was absent or already lent \
     exclusively\n  \
     likely cause: QueryAccess::validate accepted a query whose parts alias, or an archetype \
     matched without holding the component\n  \
     fix: report this with the reproduction — game code cannot cause it";
