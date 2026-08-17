# e0 run 3 — Pong

Written against `docs/api/jidousha-api.md` and `crates/jidousha/examples/` only.
No engine source, no ADRs, no internal docs, no earlier run logs were opened.

Result: **a playable Pong**, in `crates/jidousha/examples/pong/` (four files:
`main.rs`, `draw.rs`, `session.rs`, `verify.rs`). `cargo run -p jidousha
--example pong -- --verify` passes; `cargo fmt --all` and `cargo clippy
--workspace --all-targets -- -D warnings` are clean. The windowed path was
exercised only as far as this machine allows — there is no display here, so
`run` returns `RunError::NoDisplay` and prints it. **I have never seen this game
drawn.** Everything I know about what it looks like came out of
`FrameRecorder::transcript()`.

Headline: the API document was enough. I did not once need to open `src/`, and
there was no point at which I felt blocked. What follows is the friction, not a
complaint about the outcome.

---

## 1. Things the document did not tell me, that I had to guess

### 1.1 The font's character set

I wrote `"w / s to move · first to 5"` and `"you win — space to play again"`
before it occurred to me to check. The API document says nothing about which
characters the built-in font has. `prototype_kit` draws a sample of
`0x20`–`0x7e` and calls it "the whole printable range", which is *evidence* but
not a statement, and it is in an example rather than in the reference.

What made this worse than a guess: **a non-ASCII character still submits a
quad.** The `·` in my hint produced a glyph quad in the transcript, at the same
advance width as every other character. So every check I have — glyph counts,
"was text drawn", the off-screen bounds check, `width_of` centring — passes
identically whether that quad draws a middle dot, a blank, or garbage. There is
no assertion available to me that can tell the difference, because the only
thing I can inspect is quad geometry, and the geometry is right either way.

I retreated to ASCII everywhere and left a comment saying why. This is the one
piece of the game I cannot verify at all, and the document gave me no way to.

**What would have fixed it:** one sentence in `TextStyle`'s entry saying which
characters the atlas carries and what happens to one it does not.

### 1.2 The character advance width

`TextStyle::width_of` is documented as exact, and it is, so centring works. But
I could not do any layout arithmetic *ahead* of running: I could not tell
whether a 38-character banner at size 1.4 would fit in a 35.5-unit-wide camera
without building it and looking at the transcript. I laid out the score, the
banner and the hint by guessing, running, and reading the numbers back.

From the transcript the answer turns out to be `7/9 × size` (a 1.5-unit line
advances 1.1667 per character). That is a fact I extracted from output rather
than one I was told, and I have deliberately not hard-coded it anywhere.

This is a real cost and not just an aesthetic one — see §3.2, where a string I
could not measure in advance is the thing that would have shipped broken.

### 1.3 What the tick rate is *for the game I am writing*

The document says the timestep is 1/60 "unless you say otherwise" and that sixty
is the number to count in for a serve pause. That is clear. But everything
motion-related in Pong is naturally written in units *per second* — paddle
speed, ball speed — and I had to decide for myself whether to multiply by
`Time::fixed_dt` or to write per-tick constants.

`prototype_kit` writes its paddle speed as "world units per tick"
(`speed: 0.25`) and does not touch `fixed_dt`. `scripted_player.rs` writes
per-second constants and multiplies by `fixed_dt`. Both are in the examples
directory, and they are opposite conventions. I went with per-second-and-
multiply, because it survives a change to `GameConfig::fixed_dt`, but the
document has no opinion and the two worked examples disagree.

### 1.4 Whether `overlaps` is enough for collision

`Rect::overlaps` is documented, including that "touching edges do not count".
What is not addressed anywhere is the obvious next question for a game with a
fast small ball: nothing in the API sweeps, so a ball that moves further in one
tick than a paddle is thick passes straight through it.

