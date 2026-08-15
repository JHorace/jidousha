//! The M3 exit criterion: **simulation state is a pure function of (seed,
//! registered systems, per-tick inputs)** (core.md §7 CONTRACT).
//!
//! A soup of systems — spawning from the RNG, moving, ageing, reaping, and
//! churning archetypes — runs against a scripted input track. The whole run is
//! performed twice and the world is hashed after every tick. Identical hashes,
//! tick for tick, or the contract is broken.
//!
//! The hash covers iteration *order* as well as values, so a difference in
//! archetype or row order fails here too.

use jidousha_core::{
    Component, Entity, Resource, Rng, Seconds, Simulation, Startup, Time, Update, With, Without,
    World,
};

#[derive(Clone, Copy, Debug)]
struct Position {
    x: i32,
    y: i32,
}
impl Component for Position {}

#[derive(Clone, Copy, Debug)]
struct Velocity {
    x: i32,
    y: i32,
}
impl Component for Velocity {}

#[derive(Clone, Copy, Debug)]
struct Age(u32);
impl Component for Age {}

#[derive(Clone, Copy, Debug)]
struct Frozen;
impl Component for Frozen {}

/// The scripted input for the run: one value per tick, consumed in order.
///
/// Stands in for the `InputSnapshot` that arrives with the input subsystem —
/// what matters for the contract is that it is per-tick plain data.
#[derive(Debug)]
struct InputTrack {
    values: Vec<u32>,
}
impl Resource for InputTrack {}

impl InputTrack {
    /// The value for the current tick, wrapping if the run outlasts the script.
    fn at(&self, tick: u64) -> u32 {
        self.values[(tick as usize) % self.values.len()]
    }
}

fn spawn_seeds(world: &mut World) {
    for index in 0..4 {
        let entity = world.spawn();
        world.insert(
            entity,
            Position {
                x: index,
                y: -index,
            },
        );
        world.insert(entity, Velocity { x: 1, y: 2 });
        world.insert(entity, Age(0));
    }
}

/// Spawn between zero and three entities per tick, driven by the seeded RNG and
/// the scripted input together.
fn spawn_from_input(world: &mut World) {
    let tick = world.resource::<Time>().tick;
    let input = world.resource::<InputTrack>().at(tick);
    let rng = world.resource_mut::<Rng>();
    let count = rng.below(4);
    let jitter: Vec<i32> = (0..count).map(|_| rng.below(9) as i32 - 4).collect();

    let mut commands = world.commands();
    for offset in jitter {
        commands.spawn((
            Position {
                x: input as i32 + offset,
                y: offset,
            },
            Velocity {
                x: offset,
                y: 1 - offset,
            },
            Age(0),
        ));
    }
}

fn apply_velocity(world: &mut World) {
    for (_, position, velocity, ()) in
        world.query_mut::<(&mut Position, &Velocity, Without<Frozen>)>()
    {
        position.x += velocity.x;
        position.y += velocity.y;
    }
}

fn grow_older(world: &mut World) {
    for (_, age) in world.query_mut::<&mut Age>() {
        age.0 += 1;
    }
}

/// Freeze and thaw entities by age, which moves them between archetypes every
/// few ticks — the churn the determinism contract has to survive.
fn freeze_the_old(world: &mut World) {
    let mut freeze: Vec<Entity> = Vec::new();
    let mut thaw: Vec<Entity> = Vec::new();
    for (entity, age, ()) in world.query::<(&Age, Without<Frozen>)>() {
        if age.0 % 5 == 3 {
            freeze.push(entity);
        }
    }
    for (entity, age, ()) in world.query::<(&Age, With<Frozen>)>() {
        if age.0.is_multiple_of(5) {
            thaw.push(entity);
        }
    }
    let mut commands = world.commands();
    for entity in freeze {
        commands.insert(entity, Frozen);
    }
    for entity in thaw {
        commands.remove::<Frozen>(entity);
    }
}

/// Despawn what has drifted too far, using the read pass the world requires.
fn reap_the_distant(world: &mut World) {
    let mut commands = world.commands();
    for (entity, position) in world.query::<&Position>() {
        if position.x.abs() > 40 || position.y.abs() > 40 {
            commands.despawn(entity);
        }
    }
}

