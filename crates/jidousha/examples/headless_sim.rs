//! A whole game, with no window: config, systems, phases, and assertions.
//!
//! This is the shape every test and every `tools/verify` run uses — and the
//! shape the windowed driver will run unchanged (core.md §8). Orbiting motes
//! chase a drifting anchor; the run asserts exact positions, because the same
//! seed and the same systems always produce them.
//!
//! Run it: `cargo run -p jidousha --example headless_sim`

use jidousha::prelude::*;

/// Where something is, in world units.
#[derive(Clone, Copy, Debug)]
struct Position(Vec2);
impl Component for Position {}

/// How far from the anchor a mote orbits, and how fast.
#[derive(Clone, Copy, Debug)]
struct Orbit {
    radius: f32,
    speed: Radians,
}
impl Component for Orbit {}

/// The one thing everything orbits.
#[derive(Clone, Copy, Debug)]
struct Anchor;
impl Component for Anchor {}

fn spawn_the_field(world: &mut World) {
    let anchor = world.spawn();
    world.insert(anchor, Position(Vec2::new(0.0, 0.0)));
    world.insert(anchor, Anchor);

    // Randomness comes from the seeded generator, so the field is the same
    // every run — which is what lets this example assert exact numbers.
    for index in 0..5 {
        let (radius, speed) = {
            let rng = world.resource_mut::<Rng>();
            (
                2.0 + rng.next_f32() * 3.0,
                Radians(0.05 + rng.next_f32() * 0.1),
            )
        };
        let mote = world.spawn();
        world.insert(mote, Position(Vec2::new(radius, index as f32)));
        world.insert(mote, Orbit { radius, speed });
    }
}

/// Drift the anchor along +X, one unit of distance per second of game time.
fn drift_the_anchor(world: &mut World) {
    let step = world.resource::<Time>().fixed_dt.as_f32();
    for (_, position, ()) in world.query_mut::<(&mut Position, With<Anchor>)>() {
        position.0.x += step;
    }
}

/// Swing each mote around the anchor.
///
/// The anchor's position is read in a pass of its own, because a mutable query
/// borrows the whole world — the read-pass/write-pass pattern (core.md §5).
fn orbit_the_anchor(world: &mut World) {
    let anchor = world
        .query::<(&Position, With<Anchor>)>()
        .map(|(_, position, ())| position.0)
        .next()
        .unwrap_or(Vec2::ZERO);

    for (_, position, orbit) in world.query_mut::<(&mut Position, &Orbit)>() {
        let offset = position.0 - anchor;
        // Engine trig, not the standard library's: same bits everywhere
        // (ADR-0009).
        let turned = rotate(offset, orbit.speed);
        position.0 = anchor + turned.normalize_or_zero() * orbit.radius;
    }
}

/// A Draw system: it reads the world and reports through a channel outside it.
///
/// It could not write the world if it tried — `DrawCtx` exposes no method that
/// does (ADR-0008).
fn draw_the_field(ctx: &mut DrawCtx) {
    let motes = ctx.world.query::<(&Position, &Orbit)>().count();
    let tick = ctx.world.resource::<Time>().tick;
    println!("frame at tick {tick}: {motes} motes");
}

fn main() {
    let mut sim = headless(
        GameConfig {
            title: "headless sim",
            seed: 7,
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, spawn_the_field);
            app.add_system(Update, drift_the_anchor);
            app.add_system(Update, orbit_the_anchor);
            app.add_system(Draw, draw_the_field);
        },
    );
    println!("{}", sim.schedule_debug());

    // A second of game time, one tick at a time, with a frame drawn every
    // sixth tick — roughly what a 10 fps display would ask for.
    for tick in 1..=60 {
        sim.tick();
        if tick % 6 == 0 {
            sim.draw();
        }
    }

    let time = sim.world().resource::<Time>();
    println!("ran {} ticks ({})", time.tick, time.elapsed);
    assert_eq!(time.tick, 60);

    let anchor = sim
        .world()
        .query::<(&Position, With<Anchor>)>()
        .map(|(_, position, ())| position.0)
        .next()
        .unwrap_or(Vec2::ZERO);
    println!("anchor drifted to {anchor:?}");
    assert!(
        (anchor.x - 1.0).abs() < 1e-4,
        "one second of drift at one unit per second"
    );

    // Every mote is still the right distance from the anchor: the orbit held.
    for (entity, position, orbit) in sim.world().query::<(&Position, &Orbit)>() {
        let distance = (position.0 - anchor).length();
        assert!(
            (distance - orbit.radius).abs() < 1e-3,
            "{entity:?} drifted off its orbit: {distance} vs {}",
            orbit.radius
        );
    }
    println!("all 5 motes held their orbits");

    // Replay: the same config and systems, run again, land in the same place.
    let mut again = headless(
        GameConfig {
            title: "headless sim",
            seed: 7,
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, spawn_the_field);
            app.add_system(Update, drift_the_anchor);
            app.add_system(Update, orbit_the_anchor);
        },
    );
    for _ in 0..60 {
        again.tick();
    }
    let replayed: Vec<[u32; 2]> = again
        .world()
        .query::<&Position>()
        .map(|(_, position)| [position.0.x.to_bits(), position.0.y.to_bits()])
        .collect();
    let original: Vec<[u32; 2]> = sim
        .world()
        .query::<&Position>()
        .map(|(_, position)| [position.0.x.to_bits(), position.0.y.to_bits()])
        .collect();
    assert_eq!(replayed, original, "same seed, same run, bit for bit");
    println!("replayed the whole run: identical to the bit");
}
