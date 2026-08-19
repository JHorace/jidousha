# E0 run 6 — Pong

Friction log, written as it happened. I was the game's author; the engine was a
library I did not write and could not read. What I read: `docs/api/jidousha-api.md`,
`docs/api/jidousha-testing.md`, `crates/jidousha/examples/`. Nothing else — no
`src/`, no `docs/internal/`, no ADRs, no earlier run in this directory.

The game is `crates/jidousha/examples/pong/`. It works, it is playable, and
`cargo run -p jidousha --example pong -- --verify` passes with zero failures.
Most of what follows is therefore about *cost*, not about being blocked. I was
blocked exactly once, on a thing that turned out to be my own game design.

---

## 1. The one place the document told me something false

**`docs/api/jidousha-testing.md` says the clear colour is invisible to
assertions. It is one line away.**

> "Try the clear colour first, because nothing else in this document can see it."

That sentence is in the capture section, and it is wrong. `FrameRecord` has a
`pub plan: FramePlan`, and `FramePlan` has a `pub clear_color: Color`. Both are
in this document's own Reference, six hundred lines below the sentence. So:

```rust
assert_eq!(frame.plan.clear_color, palette::COURT);
```

I believed the sentence, wrote no check, and then found the hole the way the
same document recommends — by breaking the game on purpose. Seventeen deliberate
faults, sixteen caught, and the one that escaped was the clear colour, because
the document had told me not to bother. Cost: one mutation round, and it would
have cost nothing if the sentence had said "the *capture* is how you judge
whether it looks right; `frame.plan.clear_color` is how you assert it is the
colour you asked for."

Worth adding: the naive version of that assertion is itself a trap, and I wrote
it first. Comparing `plan.clear_color` against the game's own `palette::COURT`
constant does not survive somebody changing `palette::COURT` — the game and the
check move together and the mutation walks straight through. The check that
works is an absolute one about the *game's* contract, spelled in numbers the
constant cannot move: the court must be dark enough for a white ball. Both are
in the file now, and only the second catches the constant being changed.

## 2. The blocker: a correct game, a correct controller, and 0–0

This is the interesting failure of the run, and it is the one the testing
document spends the most words on — from the other side.

My first opponent predicted where the ball would cross its plane (the same
`predict_cross` my controller used) and moved at 17.5 u/s. My controller played
to win exactly as the document prescribes: constrain first (only contact points
well inside the paddle, only positions reachable in time), then optimise (push
each survivor through the game's own bounce and take the one landing furthest
from the opponent). The run came back:

```
board = 0-0, touches: 37, longest_rally: 37, top_speed: 27.0, returns: 37
approaches = 18, contacts = 18
```

A 37-touch rally at 0–0 — the exact shape the document says means *your
controller made the game degenerate*. It had not. Four lines of arithmetic:

| ball | flight across the court | opponent covers at 17.5 u/s | court is |
|---|---|---|---|
| serve, flat | 1.56 s | 27.3 units | 17.2 units tall |
| serve, steepest | 3.12 s | 54.7 units | 17.2 units tall |
| top speed, flat | 0.98 s | 17.2 units | 17.2 units tall |

The opponent could cross the entire court during the *fastest shot the game
could produce*. It was unbeatable by geometry, and no controller could ever have
scored. The fault was mine, in the game.

The document's advice is right and it is also load-bearing in a way it does not
say: **its warning is calibrated for the case where the controller is at fault,
and a run that lands in the other case needs a way to rule the controller out in
one step.** Prose cannot do that — the document itself says prose has failed at
this three times. What did it was the thing the document asks for two paragraphs
later: the controller asserting its own contract on the numbers it actually
picked. `met 18 of 18 approaches` is a controller reporting itself healthy, and
it is what let me stop suspecting it and go do arithmetic instead. I would put
that sentence *next to* the warning rather than after it: the self-check is not
a nice extra, it is what makes the warning actionable in both directions.

The fix was to the game, and it is the classic one: the opponent chases where
the ball **is** rather than where it is going, so it lags behind a steep ball and
a wall bounce sends it the wrong way first. Making it slower would not have
helped — the table above says so at every speed.

Second-order friction from the same place: once the opponent chases rather than
predicts, there is *no closed form* for where it will be, so my controller's
planner had to roll the game forward tick by tick — 13 candidate shots × up to
400 ticks, per decision. I expected that to be too slow and it is not: the whole
verify run, 2013 ticks of match plus two idle runs plus three staged screens plus
a GPU capture, takes 2.3 seconds in a debug build. Nothing in either document
gives any sense of what a headless tick costs, and I nearly designed around a
cost that is not there. One sentence — "a headless tick is cheap; a thousand of
them is not a thing to budget for" — would have saved a design decision.

## 3. Things the document did not tell me, that I had to guess at

