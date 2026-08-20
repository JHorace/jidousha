# E0 run 10 — Pong

Author: a game author, not an engine author. Read `docs/api/jidousha-api.md`,
`docs/api/jidousha-testing.md`, `docs/api/jidousha-controllers.md` and
`crates/jidousha/examples/`. Read nothing else.

Written as it happened, in order. Nothing softened.

## 0. Before writing a line

The API document is *visibly* aimed at this exercise. Sweeping collision, the
paddle-thickness-caps-speed coupling, the pre/post-move ordering decision, the
"write `opponent_target` and `rebound` as free functions now" paragraph, the
worked circle-bounds assertion, `prototype_kit`'s doc comment saying in so many
words "after this, can an agent make Pong?" — all of it is pre-aimed. So this
run starts from a much better place than a general game would, and that is worth
stating plainly, because it means friction I *don't* hit here is not evidence
the document would carry a different game.

Three documents, 3,268 lines, read end to end before the first line of code.
That is the honest cost of entry and it was worth it: almost everything below
that I got wrong the first time was something one of them had already told me.

## 1. Writing the game — what the document did not say

**`Vec2::signum` and `f32::signum` answer `1.0` for zero, and I had to check.**
The `vec2_tour` file says it for `Vec2` ("Note it answers 1.0 for zero") and the
opponent's aim is written on `f32::signum`, which the tour does not cover because
it is not a `Vec2` operation. It behaves the same way; I confirmed that from
Rust's own docs, not from anything here. Small, but it is load-bearing: the whole
reason the opponent's lean does not judder is that `signum` never returns 0.

**`Time::fixed_dt` in a `Startup` system.** `Startup` runs *inside* the first
tick and `Time` is inserted before it, so `world.resource::<Time>()` is safe
there — the resource table says so. I still hesitated, because "runs inside the
first tick" and "inserted before the first tick" are two sentences in different
sections and I had to put them together myself. Nothing went wrong.

