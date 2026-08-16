//! What a Draw system calls: `ctx.sprite(...)`.
//!
//! Key types: `Submit`, `draw_sprites`.
//! Depends on: `jidousha-core`, `sprite`.
//! INVARIANT: every method here expands to [`DrawCtx::submit`] and nothing
//! else. One mechanism for everything drawn — there is no "debug renderer"
//! mode, no second path, and no retained state (renderer.md §2).

use jidousha_core::{DrawCtx, Transform};

use crate::sprite::Sprite;

/// The drawing verbs, added to [`DrawCtx`].
///
/// An extension trait rather than inherent methods, because `DrawCtx` lives in
/// `jidousha-core` and a sprite names a texture asset, which core cannot see
/// (ADR-0015). Games get it from the prelude and never think about it.
///
/// ```
/// # use jidousha_render_core::{Sprite, Submit};
/// # use jidousha_core::{DrawCtx, Transform};
/// fn draw_the_ship(ctx: &mut DrawCtx, transform: &Transform, sprite: &Sprite) {
///     ctx.sprite(transform, sprite);
/// }
/// ```
pub trait Submit {
    /// Draw `sprite` at `transform`.
    ///
    /// The workhorse. It takes the same component types entities carry, so the
    /// canonical query-and-submit loop is a one-liner.
    fn sprite(&mut self, transform: &Transform, sprite: &Sprite);
}

impl Submit for DrawCtx<'_> {
    fn sprite(&mut self, transform: &Transform, sprite: &Sprite) {
        self.submit(sprite.quad(transform));
    }
}

/// The Draw system almost every game wants: draw every sprite there is.
///
/// Registered explicitly, like any other system:
///
/// ```
/// # use jidousha_core::{Draw, GameConfig, headless};
/// # use jidousha_render_core::draw_sprites;
/// let mut sim = headless(GameConfig::default(), |app| {
///     app.add_system(Draw, draw_sprites);
/// });
/// ```
///
/// DELIBERATE: explicit registration rather than an automatic default. A
/// schedule you can print is a schedule you can debug (core.md §7), and a
/// system that runs without appearing in the listing is exactly the kind of
/// invisible machinery this engine does not have. "A provided system you
/// register" is still one mechanism, not a second path.
pub fn draw_sprites(ctx: &mut DrawCtx) {
    // Collected first because the iterator borrows the world view, and
    // submitting borrows the context — the read-pass/write-pass shape again
    // (core.md §5, ADR-0013).
    let sprites: Vec<(Transform, Sprite)> = ctx
        .world
        .query::<(&Transform, &Sprite)>()
        .map(|(_, transform, sprite)| (*transform, *sprite))
        .collect();
    for (transform, sprite) in sprites {
        ctx.sprite(&transform, &sprite);
    }
}
