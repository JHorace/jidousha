# ninjo (人情) — Game Design Document

Repo-canonical. Landed at `games/ninjo/GDD.md` by the wave-0b session, which
also renamed the crate from `games/giri-rt`. Drafted at GDD assembly,
2026-08-30, from the brainstorm-1 capsule registry. The vault capsules remain
the *working surface*; when a capsule and this document disagree about
something decided, this document wins and the capsule is stale. Numeric values
are drawer-tunable throughout; shapes are the design.

**`DESIGN.md` beside this file is the substrate's technical doc** — the tile
world, the integer clock, the one scheduler, the pathfinder, the verify
machinery. The two are deliberately not merged: this document is what ninjo
*is*, that one is what the ground under it *does*. Where they disagree about a
decided thing, this document wins; where this document leaves the substrate's
mechanics unstated, that one is the record. `UI.md` owns presentation.

**Mark discipline.** A section that has been built carries an
`*Implemented (wN):*` paragraph naming what landed and where, and saying where
the build bent the shape above it. Everything without such a mark is design
and not yet code. The paragraph is written by the session that built it, in
the same commit.

## 1. Vision

You are the head of a small settlement — responsible for everyone, in
command of no one. Your means of control is **asking people to do
things**, and people have their own traits, wants, debts, friendships,
and dreams. They can refuse you, humor you, or agree and then do
something else entirely. Managing the settlement IS managing the
people; prosperity is capacity, and **quests — structured arcs of what
people want — are the progression**.

Lineage: **giri** (義理, duty) proved that trait-driven willingness
makes characters feel like people, and that its drama needs more room
than four beats. **giri-rt** proved the room: real time with pause, a
map, events with time-and-place addresses. **ninjo** (人情, human
feeling) is the game both were reaching for — giri's classical
counterpart, because the design moved from obligation-mechanics to
want-mechanics. Duty versus heart is the engine of every scene we hope
this game produces.

**The phasing arc (the difficulty thesis)**: the player automates
themselves. Early game: a small cast, hand-cycled parties, everyone
needs work to eat. Mid game: industries scale with population; the
adventuring company absorbs routine threats; the player's unit of
concern grows — heroic threats, difficult people, dreams. The game
never just gets harder; your attention moves up a level. Progressive
difficulty is keyed to settlement development, not a timer.

**Standing principles** (each enforced somewhere in §9):

- One decision function per question; preview and sim share it.
- Coupling through shared state and events only; no module reads
  another's interior. Modules are disableable from day one.
- The sim runs on truth; the player sees through the knowledge lens
  (identity in v1).
- Warnings derive from the numbers that produce consequences — a
  surface that could disagree with the sim is the failure mode.
- The settlement must limp without you.
- Replay determinism is THE contract, module subset notwithstanding.

## 2. Vocabulary (normative)

- **need** — ambient upkeep (data-defined list; v1: coin). A field,
  managed by policy, never by per-person attention.
- **petition** — a voiced, time-gated, addressed request with a
  declared consequence. The ledger of petitions is the quest board.
- **ask** — an order from the player to a character; asks travel.
- **task** — work at a place with a duration (site jobs, industry
  shifts). Tasks resolve; petitions expire; asks are answered.
- **quest** — a **structured arc of linked petitions and events with a
  named payoff** — authored, director-assembled, or emitted by an
  aspiration (milestone arcs are the character-driven case). Site
  tasks are not quests; quests are the stories. MVP contains quest
  *machinery* only insofar as petitions chain (see §6 templates);
  full quest authoring matures with the director.
- **aspiration** — a long-horizon motivator that generates petition
  arcs (a dream, mechanized).
- **the scorer** — the one autonomy decision function.
- **wave** — a topological layer of the build plan (§8).

"Wants" is retired as a mechanical term.

## 3. Foundation (the spine — specified here, built in wave 0/landed)

**Landed (giri-rt S1, played)**: the tile grid the sim reads (one
grid, two readers; locations are named tiles; deterministic
4-connected pathfinding, documented tie-break) · integer world-clock
with speed-as-input (pause/1×/2×/4× through the snapshot; the
speed-invariance sweep is the signature test) · one scheduler; every
occurrence has a world-time address · events carry time + place +
class. `DESIGN.md` is that half's whole record.

**Wave 0b — People** (port + adapt from giri mainline):

