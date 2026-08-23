//! The app lifecycle and the Draw phase (core.md §7–§8, ADR-0008).

use jidousha_core::{
    Component, Draw, DrawCtx, GameConfig, Resource, Rng, Seconds, Startup, Time, Update, World,
    WorldView, headless,
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position(i32);
impl Component for Position {}

fn spawn_two(world: &mut World) {
    for value in [1, 2] {
        let entity = world.spawn();
        world.insert(entity, Position(value));
    }
}

fn advance(world: &mut World) {
    for (_, position) in world.query_mut::<&mut Position>() {
        position.0 += 10;
    }
}

/// A Draw system: it can read everything and write nothing.
fn record_positions(ctx: &mut DrawCtx) {
    let mut seen: Vec<i32> = ctx
        .world
        .query::<&Position>()
        .map(|(_, position)| position.0)
        .collect();
    seen.sort_unstable();
    // Reading a resource from Draw is fine; writing one is not expressible.
    let _ = ctx.world.resource::<Time>().tick;
    println!("drew {seen:?}");
}

#[test]
fn the_default_config_is_a_sixty_tick_second() {
    let config = GameConfig::default();
    assert_eq!(config.fixed_dt, Seconds(1.0 / 60.0));
    assert_eq!(config.seed, 0);
}

#[test]
fn a_config_can_be_written_as_a_diff_from_the_default() {
    let config = GameConfig {
        title: "asteroids",
        seed: 42,
        ..GameConfig::default()
    };
    assert_eq!(config.title, "asteroids");
    assert_eq!(config.seed, 42);
    assert_eq!(config.fixed_dt, GameConfig::default().fixed_dt);
}

#[test]
fn headless_runs_startup_then_update_like_the_windowed_driver_will() {
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Startup, spawn_two);
        app.add_system(Update, advance);
    });
    sim.tick();
    sim.tick();

    let mut values: Vec<i32> = sim
        .world()
        .query::<&Position>()
        .map(|(_, position)| position.0)
        .collect();
    values.sort_unstable();
    assert_eq!(values, [21, 22]);
}

#[test]
fn the_seed_in_the_config_reaches_the_generator() {
    let draw_from = |seed| {
        let mut sim = headless(
            GameConfig {
                seed,
                ..GameConfig::default()
            },
            |_| {},
        );
        sim.world_mut().resource_mut::<Rng>().next_u32()
    };
    assert_eq!(draw_from(7), draw_from(7));
    assert_ne!(draw_from(7), draw_from(8));
}

#[test]
fn the_fixed_step_in_the_config_reaches_the_clock() {
    let sim = headless(
        GameConfig {
            fixed_dt: Seconds(0.25),
            ..GameConfig::default()
        },
        |_| {},
    );
    assert_eq!(sim.world().resource::<Time>().fixed_dt, Seconds(0.25));
}

#[test]
fn draw_systems_run_when_a_frame_is_drawn() {
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Startup, spawn_two);
        app.add_system(Draw, record_positions);
    });
    sim.tick();
    sim.draw();
    // Drawing changed nothing.
    assert_eq!(sim.world().entity_count(), 2);
}

#[test]
fn drawing_without_ticking_still_runs_startup_first() {
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Startup, spawn_two);
        app.add_system(Draw, record_positions);
    });
    sim.draw();
    assert_eq!(sim.world().entity_count(), 2);
}

#[test]
fn a_draw_system_sees_what_update_left_behind() {
    fn count_into_resource(ctx: &mut DrawCtx) {
        let seen: Vec<i32> = ctx
            .world
            .query::<&Position>()
            .map(|(_, position)| position.0)
            .collect();
        // A Draw system cannot write the world, so it reports through a
        // channel outside it — here, stdout. The renderer's submission sink
        // is the real answer (R0).
        println!("{seen:?}");
    }

    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Startup, spawn_two);
        app.add_system(Update, advance);
        app.add_system(Draw, count_into_resource);
    });
    sim.tick();
    sim.draw();
    let mut values: Vec<i32> = sim
        .world()
        .query::<&Position>()
        .map(|(_, position)| position.0)
        .collect();
    values.sort_unstable();
    assert_eq!(values, [11, 12]);
}

#[test]
fn the_schedule_listing_covers_all_three_phases() {
    let sim = headless(GameConfig::default(), |app| {
        app.add_system(Startup, spawn_two);
        app.add_system(Update, advance);
        app.add_system(Draw, record_positions);
    });
    assert_eq!(
        sim.schedule_debug(),
        "schedule:\n  \
         Startup (1)\n    0. spawn_two\n  \
         Update (1)\n    0. advance\n  \
         Draw (1)\n    0. record_positions\n"
    );
}

#[test]
fn an_engine_message_names_the_system_that_hit_it() {
    fn reads_a_missing_component(world: &mut World) {
        let entity = world.spawn();
        // Contract violation: the entity has no Position.
        let _ = world.component::<Position>(entity);
    }

    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Update, reads_a_missing_component);
    });
    let message = panic_message(move || sim.tick());
    assert!(
        message.contains("in system: reads_a_missing_component (Update)"),
        "{message}"
    );
    // The §9 shape survives the addition.
    assert!(
        message.starts_with("[jidousha] component access failed"),
        "{message}"
    );
    assert!(message.contains("likely cause:"), "{message}");
    assert!(message.contains("fix:"), "{message}");
}

