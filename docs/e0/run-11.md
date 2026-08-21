# E0 run 11 — Pong

Written by a game author, not an engine maintainer. Read: `docs/api/*.md` (all
four) and `crates/jidousha/examples/`. Nothing under `crates/*/src/`,
`docs/internal/`, `docs/adr/`, `docs/conventions.md`, `docs/agent-practices.md`,
or any other file in `docs/e0/`.

The game is `crates/jidousha/examples/pong/` — `main.rs`, `controller.rs`,
`verify.rs`, `checks.rs`, `capture.rs`. It runs in a window, it has a
`--verify` mode, and `cargo clippy --workspace --all-targets -- -D warnings`
and `cargo fmt --all` are clean.

**Headline: the four documents were enough.** I never wanted to open `src/`,
never guessed at a signature that mattered, and the game was playable on the
first windowed run. Almost everything below is a small friction, a place where
the answer was in the document but not where I looked, or a thing I had to
measure rather than read. The one genuine gap is §1.

---

## 1. `PhysicalSize::aspect` is not a `const fn`, and a layout in constants
   needs it at compile time

The API document tells a prototype to put its layout in constants and to state
the aspect it took: *"`GameConfig::window_size` is the size `run` opens at and
`PhysicalSize::aspect` is the number it implies, so pick the shape there."* So I
wrote

```rust
const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
const HALF_W: f32 = HALF_H * WINDOW.aspect();   // does not compile
```

`aspect()` is not `const`, so the half-width — the number every horizontal
position in the game is stated against — cannot be derived from the window size
the game actually opens at. I wrote `HALF_H * (16.0 / 9.0)` by hand and added a
check that the constant agrees with what `Camera::visible_bounds()` reports.
That check is the only thing standing between the two numbers.

This is exactly the shape of the `Radians::from_degrees` finding the conventions
section already records: *"Constructors and accessors of the plain-data types
are `const fn` for this reason, and a new one follows the same rule."*
`PhysicalSize::aspect` is an accessor on a plain-data type, it is the number the
document sends a prototype to, and it is not `const fn`. Same rule, same
discovery method — by trying to write the constant.

Cost: five minutes and one hand-written ratio that nothing but an assertion
couples to the window size.

## 2. The document predicted my game's balance failure exactly, and I still had
   to hit it to believe it

`jidousha-controllers.md` ends with: *"An opponent nobody can score against is
the commonest way a first game is broken... `opponent_speed * crossing_time >=
the interval it has to defend`. Every first opponent is written by picking a
speed that looks fair, and looking fair is not the test."*

I picked a speed that looked fair (opponent 20 u/s against a ball reaching 32).
First `--verify`: the rollout player finished 1–0 in 4000 ticks, the opponent
returned 96% of everything, and the chaser run came back 0–0 — the degenerate
groove the same document describes, in the same words, one page earlier.

I do not think this is a documentation failure. The document said the thing and
I read it; what it cannot do is make the arithmetic feel real before you have
run it. What it *did* buy was the diagnosis in one run rather than six: three
players and three numbers, and the report said "the controller is fine, the game
is not" without my having to suspect anything.

Two notes for whoever writes the next one:

