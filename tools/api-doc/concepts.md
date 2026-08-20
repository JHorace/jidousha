A game is **entities**, **components** and **systems**. An entity is an id. A
component is a plain struct you attach to one (`impl Component for Health {}`).
A system is a function: `fn(&mut World)` for logic, `fn(&mut DrawCtx)` for
drawing. Nothing inherits from anything, and there is no base class to fill in.

Systems run in **phases**, in this order: `Startup` once at the start of the
first tick, then `Update` for logic, then `Draw`. `Startup` and `Update` are
per *tick*; **`Draw` is not** — a window draws once a frame, however many ticks
that frame ran, including none, and a headless `tick()` does not draw at all.
So a `Draw` system sees the world the last `Update` left and must not count on
being called the same number of times; a check that wants a frame asks for one,
with `HeadlessSim::draw()`. Those three are
the whole set — `Phase` and `IntoSystem` appear in `add_system`'s signature as
bounds, are not exported, and are not names a game writes or can collide with, so
your own `enum Phase` for "which screen are we on" is yours to take. Within a
phase systems run in the order you added them, always, on every machine. There is
no scheduler deciding for you. Startup running *inside* that first tick is worth
knowing if you drive the sim by hand: `headless(..)` hands back a world that is
still empty, and it is populated once the first `tick()` returns.

**A resource is a thing there is exactly one of** — the score, the round state,
the camera. `world.insert_resource(Score::default())` puts one in,
`world.resource::<Score>()` and `world.resource_mut::<Score>()` read it back and
panic if it is absent, `world.find_resource::<Score>()` returns `Option` where
absence is expected, and `world.remove_resource::<Score>()` takes it away. A
`Draw` system reaches the same values through `ctx.world`, read-only. Most of a
game is resource access, so it is worth knowing which resources are already
there:

| Resource | Who inserts it | Can it be absent? |
|---|---|---|
| `Time` | `run` and `headless` both, before the first tick | no |
| `Rng` | the same, seeded from `GameConfig::seed` | no |
| `Input` | `run`, before every Update tick | **yes** — `Startup` never sees one, because it runs inside the first tick before that tick's `Input` exists; and never under `headless` unless a test inserts it |
| `Camera` | the game, in `Startup` | **yes** under `headless`; under `run`, a game that inserts none is given `Camera::default()` before the first frame |
| `Assets` | the game, if it has art | **yes** — a game of shapes and text never inserts one |

The three that can be absent are the three to reach for with `find_resource`.
That is why the Quickstart's `walk` system opens with
`let Some(input) = world.find_resource::<Input>() else { return };` rather than
`world.resource::<Input>()`. `Camera` is the one to watch: a game that sets one
in `Startup` may read it back with `world.resource::<Camera>()` anywhere,
because `Startup` has run by then — but a game that relies on the driver's
default has no camera at all in a headless run, and a check reading one will
panic where the window would not.

**And that is a fact about `Startup`, not about `Camera`.** Anything your own
`Startup` inserts — a scoreboard, a round state, a layout you computed once — is
there for every `Update` and every `Draw` that follows, so `world.resource::<T>()`
and `ctx.world.resource::<T>()` are the right spellings for it. The table above
is short because the engine inserts little; the resources a game inserts itself
are the ones it can count on.

**The camera is the game's; its `viewport` is the driver's.** `run` measures the
window before the first frame and stamps that size into `Camera::viewport` on
**every** frame, after the frame's Update ticks and before `Draw` — so a
`viewport` set in `Startup` is never the one anything is drawn with, and a
resized window is honest from the frame the resize arrives rather than only
until the first one. Setting it is therefore not wrong and not useful: set
`center`, `height` and `clear_color`, leave `viewport` to `..Camera::default()`,
and read the real one back. Two consequences worth having:

- `visible_bounds()` in a `Draw` system is always the window's true aspect. In
  an `Update` system it is the size the *previous* frame stamped, which is the
  same number except on the first tick, where it is whatever the game put there.
  Compute a layout in `Draw`, or from your own constants, rather than from a
  viewport read on tick 1.
- **Under `headless` nothing stamps it at all.** There is no window to measure,
  so the viewport stays exactly what the game set — which is why
  `FrameRecorder::new` takes one and overrides the resource's. *Testing your
  game* has the trap that follows from that.

**A layout in constants is the prototype answer, and it is an answer about one
aspect.** You already know which: `GameConfig::window_size` is the size `run`
opens at and `PhysicalSize::aspect` is the number it implies, so pick the shape
there, lay the game out for it, and give the recorder the same size. What that
buys is checkable — a headless run has exactly one viewport, so every bounds
assertion is about the aspect you chose. What it costs is not: a player dragging
the window narrower than that moves the edges in, and anything placed against
them goes off the sides with no check able to see it, because there is no second
viewport to run. Deriving the layout from `visible_bounds()` in `Draw` is the
other answer and it survives the drag; it also means every position is computed
rather than named, which is the trade. A prototype takes the constants, states
the aspect it took, and knows what it gave up.

