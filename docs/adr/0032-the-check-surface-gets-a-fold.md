# ADR-0032: The check surface gets `find_bounds`; the game surface does not get `Rect::union`

Status: accepted · 2026-08-20 · **the other side of ADR-0022's line**

> **`jidousha::testing::find_bounds(quads) -> Option<Rect>` is added.** It is the
> fold five hand-written copies of `min.min(min), max.max(max)` were doing across
> three worked examples and the testing document. `Rect::union` in the game's
> geometry vocabulary is **declined** — that is ADR-0022's boundary and this is
> not on that side of it.

## Context

"How big is the thing that was drawn" has no single-quad answer for most of the
drawing vocabulary. `ctx.circle` submits sixteen wedges (ADR-0020) and `ctx.text`
one quad per character, so the disc and the string are each a fold over
`DrawnQuad::bounds()` and neither is a quad anybody drew. Every check that
measures a drawn thing has therefore written the same three lines. E0 run 9
counted them:

> The testing document writes it out inline; `prototype_kit/verify.rs` writes it
> twice; I wrote a `union()` helper and called it three times (the ball's disc,
> the score's two halves, the drawn court).

Five copies, plus `examples/slalom/checks.rs` — which run 9 could not see —
makes six. The document's own circle recipe is twenty lines and about half of
them are the fold.

**Run 9 also drew the line this decision needs**, from outside the source:

> This is not a v1 boundary in the sense `Rect::sweep` is — there is no game
> model hiding behind it, it is `min.min(min), max.max(max)`.

That is the right test and it is worth stating why. ADR-0022 declined `sweep` and
`inflate` because a sweep answers an eighth of the question a game asks and the
other seven eighths — the bounce, the speed change, the resolution order — are
the game's own model, which no engine primitive can own. A fold over recorded
quads has no second seven eighths. It is total, it has one obvious
implementation, and the only judgement in it is what `None` means.

## Decision

**Add `find_bounds(quads: impl IntoIterator<Item = DrawnQuad>) -> Option<Rect>`
to `jidousha::testing`.** Defined in `jidousha-render-core` beside `DrawnQuad`.

- **In the check surface, not the game's.** The question is asked about a
  recorded frame, and `DrawnQuad` is a testing-only type. Putting the fold where
  the quads are keeps the game's geometry vocabulary closed, which is what
  ADR-0022 bought.
- **It takes quads, not rectangles**, because that is the shape the question
  arrives in: `quads()` and `covering()` both hand back `Vec<DrawnQuad>`, so a
  filtered iterator over either goes straight in and nothing has to `.map` first.
- **`find_`, because it returns `Option`** (conventions §Naming). `None` is
  "nothing was drawn there", which is a real answer a check reports rather than
  an error — it is what the circle assertion says when the ball is missing.
- **No `Rect::union`.** A game that wants the box around two rectangles writes
  the two `min`/`max` lines, as it does today. Adding it would be a second way to
  ask a question `find_bounds` already answers for the case that actually recurs,
  and would open the general-geometry door ADR-0022 closed.

## Rationale

The count is what carries this. One duplicated fold is a coincidence; six across
three examples and a document is the surface being one call short, and it is the
same evidence — reached for by independent runs who could not see each other —
that ADR-0021 accepted `visible_bounds -> Rect` on and ADR-0022 declined the
sweep on. The difference between the two is not how often it was wanted but
whether the engine can own the whole answer.

The document is the other beneficiary. The circle recipe was twenty lines of
which the fold was ten, in a document whose budget has been the binding
constraint for three triages (ADR-0030). Shortening a worked example by half
without losing anything it taught is the cheapest tokens this file has bought.

## Consequences

- `docs/api/jidousha-testing.md`'s circle recipe drops to the filter and the
  call, and the entry for `find_bounds` carries the "sixteen wedges, one quad per
  character" reason so the *why* is not lost with the ten lines.
- `examples/prototype_kit/verify.rs` (twice), `examples/slalom/checks.rs` and
  `examples/pong/verify.rs` lose their hand-written folds. That is a consequence
  of this decision reaching the examples, not a change to what any of them
  checks; run 9's Pong keeps every constant and every assertion it shipped.
- `check-api-coverage` skips `jidousha::testing`, so this needs no coverage
  entry — but the testing document shows it in the recipe, which is the same
  guarantee by another route.
- The next fold-shaped request has a precedent to be argued against: it is in if
  the engine can answer the whole question, out if the second half is the game's.

## Alternatives rejected

- **`Rect::union(self, other) -> Rect` in the game surface**, with the check
  written as `.map(bounds).reduce(Rect::union)`. Shorter to implement and one
  character longer to call. Rejected because it puts a general geometry
  primitive in a vocabulary ADR-0022 deliberately closed, answers the recurring
  question only after a `map`, and leaves `Option` handling at every call site
  anyway.
- **A method on `FrameRecord`** — `frame.bounds_of(quads)`. The quads may be
  filtered from `covering()` or from another frame entirely, so the receiver adds
  nothing and implies a relationship that does not hold.
- **Leave it out and keep documenting the fold.** What run 9 actually proposed
  was measured against the document it had; from inside, the count is six and the
  fold has no game model behind it. Declining would be declining on ADR-0022's
  authority for a case ADR-0022's reasoning does not reach.
