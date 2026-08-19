# ADR-0028: The two-backend comparison moves into a test; `prototype_kit` uses `FrameRecorder`

Status: accepted · 2026-08-19 · **supersedes ADR-0026**

> **The divergence ADR-0026 kept is removed, and the claim it was protecting is
> kept — in a test.** `crates/jidousha/tests/backend_agnostic.rs` now asserts
> that one session leaves the same world through a real GPU as through a null
> backend; `examples/prototype_kit/verify.rs` gets its frames the way the
> document says every game should. Six items leave `jidousha::testing`.

## Context

ADR-0026 faced a real conflict and resolved it the conservative way. E0 run 6
reported that the canonical example taught a frame-getting shape the document
tells games not to use, and the answer was: keep the divergence, because the
hand-driven path buys something no other check in the repository does —

> `play` runs the identical session through a `NullBackend` *and* through a real
> `WgpuBackend`, and asserts the world did the same thing both times. That
> comparison is what makes "a session is backend-agnostic" a checked claim
> rather than a design intention.

That reasoning holds. What ADR-0026 did not ask is **where such a claim
belongs**, and its own words answer it: the comparison is "a claim about the
*engine*, and a game has no reason to make it". A claim about the engine belongs
in `crates/*/tests/`. It was in an example — the one every E0 run reads to learn
the shape of a `--verify` mode — because that is where the machinery already
was, not because an example is where it should live.

Two things since have made the cost visible rather than arguable.

**The example's shape kept propagating.** E0 run 7 copied `PhysicalSize` out of
`prototype_kit`'s `jidousha::testing` import list, against a rule the document
states explicitly (F-088) — the third time a worked example has beaten a stated
rule. The header ADR-0026 added works for the *one* question it names and does
nothing for everything else a reader lifts from a file they are copying.

**The exports were never a game's.** The hand-driven path put `NullBackend`,
`plan_frame`, `compare`, `Comparison`, `Tolerance` and `diff_image` in
`jidousha::testing` — 767 tokens of the testing document's reference. *Testing
your game* has never mentioned one of the six, and `check-api-coverage` skips
`testing`, so they were exported, unexplained and uncovered for four milestones.
The document is at 97% of the budget ADR-0025 set (public-api.md §4), and a fifth
of its reference was vocabulary for a road no game is meant to walk.

## Decision

**End the divergence. Move the claim, not delete it.**

1. **`crates/jidousha/tests/backend_agnostic.rs`** runs one scripted session with
   art through a `NullBackend` and through a real `WgpuBackend` and asserts the
   worlds are identical — positions compared as float **bits**, asset status, and
   the quad count of the last frame. No adapter on the machine skips the GPU half
   loudly and keeps the run green, the same rule the golden tier follows.
2. **`prototype_kit/verify.rs` uses `FrameRecorder`**: `new`, `settle_assets`,
   `draw`, `font_texture`. The "one thing here is not the shape to copy" section
   is gone because there is no longer anything in the file that is not the shape.
3. **`prototype_kit/capture.rs` replays `frame.plan`** rather than replaying the
   session. Because this game loads art it also demonstrates the half the short
   path does not cover on its own: create the built-ins, upload the same art, and
   **check the ids agree** before trusting the picture.
4. **Six items leave `jidousha::testing`**: `NullBackend`, `plan_frame`,
   `compare`, `Comparison`, `Tolerance`, `diff_image`. They remain public in
   `jidousha-render-core`, where the engine's own golden tier uses them.
   `create_builtin_textures`, `upload_ready_textures` and `TextureTable` **stay**,
   because a capture that replays a plan naming art has to reproduce the id
   assignment, and point 3 is the worked case.

## Rationale

*A test is a better home for the claim than an example, on every axis that
matters here.* It runs on every `cargo test` rather than only when someone runs
one example's `--verify`; it fails with the two worlds printed rather than inside
a `--verify` summary; it can be minimal — twelve ticks and one texture — instead
of riding on a 130-tick game that exists for other reasons; and it is not read by
anyone learning to write a game.

*The claim got stronger in the move.* `prototype_kit` compared one `Vec<f32>` of
paddle positions. The test compares every entity's position as bits, the asset
status, and the last frame's quad count, and it carries a second test asserting
that the art really did resolve — because two backends agreeing about a session
in which nothing was uploaded would be checking that both can do nothing. The
upload path is the *only* route by which a backend can reach a world
(`upload_ready_textures` takes `Assets` mutably), so exercising it is the whole
point, and the old comparison did not check that it had been.

*ADR-0026's rejected alternative is not what happened here.* It considered
"make the example use `FrameRecorder`" and rejected it because that **deletes**
the comparison. It does not delete it if the comparison moves first. That option
was not on ADR-0026's list, and it is the one that costs nothing.

## Consequences

- **`prototype_kit` is now the shape it teaches**, so a reader who copies the
  whole file copies the right thing rather than the right thing plus a caveat.
- **The testing reference drops ~767 tokens**, from ~5,283 to ~4,516, taking the
  document from 98% of its budget to 91%. That is one E0 run's worth of headroom
  recovered structurally — by removing items rather than by compressing prose.
- **`textures_font_id` is gone**, and with it the `DELIBERATE:` tag ADR-0026
  settled: `recorder.font_texture()` answers the question directly, so there is
  no longer a longhand shape to justify.
- **The placeholder check got stronger on the way through.** Reading
  `TextureTable::placeholder()` was not available through the recorder, so the
  check now reads the *sprite's texture per frame* and asserts the tick it
  changes on. That is a stricter claim than "some frames drew the placeholder" —
  it names when — and it is read off the frames rather than off `Assets::status`.
- **A future example that needs a real backend has no special dispensation.** If
  it is making a claim about the engine, the claim goes in a test.
- ADR-0026 is superseded rather than edited (CLAUDE.md).

## Alternatives considered

**Keep ADR-0026 and accept the budget.** Rejected: raising the budget is what
ADR-0025 forecloses, and the alternative — curating the prose — was done too and
recovers less (~300 tokens) than the exports do.

**Delete the comparison outright.** What ADR-0026 rightly refused. It is the only
check that the seam holds, and it is cheap to keep once it is in the right place.

**Split `prototype_kit` into two examples.** ADR-0026 rejected this as a whole
duplicated game to say one thing twice, and that is still true; the test is not a
second game, it is twelve ticks of a scene that exists only to be uploaded to.
