# ninjo (人情) — Game Design Document

Repo-canonical. Landed at `games/ninjo/GDD.md` by the wave-0b session, which
also renamed the crate from `games/giri-rt`. Drafted at GDD assembly,
2026-08-30, from the brainstorm-1 capsule registry. The vault capsules remain
the *working surface*; when a capsule and this document disagree about
something decided, this document wins and the capsule is stale. Numeric values
are drawer-tunable throughout; shapes are the design.

**`CAST.md` beside this file is the cast bible** — who lives in Kawaza, what
the trait words mean, the seeded relationships and the first petition
templates. It is content where this document is design; where the two disagree
about a decided thing, this one wins, and `CAST.md`'s vocabulary is
**provisional through the wave-1 close** (its §7 carries the question the
playtest asks of it).

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

*Implemented (w1.1):* the content, from `CAST.md`. The placeholder rows are
deleted and the vocabulary is the founding band's: **four aptitudes whose ids
are the task ids** (`fight`, `labor`, `scout`, `craft` — so a job of a type
reads the row of that name and nothing maps between two vocabularies), **five
motivators**, each carrying a new `favors` field (a task type, or `any` for any
paid work, neutral `none`) that is the whole of a want's reach into the scorer,
and giri's nine personalities kept whole — six worn, three *parked* (`pious`,
`pragmatic`, `upright`; `people::PARKED`), which are rows the trait x mark
table still references and nobody carries until marks are common. The registry
asserts `CAST.md` §7's coverage matrix — every aptitude and motivator on at
least two sheets, every kept personality on at least one, every parked one on
none — and the **no-dead-motivator rule** against `traits::TEMPLATED_MOTIVATORS`
(wave 1.3 repoints that constant at the real template table and the assertion
does not change). **The cast is ten**, at their own homes south of the ford,
and the town is **Kawaza**.
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

*Implemented (w0a):* all of it except the petition cards, which are wave
1.3's to build to the anatomy recorded in §6 below. What the build decided
that the mockup and this section left open:

- **The event-class table is `src/attention.rs`**, one row per class carrying
  id, colour role, icon role and the mode it opens on, and **nothing in the
  game branches on a class id**: a screen asks the row how to draw it and the
  scheduler asks the config what it does. A wave-1 class is a variant and a
  row. S1's five classes are migrated onto it, so `EventClass` now lives
  there rather than in `sim.rs`.
- **The mockup's defaults, shipped**: movement (`departed`, `arrived`,
  `work-began`, `returned`) is `ignore` because the map already shows motion;
  `quest-complete` is `log`. **No class this build has opens on
  pause-and-focus**, because the petition/consequence family that does is
  wave 1.5 — so a shipped scenario never stops itself until the player asks
  it to in the config panel, which is the mockup's answer rather than an
  oversight. *(Superseded by wave 1.2: `ask-declined` opens on
  pause-and-focus, and is the first class that stops a shipped scenario. Being
  told no, by somebody you asked by name, with the reason on the banner, is
  the one thing this build has that a player must not miss — and the config
  panel is where they say otherwise.)*
- **The default modes are table data and not drawer rows**, which is the one
  place the build bent the handoff. "Drawer-overridable" would have been a
  second way to do a thing that already has one — the config panel is a live
  override *and* a recorded input, where the drawer's would be a restart —
  and it would have cost five stepper rows the drawer's two columns do not
  have. The two attention constants that *are* drawer rows are `feed_cap`
  and `pulse_tenths`.
- **The feed is a view and has no state**: `attention::feed` derives its
  entries from `Sim::events` every time it is asked, so there is nothing that
  could be stale or disagree, and `flow.rs`'s event-copying system is gone
  entirely. The old `Flow::log` survives as the *notices* trail — speed
  changes, refused orders, restarts — which are the things that did not
  happen in the world and have no world-time or place; it is drawn in a
  separate band of the same drawer, and the one-source assertion is over the
  feed alone.
- **Auto-pause is a transition inside the scheduler.** `Sim::emit` records
  the pause on the sim when a firing event's configured mode says so, and
  `sim::fire_due` puts the clock at speed 0 in the same tick — no synthetic
  input, so a replay reproduces the pause rather than a click nobody made.
  The reason and the pause count are sim state; the player's next speed
  input clears the reason. The first pause-class event of a crossed span is
  the one that stops the world, and the rest of the span still fires: a pause
  holds the future, never the present.
