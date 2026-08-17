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
use crate::visual::Quad;
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

    /// The `T` on `entity`, panicking if it has none.
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

    /// The `T` resource, panicking if the world has none.
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
/// The sink is [`submit`](DrawCtx::submit), which takes the one thing the
/// engine draws: a [`Quad`]. Games do not call it directly — they call
/// `ctx.sprite(...)`, `ctx.rect(...)` and friends, which `jidousha-render-core`
/// adds, and which expand into quads. That split is what keeps this crate free
/// of renderer machinery while `DrawCtx` still lives here (ADR-0008, ADR-0015).
pub struct DrawCtx<'w> {
    /// The world, read-only.
    pub world: WorldView<'w>,
    submissions: &'w mut Submissions,
}

impl<'w> DrawCtx<'w> {
    pub(crate) fn new(world: &'w World, submissions: &'w mut Submissions) -> Self {
        Self {
            world: WorldView::new(world),
            submissions,
        }
    }

    /// Draw one quad.
    ///
    /// The primitive every drawn thing becomes. Order matters: quads submitted
    /// at the same [`Depth`](crate::Depth) draw in submission order, and that
    /// tie-break is a CONTRACT, not an accident of sorting (renderer.md §2).
    pub fn submit(&mut self, quad: Quad) {
        self.submissions.quads.push(quad);
    }

    /// How many quads have been submitted this frame.
    ///
    /// For engine-side draw systems that batch their own work; games have no
    /// reason to ask.
    #[must_use]
    pub fn submitted(&self) -> usize {
        self.submissions.quads.len()
    }
}

/// One frame's worth of submitted quads, in submission order.
///
/// Owned by the driver and reused across frames: a game that draws a thousand
/// sprites should not allocate a thousand-quad buffer sixty times a second.
#[derive(Debug)]
pub struct Submissions {
    quads: Vec<Quad>,
}

impl Submissions {
    /// An empty frame.
    ///
    /// DELIBERATE: no `Default` impl, despite `clippy::new_without_default`
    /// (see ADR-0012) — one way to do everything, and `new` is that way.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self { quads: Vec::new() }
    }

    /// Everything submitted this frame, in submission order.
    #[must_use]
    pub fn quads(&self) -> &[Quad] {
        &self.quads
    }

    /// Forget the frame, keeping the space it used.
    pub fn clear(&mut self) {
        self.quads.clear();
    }
}
