# E0 run 5 — writing Pong against `docs/api/jidousha-api.md`

Author's brief: build a playable Pong reading only `docs/api/jidousha-api.md`
and `crates/jidousha/examples/`. Record every friction as it happens. This file
is written in order, so early entries are not corrected by later ones — where a
guess turned out wrong I have said so underneath rather than editing it away.

## Before writing a line

The API document is much better than "a generated reference". It has a
Quickstart that is a whole game, a *Concepts* section that pre-empts most of
what a first-timer would get wrong, and a *Testing your game* section that is
essentially a briefing for this exact exercise — it names Pong, it names the
paddle/ball/score decomposition, it warns about tunnelling with a fast small
ball, and it spends four paragraphs on how a `--verify` controller can lie to
you. That last part is the single most useful thing in the document and it is
not API reference at all.

So the honest headline for this run is: **most of the friction below is small.**
That is a result about the document, and I want it stated before the list, so
the length of the list is not mistaken for the size of the problem.

---

## F-1 — `FrameRecorder::transcript()` prints every frame, not the last one

Both places the document mentions it say the same thing. The reference entry:

> `pub fn transcript(&self) -> String;  // The last frame as stable, diffable text`

and the prose in *Testing your game*:

> `recorder.transcript()` renders the last frame as stable, diffable text —
> every quad's world-space extent, one per line.

It renders **all** of them. My run recorded 1,263 frames, and
`print!("{}", recorder.transcript())` produced **121,465 lines** on stdout.

The one that does what the document describes is `FrameRecord::transcript()`,
on the frame `draw` hands back — the reference gives it the near-identical
summary "The frame as text: deterministic, diffable, and readable in a diff",
so the two entries are two sentences apart and only one of them is about one
frame.

This is compounded by the `--verify` convention on the last page: "everything
after that is kept as evidence rather than reprinted, which is where the
transcript goes". An author who follows both instructions literally emits a
hundred thousand lines of evidence per run and never notices, because it is
kept rather than shown.

Cost: about five minutes, and only because the output looked wrong at a glance.
It would have cost nothing and stayed wrong if I had not looked.

## F-2 — I made the controller mistake the document explicitly warns about, in a shape the warning does not cover

This is the big one, and it cost two full cycles.

*Testing your game* spends four paragraphs on the controller-that-lies problem,
naming Pong, naming the 0-0-with-a-78-touch-rally failure, and giving the fix:

> Play to **win**: aim the return away from where the opponent is standing, meet
> the ball with the half of the paddle that sends it off-centre […] "try every
> return this paddle can produce, take the one that lands furthest from the
> middle".

I wrote exactly that. Thirteen sample contact points across the paddle, each one
pushed through the game's own `contact` function, scored by how far the return
lands from anywhere the machine can reach in time, take the best. It lost
**0-5** and made six returns in a whole minute.

The reason is that **the optimum is on the boundary of the feasible set.** The
sharpest return a paddle can produce is always the one struck at the very tip,
because that is where the bounce angle is widest — so "take the best shot"
means "stand so the ball hits the last millimetre of your paddle", every single
time. Any error at all is then a clean miss rather than a worse return. My
controller's dead band was 0.45 world units and the margin at the tip is zero,
so it missed almost everything.

The fix is one line of set arithmetic and is not in the document: **constrain
first, then optimise.** Only consider paddle positions that (a) actually touch
the ball with margin — I use 78% of the paddle's half-length, so the tip is not
on the menu — and (b) can be reached before the ball arrives. Optimise inside
what survives. `controller.rs`'s `best_aim` is the worked version.

If the document says one more thing about controllers, it should be that. The
existing warning is about a controller that is too *timid*; this is the
identical failure produced by a controller that is too *greedy*, and it reports
the same thing — that the game is unwinnable — with the same false confidence.

### F-2a — and then I did the second thing the document predicts

> when a number looks wrong, suspect the controller first — it is the newer and
> worse-tested of the two.

I did not. On the 0-5 result I went and changed `SERVE_SPEED`, `SPEED_GAIN` and
`MACHINE_SPEED`, and added a whole new difficulty knob to the game
(`MACHINE_VISION`, since deleted), before finding the fault in `best_aim`. The
document called this in advance, in a paragraph I had read that morning, with
the exact sentence "It is an instrument that will send you off to change the
thing you are measuring" — and I still did it. Reading the warning is not the
same as it working.

Recording this as friction and not as a personal failing, because the point of
the run is what the document costs its reader, and the answer here is that this
particular warning is not strong enough to survive contact with a red result.

