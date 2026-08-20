# E0 run 9 — Pong

Built `crates/jidousha/examples/pong/` from `docs/api/jidousha-api.md`,
`docs/api/jidousha-testing.md`, `docs/api/jidousha-controllers.md` and
`crates/jidousha/examples/`. No engine source, no `docs/internal/`, no
`docs/adr/`, no earlier run log, no `conventions.md` or `agent-practices.md`.

Outcome: it runs in a window and is playable; `--verify` runs three headless
sessions and passes; `cargo fmt --all` and
`cargo clippy --workspace --all-targets -- -D warnings` are clean; twenty
injected faults are all caught.

```
verified pong over 3600 ticks, three players, 0 problems
  rollout: 5-0 (winner Some(Left)), longest rally 8 touches, top ball speed 48.0 units/s
  rollout controller: met 16 of 16 approaches; planned returns aimed to land 2.41 from
                      the opponent; shots landed 0.49 from where they were planned to
  chaser: 1-5 (winner Some(Right)), longest rally 12 touches
  idle: 0-5 (winner Some(Right))
  opponent returned 16 of 21 balls that reached it (76%)
```

Frictions below, in the order they happened. Nothing is softened.

---

## 1. The three documents are ordered wrongly for the one decision that matters

**The single biggest cost of this run.** The router says to read the API
document first and the controllers document "last, and only once a `--verify`
mode needs a player that can win". I did exactly that. The controllers document
then says:

> A controller can only ask where something will be if the answer is a **pure
> function** of the world — `fn opponent_push(&Ball, &Paddle) -> f32` … It costs
> nothing while you are writing the game and is expensive to retrofit, because
> by the time the controller needs it the answer is buried in a `&mut World`.

That is correct, and the reading order guarantees you meet it *after* the answer
is buried in a `&mut World`. My opponent AI, my paddle bounce and my ball step
were all written as branches inside `Update` systems, because that is what the
API document's Quickstart and `prototype_kit` both look like. I then rewrote
three of them into free functions — `opponent_target`, `rebound`, `drift`,
plus `paddle_step`/`paddle_towards` — so the controller could roll them forward.
It was about forty minutes and a restructure of the main game loop, at the point
where the game already worked.

The API document is 1,858 lines and does not contain the words "pure function"
anywhere near the systems it teaches you to write. One sentence in *Concepts*
would have saved the whole retrofit — something to the effect that a game which
will be checked by a controller wants its opponent's decision and its collision
response written as free functions the check can call, rather than as branches
inside the systems that act on them. The controllers document already knows
this; it is filed where a first-time author cannot act on it.

## 2. `InputSnapshot` is named in the API document and is not reachable from it

`docs/api/jidousha-api.md`, in the `Input` reference entry, gives this example:

```rust
world.insert_resource(Input::new(InputSnapshot::new()));
```

`InputSnapshot` is not in `jidousha::prelude`. Confirmed by deleting it from my
`use jidousha::testing::{…}` and rebuilding:

```
error[E0425]: cannot find type `InputSnapshot` in this scope
```

So the API document's own example does not compile against the API document's
own import line. The signature `Input::new(snapshot: InputSnapshot)` is also
listed there, which names a type the game-facing surface cannot obtain. I only
knew where to get it because I had already read the testing document — a game
author who reads only the first document is stuck on a two-line example.

Either the example should not be in the game document, or `InputSnapshot`
belongs in the prelude alongside `Input`.

## 3. There is no way to fork a `HeadlessSim`, and the testing document tells you to

The testing document says:

> So simulate rather than solve: running the game forward and looking is
> allowed, and it is usually both simpler and more honest than a closed form
> kept in step by hand.

A controller cannot do this. `HeadlessSim` exposes `tick`, `draw`, `world`,
`world_mut`, `schedule_debug` — nothing that copies or snapshots a running
simulation, and `World` is not `Clone`. Rebuilding from `headless(…)` and
replaying input to the current tick is O(n²) and useless at tick 2,000 with
thirteen candidate futures per decision. `Recording` replays *input*, not state.

So "run the game forward" resolves in practice to "re-implement the game's step
in the controller and hope it stays in step" — which is the closed form the
sentence warns against. What I did instead is the third option nobody names:
extract the step into pure functions the *game* owns and have the controller
call those. That works, and it is what §1 above should have told me to do on
day one, but it is not "running the game forward" and the document should not
imply the engine offers that.

A `HeadlessSim::fork(&self) -> HeadlessSim` would make the sentence true. Short
of that, the sentence needs the qualification.

## 4. Reconstructing the camera a frame was drawn with is a recipe only an example has

