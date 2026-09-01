//! Structural operations on the world: spawning, despawning, and moving
//! entities between archetypes as components come and go (core.md §2–§4).
//!
//! These are the behaviours a game can observe without ever writing a query.

use jidousha_core::{Component, World};

#[derive(Debug, PartialEq)]
struct Position(i32);
impl Component for Position {}

#[derive(Debug, PartialEq)]
struct Velocity(i32);
impl Component for Velocity {}

#[derive(Debug, PartialEq)]
struct Frozen;
impl Component for Frozen {}

#[test]
fn a_spawned_entity_is_alive_and_counted() {
    let mut world = World::new();
    let entity = world.spawn();
    assert!(world.is_alive(entity));
    assert_eq!(world.entity_count(), 1);
}

#[test]
fn the_component_count_is_the_values_the_stores_hold_not_the_entities() {
    // The counter the performance overlay reports beside `entity_count`
    // (frame-pacing.md §7). It has to move when a component is inserted onto an
    // entity that already exists, because "components climbing while entities
    // hold steady" is the shape of a leak that an entity count alone cannot
    // see.
    let mut world = World::new();
    assert_eq!(world.component_count(), 0, "an empty world holds nothing");

    let entity = world.spawn();
    assert_eq!(world.entity_count(), 1);
    assert_eq!(world.component_count(), 0, "an entity with no components");

    world.insert(entity, Position(1));
    world.insert(entity, Velocity(2));
    assert_eq!(world.entity_count(), 1, "still one entity");
    // Three, not two, would mean an archetype counting a column it does not
    // have; one would mean it counting entities under another name.
    assert_eq!(world.component_count(), 2);

    let second = world.spawn();
    world.insert(second, Position(3));
    assert_eq!(world.component_count(), 3, "across two archetypes");

    world.remove::<Velocity>(entity);
    assert_eq!(world.component_count(), 2, "removal gives one back");
    world.despawn(entity);
    assert_eq!(world.component_count(), 1, "and despawning gives the rest");
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
fn component_order_at_insert_does_not_change_what_the_entity_has() {
    let mut world = World::new();
    let forwards = world.spawn();
    world.insert(forwards, Position(1));
    world.insert(forwards, Velocity(2));
    let backwards = world.spawn();
    world.insert(backwards, Velocity(2));
    world.insert(backwards, Position(1));

    for entity in [forwards, backwards] {
        assert_eq!(world.find_component::<Position>(entity), Some(&Position(1)));
        assert_eq!(world.find_component::<Velocity>(entity), Some(&Velocity(2)));
    }
    // Both entities have the same component set, so both moved into the same
    // archetype — one query sees them both.
    assert_eq!(world.query::<(&Position, &Velocity)>().count(), 2);
}

#[test]
fn adding_a_component_keeps_the_ones_already_there() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position(7));
    world.insert(entity, Velocity(8));
    world.insert(entity, Frozen);
    assert_eq!(world.component::<Position>(entity), &Position(7));
    assert_eq!(world.component::<Velocity>(entity), &Velocity(8));
    assert_eq!(world.component::<Frozen>(entity), &Frozen);
}

#[test]
fn removing_a_component_keeps_the_ones_that_remain() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position(7));
    world.insert(entity, Velocity(8));
    world.remove::<Position>(entity);
    assert_eq!(world.find_component::<Position>(entity), None);
    assert_eq!(world.component::<Velocity>(entity), &Velocity(8));
}

#[test]
fn moving_an_entity_between_archetypes_leaves_its_neighbours_findable() {
    let mut world = World::new();
    let entities: Vec<_> = (0..4).map(|_| world.spawn()).collect();
    for (index, entity) in entities.iter().enumerate() {
        world.insert(*entity, Position(index as i32));
    }
    // Moves entities[0] to another archetype, swapping the last row into its place.
    world.insert(entities[0], Velocity(99));

    for (index, entity) in entities.iter().enumerate() {
        assert_eq!(
            world.find_component::<Position>(*entity),
            Some(&Position(index as i32)),
            "position of entity {index} survived the move"
        );
    }
    assert_eq!(world.find_component::<Velocity>(entities[3]), None);
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