## F-3 — a drawn quad has no layer, so draw order is the one thing a check cannot see

`Depth { layer, z }` gets a lot of attention: layers are "**yours**", name them
in a `mod layers`, a debug outline goes in front by saying so. I did all of
that — `layers::TABLE`, `PLAY`, `UI`, and the score sits on `TABLE` so the ball
passes in front of it, which is how Pong has always looked.

None of it is checkable. `DrawnQuad` is `{ batch, texture, corners, tint }` and
`FrameRecord::quads()` hands back a `Vec<DrawnQuad>`, so a verification can ask
where a quad is, what colour it is and what texture it sampled, but not what
band it was sorted into. Swap `layers::TABLE` for `layers::UI` on the score and
the picture changes — the score paints over the ball — and every assertion in
this game still passes.

I could not find a way to ask, and worked around it by not asserting on
ordering at all. `Quad` (the submitted type) *does* carry `depth`, and
`HeadlessSim::draw()` returns `&Submissions` whose `quads()` are `&[Quad]` — so
the information exists before planning and is gone after it. A check could go
through `sim.draw()` instead of the recorder, but then it gets submissions
rather than a drawn frame and loses `covering`, `bounds` and the font texture.
Two half-views, no whole one.

## F-4 — `TextStyle::width_of` on a multi-line string cannot centre it

The reference is accurate:

> `pub fn width_of(&self, text: &str) -> f32;  // How wide `text` will be in
> world units — its widest line, if several`

and the consequence is not drawn out anywhere: because `ctx.text` lays a block
out from its top-left corner, centring a two-line block by `width_of` centres
the *longest* line and leaves every shorter line hanging left of the middle. My
end-of-match banner is two lines of very different lengths, so this would have
been a visibly crooked screen that passes the off-camera check, the glyph check
and the printable-ASCII check.

I caught it by reasoning rather than by seeing it, which is luck. Two separate
`ctx.text` calls, each centred by its own width, is the fix and it is three
lines. The document's own advice — "text centred by `width_of` is the usual
culprit" — is about running off the edge, not about this.

## F-5 — the two worked examples disagree about whether a Draw system needs a `Vec`

The Quickstart draws straight out of the query:

```rust
for (_, transform, _) in ctx.world.query::<(&Transform, &Player)>() {
    ctx.rect(...);
}
```

`prototype_kit`'s `draw_the_field` and `draw_the_hitboxes` both collect first:

```rust
let paddles: Vec<Vec2> = ctx.world.query::<(&Transform, &Paddle)>()
    .map(|(_, transform, _)| transform.pos).collect();
for at in paddles { ctx.rect(...); }
```

I copied `prototype_kit`, because it is the bigger example and I assumed the
`Vec` was load-bearing — the *Concepts* section's "reading while writing: the
two-pass pattern" is emphatic, and a `DrawCtx` that is borrowed mutably by
`ctx.rect` looks like exactly that situation. I even wrote a comment explaining
why the `Vec` was necessary.

It is not. `WorldView::query` returns `QueryIter<'w, Q>` — the lifetime is the
*world's*, not the `&self` borrow's — so the iterator does not hold the
`DrawCtx` at all and the direct form compiles. I only found out by deleting the
`Vec` to see what the error said, and there was no error.

The document's own convention section says two examples that disagree teach
that there is no rule, about the exact analogous question of where `sin_cos` is
spelled from (F-045 in a previous run's findings, which the document cites).
This is the same shape one level down, and it cost me two unnecessary
allocations per frame and a comment that was actively wrong.

## F-6 — nothing says which tick number `Time::tick` holds inside an Update system

I gave the machine paddle a reaction time by having it look at the ball on
every twelfth tick: `world.resource::<Time>().tick.is_multiple_of(12)`. For a
modulo it does not matter whether the first Update sees `tick == 0` or
`tick == 1`, so I did not need to know — but a game wanting "spawn the boss on
tick 600" does, and the two candidate answers are one tick apart.

What the document says is `tick: u64  // Update ticks since startup`, that
`Time::new` is "the clock at the start of a run, before the first tick", and
that `Startup` runs *inside* the first `tick()`. From that I would guess the
first Update sees 1, but it is a guess, and I did not test it because my game
does not care. Noting it as an unanswered question rather than as a wrong
answer.

## F-7 — two ways to spell "the player is at the keyboard, doing nothing"

One of my checks plays a whole match with an idle player, to prove the game can
be *lost* as well as won — every other assertion in the file would pass against
a machine paddle that never scored. That needs an `Input` meaning "present and
idle", which is not the same as inserting no `Input` at all (my paddle system
reads `find_resource` and does nothing when it is absent, so the two are
indistinguishable in this game — but they would not be in a game that pauses
when input is missing).

