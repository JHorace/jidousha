//! The simulation systems — the `fn(&mut World)` half of the game.
//!
//! Three systems, in this order, and `verify.rs` holds them to it out of
//! `schedule_debug()`:
//!
//! 1. `handle_tap` — the only system that reads `Input`, so the only one a
//!    script drives. A tap on the button or on the man calls `Backend::feed`.
//! 2. `advance_backend` — pump the backend, then rebuild `FeedState` from it.
//!    After this the world's projection is current for this tick.
//! 3. `ease_pop` — relax the tap bounce back toward rest.
//!
//! `handle_tap` before `advance_backend` so a tap registered this tick is in
//! this tick's `FeedState`, and the square the player sees grows on the frame
//! they pressed rather than the one after.

use jidousha::prelude::*;

use crate::backend::{Backend, FeedState};
use crate::game::{self, camera};

/// The tap bounce (GDD §7: "immediate feedback on tap ... even before server
/// confirmation"). `scale` multiplies the square's drawn side; a feed knocks it
/// up and `ease_pop` lets it settle.
#[derive(Clone, Copy, Debug)]
pub struct Pop {
    /// The current scale multiplier, `1.0` at rest.
    pub scale: f32,
}

impl Pop {
    /// How far a single feed pushes the scale above rest.
    pub const KICK: f32 = 0.12;
    /// The most the bounce may ever reach, however fast the taps come — so the
    /// square cannot be knocked off screen by mashing (`verify.rs` leans on
    /// this: the off-screen check uses `SQUARE_MAX * POP_MAX`).
    pub const MAX: f32 = 1.15;
    /// How fast it returns to rest, per second.
    const EASE: f32 = 9.0;
}

impl Default for Pop {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl Resource for Pop {}

/// Startup: the camera, the RNG-seeded backend, the first projection, the pop.
pub fn open_the_world(world: &mut World) {
    world.insert_resource(camera());

    // A harness may plant a backend before the first tick (a fixed board for a
    // check); otherwise seed one from the world's `Rng`.
    if world.find_resource::<Backend>().is_none() {
        let mut rng = world.resource::<Rng>().clone();
        let backend = Backend::local(&mut rng);
        world.insert_resource(backend);
    }
    world.insert_resource(Pop::default());

    // `Time::tick` is 0 inside Startup (it advances then Update runs), so the
    // opening projection is the field at tick 0.
    let opening = world.resource::<Backend>().snapshot(0);
    world.insert_resource(opening);
}

/// Feed the orange man when the pointer taps the button or the man himself.
pub fn handle_tap(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    if !input.pointer().just_pressed(PointerButton::Primary) {
        return;
    }
    let screen = input.pointer().screen;
    let world_point = world.resource::<Camera>().screen_to_world(screen);

    let global = world.resource::<FeedState>().global;
    if !game::is_feed_tap(world_point, game::square_bounds(global)) {
        return;
    }

    world.resource_mut::<Backend>().feed();
    let pop = world.resource_mut::<Pop>();
    pop.scale = (pop.scale + Pop::KICK).min(Pop::MAX);
}

/// Pump the backend and rebuild the projection the rest of the game reads.
pub fn advance_backend(world: &mut World) {
    let tick = world.resource::<Time>().tick;
    world.resource_mut::<Backend>().advance(tick);
    let snapshot = world.resource::<Backend>().snapshot(tick);
    world.insert_resource(snapshot);
}

/// Relax the tap bounce back toward rest.
pub fn ease_pop(world: &mut World) {
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    let pop = world.resource_mut::<Pop>();
    pop.scale += (1.0 - pop.scale) * (Pop::EASE * dt);
    if pop.scale < 1.0 {
        pop.scale = 1.0;
    }
}
