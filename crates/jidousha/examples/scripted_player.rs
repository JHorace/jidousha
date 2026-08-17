//! A player driven by a script instead of by hands: the loop this engine exists
//! for (input.md §5).
//!
//! An agent writes the input, runs the ticks, and asserts on world state. No
//! window is opened, nothing is timed, and the run is identical on every
//! machine — which is what makes "did my change break the jump?" a question a
//! test can answer.
//!
//! Both ways of driving one are here, because they answer different questions:
//!
//! - **`InputScript`** is the session written down in advance. It is a pure
//!   function of the tick, so a test can seek, replay and bisect it. Reach for
//!   it whenever the input does not depend on what the game does back.
//! - **`SnapshotBuilder`** is a controller deciding as it goes: look at the
//!   world, then record a press or a release. Reach for it when the input has to
//!   *respond* — a blind script never returns a ball, so it can prove the
//!   controls work and still say nothing about whether the game is playable.
//!
//! Run it: `cargo run -p jidousha --example scripted_player`

use jidousha::prelude::*;
// Driving input is a testing facility, not something a shipped game does.
use jidousha::testing::{InputEvent, InputScript, InputSnapshot, SnapshotBuilder};

/// Where the player is, in world units.
#[derive(Clone, Copy, Debug)]
struct Position(Vec2);
impl Component for Position {}

/// How fast they are going, in units per second.
#[derive(Clone, Copy, Debug)]
struct Velocity(Vec2);
impl Component for Velocity {}

/// What the player has done, so the assertions have something to check.
#[derive(Clone, Copy, Debug)]
struct Tally {
    jumps: u32,
    shots: u32,
}
impl Component for Tally {}

/// Ground level, in world units. Y is down (ADR-0010), so "up" is negative.
const GROUND: f32 = 0.0;
const RUN_SPEED: f32 = 4.0;
const JUMP_SPEED: f32 = -12.0;
const GRAVITY: f32 = 30.0;

/// How long the run is. Longer than the script, on purpose — see `main`.
const TICKS: u64 = 130;

fn spawn_the_player(world: &mut World) {
    let player = world.spawn();
    world.insert(player, Position(Vec2::new(0.0, GROUND)));
    world.insert(player, Velocity(Vec2::ZERO));
    world.insert(player, Tally { jumps: 0, shots: 0 });
}

/// Read the input once, then write the world: the read-pass/write-pass shape
/// every system with a resource and a query takes (core.md §5, ADR-0013).
fn control_the_player(world: &mut World) {
    let (run, jump, shoot) = {
        let input = world.resource::<Input>();
        let mut run = 0.0;
        if input.held(Key::A) {
            run -= RUN_SPEED;
        }
        if input.held(Key::D) {
            run += RUN_SPEED;
        }
        (
            run,
            input.just_pressed(Key::Space),
            input.pointer().just_pressed(PointerButton::Primary),
        )
    };

    for (_, position, velocity, tally) in
        world.query_mut::<(&Position, &mut Velocity, &mut Tally)>()
    {
        velocity.0.x = run;
        // Jumping only works with your feet on the ground, which is what makes
        // the edge semantics matter: a held Space must not lift you twice.
        if jump && position.0.y >= GROUND {
            velocity.0.y = JUMP_SPEED;
            tally.jumps += 1;
        }
        if shoot {
            tally.shots += 1;
        }
    }
}

fn move_the_player(world: &mut World) {
    let step = world.resource::<Time>().fixed_dt.as_f32();
    for (_, position, velocity) in world.query_mut::<(&mut Position, &mut Velocity)>() {
        velocity.0.y += GRAVITY * step;
        position.0 += velocity.0 * step;
        if position.0.y > GROUND {
            position.0.y = GROUND;
            velocity.0.y = 0.0;
        }
    }
}

