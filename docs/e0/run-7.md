# E0 run 7 — Pong

Written as I went. Entries are in the order I hit them, not sorted by severity,
and nothing has been softened afterwards. The game is
`crates/jidousha/examples/pong/`; it verifies in 3,092 ticks, 5-0, and catches
all seventeen faults I injected into it on purpose.

I read `docs/api/jidousha-api.md`, `docs/api/jidousha-testing.md` and
`crates/jidousha/examples/`. I did not open `crates/*/src/`, `docs/internal/`,
`docs/adr/`, the maintainer docs, or any other file in `docs/e0/`.

---

## Before writing a line: what the two documents left me guessing

### F1. `Camera::viewport` under `run` is undefined from the game's side

The API document says the camera is `height` world units tall "and as wide as
the window's aspect makes it", and `Camera` carries a `viewport: PhysicalSize`
field with a `Default` of 1280x720. What it never says is **who writes that
field during a windowed run**. If the driver overwrites it on resize, my
`Startup` value is only the opening one and `visible_bounds()` is honest after
the first frame. If it does not, a resized window silently draws against a stale
aspect and every layout I compute from `visible_bounds()` is wrong.

I guessed "the driver owns it, mine is the opening value", set it to match
`GameConfig::window_size`, and then made the game not depend on the answer:
every layout constant is my own `COURT` half-extents, and the camera is only
ever asked whether things are inside it. That is a defensible design, but I
chose it to route around not knowing rather than because it was better.

This is also the one thing I most wanted to open `src/` for.

### F2. Nothing says whether `Input` exists during `Startup`

The resource table says `Input` is inserted by `run` "before every Update tick"
and is absent "not before the first tick". Concepts says `Startup` runs *inside*
the first tick, before `Update`. Composing the two gives "Startup never sees
Input", which is almost certainly right, but it is an inference across two
sections rather than a sentence. A row in that table saying so would cost eight
words.

### F3. There is no circle *outline*, and no prose saying so

`ctx.circle` fills. A Pong centre marking is an outline or a dashed line, and
the vocabulary has neither, so it is either a low-alpha filled disc or shapes I
assemble myself. The document is explicit that the vocabulary is closed, so this
is a boundary rather than a gap — but there is a *reason* written next to
`Rect::sweep`'s absence and nothing next to `circle`'s. I drew the centre line
as a column of `ctx.rect` calls.

### F4. `PhysicalSize` is spelled two ways, and the document says it is not

The API document is emphatic: "A game spells them from the prelude and nowhere
else", and cites E0 run 4 finding two worked examples disagreeing about which.
`PhysicalSize` is in the prelude *and* in the `jidousha::testing` reference, and
`prototype_kit/verify.rs` imports it from `testing`. I copied that, and got an
unused-import error because the prelude glob already had it. Thirty seconds, but
it is exactly the class of thing that section claims to have settled.

### F5. The directory-example convention is undocumented

`examples/pong/main.rs` is picked up with no `[[example]]` entry in
`Cargo.toml`. I inferred that from `prototype_kit` existing rather than from
anything written down. Fine once you have seen it; invisible before.

---

## The long one: four rounds on an opponent nobody could beat

This is most of the run, so it gets the space. The short version: **the testing
document's central worked lesson is stated as a conclusion and is actually a
property of the opponent that run happened to write, and following it cost me
two rounds and two wrong changes to my game's constants.**

### F6. "Take the return that lands furthest from the middle" is not general

The document spends four paragraphs on controller design and lands on a
specific, measured prescription: aiming away from where the opponent is
*currently* standing is worthless against an opponent that drifts back to the
middle, and replacing it with "try every return this paddle can produce, take
the one that lands furthest from the middle" took a match from 79 seconds to 43
with the game byte-identical.

I wrote exactly that. My first full match was **0-0 after 3,600 ticks with a
54-touch rally** — the precise symptom the document describes.

"Furthest from the middle" works for an opponent that *drifts to the middle*.
Mine *chases the ball*, which is at least as natural a first opponent to write.
Against a chaser it is close to the worst available objective: the shots that
land furthest from the middle are the steep ones, and a steep shot gets there by
rebounding off a wall straight back into the path the chaser is already
following. My controller aimed every single return to within a tenth of the wall
and the opponent never had to stretch more than 2.09 units.

What generalises is not the prescription but the thing under it: **score a shot
against where the opponent will actually be**. For a drifting opponent that
reduces to "furthest from the middle"; for a chasing one it does not reduce to
anything, and you have to run the opponent's own rule forward beside the ball's.
The document presents the reduced form as the lesson.

That has a design consequence for the *game*, which no document mentions: to
score shots this way the game has to expose its opponent's decision as a pure
function (`machine_push`, `step_paddle`) rather than as a branch inside a
system. I had already done that for the ball (`travel_one_tick`), because the
document's "simulate rather than solve" advice pushed me there. It did not occur
to me to do it for the opponent until the controller needed it.

