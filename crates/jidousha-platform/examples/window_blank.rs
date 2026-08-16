//! A window, a running simulation, and nothing drawn in it yet (core.md §11, M5).
//!
//! This is the milestone's exit criterion made runnable: a real window on
//! Linux, Windows and the web, with the fixed-timestep loop behind it. Nothing
//! appears inside the window — a surface and something to put on it arrive with
//! R1 — so what to look for is the window itself, a title bar reading "blank
//! window", and a program that closes when you close it.
//!
//! The counting system underneath proves the loop is real rather than idle: it
//! reports every simulated second, and those seconds are ticks, not wall clock.
//!
//! Run it: `cargo run -p jidousha-platform --example window_blank`
//!
//! DELIBERATE: `tools/test` builds this example but does not run it, because it
//! opens a window and waits for a person. It is the only example that is not
//! run, and the runner says so in its output rather than quietly skipping it
//! (tooling.md).

use jidousha_core::{GameConfig, Resource, Startup, Time, Update, World};

/// How many ticks have run, so the window can prove it is doing something.
#[derive(Debug)]
struct Heartbeat {
    seconds_reported: u64,
}
impl Resource for Heartbeat {}

fn start_the_heartbeat(world: &mut World) {
    world.insert_resource(Heartbeat {
        seconds_reported: 0,
    });
    println!("window open — close it to quit");
}

/// Report each simulated second as it passes.
///
/// The seconds counted here are *simulated*: sixty ticks, whatever the frame
/// rate did. On a machine that stutters, this still reports one second per
/// second of game time, which is the whole point of the fixed timestep
/// (core.md §7).
fn report_each_second(world: &mut World) {
    let elapsed = world.resource::<Time>().elapsed.as_f32();
    let tick = world.resource::<Time>().tick;
    let heartbeat = world.resource_mut::<Heartbeat>();
    let whole = elapsed as u64;
    if whole > heartbeat.seconds_reported {
        heartbeat.seconds_reported = whole;
        println!("{whole}s of game time, {tick} ticks");
    }
}

fn main() {
    let result = jidousha_platform::run(
        GameConfig {
            title: "blank window",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, start_the_heartbeat);
            app.add_system(Update, report_each_second);
        },
    );

    // The failure a headless machine gets, printed the way every engine error
    // is printed — and it names `headless` as the thing to do instead.
    if let Err(error) = result {
        println!("{error}");
        return;
    }
    println!("window closed");
}