- Character registry: id, name, home tile, sprite/icon role.
- **Trait vocabulary**: data-defined; kinds `personality`,
  `motivator`, `aptitude`; traits parameterize decision functions and
  data (upkeep cost, pressure, competence) and never branch code. No
  list cap (attention architecture owns legibility; revisit at the
  MVP gate).
- Per-character: wallet (wealth), desperation + `source` line (the
  proven differentiator), active-petition slot.
- Shared-state stores: regard edges, bonds/grudges (pair-facts),
  marks (person-facts) — see §4.
- **The knowledge lens**: a single read-path every UI surface uses;
  v1 = identity. No screen may read sim state except through it.
- Crate renames `games/giri-rt` → `games/ninjo`; page, sync filters,
  VARIANT.md closed out (substrate adopted; hypothesis confirmed).

*Implemented (w0b):* all of it, and here is what the build decided that this
section left open.

- **The registry** is `src/people.rs` — id, name, home tile, portrait role,
  traits, wallet, desperation, `source`, and the active-petition slot (a
  field, asserted empty until wave 1.3 has something to write into it). The
  "sprite/icon role" is the library's own `Art` role, because in this codebase
  a role *is* the art contract (`src/sprites.rs`: `portrait_tim` is Tim).
  Characters stand at their home tiles on the map, named — idle, because
  autonomy is wave 1. `Lens::at_home` derives who is standing there from who
  is out with a party; nothing is stored twice.
- **The cast is four, and that is content, not architecture.** §7 wants 8–12
  at the MVP scenario. Wave 0b ships one character per committed portrait
  role, and the cast's content joins the trait list in §10's open ledger —
  more people is a data change plus curated art, not a code change.
- **Parties carry a character.** The substrate's three bands are each fielded
  by one member: the strip reads `OX - Bob` and the token is Bob's own
  portrait. Who *else* is in a party, and what their bonds do to its outcome,
  is the parties module (§5, wave 4).
- **The trait vocabulary** is `src/traits.rs`: giri's nine personalities
  ported whole, plus three motivator and three aptitude rows that are
  **placeholder content authored to prove the format**, flagged for the
  trait-content pass. The three-trait sheet cap giri enforced is gone.
- **How the `kind` field avoids becoming a branch**, which this section asks
  for and does not say how to get: every `TraitDef` carries every modifier
  field, and the vocabulary's validation asserts that a row holds the neutral
  value in every field its kind does not own. Consumers therefore apply every
  field of every trait unconditionally — a personality's upkeep multiplier is
  1/1 and drops out — and nobody filters a trait list by kind before using it.
  A kind gates which data a row may carry; it never gates which code runs.
- **The lens** is `src/lens.rs`, and the seam is structural as well as
  documented: `screens::content` takes a `Lens` and no `Sim` at all, so a
  screen cannot read around it because a screen has nothing else to read.
  `--verify` and the sweep read truth directly, by design — they are the
  simulation's instruments, and a check that could only see what the player
  sees could not catch the sim lying to the player.
- **The rename** landed with the page title, the capture prefix, the stamps
  and the committed screenshots. Two stale `games/giri-rt/` strings remain
  outside the crate — in an accepted ADR (superseded, never edited) and in an
  engine example's doc comment (out of a game session's scope). The sync
  filters are the owner's to move.

**Wave 0a — Attention** (mockup-first, then handoff):

- The **feed**: persistent, timestamped, click-to-focus event list;
  every entry carries class + place.
- **Auto-pause per event class**: ignore / log / pause-and-focus,
  player-configurable; defaults chosen in the mockup.
- **Meters and faces**: aggregates for the glance (derived from the
  same per-character truths as the sim), drill-down to characters for
  action. Petition cards show timer + declared consequence.
- Every module's capsule declares its `attention` cost; the feed is
  the budget's ledger.

## 4. Shared state (deep specs)

### 4.1 Wealth (gold; the only v1 currency)

Holders: player treasury, character wallets. **Mint at sources, burn
at sinks, conserved between holders**; the ports, exhaustively:

- MINT: site pots (→ treasury, on task resolution); industry wages
  (→ worker wallets).
- TRANSFER: shares/wages (treasury → wallets, per the dispatch
  offer); petition rewards (petitioner wallet → satisfier); petition
  gifts (treasury → wallet).
- BURN: upkeep (wallets; trait-modulated); industry construction
  (treasury); declared consequences where stated.
- The industry levy knob exists, default 0 (passive income is
  upgraded into).

**Treasury-margin**: pots land in the treasury; promised shares pay
out; the remainder is the player's income — the player as contractor.
The payout surface shows pot / shares / margin from the sim's numbers.