### F7. "Met N of N approaches" is one contract and a controller has three

The document is insistent, and right, that a controller must check its own
contract, and gives the number: `met 18 of 18 approaches`. It then says that
line is what lets a run stop suspecting its driver and go do the arithmetic.

My controller reported **27 of 27** while producing a 0-0 match. Meeting a ball
and threatening with it are different contracts, and the document's number only
covers the first. A controller can be perfect at returning and useless at
attacking, and that produces the identical symptom — the long rally at 0-0 —
which is the exact failure the number is supposed to disambiguate.

I needed three numbers before the instrument said anything true:

1. `met N of M approaches` — the document's. Cleared the controller as a
   returner.
2. `planned returns aimed to land X from the opponent` — how good the shots it
   chose were *believed* to be.
3. `shots landed Y from where they were planned to` — whether the shots it
   plans are the shots it produces.

The third is the one that broke the case open, and it is not in the document.

### F8. A correct prediction can be worthless, and nothing warns you

With the opponent modelled properly, my controller planned returns landing 3.9
units from where the opponent would be, against a paddle covering 2.35 — clean
misses, on paper. The opponent was never stretched past 2.09.

The measurement that explained it: **shots landed 7.43 units from where they
were planned to, on a court 17.1 units tall.** The prediction was not
approximately right. It was noise.

The cause is structural and I think it is worth writing down, because it is a
property of Pong rather than of my code. A keyboard paddle moves in steps of
`PLAYER_SPEED * fixed_dt` and cannot stand anywhere in between, so it arrives
within about 0.2 units of where it meant to be. Over the paddle's reach that is
0.085 of contact offset, which is five degrees of bounce angle. Five degrees
over a 28-unit crossing is four units of landing, and the wall reflections fold
that into something with no useful relationship to the intended aim. **The
flight is chaotic in the aim angle, and no amount of care in the prediction
fixes it.**

The document says "simulate rather than solve", and gives the cost argument for
why that is affordable. It never mentions that the simulation's answer can be
exactly right and still useless, or what to do about it. What worked was
minimax: score each candidate by its *worst* outcome across the aim error the
controller knows it has (three samples, ±0.085 of offset), and take the best of
those. That took the match from 0-0 to 3-0 immediately and halved the aim error,
because it stops selecting candidates whose apparent merit is a coincidence of
where the folds landed.

This is the same failure the document's own "constrain, then optimise" paragraph
describes — the optimum sitting on a boundary where any error is a clean miss —
one level up. The document treats it as a fact about paddle geometry. It is a
fact about optimising against a noisy objective, and the paddle tip is one
instance.

### F9. Two of my game's constants were changed for a controller bug

Exactly what the document warns about, and I did it anyway, twice, after reading
the warning. Between the first 0-0 and finding the real fault I moved
`OPPONENT_SPEED` (13.0 → 10.0) and `SPEED_PER_TOUCH` (1.1 → 1.8), on arithmetic
that was internally correct and measuring the wrong thing.

In fairness to the arithmetic: the first change was justified. At 13.0 the
opponent really could cover its whole half during any crossing slower than 25.9
units/s, which a rally only reaches at its very end, and that is a game fault
independent of any controller. The second change was not — it was me trying to
make a game faster to fix a controller that could not aim.

The lesson I would add to the document's is narrower and more useful than
"suspect the controller": **a controller self-check has to be a check the run
performs, on the same numbers, in the same output, or it is prose again.** Mine
was. The thing that failed was that I only had *one* number and it was healthy.

### F10. My own requirement check was too lenient twice, in the same way

The document's advice to pair `assert_eq!(drawn, the_constant_that_drew_it)`
with a check that states the requirement in numbers is the best thing in either
file, and I wrote one for winnability. It passed for two consecutive opponents
that could not be scored against inside a minute.

Both times it was too lenient for the same reason: I stated the requirement at
the *most favourable* operating point. First against `TOP_SPEED`, which a rally
only touches at the very end. Then against the wrong distance — I had the
opponent needing to reach the ball's line, when a paddle defends everything
within its own half-height plus the ball's radius and gets a third of the court
it defends without moving.

The generalisation the document could make: a requirement stated at a boundary
case is a requirement about a case that hardly ever happens. State it where the
game actually operates.

I eventually replaced the derived check with a **measured** one — the opponent
must return at least half the balls that reach it — because the derived version
assumes a precision the game does not permit (F8), and failed for a game whose
rallies ran to eighteen touches.

---

## Mutation testing

### F11. I hit the `git checkout --` trap the document warns about, in a shape the warning does not cover

The document says: commit before you start, because `git checkout -- <file>` is
the natural way back from a mutation and it throws away the *check you wrote ten
minutes ago*.