**Whether `ctx.text` submits a quad for a space.** The document says "one quad
per character" and I could not tell whether a space counts as a character for
this purpose. I wanted to assert an exact glyph count on my hint line. I did not
trust it enough to, and asserted on the combined bounds and `width_of` instead —
which is a better check anyway, so this cost me nothing but a detour. For the
record, measured from the run: 61 quads in a rally frame = 2 walls + 13 halfway
dashes + 2 paddles + 16 ball wedges + 2 score digits + **26** hint glyphs, and
`"W / S to move - first to 5"` is 26 characters including its six spaces. Spaces
do submit quads. The check now asserts the count.

**A `const` angle in degrees cannot be written.** `Radians::from_degrees` is
"for humans" but is not a `const fn`, so a `const MAX_BOUNCE: Radians` of sixty
degrees has to be written as a number. I wrote `Radians(1.0471976)` and clippy
rejected it — `approx_constant`, an approximation of `FRAC_PI_3`. The spelling
that compiles is `Radians(core::f32::consts::FRAC_PI_3)`, which is the one
spelling nothing in either document uses and which stops being writable the
moment the angle is not a nice fraction of pi. This is a real gap: every game
that bounces something has an angle constant.

**Whether `Vec2` has `lerp`.** `vec2_tour.rs` is presented as *the* entry for
`Vec2` — "the reference cannot generate an entry for it… this file is the entry
instead" — and it does not list `lerp`. Swept collision needs to interpolate a
contact point along a tick's travel, which is the one operation the Concepts
section explicitly sends a game author off to write. I wrote
`from + (to - from) * t` rather than find out. Either `lerp` exists and the tour
is not the complete entry it claims to be, or it does not and the omission is
correct — but the tour's own framing means I could not tell which, and the whole
point of that file is that I should not have to guess.

**That a game in `examples/` is held to the engine's own lint config.** The
workspace `[lints]` apply to example targets, so `cargo clippy --all-targets
-- -D warnings` failed my game on `collapsible_if` (the fix is let-chains,
`if let Some(t) = hit && …`) and on the `approx_constant` above. Neither document
mentions that the game author inherits the maintainers' lints. Two of my three
clippy failures were in the *check*, not the game. Not hard, but it is a surprise
arriving at the "definition of done" step rather than at the writing step.

**Whether `Vec2` field access works in a `const` expression.** I wrote
`const PADDLE_LIMIT: f32 = FIELD_HALF.y - PADDLE_SIZE.y * 0.5;`. It compiles.
I had no way to know it would from the documents, since `Vec2` is glam's.

## 4. Two ways to do the same thing, in the two places a game author looks

`docs/api/jidousha-api.md`'s conventions open with "One way to do everything."
Getting a frame out of a headless game has two, and the canonical example uses
the one the testing document does not recommend:

- `docs/api/jidousha-testing.md` says: `FrameRecorder::new(viewport)` then
  `recorder.draw(&mut sim)`.
- `examples/prototype_kit/verify.rs` — the file I was pointed at as the worked
  example of the `--verify` shape — does `sim.draw()`, then `plan_frame(&camera,
  &quads, &textures)`, then `backend.render(&plan)`, and builds its own
  `TextureTable` with `create_builtin_textures`, and reconstructs the font's
  backend id by making a throwaway `NullBackend` because the table is gone by
  the time its assertions run.

`prototype_kit` explains *why* it keeps the long way (it renders through a real
GPU backend as well as a null one) and even writes out the short way in a doc
comment. That is honest and I still lost time: the example is the thing you read
to learn the shape, and the shape it teaches has fifteen lines of ceremony that
the document says a game does not need. I used `FrameRecorder`, and
`recorder.font_texture()` really is one call. The friction is that I had to read
both and work out which was advice and which was an artefact.

Related, and this one is only in the capture section: the thing you hand
`WgpuBackend::render` is `frame.plan`, a `pub` field on the `FrameRecord` the
recorder already gave you. That is the join between the two documents' halves and
it is stated once, in passing. It is the sentence that makes the whole capture
recipe five lines instead of a rewrite of the game.

## 5. Things that took more than one attempt

- **The opponent.** Two designs — predict (unbeatable, §2), then chase. Not the
  document's fault; recorded because it was most of the run.
- **The speed constants.** Two rounds of tuning after the opponent was fixed.
  The document warns that a mediocre controller sends you into the game's
  constants; I want to record that I went there *twice* and both times it was
  correct, because the controller's self-check said it was healthy first. That
  check is worth the twenty lines it costs.
- **The clear-colour assertion.** Two attempts (§1).
- **`Keepsake { ..keepsake }`** — struct-update syntax partially moves. Rust,
  not the engine.
- **My own serve-direction expression**, which I wrote as four multiplied
  `signum()`s and which was nonsense. Replaced with a two-line `match`. Nobody's
  fault but mine; recorded because "the API made me write it" would have been an
  easy and false thing to claim.

## 6. Things I wanted to look up in the engine's source, and did not

1. **What a headless tick costs.** §2. This is the only one where not knowing
   changed a design decision.
2. **Whether `HeadlessSim::draw()` and `FrameRecorder::draw()` are the same
   thing underneath**, given §4. I wanted to know whether the recorder was doing
   something the long way was not.