I do not think this is a gap — it is my job as the game author. But the document
is written to hand a game author the vocabulary for a prototype, and the
tunnelling problem is the first thing that bites anyone putting a ball and a
paddle on a fixed timestep. A line under `Rect::overlaps` saying "collision is
tested at tick boundaries; a mover faster than its target is thick will step
through" would have saved me working it out.

I ended up writing the invariant into my own verify run as an assertion against
the `Time::fixed_dt` the engine actually hands the game, which is a better
outcome than a comment would have been. But I got there by worrying, not by
reading.

### 1.5 `Batch`

`prototype_kit/verify.rs` reads `plan.batches`, `batch.texture` and
`batch.quad_count()`. `Batch` has no entry in the API document at all —
`FramePlan` names the field as `Vec<Batch>` and the type is never described. I
did not need it (I used `FrameRecorder`, which is the shape the document
recommends for a game), but if I had followed the worked example instead of the
prose I would have been writing against an undocumented type.

## 2. Things I expected to exist and could not find

### 2.1 Nothing, really — with one shape I kept reaching for

I expected a way to ask "is the game over, quit". The document says plainly and
early that there is not one, and says it is a v1 boundary rather than my having
missed it. That is exactly the right way to write it and it cost me nothing.

The one thing I kept reaching for was a **swept or continuous collision helper**
(§1.4). I did not expect it to exist, and it does not; I mention it only because
it is the single piece of vocabulary a Pong needs that shapes-and-text does not
cover.

### 2.2 A way to see the game

Not a gap in the API — a gap in what this exercise can give me. I have a
transcript and I have assertions, and between them I am fairly confident about
geometry, layering and layout. I have no idea whether the colours look good,
whether the dashed halfway line reads as a line or as twelve unrelated blobs, or
whether alpha 0.22 on the border is visible at all. The document's warning that
alpha "reads brighter than the number looks" is the only guidance, and it says
explicitly to pick these by eye from a capture — which I cannot do.

`prototype_kit` writes a PNG through `WgpuBackend::offscreen`. I deliberately
did not copy that: it needs a GPU, there is none here, and it would have added a
skip-path that always skips. So my game has no captured picture. That is a
correct decision for this machine and a hole in the deliverable.

## 3. Things that behaved differently from what the document implied

### 3.1 `headless` hands back an empty world — and my *controller* is what tripped on it

The document says this outright: "`headless(..)` hands back a world that is
still empty, and it is populated once the first `tick()` returns." I read it,
and I still wrote a verify loop that read `world.resource::<Scoreboard>()`
*before* the first `tick()`, because the controller has to look at the world to
decide what to press, and the natural place for that is at the top of the loop.

It panicked on tick 1. First attempt at the verify run, straight into it.

I want to be precise about whose fault this is, because it matters: the document
told me, and I still did it. The reason is that the document frames the fact
around *arranging a test's starting state* ("Startup running inside that first
tick is worth knowing if you drive the sim by hand"), and the case that actually
bites is different — it is the **closed-loop controller**, the exact shape the
document recommends two pages later for testing whether a game is playable. The
`SnapshotBuilder` example in *Testing your game* reads the world at the top of
its loop (`let want = /* look at the world, then decide */`) and does not
mention that on the first pass there is nothing there to look at.

So: the fact is documented, and the one worked example of the pattern that runs
into it does not flag it. That combination is why I hit it despite having read
the sentence.

The panic message, for the record, was excellent — it named the resource, said
resources are inserted explicitly, gave the likely cause and gave two fixes, one
of which (`find_resource`) was the correct one. Cost: about two minutes.

### 3.2 `width_of` being silent is worse than the document makes it sound

The document warns about this, in strong terms, and dedicates a paragraph to the
one assertion that catches it. I wrote that assertion early and it passed for
the whole run.

Then I noticed that my longest string — `"the machine wins — space to play
again"` — is **only drawn when the machine wins**, and my verify controller is a
perfect tracker that wins 5–0 every time. The bounds check ran 5,400 times and
never once drew the string that would have failed it. At an estimated 41 world
units against a 35.5-unit camera, it would have run off both edges on the first
match a real person lost.

