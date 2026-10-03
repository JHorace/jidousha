# ninjo — the substrate

**Read `GDD.md` first.** That document is ninjo's design — the vision, the
vocabulary, the shared-state specs, the module registry and the wave plan.
This one is the **substrate's technical doc**: the tile world, the integer
clock, the one scheduler, the pathfinder and the verify machinery the game
stands on. The two are not merged on purpose — the GDD is what ninjo *is*,
this is what the ground under it *does*, and a change to one is rarely a
change to the other. Where the two disagree about a decided thing, the GDD
wins; where the GDD leaves the substrate's mechanics unstated, this file is
the record. Numeric values are drawer-tunable.

## 0. Where this came from

The substrate was designed and built as **giri-rt**, a crate fork of giri,
to test one hypothesis: giri's information-overload problem was a
*delivery* problem, not a systems problem — a world where parties travel
between places in continuous, pausable time, and where events arrive as
addressed moments rather than simultaneous table-rows, would make the same
social machinery legible when it returned. **Confirmed**: the owner played
the substrate, the fork was adopted as ninjo, giri retired to `attic/giri/`,
and giri's social systems returned across wave 1 as want-mechanics
(`GDD.md`). `VARIANT.md` is the closed record of that question — the
hypothesis, the exit criteria and the verdict — and is kept as such.

What the game carries from giri, by way of the fork: the asset pipeline and
Kenney art (`assets/CREDITS.md`), the floors and verify machinery, the
screenshot process, the tuning drawer with `?constants=`/`?seed=` and the
one-name-per-constant discipline, the stamp conventions, and the interim-UI
standing law (ugly is acceptable; unreadable is a regression). `UI.md`
inherits giri's `attic/giri/UI.md` on the same terms.

*The numbering below is the substrate's original — the code cites these
sections by number. §1, §2 and §11 were folded into §0 and into the sections
they annotated at the wave-1 close.*

## 3. The sim model — a tile-grid world

The world is a **tile grid, and the sim reads it**:

- **The grid** is a rectangular authored map — `grid::MAP`, a 48x27 ASCII
  literal (agent-writable, diffable). Each tile has a terrain kind from a
  small data-defined set (road, plains, forest, rough, water/peak — five
  kinds as six glyphs; water and peak are two pictures, one impassable
  fact); each passable kind carries a **movement cost in world-minutes**, a
  named drawer constant per terrain.
- **One grid, two readers.** The renderer draws the *same* grid data the sim
  consults. There is no separate decorative backdrop and no render-side copy
  — which means the map **cannot lie about terrain**, the same one-source
  discipline that made giri's bands unable to lie about the rolls. Terrain
  renders as one flat colour per kind rather than Kenney tiles, deliberately
  interim (`UI.md` §2 owns that note); the discipline is held by verify
  asserting every drawn tile's fill against the sim's grid.
- **Locations are named tiles**: the town (Kawaza, home base) and the four
  sites (id, display name, tile coordinate, icon role). Events address
  tiles; locations are what give tiles names. The authored terrain makes
  routing visible: the Watchtower's 47-tile all-road route beats the 39-tile
  overland line, and the peak ridge forces the Black Vault detour around
  x=44.
- **Parties** move tile-to-tile. Positional sim state is discrete: a party
  is always *on a tile* — either resident at a location, or following a
  **stored path** (the tile list computed at dispatch, plus the index and
  the scheduled world-time of the next tile entry). Routes out *and home*
  are computed at dispatch and stored; costs are per tile *entered*, so the
  two legs can differ by the endpoints' own costs. Smooth between-tile
  motion is derived at draw time (ADR-0041 interpolation); it is
  presentation, never sim state.
- **Pathfinding is deterministic by construction**: computed once at
  dispatch (terrain is static; re-path-on-change is a later event class),
  4-connected Dijkstra with a *documented* rule — **the frontier pops lowest
  cost, ties by lowest row-major coordinate; neighbours expand N, E, S, W; a
  recorded route is replaced only by a strictly cheaper one** — asserted by a
  deliberate-tie test on a uniform micro-grid. The tie-break is part of the
  design, and never left to hash-map iteration order.

