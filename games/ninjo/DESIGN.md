# ninjo — the substrate (design session, 2026-08-27, as giri-rt)

**Read `GDD.md` first.** That document is ninjo's design — the vision, the
vocabulary, the shared-state specs, the module registry and the wave plan.
This one is the **substrate's technical doc**: the tile world, the integer
clock, the one scheduler, the pathfinder and the verify strategy the game
stands on. The two are not merged on purpose — the GDD is what ninjo *is*,
this is what the ground under it *does*, and a change to one is rarely a
change to the other. Where the two disagree about a decided thing, the GDD
wins; where the GDD leaves the substrate's mechanics unstated, this file is
the record.

Written as the design for a tier-3 variant fork of giri, under the name
**giri-rt**, and kept in its own voice: the hypothesis it was written to
test is CONFIRMED and the fork was adopted as ninjo (`VARIANT.md` records
the verdict). Owner-approved direction; numeric values are drawer-tunable
starting points.

## 0. Why this exists (the P2 verdict, recorded honestly)

The P2 playtest returned a null result with a diagnosis. The ladder
variant *is* the deterministic variant with different tuning — and when
that tuning runs against the player, it locks a fail state into a game
with only four beats, where being locked out just feels broken. Two
causes, and the mechanics are the smaller one:

1. **The horizon is too short.** Four beats cannot absorb a bad outcome;
   probability without time to regress to the mean is just punishment.
2. **The information presentation is wrong** — and this is the deep one.
   giri asks the player to track desperation, traits, marks, regard,
   pot math, and pressure *simultaneously, in one screen, at one
   instant*. Sim and grand-strategy games solved this problem decades
   ago: **space and time provide natural, intuitive divisions between
   game events.** A thing that happens *somewhere* at *some moment* is
   legible; the same thing presented as one more row in a table is not.

So the pivot: set giri's unique mechanics aside — deliberately, and
with intent to bring them back — and build the delivery structure they
should have been arriving through. Real time with pause plus a world
map is a known quantity; this phase is not novel and is not trying to
be. **The fundamentals we are nailing are the event-delivery system**:
time spreads events out, space gives them addresses, and pause gives
the player consent over when to think.

## 1. Charter and hypothesis

**Hypothesis**: giri's information-overload problem is a delivery
problem, not a systems problem. A world where parties travel between
places in continuous (pausable) time, and where events arrive as
addressed moments rather than simultaneous table-rows, will make the
*same* social machinery legible when it returns.

**What the substrate is**: the substrate only. A map, a clock, moving
parties, arriving events, and the attention machinery (S2) that routes
the player's eye. The social layer is stubbed: parties succeed, pots
pay, nobody refuses, nobody betrays.

**What "decided" looks like** (the VARIANT.md exit): the owner plays a
build where dispatching parties, watching the world run, and being
interrupted at the right moments feels fundamentally sound — at which
point the reintegration design pass begins (willingness at dispatch,
betrayal on the road, goals as destinations), absorbing the parked P3
material. If the substrate feels dead even with good pacing, the fork
retires to `attic/` and we learn cheaply.

## 2. Fork mechanics (variants policy §8b, tier 3)

- `games/ninjo/` (`games/giri-rt/` when this was written) is a **crate
  fork** of `games/giri/`, carrying a
  `VARIANT.md` (hypothesis + exit criteria) per the policy. The two-alive
  budget holds: giri mainline (parked but green) + the fork. Wave 0b closed
  the variant out and the budget with it.
- **Kept from giri**: the asset pipeline and Kenney art (CREDITS.md
  rides along), the floors and verify machinery, the screenshot process,
  the tuning drawer with `?constants=`/`?seed=` and the one-name-per-
  constant discipline, the stamp conventions, the interim-UI standing
  law (ugly is acceptable; unreadable is a regression).
- **Stripped to stubs**: willingness, traits, marks, regard, the
  betrayal ladder and variant module, strain, the beat chain. Stub
  resolution: a party at a quest site for the quest's duration succeeds
  and pays its pot. Delete aggressively rather than commenting out —
  the code lives in giri mainline; the fork's diff should read as *the
  substrate*, not as giri-with-holes.