#[test]
fn a_message_outside_any_system_names_none() {
    let mut world = World::new();
    let entity = world.spawn();
    let message = panic_message(move || {
        let _ = world.component::<Position>(entity);
    });
    assert!(!message.contains("in system:"), "{message}");
}

/// Everything a tuning sweep would vary, as the shape *Testing your game*
/// recommends for a game that expects to be swept.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Tuning(i32);
impl Resource for Tuning {}

impl Tuning {
    /// What the game would ship with.
    const SHIPPED: Self = Self(1);
}

/// Startup takes whatever a harness left in the world, or the shipped numbers.
fn pin_tuning(world: &mut World) {
    let tuning = world
        .find_resource::<Tuning>()
        .copied()
        .unwrap_or(Tuning::SHIPPED);
    world.insert_resource(tuning);
}

#[test]
fn a_resource_inserted_before_the_first_tick_is_what_startup_finds() {
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Startup, pin_tuning);
    });
    sim.world_mut().insert_resource(Tuning(7));
    sim.tick();
    assert_eq!(*sim.world().resource::<Tuning>(), Tuning(7));
}

#[test]
fn a_run_that_sets_nothing_gets_the_shipped_numbers() {
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Startup, pin_tuning);
    });
    sim.tick();
    assert_eq!(*sim.world().resource::<Tuning>(), Tuning::SHIPPED);
}

#[test]
fn each_headless_call_builds_a_fresh_game_so_a_sweep_is_a_loop() {
    let swept: Vec<Tuning> = (1..=4)
        .map(|candidate| {
            let mut sim = headless(GameConfig::default(), |app| {
                app.add_system(Startup, pin_tuning);
            });
            sim.world_mut().insert_resource(Tuning(candidate));
            sim.tick();
            *sim.world().resource::<Tuning>()
        })
        .collect();
    assert_eq!(
        swept,
        vec![Tuning(1), Tuning(2), Tuning(3), Tuning(4)],
        "one process, four games, no recompile"
    );
}

/// The projection a UI is a picture of, written once (ADR-0039).
///
/// It takes a `&WorldView<'_>`, which is what an Update system and a Draw
/// system can both produce — the first through `World::view`, the second
/// because that is what it was handed.
fn positions_in(world: &WorldView<'_>) -> Vec<i32> {
    let mut seen: Vec<i32> = world
        .query::<&Position>()
        .map(|(_, position)| position.0)
        .collect();
    seen.sort_unstable();
    seen
}

/// A resource nothing ever inserts, so `find_resource` has an absence to report.
#[derive(Debug, Default)]
struct NeverInserted;
impl Resource for NeverInserted {}

#[test]
fn one_reader_over_a_world_view_answers_the_same_from_update_and_from_draw() {
    /// Update reads through `World::view` and stores the answer.
    fn read_in_update(world: &mut World) {
        let seen = positions_in(&world.view());
        world.resource_mut::<Reported>().0 = seen;
    }

    /// Draw reads through the view it was handed — the same function.
    fn read_in_draw(ctx: &mut DrawCtx) {
        let seen = positions_in(&ctx.world);
        // Draw cannot write the world, so it reports outside it.
        println!("{seen:?}");
        assert_eq!(seen, ctx.world.resource::<Reported>().0);
    }

    #[derive(Debug, Default)]
    struct Reported(Vec<i32>);
    impl Resource for Reported {}

    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Startup, spawn_two);
        app.add_system(Update, advance);
        app.add_system(Update, read_in_update);
        app.add_system(Draw, read_in_draw);
    });
    sim.world_mut().insert_resource(Reported::default());
    sim.tick();
    sim.draw();

    assert_eq!(
        sim.world().resource::<Reported>().0,
        vec![11, 12],
        "the reader ran in Update, and Draw agreed with it"
    );
}

#[test]
fn a_view_taken_from_a_world_reads_exactly_what_the_world_holds() {
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Startup, spawn_two);
    });
    sim.tick();

    let world = sim.world();
    let view = world.view();
    assert_eq!(view.entity_count(), world.entity_count());
    assert_eq!(positions_in(&view), vec![1, 2]);
    assert_eq!(view.resource::<Time>().tick, world.resource::<Time>().tick);
    let (entity, _) = world
        .query::<&Position>()
        .next()
        .unwrap_or_else(|| panic!("two entities were spawned"));
    assert!(view.is_alive(entity));
    assert_eq!(view.component::<Position>(entity), world.component(entity));
    assert!(view.find_component::<Position>(entity).is_some());
    assert!(view.find_resource::<NeverInserted>().is_none());
}

/// Run `body`, returning the message it panicked with.
///
/// `AssertUnwindSafe` because a `HeadlessSim` holds boxed systems and the
/// world's command cell: nothing here observes the sim after the panic — it is
/// dropped — so there is no torn state to see.
///
/// No `expect` here: `allow-expect-in-tests` covers `#[test]` functions, not
/// helpers beside them (docs/internal/tooling.md §5).
fn panic_message(body: impl FnOnce()) -> String {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
    std::panic::set_hook(previous);
    let payload = match caught {
        Ok(()) => panic!("expected a panic"),
        Err(payload) => payload,
    };
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => match payload.downcast::<&str>() {
            Ok(message) => (*message).to_owned(),
            Err(_) => panic!("panicked with a payload that is not a string"),
        },
    }
}