*Implemented (w0b, in part):* the holders exist — a treasury and a wallet per
character — and the trait side of the upkeep burn is `traits::upkeep_of`,
which multiplies a base cost by the carrier's motivators. No port actually
moves gold yet: the mint at site pots is the substrate's stub resolution, and
every other port belongs to needs, petitions, resolution and settlement
(wave 1). The conservation assertion arrives with the first transfer.

### 4.2 Regard (the master currency)

Directed integer edges, char→player and char→char, default 0, range
bounded by facts (below). **Operations** (magnitudes are drawer
constants; classes are the spec):

- Petition satisfied: large + toward whoever satisfied it (and the
  player, when the player arranged it).
- Voiced petition failed: large − toward the player only (obligation
  = voicing; ambient needs never touch regard directly).
- Wage offer vs expectation at dispatch: small ±, through the
  willingness evaluation (the giri willing.rs machinery reintegrated).
- Witnessed acts: via trait×mark reactions (giri v2 table carried).
- Slow drift toward a **fact-set baseline**: a bond raises an edge's
  floor, a grudge lowers its ceiling — the scalar is the mood, the
  facts bound its range.

Regard is also the information network (transitive knowledge), dormant
until the knowledge module.

*Implemented (w0b):* the store and the drift; the five operations are their
callers' business and four of them arrive with the modules that cause them.

- **The edges** are directed integers in `src/stores.rs`, sparse (absent is
  zero), and a target is either another character or the player. There is one
  general write, `adjust_regard`, which holds the result inside the pair's
  bounds — so no caller in any later wave can push an edge past what the facts
  allow, and the bound is decided in one place.
- **The bounds**, made exact: with no facts an edge runs `±regard_span`; a
  bond raises the floor to `bond_floor`; a grudge lowers the ceiling to
  `-grudge_ceiling`.
- **The baseline**, which this section names and does not define: it is
  **zero, held inside the bounds**. A mood with nothing behind it decays to
  indifference; a bond stops that decay at its floor and a grudge at its
  ceiling. A pair holding **both** facts — a friend who wronged you, a real
  state — has crossed bounds, and its baseline is the midpoint of the interval
  between them. That is the arithmetic answer, and it is why the crossed case
  needs no special rule anywhere else.
- **The drift** is integer and bounded: one `drift_step` toward the baseline,
  never past it, never outside the interval, with a fixed point at the
  baseline so a world nobody touches settles instead of oscillating. Its
  cadence is `drift_hours` (stated in hours because the drawer's range is
  small), and it runs **through the one scheduler with a world-time address**,
  so it is speed-invariant like every other occurrence — verify asserts the
  same drift count under all three speed scripts. It emits no event: nothing
  *happened* to anybody, and the feed is for things that did.
- **Bounds are asserted at their bounds**: over every fact-set and every value
  from well outside the widest span, one drift never leaves the interval and
  never moves away from the baseline.

### 4.3 Bonds & grudges (pair-facts) and Marks (person-facts)

Written by events, never by drift: bonds from repeated shared success
plus high mutual regard (threshold event); grudges from betrayal-class
acts, acting against a character's petition, or egregious/repeated
petition failure. Facts do not decay; goal-completion-style erasure
rules are post-MVP design. Marks carry giri v2 semantics unchanged
(public facts; trait×mark reaction table; title-marks arrive with
aspirations). Party outcome effects of bonds/grudges are the parties
module (wave 4); the *stores and write rules* are foundation.

*Implemented (w0b):* the stores and the write rules, with the rules enforced
by what exists rather than by what a comment asks for.

- **The three vectors are private to `src/stores.rs`.** There is no `&mut`
  accessor and no eraser — *facts do not decay* is the absence of a function,
  not a rule somebody has to remember — so the only ways in are the write
  functions below.
- **Bonds**: `record_shared_success` counts a pair's successes and writes the
  bond the first time they reach `bond_after` **and** both edges stand at or
  above `bond_regard`. Both halves are load-bearing and both are asserted:
  repeated success at cold regard writes nothing. A bond is written **both
  ways**, because a one-sided bond is a category error.
- **Grudges**: `record_grudge` is directed and one-sided — the wronged hold
  it, and what the wrongdoer feels is their own business. It takes a cause
  (betrayal / acted against their petition / petition failed — §4.3's three
  sources) as **data on the row, never a branch**: nothing decides differently
  for one cause than another, which is what keeps a fourth source a data
  change. Writing the fact re-holds the edge inside its new ceiling at once: a
  betrayal *is* a grudge when it happens, and letting drift walk warmth down
  over the next several hours would be the sim lying about what just occurred.
