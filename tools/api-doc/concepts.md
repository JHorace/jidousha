A game is **entities**, **components** and **systems**. An entity is an id. A
component is a plain struct you attach to one (`impl Component for Health {}`).
A system is a function: `fn(&mut World)` for logic, `fn(&mut DrawCtx)` for
drawing. Nothing inherits from anything, and there is no base class to fill in.

Systems run in **phases**, in this order, every tick: `Startup` once at the
beginning, then `Update` for logic, then `Draw`. Within a phase they run in the
order you added them, always, on every machine. There is no scheduler deciding
for you.

The engine runs on a **fixed timestep**. `Time::fixed_dt` is the same number
every tick, `Time::tick` counts them, and a slow frame runs several ticks rather
than one long one. Together with the seeded `Rng` in `GameConfig`, that means
the same inputs make the same game — which is what lets a test replay a session
and get the same answer.

**Drawing is submission, not painting.** A `Draw` system hands the renderer
quads — `ctx.sprite`, `ctx.rect`, `ctx.line`, `ctx.circle`, `ctx.text` — and
cannot change the world; the type system enforces that. Order comes from
`Depth { layer, z }`, not from the order you drew in, so a debug outline goes in
front by saying so rather than by being drawn last.

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

**Reading while writing: the two-pass pattern.** A query that borrows the world
mutably holds it for as long as you iterate, so a system that needs to look at
*other* entities while changing one reads first and writes second. Collect what
you need into a `Vec`, drop the query, then apply. `examples/homing.rs` is the
worked version, and this is the one shape that surprises people coming from
engines where everything is a global.
