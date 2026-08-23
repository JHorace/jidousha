# ADR-0041: a driver that draws once per tick reports `alpha == 1.0`

Status: accepted · 2026-08-23

## Context

`Time::alpha` has had a precise definition and no consumer since M3
(e0-findings.md **F-048**). The definition and the documented idiom are:

> keep last tick's value in a component of your own and submit
> `previous.lerp(current, alpha)` from the Draw system

The engine deliberately provides no lerp helper and no engine-side previous
transform — that would be retained render state, which renderer.md §2 rules out
— so the whole of the feature is the number plus that sentence. Which means the
sentence has to actually work.

It did not. Writing the idiom into `examples/pong` breaks the harness that
verifies pong.

`alpha` is written in exactly one place, `Simulation::advance`, as the
accumulator's leftover divided by `fixed_dt`. `Simulation::tick` — the whole of
what `HeadlessSim`, `tools/verify`, `FrameRecorder` and every doctest run —
never writes it, so it holds whatever it held: `0.0`, from `Time::new`.

With `alpha == 0.0`, `previous.lerp(current, 0.0)` is `previous`. A headless run
that adopts the documented idiom therefore draws the tick **before** the one it
just ran, and every check that compares a drawn quad against world state fails:

```
disc_bounds(frame, now.ball, BALL_RADIUS, font)   // pong/verify.rs
```

That is not pong's problem to solve. Every game that follows the documentation
meets it, on the first check it writes, and the workaround each of them would
write is the same line poked into `Time` from outside the engine. A documented
idiom that the engine's own testing thesis rejects is not a documented idiom.

## Decision

**`Time::advance` sets `alpha = 1.0`.** One tick leaves the clock standing
exactly on a tick boundary, and a driver that draws once per tick draws it
there.

- `Simulation::advance` still writes the accumulator's remainder after its ticks
  have run, so the **windowed** value is unchanged: `0.0..1.0`, exactly as
  before, and `alpha_reports_how_far_into_the_next_tick_the_frame_fell` passes
  untouched.
- A per-tick driver — `HeadlessSim::tick`, and so `tools/verify`, `FrameRecorder`
  and the capture tools — now reports `1.0`. `previous.lerp(current, 1.0)` is
  `current`, so a game that adopts interpolation draws, headless, exactly the
  quads it drew before it adopted it. `tools/verify pong` reports the same
  numbers across this change, which is the evidence.
- `DELIBERATE:` sits on `Time::advance`, which is where a reader asks why a
  clock step touches an interpolation fraction.

**What the field means is restated rather than changed.** It was "how far into
the next tick the last rendered frame fell, in `0.0..1.0`"; it is now "where this
frame falls between the previous tick and the current one". For the windowed
loop the two readings produce the same number — that is why nothing about the
windowed path moves — but only the second one has an endpoint at the tick just
run, and the interpolation idiom the field exists for is written between exactly
those two endpoints. The old wording described the accumulator; the new one
describes what a game does with it.

## Consequences

- The documented idiom works. `examples/pong` interpolates its paddles and its
  ball, `examples/prototype_kit` interpolates its paddle, and both verify
  unchanged. F-048's "the field has a user; the user is the game" is now a
  worked example rather than a claim.
- `alpha`'s range as a game may observe it is `0.0..=1.0`, and `1.0` names a
  specific, checkable situation rather than a rounding accident. The field's own
  documentation, core.md §7 and the Concepts paragraph all say which driver
  gives which, because "why is alpha 1.0 in my test and 0.4 in my window" is the
  next question after "what is alpha".
- **The determinism CONTRACT is untouched.** `alpha` is Draw-only by
  documentation and by the phase's read-only typing; no Update system may read
  it, so no simulation state depends on it, so nothing about replay identity can
  move. What changed is a value that only presentation reads.
- A game drawing at `alpha` draws one tick behind. That is the standard cost of
  interpolation and it is stated where the idiom is, not hidden: 16.7ms of
  latency bought for motion that does not judder on a display that is not an
  exact multiple of 60Hz — or in a browser that is not pacing frames evenly.
- **Declined: leave the engine alone and have each game's check compensate.**
  Every game would write the same poke into `Time` in its own verify harness, in
  order to make the engine's own documented idiom testable. The duplication
  would be forced by the surface rather than chosen, which is ADR-0039's test
  for an engine problem, and this one fails it the same way.
- **Declined: extrapolate instead —** `current + (current - previous) * alpha`,
  which needs no engine change at all because it is the identity at `alpha ==
  0.0`. It also overshoots every direction change by up to a tick of travel
  (0.73 world units for pong's ball at `MAX_SPEED`), visibly, on the one frame a
  player is most likely to be watching — the bounce. Interpolation's one tick of
  latency is the cheaper defect, and it is the one the documentation has
  described since M3.