The engine runs on a **fixed timestep**. `Time::fixed_dt` is the same number
every tick, `Time::tick` counts them, and a slow frame runs several ticks rather
than one long one. **The first `Update` sees `tick == 1`**, because a tick
advances the clock and then runs Update — so a game timing something absolute,
"spawn the boss on tick 600", is counting from one. That number is **1/60 of a second** unless you say otherwise:
`fixed_dt` is a `GameConfig` field, so a game that wants 120 ticks a second sets
`GameConfig { fixed_dt: Seconds(1.0 / 120.0), ..GameConfig::default() }`. Sixty
is the number to count in when a game wants to say "about three quarters of a
second" as a number of ticks — a serve pause, a coyote-time window, an
invulnerability period.

A fixed timestep also means **collisions are only ever tested at tick
boundaries**. Nothing in v1 sweeps, so a body that moves further in one tick
than its target is thick steps clean through it and `Rect::overlaps` never sees
the frame where they touched. That is the first thing that bites a game with a
fast small ball, and the fix is the game's: keep `speed * Time::fixed_dt`
smaller than the thinnest thing it must not miss, and assert that against the
`fixed_dt` the engine actually hands you rather than against the 1/60 you
assumed.

**Which couples two constants that look unrelated: the thinnest collider is the
ceiling on speed.** A paddle's thickness reads as a cosmetic number and is not
one — it is the largest `speed * fixed_dt` the game may ever reach. So a game
that plays too slowly is not fixed by raising the speed; it is fixed by
thickening the thing the fast body must not miss, *then* raising the speed. Pick
the two together, and put both in the assertion.

**There is no `Rect::sweep` and no `Rect::inflate`, and that is a v1 boundary
rather than something you have missed.** The reason is worth a sentence, because
the shape you write instead is short and the shape you might expect is not. A
sweep helper answers "where along this tick's travel did they first touch", which
is about eight lines of arithmetic; what follows it — the bounce angle, the speed
change, the remaining fraction of the tick, the order two collisions resolve in —
is four or five times as much code and is your game's model rather than the
engine's. A primitive that answered the first and refused the second would be the
start of a physics engine, which v1 does not have. Write the eight lines: the
plane your body's leading edge touches, whether it was approaching, whether this
tick's travel crossed it, and the fraction of the tick at which it did.

**And the thing you sweep against has usually moved this tick as well.** That
arithmetic is written against a plane that stands still; a paddle is not one,
because a system earlier in the same tick moved it. Nothing in the engine orders
that for you and there is no sub-tick to appeal to, so pick one and say so at the
site: the prototype answer is to treat the collider as stationary at its
**post-move** position for the whole tick, which is wrong by at most one tick of
its travel and is right about the case that matters, a paddle moving to meet the
ball. The failure from not deciding is a ball that passes through a paddle
closing on it, and it survives every assertion that only asks where things ended
up.

**Having picked an order, hold the game to it.** The order *is* the sequence of
your `add_system` calls, and nothing but a reader protects it — so a tidy-up that
moves one of them reverses the decision silently and the ball starts passing
through the paddle again. `HeadlessSim::schedule_debug()` returns every phase and
its systems in run order as a string, which makes the order something a check can
assert on — the one instrument in this surface that sees it, and the only thing
that catches a swap of the two systems above. *Testing your game* has it written
out.

Together with the seeded `Rng` in `GameConfig`, that means the same inputs make
the same game — which is what lets a test replay a session and get the same
answer.

**Write the two decisions a check will want as free functions, now, while they
are free.** A `--verify` mode that plays your game rather than scripting it has
to ask *where will the ball be* and *where will the opponent move to*, and it can
only ask if the answer is a function it can call —
`fn opponent_target(ball: &Ball, paddle: &Paddle) -> f32`, called by the system
that acts on it rather than written as a branch inside that system. The same goes
for the collision response above: a `rebound(..) -> Vec2` the system applies is a
question a check can put to it directly, and a mutation in the middle of an
`Update` body is not. Nothing copies a running simulation — there is no way to
fork one and roll it forward — so *your* functions are what a check rolls
forward, and it is the same eight lines either way while the game is being
written. Retrofitting is not: by the time the check needs the answer it is
buried in a `&mut World`, and one run spent forty minutes and a restructure of
its main loop moving it back out. `docs/api/jidousha-controllers.md` is what does
the asking.