- **The config is sim state**, written only by clicking a radio in the config
  drawer, which is a recorded input like a speed change.
- **The surfaces**: three drawers over one map now (feed, auto-pause config,
  tuning), never two at once, and a click that is not one of a drawer's own
  controls shuts it. Over the map: a meters band, a pause banner, the faces
  list a chip opens, and one character's panel. `UI.md` §3a owns the shapes.
- **Selection is presentation** — a click on a figure or a face row opens the
  panel and rings the map sprite. **Tap works and this game wrote no touch
  code**: the engine mirrors the first finger onto the primary pointer, and
  `verify::touch_selects` asserts a finger on Steve's doorstep selects Steve
  through the same hit-test a mouse uses.
- **TTF was available and was not adopted.** The owner's verdict against
  proportional display faces for dense information stands, and the feed is
  the densest surface in the game; the built-in bitmap face is what every row
  is measured and asserted at. The engine feature having landed is not a
  reason to spend it here.

*Implemented (the legibility session, 2026-09-09):* **the scorer's own terms
are an inspectable surface.** The feed said what somebody did and why in one
sentence — the loudest term of a sum with five or six terms in it, with the
rest computed and discarded. It is the never-lies invariant (§1) applied to a
decision: a surface that shows a number the world disagrees with is the
failure this architecture exists to refuse, and a *reason* that stands for an
arithmetic nobody can see is the same failure one level up, because the player
cannot tell a scorer that weighed them wrongly from one they misread. So a
decision now carries the `Judged` it was made from, recorded on the
occurrence beside the reason (`autonomy::Reckoning`, `Sim::remember`), and one
tap on the entry shows every term with the trait row or the fact that produced
it. The attention cost is a `?` at the end of a row and nothing else: the feed
still spends one line per occurrence, which is the budget this section sets.
The same band explains a posting the player has not made yet from the job
row's own verdict (`UI.md` §3e), out of the one `answers::read` that produced
that verdict — one derivation, so a preview and a record cannot describe one
scorer two ways.

## 3b. How the player acts — postings

**The player never orders. The player posts.** A posting is an entry on the
player's ledger:

- **who** — a named character, or *anyone*;
- **what** — a specific job, a site (any open job there), or a task type (any
  open job of that kind, anywhere);
- **until** — *done* (one fulfilment closes it) or *withdrawn* (it stands, and
  keeps recruiting, until the player takes it down);
- **wage** — gold per fulfilment, paid from the treasury to the worker's
  wallet on the job's completion (the §4.1 TRANSFER port); defaults to the
  **standing rate** for the job's task type.

Three usages, one mechanism: **targeted** (who = someone) is the contract;
**open** (who = anyone) is the bounty; **standing** (until = withdrawn) is the
repeat-until-told-otherwise order. A standing open posting for a task type at
the standing rate is the closest thing to policy the player has by hand; the
standing rates themselves are the policy.

**Standing rates** — one per task type (`fight`, `labor`, `scout`, `craft`) —
are the lever for the mass: raise fight pay and the fighters drift to the
crypt without anybody being named. (Settlement's per-industry wage, 1.3, is
the same lever for camp work.) A named posting is the exception, not the
routine.

**Hearing.** A posting binds nobody until it is heard (the petitions rule,
mirrored). Open postings are heard at camp: a character at home weighs the
board on their next rescore. A targeted posting to somebody in camp is heard
at once; to somebody away, it rides a messenger and is heard on arrival at
their location (asks travel, decided 2026-08-29). Withdrawal is instant on the
ledger, and heard the same way.

**Deciding.** The scorer weighs a heard posting as a candidate beside
everything else it weighs — the same one function, the ask as a
heavily-weighted candidate. Its terms, from data: the wage's pull (the pot
term, by `pot_affinity` and desperation — a wage is a pot the player fills); a
**regard-for-player term** (the loyal comply for less; the cold need paying);
fit (the task's aptitude row); a **targeted bonus** (being asked by name
weighs more than a notice on a board — a drawer constant; this is the
"obligation" of an ask without making it an order). Three outcomes:

