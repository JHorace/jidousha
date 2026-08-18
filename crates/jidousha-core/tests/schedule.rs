//! The schedule and the fixed-timestep loop: what runs, in what order, and how
//! real time becomes ticks (core.md §7).

use jidousha_core::{Component, Resource, Seconds, Simulation, Startup, Time, Update, World};

#[derive(Debug, PartialEq)]
struct Position(i32);
impl Component for Position {}

/// Records what ran, in order, so the tests can assert on a transcript.
#[derive(Debug, Default, PartialEq)]
struct Log(Vec<&'static str>);
impl Resource for Log {}

fn log(world: &mut World, entry: &'static str) {
    if let Some(log) = world.find_resource_mut::<Log>() {
        log.0.push(entry);
    }
}

fn first(world: &mut World) {
    log(world, "first");
}

fn second(world: &mut World) {
    log(world, "second");
}

fn third(world: &mut World) {
    log(world, "third");
}

fn setup(world: &mut World) {
    log(world, "setup");
}

fn transcript(simulation: &Simulation) -> &[&'static str] {
    &simulation.world().resource::<Log>().0
}

fn simulation_with_log() -> Simulation {
    let mut simulation = Simulation::new(1, Seconds(1.0 / 60.0));
    simulation.world_mut().insert_resource(Log::default());
    simulation
}

#[test]
fn update_systems_run_in_registration_order() {
    let mut simulation = simulation_with_log();
    simulation.add_system(Update, first);
    simulation.add_system(Update, second);
    simulation.add_system(Update, third);
    simulation.tick();
    assert_eq!(transcript(&simulation), ["first", "second", "third"]);
}

#[test]
fn startup_runs_once_before_the_first_tick() {
    let mut simulation = simulation_with_log();
    simulation.add_system(Startup, setup);
    simulation.add_system(Update, first);
    simulation.tick();
    simulation.tick();
    assert_eq!(transcript(&simulation), ["setup", "first", "first"]);
}

#[test]
fn startup_runs_even_when_nothing_ticks() {
    let mut simulation = simulation_with_log();
    simulation.add_system(Startup, setup);
    simulation.start();
    assert_eq!(transcript(&simulation), ["setup"]);
}

#[test]
fn starting_twice_runs_startup_once() {
    let mut simulation = simulation_with_log();
    simulation.add_system(Startup, setup);
    simulation.start();
    simulation.start();
    simulation.tick();
    assert_eq!(transcript(&simulation), ["setup"]);
}

#[test]
fn a_tick_advances_the_clock_by_exactly_one_step() {
    let mut simulation = Simulation::new(1, Seconds(0.5));
    simulation.tick();
    simulation.tick();
    let time = simulation.world().resource::<Time>();
    assert_eq!(time.tick, 2);
    assert_eq!(time.elapsed, Seconds(1.0));
    assert_eq!(time.fixed_dt, Seconds(0.5));
}

#[test]
fn a_system_sees_the_tick_it_is_part_of() {
    fn record_tick(world: &mut World) {
        let tick = world.resource::<Time>().tick;
        if let Some(ticks) = world.find_resource_mut::<Ticks>() {
            ticks.0.push(tick);
        }
    }
    #[derive(Debug, Default)]
    struct Ticks(Vec<u64>);
    impl Resource for Ticks {}

    let mut simulation = Simulation::new(1, Seconds(1.0));
    simulation.world_mut().insert_resource(Ticks::default());
    simulation.add_system(Update, record_tick);
    simulation.tick();
    simulation.tick();
    assert_eq!(simulation.world().resource::<Ticks>().0, [1, 2]);
}

#[test]
fn real_time_becomes_whole_ticks_with_the_remainder_carried_over() {
    let mut simulation = Simulation::new(1, Seconds(0.1));
    // 0.25s buys two ticks and carries 0.05s.
    assert_eq!(simulation.advance(Seconds(0.25), |_, _| {}), 2);
    // 0.05s carried + 0.06s is enough for one more.
    assert_eq!(simulation.advance(Seconds(0.06), |_, _| {}), 1);
    assert_eq!(simulation.world().resource::<Time>().tick, 3);
}

#[test]
fn a_frame_shorter_than_a_tick_runs_nothing() {
    let mut simulation = Simulation::new(1, Seconds(0.1));
    assert_eq!(simulation.advance(Seconds(0.05), |_, _| {}), 0);
    assert_eq!(simulation.world().resource::<Time>().tick, 0);
}

#[test]
fn alpha_reports_how_far_into_the_next_tick_the_frame_fell() {
    let mut simulation = Simulation::new(1, Seconds(0.1));
    simulation.advance(Seconds(0.25), |_, _| {});
    let alpha = simulation.world().resource::<Time>().alpha;
    assert!((alpha - 0.5).abs() < 1e-6, "{alpha}");
}

#[test]
fn a_stalled_frame_is_clamped_instead_of_spiralling() {
    let mut simulation = Simulation::new(1, Seconds(0.1));
    // Ten seconds of stall would be a hundred ticks without the clamp.
    let steps = simulation.advance(Seconds(10.0), |_, _| {});
    assert_eq!(steps, 2, "a quarter second of catch-up, no more");
}

#[test]
fn the_schedule_listing_names_every_system_in_run_order() {
    let mut simulation = simulation_with_log();
    simulation.add_system(Startup, setup);
    simulation.add_system(Update, first);
    simulation.add_system(Update, second);
    assert_eq!(
        simulation.schedule_debug(),
        "schedule:\n  \
         Startup (1)\n    0. setup\n  \
         Update (2)\n    0. first\n    1. second\n  \
         Draw (0)\n"
    );
}

#[test]
fn systems_communicate_through_the_world_alone() {
    fn spawn_one(world: &mut World) {
        let entity = world.spawn();
        world.insert(entity, Position(0));
    }
    fn advance_all(world: &mut World) {
        for (_, position) in world.query_mut::<&mut Position>() {
            position.0 += 1;
        }
    }
    let mut simulation = Simulation::new(1, Seconds(1.0));
    simulation.add_system(Update, spawn_one);
    simulation.add_system(Update, advance_all);
    simulation.tick();
    simulation.tick();

    // Tick one spawns then advances it; tick two spawns another and advances both.
    let mut values: Vec<i32> = simulation
        .world()
        .query::<&Position>()
        .map(|(_, position)| position.0)
        .collect();
    values.sort_unstable();
    assert_eq!(values, [1, 2]);
}

#[test]
fn the_first_update_system_sees_tick_one() {
    // e0-findings.md F-062: a game author could not tell from the document
    // whether the first Update reads 0 or 1, and the two answers are one apart
    // for anything timed absolutely ("spawn the boss on tick 600"). A tick
    // advances the clock and then runs Update, so the counter a game can read
    // is one-based; `Time::new`'s zero is only ever visible to a driver holding
    // a world between ticks.
    #[derive(Debug, Default)]
    struct Seen(Vec<u64>);
    impl Resource for Seen {}

    fn note_the_tick(world: &mut World) {
        let tick = world.resource::<Time>().tick;
        if let Some(seen) = world.find_resource_mut::<Seen>() {
            seen.0.push(tick);
        }
    }

    let mut simulation = Simulation::new(1, Seconds(1.0 / 60.0));
    simulation.world_mut().insert_resource(Seen::default());
    simulation.add_system(Update, note_the_tick);

    assert_eq!(
        simulation.world().resource::<Time>().tick,
        0,
        "before any tick the clock is at zero, which no Update ever observes"
    );
    simulation.tick();
    simulation.tick();
    simulation.tick();

    assert_eq!(simulation.world().resource::<Seen>().0, vec![1, 2, 3]);
}
