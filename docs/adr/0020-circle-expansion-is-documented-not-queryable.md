# ADR-0020: A circle's expansion is documented, not queryable

Status: accepted · 2026-08-18

## Context

`ctx.circle` submits sixteen quads — a fan of wedges, each the centre plus three
rim points, at a fixed thirty-two segments regardless of radius. That count
carries a `DELIBERATE:` tag at `shapes.rs`'s `CIRCLE_SEGMENTS` and the reason is
verification: a radius-dependent count would change a transcript and every golden
image when a circle grew by a pixel, and identical submissions producing identical
output is what the whole draw-verification story rests on (renderer.md §2, §9).

None of that was reachable from `docs/api/`. `Submit::circle`'s summary was
"Fill a circle."

Two E0 runs went looking, and the second cost is the one that argues this decision.

**Run 4** copied the only worked assertion of "was this thing drawn" —
`prototype_kit/verify.rs` looks for a quad *the size of the paddle* at the
paddle's position — and it fails for a ball, because nothing the size of the ball
is drawn anywhere:

> what covers the ball's centre is sixteen wedges of 0.450×0.172, 0.416×0.318,
> 0.318×0.416, 0.172×0.450 and so on. I only found out by making the assertion
> dump what it had actually found, which is a full debug cycle spent on an
> undocumented implementation detail of the one primitive a ball is made of.

**Run 3**, one run earlier, asked the same question and wrote down the wrong
answer — "the ball is one quad, exactly `2r × 2r`" — recorded it as resolved, and
was not corrected, because its check never depended on it. A document silent about
behaviour does not only cost time. It manufactures confident false findings.

The fix that closes the *lookup* is a sentence. The question this ADR answers is
whether the engine should also close the *assertion*, by giving `FrameRecord` a
way to ask "is a disc of this size drawn here" so a game stops hand-rolling one.
Run 4's hand-rolled version is `disc_drawn` in `pong/verify.rs`, and it is the
statement of what was wanted.

## Decision

**Document the expansion. Do not add a disc query.**

Three things reach `docs/api/` and one thing does not get built.

1. `Submit::circle`'s summary — the sixty-eight characters the reference prints —
   is now "Fill a circle, as a fan of sixteen quads rather than as one."
2. Concepts states the quad count of every verb together, with the budget
   consequence: one for `rect` and `line`, sixteen for `circle`, one per character
   for `text`. "A circle costs sixteen rectangles" is what makes a frame's quad
   count predictable, and run 4's ball was sixteen of its last frame's hundred and
   one quads.
3. *Testing your game* carries the worked disc assertion in full — union the
   bounds of the quads covering the centre that fit inside `2r × 2r`, check the
   union — generalised from `disc_drawn` and written out rather than cited,
   because a game's own files are deleted before the next E0 run starts (F-019).

The assertion is exact rather than approximate, and it is worth saying why it can
be, because that is what makes documentation sufficient here. Every wedge is
inscribed, so nothing a circle draws leaves `2r × 2r`. All sixteen share the
centre as a corner and `FrameRecord::covering` counts a point on an edge, so
asking about the centre returns all sixteen. The extreme rim points fall exactly
on the axes, so their union is exactly `2r × 2r`. The check is not "close enough
for a circle"; it is an identity.

## Consequences

- **A game that checks a circle writes ten lines it could have called.** That is
  the cost of this decision and it is not hidden: `pong/verify.rs`'s `disc_drawn`
  stays exactly as long as it was. What changed is that it is the documented
  idiom instead of something an author had to invent under a failing assertion.
- The sixteen is now a **published number**, so changing `CIRCLE_SEGMENTS`
  becomes a documentation change and a change to every game's disc assertion that
  hard-codes a count. None should — the documented form unions whatever it finds
  — but the number is in `docs/api/` and moving it needs a superseding ADR.
- `DrawnQuad::contains` now states that edges and corners count as inside, which
  the fan relies on and which run 4 listed as a thing it wanted to look up and
  could not.
- `Rect::contains` stays half-open and now says so beside `contains`, because the
  documented assertion deliberately does *not* use it: half-open would discard
  the one wedge that reaches the far edge, which is the bug a reader would write.

## Alternatives considered

**Give `FrameRecord` a `disc_covering(point, size) -> Option<Rect>` or a
`disc_drawn(at, radius) -> bool`.** The option run 4 asked for, and the one that
makes the game shorter. Rejected for v1 on three grounds, in order of weight.

*It is a second way to ask one question.* `covering(point)` plus `bounds()`
already answers "what is drawn here, exactly", and every other primitive is
checked with them. A disc query would be the recorder growing a special case for
one of five verbs — and then the argument for `text_drawn`, `line_drawn` and
`rect_drawn` is the same argument, which is how a general vocabulary becomes a
list.

*It puts the primitive's tessellation into the assertion vocabulary.* The whole
value of the fan being an implementation detail is that it can change. A
`disc_drawn` on `FrameRecord` is render-core promising that circles will keep
being unionable, which is a stronger promise than the `DELIBERATE:` tag makes and
one nothing currently needs.

*The sentence closes most of the cost and the ten lines close the rest.* Run 4's
loss was a debug cycle spent not knowing, and run 3's was a false belief. Both are
closed by three documented facts. Neither is closed by a helper, because an author
who does not know a circle is sixteen quads does not know to reach for a disc
helper either — they reach for the paddle assertion, which is exactly what
happened.

**Scale the segment count with radius, so a small ball is one or two quads.**
Rejected on the existing `DELIBERATE:` tag's reasoning, which this ADR does not
reopen: determinism of submissions is the foundation of the verification story,
and "the transcript changed because the ball grew" is a worse day than sixteen
quads.

**Say nothing and let the assertion teach it.** What the repository did until now,
and it produced one lost cycle and one recorded falsehood in two consecutive runs.
The failure is silent in the way that matters: the geometry is right, the check is
wrong, and nothing says so.
