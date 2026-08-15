//! Commands: deferred structural change, and when it lands (core.md §6).

use jidousha_core::{CommandKind, Component, Entity, Resource, Seconds, Simulation, Update, World};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position(i32);
impl Component for Position {}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity(i32);
impl Component for Velocity {}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Doomed;
impl Component for Doomed {}

#[derive(Debug, Default, PartialEq)]
struct Seen(Vec<i32>);
impl Resource for Seen {}

#[test]
fn a_recorded_spawn_lands_when_the_system_returns() {
    fn spawn_two(world: &mut World) {
        let mut commands = world.commands();
        commands.spawn((Position(1), Velocity(2)));
        commands.spawn((Position(3),));
    }
    fn count_them(world: &mut World) {
        let mut values: Vec<i32> = world
            .query::<&Position>()
            .map(|(_, position)| position.0)
            .collect();
        values.sort_unstable();
        if let Some(seen) = world.find_resource_mut::<Seen>() {
            seen.0 = values;
        }
    }

    let mut simulation = Simulation::new(1, Seconds(1.0));
    simulation.world_mut().insert_resource(Seen::default());
    simulation.add_system(Update, spawn_two);
    simulation.add_system(Update, count_them);
    simulation.tick();

    // The next system saw both spawns, so they applied when spawn_two returned.
    assert_eq!(simulation.world().resource::<Seen>().0, [1, 3]);
    assert_eq!(simulation.world().entity_count(), 2);
}

#[test]
fn a_system_can_record_while_reading_the_world() {
    fn reap(world: &mut World) {
        let mut commands = world.commands();
        // The read-only query and the recorder coexist: this is what commands
        // are for (ADR-0013, core.md §6).
        for (entity, _, ()) in world.query::<(&Position, jidousha_core::With<Doomed>)>() {
            commands.despawn(entity);
        }
    }

    let mut simulation = Simulation::new(1, Seconds(1.0));
    simulation.add_system(Update, reap);
    let world = simulation.world_mut();
    let doomed = world.spawn();
    world.insert(doomed, Position(1));
    world.insert(doomed, Doomed);
    let survivor = world.spawn();
    world.insert(survivor, Position(2));

    simulation.tick();

    assert!(!simulation.world().is_alive(doomed));
    assert!(simulation.world().is_alive(survivor));
}

#[test]
fn commands_apply_in_recording_order() {
    fn churn(world: &mut World) {
        let entity = first_entity(world);
        let mut commands = world.commands();
        commands.insert(entity, Position(1));
        commands.insert(entity, Position(2));
        commands.insert(entity, Position(3));
    }

    let mut simulation = Simulation::new(1, Seconds(1.0));
    simulation.add_system(Update, churn);
    simulation.world_mut().spawn();
    simulation.tick();

    let entity = first_entity(simulation.world_mut());
    assert_eq!(
        simulation.world().component::<Position>(entity),
        &Position(3),
        "the last recorded insert wins, so they applied in order"
    );
}

#[test]
fn a_recorded_insert_and_remove_reach_the_world() {
    fn freeze_then_thaw(world: &mut World) {
        let entity = first_entity(world);
        let mut commands = world.commands();
        commands.insert(entity, Velocity(9));
        commands.remove::<Position>(entity);
    }

    let mut simulation = Simulation::new(1, Seconds(1.0));
    simulation.add_system(Update, freeze_then_thaw);
    let world = simulation.world_mut();
    let entity = world.spawn();
    world.insert(entity, Position(1));

    simulation.tick();

    assert_eq!(simulation.world().find_component::<Position>(entity), None);
    assert_eq!(
        simulation.world().component::<Velocity>(entity),
        &Velocity(9)
    );
}

#[test]
fn a_command_naming_an_entity_that_died_first_is_a_no_op() {
    fn despawn_twice(world: &mut World) {
        let entity = first_entity(world);
        let mut commands = world.commands();
        commands.despawn(entity);
        // Recorded against an entity the previous command destroys. Deferral
        // means the world moves underneath commands; this is not a failure.
        commands.insert(entity, Velocity(1));
        commands.despawn(entity);
    }

    let mut simulation = Simulation::new(1, Seconds(1.0));
    simulation.add_system(Update, despawn_twice);
    let world = simulation.world_mut();
    let entity = world.spawn();
    world.insert(entity, Position(1));

    simulation.tick();

    assert!(!simulation.world().is_alive(entity));
    assert_eq!(simulation.world().entity_count(), 0);
}

#[test]
fn commands_recorded_while_applying_commands_still_land_this_flush() {
    // A spawned entity's bundle is itself applied through the world, so a
    // command that records more work must not be left behind.
    fn spawn_a_spawner(world: &mut World) {
        let mut commands = world.commands();
        commands.spawn((Position(1),));
    }

    let mut simulation = Simulation::new(1, Seconds(1.0));
    simulation.add_system(Update, spawn_a_spawner);
    simulation.tick();
    assert_eq!(simulation.world().entity_count(), 1);
}

#[test]
fn the_recorder_lists_what_is_queued() {
    let world = World::new();
    let mut commands = world.commands();
    commands.spawn((Position(1),));
    let queued = commands.pending();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].0, CommandKind::Spawn);
    assert_eq!(queued[0].1, None);
}

#[test]
#[should_panic(expected = "already has a command recorder")]
fn taking_a_second_recorder_panics() {
    let world = World::new();
    let _first = world.commands();
    let _second = world.commands();
}

fn first_entity(world: &mut World) -> Entity {
    let found: Vec<Entity> = world
        .query::<&Position>()
        .map(|(entity, _)| entity)
        .collect();
    match found.first() {
        Some(entity) => *entity,
        None => match world.query::<(jidousha_core::Without<Position>,)>().next() {
            Some((entity, ())) => entity,
            None => panic!("the world has no entities"),
        },
    }
}
