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

    /// Fill a circle, as a fan of sixteen quads rather than as one.
    ///
    /// **Sixteen, whatever the radius.** The count is fixed rather than scaled,
    /// so the same circle always produces the same vertices and a circle that
    /// grows by a pixel does not rewrite a transcript (renderer.md §2, §9).
    /// A circle therefore costs sixteen times what a rectangle costs; one ball
    /// in a Pong is comfortably the largest single item in that game's frame.
    ///
    /// Each quad is the centre and three points on the rim, so all sixteen
    /// share the centre as a corner and every one of them lies inside the
    /// circle's bounding box. Two consequences a check depends on: nothing a
    /// circle draws reaches outside `2r × 2r`, and the union of the quads
    /// covering the centre is exactly `2r × 2r`. That union is how a test asks
    /// "was a disc of this size drawn here" — *Testing your game* has it
    /// written out, because "a quad the size of the thing" is the answer for
    /// every other primitive and is the wrong answer for this one.
    ///
    /// DELIBERATE: that union is written out in the document and **not** offered
    /// as a `FrameRecord::disc_drawn` (see ADR-0020). Do not add one: it would be
    /// a second way to ask what `covering` plus `bounds` already answers, and it
    /// would promise that circles keep being unionable, which is a stronger
    /// promise than the fixed segment count makes.
    fn circle(&mut self, center: Vec2, radius: f32, color: Color, depth: Depth);

    /// Draw `text` with its first character's top-left corner at `at`.
    ///
    /// Monospace, from the engine's embedded font — no asset, and it works on
    /// the first frame of a program before anything has loaded. `\n` starts a
    /// new line; nothing wraps (renderer.md §6). Use
    /// [`TextStyle::width_of`] to center it.
    ///
    /// The depth goes in the [`TextStyle`], not after it.
    ///
    /// DELIBERATE: the one verb here that takes no trailing `Depth`, and it
    /// looks like a wobble in a five-verb API whose first rule is one way to do
    /// everything. Text needs a style object regardless — size and color have
    /// nowhere else to live — so the choice is one struct or a struct plus an
    /// argument, and depth travels with whatever else describes how the thing
    /// looks. See ADR-0018, which also says what this does *not* license.
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