- **agree** — the character takes the job through the ordinary dispatch loop;
  the event says why in words ("Ludo took the mushroom haul — asked, and needs
  the money");
- **decline** — the scorer chose something else; a targeted decline emits
  `ask-declined` with the reason ("Tim won't — the pay was short and he's owed
  already"); an open posting simply goes unfilled;
- **drop** — a character on a posting's job abandons it when something presses
  harder (this wave: nothing presses harder than work, so `drop` is the event
  class and the seam; needs and petitions supply the pressure).

**Regard.** Asking is free and declining is free (the ask-spam question stays
open; nothing here spends regard for the asking). Payment moves regard through
the wage-vs-expectation rule already in §4.2: expectation is the standing rate
for the task type; paying above it small +, below it small −, whether or not
they took it for less. Shirk and subvert — the betrayal-ladder rungs — are
parked fills until marks are common.

**Surfaces.** The site panel's job row is where a posting is made: with a
character selected, the one-tap gesture that ordered them before now **posts
to them at the standing rate**, and the row reads what the scorer makes of it.
The **postings ledger** is a drawer: every posting, its who/what/wage/until,
who heard it, who answered and why; withdraw from the row. The **standing
rates** panel is four rows.

**Degrades to** (asks off): pure observation; no ledger, no postings; the site
panel is read-only. Verify runs the matrix that way.

*Implemented (w1.2):* all of it, and here is what the build decided that this
section left open.

- **The record and the rules are `src/asks.rs`; the answering and the money
  are `src/answers.rs`.** Two files, one module, because the record and what
  happens once somebody has heard it are two subjects and the file was the
  length of both. `Postings` is a private vector on the `Sim` with named
  writes, the way `stores.rs` holds regard: a posting exists by `post`, ends
  by `withdraw`, and is answered by nothing else.
- **Hearing an ask is being asked.** A targeted posting to somebody standing
  at their own door is heard *and answered* in the same world-minute, through
  the ordinary rescore — otherwise "heard at once" would mean nothing the
  player could see, and the answer would wait up to `scorer_hours`. Somebody
  who is out hears it at their **next arrival anywhere** — the site they were
  walking to, or their own door on the way home — because that is where a
  messenger catches them, and it lands on the one scheduler's world-time
  address like everything else.
- **The scorer weighs the posting; the module owns the terms.** `Action` has a
  fourth variant (`Answer { posting, job }`) and `weigh` a fourth arm, which
  is the shape `autonomy.rs` was written for. The arm hands straight to
  `answers::terms`, because the arithmetic over a posting belongs with the
  module that owns the record — the record itself is shared sim state, read
  the way the quest board is.
- **The ask replaces the pot term rather than adding to it.** A posted job
  pays its taker the *wage*; the pot is the player's (§4.1's treasury margin).
  Weighing both would pay the same gold twice, so an ask's money term is the
  wage — felt by `pot_affinity` **and** desperation, which is what this
  section means by "a wage is a pot the player fills".
- **A refusal is what the scorer did instead**, said in the scorer's own
  words, and it is the first class in this game that stops the world.
- **`Motive::ordered` is gone.** Wave 1.1 had two ways an errand began — the
  player's order and the scorer's own idea — and this wave replaced the first
  with a posting somebody agrees to. `chosen` still decides which event closes
  a journey: `action-done` for their own idea, and nothing but the return for
  an ask, which `ask-agreed` already opened.
- **The standing rates are content, not a drawer row** (`asks::RATES`, four
  rows), exactly as an event class's default mode is: the panel is a live
  write *and* a recorded input, where a drawer row would be a restart, and two
  ways to move one number is the second way this repo's first convention
  refuses. They ride every stamp (`Rates::stamp`).
- **What the wave did not build**: `shirk` and `subvert` (parked with marks),
  a messenger anybody can see on the map, and a surface for site-shaped
  postings — the record carries all three shapes and the batteries exercise
  them, but the player's own hand makes job-shaped and task-shaped postings
  only.

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

*Implemented (w1.2): the first TRANSFER, and the assertion it was waiting
for.* A posting's wage moves treasury → wallet when the job completes
(`answers::settle_wage`), in full and never in part: the promise was the
posting's, and paying what was left rather than what was owed would be a
silent failure with a debtor's face. **The treasury may therefore go
negative**, which is a fact the top bar shows rather than a rule anybody
enforces — a settlement that has promised more than it has is a state this
game should be able to be in, and the needs wave is what makes it press.
`sweep::judge_orders` now asserts the conservation identity every run:
treasury + wallets − opening wallets = everything the pots minted. The pot is
still the player's and the wage is the worker's, which is the treasury-margin
this section describes, and the job board's row is where the two are seen
together.

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

*Implemented (w1.2): the wage-vs-expectation operation, the third of the
five.* Paying a posting's wage moves the worker's edge toward the player by
`wage_regard`, up when the wage beat the standing rate the posting recorded
and down when it fell short — the expectation is the **rate at posting**, so
moving the rates afterwards cannot retroactively make somebody feel cheated.
It goes through `adjust_regard` like every other write, so the bounds hold it.

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
| asks | mvp | **1** | autonomy, grid | regard, wealth | regard, wealth |
| aspirations | post | 3 | petitions | — | marks |
| threats | post | 3 | grid, events-director | — | — |
| arrival | post | 3 | grid, autonomy | — | — |
| parties | post | 4 | asks, resolution | bonds, regard | bonds |
| knowledge | post | exp | knowledge-lens | regard, wealth | — |
| whispers | post | exp | knowledge, autonomy | regard, marks | marks |

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
parameters; regard-unlock leading. **whispers** — seeded rumors about real
work, spreading along regard edges; belief needs the knowledge lens, so parked
until knowledge; a rumor is stale, never false, and a voice that keeps sending
people to nothing is learned about.

*Implemented (w1.1): autonomy.* `src/autonomy.rs` is the scorer, and it is one
function: `choose(sim, tuning, now, who, candidates) -> Judged` — the action,
the score, every term of it, and **the words**. The candidates are the
caller's, which is how wave 2's ask arrives (a fourth `Action`, not a fork).
This wave's three: **seek work** (claim a named job at a site and go),
**socialize** (walk to somebody's door, stay `visit_minutes`, small regard both
ways by a drawer row) and **idle** (the floor everything else has to beat).

- **The terms, each from data**: desperation opens the sum as it did in giri; a
  want's `pressure` applies where its `favors` field covers the candidate's
  task type; the aptitude is the row whose id *is* that task type; the pot
  pulls by `pot_affinity`; regard toward whoever is at the door is weighed by
  the visitor's own bond and grudge multipliers; and a rest term keeps somebody
  who just finished a job at home for `rest_hours`. **No term branches on a
  trait id** — the neutrality rule is what makes multiplying every row in safe.
- **One party per character, one dispatch loop.** The roster and `Sim::parties`
  are the same list twice, so a character the scorer sends out goes through
  `sim::dispatch` — the player's own order path — and `sim::begin_journey` is
  the only place `Activity::Outbound` is written. Verify asserts a
  self-dispatched journey and an ordered one come out the same five movement
  classes with the same verbs.
- **Cadence**: every `scorer_hours`, staggered by roster index by
  `scorer_stagger` minutes, on the one scheduler with a world-time address —
  so it is speed-invariant like everything else. **Somebody who is out is not
  rescored**; the occurrence fires, finds them abroad and reschedules.
- **The player's order stands and wins**: the scorer only ever looks at an idle
  party, and an order for somebody already out bounces with its reason, as
  before.
- **Two classes**, `action-started` (log) and `action-done` (ignore). The
  journey itself is told in the five classes the substrate already had — one
  story per movement — and these two carry the *decision*, with the reason on
  the note: `Ludo took the mushroom haul at the Deep Cave - needs the money`.
  An idle choice emits nothing: nothing happened, and the feed is for things
  that did.
- **Relationship presets** are `bonds_preset`, a drawer row like every other
  constant and so on every stamp: 0 is flat, 1 writes `CAST.md` §5's seeds
  through the store APIs at scenario open.
- **The quest board grew to six a site.** S1 authored seven jobs for a player
  who dispatches three parties by hand; ten people looking for work empty that
  before the first day is out, and a settlement with nothing to do is not one
  the scorer can be judged on. Sites still run dry — they take longer.

*Amended (the job-board session, 2026-09-06): the candidates go per open job.*
`Action::SeekWork` names a `sim::JobId` — a `(site, slot)` — and
[`candidates`] pushes one per **open job** rather than one per site, so each
row is weighed on its own task type. Wave 1.1 weighed only the front of a
site's list, which meant a board whose first open row was fight work offered
the crafter the patrol or nothing; the six-a-site board this wave authored is
what made that visible. **A deliberate behaviour change**: the same seed and
the same orders now produce different choices, so the sweep's fixtures moved
with it. The judge battery pins it with a staged case — a site whose first
open row is fight work and whose second is the signal repair, and Ines takes
the repair. Nothing branches on a trait id, and the aptitude term is still
`traits::competence_at`, which is also what the job board's fit column prints
(UI.md §3c).

*Implemented (w1.2): asks.* `src/asks.rs` is the record, the standing rates
and the hearing; `src/answers.rs` is the candidates, the terms, the row's read
and the money. The module's registry row is the second in `modules.rs`, and
the matrix is three passes. **The player's whole verb is a posting**: the S1
dispatch stub is gone, `Motive::ordered` with it, and `sim::dispatch` is now
reached only by somebody who decided to go — their own idea, or an ask they
agreed to. The demo character `CAST.md` §4.1 names for this module is Tim, the
proud refuser; what the build found is that the refusals the shipped set
produces are the *fits* rather than the pride — Alex will not leave scouting,
Ines and Rin will not leave crafting — and that pride's own refusal needs the
gift-and-charity field 1.5 gives it.

*Implemented (w0b): the registry as machinery, empty of rows.*
`src/modules.rs` holds the table above's shape (`ModuleSpec`: id, tier, wave,
degrades-to), the per-module disable flags (`ModuleSet`, a bitmask planted as
a resource before Startup like the constants), and the matrix §9 iterates.
**No row of the table above had been built** at that point, so the registry
was empty and the matrix was one pass — which the harness *runs* rather than skips. Adding a
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
- **Petition card** (the anatomy, recorded by wave 0a's mockup for **wave
  1.3 to build**; not built now): who is asking (portrait + name) + a trait
  chip + the request text + the reward + a timer bar against the deadline +
  the **declared consequence**, and an assign-picker carrying **willingness
  hints** per candidate. The card is the petition's whole surface; the feed
  entry for a voiced petition is what opens it.

