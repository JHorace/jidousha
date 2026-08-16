//! A complete fixed-timestep simulation: systems, resources, the seeded RNG,
//! and commands, run headless for a fixed number of ticks.
//!
//! It spawns motes at random, drifts them outward, and reaps the ones that
//! leave the field — the smallest shape that exercises everything M3 added.
//! The same seed always produces the same run, which is why the assertions at
//! the end can be exact numbers rather than ranges.
//!
//! Run it: `cargo run -p jidousha-core --example spawn_and_reap`

use jidousha_core::{Component, Resource, Rng, Seconds, Simulation, Startup, Time, Update, World};

/// Where a mote is, in whole units.
#[derive(Clone, Copy, Debug)]
struct Position {
    x: i32,
    y: i32,
}
impl Component for Position {}

/// How fast it drifts, per tick.
#[derive(Clone, Copy, Debug)]
struct Velocity {
    x: i32,
    y: i32,
}
impl Component for Velocity {}

/// How many motes have ever been reaped — world-global state, so a resource.
#[derive(Debug, Default)]
struct Reaped(u32);
impl Resource for Reaped {}

/// The field's edge; beyond it, motes are reaped.
const EDGE: i32 = 12;

fn seed_the_field(world: &mut World) {
    for index in -1..=1 {
        let mote = world.spawn();
        world.insert(mote, Position { x: index, y: 0 });
        world.insert(
            mote,
            Velocity {
                x: index,
                y: 1 - index,
            },
        );
    }
}

/// Spawn a mote every third tick, with a velocity drawn from the seeded RNG.
///
/// Randomness comes only from the `Rng` resource: the same seed replays the
/// same field, forever.
fn spawn_motes(world: &mut World) {
    if !world.resource::<Time>().tick.is_multiple_of(3) {
        return;
    }
    let rng = world.resource_mut::<Rng>();
    let velocity = Velocity {
        x: rng.below(5) as i32 - 2,
        y: rng.below(5) as i32 - 2,
    };
    // Recorded, not applied: spawning mid-system is what commands are for.
    world.commands().spawn((Position { x: 0, y: 0 }, velocity));
}

fn drift(world: &mut World) {
    for (_, position, velocity) in world.query_mut::<(&mut Position, &Velocity)>() {
        position.x += velocity.x;
        position.y += velocity.y;
    }
}

/// Reap what has left the field, counting it as it goes.
///
/// The read-only query and the command recorder coexist, so this is one pass
/// rather than the collect-then-write shape a direct despawn would need.
fn reap_the_strays(world: &mut World) {
    let mut reaped = 0;
    let mut commands = world.commands();
    for (mote, position) in world.query::<&Position>() {
        if position.x.abs() > EDGE || position.y.abs() > EDGE {
            commands.despawn(mote);
            reaped += 1;
        }
    }
    drop(commands);
    if let Some(counter) = world.find_resource_mut::<Reaped>() {
        counter.0 += reaped;
    }
}

fn main() {
    let mut simulation = Simulation::new(2026, Seconds(1.0 / 60.0));
    simulation.world_mut().insert_resource(Reaped::default());
    simulation.add_system(Startup, seed_the_field);
    simulation.add_system(Update, spawn_motes);
    simulation.add_system(Update, drift);
    simulation.add_system(Update, reap_the_strays);

    println!("{}", simulation.schedule_debug());

    // Real time in, whole ticks out: sixty frames of a sixtieth of a second.
    for _ in 0..60 {
        simulation.advance(Seconds(1.0 / 60.0), |_, _| {});
    }

    let time = simulation.world().resource::<Time>();
    let alive = simulation.world().entity_count();
    let reaped = simulation.world().resource::<Reaped>().0;
    println!(
        "after {} ticks ({}): {alive} motes in the field, {reaped} reaped",
        time.tick, time.elapsed
    );

    assert_eq!(time.tick, 60, "sixty frames at one tick each");
    assert!(reaped > 0, "some motes should have left the field");
    assert!(alive > 0, "the field should not be empty");

    // The seed is the whole story: a second run of the same length matches.
    let mut again = Simulation::new(2026, Seconds(1.0 / 60.0));
    again.world_mut().insert_resource(Reaped::default());
    again.add_system(Startup, seed_the_field);
    again.add_system(Update, spawn_motes);
    again.add_system(Update, drift);
    again.add_system(Update, reap_the_strays);
    for _ in 0..60 {
        again.advance(Seconds(1.0 / 60.0), |_, _| {});
    }
    assert_eq!(again.world().entity_count(), alive);
    assert_eq!(again.world().resource::<Reaped>().0, reaped);
    println!("replayed from the same seed: identical field");
}
