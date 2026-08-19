# ADR-0026: `prototype_kit`'s verify drives its own backend; `FrameRecorder` is still the one way

Status: **superseded by ADR-0028** · 2026-08-19

> ADR-0028 moves the two-backend comparison into
> `crates/jidousha/tests/backend_agnostic.rs` and puts `prototype_kit` back on
> `FrameRecorder`. The reasoning below is kept because it is why the divergence
> survived as long as it did — and the thing it did not ask is where a claim
> about the engine belongs.


> **The divergence is kept and made unmissable, rather than removed or split.**
> Nothing in the API changed. What changed is where the explanation lives: at
> the top of the file a reader copies from, instead of two hundred lines down in
> a private helper's doc comment.

## Context

`docs/api/jidousha-testing.md` prescribes one way for a headless game to get a
frame: `FrameRecorder::new(viewport)`, then `recorder.draw(&mut sim)` once a
tick, which hands back the `FrameRecord` every assertion reads.
`recorder.font_texture()` answers which texture the font landed on, and
`frame.plan` is what a capture path replays to make a PNG.

`crates/jidousha/examples/prototype_kit/verify.rs` does something else. Its
`play` takes a `&mut dyn RenderBackend` and, per tick, calls `sim.draw()`, builds
its own `TextureTable` with `create_builtin_textures`, calls `plan_frame` and
then `backend.render`. Because the table is gone by the time its assertions run,
it reconstructs the font's backend id by building a second table against a
throwaway `NullBackend`.

E0 run 6 found both, could not tell which was advice and which was an artefact,
and lost time to it:

> `prototype_kit` explains *why* it keeps the long way […] and even writes out
> the short way in a doc comment. That is honest and I still lost time: the
> example is the thing you read to learn the shape, and the shape it teaches has
> fifteen lines of ceremony that the document says a game does not need.

The complaint is real and it is against `docs/api/jidousha-api.md`'s own opening
convention, "One way to do everything". Getting a frame out of a headless game
had two, and the canonical example used the one the document does not recommend.

## Decision

1. **`FrameRecorder` is the one way a game gets a frame.** Unchanged. Nothing is
   added, deprecated or renamed.
2. **`prototype_kit/verify.rs` keeps driving the backend by hand**, because it is
   doing a second job that no game has: `play` runs the identical session through
   a `NullBackend` *and* through a real `WgpuBackend`, and asserts the world did
   the same thing both times. That comparison is what makes "a session is
   backend-agnostic" a checked claim rather than a design intention, and
   `FrameRecorder` records into a null backend only, so it cannot buy it.
3. **The divergence is named at the top of that file**, under its own heading,
   before any code: what a game does, what this file does instead, which lines
   are the difference, why they are here, and the instruction to read the file
   for its *checks* and not for how to get a frame.
4. **The testing document says the same thing from its side**, in one sentence,
   without naming a file.

## Rationale

The two alternatives were considered and both cost more than they buy.

- **Make the example use `FrameRecorder`.** This deletes the two-backend
  comparison, which is the only check in the repository that a session produces
  the same world through a real GPU as through a null backend. Trading a real
  engine guarantee for a documentation tidy is the wrong direction.
- **Split it into two examples.** One would be a game with a recorder-shaped
  check, the other the same game again with a backend-shaped one. That is a whole
  duplicated game to say one thing twice, and two files that must not drift.

What was actually broken was neither of those. The reasoning existed and was
correct; it was two hundred lines below the code that raises the question, in the
doc comment of a private function, which a reader meets *after* having copied the
shape. A `DELIBERATE:` tag is only a defense where the surprise is
(agent-practices §1), and the surprise here is the file's whole structure, so the
tag belongs at the file's top.

## Consequences

- `prototype_kit/verify.rs` opens with a "One thing here is not the shape to
  copy" section. It states the recorder's two calls positively, so a reader who
  starts there leaves with the right shape even if they read nothing else.
- The existing `DELIBERATE:` on `textures_font_id` — which writes the short way
  out longhand rather than citing another example's file — is settled by this ADR
  too, and for a reason worth keeping: the file it used to cite was an E0 run's
  game, and those are deleted before the next run (`e0-prompt.md` step 2). No
  permanent document may point at one.
- The same rule binds the testing document, which is why point 4 names no file.
- A future example that needs a real backend follows this shape: keep the long
  way, and say at the top that it is not the shape.

## Alternatives rejected

- **Deprecate the hand-driven path.** It is `jidousha::testing`'s public
  vocabulary and the golden-image tier is written against it. Not a candidate.
- **Say nothing and let the doc comment carry it.** That is the status quo E0
  run 6 measured the cost of.
