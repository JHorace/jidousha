# ADR-0022: Swept collision and `Rect::inflate` stay out of v1

Status: accepted · 2026-08-18

> **Accepted as recommended, which means the primitives are declined.** Three E0
> runs reached for a sweep and no document said whether it had ever been
> considered; that is what this ADR ends. Nothing was added to the API — what
> changed is that the absence is now a stated boundary with the shape to write
> instead, the same treatment `App::quit` gets.

## Context

Concepts already says the hard part, and says it well (F-034):

> A fixed timestep also means **collisions are only ever tested at tick
> boundaries**. Nothing in v1 sweeps, so a body that moves further in one tick
> than its target is thick steps clean through it […] That is the first thing that
> bites a game with a fast small ball, and the fix is the game's.

Then the entire collision vocabulary is `Rect::overlaps` and `Rect::contains`,
neither of which answers a ball against a paddle. **Three runs have reached for
the missing primitive**, none of them able to read the others:

- **Run 1**, having found no overlap test at all: "I ended up needing a *swept*
  test anyway, which no engine helper would have given me."
- **Run 3** §2.1: "the one thing I kept reaching for was a swept or continuous
  collision helper […] it is the single piece of vocabulary a Pong needs that
  shapes-and-text does not cover."
- **Run 4**: "There is no segment-versus-rect helper, no `Rect::sweep`, and no
  `Rect::inflate` either — expanding a paddle by the ball's radius is
  `PADDLE_SIZE.y * 0.5 + BALL_RADIUS` spelled out at three call sites."

That makes it the second-most-corroborated finding in `e0-findings.md` after the
controller trap. F-034 answered the *warning* and left the *primitive*, and the
third run through named the remaining gap directly.

**What the forty lines actually are.** Run 4 predicted that "every Pong written
against this engine will write that same forty lines", and it is worth reading
`advance` in `pong/main.rs` before believing a primitive would absorb them. About
eight lines are the geometry: the plane the ball's edge touches, whether the ball
is approaching, whether this tick's travel crossed it, and where along that travel.
The other thirty are the **response** — reflect by where on the paddle's face
contact landed, gain speed, cap it, advance the remainder of the tick from the
contact point, then resolve the walls so a corner hit still ends on the field.

A `Rect::sweep` absorbs the eight. It cannot absorb the thirty, because the
response is the game's model of Pong and no engine can own it. So the honest number
is about a fifth of the function, and the argument has to be made on that number
rather than on the sentence.

`Rect::inflate` is smaller again. Of run 4's three sites, two are the paddle (`x`
and `y`) and the third pair are the field walls, which are a different rectangle
inflated the other way. Inflate replaces two scalar expressions with a `Rect` the
call sites then destructure — roughly break-even in a game with one collider shape,
and clearly positive in a game with several.

## Decision

**Decline both for v1, and say so in the document.**

Concretely:

1. **No `Rect::sweep`, no segment-versus-rect helper, no `Rect::inflate` in v1.**
2. Concepts' fixed-timestep paragraph gains one sentence saying the absence is a
   scope decision rather than an oversight, pointing here — the same treatment
   F-027 gave `App::quit`, which run 4 singled out as "the right way to document
   an absence" and which cost it nothing.
3. *Testing your game* keeps F-034's advice unchanged, because run 4 confirmed it
   works: assert the tunnelling margin against the `fixed_dt` the engine hands the
   game, which catches a raised timestep that no primitive would.

The case for declining, in order of weight:

**A sweep primitive that stops at contact is a physics API wearing a geometry
hat.** The eight lines it replaces are the easy eight. The moment the engine
answers "where did they touch", the next question is "and what happens now" — a
normal, a restitution, an order in which to resolve two colliders — and that is a
collision subsystem, which ADR-0001 scopes out of v1. A primitive that answers the
first question and refuses the second is the shape that invites the second, and
"we shipped half a physics engine" is worse than shipping none.

