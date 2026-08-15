//! Engine error text and the world's recoverable error type.
//!
//! Key types: `EntityDeadError`, `message`.
//! Depends on: `entity`. Must never depend on: `world`, `storage` — error text
//! is built from values, never by reaching back into engine state.
//! INVARIANT: every engine-authored failure states what happened, the
//! specifics, the likely cause, and the fix, in that order (core.md §9). The
//! same text is used whether the failure is delivered as a panic (contract
//! violation) or as a `Result` (the `try_*` operations).

use core::fmt;
use core::num::NonZeroU32;

use crate::entity::Entity;

/// Format one engine failure in the house style.
///
/// ```text
/// [jidousha] <what happened>
///   <specifics: entity/component/system names and values>
///   likely cause: <the most common mistake producing this>
///   fix: <the concrete change to make>
/// ```
///
/// When a system is running, an `in system: <name> (<Phase>)` line is inserted
/// after the specifics — §9's full shape. Outside a system (setup, a test
/// driving the world directly) there is nothing to name and the line is
/// omitted rather than filled with a placeholder.
pub(crate) fn message(what: &str, specifics: &str, likely_cause: &str, fix: &str) -> String {
    // The running system, when there is one — §9 asks for it in every message,
    // and the schedule is the only thing that knows it.
    let in_system = crate::panic_hook::in_system_line();
    format!(
        "[jidousha] {what}\n  {specifics}{in_system}\n  likely cause: {likely_cause}\n  fix: {fix}"
    )
}

/// A structural operation was asked to act on an entity that is not alive.
///
/// Returned by the `try_*` operations. The panicking operations of the same
/// name report the identical text — using a dead entity is normally a bug in
/// game code, and only the rare legitimately-racy call site wants a `Result`
/// (core.md §2, §9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityDeadError {
    operation: &'static str,
    entity: Entity,
    slot_generation: Option<NonZeroU32>,
}

impl EntityDeadError {
    pub(crate) fn new(
        operation: &'static str,
        entity: Entity,
        slot_generation: Option<NonZeroU32>,
    ) -> Self {
        Self {
            operation,
            entity,
            slot_generation,
        }
    }

    /// The entity the failed operation named.
    pub fn entity(&self) -> Entity {
        self.entity
    }

    /// The operation that failed, as it is spelled in the API — `"despawn"`,
    /// `"insert"`, `"remove"`.
    pub fn operation(&self) -> &'static str {
        self.operation
    }
}

impl fmt::Display for EntityDeadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let specifics = match self.slot_generation {
            Some(generation) => format!("its slot now holds generation {generation}"),
            None => "its slot has never been allocated in this world".to_owned(),
        };
        formatter.write_str(&message(
            &format!("{} failed: {:?} is not alive", self.operation, self.entity),
            &specifics,
            "the entity was already despawned, or the handle was stored across a despawn",
            &format!(
                "check world.is_alive(entity) first, or use world.try_{}(...) if the entity may \
                 legitimately be gone",
                self.operation
            ),
        ))
    }
}

impl core::error::Error for EntityDeadError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_states_what_happened_then_cause_then_fix() {
        let text = message("it broke", "specifically here", "you did X", "do Y instead");
        assert_eq!(
            text,
            "[jidousha] it broke\n  specifically here\n  likely cause: you did X\n  fix: do Y instead"
        );
    }

    #[test]
    fn a_dead_entity_error_names_the_operation_the_handle_and_the_live_generation() {
        let mut allocator = crate::entity::EntityAllocator::new();
        let entity = allocator.create();
        allocator.destroy(entity);
        let error = EntityDeadError::new("despawn", entity, allocator.slot_generation(entity));
        let text = error.to_string();
        assert_eq!(
            text.lines().next(),
            Some("[jidousha] despawn failed: Entity(0 v1) is not alive")
        );
        assert!(text.contains("its slot now holds generation 2"), "{text}");
        assert!(text.contains("world.try_despawn(...)"), "{text}");
    }

    #[test]
    fn an_entity_from_another_world_is_reported_as_never_allocated() {
        let mut other = crate::entity::EntityAllocator::new();
        let stranger = other.create();
        let error = EntityDeadError::new("insert", stranger, None);
        assert!(
            error
                .to_string()
                .contains("its slot has never been allocated in this world"),
            "{error}"
        );
    }
}
