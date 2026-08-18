# ADR-0023: What `FrameRecorder` hands back, and how long it keeps it

Status: accepted · 2026-08-18

## Context

`FrameRecorder` is F-010's fix and it worked: run 2 called it "the right shape",
run 3 used it throughout, run 4 used it for everything and the nine-line apology
comment about throwaway backends is gone from every game. This ADR is about a
smaller problem found *because* the recorder is now in the middle of everything.

```rust
pub fn draw(&mut self, sim: &mut HeadlessSim) -> &FrameRecord;
pub fn frames(&self) -> &[FrameRecord];
```

Those two cannot be used in one function. E0 run 4:

> The document gives two snippets a page apart: draw a frame per tick and then look
> at `recorder.frames().last()`; and then "check the screens your run never reaches"
> with another `recorder.draw()`. Doing both is a borrow error — `frames()` holds
> the recorder immutably for as long as the frame reference lives, and `draw()`
> wants it mutably. I ended up doing *both* workarounds: `.cloned()` for the
> match's last frame, and a second `FrameRecorder` for the staged screens.

The second `FrameRecorder` cost more than the borrow did: it also moves what
`transcript()` prints, and the run printed a synthetic staged screen instead of the
real last frame of the match before catching it — losing, briefly, the one artifact
a run with no display has.

**This is not an exotic composition. It is the shape the document recommends.**
F-032's fix — build the screens your run never reaches — was written into
`testing.md` a page after the record-every-tick snippet, and every check that does
both needs both halves in one function.

**Related, same design question.** The recorder retains every frame, 2,598 of them
in run 4 to look at one, and has no `clear()` — though `NullBackend`, the
lower-level path the recorder exists to replace, has one.

The documentation half is fixed already: the first snippet ends `.clone()`, with a
paragraph naming the borrow rule, saying `draw`'s return value has it too, saying to
read `font_texture()` out before the loop for the same reason, and stating the
retention plainly. What follows is whether v1 should also change the shape.

## Decision

**Recommendation: `draw` returns an owned `FrameRecord`. Add nothing else.**

```rust
pub fn draw(&mut self, sim: &mut HeadlessSim) -> FrameRecord;   // was &FrameRecord
```

`FrameRecord` is already `Clone`, so this is the clone the document now tells every
caller to write, moved to the one place that knows it is needed. The borrow ends
when `draw` returns; `frames()` and `draw` compose; the second recorder and the
`.clone()` both disappear; nothing else about the type changes.

The case, in order of weight:

**The borrow buys nothing.** Returning `&FrameRecord` saves a copy the caller
almost always makes anyway — the paragraph the documentation just gained says to
make it "as a matter of course" — and in exchange it makes the recommended
composition a compile error. A reference is the right return type when the caller
usually just looks; here the caller usually keeps.

**It is the cheap end of a real cost.** Run 4 paid for this twice: once in the
`.cloned()`, once in a second recorder that silently redirected `transcript()`. The
second is the expensive kind of workaround — it compiles, it passes, and it changes
what the run's only artifact contains.

**The copy is small and the scale is prototype scale.** A `FrameRecord` is a
`FramePlan`: a view-projection, a clear color, and the frame's batches. Run 4's
largest frame was 101 quads. A test drawing 2,598 of those is already keeping all
of them.

### `clear()` is declined, and retention stays

Adding `clear()` would make the recorder's frame history a thing a check manages,
and the reason not to is what the history is *for*: it is the record a failing
assertion reads backwards. A check that clears is a check that has thrown away the
tick before the one that broke, which is the tick the failure message wants. Run 4's
own diagnosis of its sign error came from a *quantity accumulated over the whole
run* — "the longest rally was 1 paddle touch over 502 ticks" — not from the last
frame.

`NullBackend` has `clear()` because it is the backend seam, where a golden-image
test genuinely wants a fresh surface between comparisons. That is a different job,
and "the lower-level thing has it" is not an argument that the higher-level thing
should.

If retention ever becomes a real cost, the answer is a constructor that says so —
`FrameRecorder::keeping_last(viewport)` — not a mutator that makes every existing
check's history conditional on nobody having called it.

## Consequences

- **A breaking change to a published signature**, and the mildest of the three
  taken in this batch: every existing caller writing `let frame = recorder.draw(&mut sim);`
  kept compiling, and the callers that relied on the borrow were the ones the
  borrow was breaking.
- `testing.md`'s borrow-error warning is gone. What is left is the loop keeping the
  frame it drew, plus the two facts that remain true — `frames()` still borrows, and
  `font_texture()` is still worth reading out early. **That shrinkage is the test
  that the change worked.**
- **`pong/verify.rs` lost a whole `FrameRecorder`.** The second one existed only to
  draw the staged screens while a reference into `frames()` was alive; the staged
  screens now go through the same recorder as the match, and the three-line comment
  apologising for the clone is gone with it. `prototype_kit/verify.rs` was
  unaffected.
- `a_recorded_frame_outlives_the_next_draw` is the regression guard, and it is
  written as the two paragraphs of `testing.md` that did not compile together.
- `tools/gen-api-doc` rerun; no coverage change, since `draw` is already shown.

## Alternatives considered

**Leave `draw` returning a reference and keep the documentation fix.** The status
quo after this commit, and it is genuinely defensible: the composition works, the
`.clone()` is one call, and the document now names the rule in the place a reader
hits it. The argument against is that it teaches every caller a workaround for a
constraint that buys nothing, and the "second recorder" mistake is the one it does
not prevent — a reader who does not want to think about borrows reaches for a second
recorder before reaching for `.clone()`, which is what happened.

**Return `&FrameRecord` from `draw` and add `frames_cloned()`.** Two ways to get a
frame's history, and the second one named for its implementation. Rejected on
ADR-0012's rule.

**Add `clear()` as well as the owned return.** Covered above: it makes the history
conditional, which is the property a failure message depends on.

**Have `draw` return an index into `frames()`.** Composes — an index borrows
nothing — and is the worst option to read: every assertion becomes
`recorder.frames()[index]`, which reintroduces the borrow at the use site and puts
an integer between a test and the thing it is asserting about.