- giri mainline is **not** touched. Its tests stay green; its published
  page stays up. The fleet gains one published page: the fork's (`ninjo`
  since wave 0b).

## 3. The sim model — a tile-grid world

The world is a **tile grid, and the sim reads it**:

- **The grid** is a rectangular authored map (an ASCII map literal in
  data is the recommended authoring form — agent-writable, diffable).
  Each tile has a terrain kind from a small data-defined set (road,
  plains, forest, rough, water/peak); each kind carries a passable
  flag and a **movement cost in world-minutes** (drawer constants, one
  per terrain).
- **One grid, two readers.** The renderer draws the *same* grid data
  the sim consults (terrain kind → Kenney tile). There is no separate
  decorative backdrop and no render-side copy — which means the map
  **cannot lie about terrain**, the same one-source discipline that
  made P2's bands unable to lie about the rolls.
- **Locations are named tiles**: the town (home base) and quest sites
  (id, display name, tile coordinate, icon role). Events address
  tiles; locations are what give tiles names.
- **Parties** move tile-to-tile. Positional sim state is discrete: a
  party is always *on a tile* — either resident at a location, or
  following a **stored path** (the tile list computed at dispatch,
  plus the index and the scheduled world-time of the next tile entry).
  Smooth between-tile motion is derived at draw time (ADR-0041
  interpolation); it is presentation, never sim state.
- **Pathfinding is deterministic by construction**: computed once at
  dispatch (terrain is static in S1; re-path-on-change is a later
  event class), 4-connected, Dijkstra/A* with a *documented* expansion
  order (N, E, S, W) and tie-break (lowest cost, then row-major
  coordinate). The tie-break is part of the design — assertable, and
  never left to hash-map iteration order.