- The fix was not a speed. It was **`OPPONENT_BIAS`** — the opponent had to meet
  the ball *off its own centre*, so that its returns carry an angle. An opponent
  that centres on the ball plays a flat rally against anyone who also centres on
  the ball, and no amount of speed tuning gets out of that. The document says
  this about *controllers* ("a controller that tracks the ball perfectly returns
  it dead flat") and the same sentence is true of the **game's own opponent**,
  which is a different piece of code in a different file. I had to make that
  transfer myself, and I nearly tuned speeds for a round first.
- The other half of the fix was making the ball's vertical speed exceed *both*
  paddles' speeds at the steep end. That is what turns "follow the ball" from a
  winning strategy into a losing one, and it is the same coupling the API
  document names between paddle thickness and top speed, seen from the gameplay
  side rather than the collision side. Worth saying once in the balance
  paragraph: **`MAX_SPEED * sin(MAX_BOUNCE)` versus paddle speed is what decides
  whether the game is a game.**

## 3. `Depth`, `Rect`, `f32::signum`, the `let else` idiom — the document was
   ahead of me on all of them

Listing these because a run that reports no friction is less useful than one
that says where it nearly went wrong. Each of these I would have got wrong, and
did not, because the document said so first:

- **Submitting the court *after* the play so the band is observable.** I would
  have written `draw_the_court` first, out of habit, and then written a "layers
  work" check that could not fail. The paragraph beginning *"a band is only
  visible where it changes the order"* is the single most useful thing in the
  testing document, and it is not a fact about `Depth` at all — it is a fact
  about what a recorded frame can and cannot see.
- **`neg_cmp_op_on_partial_ord` and NaN being the same edit.** I wrote the
  sweep's guard in the document's exact shape and it was clippy-clean first
  time. The observation that `a <= b` is the *behaviour change* the lint's name
  invites is not guessable and saved a real bug.
- **`signum` answers 1.0 for zero.** My opponent's aim bias is written on the
  sign of the ball's height. Without that sentence I would have written a
  three-way `match` on a comparison and got a judder at exactly y = 0.
- **`Rect::contains` is half-open and `contains_rect` is closed.** The disc-size
  filter is spelled out with explicit comparisons for the reason the document
  gives; I copied the shape rather than deriving it.
- **The `capture:` line `tools/verify` parses.** I would have written
  `capture: wrote target/verify/pong.png` and been silently unreported.

## 4. Things I had to measure rather than read, and would have liked to read

None of these are wrong in the document. They are places where I ran the program
to find out something a sentence could have told me.

- **How long a `--verify` run costs.** The testing document says a tick is cheap
  and gives one worked figure (2,013 ticks and a controller thirteen futures
  deep in about two seconds, debug). My first run took what felt like a minute
  and I started designing around the cost before realising most of that was
  `cargo build`. Three headless 4000-tick matches with a rollout controller doing
  ~1,600 simulated steps per tick come out at **0.8 seconds**. The figure in the
  document is accurate; what I wanted was the reminder that `cargo run` prints
  nothing while it compiles.
- **How many quads a played frame is.** 73 for this game. Useful for sizing
  assertions, and I had to print it.
- **What `create_builtin_textures` covers.** The capture document says "the three
  textures the renderer always has" and `FONT_TEXTURE` is one of them, so a game
  of pure shapes and text needs no `Assets`, no `MemorySource`, and no
  `upload_ready_textures` in its capture path — the whole texture table is one
  call. `prototype_kit/capture.rs` is the worked version *for a game with art*,
  and the shapes-only path is shorter than it in a way I had to confirm by
  deleting lines and seeing the picture still come out right. One sentence in
  the capture document — "a game of shapes and text needs only
  `create_builtin_textures`" — would have saved that.

## 5. Two things that took more than one attempt

- **The score-position check, twice.** I wrote
  `assert!(quad.min.y < SCORE_TOP + margin)` first, then reread the paragraph
  about a check that moves with its own constant, and rewrote it as "the score's
  glyphs sit in the top third of `visible_bounds()`, one number either side of
  the centre line, evenly set". The mutation round then caught
  `SCORE_TOP = -1.0` immediately. The original would have passed. The document
  is explicit that layout is where this bites and colour is only where it is
  easiest to see — I still wrote the colour pair correctly and the layout one
  wrongly on the first pass, which is the transfer failure the document says has
  happened three runs running. Make that four.
- **`predict_contact` returning `(tick, height)`.** I destructured it as
  `(landed, _)` and got a type error rather than a wrong number, which is the
  good outcome. Worth one line only because it is the kind of thing that would
  have been a silent 0.0 in a dynamically typed engine.

## 6. What I wanted to look up in the source, and did not

Twice, and both times the document had it and I had misread:

- **Whether `ctx.text`'s quad is the glyph's ink or its whole cell.** My "score
  set evenly either side of the line" check compares the left number's right
  edge against the right number's left edge, which is only exact if the quad is
  the cell. The API document says it plainly — *"a glyph's quad is `size` tall
  whatever the character draws inside it"* — in the vertical-metric paragraph,
  which I had read as being about `\n` spacing. It is about both.
- **Whether `Startup` runs before or inside the first `tick()`.** Asked three
  separate times, answered three separate times, in Concepts, in the resource
  table and in the testing document's controller section. I kept re-reading it
  rather than trusting it, because it is the one fact in the surface that
  contradicts the name of the thing.

## 7. One thing that behaved differently from what I expected (not from what the
   document implied)