- **Marks** are person-facts, idempotent (wearing one twice would double every
  reaction to it), with giri v2's tones and the trait×mark reaction table
  carried unchanged — including the `(pragmatic, skimmer, +2)` cell that makes
  a known skimmer preferable to a stranger, so reactions open doors as well as
  close them.
- The callers — the acts that cause any of this — arrive with their modules.
  Wave 0b builds the doors and the checks that walk through them.

## 5. Module registry

Tier/wave/edges are normative here; capsule bodies hold the working
detail. All modules disableable; "degrades to" per capsule.

| module | tier | wave | requires | reads | writes |
|---|---|---|---|---|---|
| autonomy | mvp | 1 | clock, grid, traits | regard, wealth, bonds, marks | — |
| needs | mvp | 1 | clock, traits | wealth | wealth |
| petitions | mvp | 1 | traits | regard, wealth | regard, wealth |
| resolution | mvp | 1 | grid, clock, traits | wealth | wealth |
| settlement | mvp | 1 | grid | wealth | wealth |
| events-director | mvp | 1 (minimal) | clock | — | — |
| asks | mvp | 2 | autonomy, grid | regard | regard |
| aspirations | post | 3 | petitions | — | marks |
| threats | post | 3 | grid, events-director | — | — |
| arrival | post | 3 | grid, autonomy | — | — |
| parties | post | 4 | asks, resolution | bonds, regard | bonds |
| knowledge | post | exp | knowledge-lens | regard, wealth | — |

Module summaries (one line each; capsules canonical for detail):
**autonomy** — the scorer; actions: seek work, work industry, join
party, pursue/poach petition, socialize, idle; desperation reshapes
weights. **needs** — coin upkeep, trait-modulated; shortfall raises
desperation (escalation pipe). **petitions** — the ledger; three
sources; voicing binds; cliffs with declared consequences.
**resolution** — tasks read aptitude-kind traits; degrades to the
landed stub. **settlement** — capacity; industries as job slots; one
generic industry at MVP. **events-director** — templates with
triggers; scenarios are files; MVP ships 3–4 canned petition-flavored
templates. **asks** — orders travel; the compliance ladder (comply /
shirk / subvert / wander off) via the scorer with the ask as weighted
candidate. **aspirations** — petition-arc generators; the baker
dream; title-marks. **threats** — routine vs heroic; emits petitions.
**arrival** — newcomers and notable movers. **parties** — summons to
rendezvous; who-shows-up as foreshadowing. **knowledge** — lens
parameters; regard-unlock leading.

*Implemented (w0b): the registry as machinery, empty of rows.*
`src/modules.rs` holds the table above's shape (`ModuleSpec`: id, tier, wave,
degrades-to), the per-module disable flags (`ModuleSet`, a bitmask planted as
a resource before Startup like the constants), and the matrix §9 iterates.
**No row of the table above has been built**, so the registry is empty and the
matrix is one pass — which the harness *runs* rather than skips. Adding a
module is adding a row here and reading `ModuleSet::enabled` where the
module's systems and data are installed; the matrix, the stamp and the reports
all walk the table, so nothing else changes.

## 6. Data formats (all data-defined, drawer-tunable, ASCII)

- **Trait row**: id, name, kind (personality|motivator|aptitude),
  icon role, description (stranger-facing gist), modifier set
  (scorer weights, upkeep multiplier, aptitude values, reaction rows).
- **Petition/event template** (one format — the director speaks in
  petitions): id, source class (motivator|shortfall|director),
  trigger (state predicates + world-time window + seeded roll),
  petition body (text template, deadline, reward spec, declared
  consequence — itself a template reference), optional `next` links
  (this is quest chaining: a quest is a template arc).
- **Scenario file**: seed, map, roster, starting balances, pinned
  template firings (world-time or predicate), director on/off +
  pressure params. The tutorial is the most-pinned scenario;
  freeplay is the least.
- **Needs list**: kind, interval, base cost (v1: one row — coin).

