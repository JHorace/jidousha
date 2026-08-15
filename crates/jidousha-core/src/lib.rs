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
//! Built so far (`docs/internal/core.md` §11): M2 — entities, archetype
//! storage, and queries. The schedule, resources and commands land in M3.
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

mod archetype;
mod component;
mod entity;
mod error;
mod query;
mod world;

pub use component::Component;
pub use entity::Entity;
pub use error::EntityDeadError;
pub use query::{
    ColumnsMut, ColumnsRef, Query, QueryAccess, QueryIter, QueryIterMut, ReadOnlyQuery, With,
    Without,
};
pub use world::World;
