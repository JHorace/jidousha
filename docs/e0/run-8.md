# E0 run 8 — Pong

Written while building `crates/jidousha/examples/pong/`, from
`docs/api/jidousha-api.md`, `docs/api/jidousha-testing.md` and
`crates/jidousha/examples/` only. Entries are in the order I hit them.

Headline: **the two API documents were enough.** I did not get blocked on the
engine once, and I never wanted to open `src/` to find out what a function
did — only, twice, to find out whether something *existed*. Almost everything
below is either a gap in what the documents list, or a place where the game
(not the engine) was wrong and the documents told me how to find out.

---

## 1. Things the document did not say, that I had to guess

### 1.1 `Depth`'s two constructors do not cover "a layer, and a z inside it"

`Depth::layer(n)` is documented as "the front of `layer`'s band" and
`Depth::default()` is `{ layer: 0, z: 0.0 }`. The struct's fields are public, so
`Depth { layer: layers::UI, z: 2.0 }` works — but nothing says whether writing
the literal is the sanctioned spelling or whether `layer()` is meant to be the
only door. I used `Depth::layer(..)` everywhere and never needed a `z`, so this
cost me nothing; it would have cost something in a game with more furniture.

Related and more confusing: `Depth::layer(n)` is documented as "the **front** of
`layer`'s band", but its value is `z: 0.0` (from `Depth::default()`), and the
Conventions section says **higher `z` draws on top**. So `Depth::layer(n)` is
the *back* of the band by that rule, not the front, unless z is allowed to be
negative. I could not tell which of the two sentences to believe and it did not
matter for a game with three bands and nothing sharing one, but I would not
have been able to build a z-ordered UI from this.

### 1.2 `Rect`'s field names against a Y-down world

`Rect { min: /* Top-left */, max: /* Bottom-right */ }` is documented, and
that is the right choice for Y-down. But `Rect::size()` and
`from_center_size` do not say what happens to a rect whose `min.y > max.y`, and
I built several rects by hand (`Rect { min: view.min + .., max: view.max - .. }`
style) before deciding to always go through `from_center_size`. A one-line
"a Rect is only well-formed with min <= max component-wise; nothing checks"
would have settled it.

### 1.3 Nothing says what `TextStyle::size` measures against a *descender*

The document is unusually precise about text metrics — `size` tall, `size * 7/9`
wide, laid out from the top-left, `\n` the only exception, N lines occupy
`N * size`. That is more than enough to lay out and to assert on, and I used all
of it. What is missing is whether a glyph's quad is the full `size` tall for
every character or only for tall ones. It turns out to be the full cell for
every character including a space — the document does say a space is "a blank
cell of its own" — but I only became sure by asserting a band and watching the
count come out right. My `glyphs_in_band` check depends on it.

### 1.4 `Camera::visible_bounds()` on a resized window is not a game's problem
### but the layout is

The document is very good on the `viewport` trap: `run` stamps the window size
every frame, `headless` stamps nothing, give the recorder the camera's own size
and the question stops existing. I followed that and it worked first time.

What it does not say is what a game should do about a *player* resizing the
window. My layout is constants, so at 16:9 everything sits inside the camera and
`contains_rect` passes; drag the window narrower than about 1.7:1 and the walls
and the hint line go off the sides, and no check I can write sees it, because a
headless run has one viewport. I left it — a prototype — but I would have liked
one sentence telling me whether the intended answer is "derive your layout from
`visible_bounds()` in `Draw`" or "pick an aspect and accept it".

### 1.5 Nothing says whether a `Draw` system may read a resource the game
### inserted in `Startup`

It follows from "Startup has run by then", which the document does say for
`Camera`. But it says it *about `Camera`*, in a paragraph about the driver's
default, and I re-read it twice to convince myself it generalised to my own
`Scoreboard`. It does. One clause — "the same is true of any resource your
Startup inserts" — would have saved the second read.

---

## 2. Things I expected to exist and could not find