**The response is where the bugs are, and it stays the game's either way.** Run 1's
worst bug was a bounce plane 1.5 units behind the opponent's paddle; run 4's was a
sign error in the reflection. Neither is in the eight lines. A sweep helper would
have shipped both games with the same bugs and a shorter function, which is not the
improvement the finding implies.

**`Rect::inflate` alone does not carry its weight, and adding it alone is worse
than adding neither.** Two expressions in one game, and it is the kind of method
that reads as a companion to a sweep API that does not exist — a reader who finds
`inflate` and no sweep concludes the sweep is somewhere and keeps looking. If the
sweep is declined, `inflate` should be declined with it, so the absence is one
coherent boundary instead of a partial one.

**The measured cost of declining is a documented sentence.** F-027 is the control:
an absence named as a boundary cost run 4 nothing, and an absence *not* named
(sound, F-052) was felt as a loss. The complaint here is not "I could not write
it" — all three runs wrote it, first try, correctly except for their own arithmetic.
It is "I could not tell whether I was supposed to".

## Consequences

- Concepts gains a paragraph and `docs/api/` is regenerated. **No API changed and
  no example changed** — `pong/verify.rs` and `advance` in `pong/main.rs` stay
  exactly as run 4 wrote them, which is the one case where the E0 rule that "the
  game's workaround should get simpler" does not apply, because the decision is
  that the workaround is the game's job.
- The paragraph does more than name the absence: it gives the eight-line shape
  (the plane the leading edge touches, whether the body was approaching, whether
  this tick's travel crossed it, the fraction of the tick at which it did) and says
  why the thirty lines after it are the game's. A boundary that says only "not in
  v1" leaves the reader where run 4 was.
- Every future Pong-shaped game writes those eight lines. That is the accepted cost
  and it is now a number rather than an impression.
- **The `make-game` skill inherits this.** Per `e0-findings.md` §7 a friction that
  cannot be designed away is exactly what a skill is for, and a declined primitive
  is designed-away-in-the-other-direction: the skill should carry the eight-line
  shape, and it is the clearest candidate in run 4's set after the controller trap.

## What adding them would have looked like

*Kept because the next person to reach for this deserves the design rather than
just the refusal, and because superseding this ADR should start from a shape
rather than from scratch.*

- `Rect::sweep(self, motion: Vec2, against: Rect) -> Option<f32>` — the fraction of
  `motion` at which `self` first touches `against`, or `None`. Half-open at the
  start so a body already touching does not re-report, matching `overlaps`.
- `Rect::inflate(self, by: Vec2) -> Rect`, and it must be `Vec2` rather than `f32`:
  "expanded by the ball's radius" is isotropic in Pong and is not in general, and a
  scalar version would be the second way to do it the moment someone needs an
  anisotropic one.
- Both need examples for `tools/check-api-coverage`, and the sweep needs a test
  that a body exactly touching at t=0 reports `None`, which is the case run 2 wrote
  a defensive guard for without knowing the answer (F-024).
- Concepts' tunnelling paragraph has to be rewritten rather than extended, because
  "nothing in v1 sweeps" becomes false and it is currently the sentence that
  teaches the whole problem.

## Alternatives considered

**Add the sweep, decline `inflate`.** The sweep is the load-bearing half, so this
is the coherent version of accepting. Its problem is the physics-API slope above.

**Add `inflate`, decline the sweep.** Cheapest by far and the worst of the three:
it adds the piece that saves two lines and withholds the piece that was actually
missing, and it leaves a reader hunting for the companion method.

**Ship a worked helper in an example rather than in the API.** Rejected for the
reason F-037 declined its worked controller: `crates/jidousha/examples/` is on E0's
allowed list, so an example containing the sweep is an example containing the answer
to the exercise, and the next run finds it first (F-020). If the eight-line shape is
to be written down for game authors, `testing.md` prose or the `make-game` skill are
the places that do not contaminate the measurement.

**Leave it undecided and let a fifth run say whether it costs anything.** Rejected
on ADR-0018's reasoning, which applies verbatim here and with three times the
evidence: a finding reported and then not answered teaches its readers that
reporting is pointless. Three runs have reported this. The answer may be no; it may
not be silence.