*Implemented (S1):* the grid is a 48x27 ASCII literal (`grid::MAP`) with
the five-kind set as six glyphs — water and peak are two pictures, one
impassable fact; each passable kind's cost is a named drawer constant. One
town (Ebisu) and four sites are named tiles; three parties field. The
authored terrain makes routing visible exactly as asked: the Watchtower's
47-tile all-road route beats the 39-tile overland line, and the peak ridge
forces the Black Vault detour around x=44. The pathfinder is 4-connected
Dijkstra with the documented rule — **the frontier pops lowest cost, ties
by lowest row-major coordinate; neighbours expand N, E, S, W; a recorded
route is replaced only by a strictly cheaper one** — asserted by a
deliberate-tie test on a uniform micro-grid. Routes out *and home* are
computed at dispatch and stored. One deliberate interim: terrain renders as
one flat colour per kind rather than Kenney tiles (the packs live on the
owner's machine; no terrain region is in the curated set) — the
one-grid-two-readers discipline is held by verify asserting every drawn
tile's fill against the sim's grid, and real tiles are an import away
(UI.md §2).

Owner decision on record: the grid arrives **ahead of mechanical
need**. The menu-not-backlog policy would normally defer tile
simulation until a mechanic demanded it; the owner made the explicit
call that tile mechanics are strongly expected (terrain, roads, things
that happen *on the way*), and retrofitting a grid under a
graph-shaped sim later would be the expensive path. The compensating
discipline: **S1's mechanical surface of the grid is exactly
passability and movement cost, nothing more** — no fog, no encounters,
no territory. Those wait for the mechanics that need them.

## 4. Time — the clock is game state, the speed is an input

- The engine's fixed 60 Hz timestep is **untouched**. Ticks always run.
- The game holds a **world clock**: integer world-minutes (never
  floats), advanced each tick by the current **speed**: `pause` (0),
  `1x`, `2x`, `4x` — world-minutes-per-N-ticks as named constants in
  the drawer (starting point: 1x = 1 world-minute per 30 ticks, i.e.
  one world-hour ≈ 30 wall-seconds; tune by feel).
- **Speed is player input through the snapshot.** Speed changes
  (keyboard: space toggles pause, 1/2/3 select speeds; clickable chips
  too) are recorded in the InputSnapshot like any other input, which
  makes replays carry the player's pacing for free and keeps the
  determinism contract whole. The current speed is game state set by
  that input — the engine knows nothing about it.
- **The invariant that makes this deterministic and testable**: every
  scheduled occurrence (an arrival, a resolution completing) has a
  world-time address, and **its world-time is independent of the speed
  schedule**. Fast-forwarding may cross several world-minutes in one
  tick; everything due in the crossed span fires that tick, in
  world-time order. Same seed + same orders at the same world-times ⇒
  identical event sequence with identical world-time stamps, under
  *any* speed script. This is the substrate's core assertion (§7).
- **Pause is not a freeze of the program** — ticks run, input is
  processed, the camera moves, the UI works; only the world clock
  holds. Orders issued while paused are therefore ordinary recorded
  input that takes effect immediately. (S2's orders-while-paused is
  thus a property of the model, not a feature to build.)

*Implemented (S1):* the clock is integer world-minutes plus an integer
tick-accumulator: every tick adds the current speed's accumulation
(`speed_1x`/`speed_2x`/`speed_4x`, 0 paused) and every `minute_ticks`
accumulated carries one minute — 1x is exactly the stated 1 minute per 30
ticks, and the constants are drawer rows. Space toggles pause, 1/2/3 set
the rate, the chips do the same, all through the `InputSnapshot`. The
scenario **opens paused** (pause is consent — the player starts the
world). The one scheduler is `sim.rs`: every occurrence carries
`(world-minute, scheduling sequence)` and a tick fires everything due in
the crossed span in that order, cascades included. The speed-invariance
sweep is `sweep.rs` and runs on every verify.

## 5. What happens in the world (S1 scope)

Deliberately thin — enough that the world visibly runs:

- **Quests** exist at quest-site nodes (authored for S1; a small
  generator can wait): pot, duration in world-minutes, site.
- **Dispatch**: the player selects an idle party in town and sends it
  to a site with an open quest. That is the entire order vocabulary.
- A dispatched party **travels** (follows its computed path tile by
  tile, each tile entry costing that terrain's world-minutes), **works**
  the quest at the site for its duration, **succeeds** (stub — no
  willingness, no failure, no betrayal), collects the pot, and
  **travels home**.
- **Events** are emitted at world-time moments, each carrying
  **time + place**: departed, arrived, work-began, quest-complete
  (with pot paid), returned. S1 presents them as a timestamped log in
  mechanical-narration style; S2 builds the attention machinery on top
  of exactly these addresses.
- Treasury: pots accumulate into a single visible number. No spending.

*Implemented (S1):* seven authored quests over the four sites (pot,
duration, name — data in `sim.rs`); a quest is claimed at dispatch, so two
parties cannot take one; **sites run dry** when their list is spent (the
§10 open question, resolved to the simpler choice for S1 and noted in the
PR). Dispatch is two clicks — an idle party on the strip, then a site's
marker — and a refused order bounces with its reason. The five event
classes land with world-time + tile + named location on every entry; the
log renders them in mechanical narration (`d1 02:41 - OWL completed the
mushroom haul - 40g into the treasury (40g held) - turning for home`).

## 6. Attention (S2 — designed now, built after its mockup)

Recorded as architecture so S1 lays the right rails; S2 gets the full
mockup-first treatment (agreement 8) after the owner has felt S1 run.

- **Events are the unit of attention.** Every event has a world-time, a
  place (a tile — named when it's a location), and a class. S1's five classes are the seed;
  reintegrated giri mechanics become new classes (refusal, betrayal,
  goal-milestone...), which is precisely how the social layer plugs
  back in.
- **The feed**: a persistent, timestamped, click-to-focus event list —
  clicking an entry jumps the camera to the event's place.
- **Auto-pause is per event class, player-configurable** (the Paradox
  convention): each class is *ignore / log / pause-and-focus*. Defaults
  are an S2-mockup question, not a guess made now.
- The S1 log must already carry class + place on every entry so S2 is
  a presentation layer over existing data, not a sim change.

*Implemented (wave 0a, which is what S2 became — `GDD.md` §3 is the record):*
the class table, the feed as a view of `Sim::events`, click-to-focus, the
per-class config, the meters and the character panel. **One shape here was
bent and it is worth stating in the substrate's own voice**: this section
calls attention "a presentation layer over existing data", and the auto-pause
half of it is not. A pause that only the screen knew about would be a replay
divergence: the world would stop for one player and not for another off the
same recorded inputs. So `Sim::emit` records the pause and `sim::fire_due`
puts the clock at speed 0 in the same tick — a simulation transition, with
the per-class config as sim state beside it. The rest of the section holds
exactly as written: the feed, the focus and the pulse are presentation over
addresses S1 already carried.

## 7. Verify — the world moves the same way twice

- **The speed-invariance sweep** (the substrate's signature test): one
  authored scenario, one fixed InputScript of orders at fixed
  world-times, run under several speed scripts (all-1x, all-4x, a
  mid-travel mix with pauses) — transcripts must contain **identical
  event sequences with identical world-time stamps**. A divergence is
  the exact failure this design exists to prevent. *(Wave 0a runs all three
  a second time under a config that stops the world at every completion,
  resumed by the key the script already runs at: an auto-pause must stretch
  wall time and move no address, and the transcripts must come out the same
  as the ones above.)*
- Fixed-seed beats-style scripts: dispatch, assert arrival at the exact
  world-minute (the sum of terrain costs along the asserted path),
  assert pot paid, assert return.
- **Pathfinding unit tests** on authored micro-grids: known shortest
  paths (a road route beating a shorter-in-tiles overland route), an
  unreachable case, and **a deliberate tie asserting the documented
  tie-break** — the test that keeps route choice out of hash-order's
  hands.
- Floors bind the map screen: clock readout, speed chips, node labels,
  party tokens, log lines — all pass the readability assertions.
- Screenshots recaptured and personally viewed, as established:
  the map with parties mid-travel, the log after a completed quest.
- Determinism hygiene: world clock integer-only; no `Rng` reads in S1
  (the plumbing stays; the seed still stamps everything); rendered
  between-tile progress derived at draw time and never written back;
  no second copy of the grid anywhere (one grid, two readers).
- Mutation round over the new constants: a perturbed terrain cost or
  speed constant must break an arrival-time assertion.

*Implemented (S1):* all of it, plus the conductor that makes it runnable
(`sweep.rs`): orders are world-minute-addressed directives executed through
a `SnapshotBuilder` — real clicks on real rectangles — and the sweep's three
speed scripts (all-1x, all-4x, and a mix that changes speed mid-travel,
pauses exactly as one order falls due, and resumes 300 ticks later) assert
byte-identical transcripts. The fixed script pins all twenty events to the
minute and the treasury to the gold; the pathfinding battery covers the
tie, the road-beats-overland miniature, the unreachable column, and the
authored map's own routes; pacing probes pin ticks-per-minute at each
speed against shipped literals; seed independence is asserted at seeds 7
and 7,777,777; the mutation round notices all eight constants. Floors bind
clock, chips, labels, tokens and log; the off-screen floor is restated for
a roaming camera (UI.md §4). Screenshots: the mid-travel map (two parties
on visibly different routes), the log after completed quests, both at two
surfaces, plus the tuning drawer.

## 8. Engine expectations: nothing

The engine envelope should already suffice: Camera pan/zoom exists,
ADR-0041 interpolation covers smooth token motion, and the game can
cull to camera bounds on its own. The tile map is likely the largest
sprite count a game has asked of the renderer so far — culling to
camera bounds is the game's job, and if draw throughput turns out to
be a real gap it lands as a FINDINGS entry (G-numbers continue), not
as an engine change smuggled into a game session. **Expectation, not
license.**

## 9. Phases

- **S1 — "The World Moves"** — **Implemented (S1)**, this build: fork + strip; the grid
  with terrain costs and deterministic pathfinding, clock,
  speeds-as-input, dispatch, travel, stub resolution, events with
  time+place, timestamped log; pan/zoom map screen; interim UI under
  floors; the speed-invariance sweep.
- **S2 — "Attention"** (mockup first, then handoff): the feed,
  click-to-focus, per-class auto-pause config, plus whatever the S1
  playtest surfaces about pacing.
- **S3 — reintegration design pass** (design session, not a handoff):
  giri's social systems mapped onto the substrate's addresses; absorbs
  the parked P3 material (two-tier goals, two run modes). Only reached
  if the exit criteria in VARIANT.md are met.

**S3 happened, and produced a game rather than a patch.** The exit criteria
were met, the substrate was adopted, and the reintegration design is
`GDD.md` — ninjo. The wave plan there supersedes the S-phases above from S3
onward: S2's attention work is wave 0a, and giri's social systems return
across waves 1–4 as want-mechanics rather than obligation-mechanics. The
sections above stay as the substrate's own record and are still the
authority on the grid, the clock, the scheduler and the pathfinder.
`GDD.md` §3 marks what wave 0b built on top of them.

## 10. Open questions (deliberately deferred)

Auto-pause defaults and feed layout (S2 mockup) · terrain-cost and
speed constants, and map scale (drawer, playtest) · 8-connectivity and
diagonal costs (S1 is 4-connected; revisit if paths look dumb) ·
re-path-on-terrain-change (an event class for when terrain becomes
dynamic; not S1) · whether quest sites regenerate quests in S1 or the
scenario simply runs dry (S1 may pick the simpler; note it in the PR) ·
multiple simultaneous parties count for S1 (minimum two — simultaneity
is the point).

## 11. Implementation notes — S1, "The World Moves" (2026-08-27)

Everything here is a place where implementing this document decided
something it left open, or bent a stated shape and says so. The changes are
inline above; this is the index, and the PR that landed the slice lists
the deviations one line each.

- **§3: terrain renders as flat colour tiles, not Kenney tiles.** The
  owner's packs live on the owner's machine and the repo's curated set has
  no terrain regions; one named colour per kind ships instead, and the
  one-grid-two-readers claim is held by verify asserting every drawn
  tile's fill against the sim's grid. Curating real tiles is an
  `art/import_pack.py` run away and changes no code.
- **§3: the pathfinder's rule, made exact**: lowest cost, then lowest
  row-major coordinate at the pop; N, E, S, W at the expansion; a route
  replaced only by a strictly cheaper one. The deliberate-tie test pins
  the east-first outcome on a uniform grid.
- **§3: the route home is computed at dispatch too** and stored beside
  the route out — "computed once at dispatch" applied to the whole
  journey, and costs are per tile *entered*, so the two legs can differ
  by the endpoints' own costs.
- **§4: the clock is an integer accumulator**, `accum += speed;
  minute per minute_ticks accumulated` — 2x and 4x are exactly two and
  four times 1x, every world-minute is visited at some tick at the
  shipped speeds, and a tick that accumulates past several boundaries
  fires the whole span (the scheduler handles cascades regardless).
- **§4: the scenario opens paused.** The design says pause is consent
  over when to think; the build starts at the fullest version of that —
  the player starts the world.
- **§4: speed chips are four** (PAUSE plus the three rates); PAUSE
  toggles like space, a rate chip resumes like its key.
- **§5: sites run dry** (the §10 open question): seven authored quests
  over four sites, claimed at dispatch, no regeneration in S1.
- **§6: S2's rails are laid as data**: every event carries world-time,
  tile, named location and class; the log is one presentation of that
  vector, and the sweep's transcripts are another.
- **§7: the sweep's mixed script pauses exactly when an order falls
  due**, so orders-while-paused is exercised by the signature test
  itself, not by a separate case.
- **§7: the off-screen floor is restated for a roaming camera** (UI.md
  §4): chrome inside the UI rect; map content culled to the view; zoomed
  in, fewer tiles submitted. giri's everything-inside-the-design-rect
  form assumes a fixed camera the fork does not have.
- **The chrome rides a UI mapping** (UI.md §1): giri's 960x540 design
  rect became a floating UI space fitted inside the camera's view, which
  is how the floors stay stated in reference pixels while the map pans
  underneath. At the default zoom on a 16:9 surface the mapping is the
  identity up to translation.