**Drawing is submission, not painting.** A `Draw` system hands the renderer
quads — `ctx.sprite`, `ctx.rect`, `ctx.line`, `ctx.circle`, `ctx.text` — and
cannot change the world; the type system enforces that. Order comes from
`Depth { layer, z }`, not from the order you drew in, so a debug outline goes in
front by saying so rather than by being drawn last. `layer`'s numbers are
**yours**: the engine sorts by them and has no opinion about what they mean, so
name your bands once in a `mod layers` of your own rather than writing `2` in
forty places. `examples/prototype_kit` is the worked version.

**Write the literal when you want a `z`; `Depth::layer(n)` is the shorthand for
`z` 0.** Higher `z` draws on top, so `Depth::layer(n)` is that band's *floor* and
everything else you put in the band gets a positive number above it. The fields
are public for this: a game with three bands and nothing sharing one never needs
a `z`, and a game with a stack of UI inside one band writes
`Depth { layer: layers::UI, z: 2.0 }` and is not reaching past an intended door.

**A quad is the unit, and two verbs are not one quad.** `ctx.rect` and
`ctx.line` each submit exactly one; `ctx.circle` submits **sixteen**, a fan of
wedges around the centre, and that count is fixed rather than scaled by radius.
`ctx.text` submits one quad per character, **spaces included** — each exactly
`size` tall and `size * 7 / 9` wide, laid out from its top-left corner, with `\n`
the only exception, counting as a line break and submitting nothing, which is the
whole of text's vertical metric: an N-line block occupies `N * size`. So a
26-character line with six spaces in it is 26 quads, and that is a contract you
can assert an exact count against rather than a coincidence: a space is one of
the ninety-five printable ASCII characters the font covers, with a blank cell of
its own.

So a circle costs sixteen rectangles and a score line costs one per digit — worth
knowing before a frame has three hundred of them, and worth knowing when you
assert on what was drawn, because "a quad the size of the thing" is the right
question for a rectangle and the wrong one for a circle.
*Testing your game* has the circle version written out.

**Those five verbs are the whole vocabulary, and every one of them fills.**
There is no outline mode, no stroke width and no dash pattern anywhere: a disc is
`ctx.circle` and a ring is not a thing you ask for. The reason is `Rect::sweep`'s
reason — an outline is one parameter until you ask how it behaves at a corner, at
a camera zoom, or with a gap in it — and the shapes you write instead are short.
A ring is two circles with the inner one in the background colour. A box around
something is four `ctx.line` calls, which is what `examples/prototype_kit` draws
its field border with. A dashed centre marking is a column of `ctx.rect` calls.

`Draw` reads the world's **committed** state — the values the last `Update`
left — so a fast body steps rather than glides at whatever rate the frames come.
`Time::alpha` is how far into the next tick the last frame fell, for a game that
minds enough to keep last tick's position in a component of its own and submit
`previous.lerp(current, alpha)`. Nothing in v1 consumes it and there is no
interpolation helper; a prototype ignores it and is right to.

**A game of pure shapes needs no asset story at all.** `ctx.rect`, `ctx.circle`,
`ctx.line` and `ctx.text` draw without a single file, and nothing requires an
`Assets` resource to exist — neither `run` nor `headless` inserts one or asks
for one. A whole game can ship without ever naming an asset, and the paragraph
below is what you read only once you want a picture.

**Assets load in the background and are never waited for.** `load_texture`
returns a handle immediately and the file arrives later. A sprite whose texture
has not arrived draws a magenta checkerboard, so a game runs from the first
frame and a missing file is visible rather than silent. `Assets::all_ready` is
there when you genuinely want a loading screen.

**Input is one value per tick.** `Input` answers `held`, `just_pressed` and
`just_released` about this tick only — no events, no callbacks, no polling
mid-tick. A tap that begins and ends between two frames still produces both
edges, because edges are recorded rather than inferred from a difference.

**Coordinates are Y-down**: `+X` is right, `+Y` is *down*, and everything is in
world units, not pixels. The camera is `height` world units tall and as wide as
the window's aspect makes it. `Camera::world_to_screen` and `screen_to_world`
convert when you need pixels — pointer positions arrive in pixels and become
world coordinates through the camera you choose.

Which is why a `Rect` is `min` at the **top-left** and `max` at the
bottom-right, and why **a `Rect` is only well-formed with `min <= max`
component-wise — nothing checks**. Every method assumes it: `size()` on an
inverted rect returns negative components rather than complaining, and
`overlaps` and `contains` quietly answer about a rectangle that is not there.
`from_min_size` and `from_center_size` cannot produce one from a non-negative
`size`, so build through them, and when you do assemble a rect by hand from two
corners, subtract from `max` and add to `min` rather than the other way round.