fn build(seed: u64, script: Vec<u32>) -> Simulation {
    let mut simulation = Simulation::new(seed, Seconds(1.0 / 60.0));
    simulation
        .world_mut()
        .insert_resource(InputTrack { values: script });
    simulation.add_system(Startup, spawn_seeds);
    simulation.add_system(Update, spawn_from_input);
    simulation.add_system(Update, apply_velocity);
    simulation.add_system(Update, grow_older);
    simulation.add_system(Update, freeze_the_old);
    simulation.add_system(Update, reap_the_distant);
    simulation
}

/// FNV-1a over a canonical rendering of the world, in iteration order.
///
/// Order is part of what is hashed on purpose: the determinism contract covers
/// iteration order (core.md §4), so a run that produced the same values in a
/// different order must still fail.
fn world_hash(world: &World) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };

    eat(&world.entity_count().to_le_bytes());
    eat(&world.resource::<Time>().tick.to_le_bytes());
    for (entity, position, velocity, age) in world.query::<(&Position, &Velocity, &Age)>() {
        eat(format!("{entity:?}").as_bytes());
        eat(&position.x.to_le_bytes());
        eat(&position.y.to_le_bytes());
        eat(&velocity.x.to_le_bytes());
        eat(&velocity.y.to_le_bytes());
        eat(&age.0.to_le_bytes());
    }
    for (entity, ()) in world.query::<(With<Frozen>,)>() {
        eat(format!("frozen {entity:?}").as_bytes());
    }
    hash
}

/// Run `ticks` ticks, hashing the world after each one.
fn run(seed: u64, script: Vec<u32>, ticks: u32) -> Vec<u64> {
    let mut simulation = build(seed, script);
    (0..ticks)
        .map(|_| {
            simulation.tick();
            world_hash(simulation.world())
        })
        .collect()
}

const SCRIPT: [u32; 7] = [3, 17, 0, 9, 25, 4, 12];
const TICKS: u32 = 200;

#[test]
fn the_same_seed_and_inputs_replay_to_the_same_state_every_tick() {
    let first = run(11, SCRIPT.to_vec(), TICKS);
    let second = run(11, SCRIPT.to_vec(), TICKS);
    match first.iter().zip(&second).position(|(one, two)| one != two) {
        Some(tick) => panic!(
            "runs diverged at tick {}: {:#x} then {:#x}",
            tick + 1,
            first[tick],
            second[tick]
        ),
        None => assert_eq!(first.len(), second.len()),
    }
}

#[test]
fn the_replay_actually_exercises_the_world() {
    // Guards the test above: identical empty worlds would also match. The run
    // must churn — entities appearing, moving, freezing, and dying.
    let mut simulation = build(11, SCRIPT.to_vec());
    let mut seen_hashes = std::collections::BTreeSet::new();
    for _ in 0..TICKS {
        simulation.tick();
        seen_hashes.insert(world_hash(simulation.world()));
    }
    assert!(
        seen_hashes.len() > TICKS as usize / 2,
        "only {} distinct states across {TICKS} ticks",
        seen_hashes.len()
    );
    assert!(
        simulation.world().entity_count() > 0,
        "the run despawned everything, so it proves little"
    );
    assert!(
        simulation.world().query::<(With<Frozen>,)>().count() > 0,
        "no entity ever froze, so archetype churn went untested"
    );
}

#[test]
fn a_different_seed_gives_a_different_run() {
    let first = run(11, SCRIPT.to_vec(), TICKS);
    let second = run(12, SCRIPT.to_vec(), TICKS);
    assert_ne!(first, second, "the seed must reach the simulation");
}

#[test]
fn different_inputs_give_a_different_run() {
    let first = run(11, SCRIPT.to_vec(), TICKS);
    let second = run(11, vec![1, 2, 3, 4, 5, 6, 7], TICKS);
    assert_ne!(first, second, "the input track must reach the simulation");
}

#[test]
fn a_run_resumed_at_the_same_tick_continues_identically() {
    // Replay is per-tick, not just per-run: the state after N ticks is the same
    // whether it was reached in one go or checked at every step along the way.
    let straight = {
        let mut simulation = build(11, SCRIPT.to_vec());
        for _ in 0..50 {
            simulation.tick();
        }
        world_hash(simulation.world())
    };
    let stepped = run(11, SCRIPT.to_vec(), 50);
    assert_eq!(straight, stepped[49]);
}