**The grid arrived ahead of mechanical need**, on the owner's explicit call
that tile mechanics (terrain, roads, things that happen *on the way*) were
strongly expected and that retrofitting a grid under a graph-shaped sim later
would be the expensive path. The compensating discipline stands: **the grid's
mechanical surface is exactly passability and movement cost, nothing more** —
no fog, no encounters, no territory. Those wait for the mechanics that need
them.

## 4. Time — the clock is game state, the speed is an input

- The engine's fixed 60 Hz timestep is **untouched**. Ticks always run.
- The game holds a **world clock**: integer world-minutes (never floats),
  plus an integer tick-accumulator. Every tick adds the current speed's
  accumulation (`speed_1x` / `speed_2x` / `speed_4x`; 0 paused) and every
  `minute_ticks` accumulated carries one minute. The shipped set is
  `minute_ticks 30` with accumulations `12 / 24 / 48`: **1x is 24
  world-minutes a real second** — a world-day every minute of wall time —
  and 2x and 4x are exact multiples of it. All four are drawer rows.
- **Speed is player input through the snapshot.** Space toggles pause, 1/2/3
  select speeds, the four chips (PAUSE plus the three rates) do the same —
  all recorded in the `InputSnapshot` like any other input, which makes
  replays carry the player's pacing for free and keeps the determinism
  contract whole. The current speed is game state set by that input — the
  engine knows nothing about it. The scenario **opens paused** (pause is
  consent — the player starts the world).
- **The invariant that makes this deterministic and testable**: every
  scheduled occurrence has a world-time address, and **its world-time is
  independent of the speed schedule**. Fast-forwarding may cross several
  world-minutes in one tick; everything due in the crossed span fires that
  tick, in world-time order, cascades included. Same seed + same inputs at
  the same world-times ⇒ identical event sequence with identical world-time
  stamps, under *any* speed script. This is the substrate's core assertion
  (§7).
- **The one scheduler is `sim.rs`**: every occurrence carries `(world-minute,
  scheduling sequence)` and a tick fires everything due in the crossed span
  in that order.
- **At 4x a tick carries 1.6 world-minutes, so the clock does not visit every
  world-minute.** A recorded input can therefore only be addressed to a
  minute the clock visits at every speed it is asked to be identical under
  (`FINDINGS.md` G-016); the sweep's `When::Approaching` is the other half of
  the answer (§7).
- **Pause is not a freeze of the program** — ticks run, input is processed,
  the camera moves, the UI works; only the world clock holds. Inputs given
  while paused are ordinary recorded input that takes effect at the clock's
  minute.

## 5. What happens in the world

- **Jobs** stand at sites: each site's board is six authored rows (pot,
  duration in world-minutes, name, and a task type — `CAST.md` §2), each
  with a state of its own (`Open` / `Claimed { by }` / `Done { by }`), named
  by `(site, slot)`. A job is claimed by the character who takes it, so two
  cannot take one; **sites run dry** when their list is spent. An industry's
  standing slots are one more site, at the camp (GDD §5).
- **Nobody is dispatched by the player.** The player posts (GDD §3b);
  `sim::dispatch` is reached only by a character who decided to go — their
  own idea, or an ask they heard and agreed to — and it is the one dispatch
  path. A site marker opens that site's board and asks nothing; the job row
  is where a posting is made (`UI.md` §3c).