- **Posting record** (wave 1.2): id, who (character id | any), what (job id |
  site id | task type), until (done | withdrawn), wage,
  standing-rate-at-posting, made-at (world-minute), heard-by (character →
  world-minute), answered-by (character → agreed | declined + reason), status
  (open | filled | withdrawn). Sim state; replay-carried; every posting is
  recorded input (made and withdrawn are snapshot inputs).

*Implemented (w1.2): the posting record, whole.* `asks::Posting` carries every
field above and the ledger drawer is a view of the vector they live in — there
is no second list, so a posting the player can see and a posting the scorer
can weigh cannot be two different things. The two new recorded inputs are a
tap on a job row and a tap on WITHDRAW; the standing rates are sim state
beside them and ride the scenario's opening stamp.

*Implemented (w0b): the trait row only.* Its modifier set is split by kind as
this section says — bond and grudge multipliers plus the pot's pull for a
personality, an upkeep multiplier and a scorer pressure for a motivator, a
competence value for an aptitude — with the reaction rows in their own table
keyed by trait id. The description is validated as one stranger-facing ASCII
line. The other three formats arrive with the modules that read them; the
roster and starting balances are authored in code until the scenario file
lands (§8, wave 1.5).

## 7. The MVP

**Modules**: foundation + wave 1 (autonomy, asks, needs,
settlement-with-one-industry, resolution, petitions, minimal injector).
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
- **1.1 autonomy** → **1.2 asks** → **1.3 needs + settlement** (one session —
  the economy loop halves are one concern; the wage lever goes from
  under-driven to live, and standing rates and the per-industry wage are one
  policy family) → **1.4 resolution** (fit matters) → **1.5 petitions** →
  **1.6 injector** (wave close: sanitation whose first pass is the UI exemplar
  audit; the vocabulary question) → MVP gate. Each lands into a running world;
  owner sanity-plays between sessions but fun is not judged. **Reordered
  2026-09-03** on the wave-1.1 playtest: the control verb is fixed before the
  economy is built on it — the MVP loop in the order it is played (ask, feed,
  judge, listen, pressure).
  - **1.5** builds the petition card to §6's anatomy, and registers the
    petition/consequence classes on `pause-and-focus`. It is no longer the
    wave that makes the world interrupt you at all: 1.2's `ask-declined` is
    the first class that stops the world, because being told no, with a
    reason, is the whole of what the asks wave added to the loop.
  - **1.6's starting calibration, from the wave-0a mockup**: the mockup
    played best at roughly **twice** the first-guess event density. Land the
    scenario and injector constants there rather than at the first guess and
    tune down; a world that interrupts you twice as often as the drawing
    board expected was the one that felt alive. It is a starting point for
    the drawer, not a finding about the design.