fn main() {
    // The whole session, written down. Run right for a second and a half,
    // jumping twice on the way, and fire once at something on the right of the
    // screen.
    let script = InputScript::new()
        .hold(Key::D, 10..100)
        .press(Key::Space, 20)
        .press(Key::Space, 70)
        .pointer_at(64, Vec2::new(640.0, 200.0))
        .click(PointerButton::Primary, 65);

    let mut sim = headless(
        GameConfig {
            title: "scripted player",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, spawn_the_player);
            app.add_system(Update, control_the_player);
            app.add_system(Update, move_the_player);
        },
    );
    // Startup runs on the first tick, which needs the resource to already be
    // there — systems read input, they do not ask whether it exists.
    sim.world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));

    // The script ends at tick 100; the game keeps running to 130. A script
    // says what the *player* does, not how long the world moves — the second
    // jump is still in the air when the last key is let go.
    for tick in 1..=TICKS {
        // The input choke point: one snapshot per tick, and simulation sees
        // nothing else (input.md §1).
        sim.world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        sim.tick();
    }

    let (position, tally) = sim
        .world()
        .query::<(&Position, &Tally)>()
        .map(|(_, position, tally)| (position.0, *tally))
        .next()
        .unwrap_or((Vec2::ZERO, Tally { jumps: 0, shots: 0 }));

    println!(
        "after {TICKS} ticks (script ends at {}): x={:.2}, {} jumps, {} shots",
        script.last_tick(),
        position.x,
        tally.jumps,
        tally.shots
    );

    // Held for 90 ticks at 4 units/second on a 60 Hz timestep: 6 units right.
    assert!(
        (position.x - 6.0).abs() < 1e-4,
        "expected to run 6 units, got {}",
        position.x
    );
    assert_eq!(tally.jumps, 2, "two taps of Space, two jumps");
    assert_eq!(tally.shots, 1, "one click, one shot");
    assert!(
        (position.y - GROUND).abs() < 1e-4,
        "back on the ground by the end"
    );

    // The same script, run again, lands in exactly the same place — the point
    // of the whole exercise.
    let mut again = headless(
        GameConfig {
            title: "scripted player",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, spawn_the_player);
            app.add_system(Update, control_the_player);
            app.add_system(Update, move_the_player);
        },
    );
    again
        .world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));
    for tick in 1..=TICKS {
        again
            .world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        again.tick();
    }
    let replayed = again
        .world()
        .query::<&Position>()
        .map(|(_, position)| [position.0.x.to_bits(), position.0.y.to_bits()])
        .next()
        .unwrap_or([0, 0]);
    let original = [position.x.to_bits(), position.y.to_bits()];
    assert_eq!(replayed, original, "same script, same run, bit for bit");
    println!("replayed the script: identical to the bit");

    chase_a_target();
}

/// The other half: input decided from what the game is doing, tick by tick.
///
/// The player runs towards a target that is not where the script-writer could
/// have known it would be, and stops when it arrives. No script can express
/// that, because the answer on tick 40 depends on the world at tick 39.
///
/// `SnapshotBuilder` is the driver's own accumulator, so this exercises the real
/// edge rules: a key held across many ticks presses once, and letting go
/// releases once. Building a one-tick `InputScript` per tick would press on
/// every one of them.
fn chase_a_target() {
    /// Where the player is told to stop, in world units.
    const TARGET: f32 = 5.0;
    /// How close counts as arrived.
    const REACH: f32 = 0.1;

    let mut sim = headless(
        GameConfig {
            title: "closed-loop player",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, spawn_the_player);
            app.add_system(Update, control_the_player);
            app.add_system(Update, move_the_player);
        },
    );
    sim.world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));

    // A controller sends events, so it has to remember what it is already
    // holding. That is what a keyboard is, and it is why the edges come out
    // right.
    let mut keyboard = SnapshotBuilder::new();
    let mut holding_right = false;
    let mut arrived_at = None;

    for tick in 1..=TICKS {
        let here = sim
            .world()
            .query::<&Position>()
            .map(|(_, position)| position.0.x)
            .next()
            .unwrap_or(0.0);

        // The decision. This is the line a script cannot hold, because `here`
        // is not known until the tick before.
        let want_right = here < TARGET - REACH;
        if want_right != holding_right {
            keyboard.record(if want_right {
                InputEvent::KeyPressed(Key::D)
            } else {
                InputEvent::KeyReleased(Key::D)
            });
            holding_right = want_right;
        }
        if !want_right && arrived_at.is_none() {
            arrived_at = Some(tick);
        }

        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();
    }

    let finished = sim
        .world()
        .query::<&Position>()
        .map(|(_, position)| position.0.x)
        .next()
        .unwrap_or(0.0);
    let tally = sim
        .world()
        .query::<&Tally>()
        .map(|(_, tally)| *tally)
        .next()
        .unwrap_or(Tally { jumps: 0, shots: 0 });

    // Report the numbers the assertion judged, not just its verdict: nobody
    // writing a game this way can look at it, so the message is the instrument.
    println!(
        "closed loop: stopped at x={finished:.2} (target {TARGET}), arrived on tick {:?}, \
         {} jumps",
        arrived_at, tally.jumps
    );
    assert!(
        (finished - TARGET).abs() < 0.5,
        "expected to stop near {TARGET}, stopped at {finished:.3} after {TICKS} ticks"
    );
    assert!(
        arrived_at.is_some_and(|tick| tick < TICKS),
        "expected to arrive before the run ended; arrived_at={arrived_at:?}"
    );
    // Space was never recorded, so no edge can have been invented for it — the
    // check that a one-tick script per tick would have failed.
    assert_eq!(
        tally.jumps, 0,
        "a controller that never presses Space must produce no press edge for it"
    );
}
