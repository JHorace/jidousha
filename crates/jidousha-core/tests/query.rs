//! Query behaviour: what matches, what is yielded, and the order it comes in
//! (core.md §4–§5).

use jidousha_core::{Component, Entity, With, Without, World};

#[derive(Debug, PartialEq)]
struct Position(i32);
impl Component for Position {}

#[derive(Debug, PartialEq)]
struct Velocity(i32);
impl Component for Velocity {}

#[derive(Debug, PartialEq)]
struct Frozen;
impl Component for Frozen {}

#[derive(Debug, PartialEq)]
struct Player;
impl Component for Player {}

/// A world with one entity per interesting component set.
fn populated() -> (World, Vec<Entity>) {
    let mut world = World::new();
    let bare = world.spawn();

    let positioned = world.spawn();
    world.insert(positioned, Position(10));

    let moving = world.spawn();
    world.insert(moving, Position(20));
    world.insert(moving, Velocity(2));

    let frozen_player = world.spawn();
    world.insert(frozen_player, Position(30));
    world.insert(frozen_player, Velocity(3));
    world.insert(frozen_player, Frozen);
    world.insert(frozen_player, Player);

    (world, vec![bare, positioned, moving, frozen_player])
}

fn positions(world: &World) -> Vec<i32> {
    world
        .query::<&Position>()
        .map(|(_, position)| position.0)
        .collect()
}

#[test]
fn a_query_yields_every_entity_carrying_the_component() {
    let (world, _) = populated();
    let mut found = positions(&world);
    found.sort_unstable();
    assert_eq!(found, [10, 20, 30]);
}

#[test]
fn a_query_skips_entities_missing_any_requested_component() {
    let (world, entities) = populated();
    let found: Vec<Entity> = world
        .query::<(&Position, &Velocity)>()
        .map(|(entity, _, _)| entity)
        .collect();
    assert!(!found.contains(&entities[1]), "positioned-only entity");
    assert_eq!(found.len(), 2);
}

#[test]
fn an_empty_world_yields_nothing() {
    let world = World::new();
    assert_eq!(world.query::<&Position>().count(), 0);
}

#[test]
fn a_query_over_no_matching_archetype_yields_nothing() {
    let (world, _) = populated();
    assert_eq!(world.query::<&Player>().count(), 1);
    assert_eq!(world.query::<(&Player, &Frozen, &Velocity)>().count(), 1);
    assert_eq!(world.query::<(&Player, Without<Frozen>)>().count(), 0);
}

#[test]
fn with_filters_to_entities_carrying_the_component_without_yielding_it() {
    let (world, entities) = populated();
    let found: Vec<Entity> = world
        .query::<(&Position, With<Player>)>()
        .map(|(entity, _, ())| entity)
        .collect();
    assert_eq!(found, [entities[3]]);
}

#[test]
fn without_filters_out_entities_carrying_the_component() {
    let (world, _) = populated();
    let mut found: Vec<i32> = world
        .query::<(&Position, Without<Velocity>)>()
        .map(|(_, position, ())| position.0)
        .collect();
    found.sort_unstable();
    assert_eq!(found, [10]);
}

#[test]
fn filters_can_stand_alone_in_a_query() {
    let (world, entities) = populated();
    let found: Vec<Entity> = world
        .query::<(With<Player>,)>()
        .map(|(entity, ())| entity)
        .collect();
    assert_eq!(found, [entities[3]]);
}

#[test]
fn a_mutable_query_writes_through_to_the_world() {
    let (mut world, entities) = populated();
    for (_, position, velocity) in world.query_mut::<(&mut Position, &Velocity)>() {
        position.0 += velocity.0;
    }
    assert_eq!(world.component::<Position>(entities[1]), &Position(10));
    assert_eq!(world.component::<Position>(entities[2]), &Position(22));
    assert_eq!(world.component::<Position>(entities[3]), &Position(33));
}

#[test]
fn a_mutable_query_can_write_two_components_at_once() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position(1));
    world.insert(entity, Velocity(1));
    for (_, position, velocity) in world.query_mut::<(&mut Position, &mut Velocity)>() {
        position.0 = 5;
        velocity.0 = 6;
    }
    assert_eq!(world.component::<Position>(entity), &Position(5));
    assert_eq!(world.component::<Velocity>(entity), &Velocity(6));
}