**Nothing said whether `ctx.text`'s `at.y` is the top of the glyph *cell* or the
top of the tallest glyph.** "laid out from its top-left corner" and "each exactly
`size` tall" together imply the cell, so a line drawn at `y` occupies `y ..
y + size`. I bet on that for every vertical layout number in the game and it held
— the recorded quad bounds agree exactly. It would have cost a cycle if it had
not, and one sentence would settle it.

**The line spacing of a multi-line block is `size` exactly, with no leading.**
Also inferred rather than stated: "an N-line block occupies `N * size`" only
constrains the total. I ended up drawing my two-line banner as two separate
`ctx.text` calls anyway — the document *requires* that, for centring — so the
question stopped mattering. A game drawing a genuine multi-line block would still
have to guess.

## 2. Tuning the game — where the real time went

This is the bulk of the run, and none of it was about the engine.

**The first opponent I wrote was unbeatable, and the API document told me so
before I ran it.** *When all three are healthy and the game still will not play*
gives the arithmetic: `opponent_speed * crossing_time >= the interval it has to
defend`. I did that sum on paper against my first constants — a court 28.4 units
wide for the ball, 1.36 s to cross at serve speed, an opponent covering 23 units
of a 17-unit interval — and it failed by a mile. Rewrote the constants before
writing a line of the check. That paragraph saved a whole cycle and is the single
most valuable thing in the three documents.

**The second opponent produced exactly the degenerate rally the controllers
document warns about, and only the middle controller could see it.** Three
players, three lines:

```
rollout: 5 - 0 after 2364 ticks, longest rally 8
chaser:  0 - 0 after 2400 ticks, longest rally 43   <- the game, not the player
idle:    0 - 5 after 465 ticks
```

The good controller won 5–0 and would have let me ship it. The chaser held one
43-touch rally for the entire session. `Write three players, not one` is not
advice, it is the instrument; with one controller I would have shipped a game
nobody can play and had no way to find out.

The fix was a *game* change — the opponent now leans into the shot, standing
three quarters of a reach off the ball so no return it plays is ever flat — which
is the document's point that a mediocre controller sends you to tune the wrong
half of the program. Here the numbers sent me to the right half immediately.

**Then four more rounds of constants, all of them measured rather than guessed.**
The scores after each round:

| change | rollout | chaser | idle |
|---|---|---|---|
| first constants | 5–0 | 0–0 (43-touch rally) | 0–5 |
| player 27 → 20, opponent leans 0.75 | 5–0 | 3–0 | 0–5 |
| opponent 15 → 18 | 3–0 (unfinished) | 1–1 (unfinished) | 0–5 |
| paddle 0.9 → 1.1 thick, ball 34 → 42, ramp 1.07 → 1.12 | 5–0 | 1–5 | 0–5 |

The third row is the interesting one: balanced, and *nobody finished a match in
2400 ticks*. Long rallies read as "balanced" on every number except the one that
matters. What fixed it was raising the ball's top speed past what either paddle
can track — and the API document's coupling paragraph is what made that safe to
do: **thicken the paddle first, raise the speed second.** I would have raised the
speed alone.

**Friction, and it is the engine's:** every one of those rounds is a recompile,
because the constants are `const`s in the game. That is correct Rust and the
right thing for determinism, and it still means a tuning sweep is a shell script
that rewrites the source file with a regex between runs, which is what I did.
Nothing in the surface offers a knob — no `GameConfig` passthrough, no way for
`--verify` to take a parameter. I am not sure it should; `tools/verify` takes no
arguments either. But "simulate rather than solve" plus "a tick is cheap" invites
sweeping, and sweeping is the one thing the shape makes awkward.

## 3. Writing the check

**Two clippy lints bite that the API document's list of five does not have, and
one of them contradicts advice the repository gives elsewhere.**

- `clippy::neg_cmp_op_on_partial_ord`, denied, rejects `!(travel > 0.0)`. That
  spelling is *deliberate* NaN safety — `travel <= 0.0` is false for NaN and lets
  a poisoned velocity through as a contact — and `prototype_kit/checks.rs` spends
  a paragraph explaining why its own comparisons are spelled out for exactly that
  reason. The lint forbids the same idiom in game code. The fix is fine (name the
  conditions, negate the conjunction once), but I met it as a build error after
  the game was written, which is the bad place to meet a rule for the first time
  — the document's own words about the five it does list.
- `clippy::question_mark` rejects `let Some(x) = f() else { return None; }`. The
  API document teaches `let else` as the idiom that replaces `unwrap`, and this is
  a case where it is wrong. Small, and it costs a cycle if the first you hear of
  it is `-D warnings`.

Both belong in that list. Neither is guessable from the prose.

**The `--verify` verdict token and `tools/verify`'s `capture:` line are stated
precisely and both worked first time.** Worth saying, because they are the kind
of thing usually learned by failing.

**`FrameRecorder::draw` returning the owned frame is the detail that made the
staged screens easy**, exactly as the document claims. Building an end screen
after inspecting the run's last live frame is one function; with `frames()` it
would have been a clone dance.

**The document's warning that a staged frame is corrective, not additive, is
worth its paragraph and I still nearly walked into it.** My band check parks the
ball under the hint line and asks what is in front. It runs *after* the two
staged end screens, and the `Round` resource was still saying `Over` — so the
frame would have carried the banner and the answer would have been a banner
glyph. I set the stage back to `Rally` because the document told me the failure
in advance. Without that paragraph this would have been twenty minutes of
re-reading correct drawing code, which is what it says it cost somebody.

## 4. What took more than one attempt

**The end screen's centring check, twice.** First version counted glyph bands
and demanded exactly two — but the score is still drawn on an end screen, so
there are three. Second version filtered bands by height to drop the score, and
then broke the moment I nudged the banner up the court: the filter was a
statement about where the banner *was*, so the check was about itself. Third
version finds each line by its glyph count against the string the game says it
draws, and does not care where it sits. That is the shape the document argues for
in the colour section — state the requirement, not the constant — and I had to
arrive at it twice by hand before applying it here.

**The hint line was 0.03 world units from running off the bottom of the
screen.** It passed. It would have passed for a long time. `at` is the top of the
glyph cell and a line is exactly `size` tall, so a hint at `COURT.y + 0.35` with
`size` 0.62 ends at 9.97 against a camera reaching 10.0. The off-screen check
would have caught it the day anything moved, which is the check working — but I
only noticed the margin because I read the transcript, not because anything said
so. A "how close was the nearest quad to the edge" number in the summary would
have made it visible; I did not add one and probably should have.

## 5. The picture found two faults nothing else could

The API document says a capture answers what no assertion reaches. It did, twice,
on the first end screen I looked at:

- **"YOU WINS 5 - 2".** One `format!` with the side's name and a literal `WINS`.
  Right glyph count, right width, perfectly centred, every character printable —
  every check in the file passed, and the picture is the only reason the game does
  not ship saying that.
- **The second banner line ran the width of the court and through both paddles.**
  Inside the camera, so the off-screen check was happy. "On screen is not in the
  right place", exactly as written, and the only instrument that saw it was my
  eyes on a PNG.

Both fixed. Both are the document being right about its own advice.

## 6. Breaking the game on purpose

Twenty-three mutations, one at a time, each reverted with `git checkout`. The
harness makes a search-and-replace that matches nothing an error rather than a
no-op, and tells a failed build apart from a failed check, because the document
says both are silent when you get them wrong — and both would have been.

**22 of 23 caught.** The escape is not a hole: setting the sweep's
`approaching` guard to `true` changes no behaviour, because the other two
conditions already imply it (the only way past them travelling the wrong way is a
zero-length step at the plane, which divides to NaN and fails the reach test).
The mutation that actually reduces the sweep to a position test — `at = 1.0` —
is caught by the contract test, which is the point. I left the redundant guard in
and said so at the site.

Two things the round changed:

- **`SERVE_PAUSE` could go to zero and the whole run passed.** A game that
  re-serves on the tick the point lands is a game nobody can read the score of,
  and nothing measured it. The fix is a check about a person's eye — the ball sat
  still for at least twenty ticks — rather than about the constant.
- Everything else held, including the two the document warns are usually loose:
  the swept contract, and the paddle drawn 45% out of position (caught, because
  the check asserts on bounds rather than on something being there — I wrote it
  that way only because the document told me the naive version passes).

The mutations that produce the *most* readings are the interesting ones. "The
rebound does not turn the ball round" reports six problems, and the diagnostic
one is not first. An instrument that exited on the first would have said "the
game cannot be won" and nothing else.

## 7. Things I wanted to look up in the engine's source, and did not

- **How `ctx.text` positions the second line of a block.** Not stated; inferred
  from "an N-line block occupies `N * size`" and confirmed against recorded quad
  bounds. I wanted to read the layout function. I did not; the frame answered it.
- **Whether `Rect::from_center_size` handles a negative `size`.** The document
  says `from_min_size` and `from_center_size` "cannot produce" an inverted rect
  "from a non-negative `size`", which carefully does not say what a negative one
  does. I never passed one, so it never mattered.
- **What `Camera::viewport` does under `run` if the window is resized mid-frame.**
  The document says it is stamped every frame after Update and before Draw, which
  is enough. I wanted to see it because my layout is in constants and the answer
  decides how badly a resize breaks it. It does not need reading: the document
  states the consequence, and I stated the trade in the game.
- **`Rng::next_f32`'s distribution at the endpoints.** `0.0..1.0` — half open, I
  assumed. My serve angle is `roll * 2.0 - 1.0`, so an inclusive 1.0 would just
  mean an occasional exactly-maximum angle. Harmless either way, so I did not look.

None of these blocked anything. All four are one sentence each.

## 8. What I would ask for, in order

1. **Add `neg_cmp_op_on_partial_ord` and `question_mark` to the five lints.** They
   bite, they are not guessable, and the first contradicts the NaN-safety idiom
   the repository teaches elsewhere.
2. **One sentence on `ctx.text`'s vertical metric**: the `at` is the top of the
   glyph cell, and line N of a block starts at `at.y + N * size`. Everything else
   about text is specified to the character; this is the one gap, and every
   vertical layout number in a game rests on it.
3. **A worked note on tuning.** "A tick is cheap, so simulate rather than solve"
   is the best advice in the three documents and it ends where the interesting
   part starts: a game's constants are `const`s, so a sweep is a script that
   rewrites the source between runs. Either bless that (it is what I did, and it
   works) or say that a game which expects to be tuned should put its numbers in a
   resource its `--verify` mode can set. Right now a reader has to invent one of
   the two.
4. Nothing else. The three documents were enough to write a working, checked,
   playable game without opening `src/` once, and the two places I nearly went
   wrong — the collider-ordering decision and the degenerate rally — are both
   places the documents got there first.

## 9. What was built

`crates/jidousha/examples/pong/` — `main.rs` (the game), `controller.rs` (three
players), `verify.rs` (the check), `checks.rs` (the instrument), `capture.rs`
(the picture).

```
verified pong over 5036 ticks of play and 2083 recorded frames, 0 problems
  rollout: 5 - 0 in 2080 ticks (34.7s), longest rally 6, top speed 42.0
  rollout: met 13 of 13 approaches
  rollout: planned 909 returns aimed to land 2.26 from the opponent
  rollout: shots landed 0.01 from where they were planned to
  chaser: 1 - 5 in 2491 ticks, longest rally 6 - the one that says it is playable
  chaser: met 16 of 21 approaches
  idle: 0 - 5 in 465 ticks - the game can be lost
  opponent: returned 12 of the 17 balls that reached it
  ball: longest tick 0.700 against a paddle 1.10 thick, shortest pause 45 ticks
```

Also played in a window under Xvfb: keyboard reaches it, a match runs to a
winner, and `space` starts a new one.