This is a sharper version of the documented trap and I do not think the document
gets it across: the danger is not "text is silently too wide", it is "**the
screen that is too wide is the one your test never reaches**". A test that wins
never sees the losing screen; a test that finishes never sees the timeout
screen.

I fixed it two ways: split the banner into two independently-centred lines, and
added a check that builds the losing screen by hand — one tick to let Startup
run, then set `Scoreboard` directly and draw one frame. Confirmed the check
works by temporarily lengthening the string (it failed, with the numbers) and by
temporarily emptying it (it failed differently, with a glyph count).

**Suggested addition to the document**, one sentence after the existing
`visible_bounds` paragraph: *"Check the screens your run does not reach —
build them by hand and draw one frame. The banner that overflows is usually the
one on the losing screen."*

### 3.3 `Rng::below` vs `next_u32() % 2`

Not the engine's fault, but worth logging as friction the *conventions* created:
I wrote `next_u32() % 2 == 0` for a coin flip and clippy rejected it
(`manual_is_multiple_of`). The right answer was `Rng::below(2)`, which is
documented and which I had read past. The lint pushed me toward
`is_multiple_of(2)` — the engine's own "one way to do everything" answer was the
better one and the lint does not know about it.

## 4. Things that took more than one attempt

### 4.1 Making the game *fun* — five attempts, and the only real fight of the run

This had nothing to do with the API and everything to do with Pong. Logging it
because "took more than one attempt" is the question asked.

The verify run's assertion was "somebody reaches five points inside ninety
seconds". Sequence:

1. **0–0 after 90 seconds, one rally of 78 touches.** Both paddles centred the
   ball perfectly, and a centred hit returns at angle zero. The rally locked
   into a flat horizontal groove neither side could ever lose.
2. Weakened the AI (speed, later reaction). **Still 0–0, rally of 73.** Same
   groove; the AI does not have to be fast to hold a horizontal ball.
3. Added `MIN_BOUNCE` — a floor on the return angle, so a centred hit still
   moves the ball a little — and rewrote the test controller to *aim* rather
   than to track: it puts its paddle deliberately off-centre so the ball leaves
   at an angle away from where the opponent is standing. **3–0.** Playable, but
   a point took thirty seconds.
4. Weakened the AI again (wakes at x=8 rather than x=2, so it only reacts on the
   last quarter of the field). **5–0, won at tick 4932 — 82 seconds.** Passing,
   but still slow.
5. Raised every speed by about a third and widened the paddle to keep the
   tunnelling margin (§1.4). **5–0, won at tick 3522 — 59 seconds, a point every
   eleven seconds.** That is the shipped tuning.

The lesson I would pass on, and the reason step 3 is the interesting one: a
closed-loop test controller that plays *safe* is not a playability test. A
tracker that centres every return proves the controls work and simultaneously
proves the game cannot be won, because it has made the game degenerate. The
document is right that a blind script says nothing about playability — but a
naive tracker is only one step better, and it took me two failed runs to see
that. `scripted_player.rs`'s closed-loop example chases a target, which is
tracking; there is no worked example of a controller that plays to *win*.

### 4.2 Everything else was one attempt

Spawning, querying, the two-pass read-then-write shape, `Depth` layering,
`FrameRecorder`, `SnapshotBuilder` edges, `InputScript` (which I ended up not
using), the four-part failure message via `message()`, `Camera::visible_bounds`,
and the determinism replay check all worked first time from the document alone.
The two-pass pattern in particular is called out in the document as "the one
shape that surprises people" and it did not surprise me, because it was called
out.

Total compile errors across the whole game: **zero on the first `cargo check`.**
Everything after that was gameplay tuning and two lint fixes.

## 5. Things I wanted to look up in the source, and what for

Three, all of which I resolved another way rather than opening `src/`:

1. **The font atlas's character coverage** (§1.1). I wanted to grep the glyph
   table for what is in it and what a missing character maps to. I stayed out
   and used ASCII instead, which is a workaround, not an answer — I still do not
   know.

2. **What `ctx.circle` expands into** — how many quads, and whether the quads
   stay inside the circle's bounding box. My off-screen assertion compares quad
   bounds against the camera, so a circle that expands to something larger than
   `2r` square would have made that check subtly wrong near the walls. I
   answered it from the transcript instead (the ball is one quad, exactly
   `2r × 2r`), which is a better answer than reading the code would have been,
   because it is the observed behaviour rather than the implementation.

3. **`Batch`** (§1.5), only to find out what `prototype_kit/verify.rs` was
   doing. I stopped when I realised `FrameRecorder` made the question moot.

## 6. What the document does unusually well

Recording these because a log of only complaints would misrepresent the run.

- **The resource-availability table.** Which resources exist, who inserts them,
  and which three can be absent, with `Input` and `Camera` called out
  specifically. I used `find_resource` in exactly the right two places on the
  first try because of that table.
- **Naming the failure modes rather than the features.** "A blind script never
  returns a ball." "`width_of` is exact and completely silent." "A failing
  assertion has to report the numbers it judged." Every one of those changed
  what I wrote, and the last one changed it the most — every failure in my
  verify run prints its numbers, and during the tuning fight in §4.1 those
  numbers were the *entire* diagnosis. "No one won: score 0-0, longest rally 78,
  top speed 27.0" told me the rally was degenerate in one line, without a
  screenshot and without a debugger.
- **The engine's own error messages.** The one panic I caused (§3.1) and the one
  `RunError` I hit (no display) were both four-part messages that named the fix.
  I acted on both without investigating anything.
- **Saying what does not exist.** `App::quit`, 0–255 colour constructors, the
  numpad. Being told "this is a v1 boundary, not something you missed" is worth
  a lot when you cannot go and look.

## 7. Summary of concrete suggestions

| # | Where | Suggestion |
|---|---|---|
| 1 | `TextStyle` entry | State the font's character range, and what a character outside it draws. It currently submits a quad, so nothing a game can assert on will catch it. |
| 2 | `TextStyle::width_of` | State the character advance as a fraction of `size`, so layout can be reasoned about before it is run. |
| 3 | *Testing your game*, after the `visible_bounds` paragraph | "Check the screens your run does not reach — build them by hand and draw one frame. The banner that overflows is usually the one on the losing screen." |
| 4 | *Testing your game*, `SnapshotBuilder` snippet | Note that on the way into tick 1 the world is still empty, so a controller reading it must use `find_resource`. The fact is stated earlier; the example that trips on it does not repeat it. |
| 5 | `Rect::overlaps` | One line: collision is tested at tick boundaries, so a mover that travels further in a tick than its target is thick will pass through. |
| 6 | Examples | The two worked examples disagree on whether speeds are per-tick or per-second. Pick one. |
| 7 | `FramePlan` | `Batch` is used by a worked example and has no entry. |
| 8 | Examples | There is no worked closed-loop controller that plays to *win*. A tracker that centres the ball can make a game degenerate and then report that it is unplayable; that cost me two full runs. |

## 8. Where this run stopped short

- No captured picture (§2.2). Nothing in this run rendered the game or looked at
  it. **Resolved after the fact, outside the run:** a human ran it on native
  Linux, confirmed it plays, and reported nothing wrong. So the colours, the
  dashed halfway line and the border alpha are fine — but that is a person
  checking afterwards, not something the run could establish, and the gap in
  §2.2 stands as written for any agent working the way this one did.
- The font question (§1.1) is unresolved, not solved. I worked around it.
- `tools/test` was not run, per the brief — this example opens a window.
- The AI is one difficulty. There is no menu and no second-player option; the
  brief allowed either and I chose the AI.