There are two spellings and the document blesses both:

- `Input::new(InputSnapshot::new())` — "A tick in which the player did nothing",
  and this is what `scripted_player.rs` uses to prime its first tick.
- `Input::new(SnapshotBuilder::new().first_tick_snapshot())` — a builder with
  nothing recorded.

I used the second because my controller already had a `SnapshotBuilder` and I
wanted the idle session to go through the identical path. Both are one line and
they are the same value. This is a mild scratch against "one way to do
everything" rather than a real cost.

## F-8 — the `--verify` skeleton and the worked example disagree about failure

The document's skeleton is:

```rust
verify::run();                 // ticks, asserts, prints "verified ..."
return ExitCode::SUCCESS;      // or FAILURE, if an assertion reported one
```

with the interesting half in a comment. `prototype_kit`'s `verify.rs` resolves
it the other way, with `fn fail(..) -> !` that calls `std::process::exit(1)` on
the first problem.

Those two are not the same design, and the document elsewhere argues hard for
the one the example does not implement:

> A failing assertion has to report the numbers it judged. […] the assertion is
> the only instrument there is, so a message that says only *this is wrong*
> costs a whole cycle to turn into a diagnosis.

An instrument that stops at the first bad reading costs a cycle per fault for
the same reason. I built the third thing — a `Checks` accumulator that records
every failure, prints them all in the engine's four-part `message` shape, and
returns `ExitCode::FAILURE` — and it paid for itself immediately: when I
deliberately broke the paddle's span test, the run reported six problems and
the precisely diagnostic one ("a ball that misses the paddle is counted as a
hit") was fourth. Under `prototype_kit`'s shape I would have seen only the
first, which was "no one won the match".

Not a gap in the document so much as a place where the document is right and
the example it points at is not.

## F-9 — the speed ceiling means nothing in the game ever exercises its own swept collision test

Not the document's fault; recording it because it is the most interesting thing
the run found about verification.

The document is emphatic that there is no `Rect::sweep`, that the eight lines
are the game's to write, and that a fast small ball tunnelling is "the first
thing that bites". I wrote the sweep. I also capped the ball at 33 units/s,
which at 60 Hz is 0.55 units of travel against a paddle 0.7 units thick — so
the ball *cannot* tunnel, and therefore the sweep never does anything the naive
position test would not.

I found this by mutation-testing my own verification: replacing the swept test
with a position-only one passed the entire session. The check that the ball
never left the table passed, the match still finished 5-0, every drawn-frame
assertion held. The sweep is real safety and the run could not see it.

The fix is to ask the function its contract directly rather than hoping play
reaches the case — one tick of travel eight units long across a paddle 0.7
thick, plus the two negative cases (past the end of the paddle, and leaving
through the same face). That is `check_the_swept_test` in `verify.rs`, and it
is the only check in the file that is not about a played match.

Generalising: everything the document teaches about verification is about
observing a *run*, and a run only exercises the states it reaches. The safety
margins a game is built on are exactly the states a correct game never reaches.

## F-10 — an opponent that reads the ball every tick is unbeatable at any believable speed, and the arithmetic is not obvious

A game-design finding rather than an API one, but it is where the tuning cycles
went, so it belongs in an honest account of what the exercise cost.

My first machine paddle chased the ball's current y at 18.5 units/s against a
player at 26. It went 2-0 up over sixty seconds with rallies of thirty touches,
and every knob I reached for made it worse. The arithmetic is why: the ball
crosses a 30-unit table in about a second, the paddle only has 14 units of
travel to cover, so *any* speed above about 14 units/s reaches everything.
Dropping it far enough to miss makes it visibly asleep between points.

The knob has to be a reaction *time*. Mine reads the ball every twelfth tick
and drives at what it last saw, which at the ball's top speed is more than a
paddle's length of lag — so what beats it is hitting hard and steep, which is
what beats a person. One constant, `MACHINE_REACTION`, and it took three tries
to work out that a constant of that *kind* was what was needed.

## F-11 — the window is unverified on this machine, and that is the honest status

`cargo run -p jidousha --example pong` on this container prints:

```
[jidousha] no display to open a window on
  os error at […]: neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set.
  likely cause: the program is running headless — over SSH without X forwarding,
  in a container, or on a CI runner
  fix: run it on a machine with a desktop session, or use jidousha::headless […]
```

which is a genuinely excellent error message and exactly the right four parts.
But it means **I have never seen this game.** "It runs in a window" rests on
the windowed path being the Quickstart's four lines unchanged, and on 1,263
recorded frames of geometry that I have read as numbers. Everything about how
it *feels* — whether 26 units/s is a nice paddle, whether the machine's
twelve-tick stutter looks like thought or like lag, whether 0.30 alpha on the
score reads as "behind" or as "smudge" — is inference from the transcript.

The document is unusually good about this. Its warnings that a low alpha reads
much brighter than the number suggests, and that a wrong character draws as a
correctly-sized box no assertion can see, are both written by someone who knows
the reader cannot look. I took both: the field markings are lower than felt
right, and every literal is checked against the printable range.

## Things the document got right that I would otherwise have got wrong

Listing these because a friction log that only lists friction misrepresents the
document badly.

- **Y is down.** Stated three times, including once in the reference for
  `Transform`. I never once got a sign wrong.
- **Collisions only happen at tick boundaries, and there is no sweep.** Told me
  what to write and why the helper is absent, in a paragraph that names the four
  things a sweep helper would have to answer next. I wrote the eight lines
  without ever wondering whether I was missing an API.
- **`ctx.circle` is sixteen quads.** The worked union-of-wedges assertion is
  copied almost verbatim into `checks.rs::disc_at`. Without it my ball check
  would have been "a quad the size of the ball is at the ball", which is false
  for every circle ever drawn, and I would have spent a cycle finding out.
- **`contains_rect` is closed, `contains` is half-open.** Both used, for the two
  different questions, on the strength of one paragraph.
- **`find_resource` for `Input` and `Camera`, and "on the way into tick 1 there
  is nothing to look at".** My controller reads the world at the top of every
  tick including the first. It would have panicked on tick 1 without that
  paragraph.
- **`SnapshotBuilder` sends events, not states.** The sentence "Building a
  one-tick script per tick instead puts a press edge on *every* tick" is the
  difference between a working controller and a paddle that restarts a finished
  match forty times a second.
- **Check the screens the run never reaches.** My controller wins 5-0, so the
  losing banner is drawn exactly zero times in the match. Three lines per
  screen, straight from the document.
- **Print the numbers, not the conclusion.** Every failure message in
  `verify.rs` quotes what it looked at. "no one won the match" prints the score,
  the longest rally, the top ball speed and the number of returns by each
  paddle — which is enough to tell a slow ball from a broken paddle without
  running anything again.
- **A game of pure shapes needs no asset story.** Held exactly. There is no
  `Assets` resource anywhere in this game and I never thought about loading.
- **A game does not close itself.** Saved me looking for `App::quit`. The end of
  a match is a state with a way out of it, which is better design anyway.

## Things I wanted to look up in `src/` and did not

- Whether `Time::tick` inside the first Update is 0 or 1 (F-6). Answered by not
  needing it.
- Whether `FrameRecorder::transcript` was really all-frames or whether I was
  holding it wrong (F-1). Answered with `wc -l`.
- Whether a `ctx.line` quad extends past its endpoints, since my table border
  sits exactly on the goal lines and a quarter-unit of overhang would have put
  it off camera. Answered from the transcript: `quad (-17.000, -9.110)
  (17.000, -8.890)` for a line from `(-17, -9)` to `(17, -9)` at thickness
  0.22 — the thickness goes perpendicular only.
- Whether `FrameRecord::covering` tests the rotated quad or its bounding box.
  The document says exact rotated containment; the sixteen ball wedges behaved
  as it says, so I believed it.
- What the `Rng` sequence does across a `Serving` state that is entered but
  never resolved. Never came up, because the serve draws exactly once.

None of these sent me to the engine's source.

## Verdict

I did not get blocked, and I did not need the source. The document was enough,
and it was enough by a wide margin — the substantive costs of this run were
F-2 (a controller that optimised onto the edge of feasibility) and F-10 (an
opponent that needed a reaction time rather than a speed limit), and only the
first of those is something a document could have prevented.

The four things I would change in it:

1. Fix `FrameRecorder::transcript`'s description, or the function (F-1).
2. Add "constrain to the shots you can actually make, with margin, *then*
   optimise" to the play-to-win paragraph (F-2).
3. Say that a `Draw` system does not need the two-pass `Vec`, and make
   `prototype_kit` stop doing it (F-5).
4. Say somewhere that a run only tests the states it reaches, so the margins a
   correct game never reaches need their contracts asked directly (F-9).

And one thing I would add to the engine if it were mine to add, which it is
not: a layer or depth on `DrawnQuad`, so that draw order is something a
verification can see at all (F-3).
