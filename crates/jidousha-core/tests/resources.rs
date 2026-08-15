//! Resources: the world's typed singletons (core.md §6).

use jidousha_core::{Resource, Rng, Seconds, Simulation, Time, World};

#[derive(Debug, PartialEq)]
struct Score(u32);
impl Resource for Score {}

#[derive(Debug, PartialEq)]
struct Level(u32);
impl Resource for Level {}

#[test]
fn an_inserted_resource_can_be_read_back() {
    let mut world = World::new();
    world.insert_resource(Score(10));
    assert_eq!(world.resource::<Score>(), &Score(10));
}

#[test]
fn resources_of_different_types_are_independent() {
    let mut world = World::new();
    world.insert_resource(Score(1));
    world.insert_resource(Level(2));
    world.remove_resource::<Score>();
    assert_eq!(world.find_resource::<Score>(), None);
    assert_eq!(world.resource::<Level>(), &Level(2));
}

#[test]
fn inserting_a_resource_twice_keeps_the_second_value() {
    let mut world = World::new();
    world.insert_resource(Score(1));
    world.insert_resource(Score(2));
    assert_eq!(world.resource::<Score>(), &Score(2));
}

#[test]
fn a_resource_can_be_changed_in_place() {
    let mut world = World::new();
    world.insert_resource(Score(1));
    world.resource_mut::<Score>().0 += 41;
    assert_eq!(world.resource::<Score>(), &Score(42));
}

#[test]
fn removing_a_resource_the_world_never_had_is_not_a_failure() {
    let mut world = World::new();
    world.remove_resource::<Score>();
    assert_eq!(world.find_resource::<Score>(), None);
}

#[test]
fn a_missing_resource_is_reported_rather_than_invented() {
    let world = World::new();
    assert_eq!(world.find_resource::<Score>(), None);
}

#[test]
#[should_panic(expected = "resource access failed")]
fn reading_a_missing_resource_panics() {
    let world = World::new();
    let _ = world.resource::<Score>();
}

#[test]
#[should_panic(expected = "resource access failed")]
fn modifying_a_missing_resource_panics() {
    let mut world = World::new();
    let _ = world.resource_mut::<Score>();
}

#[test]
fn a_simulation_starts_with_a_clock_and_a_generator() {
    let simulation = Simulation::new(7, Seconds(1.0 / 60.0));
    assert_eq!(simulation.world().resource::<Time>().tick, 0);
    // The generator is seeded and ready; drawing from it is a system's job.
    assert!(simulation.world().find_resource::<Rng>().is_some());
}

#[test]
fn resources_survive_across_ticks() {
    let mut simulation = Simulation::new(1, Seconds(1.0));
    simulation.world_mut().insert_resource(Score(5));
    simulation.tick();
    simulation.tick();
    assert_eq!(simulation.world().resource::<Score>(), &Score(5));
}

#[test]
#[should_panic(expected = "the Time resource is missing")]
fn removing_the_clock_is_reported_loudly() {
    let mut simulation = Simulation::new(1, Seconds(1.0));
    simulation.world_mut().remove_resource::<Time>();
    simulation.tick();
}