- **MVP gate playtest** after 1.6.
- **3+** aspirations / threats / arrival (any order) → **4 parties**
  → knowledge when the experiment is wanted. Re-derive waves at each
  GDD refresh; the registry is the source.

## 9. Verify strategy

- **Module-off matrix**: verify runs the full suite with each module
  individually disabled — green is the definition of modular.
- **Speed-invariance sweep** (landed) over every new event source, **and
  over the auto-pauses**: the same three speed scripts under a config that
  stops the world at every completion, resumed by the key the script is
  already running at, must produce the identical transcript. A pause
  stretches wall time and moves no world-time address.
- **Economy sweeps**: ~200 idle-player seeds; wallet/treasury bands;
  nobody-starves-at-subsistence (the limp-floor as an assertion); a
  mutated wage or upkeep constant must break a band.
- **Distribution sweeps** over the compliance ladder when asks land
  (P2 machinery, right horizon this time).
- Floors + screenshot process on every surface; stamps carry seed,
  constants, variant, module set.
- One-source checks: any warning surface asserted equal to its
  consequence's inputs (band-chip discipline, generalized).

*Implemented (w0a):* the auto-pause battery is `src/pauses.rs` — the
transition (the clock at speed 0 in the tick the event fired, at the event's
own world-minute), the replay (the same recorded inputs, twice, to the tick),
the config as the whole difference (the same script without the three clicks
never stops and runs the authored timeline out), and the invariance sweep
above. The feed's one-source claim is `attention::feed_is_a_view`: the feed
equals the transcript filtered by the config, at both settings of the
ignored toggle, with a check that the filter is hiding something so the
assertion cannot pass vacuously. The floors bind the feed's rows, the meter
chips, the config's radios and the character panel; the screenshot set is
eight, and the photographed session is itself the auto-pause half of the
invariance claim — it plays the config change, gets stopped four times and
must still match the sweep's transcript exactly. The mutation round grew to
nineteen constants and notices all of them; the two new ones are seen by
`attention::judge_at`, written with shipped literals.

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