3. **Whether `ctx.text` emits a quad for a space.** §3. Ten seconds in the
   source; instead I wrote a weaker assertion and then measured it out of the
   run's own numbers.
4. **Whether `Vec2::lerp` exists.** §3.
5. **What `Time::alpha` is during a `FrameRecorder::draw`.** Idle curiosity; the
   document says nothing in v1 consumes it and a prototype should ignore it, and
   I did.

None of these blocked me. Items 3 and 4 are both "does this one method exist",
which is the shape of question a generated reference is supposed to eliminate,
and both are about `Vec2`/text — the two places where the reference hands off to
a worked file instead of generating an entry.

## 7. What the documents got right, since a log of only complaints is a lie

- **The swept-collision paragraph in Concepts.** It states that there is no
  `Rect::sweep`, that this is a boundary and not an omission, why a partial
  helper would be worse than none, and then describes the eight lines to write:
  the plane the leading edge touches, whether it was approaching, whether this
  tick's travel crossed it, the fraction of the tick at which it did. I wrote
  exactly that and it worked first time. That paragraph is the single
  highest-value thing in either document for a game with a ball in it.
- **"A run only tests the states it reaches."** The suggestion to ask
  `sweep_contact` its contract directly — an eight-unit travel across a paddle
  0.8 thick, plus the two negative cases — is the only check in my file that a
  played match cannot reach, and it is the one that caught my "replace the sweep
  with a position test" mutation. Nothing else did.
- **"Collect the failures; do not exit on the first one."** My mutation round
  produced runs reporting three faults at once, and in one of them the precisely
  diagnostic line was third. Exiting first would have shown "no rally lasted long
  enough to be a rally", which is the symptom.
- **"Assert on the quad's bounds, not on the fact that something is there."**
  I displaced a paddle by 1.4 units in the draw system and the bounds check
  caught it twice (once per paddle). A covering-a-point check would not have.
- **The alpha-reads-brighter warning.** My score is drawn at alpha 0.13 and my
  halfway line at 0.10, both picked by eye off a capture as instructed, and both
  read correctly. Arithmetic would have put them three times higher.
- **"On the way into tick 1 there is nothing to look at."** My controller's
  world read returns `Option` for exactly this reason, and I did not have to
  discover it by crashing.

## 8. What I could not check

`tools/serve-web pong --check` would have put the game in a real browser window;
it needs `wasm-bindgen-cli` 0.2.127, which is not installed here, and installing
a toolchain is not a thing I should do to route around it. There is no display in
this container either, so `cargo run -p jidousha --example pong` reports
`RunError::NoDisplay` — with a genuinely good four-part message that names
`headless` as the fix.

So the windowed path is the one part of "done" I have not *seen*. What I have
instead: the `--verify` run drives the identical systems and config through
`headless`, and the capture renders one of its recorded frames through the same
`WgpuBackend` the window would use, on a real GPU, and the PNG looks like Pong.
Everything above the window is exercised; window creation and the keyboard
plumbing between `winit` and `Input` are not, and they are engine code rather
than mine.

## 9. Numbers, for whoever reads this next

Seventeen deliberate faults injected, one at a time, each reverted before the
next:

| broke | run said |
|---|---|
| clear colour constant | *(escaped, then caught — §1)* |
| W and S swapped | controller did not win; kept missing the ball |
| `layers::SCORE` moved above `PLAY` | the score is not painted behind the play |
| paddle drawn 1.4 units out of position | no paddle-shaped quad where a paddle is (×2) |
| swept contact replaced by a position test | a ball that crosses a paddle in one tick is not caught |
| em dash in the hint string | a character the font cannot draw |
| hint centred by 0.9× instead of 0.5× | not the width the layout measured; drawn off screen |
| wall reflection removed | the ball went through a wall |
| score drawn 0.4 off its column | a score is not in its column |
| opponent speed 15 → 60 | no frame of live play was recorded |
| bounce angle sign flipped | did not hit where it meant to; tip does not leave at the steepest angle |
| speed ramp removed | nobody won; the ball never got faster than its serve |
| paddle clamp removed | a paddle left the court |
| right goal line moved out | the ball left the court sideways and kept going |
| ball not drawn | no ball-sized disc where the world puts the ball |
| serve angle made constant | no rally lasted long enough; a player doing nothing does not lose |
| play-again line made too long | drawn off screen (×3) |
| camera cleared to the paddle colour | did not clear to what the camera asked for; not dark enough |

Run: 5–0 to the controller on tick 2013 (33.6 s of play), 26 returns, longest
rally 8 touches, top ball speed 40 u/s. Controller met 13/13 approaches and
landed 13/13 aimed returns. Idle run loses 0–5 in 506 ticks and replays bit for
bit. Whole thing: 2.3 s, debug build.

## 10. One process note

Mutation testing is the most valuable thing either document recommends, and the
natural way to revert a mutation — `git checkout -- <file>` — destroys any
uncommitted work in that file. It ate one of my checks twice before I learned to
commit first. If the testing document is going to recommend breaking the game on
purpose, "commit before you do" belongs in the same paragraph.