- A character who goes **travels** (follows the stored path tile by tile,
  each tile entry costing that terrain's world-minutes), **works** the job at
  the site for its duration, **is resolved** (three tiers, a seeded roll with
  the odds shown — GDD §5's resolution; a shift is never rolled), is paid by
  the port the job's kind names (GDD §4.1), and **travels home** to the
  doorstep they left from — which is why journey minutes differ per
  character.
- **Events** are emitted at world-time moments, each carrying **time +
  place + class**: the five movement classes — departed, arrived,
  work-began, quest-complete (with the tier and what was paid), returned —
  and every class the modules added (`attention::CLASSES`). The log renders
  them in mechanical narration (`d1 02:41 - Bob completed the mushroom haul -
  40g into the treasury (40g held) - turning for home`).
- **The treasury** is one visible number; what moves it is GDD §4.1's ports.

## 6. Attention — the substrate's contract

Every event has a world-time, a place (a tile — named when it is a location)
and a class; the feed, the focus and the pulse are presentation over those
addresses, and the feed is a view of `Sim::events`, never a copy. `GDD.md` §3
and `UI.md` §3a own the attention architecture that stands on this.

**One thing here is not presentation, in the substrate's own voice:**
auto-pause. A pause that only the screen knew about would be a replay
divergence — the world would stop for one player and not for another off the
same recorded inputs. So `Sim::emit` records the pause and `sim::fire_due`
puts the clock at speed 0 in the same tick: a simulation transition, with the
per-class config as sim state beside it.

## 7. Verify — the world moves the same way twice

- **The speed-invariance sweep** (`sweep.rs`, the substrate's signature
  test): one authored scenario, one fixed script of inputs at fixed
  world-times, run under three speed scripts — all-1x, all-4x, and a mix
  that changes speed mid-travel, pauses exactly as an input falls due (so
  inputs-while-paused is exercised by the signature test itself) and resumes
  300 ticks later — must produce **identical event sequences with identical
  world-time stamps**. A divergence is the exact failure this design exists
  to prevent. It runs a second time under a config that stops the world at
  every completion, resumed by the key the script already runs at: an
  auto-pause stretches wall time and moves no address. GDD §9 says what the
  modules extended it over.
- **The conductor.** Inputs are world-minute-addressed directives executed
  through a `SnapshotBuilder` — real clicks on real rectangles, three of
  them for a posting (the pick, the marker, the row). An input is addressed
  by `When::Approaching`: the conductor simulates the clock forward the ticks
  the clicks take and starts them early enough to *land* on the minute
  named, because at 4x a click begun when the minute arrives takes effect
  several minutes later, by a different several at every speed.
  `sweep::orders_are_addressable` asserts every scripted minute is one the
  clock reads at 1x, 2x and 4x, so G-016's rule is a check rather than a
  comment. The sweep is judged over a world-time window (`sweep::WINDOW`,
  the scenario's first day), because a world with people deciding things in
  it never comes to rest.
- **Fixed-seed scripts** pin arrival at the exact world-minute (the sum of
  terrain costs along the asserted path), the pay, and the return.
- **Pathfinding unit tests** on authored micro-grids: a road route beating a
  shorter-in-tiles overland route, an unreachable column, **a deliberate tie
  asserting the documented tie-break**, and the authored map's own routes.
- **Pacing probes** pin ticks-per-minute at each speed against shipped
  literals.
- **Determinism hygiene**: the world clock is integer-only; **every `Rng`
  read is addressed by the occurrence** — the seed, the world-minute and what
  the draw is for, mixed before seeding (`FINDINGS.md` G-045) — the
  resolution roll, the petition check's roll and the director's draws, never
  by call order and never by frame; rendered between-tile progress is
  derived at draw time and never written back; there is no second copy of
  the grid anywhere (one grid, two readers). `verify::seed_independence`
  asserts two far-apart seeds part the transcript with resolution on and
  leave it identical with resolution off.
- **Floors** bind the map screen — clock readout, speed chips, node labels,
  party tokens, log lines — and the off-screen floor is restated for a
  roaming camera (`UI.md` §4). Screenshots are recaptured and viewed
  (`UI.md` §5).
- **The mutation round** over the substrate's constants: a perturbed terrain
  cost or speed constant must break an arrival-time assertion.

## 8. Engine expectations: nothing

The engine envelope suffices: Camera pan/zoom exists, ADR-0041 interpolation
covers smooth token motion, and the game culls to camera bounds on its own.
The tile map is likely the largest sprite count a game has asked of the
renderer; if draw throughput turns out to be a real gap it lands as a
FINDINGS entry (G-numbers continue), not as an engine change smuggled into a
game session. **Expectation, not license.**

## 9. Phases

The substrate's own phases (S1 "The World Moves", S2 attention, S3 the
reintegration design) are closed: S2 became wave 0a, S3 produced `GDD.md`,
and the wave plan there (`GDD.md` §8) is the plan. The sections above are the
substrate's own record and stay the authority on the grid, the clock, the
scheduler and the pathfinder.

## 10. Open questions (deliberately deferred)

Terrain-cost and speed constants, and map scale (drawer, playtest) ·
8-connectivity and diagonal costs (4-connected; revisit if paths look dumb)
· re-path-on-terrain-change (an event class for when terrain becomes
dynamic) · map generation (`GDD.md` §10).
