//! The Draw phase: a world you can read and cannot write (ADR-0008).
//!
//! Key types: `DrawCtx`, `WorldView`, `Draw`.
//! Depends on: `query`, `resource`, `world`. Must never depend on: `simulation`.
//! INVARIANT: `WorldView` exposes no method that mutates. Draw runs once per
//! rendered frame, not once per tick, so a write here would make simulation
//! state depend on frame rate — the one thing the fixed timestep exists to
//! prevent. Nothing is checked at runtime because nothing needs to be: the
//! mutating methods do not exist on the type.

use crate::component::Component;
use crate::entity::Entity;
use crate::query::{QueryIter, ReadOnlyQuery};
use crate::resource::Resource;
use crate::world::World;

/// A read-only view of the world, handed to Draw systems.
///
/// It carries the same query syntax as [`World`], restricted to read-only
/// access: `&T` yes, `&mut T` no — the bound reports the difference as a
/// compile error naming the fix (ADR-0008).
///
/// ```
/// # use jidousha_core::{Component, DrawCtx};
/// # #[derive(Debug)] struct Position(i32);
/// # impl Component for Position {}
/// fn draw_positions(ctx: &mut DrawCtx) {
///     for (entity, position) in ctx.world.query::<&Position>() {
///         println!("{entity:?} at {position:?}");
///     }
/// }
/// ```
pub struct WorldView<'w> {
    world: &'w World,
}

impl<'w> WorldView<'w> {
    pub(crate) fn new(world: &'w World) -> Self {
        Self { world }
    }

    /// Iterate every entity matching a read-only query.
    ///
    /// A `&mut T` part fails to compile: see [`ReadOnlyQuery`].
    pub fn query<Q: ReadOnlyQuery<'w>>(&self) -> QueryIter<'w, Q> {
        self.world.query::<Q>()
    }

    /// The `T` on `entity`.
    ///
    /// # Panics
    ///
    /// If `entity` is not alive or carries no `T`. Use
    /// [`WorldView::find_component`] where absence is expected.
    #[must_use]
    pub fn component<T: Component>(&self, entity: Entity) -> &'w T {
        self.world.component::<T>(entity)
    }

    /// The `T` on `entity`, or `None` if it has none — or is not alive.
    #[must_use]
    pub fn find_component<T: Component>(&self, entity: Entity) -> Option<&'w T> {
        self.world.find_component::<T>(entity)
    }

    /// The `T` resource.
    ///
    /// # Panics
    ///
    /// If the world has no `T`. Use [`WorldView::find_resource`] where absence
    /// is expected.
    #[must_use]
    pub fn resource<T: Resource>(&self) -> &'w T {
        self.world.resource::<T>()
    }

    /// The `T` resource, or `None` if the world has none.
    #[must_use]
    pub fn find_resource<T: Resource>(&self) -> Option<&'w T> {
        self.world.find_resource::<T>()
    }

    /// Whether `entity` is still live.
    #[must_use]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.world.is_alive(entity)
    }

    /// How many entities are alive.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.world.entity_count()
    }
}

/// What a Draw system is called with: the world to read, and the sink to draw
/// into.
///
/// ```
/// # use jidousha_core::{Component, DrawCtx, GameConfig, Draw, headless};
/// # #[derive(Debug)] struct Position(i32);
/// # impl Component for Position {}
/// fn draw_world(ctx: &mut DrawCtx) {
///     for (_entity, _position) in ctx.world.query::<&Position>() {
///         // ctx.draw(...) — the submission sink arrives with the renderer.
///     }
/// }
///
/// let mut sim = headless(GameConfig::default(), |app| {
///     app.add_system(Draw, draw_world);
/// });
/// ```
///
/// The submission sink is not here yet: the renderer owns its shape, and
/// inventing a placeholder vocabulary now would only have to be unlearned
/// (core.md §11, R0). What M4 delivers is the *signature* and the read-only
/// world, so no game's Draw systems need rewriting when the sink lands.
pub struct DrawCtx<'w> {
    /// The world, read-only.
    pub world: WorldView<'w>,
}

impl<'w> DrawCtx<'w> {
    pub(crate) fn new(world: &'w World) -> Self {
        Self {
            world: WorldView::new(world),
        }
    }
}
