//! Resources: typed singletons for world-global state.
//!
//! Key types: `Resource`, `Resources`.
//! Depends on: `error`, `world`. The store itself knows nothing about the
//! world; the `impl World` block at the bottom is the world's resource API,
//! kept here beside the store it reaches rather than swelling `world.rs`.
//! INVARIANT: resources live in a `Vec` in insertion order and are found by
//! linear scan. No hash map: nothing in the engine may iterate one where the
//! order could reach observable state (core.md §4).

use core::any::{Any, TypeId, type_name};

use crate::error::message;
use crate::world::World;

/// Marks a type as storable as a world resource.
///
/// Like [`Component`](crate::Component) the trait is a marker with no items,
/// implemented in one line:
///
/// ```
/// use jidousha_core::Resource;
///
/// struct Score(u32);
/// impl Resource for Score {}
/// ```
///
/// A resource is a singleton: the world holds at most one value per type. Use
/// one for state the whole simulation shares — the score, the current level,
/// tuning constants — and components for anything an entity owns.
pub trait Resource: 'static + Send + Sync {}

/// Every resource in the world.
pub(crate) struct Resources {
    /// Insertion order; scanned linearly, never hashed.
    values: Vec<(TypeId, Box<dyn Any + Send + Sync>)>,
}

impl Resources {
    pub(crate) fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Store `value`, replacing any resource of the same type.
    pub(crate) fn insert<T: Resource>(&mut self, value: T) {
        let type_id = TypeId::of::<T>();
        match self.values.iter_mut().find(|(id, _)| *id == type_id) {
            Some(slot) => slot.1 = Box::new(value),
            None => self.values.push((type_id, Box::new(value))),
        }
    }

    /// Drop the `T` resource if there is one.
    ///
    /// CONTRACT: like `World::remove` for components, this states an end state
    /// — the world has no `T` afterwards — so removing one that was never
    /// there is not a failure.
    pub(crate) fn remove<T: Resource>(&mut self) {
        let type_id = TypeId::of::<T>();
        self.values.retain(|(id, _)| *id != type_id);
    }

    pub(crate) fn find<T: Resource>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let value = &self.values.iter().find(|(id, _)| *id == type_id)?.1;
        match value.downcast_ref::<T>() {
            Some(value) => Some(value),
            None => unreachable!("{}", MISKEYED_RESOURCE),
        }
    }

    pub(crate) fn find_mut<T: Resource>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        let value = &mut self.values.iter_mut().find(|(id, _)| *id == type_id)?.1;
        match value.downcast_mut::<T>() {
            Some(value) => Some(value),
            None => unreachable!("{}", MISKEYED_RESOURCE),
        }
    }
}

impl World {
    /// Store a resource, replacing any of the same type.
    pub fn insert_resource<T: Resource>(&mut self, value: T) {
        self.resources.insert(value);
    }

    /// Drop the `T` resource.
    ///
    /// CONTRACT: like [`World::remove`] this states an end state, so removing a
    /// resource the world never had is not a failure.
    pub fn remove_resource<T: Resource>(&mut self) {
        self.resources.remove::<T>();
    }

    /// The `T` resource.
    ///
    /// # Panics
    ///
    /// If the world has no `T` — a contract violation. Use
    /// [`World::find_resource`] where absence is expected.
    #[must_use]
    pub fn resource<T: Resource>(&self) -> &T {
        match self.resources.find::<T>() {
            Some(value) => value,
            None => panic!("{}", missing_resource_message::<T>()),
        }
    }

    /// The `T` resource, for modification.
    ///
    /// # Panics
    ///
    /// If the world has no `T` — a contract violation. Use
    /// [`World::find_resource_mut`] where absence is expected.
    #[must_use]
    pub fn resource_mut<T: Resource>(&mut self) -> &mut T {
        match self.resources.find_mut::<T>() {
            Some(value) => value,
            None => panic!("{}", missing_resource_message::<T>()),
        }
    }

    /// The `T` resource, or `None` if the world has none.
    #[must_use]
    pub fn find_resource<T: Resource>(&self) -> Option<&T> {
        self.resources.find::<T>()
    }

    /// The `T` resource for modification, or `None` if the world has none.
    #[must_use]
    pub fn find_resource_mut<T: Resource>(&mut self) -> Option<&mut T> {
        self.resources.find_mut::<T>()
    }
}

/// Panic text for a resource that is absent when the caller promised it is not.
fn missing_resource_message<T: Resource>() -> String {
    message(
        &format!(
            "resource access failed: no {} in this world",
            type_name::<T>()
        ),
        "resources are inserted explicitly; nothing creates one on first use",
        "the resource was never inserted, or a previous operation removed it",
        "insert it during setup with world.insert_resource(..), or use \
         world.find_resource::<T>() if absence is expected here",
    )
}

/// INVARIANT: a resource is stored under the `TypeId` of its own type.
const MISKEYED_RESOURCE: &str = "[jidousha] engine bug: a resource is stored under a TypeId that is not its own\n  \
     likely cause: Resources::insert wrote a value with a key from a different type\n  \
     fix: report this with the reproduction — game code cannot cause it";

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Score(u32);
    impl Resource for Score {}

    #[derive(Debug, PartialEq)]
    struct Level(u32);
    impl Resource for Level {}

    #[test]
    fn a_resource_is_found_by_its_type() {
        let mut resources = Resources::new();
        resources.insert(Score(3));
        resources.insert(Level(1));
        assert_eq!(resources.find::<Score>(), Some(&Score(3)));
        assert_eq!(resources.find::<Level>(), Some(&Level(1)));
    }

    #[test]
    fn inserting_the_same_type_twice_replaces_the_value() {
        let mut resources = Resources::new();
        resources.insert(Score(1));
        resources.insert(Score(2));
        assert_eq!(resources.find::<Score>(), Some(&Score(2)));
    }

    #[test]
    fn a_removed_resource_is_gone() {
        let mut resources = Resources::new();
        resources.insert(Score(1));
        resources.remove::<Score>();
        assert_eq!(resources.find::<Score>(), None);
    }

    #[test]
    fn removing_a_resource_that_was_never_there_is_not_a_failure() {
        let mut resources = Resources::new();
        resources.remove::<Score>();
        assert_eq!(resources.find::<Score>(), None);
    }

    #[test]
    fn a_resource_can_be_changed_in_place() {
        let mut resources = Resources::new();
        resources.insert(Score(1));
        if let Some(score) = resources.find_mut::<Score>() {
            score.0 = 9;
        }
        assert_eq!(resources.find::<Score>(), Some(&Score(9)));
    }
}
