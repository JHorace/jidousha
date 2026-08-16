//! Homing missiles: the canonical shape for **reading other entities while
//! mutating** (core.md §5, ADR-0013).
//!
//! Each missile carries a `Target(Entity)` and steers toward that target's
//! `Position`. The tempting version nests a point lookup inside a mutable
//! query and does not compile:
//!
//! ```text
//! for (_, position, target) in world.query_mut::<(&mut Position, &Target)>() {
//!     let goal = world.component::<Position>(target.0);   // ✗ world is exclusively borrowed
//!     position.x += (goal.x - position.x).signum();
//! }
//! ```
//!
//! The correct form is two passes: a read-only `query` collects what the write
//! needs, then `query_mut` consumes it. Nothing in the engine helps you do
//! this, on purpose — plain `collect()` is the one way (agent-practices §5.3).
//!
//! Run it: `cargo run -p jidousha --example homing`

use jidousha::prelude::*;

/// Where something is. Integers keep the example's arithmetic exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}
impl Component for Position {}

/// The entity a missile is chasing.
#[derive(Clone, Copy, Debug)]
struct Target(Entity);
impl Component for Target {}

/// Move every missile one step toward its target.
///
/// Pass one reads: for each missile, where is its target? Pass two writes,
/// using only what pass one collected. The `Vec` between them is the whole
/// pattern.
fn home_toward_targets(world: &mut World) {
    // Pass one — read-only. The world stays readable, so point lookups on
    // other entities are fine here.
    let mut goals: Vec<(Entity, Position)> = Vec::new();
    for (missile, _, target) in world.query::<(&Position, &Target)>() {
        // A target may have been despawned; find_component says so rather than
        // panicking, and the missile simply does not move this step.
        if let Some(goal) = world.find_component::<Position>(target.0) {
            goals.push((missile, *goal));
        }
    }

    // Pass two — write. Iterating the collected list keeps each write pointed
    // at one entity, so `component_mut` is enough; a `query_mut` pass works the
    // same way when the write touches every matching entity.
    for (missile, goal) in goals {
        let position = world.component_mut::<Position>(missile);
        position.x += (goal.x - position.x).signum();
        position.y += (goal.y - position.y).signum();
    }
}

fn main() {
    let mut world = World::new();

    let runner = world.spawn();
    world.insert(runner, Position { x: 10, y: 4 });

    let missile = world.spawn();
    world.insert(missile, Position { x: 0, y: 0 });
    world.insert(missile, Target(runner));

    let stray = world.spawn();
    world.insert(stray, Position { x: -5, y: -5 });
    world.insert(stray, Target(runner));

    for step in 1..=4 {
        home_toward_targets(&mut world);
        println!(
            "step {step}: missile {:?}, stray {:?}",
            world.component::<Position>(missile),
            world.component::<Position>(stray)
        );
    }

    // Four steps of one unit each, diagonally where both axes differ.
    assert_eq!(
        world.component::<Position>(missile),
        &Position { x: 4, y: 4 },
        "the missile closed four steps toward the runner"
    );
    assert_eq!(
        world.component::<Position>(stray),
        &Position { x: -1, y: -1 },
        "the stray closed four steps too"
    );

    // A despawned target stops its missiles rather than crashing them.
    world.despawn(runner);
    let before = *world.component::<Position>(missile);
    home_toward_targets(&mut world);
    assert_eq!(
        world.component::<Position>(missile),
        &before,
        "a missile whose target is gone holds position"
    );
    println!("target despawned: missile holds at {before:?}");
}
