# ADR-0024: A recorded frame shows draw *order*, not the depth that produced it

Status: accepted · 2026-08-18

> **Accepted as recommended, which means `DrawnQuad` does not gain a depth.**
> Nothing was added to the API. What changed is that the ordering vocabulary a
> frame already has is now stated, because E0 run 5 concluded from its silence
> that draw order was unassertable — and it is not.

## Context

`Depth { layer, z }` is prominent in `docs/api/jidousha-api.md`: order comes
from depth rather than from submission order, `layer`'s numbers are the game's
own, name the bands once in a `mod layers`. E0 run 5 did all of that, put its
score on a `TABLE` band so the ball passes in front of it, and then found it
could not check any of it:

> `DrawnQuad` is `{ batch, texture, corners, tint }` […] so a verification can
> ask where a quad is, what colour it is and what texture it sampled, but not
> what band it was sorted into. Swap `layers::TABLE` for `layers::UI` on the
> score and the picture changes — the score paints over the ball — and every
> assertion in this game still passes.

The run's request, stated as the one thing it would add to the engine: a layer
or depth on `DrawnQuad`, "so that draw order is something a verification can see
at all".

**The premise is wrong, and that is the finding.** Draw order is exactly what a
recorded frame does show. `plan_frame` sorts by `(layer, z, submission index)`
and the sorted sequence *is* the frame:

- `FrameRecord::quads()` returns every quad "in draw order" — the sorted order,
  not the order the game submitted in. A quad's index in that `Vec` is its place
  in the painter's sequence.
- `FrameRecord::covering(point)` returns the quads at a point "front to back —
  the last one drawn first", so `covering(p)[0]` is what a player looking at `p`
  actually sees.

So the run's own example is a three-line assertion. At any point where the score
and the ball overlap, `covering(p)[0].texture == recorder.font_texture()` is true
exactly when the score is painting over the ball, which is the bug it wanted to
catch. Where nothing overlaps, comparing the two quads' indices in `quads()`
answers the same question without needing an overlap at all.

The run could not have known this — it reads a filename-free reference, and
neither method's entry connects "draw order" to `Depth`. Two documents away,
Concepts says order comes from depth; here, "in draw order" reads as "the order
they came out", which is precisely what it is not.

## Decision

**Do not put `Depth` on `DrawnQuad`. State the ordering vocabulary instead.**

Three things reach `docs/api/`, and one thing does not get built.

1. `FrameRecord::quads()` now says the order is the depth sort — layer, then
   `z`, then submission order — so a quad's index is its place in the painter's
   sequence and two quads' relative order is a comparison of indices.
2. `FrameRecord::covering()` now says the front-to-back order is that same sort
   read backwards, so the first element is what the player sees.
3. *Testing your game* carries the worked "is this behind that?" assertion, in
   both spellings, next to the checks it belongs with.

### Why not the depth

*A layer number read back is a tautology.* Concepts is emphatic that `layer`'s
numbers are the game's — the engine sorts by them and has no opinion about what
they mean. So an assertion that the score's quad carries `layers::TABLE` asserts
that the game submitted what the game submitted; it passes for a `mod layers`
whose constants are in the wrong order, which is the actual bug. The question
worth asking is *what ends up in front*, and that is a question about order.
Handing back the input to the sort instead of a reading of its output is the one
shape guaranteed not to catch the mistake it is asked to catch.

*It is a second way to ask one question.* `quads()`'s order and `covering()`'s
front-to-back already answer relative ordering exactly. A depth field would be a
second, and the weaker of the two — the same objection ADR-0020 raised against
`disc_drawn`, and it applies harder here, because there the second way was at
least equivalent.

*The plan has spent the depth by then, on purpose.* A `FramePlan` is
`{ clear_color, view_projection, batches }` and a `Batch` is a texture and a
`Vec<QuadVertex>` — what the GPU is handed. Depth is consumed by the sort and
does not survive into a vertex, because nothing downstream needs it: the
painter's algorithm is the sort, and there is no depth buffer. Restoring it
means either a field on every vertex of every frame that only a test ever reads,
or a parallel per-quad array in `Batch` that the real backends must carry and
ignore. Both put test-only payload in the hot path, across the seam two backends
have to agree about (renderer.md §1, §7).

*A run that hits the limit is not the same as a run that hits the wall.* This one
worked around the absence by not asserting on ordering — which was the right call
given what it knew, and cost it nothing but the assertion it wanted. There is no
lost cycle here to price the change against, and the sentence closes the whole
cost.

## Consequences

- **The run's stated request is refused and its underlying want is met.** A game
  can assert that its score is behind its ball, today, with no new API — and the
  document now says how, in the place the question is asked.
- **The sort's tie-break becomes something games depend on.** It was already a
  CONTRACT for transcript reproducibility (renderer.md §2); it is now also the
  thing an ordering assertion reads, so `plan_frame`'s stable sort by
  `(layer, z, index)` is a published guarantee rather than an internal one.
  `a_frames_draw_order_is_the_depth_sort_not_the_submission_order` is the guard.
- **`layer` stays unreadable from a frame, and that is the point.** A game that
  genuinely wants to assert on its own band numbering asserts on its own `mod
  layers` constants, which is where that information lives and is not the
  engine's to hand back.
- No signature changes; `tools/gen-api-doc` rerun for the two summaries.

## Alternatives considered

**Add `pub depth: Depth` to `DrawnQuad`.** What run 5 asked for. Rejected on all
four grounds above; the tautology argument is the one that would still hold even
if the plumbing were free.

**Add `FrameRecord::in_front_of(a, b) -> bool`.** Closes the assertion without
exposing depth, and reads well. Rejected as the same special-casing ADR-0020
declined for `disc_drawn`: `quads()` plus an index comparison is the general
vocabulary, and a helper for one of the questions it answers invites helpers for
the rest. It is also a worse fit than it looks — the caller still has to identify
which two quads, which is the whole difficulty.

**Keep the depth on the plan but not on the vertices, as a per-quad side array in
`Batch`.** The cheapest plumbing, and still rejected: `Batch` is what crosses the
backend seam, and a field there is a field `jidousha-render-wgpu` and every
future `ash` backend must carry, serialize and ignore. Verification payload
belongs on the recording side of the seam or nowhere.

**Say nothing, on the grounds that the run worked around it.** What the
repository did until now, and it produced a run that concluded in writing that
the engine cannot see draw order and filed it as the one thing it would change.
A silence that manufactures a confident false finding is the failure mode
ADR-0020 was written about; this is its second instance.
