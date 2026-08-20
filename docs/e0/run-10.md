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
