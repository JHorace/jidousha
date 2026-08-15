//! The `Component` marker trait.
//!
//! Key types: `Component`.
//! Depends on: nothing. Must never depend on: `world`, `storage` — components
//! are plain data and know nothing about where they are stored.
//! INVARIANT: the trait carries no methods and no associated items. Components
//! hold data only; logic lives in systems (core.md §3).

/// Marks a plain-data type as storable on an entity.
///
/// Implementing it is a one-liner and adds no items to your type:
///
/// ```
/// use jidousha_core::Component;
///
/// struct Position {
///     x: f32,
///     y: f32,
/// }
/// impl Component for Position {}
///
/// // Zero-sized components are idiomatic as tags.
/// struct Frozen;
/// impl Component for Frozen {}
/// ```
///
/// The `'static + Send + Sync` bounds are free today — the engine is
/// single-threaded (ADR-0002) — and cannot be added later without breaking
/// every game, so they are here from the start.
///
/// A `#[derive(Component)]` that expands to exactly this impl arrives with the
/// public facade; it will generate no new public symbols, so nothing written
/// against the manual impl changes (core.md §3).
pub trait Component: 'static + Send + Sync {}