The ball drawn flush against the camera's edge. `GOAL_X = HALF_W - BALL_RADIUS`
is the obvious "the ball is never drawn off screen" constant, and it is correct:
`contains_rect` is closed, the check passes. It passes by **0.06 world units**,
and the only reason I knew is the clearance line the testing document tells you
to print. I added a `GOAL_MARGIN` and the number went to 0.20.

That paragraph — *"the check itself is a cliff... a game has shipped at 0.03
without anybody knowing"* — earned its place in this run. It is one `println!`
and it found a real thing.

## 8. Mutation round: 19 injected faults, 19 caught

Done as the testing document asks, after committing. Every fault was a one-line
edit to `main.rs`; the harness treats a search-and-replace that matches anything
other than exactly once as an error, and tells a failed build apart from a
failed check.

Faults injected: paddle drawn out of position · W and S swapped · ball moved
before the paddles in the schedule · rebound's vertical sign flipped · the sweep
replaced by a position test · court cleared to something bright · score moved
into the middle of the play · score set unevenly · field band moved above the
play band · opponent's aim bias set to zero · opponent made unbeatable ·
opponent made a sieve · em dash typed into the hint line · banner centred by its
longest line · ball not drawn at all · `MAX_SPEED` raised past a paddle's
thickness · one wall stopped reflecting · the loser congratulated · the paddles
swapped ends.

Three of those checks did not exist before the round, and I would not have
written them:

- **"the winning and losing screens are the same screen".** Every check I had
  judged each end screen on its own — on screen, printable, each line centred —
  and all of them pass for a banner that congratulates the loser. The capture
  document names `YOU WINS 5 - 2` as a real caught fault; this is its sibling
  and no assertion I had written could see it.
- **"a paddle is drawn at the wrong end of the court".** My paddle-position
  check took the expected position from `Side::sign()`, so flipping `sign()`
  moved the check along with the game. The colours are what say *whose* paddle
  is where. This is the constant-moves-with-the-check trap in a place I did not
  expect it: not a layout constant, a *method*.
- **"a wall does not turn the ball round".** Removing one wall's velocity flip
  was caught only by "nobody won the match" — the conclusion, four checks
  downstream of the fault, with the diagnostic ones silent because the position
  is still folded back inside the court and every extent check and every drawn
  frame is unchanged. Asking `bounce_off_walls` its contract directly is three
  lines and names the fault first.

The pattern in all three: **a check that reads the game's own answer back cannot
see the game's answer change.** The document says this about constants. It is
equally true of methods, of enum arms, and of any pair of screens judged only
one at a time.

## 9. What the run reports

```
verified pong over 4000 ticks, three players
  rollout: 7-0 won on tick 3440; longest rally 8 touches, top speed 35.5 u/s
  controller: met 17 of 17 approaches; planned returns aimed to land 2.15 from
              the opponent; shots landed 0.13 from where they were planned to
  chaser: 1-1   do-nothing: 0-7
  opponent returned 16 of 23 balls (70%)
  last live frame: tick 3439, 73 quads, 42 glyphs, 2 batches
  closest quad to the edge: 0.20 world units
  ball stayed inside Rect { min: (-14.65, -6.35), max: (16.95, 6.35) }
  capture: 480x270 written to target/verify/pong.png
```

The `chaser: 1-1` line is the one that says the game is worth playing, and it
is the line I would not have printed without the controllers document. A blind
person mashing W and S at a window under Xvfb lost 1–5, which is about the same
verdict from the other direction.

## 10. Nothing I was blocked on

For completeness, since being blocked is a result: I was not, at any point. The
one thing I could not do (§1) had a one-line workaround plus an assertion, and
everything else in the four documents was either present or derivable from what
was present.
