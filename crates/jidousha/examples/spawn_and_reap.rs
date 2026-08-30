//! A complete fixed-timestep simulation: systems, resources, the seeded RNG,
//! and commands, run headless for a fixed number of ticks.
//!
//! It spawns motes at random, drifts them outward, and reaps the ones that
//! leave the field — the smallest shape that exercises everything M3 added.
//! The same seed always produces the same run, which is why the assertions at
//! the end can be exact numbers rather than ranges.
//!
//! Run it: `cargo run -p jidousha --example spawn_and_reap`

use jidousha::prelude::*;

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

/// Despawn one mote by hand, tolerating one that is already gone.
///
/// `despawn` panics on a dead entity, because a game that despawns something
/// twice has a bug worth hearing about. `try_despawn` is the version for the
/// rare case where absence is expected, and it is the one place a game meets
/// [`EntityDeadError`].
fn retire(world: &mut World, mote: Entity) -> Result<(), EntityDeadError> {
    world.try_despawn(mote)
}

/// The game, built the same way twice — which is the point of the example.
fn field() -> HeadlessSim {
    let mut sim = headless(
        GameConfig {
            seed: 2026,
            // Units live in types (conventions): a tick is a `Seconds`, not a
            // bare f32 that might have been milliseconds.
            fixed_dt: Seconds(1.0 / 60.0),
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, seed_the_field);
            app.add_system(Update, spawn_motes);
            app.add_system(Update, drift);
            app.add_system(Update, reap_the_strays);
        },
    );
    sim.world_mut().insert_resource(Reaped::default());
    sim
}

fn main() {
    let mut sim = field();
    for _ in 0..60 {
        sim.tick();
    }

    let time = sim.world().resource::<Time>();
    let alive = sim.world().entity_count();
    // What the stores hold, which is the other half of the same question: one
    // entity with three components counts three here and one above. Entities
    // climbing means a spawner with no reaper; components climbing while
    // entities hold steady means something is inserting onto the same entities
    // over and over. The performance overlay reports both for that reason
    // (`JIDOUSHA_FRAMETIME=2`).
    let components = sim.world().component_count();
    let reaped = sim.world().resource::<Reaped>().0;
    println!(
        "after {} ticks ({}): {alive} motes in the field holding {components} \
         components, {reaped} reaped",
        time.tick, time.elapsed
    );

    assert_eq!(time.tick, 60, "sixty ticks");
    // Every mote carries a Position and a Velocity, and nothing else does.
    assert_eq!(components, alive * 2, "two components to a mote");
    assert!(reaped > 0, "some motes should have left the field");
    assert!(alive > 0, "the field should not be empty");

    // The seed is the whole story: a second run of the same length matches.
    let mut again = field();
    for _ in 0..60 {
        again.tick();
    }
    assert_eq!(again.world().entity_count(), alive);
    assert_eq!(again.world().resource::<Reaped>().0, reaped);
    println!("replayed from the same seed: identical field");

    // A mote retired by hand, and then retired again: the second attempt is an
    // error value rather than a panic, because this is the one call that
    // expects to find nothing.
    if let Some((mote, _)) = sim.world().query::<&Position>().next() {
        assert!(retire(sim.world_mut(), mote).is_ok(), "it was alive");
        assert!(retire(sim.world_mut(), mote).is_err(), "and now it is not");
        println!("retired one mote by hand; retiring it again is an error, not a panic");
    }
}
