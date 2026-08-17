# E0 run 2 — the same exercise, against the fixed document

Written by a second author, under the same rules and with the same two sources:
`docs/api/jidousha-api.md` and `crates/jidousha/examples/`, and none of
`crates/*/src/`, `docs/internal/`, `docs/adr/`, `docs/agent-practices.md` or
`docs/conventions.md`. A fresh Pong, written from nothing: run 1's `pong/` was
deleted without being opened, and is in `main`'s history if anyone wants to diff
the two. The game is `crates/jidousha/examples/pong/` again; `cargo run -p
jidousha --example pong -- --verify` is green, `cargo fmt --all` and `cargo
clippy --workspace --all-targets -- -D warnings` are clean.

Run 1 is in `run-1.md`. It was one file with this one when this was written —
see *Contamination, stated plainly* below, which is why they are two now.

---

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
