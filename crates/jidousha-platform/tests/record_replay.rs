//! I2's exit criterion: record a session, replay it, get the same game
//! (input.md §8).
//!
//! A scripted run is played once while a `Recording` is written, and then a
//! second time driven by nothing but that recording. The world is hashed after
//! every tick and the two runs must agree, tick for tick. Assets resolve on the
//! ticks the recording says rather than whenever the second run's store feels
//! like it, which is what makes readiness part of the timeline rather than
//! something the environment decides (assets.md §4).
//!
//! Lives in the platform crate because it is the only one that can see all
//! three pieces at once — core's simulation, input's recording, and assets'
//! store. Nothing here touches winit.

use jidousha_assets::{
    AssetStatus, Assets, MemorySource, ReplaySource, TextureData, TextureHandle, encode_png,
};
use jidousha_core::math::Vec2;
use jidousha_core::{
    Component, GameConfig, HeadlessSim, Resource, Rng, Startup, Time, Update, World, headless,
};
use jidousha_input::{AssetReady, Input, InputScript, Key, PointerButton, Recording, TickRecord};

/// How long the scripted session runs.
const TICKS: u64 = 120;

/// The seed the session is played with.
const SEED: u64 = 0x51D2_7E57;

#[derive(Clone, Copy, Debug)]
struct Position(Vec2);
impl Component for Position {}

#[derive(Clone, Copy, Debug)]
struct Wander(u32);
impl Component for Wander {}

/// The art the session waits on, so readiness is part of what replays.
#[derive(Debug)]
struct Art {
    hero: TextureHandle,
    missing: TextureHandle,
}
impl Resource for Art {}

/// What the game did, in a form two runs can be compared by.
#[derive(Debug, Default, PartialEq, Eq)]
struct Trace(Vec<u64>);
impl Resource for Trace {}

fn set_the_scene(world: &mut World) {
    let assets = world.resource_mut::<Assets>();
    let hero = assets.load_texture("sprites/hero.png");
    let missing = assets.load_texture("sprites/nowhere.png");
    world.insert_resource(Art { hero, missing });
    world.insert_resource(Trace::default());
}

/// Move on input, spawn on a die roll, and branch on what has loaded.
///
/// The branch is the point: a game whose behaviour depends on readiness is
/// exactly the game a recording has to reproduce, and one that ignored it would
/// pass this test without the readiness records doing anything.
fn play(world: &mut World) {
    let (left, right, fire, clicked) = {
        let Some(input) = world.find_resource::<Input>() else {
            return;
        };
        (
            input.held(Key::A),
            input.held(Key::D),
            input.just_pressed(Key::Space),
            input.pointer().just_pressed(PointerButton::Primary),
        )
    };
    let ready = {
        let art = world.resource::<Art>();
        let assets = world.resource::<Assets>();
        (
            assets.status(art.hero) == AssetStatus::Ready,
            assets.status(art.missing) == AssetStatus::Failed,
        )
    };

    let step = Vec2::new(f32::from(right) - f32::from(left), 0.0);
    // Twice as fast once the art is in, so readiness changes the world rather
    // than merely being observable.
    let speed = if ready.0 { 2.0 } else { 1.0 };
    for (_, position) in world.query_mut::<&mut Position>() {
        position.0 += step * speed;
    }

    if fire || clicked {
        let roll = world.resource_mut::<Rng>().next_u32();
        let spawned = world.spawn();
        world.insert(spawned, Position(Vec2::new(roll as f32 % 17.0, 0.0)));
        world.insert(spawned, Wander(roll));
    }
    if ready.1 {
        // The failed load is observable too, and drives a different branch.
        for (_, wander) in world.query_mut::<&mut Wander>() {
            wander.0 = wander.0.wrapping_mul(3).wrapping_add(1);
        }
    }

    let hash = hash_world(world);
    world.resource_mut::<Trace>().0.push(hash);
}