Both documents flag the trap clearly — the recorder's viewport overrides the
`Camera` resource's, nothing writes it back, and a check comparing world bounds
against recorded quads is comparing against the wrong rectangle. Neither
document gives the fix. The fix is:

```rust
let camera = Camera { viewport: HEADLESS_VIEWPORT, ..*sim.world().resource::<Camera>() };
```

I got it out of `prototype_kit/verify.rs`. It is three tokens and it is
load-bearing for every `visible_bounds()` assertion in the file — which the
testing document itself calls "the highest-value check a game of shapes and
text can write". The document that names the trap should carry the two lines
that clear it.

## 5. `schedule_debug()`'s format is undocumented, so the assertion on it is a guess

The testing document says the string holds "every phase and its systems in run
order" and that a check should "assert that the mover you decided goes first
appears before the other in it". It does not say what is *in* the string, so I
wrote `schedule.find("drive_the_player") < schedule.find("move_the_ball")` on
the assumption that system names appear verbatim. They do — I printed it to
check, and it is:

```
schedule:
  Startup (1)
    0. set_the_scene
  Update (6)
    0. restart_the_match
    1. drive_the_player
    …
```

but that is a format nothing promises. My check is written so that a missing
name fails rather than passes, so the failure mode is not a false green — it is
that the one assertion guarding the engine's own "pick an order and hold the
game to it" advice is coupled to an undocumented string, and would go red for a
reason that has nothing to do with the game. One example of the output in the
reference entry would make this an assertion rather than a bet. (`Depth` gets a
whole paragraph explaining what a recorded frame does *not* carry; this gets
none.)

## 6. Every check that measures a drawn thing rebuilds the same union fold

`Rect` has no union, merge or expand. The question "how big is the thing that was
drawn" is unavoidable for `ctx.circle` (sixteen wedges), for `ctx.text` (one
quad per character) and for anything drawn as several primitives — and the
answer is always the same three-line fold over `quad.bounds()`. The testing
document writes it out inline; `prototype_kit/verify.rs` writes it twice; I
wrote a `union()` helper and called it three times (the ball's disc, the score's
two halves, the drawn court).

This is not a v1 boundary in the sense `Rect::sweep` is — there is no game model
hiding behind it, it is `min.min(min), max.max(max)`. If `jidousha::testing` had
`fn bounds_of(quads: impl IntoIterator<Item = DrawnQuad>) -> Option<Rect>` every
check that looks at a circle or a string would be two lines shorter and the
circle recipe in the document would be four lines instead of twenty.

## 7. The capture recipe hands you a discard, then asks you to check what you discarded

The testing document's capture snippet:

```rust
// … A game of shapes and text needs nothing else; the table it returns is not used.
let _ = create_builtin_textures(&mut gpu);
```

and, two paragraphs later:

> That "the ids mean the same thing" step is the load-bearing one … Check the
> ids; the example does.

The second sentence is scoped to games with art, but the check is free for a
game without any, and it is the only thing standing between "a PNG was written"
and "a PNG of this game was written". `let _ =` in the copyable snippet is what
a reader copies. I kept the table and asserted
`textures.resolve(FONT_TEXTURE) == font`; it costs one line and it belongs in
the snippet.

## 8. *Concepts* says Draw runs every tick; under `headless` it does not

> Systems run in **phases**, in this order, every tick: `Startup` once at the
> start of the first tick, then `Update` for logic, then `Draw`.

Under `headless`, `tick()` runs Update only — the `HeadlessSim` reference entry
is explicit ("Run one Update tick" / "Run the Draw phase once") and the truth is
recoverable, but the Concepts sentence is the one a reader carries into the
check, and it is wrong there. This cost me a minute, not an hour, but it is the
kind of thing that makes a first `--verify` mode draw nothing and look broken.

## 9. The tunnelling cap is the binding constraint on how exciting the game can be

The API document says to keep `speed * fixed_dt` under the thinnest thing the
ball must not miss, and asserts nothing else about it. What it does not say —
and what cost two balance passes to work out — is that this makes **paddle
thickness the ceiling on ball speed**, and paddle thickness is otherwise a
purely cosmetic number nobody would think to tune. My first Pong was too slow
to be fun and the fix was not "make the ball faster", it was "make the paddle
0.9 wide instead of 0.7, then make the ball faster". A sentence saying so
belongs next to the sweep advice, because the two constants look unrelated and
are not.

For the record: `MAX_SPEED = 50.0` against `PADDLE_SIZE.x = 0.9` is 0.833 units
of travel per tick — asserted in `verify.rs` against the `fixed_dt` the engine
actually hands the game, as the document asks.