### 2.1 A way to end the game

Found, and found *deliberately*: "A game does not close itself. There is no
`App::quit`… `Key::Escape` is listed because games use it to back out of menus,
not because it exits." That paragraph is the reason I did not spend twenty
minutes looking. It is the single most useful kind of entry in the whole
document and there should be more of them.

### 2.2 `Rect::sweep` / `Rect::inflate`

Same: absent, and the document says so, says why, and says what to write
instead ("the plane your body's leading edge touches, whether it was
approaching, whether this tick's travel crossed it, and the fraction of the tick
at which it did"). My `crossing()` is those four tests and it is 20 lines with
comments. I would not have got the "already through it, so it is *leaving* by
this face" case right on my own; it is not in the four-item list but it is the
one that makes a ball stick to a paddle, and I only wrote it because the
Concepts paragraph made me enumerate the cases before coding.

### 2.3 A `Vec2::lerp` I could trust

`vec2_tour.rs` lists it, with a note that the list is hand-maintained and that
E0 run 6 hit exactly this. I used `lerp` for the contact point without
hesitating. That note is doing real work.

### 2.4 A component-wise `Vec2` **sign-preserving clamp of a magnitude**

I wanted "cap this velocity at `BALL_SPEED_MAX`" and reached for something like
`clamp_length_max`. It is not in `vec2_tour.rs`. Per the tour's own instruction
I did not assume the omission meant anything — but I also did not go and read
glam's docs; I wrote `Vec2::new(dir.x, dir.y) * speed.min(MAX)` because my
bounce already reconstructs the vector from an angle and a speed. So this is a
gap I stepped around rather than hit. Reporting it as the tour asks: `glam` has
`clamp_length_max` and the tour does not list it.

### 2.5 A way to ask the engine what the window's aspect will be, before tick 1

There is `GameConfig::window_size` (a `PhysicalSize`) and `PhysicalSize::aspect`,
so in fact there is. I did not notice until late and had already committed to
constants. Not a gap; a thing I did not connect. Worth a cross-reference from
the layout discussion.

---

## 3. Things that behaved differently from what the document implied

### 3.1 "The iterator yields the entity first" — including for filters — is right,
### but `query_mut::<(&mut T, &U)>` reads oddly in a `for` head

Purely cosmetic and entirely documented; I mention it because I wrote
`for (_, transform, paddle) in world.query_mut::<(&mut Transform, &Paddle)>()`
about nine times and got the arity wrong exactly once, on the two-part query
where the tuple is three-wide. The document's four example lines are the fix and
I went back to them.

### 3.2 Nothing surprised me about drawing

`ctx.circle` really is sixteen wedges sharing the centre; the union trick in the
testing document is exactly right and I copied it into a helper. `ctx.text`
really is one quad per character including spaces, which is what let me assert
`in_hint == HINT.len()` on the nose. `covering(p)[0]` really is what a player
sees. The clear colour really is on `frame.plan`. Every single claim in
"Testing your game" held.

### 3.3 One thing was more true than I expected: **tick cost**

"A tick is cheap, and thousands of them are not something to budget for." My
`--verify` runs three matches — 2,107 ticks with 2,107 recorded frames, plus two
more headless matches of 5,575 and 900 — with the controller rolling the game
forward eight to twenty candidate futures deep, up to 400 ticks each, on every
decision. **1.8 seconds, debug build.** I had been planning to sample frames
rather than record them all. I did not need to.

---

## 4. Things that took more than one attempt, and why

### 4.1 The controller was aiming at noise, and the document told me the number
### to print but not the fix

First working run: `met 16 of 16`, `planned … 0.70 from the opponent`,
`shots landed 4.59 from where they were planned to`. The testing document
predicts this shape almost word for word, including the 7.43-on-a-17.1-tall-court
figure, and prescribes minimax over ±1 quantisation step. **I did that first and
it did not help** — the minimax was over a set of candidate positions the paddle
could not occupy in the first place, so all three samples were fictions and the
worst-of-three was noise about noise.

What fixed it, completely, was different: **enumerate only the positions the
paddle can actually stand on.** A paddle driven by a key moves a whole
`speed * fixed_dt` a tick and my steering stops it inside half a step, so its
reachable set is the lattice `current_y + k * step`. Scoring lattice points
instead of arbitrary heights took the aim error from **4.59 to 0.00** in one
edit, and made the minimax unnecessary — I removed it. The planned gap went from
0.70 to 1.6–2.3 at the same time, because the objective was no longer being
computed about futures that would not happen.

I think the document's advice is right for a controller that *cannot* model its
own quantisation, and mine could. But the document presents minimax as **the**
answer ("What works is scoring each candidate by its worst outcome") and I spent
a cycle applying it before noticing that the cheaper and exact fix was
available. A sentence — "if your controller can enumerate the positions its
paddle can actually occupy, do that instead; the minimax is for when it cannot"
— would have saved that cycle.

### 4.2 A staged frame is not staged until *all* of it is staged

I parked the ball on a centre-line dash and asked `covering(p)[0]` which won.
It answered: a quad tinted `PLAYER_COLOR`, 1.556 × 2.0, at (−0.78, −1.6). That
is a glyph of the **winning banner** — the staged frame was drawn after the
match had ended, so `Stage::Over` was still set and "YOU WIN" was sitting over
the middle of the court. Twenty minutes, because I was certain the paddle was
somehow at the origin and kept re-reading the paddle code.

The testing document's staging recipe is `tick(); insert_resource(the screen you
want); draw()`. Mine was `place the ball; draw()` — I staged the half I was
thinking about and inherited the half I was not. The general form of the lesson
is in the document ("a run only tests the states it reaches") but the staging
recipe reads as *additive*, and mine needed to be *corrective*: put the match
back into a rally first.

### 4.3 The glyph accounting failed because the last frame is not a play frame

Same root cause, different symptom, found in the same run: "88 glyphs in all, 2
in the score band and 50 in the hint band". The 36 unaccounted were the banner,
because `last` was the frame on which somebody won. Every geometric assertion in
my file wants a picture of the game being *played*, so `last` is now the last
frame drawn while the ball was live, carried out of the loop with the score and
the positions from that same tick.

The testing document does say to keep the frame you want and that `draw` hands it
back so this composes. What it does not say — and what I would put next to the
`--verify` skeleton — is that **the frame a match ends on is a special frame**,
and a game with an end state should not assert its ordinary layout against it.

### 4.4 The game was broken and only the third player found it

This is the one I am most glad I did. My `--verify` played the match with the
rollout controller, won 5–0, and passed. I had also written a "the match was
one-sided" check, which failed — and my first instinct was that the check was
wrong, because the controller is superhuman and asking it to concede points is
asking the game to be unwinnable. That instinct was right about the check and
wrong about the game.

So I added a third player: a left paddle that simply chases the ball's current
Y, which is roughly what a person does on their first try. It scored **one point
in seven thousand ticks** — a hundred and sixteen seconds for one point. Both
paddles were centring on the ball, both were returning it dead flat down the
middle, and the rally had nowhere to go. This is precisely the degenerate groove
the testing document describes, except that the document describes it as a
*controller* failure ("the game is fine; the controller made it degenerate") and
mine was a **game** failure: an opponent that centres on the ball cannot be
played against by anyone who also centres on the ball.

The fix was in the game — the opponent now meets a descending ball above its own
centre and a climbing one below, so it plays a shot instead of a return. Then
the balance arithmetic the document names (`opponent_speed * crossing_time >=
the interval it has to defend`) said my *paddles were too big*: at 3.2 units on
an 18-unit court, a paddle at 20 units/s covers 36 units of travel in a
crossing, so a perfect tracker on either side literally cannot be beaten. Paddle
down to 2.0, serve up to 19, gain up to 1.12, opponent up to 15.5.

Final gradient, which is the thing I actually shipped and the thing I assert:

| player | result |
|---|---|
| rollout controller | wins 5–0 in 2,107 ticks (35 s) |
| chases the ball | **loses 4–5** in 5,575 ticks |
| does nothing | loses 0–5 |

Three players, one line of verdict each. I would put this in the testing
document: **one controller cannot measure a game's difficulty, because the only
thing it can tell you is whether the game is beatable by that controller.** The
document already argues for a good controller and for a do-nothing run; the
mediocre one in the middle is the one that found the bug, and nothing told me to
write it.

### 4.5 `manual_range_contains`

One clippy error at the end, in the controller: `stand < -LIMIT || stand > LIMIT`
wants `!(-LIMIT..=LIMIT).contains(&stand)`. The API document lists four lints
that "bite in practice" and this is a fifth. It cost thirty seconds because I
was running clippy as I went, as the document says to. Curiously the identical
shape in `crossing()` — `at < reach.0 || at > reach.1` — is not flagged, because
the bounds are tuple fields rather than a symmetric pair.

---

## 5. Things I wanted to look up in the engine's source, and what for

Two, and I looked up neither.

1. **Whether `covering()` includes a quad the point is only on the boundary
   of.** The testing document answers this explicitly ("counts a quad whose edge
   or corner passes exactly through the point, which is what makes asking about
   the centre work at all"). I wanted to check because my ball assertion depends
   on it entirely, and a wrong guess would have made the disc-union check
   silently measure fifteen wedges instead of sixteen. The document was enough;
   I only wanted the source for reassurance.

2. **Whether a tie in `(layer, z)` really falls back to submission order and
   really is stable.** Both documents assert it, in the same words, in two
   places. My band checks depend on it and so does my "the ball is drawn after
   the paddles" reasoning. Again: documented, and I wanted the source only to
   *believe* it. Both times, arranging an assertion that would fail if I were
   wrong was cheaper than looking, and that is the right trade for this exercise
   — but it is worth saying that the pull toward the source was about confidence
   rather than about information.

---

## 6. Two things about the environment, not the engine

- **The windowed build cannot be run on this machine.** `--verify` is green and
  the capture renders through lavapipe, but `cargo run -p jidousha --example
  pong` under `xvfb-run` panics inside `xkbcommon-dl` because
  `libxkbcommon-x11.so` is not installed (only `libxkbcommon.so.0` is). That is
  a missing system dependency, not a game or engine fault, and I did not try to
  work around it. The no-display path itself is fine: without `DISPLAY`,
  `RunError::NoDisplay` prints the engine's four-part message and the program
  exits cleanly, which is exactly what the Quickstart's `eprintln!("{error}")`
  is for.
- So **"it runs in a window and is playable" is asserted by the `--verify` run
  and by the captured frame, not observed.** The window path is three lines of
  `main` that the Quickstart supplies verbatim, and every system behind it is
  the one the headless run exercises, but I want to be honest that I have not
  seen this game in a window.

---

## 7. What the documents got right, briefly, because a log of only complaints
## misrepresents the run

- The `--verify` skeleton, the `verified ` token, the `capture:` line format,
  and the "collect failures, do not exit on the first" rule: copied, worked,
  and the accumulator paid for itself the very first run when five checks failed
  at once and the useful one was fourth.
- "A failing assertion has to report the numbers it judged." Every message in
  my `checks.rs` does, and the run where five failed at once was diagnosable
  from the output alone without opening a file.
- The "state the requirement, not the constant" pairing for the clear colour.
  I wrote both forms and the second one is the only one that would survive me
  changing `COURT`.
- The `DELIBERATE:`-shaped guidance about paddles having already moved this
  tick. I would not have thought about it, my ball would have passed through a
  paddle closing on it occasionally, and — exactly as the document warns — every
  assertion about where things ended up would have passed.