I committed. I still lost work. My harness reverts `main.rs` after each
mutation, and between two rounds of mutations I fixed something in `main.rs`
without committing it. The warning's framing pointed me at the wrong file to
worry about — I was watching `verify.rs`, because that is where "the check you
wrote" lives.

It also failed silently in a second way, which is mine rather than the
document's: a Python string-replace that does not match writes the file back
unchanged and reports nothing. So I spent a round believing my fix had not
*worked* rather than that it was not *there*.

### F12. Three of seventeen escaped, and each named a real gap

- **A wall that clamps instead of reflecting** keeps the ball on court, lets the
  match finish, and passes "the ball never went through the top or bottom" —
  because a clamped ball sits exactly *on* the wall rather than past it. Caught
  now by asking `travel_one_tick` its contract directly, which is the
  document's own advice about the swept test applied to a second function it
  does not mention.
- **The winner's banner moved from the UI band to the field band** was invisible
  to every assertion. This one deserves its own entry, below.
- **A field marking drawn over the ball** is not a glyph either way, so my band
  check — which asked "is the front-most quad text?" — could not see it. It had
  to compare tints.

### F13. Draw bands are unobservable unless the game happens to overlap them

The document says a frame carries the order and not the `Depth` that produced
it, deliberately, and that `covering(p)[0]` is what a player sees. Both true.
What it does not say is the consequence: **where submission order already agrees
with the layers, no assertion over drawn quads can see a layer at all.** My
banner was submitted last, so moving it to the bottom band changed nothing.

Nothing my game drew put two bands over the same point. So the layering was
completely untested, and no amount of care in writing assertions would have
changed that.

The fix was to change *the game*: I had been hiding the ball on the winner's
screen so the banner had the court to itself, and I stopped, so the banner sits
over the ball and the overlap exists. It is a better screen anyway, but I want
to be clear that I changed a game's appearance to make a property of it
testable. That is a real design consequence of "a frame does not carry Depth",
and the document should probably say so, because the alternative reading — "just
assert on `covering()`" — quietly does nothing.

---

## Smaller things

### F14. Nothing states that `World::query` iteration order is stable

Determinism is described as sacred, and the conventions ban introducing
iteration-order dependence into simulation — as a rule *for the engine*. My game
reads "the ball" as `query::<(&Transform, &Ball)>().next()` and collects paddles
into a `Vec`, both of which assume `query` yields entities in a stable order
across ticks and across runs. My replay check passes, so it does hold. I could
not confirm it from the document, and it is the kind of thing a game leans on
everywhere without noticing.

### F15. Sub-tick ordering of two moving things is a game's problem, unmentioned

Concepts writes out the swept test for a ball against a static plane. A paddle
is not static — it moved earlier in the same tick. Nothing discusses what a game
should do about a collider that is itself in motion. I treated the paddle as
stationary within the tick, using its post-move position, which is the usual
prototype simplification, and documented it at the site. That is a fine answer;
I just had to decide it was fine on my own, and the document is otherwise very
willing to tell a game which shape to write.

### F16. The `--verify` summary convention is precise and worth the precision

The verdict token, one indented line per fact, the transcript as evidence rather
than output, the `capture: ... written to ...` wording that `tools/verify`
parses. All stated exactly, all easy to get right first time. No friction; noting
it because it is the part of the process that cost me nothing.

### F17. What the documents got right enough that I never thought about it

Recorded because a log of only pain is its own kind of lie:

- The `collapsible_if` warning names the fix (a let-chain). I hit it once and
  fixed it in ten seconds.
- `Radians::from_degrees` being `const fn`, and the explanation of why, meant I
  wrote `MAX_BOUNCE` correctly on the first try without meeting
  `approx_constant`.
- The circle-is-sixteen-wedges contract, with the union-of-bounds recipe written
  out. I would not have guessed the shape of that assertion.
- `TextStyle::width_of` measuring only the widest line. I wrote one `ctx.text`
  per line from the start because the document told me to, and never saw the
  crooked banner.
- The printable-ASCII check. One line, and it is the only thing that can catch a
  stray em dash — which my mutation testing confirmed.
- The Y-down convention, stated three times in three places. I never once got a
  sign wrong.
- "A tick is cheap": my whole `--verify`, including a full match, a replay of
  it, four staged frames and a GPU capture, runs in about two seconds in a debug
  build, with a controller rolling nine candidate futures forward on every tick.
  Designing for a slow tick would have cost me the controller entirely.

---

## Where I ended up

- 17 of 17 injected faults caught.
- `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`
  clean.
- One thing I could not check: **I have never seen this game in a window.** This
  container has no display, so `run` returns `NoDisplay` — correctly, with a
  four-part message that told me exactly why. Everything I know about how the
  game looks comes from the captured PNG and the frame transcript. The PNG looks
  like Pong.
