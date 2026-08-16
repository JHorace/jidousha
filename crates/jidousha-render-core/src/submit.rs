//! What a Draw system calls: `ctx.sprite(...)`, `ctx.rect(...)`, `ctx.text(...)`.
//!
//! Key types: `Submit`, `draw_sprites`.
//! Depends on: `jidousha-core`, `sprite`, `shapes`, `font`.
//! INVARIANT: every method here expands to [`DrawCtx::submit`] and nothing
//! else. One mechanism for everything drawn — there is no "debug renderer"
//! mode, no second path, and no retained state (renderer.md §2). A rectangle
//! and a sprite at the same depth interleave in submission order, because by
//! the time anything is sorted they are the same kind of thing.

use jidousha_core::math::Vec2;
use jidousha_core::{Color, Depth, DrawCtx, Rect, Transform};

use crate::font::{TextStyle, glyph_quad, layout};
use crate::shapes::{circle_quads, line_quad, rect_quad};
use crate::sprite::Sprite;

/// The drawing verbs, added to [`DrawCtx`].
///
/// An extension trait rather than inherent methods, because `DrawCtx` lives in
/// `jidousha-core` and a sprite names a texture asset, which core cannot see
/// (ADR-0015). Games get it from the prelude and never think about it.
///
/// ```
/// # use jidousha_render_core::{Sprite, Submit, TextStyle};
/// # use jidousha_core::{Color, Depth, DrawCtx, Transform, math::Vec2};
/// fn draw_the_game(ctx: &mut DrawCtx, transform: &Transform, sprite: &Sprite) {
///     ctx.sprite(transform, sprite);
///     ctx.circle(Vec2::ZERO, 0.5, Color::WHITE, Depth::layer(1));
///     ctx.text(Vec2::new(-8.0, -4.0), "score 12", TextStyle::default());
/// }
/// ```
pub trait Submit {
    /// Draw `sprite` at `transform`.
    ///
    /// The workhorse. It takes the same component types entities carry, so the
    /// canonical query-and-submit loop is a one-liner.
    fn sprite(&mut self, transform: &Transform, sprite: &Sprite);

    /// Fill an axis-aligned rectangle.
    ///
    /// Y is down, so `rect.min` is its top-left corner (ADR-0010).
    fn rect(&mut self, rect: Rect, color: Color, depth: Depth);

    /// Draw a line from `from` to `to`, `thickness` world units wide.
    ///
    /// The thickness is in world units like everything else, so a line keeps
    /// its weight relative to the scene when the camera zooms — which is what a
    /// hitbox outline wants, and what a UI divider does not. v1 has one kind of
    /// line; a screen-space one is a separate future thing, not a flag on this.
    fn line(&mut self, from: Vec2, to: Vec2, thickness: f32, color: Color, depth: Depth);

    /// Fill a circle.
    ///
    /// Made of a fixed number of straight edges, so the same circle always
    /// produces the same vertices (renderer.md §2).
    fn circle(&mut self, center: Vec2, radius: f32, color: Color, depth: Depth);

    /// Draw `text` with its first character's top-left corner at `at`.
    ///
    /// Monospace, from the engine's embedded font — no asset, and it works on
    /// the first frame of a program before anything has loaded. `\n` starts a
    /// new line; nothing wraps (renderer.md §6). Use
    /// [`TextStyle::width_of`] to center it.
    fn text(&mut self, at: Vec2, text: &str, style: TextStyle);
}

impl Submit for DrawCtx<'_> {
    fn sprite(&mut self, transform: &Transform, sprite: &Sprite) {
        self.submit(sprite.quad(transform));
    }

    fn rect(&mut self, rect: Rect, color: Color, depth: Depth) {
        self.submit(rect_quad(rect, color, depth));
    }

    fn line(&mut self, from: Vec2, to: Vec2, thickness: f32, color: Color, depth: Depth) {
        self.submit(line_quad(from, to, thickness, color, depth));
    }

    fn circle(&mut self, center: Vec2, radius: f32, color: Color, depth: Depth) {
        // The only verb that expands to more than one quad, which is why it is
        // the only one that needs somewhere to put them.
        let mut quads = Vec::new();
        circle_quads(center, radius, color, depth, &mut quads);
        for quad in quads {
            self.submit(quad);
        }
    }

    fn text(&mut self, at: Vec2, text: &str, style: TextStyle) {
        for glyph in layout(at, text, &style) {
            self.submit(glyph_quad(&glyph, &style));
        }
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