*Implemented (w1.1):* the module-off matrix is two passes now, and the
autonomy-off one is what makes the row's degrades-to sentence a fact: with the
scorer off nothing is scheduled at all, nobody emits `action-started`, and the
world comes to rest with everybody at their own door — the wave-0b world. The
speed-invariance sweep is extended over the scorer, and **its transcripts now
carry the sentences**, because a replay that reproduced the choices and not the
reasons would be reproducing half a decision.

Two shapes of the harness changed, and both are the retuned clock's doing
rather than the module's. **The sweep is judged over a world-time window**
(`sweep::WINDOW`, the scenario's first day) instead of running to rest: a world
with people deciding things in it never comes to rest, and two speed scripts
are only comparable over the same span of *world*-time. And an order is
addressed by `When::Approaching` — the conductor simulates the clock forward
the three ticks a two-click order takes and starts it early enough to *land* on
the world-minute it names — because at 4x the retuned clock carries 1.6 minutes
a tick and a click begun when the minute arrives takes effect several minutes
later, by a different several at every speed.

The scorer's own battery is `autonomy::judge_module`: the choices replay
identically (transcript and sentences); the **alive sweep** runs an idle player
over `alive_days` world-days and asserts everybody takes paid work, nobody is
dispatched while already out, and Ludo — `CAST.md` §4.1's eager worker — takes
work at the first minute he is asked to think; the **preset flip** shows at
least one choice differing between flat and authored, and it is the one the
seeds are for; and the **one-dispatch-path** claim compares an ordered journey
against a self-sent one. The mutation round grew to thirty-four constants and
notices all of them, through a seventh instrument — the scorer battery, every
expectation a shipped literal.

Economy sweeps wait for an economy.

*Implemented (w1.2): the asks battery, and the distribution sweep the ladder
was waiting for.* `src/compliance.rs` is the module's own, and it carries the
five claims GDD §9 owes it:

- **The compliance transcript**: the four scripted postings are agreed to at
  the same world-minutes with the same sentences on a replay; a posting
  withdrawn before it was heard is never heard and never answered.
- **The distribution sweep over the ladder.** The plan asks for ~200 seeds;
  this build reads no `Rng` at all, so the population that actually varies is
  the *offer*, and the sweep walks a seven-rung wage ladder against one job of
  each task type for every character — 280 answers a pass. The four bands
  (loyal, cold, greedy, desperate) take a pinned number of those offers each,
  as shipped literals, and the mutation round breaks them by moving
  `ask_targeted`, `wage_regard` or any weight the sums are made of. The shape
  is the design's: the loyal and the desperate comply most, the cold least.
- **One function**: over all 280 staged offers, the job row's verdict and what
  the character actually does about the same posting agree — the row is
  produced by the same `autonomy::choose` over the same candidates a rescore
  would build, so the two can only differ by the world having moved.
- **The standing-rate policy test**, and how coarse the lever is: walking
  fight pay across the panel's range in the panel's own step changes somebody's
  *work* at 12g, 16g, 36g and 56g — five of the ten drift into fight work as
  it rises — and from the shipped 24g it takes **three** steps to move
  anybody. That distance is a pinned literal and a question for the playtest,
  not a bug: a wage has to beat a six-point aptitude term to pull somebody off
  their own trade.
- **Speed-invariance over postings and messengers**: the transcripts of all
  three speed scripts carry the posting, hearing and agreement events at
  identical world-minutes, and a messenger's hearing is asserted at the
  arrival minute it rides.
- **Module-off with asks off** is the third matrix pass: nothing can be
  posted, the ledger will not open, the site panel is a read of the work, and
  the world is wave 1.1's.

The mutation round grew to thirty-six constants and notices all of them,
through an eighth instrument — the asks battery, every expectation a shipped
literal.

## 10. Confidence & open ledger

Foundation: grid/clock/pathfinding **played**; people substrate
**played** (in giri; ported and green here); attention **built and
owner-playable** — the mockup raised it to mocked and wave 0a landed it, and
the played verdict is the owner's next playtest (does the world interrupt
you at the right moments, and only those?). All modules **speculative** —
correct and expected; wave gates convert speculation to played evidence one
wave at a time.

Open (deliberately): **the expectation model beyond the standing rate**
(fit-adjusted? regard-adjusted?) · **open postings travelling** (this wave:
heard at camp only) · **standing-posting fatigue** · **the ask-spam question**
(does asking spend regard?) · **whispers' rumor vocabulary and spread model**
(capsule) · **a surface for site-shaped postings**, and a messenger anybody
can see · the wave-1 class registrations and whether the mockup's
defaults survive a real petition load (wave 0a shipped them; nothing this
build has opens on pause) · **the trait vocabulary is provisional through the
wave-1 close** — the words are `CAST.md` §3, chosen before the context that
tests them exists, and §7 carries the question the wave-1.5 playtest asks of
them; a rename is a data edit until 1.3 writes petition copy against them ·
aptitude-change mechanism (two candidates recorded) · bond/grudge
erasure rules · quest authoring surface beyond template `next` links ·
settlement stock list beyond gold-only (bound to a famine/siege design
need) · map generation (post-GDD session; requirements now stateable:
one town, sites, terrain variety, readable at 8–12 characters' scale).