/// A hash of everything the game can see, iteration order included.
///
/// Deliberately hand-rolled and deliberately total: a difference in archetype
/// or row order fails here too, which is what the core replay test found worth
/// checking (core.md §7).
fn hash_world(world: &World) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut eat = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    // `Entity` is opaque, and its `Debug` is the public way to see the index
    // and generation it carries — the same approach core's own replay test
    // takes, for the same reason.
    for (entity, position) in world.query::<&Position>() {
        eat(format!("{entity:?}").as_bytes());
        eat(&position.0.x.to_le_bytes());
        eat(&position.0.y.to_le_bytes());
    }
    for (entity, wander) in world.query::<&Wander>() {
        eat(format!("{entity:?}").as_bytes());
        eat(&wander.0.to_le_bytes());
    }
    eat(&world.resource::<Time>().tick.to_le_bytes());
    hash
}

/// The hero's file, as a disk would hand it over.
///
/// A real PNG rather than three bytes: a texture request decodes what it
/// resolves (assets.md §3), so a stand-in would script a *failure* and this
/// test is about an asset that arrives.
fn hero_png() -> Vec<u8> {
    encode_png(&TextureData {
        width: 2,
        height: 2,
        rgba: vec![255; 16],
    })
}

/// The store the session loads from, with its scripted arrival ticks.
fn scripted_store() -> MemorySource {
    let mut source = MemorySource::new();
    source.insert("sprites/hero.png", hero_png());
    // Arrives partway through, so the run spends time on both sides of it.
    source.complete_at("sprites/hero.png", 40);
    // `sprites/nowhere.png` is absent, and fails at the commit that resolves it.
    source.complete_at("sprites/nowhere.png", 70);
    source
}

/// The input the session is played with.
fn script() -> InputScript {
    InputScript::new()
        .hold(Key::D, 5..45)
        .hold(Key::A, 60..90)
        .press(Key::Space, 20)
        .press(Key::Space, 96)
        .pointer_at(30, Vec2::new(120.0, 64.0))
        .click(PointerButton::Primary, 31)
}

fn new_sim(source: impl jidousha_assets::ByteSource) -> HeadlessSim {
    let mut sim = headless(
        GameConfig {
            seed: SEED,
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, set_the_scene);
            app.add_system(Update, play);
        },
    );
    sim.world_mut().insert_resource(Assets::new(source));
    sim
}

/// Play the scripted session, writing down everything that happened.
fn record() -> (Recording, Vec<u64>) {
    let mut sim = new_sim(scripted_store());
    let script = script();
    let mut recording = Recording::new(SEED, GameConfig::default().fixed_dt);

    for tick in 1..=TICKS {
        // The driver's order, by hand: commit, then set the tick's input, then
        // tick (assets.md §4, input.md §2).
        let assets = match sim.world_mut().find_resource_mut::<Assets>() {
            Some(assets) => assets,
            None => panic!("the store is inserted before the first tick"),
        };
        assets.commit(tick);
        let readiness: Vec<AssetReady> = assets
            .resolved()
            .iter()
            .map(|resolution| AssetReady {
                request: resolution.request.bits(),
                arrived: resolution.arrived,
            })
            .collect();

        let snapshot = script.snapshot_at(tick);
        sim.world_mut()
            .insert_resource(Input::new(snapshot.clone()));
        sim.tick();

        recording.push(TickRecord {
            tick,
            input: snapshot,
            readiness,
        });
    }
    let trace = sim.world().resource::<Trace>().0.clone();
    (recording, trace)
}

/// Play a recording back, with nothing but the recording to go on.
fn replay(recording: &Recording) -> Vec<u64> {
    // Readiness comes from the file, not from the store's own scheduling — so
    // the schedule the second run's source was built with is deliberately
    // *wrong*, and only the recording can make it come out right.
    let schedule: Vec<(u64, u64)> = recording
        .ticks()
        .iter()
        .flat_map(|record| {
            record
                .readiness
                .iter()
                .map(move |ready| (ready.request, record.tick))
        })
        .collect();
    let mut store = MemorySource::new();
    store.insert("sprites/hero.png", hero_png());
    // Everything resolves immediately here; the replay source holds it back.
    let mut sim = new_sim(ReplaySource::new(store, schedule));

    for record in recording.ticks() {
        let assets = match sim.world_mut().find_resource_mut::<Assets>() {
            Some(assets) => assets,
            None => panic!("the store is inserted before the first tick"),
        };
        assets.commit(record.tick);
        sim.world_mut()
            .insert_resource(Input::new(record.input.clone()));
        sim.tick();
    }
    sim.world().resource::<Trace>().0.clone()
}

