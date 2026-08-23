//! The interpolation idiom, run against the real accumulator.
//!
//! `Time::alpha` has one consumer and it is the game: a component of its own
//! holding last tick's position, and `previous.lerp(current, alpha)` submitted
//! from `Draw` (core.md §7, renderer.md §2, e0-findings.md F-048). Nothing in
//! the engine does it, so nothing in the engine's tests would exercise it
//! either — which is how the idiom stayed unusable through several milestones
//! until ADR-0041.
//!
//! This is the engine-side proof that the documented four lines work: a body
//! moving at a fixed rate, frames arriving at a rate that is *not* the tick
//! rate, and the assertion that what is drawn advances between ticks rather
//! than in tick-sized steps. It is also the check a container with no display
//! can make about smooth motion, which is the whole of what "look at it" can
//! mean here.

use jidousha_core::math::Vec2;
use jidousha_core::{
    Color, Component, Depth, Draw, DrawCtx, Quad, Seconds, Simulation, TextureId, Time, Update,
    World,
};

/// A body travelling at a constant speed along X.
struct Body {
    /// World units per second.
    speed: f32,
}
impl Component for Body {}

/// Where the body stood at the start of the tick it is now past — the game's
/// own state, which is the entire point of the idiom.
struct Previous(Vec2);
impl Component for Previous {}

/// Where the body is now.
struct At(Vec2);
impl Component for At {}

/// Registered first: everything below moves away from a remembered position.
fn remember_where_things_were(world: &mut World) {
    for (_, previous, at) in world.query_mut::<(&mut Previous, &At)>() {
        previous.0 = at.0;
    }
}

fn travel(world: &mut World) {
    let dt = world.resource::<Time>().fixed_dt.as_f32();
    for (_, at, body) in world.query_mut::<(&mut At, &Body)>() {
        at.0.x += body.speed * dt;
    }
}

/// Submits one quad per body, centred where the frame says it is.
fn draw_the_bodies(ctx: &mut DrawCtx) {
    let alpha = ctx.world.resource::<Time>().alpha;
    for (_, at, previous) in ctx.world.query::<(&At, &Previous)>() {
        let centre = previous.0.lerp(at.0, alpha);
        ctx.submit(Quad {
            corners: [centre; 4],
            uvs: [Vec2::ZERO; 4],
            tint: Color::WHITE,
            texture: TextureId::WHITE,
            depth: Depth::layer(0),
        });
    }
}

/// A simulation with one body travelling at 60 units a second — one unit a
/// tick, so a drawn X reads directly as "how many ticks along".
fn travelling_body() -> Simulation {
    let mut simulation = Simulation::new(1, Seconds(1.0 / 60.0));
    simulation.add_system(Update, remember_where_things_were);
    simulation.add_system(Update, travel);
    simulation.add_system(Draw, draw_the_bodies);
    let entity = simulation.world_mut().spawn();
    simulation.world_mut().insert(entity, At(Vec2::ZERO));
    simulation.world_mut().insert(entity, Previous(Vec2::ZERO));
    simulation.world_mut().insert(entity, Body { speed: 60.0 });
    simulation
}

/// Where the one quad of the last frame was drawn.
fn drawn_x(simulation: &Simulation) -> f32 {
    let quads = simulation.submissions().quads();
    assert_eq!(quads.len(), 1, "one body, one quad");
    quads[0].corners[0].x
}

/// Where the body actually is — what an uninterpolated Draw would submit.
fn committed_x(simulation: &Simulation) -> f32 {
    let Some((_, at)) = simulation.world().view().query::<&At>().next() else {
        panic!("one body");
    };
    at.0.x
}

#[test]
fn a_frame_between_two_ticks_draws_the_body_between_them() {
    let mut simulation = travelling_body();
    // A frame and a half of real time: one tick runs, half a tick is carried.
    simulation.advance(Seconds(1.5 / 60.0), |_, _| {});
    simulation.draw();
    // Tick 1 put the body at 1.0 and tick 0 left it at 0.0, so a frame half way
    // between them draws it at 0.5 — a position no tick ever held.
    let drawn = drawn_x(&simulation);
    assert!((drawn - 0.5).abs() < 1e-5, "{drawn}");
}

#[test]
fn frames_arriving_off_the_tick_rate_still_advance_by_equal_steps() {
    // The defect this exists to prevent, stated as an assertion. At 100 frames
    // a second against a 60Hz tick the ticks-per-frame sequence is 1, 1, 0, 2,
    // … — so an uninterpolated body draws two frames in the same place and then
    // jumps two ticks, and an interpolated one moves the same distance every
    // frame however the ticks fall.
    let mut simulation = travelling_body();
    let frame = Seconds(1.0 / 100.0);
    // Warm-up, not fudge: the first frames of a run draw a body that has not
    // moved yet — nothing has ticked, so `previous` and `At` are the same
    // point and there is no interval to interpolate across. The claim being
    // made is about a body in motion.
    for _ in 0..4 {
        simulation.advance(frame, |_, _| {});
        simulation.draw();
    }
    let mut drawn = Vec::new();
    let mut committed = Vec::new();
    for _ in 0..40 {
        simulation.advance(frame, |_, _| {});
        simulation.draw();
        drawn.push(drawn_x(&simulation));
        committed.push(committed_x(&simulation));
    }
    let steps: Vec<f32> = drawn.windows(2).map(|pair| pair[1] - pair[0]).collect();
    // 60 units a second for a hundredth of a second: 0.6 units every frame,
    // every frame, with no frame standing still and none jumping.
    for (index, step) in steps.iter().enumerate() {
        assert!(
            (step - 0.6).abs() < 1e-4,
            "frame {index} advanced by {step}, not 0.6 — the drawn positions were {drawn:?}"
        );
    }

    // And the other half of the claim, without which the first half proves
    // nothing: over the *same* frames, the committed positions do not move by
    // equal steps. If they did, this scenario would have no defect in it and
    // the assertion above would be passing for free.
    let committed: Vec<f32> = committed.windows(2).map(|pair| pair[1] - pair[0]).collect();
    assert!(
        committed.contains(&0.0),
        "no frame repeated a position: {committed:?}"
    );
    assert!(
        committed.iter().any(|step| *step >= 1.0),
        "no frame moved a whole tick: {committed:?}"
    );
    // Stand still for a frame, then move a whole tick's worth in the next one —
    // at 100 frames against 60 ticks that is the shape of it, and below the tick
    // rate it is the mirror image, a frame that moves two ticks at once. Either
    // way it is the same defect and the same fix.
}

#[test]
fn a_driver_that_draws_once_per_tick_draws_the_committed_state() {
    // Why interpolating a game does not move a single verify outcome: a
    // per-tick driver's frame lands on the tick it ran, alpha is 1.0, and the
    // lerp is the identity (ADR-0041).
    let mut simulation = travelling_body();
    for tick in 1..=5 {
        simulation.tick();
        simulation.draw();
        let drawn = drawn_x(&simulation);
        assert!(
            (drawn - tick as f32).abs() < 1e-5,
            "tick {tick} drew at {drawn}"
        );
    }
}