**A query is a shape, and these are all the shapes there are.** A part is `&T`,
`&mut T`, `With<T>` or `Without<T>`; a query is one part, or a tuple of up to six
of them, and the one-tuple `(&Transform,)` works as well as the bare form.
`world.query::<Q>()` takes read-only parts; `world.query_mut::<Q>()` is the one
that accepts `&mut T`. **The iterator yields the entity first**, then one item
per part, and the two filters yield `()` — they still occupy their position:

```rust
for (entity, transform) in world.query::<&Transform>() { }
for (entity, transform, velocity) in world.query_mut::<(&mut Transform, &Velocity)>() { }
for (entity, transform, _) in world.query::<(&Transform, With<Player>)>() { }
for (entity, transform, _) in world.query::<(&Transform, Without<Frozen>)>() { }
```

A filter is worth it when the marker carries no data and you would otherwise
bind a component you do not read. A component holding an enum — `Paddle {
control: Control }` — is the other way to say the same thing, and it keeps the
tuple shorter.

**Iteration order is deterministic, which is not the same as stable, and the
difference is what a game may lean on.** The order a query yields entities in is
a pure function of the world's operation history: the same spawns, inserts and
despawns in the same sequence give the same order, on every machine and every
run. That is what lets a check replay a session and compare it to the first one
bit for bit. What it is *not* is sorted by entity id, and it is not spawn order
once anything has been despawned — a removal moves the last row into the hole it
left. So: rely on "the same run twice yields the same order", never on "the first
one out is the one I spawned first". A query that matches exactly one entity is
unambiguous however things are ordered, so `.next()` for *the* ball is fine; a
`Vec` of both paddles is not, and wants a `sort_by_key` on something the game
owns — the side they play, not the order they came out.

**A game does not close itself.** There is no `App::quit` and nothing on `World`
or `Commands`: `run` is the whole program until the player closes the window.
That is a v1 boundary rather than an omission you have missed — `Key::Escape` is
listed because games use it to back out of menus, not because it exits.

**Reading while writing: the two-pass pattern.** A query that borrows the world
mutably holds it for as long as you iterate, so a system that needs to look at
*other* entities while changing one reads first and writes second. Collect what
you need into a `Vec`, drop the query, then apply. `examples/homing.rs` is the
worked version, and this is the one shape that surprises people coming from
engines where everything is a global.

**It is a `query_mut` rule, and a `Draw` system is not subject to it.** `ctx.rect`
takes the context mutably, which looks like the same situation and is not:
`ctx.world.query(..)` hands back an iterator borrowed from the *world* rather
than from the context, so a Draw system draws straight out of its query and
never needs the `Vec`. Both worked examples do it that way. Collecting first in a
Draw system costs an allocation a frame and buys nothing.

**An example can be a directory, and a game with a `--verify` mode wants one.**
`cargo run --example <name>` picks up `examples/<name>.rs` and
`examples/<name>/main.rs` alike, with no `[[example]]` entry in any `Cargo.toml`
either way. The second is what lets the game, its checks and its capture path be
files beside each other, reached with `mod verify;` from `main.rs`.
`examples/prototype_kit` is the worked version.

**A game written in this repository's `examples/` inherits the engine's own
lints.** `crates/jidousha/Cargo.toml` has `[lints] workspace = true`, and that
applies to example targets as much as to the crate — so
`cargo clippy --all-targets -- -D warnings` holds your game to the maintainers'
rules, and it is the last step of "done" rather than the first, which is a bad
place to meet a rule for the first time. Five bite in practice:

- `missing_docs`, denied — the file needs a `//!` header, and any `pub` item in
  it needs a doc comment. This one is a compile error before clippy is reached.
- `unwrap_used` and `expect_used`, denied — including in the `--verify` mode,
  where they are the natural spelling. Say what went wrong instead: a `let else`
  that reports the missing thing is better evidence than a panic, and every
  example here is written that way.
- `collapsible_if` — the fix is a let-chain, `if let Some(t) = hit && t > 0.0`.
- `approx_constant` — a float literal close to π or one of its fractions is
  rejected, which is what a hand-typed angle in radians looks like. Write the
  angle in degrees; `Radians::from_degrees` is a `const fn` for this.
- `manual_range_contains` — `y < -LIMIT || y > LIMIT` wants
  `!(-LIMIT..=LIMIT).contains(&y)`. A clamp against a symmetric pair is the
  shape that trips it, and the same test written against two separately named
  bounds does not, so it arrives once you tidy the constants rather than when
  you write the check.

Run it while you write rather than at the end, and none of them costs anything.