## 10. Three controllers found two faults one controller could not

This is the controllers document being right, recorded because it was right.
The rollout player won 5–0 on the first balance pass I would otherwise have
shipped. What the other two said:

| pass | rollout | chaser | idle | verdict |
|---|---|---|---|---|
| 1 | 2–0, unfinished in 2400 ticks | 1–0 | 1–5 | opponent returns 85%; points take 20s each |
| 2 | 5–0 | 2–0 | 1–5 | **a naive player never loses** — the game has no threat |
| 3 | 5–0 | 1–5 | 0–5 | shipped |

Pass 2 is the one that matters: a rollout win of 5–0 and a chaser win of 2–0
describe a game where a person who simply steers at the ball cannot be scored
against. Nothing about the rollout run says so. The fix was
`PLAYER_SPEED 22 → 18`, `MAX_SPEED 46 → 50`, `PADDLE_SIZE.y 3.6 → 3.4`.

The idle run also found something on pass 1 that I nearly wrote off: a player
doing nothing at all scored a point, because the opening serve is aimed at the
opponent and the opponent's chase can lose a steep one. That is now 0–5 and the
opponent still misses 24% of what reaches it — which is the game, not a fault.

## 11. Twenty injected faults, and the two that got through

Committed first, as the document says, then broke the game on purpose twenty
times with a harness that treats a search matching nothing as an error and a
failed build as not-a-caught-fault. Eighteen were caught first time. The two
that escaped were both the shape the document predicts and neither was
guessable:

- **Paddle twice as long** (`3.4 → 6.8`). Every drawing check compares the quad
  against `PADDLE_SIZE`, so they all moved with it, and the play checks survived
  a paddle half the height of the goal. Caught only after adding a check that
  names the *requirement* — a paddle is between an eighth and a third of the
  court measured off the **drawn** markings, not off any constant the game owns.
  This is exactly the `assert_eq!(what_was_drawn, the_constant_that_drew_it)`
  trap, met in layout rather than in colour. I had guarded the clear colour
  correctly and still walked into it, which is the failure mode the document
  describes verbatim.

- **Deleting the sweep's "already behind the plane" guard.** With it gone,
  `face_crossing` returns a *negative* fraction of the tick — a contact
  extrapolated backwards out of the travel — for a ball on its way to the goal
  line, and reflects it off a paddle it passed two ticks ago. The whole session
  survived: 5–0, 1–5, 0–5, every frame check passing, because by then the ball
  is two ticks from the goal and the extrapolated contact usually lands off the
  end of the paddle. Caught only by asking the function its contract directly,
  with a third negative case. This is the document's "a run only tests the
  states it reaches" and it is worth more than the sentence suggests: the
  guard's *absence* is invisible to a played match, not merely unlikely to show.

## 12. Smaller things

- **The font's `0` is a slashed zero (Ø).** Nothing says so, and a scoreboard is
  mostly zeroes. It is fine, it is just a surprise that only a capture shows.
- **`MARKING` at alpha 0.14 reads as solid grey**, exactly as the alpha-in-linear-light
  warning predicts. The warning paid for itself; recorded as a document that
  was right rather than as a friction.
- **None of the five clippy lints the document names ever fired.** I wrote
  around all five pre-emptively because the document listed them, which is what
  that list is for. `Radians::from_degrees` as a `const fn` was the one I would
  otherwise have got wrong.
- **`With<T>` in the last tuple position still yields its `()`.** Documented, but
  I checked before trusting it; three of my queries are four-tuples ending in a
  filter and the compiler is the only thing that would have said otherwise.
- **`cargo fmt` reflowing Rust string continuations between edits** broke two
  scripted search-and-replaces mid-run. Self-inflicted, recorded because a
  mutation harness that edits source between `cargo fmt` runs will hit it too.

## 13. What I wanted to open the engine's source for, and did not

- The format of `HeadlessSim::schedule_debug()` (§5). Resolved by printing it.
- Whether `FrameRecorder::draw` reads the world as it stands, so a staged frame
  can be built by writing components between draws. Resolved by trying it.
- Whether inserting `Input` before `sim.tick()` is what *that* tick sees.
  Resolved by the testing document's loop, which does exactly that.
- Whether `face_crossing`'s `from.lerp(to, at)` is the same interpolation the
  engine would use anywhere. Never needed — nothing in the engine interpolates.
- What `Time::alpha` would be under `headless`. Never needed; v1 consumes it
  nowhere and a prototype ignores it, as the document says.

None of these were opened. The three documents were enough to finish, and the
places where they were not are §1 through §8 above.
