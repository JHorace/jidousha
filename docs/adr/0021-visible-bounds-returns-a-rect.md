# ADR-0021: `Camera::visible_bounds` returns a `Rect`

Status: accepted · 2026-08-18

## Context

```rust
pub fn visible_bounds(&self) -> (Vec2, Vec2);   // (top-left, bottom-right)
pub struct Rect { pub min: Vec2, pub max: Vec2 }  // min: top-left, max: bottom-right
```

Those are the same pair. `Rect`'s own documentation says so in the same words the
camera's does, and E0 run 4 noticed:

> `Rect { min, max }` is documented as "min: top-left, max: bottom-right", which
> is precisely the pair this returns. The consequence is visible in the document's
> own recommended assertion: six lines of hand-written `>=`/`<=` comparisons that
> would be one call on a `Rect`.

**Nothing settles the tuple deliberately, and this was checked rather than
assumed.** `camera.rs` carries a `DELIBERATE:` tag on `Camera::default` and none on
`visible_bounds`. No ADR mentions it. `renderer.md` does not raise it. There is no
crate-boundary reason: `Rect` is in core, the camera is downstream of core, and
`camera.rs` already imports from it. The signature is not a decision that was
made — it is one that was never revisited, and F-001's fix put it in front of a
reader for the first time.

**The cost lands in the check the document pushes hardest.** F-029 established
"nothing is drawn outside `Camera::visible_bounds()`" as the highest-value
assertion a shapes-and-text game can write, and `testing.md` spells it out in six
lines *because of this signature*. Run 2 wrote those six lines. Run 3 wrote them.
Run 4 wrote them twice — once per frame of the match, once per staged screen — and
factored them into a helper. Three consecutive runs have paid the same tax on the
one assertion the document most wants written.

## Decision

**Return `Rect`. Add `Rect::contains_rect`. Do not keep a tuple form.**

```rust
impl Camera {
    pub fn visible_bounds(&self) -> Rect;
}

impl Rect {
    /// Whether `other` is entirely inside, edges included.
    pub fn contains_rect(self, other: Rect) -> bool;
}
```

The off-screen assertion then reads:

```rust
let view = camera.visible_bounds();
for quad in frame.quads() {
    let bounds = quad.bounds();
    assert!(view.contains_rect(bounds), "drawn off screen: {bounds:?} against {view:?} \
        — text centred by width_of is the usual culprit");
}
```

`contains_rect` is the load-bearing half of the proposal and the reason this is
one ADR rather than two. Returning a `Rect` and leaving the comparison hand-written
saves one destructuring line and nothing else; the six lines are the *comparison*,
not the tuple. A change that does not shorten `testing.md`'s snippet has not
addressed the finding.

**Edges included, unlike `Rect::contains`.** A quad flush against the camera's edge
is on screen — that is what the assertion means — and `contains` is half-open so
that adjacent rectangles never both claim a point, which is a partition rule and
the wrong rule here. The two must not be spelled the same way; `contains_rect` is
a containment test between boxes and `contains` is a point-in-partition test, and
the doc comments have to say which is which or this ADR has traded one silent trap
for another. That is the sharpest objection to the proposal and it is met by naming
it, the way `DrawnQuad::contains` and `Rect::contains` are now distinguished.

## Consequences

- **A breaking change to a published signature**, taken in one pass with no
  deprecation, because ADR-0012's "one way to do everything" forbids shipping both
  forms. Every call site migrated: `pong/verify.rs`, `prototype_kit/main.rs`,
  `input_echo.rs`, the camera's own `world_to_screen` and `screen_to_world` (both
  of which destructured the tuple to throw half of it away), and `testing.md`'s
  snippet.
- **`testing.md`'s off-screen check went from six lines to three**, which was the
  test of whether the change addressed the finding rather than its symptom.
- **`pong/verify.rs`'s `assert_on_screen` lost its four-comparison body and its
  tuple parameter**, and is now one `contains_rect` call. The regression-target
  rule holds: the example got simpler, not longer.
- One more inherent method on `Rect`, which is a type that has grown twice on E0
  evidence already (`contains` and `overlaps` were invisible in run 1, F-003).
  This is the third.
- **The two containment rules are now a thing to keep straight**, which is the
  cost this decision accepts. `Rect::contains` is half-open and takes a point;
  `Rect::contains_rect` is closed and takes a rectangle. Both doc comments name
  the other and say why they differ, and
  `a_box_flush_against_the_edge_is_still_inside_the_box_around_it` is the test
  that pins the distinction — without it, the off-screen check would report a quad
  drawn hard against the camera's edge as drawn off screen.

## Alternatives considered

**Keep the tuple; add nothing.** The status quo, and it is defensible on exactly
one ground: a tuple has no opinion about whether its members are a rectangle, and
`visible_bounds` describes a *view* rather than a box in the world. That reading is
thin — the view is an axis-aligned box in world space, which is what `Rect` is —
and it costs six lines in the most-recommended assertion in the document, three
runs running. If this is the decision, it should be recorded here with the
reasoning, because the alternative is a fifth run rediscovering it.

**Return `Rect` and skip `contains_rect`.** Cheaper, and it addresses the wrong
half. See above.

**Add `contains_rect` and keep the tuple.** Composes badly — the caller builds a
`Rect` from the tuple in order to compare it against another `Rect` — and it puts
the seam in the middle of one operation.

**Give `Camera` an `is_visible(rect: Rect) -> bool` instead.** Tempting, because it
makes the assertion one call without touching `Rect`. Rejected: it puts a
containment test on the camera, so the next game that wants "is this quad inside
the field" has no way to ask, and `Camera` starts collecting geometry helpers that
belong on the geometry type. `visible_bounds` returning a `Rect` composes with
everything; `is_visible` composes with the camera.
