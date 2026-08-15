//! Deterministic simulation core: entities, components, queries, schedule, time, app model.
//!
//! Key types: `World`, `Entity`, `Component`, `EntityDeadError`.
//! Depends on: nothing. Must never depend on any other jidousha crate, and
//! never on `winit`/`wgpu` (core.md §1 CONTRACT).
//! INVARIANT: compiles on every target including `wasm32-unknown-unknown` with
//! zero `cfg` branches in simulation logic, and observes no wall clock
//! (ADR-0005). Nothing here reads a `HashMap` in an order-observable path, so
//! world state stays a pure function of the operation history (core.md §4).
//!
//! Built so far (`docs/internal/core.md` §11): M3 — entities, archetype
//! storage, queries, resources, commands, the schedule, and the fixed-timestep
//! clock. The app lifecycle and Draw phase land in M4.
//!
//! ```
//! use jidousha_core::{Component, World};
//!
//! #[derive(Debug, PartialEq)]
//! struct Position {
//!     x: i32,
//!     y: i32,
//! }
//! impl Component for Position {}
//!
//! let mut world = World::new();
//! let entity = world.spawn();
//! world.insert(entity, Position { x: 1, y: 2 });
//!
//! world.component_mut::<Position>(entity).y += 1;
//! assert_eq!(world.component::<Position>(entity), &Position { x: 1, y: 3 });
//! ```

mod access;
mod archetype;
mod command;
mod component;
mod entity;
mod error;
mod query;
mod resource;
mod rng;
mod schedule;
mod simulation;
mod time;
mod units;
mod world;

pub use access::{ColumnsMut, ColumnsRef, QueryAccess};
pub use command::{Bundle, CommandKind, Commands};
pub use component::Component;
pub use entity::Entity;
pub use error::EntityDeadError;
pub use query::{Query, QueryIter, QueryIterMut, ReadOnlyQuery, With, Without};
pub use resource::Resource;
pub use rng::Rng;
pub use schedule::{Phase, Startup, Update};
pub use simulation::Simulation;
pub use time::Time;
pub use units::Seconds;
pub use world::World;
