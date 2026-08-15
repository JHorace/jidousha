//! Entity handles and the generational slot allocator behind them.
//!
//! Key types: `Entity`, `EntityAllocator`.
//! Depends on: nothing. Must never depend on: storage or world state — the
//! allocator knows slots and generations, nothing about components.
//! INVARIANT: allocation is a pure function of the operation history — free
//! slots are reused LIFO and a slot's generation only ever increases, so the
//! same sequence of spawns and despawns yields the same handles on every
//! platform and every run (core.md §2).

use core::fmt;
use core::num::NonZeroU32;

/// The first generation handed out for a slot. Generations start at 1 so the
/// niche in `NonZeroU32` keeps `Entity` the size of two `u32`s.
const FIRST_GENERATION: NonZeroU32 = NonZeroU32::MIN;

/// A handle to a thing in the world — the only way game code refers to one.
///
/// Copyable and opaque: no pointers, no lifetimes, no borrowing of the world.
/// Handles are *generational*, so a handle to a despawned entity is detectably
/// dead rather than silently pointing at whatever reused its slot. Check with
/// [`World::is_alive`](crate::World::is_alive).
///
/// The debug format is `Entity(index vGeneration)` — for example `Entity(17 v3)`
/// — and appears verbatim in engine error messages, so it greps.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity {
    index: u32,
    generation: NonZeroU32,
}

impl fmt::Debug for Entity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Entity({} v{})", self.index, self.generation)
    }
}

impl Entity {
    /// The slot this handle refers to. Engine-internal: the index alone is not
    /// a valid handle, since it says nothing about which generation is live.
    pub(crate) fn index(self) -> usize {
        self.index as usize
    }
}

/// One slot in the allocator: the generation currently occupying it, and
/// whether that generation is live.
#[derive(Clone, Copy, Debug)]
struct Slot {
    generation: NonZeroU32,
    alive: bool,
}

/// Hands out entity handles and recycles the slots of despawned ones.
///
/// INVARIANT: `free` holds exactly the indices of slots whose `alive` is false,
/// most recently freed last — popping from the end is the LIFO reuse the
/// determinism contract promises.
#[derive(Debug)]
pub(crate) struct EntityAllocator {
    slots: Vec<Slot>,
    free: Vec<u32>,
}

impl EntityAllocator {
    pub(crate) fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    /// Allocate a handle, reusing the most recently freed slot when there is one.
    pub(crate) fn create(&mut self) -> Entity {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.alive = true;
            return Entity {
                index,
                generation: slot.generation,
            };
        }
        let Ok(index) = u32::try_from(self.slots.len()) else {
            panic!(
                "[jidousha] entity allocation failed: the world already holds {} entity slots\n  \
                 the limit is u32::MAX slots, and every slot ever allocated is counted\n  \
                 likely cause: entities are spawned every tick and never despawned\n  \
                 fix: despawn what the simulation no longer needs",
                self.slots.len()
            );
        };
        self.slots.push(Slot {
            generation: FIRST_GENERATION,
            alive: true,
        });
        Entity {
            index,
            generation: FIRST_GENERATION,
        }
    }

    /// Free `entity`'s slot and bump its generation, making every outstanding
    /// handle to it detectably dead.
    ///
    /// CONTRACT: the caller has already established that `entity` is alive
    /// (`World` checks, and reports the failure with the operation's name).
    pub(crate) fn destroy(&mut self, entity: Entity) {
        debug_assert!(self.is_alive(entity), "destroy called on a dead entity");
        let slot = &mut self.slots[entity.index()];
        slot.alive = false;
        let Some(next) = slot.generation.checked_add(1) else {
            panic!(
                "[jidousha] entity slot {} has exhausted its generations\n  \
                 the slot has been reused u32::MAX times\n  \
                 likely cause: one slot is being spawned and despawned every tick for years of \
                 simulated time\n  \
                 fix: this is an engine limit — report it with the reproduction",
                entity.index
            );
        };
        slot.generation = next;
        self.free.push(entity.index);
    }

    pub(crate) fn is_alive(&self, entity: Entity) -> bool {
        matches!(
            self.slots.get(entity.index()),
            Some(slot) if slot.alive && slot.generation == entity.generation
        )
    }

    /// The generation currently occupying `entity`'s slot, if the slot exists.
    ///
    /// Used by error messages to say *how* a handle went stale: a higher
    /// generation means the entity was despawned and its slot reused.
    pub(crate) fn slot_generation(&self, entity: Entity) -> Option<NonZeroU32> {
        self.slots.get(entity.index()).map(|slot| slot.generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_debug_format_names_the_slot_and_generation() {
        let mut allocator = EntityAllocator::new();
        let entity = allocator.create();
        assert_eq!(format!("{entity:?}"), "Entity(0 v1)");
    }

    #[test]
    fn fresh_slots_are_handed_out_in_allocation_order() {
        let mut allocator = EntityAllocator::new();
        let entities: Vec<String> = (0..3)
            .map(|_| format!("{:?}", allocator.create()))
            .collect();
        assert_eq!(entities, ["Entity(0 v1)", "Entity(1 v1)", "Entity(2 v1)"]);
    }

    #[test]
    fn a_despawned_handle_is_no_longer_alive() {
        let mut allocator = EntityAllocator::new();
        let entity = allocator.create();
        allocator.destroy(entity);
        assert!(!allocator.is_alive(entity));
    }

    #[test]
    fn freed_slots_are_reused_most_recently_freed_first() {
        let mut allocator = EntityAllocator::new();
        let first = allocator.create();
        let second = allocator.create();
        allocator.destroy(first);
        allocator.destroy(second);
        // LIFO: `second`'s slot was freed last, so it comes back first.
        assert_eq!(format!("{:?}", allocator.create()), "Entity(1 v2)");
        assert_eq!(format!("{:?}", allocator.create()), "Entity(0 v2)");
    }

    #[test]
    fn reusing_a_slot_does_not_revive_the_old_handle() {
        let mut allocator = EntityAllocator::new();
        let old = allocator.create();
        allocator.destroy(old);
        let reused = allocator.create();
        assert!(allocator.is_alive(reused));
        assert!(!allocator.is_alive(old));
        assert_ne!(old, reused);
    }

    #[test]
    fn a_handle_from_another_world_is_not_alive_here() {
        let mut other = EntityAllocator::new();
        let stranger = other.create();
        let empty = EntityAllocator::new();
        assert!(!empty.is_alive(stranger));
    }
}
