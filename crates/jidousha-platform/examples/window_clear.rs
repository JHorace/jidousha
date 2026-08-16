//! A window with a color in it (renderer.md §11, R1).
//!
//! The first thing the engine puts on a screen. There is still nothing *drawn*
//! — the sprite pipeline is R2 — so what a GPU does here is clear the surface
//! and present it. That is the whole of R1, and it is more than it sounds: it
//! means a surface exists, an adapter and a device were negotiated, the swap
//! chain is configured for the window's real size, and the frame loop is
//! reaching the backend.
//!
//! The color cycles slowly, driven by simulated time rather than wall clock, so
//! a stutter changes nothing about which color belongs to which tick. Resize
//! the window and it should keep filling it, without stretching or tearing.
//!
//! Run it: `cargo run -p jidousha-platform --example window_clear`
//!
//! DELIBERATE: built but not run by `tools/test`, like `window_blank` — it
//! opens a window and waits for a person (tooling.md).

use jidousha_core::{Color, GameConfig, Startup, Time, Update, World};
use jidousha_render_core::Camera;

/// How long one trip around the color wheel takes, in simulated seconds.
const CYCLE: f32 = 6.0;

fn insert_the_camera(world: &mut World) {
    // The camera carries the clear color, so a game changes the background by
    // changing the camera rather than by talking to the renderer.
    world.insert_resource(Camera {
        clear_color: Color::BLACK,
        ..Camera::default()
    });
    println!("window open — close it to quit");
}

/// Walk the clear color around a simple wheel, once per `CYCLE` seconds.
///
/// Simulated time, not wall clock: the color at tick 600 is the same color on
/// every machine, which is the same promise every other part of the engine
/// makes (core.md §7).
fn cycle_the_color(world: &mut World) {
    let elapsed = world.resource::<Time>().elapsed.as_f32();
    let phase = (elapsed % CYCLE) / CYCLE;
    // Three ramps a third of a turn apart — a hand-rolled hue sweep, since a
    // color wheel is not worth a dependency and trig is banned outside the
    // engine's own (ADR-0009).
    let channel = |offset: f32| {
        let position = (phase + offset).fract();
        let ramp = if position < 0.5 {
            position * 2.0
        } else {
            2.0 - position * 2.0
        };
        ramp.clamp(0.0, 1.0)
    };
    let camera = world.resource_mut::<Camera>();
    camera.clear_color = Color::rgb(channel(0.0), channel(1.0 / 3.0), channel(2.0 / 3.0));
}

/// Report the viewport whenever it changes, so a resize is visible in the
/// terminal as well as on screen.
fn report_resizes(world: &mut World) {
    #[derive(Debug)]
    struct LastSeen(jidousha_render_core::PhysicalSize);
    impl jidousha_core::Resource for LastSeen {}

    let viewport = world.resource::<Camera>().viewport;
    match world.find_resource::<LastSeen>() {
        Some(last) if last.0 == viewport => {}
        _ => {
            println!("viewport: {}x{}", viewport.width, viewport.height);
            world.insert_resource(LastSeen(viewport));
        }
    }
}

fn main() {
    let result = jidousha_platform::run(
        GameConfig {
            title: "window clear",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, insert_the_camera);
            app.add_system(Update, cycle_the_color);
            app.add_system(Update, report_resizes);
        },
    );

    // On a machine with no display this is the §9 message naming `headless` as
    // the thing to do instead.
    if let Err(error) = result {
        println!("{error}");
        return;
    }
    println!("window closed");
}