#[test]
fn a_recorded_session_replays_to_the_same_world_every_tick() {
    let (recording, recorded) = record();
    let replayed = replay(&recording);

    assert_eq!(recorded.len(), TICKS as usize, "every tick was traced");
    assert_eq!(
        replayed.len(),
        recorded.len(),
        "the replay ran the same number of ticks"
    );
    for (tick, (a, b)) in recorded.iter().zip(&replayed).enumerate() {
        assert_eq!(a, b, "worlds diverged at tick {}", tick + 1);
    }
}

#[test]
fn a_session_replays_the_same_after_a_round_trip_through_bytes() {
    // The file is the artifact, so the bytes are what has to replay — not the
    // in-memory value that happened to produce them.
    let (recording, recorded) = record();
    let Ok(from_bytes) = Recording::try_decode(&recording.encode()) else {
        panic!("what was just written must read back");
    };
    assert_eq!(replay(&from_bytes), recorded);
}

#[test]
fn the_recording_actually_carries_the_asset_timeline() {
    // Guards against a test that would pass with the readiness records empty:
    // if nothing were written down, the replay's store would resolve everything
    // at tick 1 and the traces would diverge. Worth asserting directly, because
    // "the test passes" is not the same as "the test is testing something".
    let (recording, _) = record();
    let readied: Vec<(u64, u64, bool)> = recording
        .ticks()
        .iter()
        .flat_map(|record| {
            record
                .readiness
                .iter()
                .map(move |ready| (record.tick, ready.request, ready.arrived))
        })
        .collect();
    assert_eq!(
        readied,
        vec![(40, 0, true), (70, 1, false)],
        "the hero arrives at 40 and the missing file fails at 70"
    );
}

#[test]
fn a_replay_whose_readiness_is_dropped_does_not_match() {
    // The negative control for the test above. If the world hash did not depend
    // on when assets resolved, every assertion here would hold with the
    // readiness records thrown away — and this whole milestone would be
    // asserting nothing.
    let (recording, recorded) = record();
    let mut store = MemorySource::new();
    store.insert("sprites/hero.png", hero_png());
    let mut sim = new_sim(store);
    for record in recording.ticks() {
        let Some(assets) = sim.world_mut().find_resource_mut::<Assets>() else {
            panic!("the store is inserted before the first tick");
        };
        assets.commit(record.tick);
        sim.world_mut()
            .insert_resource(Input::new(record.input.clone()));
        sim.tick();
    }
    let without = sim.world().resource::<Trace>().0.clone();
    assert_ne!(
        without, recorded,
        "readiness has to matter, or replaying it proves nothing"
    );
}

#[test]
fn a_recording_replays_from_a_file_that_was_cut_short() {
    // The append-only CONTRACT, end to end: a session that crashed leaves a
    // file valid up to its last whole tick, and that prefix replays.
    let (recording, recorded) = record();
    let whole = recording.encode();
    let Ok(partial) = Recording::try_decode(&whole[..whole.len() * 2 / 3]) else {
        panic!("a truncated recording is not an error");
    };
    let ticks = partial.ticks().len();
    assert!(
        ticks > 0 && ticks < TICKS as usize,
        "a real prefix: {ticks}"
    );
    assert_eq!(replay(&partial), recorded[..ticks]);
}

#[test]
fn the_same_script_records_the_same_bytes_twice() {
    // Byte-stability, the property that lets a recording be checked in and
    // diffed (input.md §5).
    assert_eq!(record().0.encode(), record().0.encode());
}
