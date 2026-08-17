A game is **entities**, **components** and **systems**. An entity is an id. A
component is a plain struct you attach to one (`impl Component for Health {}`).
A system is a function: `fn(&mut World)` for logic, `fn(&mut DrawCtx)` for
drawing. Nothing inherits from anything, and there is no base class to fill in.

Systems run in **phases**, in this order, every tick: `Startup` once at the
start of the first tick, then `Update` for logic, then `Draw`. Within a phase
they run in the order you added them, always, on every machine. There is no
scheduler deciding for you. Startup running *inside* that first tick is worth
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
| `Input` | `run`, before every Update tick | **yes** — not before the first tick, and never under `headless` unless a test inserts it |
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

The engine runs on a **fixed timestep**. `Time::fixed_dt` is the same number
every tick, `Time::tick` counts them, and a slow frame runs several ticks rather
than one long one. That number is **1/60 of a second** unless you say otherwise:
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

Together with the seeded `Rng` in `GameConfig`, that means the same inputs make
the same game — which is what lets a test replay a session and get the same
answer.

**Drawing is submission, not painting.** A `Draw` system hands the renderer
quads — `ctx.sprite`, `ctx.rect`, `ctx.line`, `ctx.circle`, `ctx.text` — and
cannot change the world; the type system enforces that. Order comes from
`Depth { layer, z }`, not from the order you drew in, so a debug outline goes in
front by saying so rather than by being drawn last.

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
