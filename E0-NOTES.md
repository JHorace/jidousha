# E0 — friction notes from writing Pong against the API document

Written by the game's author, who did not read `crates/*/src/`, `docs/internal/`,
`docs/adr/`, `docs/agent-practices.md` or `docs/conventions.md`. Sources used:
`docs/api/jidousha-api.md` and `crates/jidousha/examples/`.

The game is `crates/jidousha/examples/pong/`. It compiled on the first attempt
and `--verify` is green. That is not the interesting part of this file.

> **This file now carries two runs.** Everything down to "What worked, briefly"
> is run 1, written against the API document as it stood before the F-0xx
> fixes. [Run 2](#run-2--the-same-exercise-against-the-fixed-document) is at the
> bottom: the same exercise, a fresh Pong, against the document as it stands now.

---

## The headline

**The API document is a table of contents, not an API.** Its "Reference" section
is ~90 bullet points of the form `**Rect** — An axis-aligned rectangle, in
whatever space its user is working in`. There are no signatures, no struct
fields, no enum variants, no argument orders, no defaults. It tells you that a
thing exists and roughly what it is *for*. It never tells you how to call it.

Every single call in this game came from `examples/`, not from the reference:
`World::query_mut`, `World::component_mut`, `World::find_resource`,
`Rect::from_center_size`, `Rect::size`, `TextStyle::width_of`,
`Camera::visible_bounds`, `Depth::layer`, `Time::fixed_dt`, `Rng::next_f32`,
`InputScript::hold`, `Input::just_pressed`, `Radians`, `sin_cos`,
`Vec2::normalize_or_zero`, and the entire `NullBackend` ceremony.

**If `examples/` had not been in scope, I could not have written this game from
the document.** Not "it would have been harder" — I could not have made the
first `ctx.rect` call, because nothing states its argument order.

The Quickstart is doing almost all of the document's real work. It is good. But
it is one screen of one game, and the moment you want something it does not do,
the document stops helping and you go and read other examples instead.

---

## Things the document did not tell me, that I had to guess

### `Key` has no listed variants

`**Key** — A physical key, by position on the keyboard rather than by the letter
printed on it`. That is the whole entry. I needed arrow keys for a second
control scheme. The examples only ever use `W A S D Space`. I guessed
`Key::ArrowUp` / `Key::ArrowDown` from the shape of `Key::Space` and it compiled
first try — that was luck, not inference. `ArrowUp` vs `Up` vs `Arrow(Up)` were
all equally plausible.

There is no way to discover the key set from the document. `input_echo.rs`
*prints* held keys at runtime, which is no help to someone writing code in a
container with no display. This is the single most-wanted missing list.

I also wanted `Key::Escape` (to quit) and `Key::Digit1`/`Key::P` (pause) and gave
up on all of them rather than play compile-error roulette.

### `GameConfig` has no listed fields

I know `title` and `seed` exist because `prototype_kit` and `headless_sim` set
them. I wanted to set the window's initial size, because the game's field is a
fixed 34×19 world units and only fits a 16:9-ish window. I could not tell
whether such a field exists. I left `..GameConfig::default()` and wrote a
comment admitting a narrow window will crop the field. That is a gameplay
decision made by ignorance.

### `fixed_dt`'s actual value is never stated

The Concepts section says the timestep is fixed and `Time::fixed_dt` is "the same
number every tick". It never says what number. Everything in my game is in
units-per-second so mostly this did not matter — but `SERVE_DELAY` is a pause
measured in *ticks*, and to pick "about three quarters of a second" I had to
assume 60 Hz. I got 60 Hz from a comment inside `scripted_player.rs` ("90 ticks
at 4 units/second on a 60 Hz timestep: 6 units"), i.e. from arithmetic in an
example's assertion, not from the reference.

Anything a game wants to express in ticks — an animation, a coyote-time window,
a serve pause, an invulnerability period — needs this number.

### `ctx.text` puts depth somewhere different from every other draw verb

```rust
ctx.rect(rect,        color, depth);
ctx.circle(at, radius, color, depth);
ctx.line(from, to, width, color, depth);
ctx.text(at, string, style);          // depth lives *inside* TextStyle
```

Four verbs take depth as a trailing argument; the fifth hides it in a struct.
For a codebase whose first rule is "one way to do everything", this is a wobble,
and the document gives no signatures at all so you only find it by trying.

### `message(...)`'s arguments

The reference entry is `**message** — The failure in the engine's message format
(core.md §9)`, pointing at a document I am not allowed to read. It is in the
prelude, so it is clearly meant for games. I copied the four-argument shape
(what / specifics / likely cause / fix) out of `prototype_kit/verify.rs`. I still
do not know if there is a fifth optional thing or what the field names are.

### Whether `Assets` must exist

The Concepts section spends a paragraph on assets and placeholders, and
`prototype_kit` carefully checks `if world.find_resource::<Assets>().is_none()`
before inserting one. My game touches no assets at all. I could not tell from
the document whether `run()` or `headless()` requires an `Assets` resource to be
present, so I wrote the game without one and braced for a panic. It was fine.
Worth stating explicitly somewhere: **a game of pure shapes needs no asset story.**
That is a genuine strength and the document buries it.

### `Camera::viewport`

Not mentioned in the document at all. `Camera` is described as "What the frame is
looking at" and the coordinate section explains `height` and aspect. But the
headless path needs `Camera { viewport, ..*world.resource::<Camera>() }`
per frame, and I only know that because `prototype_kit/verify.rs` does it. Who
owns `viewport` in a windowed run — the driver? the game? — is not stated.

---

## Things I expected to exist and could not find

### A rectangle overlap test

`Rect` has `from_center_size`, `min`, `max`, `center()`, `size()`. It has no
`intersects`, no `contains`, no `overlaps`. "Do these two rectangles overlap" is
the very first thing every 2D game needs after it can draw, and Pong is the
canonical example of needing it. I wrote the arithmetic by hand — which is fine,
and arguably correct for a v1 that does not want to own collision — but the
absence is conspicuous next to a `Rect` type that exists purely to describe
boxes.

(I ended up needing a *swept* test anyway, which no engine helper would have
given me. But I did not know that when I went looking.)

### Any way to ask "was this entity drawn?"

`FrameRecord::covering(point)` answers "what quads cover this world point", and
`DrawnQuad` exposes `bounds()` and `texture`. So the only way to assert "the ball
was drawn" is "some quad of exactly 0.8×0.8 covers the ball's position" — you
identify your own game objects in the frame by *reading their size back*. That is
the trick `prototype_kit` uses and I copied it, but it is indirect enough that I
would not have invented it, and it gets fragile the moment two things in a game
are the same size.

### A shorter road to "assert on what was drawn"

The document says: *"To check what was drawn, render into
`jidousha::testing::NullBackend`, which records every frame as structured data."*
That sentence undersells it by a lot. The actual ceremony is:

```rust
let textures = create_builtin_textures(backend);
let quads    = sim.draw().quads().to_vec();
let plan     = plan_frame(&camera, &quads, &textures);
backend.render(&plan)?;
let last     = backend.last_frame();
```

...plus, to find out which `BackendTextureId` the font landed on at assertion
time, building a **second throwaway `NullBackend` and a second texture table** in
the same order and asking that one, because the real table is out of scope by
then. `prototype_kit/verify.rs` has a 9-line doc comment apologising for this. I
copied it verbatim, including the apology's logic, and I do not fully understand
why the frame does not just carry the mapping.

`sim.draw().quads()` hands you quads directly with none of that — but `Quad`'s
fields are undocumented, so I could not tell whether asserting on them straight
was viable, and took the long road.

---

## Things that behaved differently from what the document implied

### `Startup` runs *inside* the first `tick()`, not before it

The reference says `**Startup** — Runs once, before the first tick`. I read that
as "before you call `tick()`", i.e. `headless(...)` returns a sim whose world is
already populated. It does not. The world is empty until the first `sim.tick()`
returns.

This cost me a panic: my rally harness reads the ball's position *before* each
tick (to steer a paddle at it) and indexed an empty `Vec` on tick 1.

Two things point the right way in the examples — `prototype_kit/verify.rs`
inserts a resource "Before Startup" and then enters the tick loop, and several
systems guard with `let Some(input) = world.find_resource::<Input>() else
{ return }` — but the reference's own wording says the opposite of what happens.
"Runs once, at the start of the first tick" would have been unambiguous.

### Alpha reads brighter than the number suggests

Not something I hit, because `prototype_kit` warns about it in a comment
(blending happens in linear light, so 0.06 white on dark reads as solid grey).
I picked 0.16 for field markings straight off that warning. The API document's
Color section says "sRGB-encoded... linearization happens inside the render
backend, invisibly" — which is exactly the sentence that would lead someone to
expect alpha to behave the way the number looks. The example knows better than
the document here.

---

## Things that took more than one attempt

### The paddle bounce plane (my bug, but instructive)

I wrote a "clever" symmetric formula for where the ball's centre sits when it
touches a paddle:

```rust
let face = facing * (-PADDLE_X + facing * (PADDLE_SIZE.x * 0.5 + BALL_HALF));
```

Correct for the left paddle. For the right paddle it puts the bounce plane at
x = 15.75 — **1.5 world units behind the paddle**. The ball visibly passes
through the opponent's paddle and then comes back.

What is worth recording is that nothing caught it. The game ran. The ball came
back. The opponent scored. My first six `--verify` assertions all passed. I found
it by hand-checking the arithmetic on paper, not by running anything.

I then wrote the assertion that catches it ("the ball turned round inside the
opponent's paddle": every reversal of the ball's X velocity must happen on the
playing side of that paddle's face), confirmed it fails on the old code, and
restored the fix. That round trip is the argument for `--verify` being about
*game invariants* rather than "did it run without crashing" — and it is an
argument the engine's own framing supports well. The tooling made the assertion
easy to write once I knew what to assert.

### Making it fun took three passes, and one of them was a real surprise

1. **Pass one:** ball speed 17, paddles 3.4 tall, AI at 13.5 u/s tracking the
   ball whenever it moved toward it. Against an idle scripted player it won 7-0,
   so the game "worked". Against a player that tracked the ball, **nobody scored
   in thirty seconds** — a single unbroken 25-hit rally.

2. **Pass two:** gave the AI a handicap — it does not commit until the ball is
   past halfway. Still 0-0 after thirty seconds. The reason turned out to be my
   *test player*, not the AI: a player that tracks the ball exactly meets it with
   the middle of the paddle every time, and a centre hit returns the ball dead
   flat, so two exact trackers rally forever at a fixed height. The degenerate
   equilibrium is an artifact of the perfect tracker, and it hid the real
   question.

   Fix: give the stand-in player a 12-tick reaction lag. Lag means off-centre
   contact, off-centre contact means angle, angle means rallies that end. This is
   now documented in `verify.rs` as the reason the lag exists, because it looks
   like gratuitous realism and is not.

3. **Pass three:** with a realistic opponent to measure against, tuned for
   ~10-hit rallies: faster serve, much sharper per-hit speed-up (0.7 → 1.4),
   smaller paddles, first-to-five instead of first-to-seven. Result: 1-1 in
   thirty seconds against the lagged tracker, 5-0 in twelve seconds against
   somebody who is not playing.

None of this is the engine's fault. It is the honest cost of "fun for about
thirty seconds", and it is most of the wall-clock this task took. Recording it
because "a working Pong" and "a Pong worth thirty seconds" are two different
deliverables and only the second one required this.

### The score/points accounting assertion

Wrote `peak(score.left) + peak(score.right) == tally.points`, which is wrong the
moment the game supports a rematch, because the board is wiped between matches.
Replaced with a per-tick invariant (the board never shows more points than have
been scored) plus a per-match check either side of the restart. My error, found
immediately by the check failing.

---

## Things I wanted to look up in the source, and what for

Listed because "what an agent would have grepped for" is the measurement here.
I did not open any of these.

| Wanted | What for |
|---|---|
| The `Key` enum | The variant list. Arrow keys, `Escape`, digits. |
| `GameConfig`'s fields | Whether the initial window size is settable. |
| The default `fixed_dt` | To express a serve pause of "about 0.75s" in ticks. |
| `Rect`'s inherent impl | Whether an overlap/contains test exists before writing my own. |
| `Quad`'s fields | Whether I could assert on `sim.draw().quads()` and skip the whole `NullBackend`/`plan_frame`/texture-table ceremony. |
| `InputScript::hold` | Whether the range is half-open. My assertions pass either way, so I never learned. |
| `Camera`'s fields | Who sets `viewport`, and what `Camera::default()` actually contains. |
| `message`'s signature | Whether the four arguments I copied are the whole story. |
| Whether `run()` inserts `Assets` | To know if a shapes-only game is really a supported shape. |

---

## What I could not check at all

**I have never seen this game.** The container has no display and no GPU:

- `cargo run -p jidousha --example pong` exits with
  `RunError::NoDisplay { detail: "neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor
  DISPLAY is set" }` — a clean, well-worded failure, but a failure.
- `WgpuBackend::offscreen` reports `no suitable graphics adapter`, so I did not
  add a PNG capture step the way `prototype_kit` does; it would have been dead
  code on every machine I can reach.

So "it runs in a window and is playable" is inferred, not observed. What I
*could* do — and this is the engine's best moment in the whole exercise — is read
`FrameRecord::transcript()` and check every quad's world-space extent by eye:

```
quad (-15.350, -9.500) (-14.650, -6.500) tint #73f2ffff   <- player paddle, 0.7 x 3.0, jammed at the top
quad ( 14.650, -1.300) ( 15.350,  1.700) tint #ff8c73ff   <- opponent paddle, centred at y=0.2
quad ( -0.400, -0.400) (  0.400,  0.400) tint #ffffffff   <- ball, 0.8 square, on the centre spot
quad ( -3.867, -8.500) ( -2.000, -6.100) tint #ffffffbf   <- left score digit
```

That is a genuinely good substitute for a screenshot, and it is the reason I am
reasonably confident about the layout despite never having looked at it. The
`--verify` run prints it.

### Postscript: someone else ran it, and the window did not open

Added after the notes above were written. The repository's owner ran the game on
a real Linux desktop. Two things came out of it, and both are worth having here
even though neither is about the API document.

**The game is fine.** Controls felt good, the opponent was judged hard but
enjoyable at roughly a one-in-four or one-in-five win rate, the ball's top speed
was not a problem, and first-to-five was the right match length. No constant was
changed as a result. That is the acceptance criterion this run could not check
for itself, and it passed.

**The window did not open, for reasons entirely below the game.** On a machine
with a discrete NVIDIA GPU (the compositor's) and an integrated AMD one, every
windowed example — `pong` and the two-shape `window_clear` alike — dies at surface
setup with `error 7: importing the supplied dmabufs failed`. wgpu selects the
*integrated* GPU, and cross-vendor dmabuf import into an NVIDIA-driven compositor
fails. `VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json` fixes it
completely; `WGPU_POWER_PREF=high` does not, which says the platform crate is not
consulting `PowerPreference::from_env()` and is passing wgpu's default —
`PowerPreference::None`, which performs no adapter sorting whatsoever.
Filed as [#23](https://github.com/JHorace/jidousha/issues/23), with
`PowerPreference::HighPerformance` as the suggested fix.

Two observations that do belong in a document about writing games against this
engine:

- **A game author cannot diagnose this and should not try.** Adapter selection is
  behind the backend boundary, four crates away from anything `DrawCtx` exposes.
  The only reason it got diagnosed at all is that `window_clear` exists — a
  windowed example small enough to prove the failure is not yours. That example
  earns its place in the repository on this alone.
- **The failure message points the wrong way.** `RunError::EventLoop` says "the
  display server went away mid-run" and advises restarting. The display server
  was fine and restarting never helped; the real cause was printed by the Wayland
  client library one line above and never reached the engine's message. For a
  project whose rule is that an error states what happened, its likely cause and
  its fix, this is the one message encountered in the whole exercise that got all
  three wrong — and it is the message a new user is most likely to hit first,
  because it fires before their game runs at all.

So the honest final state of the "I have never seen this game" note above: I still
have not. Someone else has, on the second attempt, after being handed an
environment variable.

---

## What worked, briefly

Worth stating so the friction above is read in proportion.

- **The two-pass read/write pattern** is called out in the Concepts section *and*
  has a dedicated example (`homing.rs`). I hit it three times, recognised it
  instantly each time, and never fought the borrow checker. This is the document
  working exactly as intended.
- **Y-down** is stated four times in four different places and I never once got
  confused about it, despite it being the thing most likely to confuse someone.
- **Determinism is real and effortless.** Replaying a 1500-tick session and
  comparing float bits landed identical with no care taken. `Rng` from the world
  and `sin_cos` from `jidousha::math` were the only two things I had to remember,
  and both are flagged in the document.
- **Nothing forced an asset pipeline on a game that has no art.** Shapes and text
  are enough for a whole game, and that is a real design achievement.
- **Compiled on the first try**, ~450 lines of game written against examples
  alone, with zero clippy warnings. The API's shape is guessable once you have
  seen one worked example of each thing you need.

The gap is not the API. The gap is that the *document* describing the API is a
list of names, and everything that made this possible was in the examples beside
it.

---
---

# Run 2 — the same exercise, against the fixed document

Written by a second author, under the same rules and with the same two sources.
A fresh Pong, written from nothing: the previous run's `pong/` was deleted
without being opened, and is in `main`'s history if anyone wants to diff the
two. The game is `crates/jidousha/examples/pong/` again; `cargo run -p jidousha
--example pong -- --verify` is green, `cargo fmt --all` and `cargo clippy
--workspace --all-targets -- -D warnings` are clean.

## The headline

**Run 1's headline is fixed, and it is not a small fix.** The reference is no
longer a list of names — it is signatures, struct fields with per-field
comments, enum variants, `Default` values spelled out, and trait method
signatures. Every single one of run 1's fourteen findings that was about a
missing fact is closed:

- `Key` lists all ninety-odd variants. I used `ArrowUp`, `ArrowDown` and
  `Space` without guessing at any of them.
- `GameConfig` lists its four fields *and its default value*, so I knew
  `window_size` existed and could stop worrying about the field being cropped.
- `fixed_dt` is stated to be 1/60, in a paragraph that then explicitly says
  sixty is the number to count in for "a serve pause" — which is precisely and
  literally what I needed it for, two hundred lines later.
- `ctx.rect(rect, color, depth)`'s argument order came from the document.
- `Rect::overlaps` exists, which is run 1's "things I expected to exist and
  could not find" #1, and it is the whole of my paddle collision.
- `TextStyle` carries its own `depth`, so text is no longer the odd one out.
- `FrameRecorder::font_texture()` exists, so "was that text?" is one call
  rather than the rebuild-a-texture-table ceremony `prototype_kit` still
  carries a long comment about.

The game compiled on the first attempt with two warnings, both mine (an unused
import and an unused binding). I did not consult the engine's source, and I did
not want to except in the five places listed at the end.

**So the gap has moved.** It is no longer "the document does not say how to call
this". It is now, almost entirely, "the document does not say that this is a
*resource*, or how a game gets hold of one".

---

## Things the document did not tell me

### `World` has an entire resource API that the reference does not mention

This is the biggest gap of the run, and it is strange because the document
*uses* the missing methods on its own first page.

`World`'s impl block lists seventeen methods: `spawn`, `despawn`, `insert`,
`remove`, `query`, `component_mut`, `commands`, and so on. Not one of them is
about resources. But the Quickstart — in the same document, above the
reference — calls `world.insert_resource(Score::default())`,
`world.find_resource::<Input>()` and `world.resource_mut::<Rng>()`. And
`WorldView`, the read-only view, *does* document `resource` and `find_resource`.

So the read-only half of the world documents its resource access and the
mutable half documents none of it. I recovered the set — `insert_resource`,
`resource`, `resource_mut`, `find_resource`, `find_resource_mut` — by grepping
`examples/` for `resource`. My Pong calls four of those five, constantly: the
score, the round state, the volley counter, the camera and the clock are all
resources, and a game like this is *mostly* resource access.

Things I still do not know, because nothing states them: whether
`remove_resource` exists; whether `resource::<T>()` panics or does something
else when the resource is absent, and what it says when it does. I guarded with
`find_resource` everywhere a miss seemed possible, which may be pure ceremony.

### The document never says `Camera` is a resource

`Camera` is documented under "Render" as a struct with four fields and six
methods, one of which (`visible_bounds`) I use in two draw systems. Nothing
anywhere says how a game *sets* one. The answer is
`world.insert_resource(Camera { .. })` in a `Startup` system, which I got from
`window_clear.rs`. Nothing says whether a default camera exists if you never
insert one, either — the `Default` line implies one could, but "the engine
installs it for you" and "you must install it" are very different, and only one
of them is true.

`Time` has the same problem: documented under "Math and primitives" with a
`Time::new` constructor, never described as a world resource, and every game
reads it as `world.resource::<Time>()` to get `fixed_dt`. `Rng`, meanwhile,
says "held as a world resource" right in its summary line. So of the three
engine-provided resources a game touches, one says what it is and two do not.

A "Resources the engine provides" section — `Time`, `Rng`, `Input`, `Camera`,
`Assets`, which of them the engine installs and which the game must — would
close this and the item above it together.

### `Submissions` is not in the document at all

`HeadlessSim::draw()` is documented as returning `&Submissions`. `Submissions`
appears nowhere else: not in the reference, not in the testing section, not in
Concepts. `prototype_kit` calls `.quads()` on it and that is the only evidence
it has methods.

It did not block me, because "Testing your game" steers you to `FrameRecorder`
instead and that is what I used. But a return type named in a signature and
then never defined is the one kind of gap that has no workaround if you happen
to need it.

### Nobody says who owns the camera's viewport in a headless run

`FrameRecorder::new(viewport)` takes a `PhysicalSize`. The `Camera` resource
*also* has a `viewport` field, defaulting to 1280×720. Does the recorder
override the camera's, or are the two independent and the game's job to keep in
agreement?

I do not know, and I made sure it could not matter: I pass the recorder the
same 1280×720 the camera defaults to, so the two cannot disagree. That is a
dodge. It matters more than it looks, because my most valuable assertion —
nothing may be drawn outside the visible screen — reads the rectangle from the
`Camera` resource and the quads from the recorder. If those two have different
viewports, that assertion is quietly comparing against the wrong rectangle and
will keep passing while the game is broken.

### Query shapes are shown, never stated

`World::query<'w, Q: ReadOnlyQuery<'w>>` tells me there is a trait called
`ReadOnlyQuery`. It does not tell me what may implement it. From the Quickstart
I know a 2-tuple of references works and that the iterator yields the entity
prepended — `(entity, &Transform, &Player)`. I do not know the maximum arity, or
whether a 1-tuple works.

`With<T>` and `Without<T>` are listed as types that implement `Query`, with no
example anywhere of where one goes in a tuple or what it yields for that
position. I never used either.

I structured the whole game around 2-component queries — putting a `Control`
enum *inside* the `Paddle` component rather than using separate `Player`/`Cpu`
marker components — so that I would never find out. It happens to be the better
design, and I would defend it on the merits now. But I did not choose it on the
merits; I chose it because I could not tell what would compile.

### There is no way to feed one tick of input

`Input::new` takes an `InputSnapshot`. `InputSnapshot::new()` is documented as
"a tick in which the player did nothing", and its seven other methods are all
readers — `held_keys`, `pressed_keys`, and so on. There is no setter. The only
route to a snapshot with anything in it is `InputScript::hold(key, range)`
followed by `snapshot_at(tick)`.

That is exactly right for a scripted session and no use at all for a controller
that has to *see the game* before it decides what to press. My check needs one
of those — see below, it is the only thing that proved the game playable — so
it builds a throwaway one-tick script every single tick:

```rust
InputScript::new().hold(Key::S, tick..tick + 1).snapshot_at(tick)
```

It works, it is deterministic, and it is faintly absurd. An
`InputSnapshot::with_keys(&[Key])` is the missing word.

### Nothing says how a game exits

There is no `App::quit`, nothing on `World` or `Commands`, and `run` is
documented as "Run a game in a window, **forever**". I read the whole reference
looking for it rather than guessing, so I am fairly confident this is "not in
v1" rather than "undocumented" — but the document does not say that either, and
`Key::Escape` being listed invites you to look. My Pong cannot close itself;
the player closes the window. Fine for Pong, not fine for anything with a menu.

### `Vec2` is out of scope of a document that says nothing is out of scope

> Also in `math`, re-exported from `glam` and documented there.

The top of the document says "Everything here is reachable from one import ...
If something you want is not here, it is not part of v1." `Vec2` is in almost
every line of this game — `length`, `abs`, `splat`, `min`, arithmetic
operators, `const fn new` in a `const` item — and it is part of v1, and it is
not here. "Documented there" points at a crate whose docs are not in this
container.

This cost me nothing, because I happen to know glam. That is luck, not the
document working. Ten lines listing the dozen `Vec2` methods a game actually
reaches for would close it.

---

## Things that behaved differently from what I expected

### `fn main() -> Result<(), RunError>` throws away the good error message

This is the signature the Quickstart uses and every example copies, mine
included. With no display, here is what my game printed:

```
Error: NoDisplay { detail: "os error at /root/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/linux/mod.rs:765: neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set." }
```

The *content* is excellent, and worth saying plainly against run 1's postscript,
which found the equivalent message pointing squarely the wrong way: this one is
accurate, it names the real cause, and it is the right variant. `NoDisplay` is
what happened.

But that is the `Debug` form. `RunError` implements `Display`. The engine has a
whole house style for failures — `message(what, specifics, likely_cause, fix)`,
which the reference documents and which my own check uses to report *its*
failures. And the documented shape of `main` discards all of it in favour of a
struct dump with a vendored crate path and a line number from a dependency in
it.

Every game written from the Quickstart has this. The fix is a `main` that
matches on the error and prints `Display` — but then the Quickstart has to show
that, because nobody writes it unprompted, and I did not either until I saw the
output.

### An engine example points at my game

`prototype_kit/verify.rs` carries a doc comment reading "**A game does not do
this.** `FrameRecorder::font_texture()` answers the question directly — see
`pong/verify.rs`". So an engine example documents itself by reference to a file
in a *game* that an exercise like this one is expected to produce. It happened
to stay true — I did use `font_texture()` — but it was true by luck, and I had
deleted the file it names before I read the comment.

---

## Things that took more than one attempt

### The serve pause was off by one, and only the check could have known

`Round::Serving { ticks: 48 }` is set *on* the tick that awards the point, so
that tick already ends with the ball parked — an observer sees `Serving` for 49
ticks, not 48. My assertion said 48 and the run failed on it.

I fixed the game rather than the assertion, so that `SERVE_TICKS` means the
number of ticks somebody actually waits. Entirely my own bug. Worth recording
because the discrepancy is exactly one tick at 60Hz: no one watching the window
would ever have seen it, and I would have shipped a constant whose doc comment
was a lie.

### The game was not fun, three times over, and the check was the only instrument

This is where the time went, and none of it is the engine's fault. It is
included because it is the strongest evidence in this file about what the
engine's testing story is actually *for*.

I cannot see this game (below). So after the mechanics were green I added a
second scenario: a closed-loop player that watches the ball, played to a
win, asserting that a rally happens, that the ball speeds up, and that a
competent player can beat the computer.

**First run: 0–0 after a hundred simulated seconds.** Not a crash — an
*unloseable* rally. A player who perfectly centres the ball on its paddle
returns it dead flat, because the bounce angle comes from where on the paddle it
lands and the middle of the paddle means zero. So the computer never has to
move, and neither does the player, forever. The bounce model has a fixed point
at "hit it in the middle" and two perfect trackers sit down in it. The fix was
to make the test player *aim* — meet the ball with the half of the paddle that
sends it away from the opponent — which is what a person does and a naive
tracker does not.

**Second run: 3–0 in a hundred seconds.** Rallies now ended, so that was the
real bug. But a point was taking thirty-three seconds. My first failure message
said only the score, which told me nothing, so I rewrote it to report the
longest rally and the top ball speed as well — 14 touches, 25.6 units/s — and
the diagnosis was immediate: the ball was simply too slow for a field 32 units
wide, and the computer had time to reach anything.

**This is the lesson, and it generalises past this game.** An assertion that
says only "this is wrong" is nearly useless to an author who cannot look at the
thing. It has to report the numbers it judged. That took me one wasted cycle to
learn and it should probably be a sentence in "Testing your game".

**Third run:** retuned the whole speed budget around *crossing time* rather than
around each number on its own — serve 20, cap 40, gain 2.0 a touch, computer
15.5 — so a fresh ball is comfortably reachable and a wound-up one is not. A
full game to five now takes 68 seconds, with rallies up to 15 touches and the
ball reaching its cap. That is a game.

### The transcript found a bug that eight assertions missed

The first game-over banner was one centred line: "the computer wins 5-0 — space
to play again". Forty-three characters at size 1.3 is 43.5 world units, across a
screen 35.6 units wide. It ran off *both* edges.

Every assertion passed. Glyphs existed, the score was where the layout put it,
the world state was correct, the paddles and ball were drawn in the right
places. I found it by reading the frame transcript, which the document promises
is "good enough to check a layout by eye" — it is, and this is the proof.

Two things follow, and I think both belong in the document:

- **`TextStyle::width_of` is exact and completely silent.** Centring by it is
  the documented idiom, `prototype_kit` demonstrates it, and it overruns the
  screen without a word from anything.
- **"Nothing is drawn outside `Camera::visible_bounds()`" is the single highest
  value assertion a shapes-and-text game can write**, and it is mentioned
  nowhere. It is six lines. I wrote it, then negative-tested it by lengthening
  the banner again, and it correctly reported the offending quad, its extent,
  the camera's extent, and that centred text is the usual culprit.

---

## Things I wanted to look up in the source, and what for

Five, none of which I looked up:

1. Whether `World::resource::<T>()` panics on a missing resource and what the
   message says. I used `find_resource` defensively in several places where it
   may be needless.
2. Whether `Rect::overlaps` counts a shared edge as overlapping. This one
   genuinely matters here: after a bounce I place the ball exactly against the
   paddle's face, so if touching counts, the ball could re-trigger the bounce
   and rattle in place. I made the question moot by *also* requiring the ball to
   be travelling toward the paddle — but I wrote that guard because I did not
   know the answer, not because I had reasoned it was needed. The
   `contains` entry says it counts the top-left edges and not the others;
   `overlaps` says nothing.
3. Whether `FrameRecorder` overrides the `Camera` resource's `viewport`.
4. What `Submissions` is.
5. Whether `ctx.text` lays out `\n` as multiple lines, and what `width_of`
   returns for a string containing one — the widest line, or the total?
   `prototype_kit` passes multi-line strings to `ctx.text`, so the first half
   evidently works; the second half decides whether centring a two-line banner
   is possible. I avoided it entirely and drew two separately-centred `ctx.text`
   calls instead.

---

## What I could not check at all

**I have still never seen this game.** No `/dev/dri`, no
`/usr/share/vulkan/icd.d`, `libvulkan.so.1` present with no driver behind it,
and neither `DISPLAY` nor `WAYLAND_DISPLAY` set. So there is no window, and —
unlike run 1, which at least had the option — there is no captured PNG either,
because `WgpuBackend::offscreen` has nothing to run on.

Everything above is inferred from 900 recorded frames, one frame transcript, and
a 4087-tick closed-loop session. The geometry is checked and the behaviour is
asserted. Whether it *feels* right at forty world units a second is a claim I
cannot make, and I am not going to make it.

## Contamination, stated plainly

Run 1's notes are in this file, and this file is the one I was told to write
into, so I read them before writing a line of Pong. That means I knew — before
opening the API document — that the timestep is 1/60, that `Key::ArrowUp`
exists, and that `ctx.rect` takes `(rect, color, depth)`.

All three of those are now *in the document*, and I have said above where the
document told me each of them. So I do not think the conclusion changes. But
"run 2 guessed at nothing" is weaker evidence than it looks, and a clean
measurement would have put the two runs in separate files.

## What worked, briefly

- **The reference is now usable on its own for anything that is a function
  call.** I checked argument orders against it, not against examples, for the
  entire drawing vocabulary. That is the single biggest change from run 1.
- **Y-down never once confused me**, again.
- **Determinism is free.** 900 ticks replayed and compared bit-for-bit,
  identical, with no care taken beyond using `sin_cos` from `jidousha::math`.
  I did not have to think about it once.
- **The two-pass read/write pattern** is described in Concepts, has a named
  example, and I hit it four times without ever fighting the borrow checker.
- **A whole game of shapes and text, with no asset story anywhere.** The
  paragraph promising this is accurate: `Assets` never appears in my game.
- **`FrameRecorder` is the right shape.** `covering(point)` answering "what is
  drawn here, with exact quad containment" is what made every drawing assertion
  in this game a two-liner.