#[test]
fn the_entity_is_always_the_first_thing_yielded() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position(1));
    let (yielded, _) = world
        .query::<&Position>()
        .next()
        .expect("the entity carries a Position");
    assert_eq!(yielded, entity);
}

#[test]
fn a_query_sees_components_moved_between_archetypes() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position(1));
    assert_eq!(world.query::<(&Position, &Velocity)>().count(), 0);

    world.insert(entity, Velocity(1));
    assert_eq!(world.query::<(&Position, &Velocity)>().count(), 1);

    world.remove::<Velocity>(entity);
    assert_eq!(world.query::<(&Position, &Velocity)>().count(), 0);
    assert_eq!(world.query::<&Position>().count(), 1);
}

#[test]
fn a_query_skips_despawned_entities() {
    let (mut world, entities) = populated();
    world.despawn(entities[2]);
    let mut found = positions(&world);
    found.sort_unstable();
    assert_eq!(found, [10, 30]);
}

#[test]
fn reading_other_entities_is_possible_during_a_read_only_query() {
    let (world, entities) = populated();
    // The read-only query leaves the world readable, which is the whole point
    // of it taking a shared borrow (ADR-0013).
    let seen: Vec<bool> = world
        .query::<&Position>()
        .map(|(_, _)| world.find_component::<Velocity>(entities[2]).is_some())
        .collect();
    assert_eq!(seen, [true, true, true]);
}

#[test]
#[should_panic(expected = "more than once")]
fn naming_one_component_twice_in_a_query_panics() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position(1));
    // Two accesses to Position would alias.
    let _ = world
        .query_mut::<(&mut Position, &Position)>()
        .next()
        .is_some();
}

#[test]
fn iteration_order_is_identical_for_identical_operation_histories() {
    let transcript = |()| -> Vec<(String, i32)> {
        let mut world = World::new();
        let entities: Vec<Entity> = (0..8).map(|_| world.spawn()).collect();
        for (index, entity) in entities.iter().enumerate() {
            world.insert(*entity, Position(index as i32));
            if index % 2 == 0 {
                world.insert(*entity, Velocity(index as i32));
            }
        }
        // Structural churn: despawns, re-spawns, and archetype moves.
        world.despawn(entities[1]);
        world.despawn(entities[4]);
        let late = world.spawn();
        world.insert(late, Position(100));
        world.remove::<Velocity>(entities[6]);
        world.insert(entities[3], Velocity(33));

        world
            .query::<&Position>()
            .map(|(entity, position)| (format!("{entity:?}"), position.0))
            .collect()
    };
    assert_eq!(transcript(()), transcript(()));
}

#[test]
fn archetypes_are_visited_in_creation_order() {
    let mut world = World::new();
    // Archetype creation order: {Position}, then {Position, Velocity}.
    let first = world.spawn();
    world.insert(first, Position(1));
    let second = world.spawn();
    world.insert(second, Position(2));
    world.insert(second, Velocity(2));

    let order: Vec<i32> = world
        .query::<&Position>()
        .map(|(_, position)| position.0)
        .collect();
    assert_eq!(order, [1, 2]);

    // A third entity joining the older archetype is visited before the newer
    // archetype's rows, whatever order the entities were spawned in.
    let third = world.spawn();
    world.insert(third, Position(3));
    let order: Vec<i32> = world
        .query::<&Position>()
        .map(|(_, position)| position.0)
        .collect();
    assert_eq!(order, [1, 3, 2]);
}

#[test]
fn a_despawn_moves_the_last_row_into_the_hole() {
    let mut world = World::new();
    let entities: Vec<Entity> = (0..3)
        .map(|index| {
            let entity = world.spawn();
            world.insert(entity, Position(index));
            entity
        })
        .collect();
    world.despawn(entities[0]);
    // Swap-remove: the last row backfills row 0, so iteration sees 2 then 1.
    assert_eq!(positions(&world), [2, 1]);
}