*Implemented (w0b): the trait row only.* Its modifier set is split by kind as
this section says — bond and grudge multipliers plus the pot's pull for a
personality, an upkeep multiplier and a scorer pressure for a motivator, a
competence value for an aptitude — with the reaction rows in their own table
keyed by trait id. The description is validated as one stranger-facing ASCII
line. The other three formats arrive with the modules that read them; the
roster and starting balances are authored in code until the scenario file
lands (§8, wave 1.5).

## 7. The MVP

**Modules**: foundation + waves 1–2 (autonomy, needs, petitions,
resolution, settlement-with-one-industry, minimal injector, asks).
**Scenario**: one town, 3–5 sites, 8–12 characters, authored
templates. **The loop under test**: watch people live → hear
petitions → ask people to work → set wages → watch compliance → spend
margin → keep everyone fed enough — under mild injected pressure.

**Gates**: wave gates are alive-and-correct (verify green with each
module off; sweeps green; the world runs and is watchable). The MVP
gate is the first **fun** judgment, owner-played, with one question:
*is being the person everyone asks things of, who rules only by
asking, a loop you want to keep playing?* If no, the postmortem is at
the plan level. Fresh-eyes testers are NOT spent here unless the owner
gate passes (playtester budget).

## 8. The wave plan (sessions are sequential within a wave; one
session per handoff stands)

- **0b People** — port/adapt from giri; stores; lens seam; rename to
  ninjo. **Done.**
- **0a Attention** — interactive mockup first (design side, runs the
  real event data shapes), then the handoff. (Next handoff.)
- **1.1 autonomy** → **1.2 needs + settlement** (one session — the
  economy loop halves are one concern) → **1.3 petitions** → **1.4
  resolution** → **1.5 injector**. Each lands into a running world;
  owner sanity-plays between sessions but fun is not judged.
- **2 asks** → **MVP gate playtest.**
- **3+** aspirations / threats / arrival (any order) → **4 parties**
  → knowledge when the experiment is wanted. Re-derive waves at each
  GDD refresh; the registry is the source.

## 9. Verify strategy

- **Module-off matrix**: verify runs the full suite with each module
  individually disabled — green is the definition of modular.
- **Speed-invariance sweep** (landed) over every new event source.
- **Economy sweeps**: ~200 idle-player seeds; wallet/treasury bands;
  nobody-starves-at-subsistence (the limp-floor as an assertion); a
  mutated wage or upkeep constant must break a band.
- **Distribution sweeps** over the compliance ladder when asks land
  (P2 machinery, right horizon this time).
- Floors + screenshot process on every surface; stamps carry seed,
  constants, variant, module set.
- One-source checks: any warning surface asserted equal to its
  consequence's inputs (band-chip discipline, generalized).

*Implemented (w0b):* the module-off matrix (one pass, empty registry: it plants
a `ModuleSet`, conducts a real run under it and asserts the world moved and
came to rest — the everything-on pass additionally asserts the authored
timeline, because a module being off is *supposed* to change what happens);
the speed-invariance sweep extended over drift, which adds no event source and
must therefore leave the transcript identical; floors over the new surfaces
(character figures and their names, the party strip's member line, the drawer
at seventeen constants) and a sixth screenshot, the settlement before anything
is dispatched. **Stamps carry seed, constants and the module set** — the
verify report and the scenario's opening log line both. "Variant" has nothing
left to say: the fork was adopted and `VARIANT.md` is closed.

The mutation round grew with the drawer: it walks every constant, and the nine
new ones are seen by two new instruments — the trait arithmetic and the store
battery — both written with **shipped literals**, because a check that derives
its expectation from the constant under test cannot see that constant move.
17 of 17 noticed.

Economy sweeps wait for an economy; distribution sweeps wait for the ladder.

## 10. Confidence & open ledger

Foundation: grid/clock/pathfinding **played**; people substrate
**played** (in giri; ported and green here); attention **speculative**
(mockup will raise to mocked). All modules **speculative** — correct
and expected; wave gates convert speculation to played evidence one
wave at a time.

Open (deliberately): attention-mockup decisions (auto-pause defaults,
feed layout, petition-card anatomy) · scorer cadence · **trait list
content, and the cast content beside it** — who ninjo's people are, how
many of them, what they carry, and the portrait art more of them would
need (wave 0b ships four and six placeholder trait rows) ·
aptitude-change mechanism (two candidates recorded) · bond/grudge
erasure rules · quest authoring surface beyond template `next` links ·
settlement stock list beyond gold-only (bound to a famine/siege design
need) · map generation (post-GDD session; requirements now stateable:
one town, sites, terrain variety, readable at 8–12 characters' scale).
