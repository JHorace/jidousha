# ninjo (人情) — Game Design Document

Repo-canonical. Drafted at GDD assembly, 2026-08-30, from the brainstorm-1
capsule registry. The vault capsules remain the *working surface*; when a
capsule and this document disagree about something decided, this document
wins and the capsule is stale. Numeric values are drawer-tunable throughout;
shapes are the design.

**`CAST.md` beside this file is the cast bible** — who lives in Kawaza, what
the trait words mean, the seeded relationships and the petition templates.
It is content where this document is design; where the two disagree about a
decided thing, this one wins, and `CAST.md`'s vocabulary is **locked**
(owner, 2026-10-02 — petition copy is written against it, so a change is a
rename; its §7 carries the question the playtest asks of it).

**`DESIGN.md` beside this file is the substrate's technical doc** — the tile
world, the integer clock, the one scheduler, the pathfinder, the verify
machinery. The two are deliberately not merged: this document is what ninjo
*is*, that one is what the ground under it *does*. Where they disagree about a
decided thing, this document wins; where this document leaves the substrate's
mechanics unstated, that one is the record. `UI.md` owns presentation.

**What is built, and the mark discipline.** Wave 1 is closed: §3–§6 and §9
describe the built game in the present tense, and the trail of which wave
landed what is in git. §5's post-MVP rows and §10's open items are design and
not yet code. A section a *live* wave builds carries an `*Implemented (wN):*`
paragraph, written by the session that built it in the same commit, naming
what landed and where and saying where the build bent the shape above it;
the wave-close sanitation pass folds those paragraphs into their sections
once the wave is closed (`docs/conventions.md` §Documents).

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

## 3. Foundation (the spine)

**The substrate** (`DESIGN.md` is its whole record): the tile grid the sim
reads (one grid, two readers; locations are named tiles; deterministic
4-connected pathfinding, documented tie-break) · integer world-clock with
speed-as-input (pause/1×/2×/4× through the snapshot; the speed-invariance
sweep is the signature test) · one scheduler; every occurrence has a
world-time address · events carry time + place + class.

**People:**

- **The registry** is `src/people.rs`: id, name, home tile, portrait role,
  traits, wallet, desperation and its `source` line (the proven
  differentiator), the active-petition slot, the arrival minute
  (`present_from`), and the visited set (`Character::memory`). The portrait
  role is the library's own `Art` role, because in this codebase a role *is*
  the art contract (`src/sprites.rs`). **The cast is ten** (`CAST.md` §4),
  at their own homes south of the ford, and the town is **Kawaza**.
- **A party is a character.** The roster and `Sim::parties` are the same list
  twice, in registry order; who *else* is in a party, and what their bonds do
  to its outcome, is the parties module (§5, wave 4).
- **The trait vocabulary** is `src/traits.rs`, data-defined, kinds
  `personality` / `motivator` / `aptitude`: **four aptitudes whose ids are the
  task ids** (`fight`, `labor`, `scout`, `craft` — a job of a type reads the
  row of that name and nothing maps between two vocabularies), **five
  motivators**, each with a `favors` field (a task type, `any` for any paid
  work, neutral `none`) that is the whole of a want's reach into the scorer,
  and giri's nine personalities — six worn, three *parked* (`people::PARKED`:
  `pious`, `pragmatic`, `upright`, rows the trait × mark table references and
  nobody carries until marks are common). Traits parameterize decision
  functions and data (upkeep cost, pressure, competence) and never branch
  code. No list cap (attention architecture owns legibility).
- **How the `kind` field avoids becoming a branch**: every `TraitDef` carries
  every modifier field, and the vocabulary's validation asserts that a row
  holds the neutral value in every field its kind does not own. Consumers
  apply every field of every trait unconditionally — a personality's upkeep
  multiplier is 1/1 and drops out — and nobody filters a trait list by kind
  before using it. A kind gates which data a row may carry; it never gates
  which code runs.
- The registry asserts `CAST.md` §7's coverage matrix and the
  no-dead-motivator rule (every motivator has a template whose trigger names
  it, walked over `petitions::templated_motivators()`).
- **Shared-state stores**: regard edges, bonds/grudges (pair-facts), marks
  (person-facts) — §4.
- **The knowledge lens** is `src/lens.rs`, a single read-path every UI
  surface uses; v1 = identity. The seam is structural: `screens::content`
  takes a `Lens` and no `Sim`, so a screen cannot read around it because a
  screen has nothing else to read. `--verify` and the sweeps read truth
  directly, by design — they are the simulation's instruments, and a check
  that could only see what the player sees could not catch the sim lying to
  the player.

**Attention:**

- **The event-class table is `src/attention.rs`**, one row per class
  carrying id, colour role, icon role and the mode it opens on; **nothing in
  the game branches on a class id** — a screen asks the row how to draw it
  and the scheduler asks the config what it does. A new class is a variant
  and a row. Twenty-three classes today.
- **Auto-pause per event class**: ignore / log / pause-and-focus,
  player-configurable. The shipped defaults: movement (`departed`, `arrived`,
  `work-began`, `returned`) and the posting bookkeeping are `ignore` because
  the map already shows motion; decisions, completions, shortfalls, arrivals
  and buildings are `log`; four classes open on **pause-and-focus** —
  `ask-declined` (being told no, by somebody you asked by name, with the
  reason on the banner), `task-failed` (a contract you wrote going wrong),
  `petition-voiced` and `petition-failed`. The default modes are table data
  and not drawer rows — the config panel is a live override *and* a recorded
  input, where a drawer row would be a restart, and two ways to move one
  number is the second way this repo's first convention refuses. The two
  attention constants that *are* drawer rows are `feed_cap` and
  `pulse_tenths`.
- **The feed is a view and has no state**: `attention::feed` derives its
  entries from `Sim::events` every time it is asked, so nothing can be stale
  or disagree. The *notices* trail — speed changes, refused taps, restarts —
  is the things that did not happen in the world and have no world-time or
  place; it is a separate band of the same drawer, and the one-source
  assertion is over the feed alone.
- **Auto-pause is a transition inside the scheduler.** `Sim::emit` records
  the pause when a firing event's configured mode says so, and `sim::fire_due`
  puts the clock at speed 0 in the same tick — no synthetic input, so a replay
  reproduces the pause rather than a click nobody made. The reason and the
  pause count are sim state; the player's next speed input clears the reason.
  The first pause-class event of a crossed span is the one that stops the
  world, and the rest of the span still fires: a pause holds the future,
  never the present.
- **The config is sim state**, written only by clicking a radio in the config
  drawer, which is a recorded input like a speed change.
- **Meters and faces**: aggregates for the glance (derived from the same
  per-character truths as the sim), drill-down to characters for action.
  Petition cards show timer + declared consequence (§6). `UI.md` §3a owns the
  shapes; every module's capsule declares its `attention` cost, and the feed
  is the budget's ledger.
- **Selection is presentation** — a click on a figure or a face row opens the
  panel and rings the map sprite; nothing about it is sim state. Tap works
  and this game wrote no touch code: the engine mirrors the first finger
  onto the primary pointer.
- **The scorer's own terms are an inspectable surface.** A decision carries
  the `Judged` it was made from, recorded on the occurrence beside the reason
  (`autonomy::Reckoning`, `Sim::remember`), and one tap on the feed entry
  shows every term with the trait row or the fact that produced it. It is the
  never-lies invariant (§1) applied to a decision: a *reason* that stands for
  an arithmetic nobody can see is the same failure one level up. The
  attention cost is a `?` at the end of a row and nothing else. The same band
  explains a posting the player has not made yet from the job row's own
  verdict (`UI.md` §3e), out of the one `answers::read` that produced that
  verdict — one derivation, so a preview and a record cannot describe one
  scorer two ways.
- **Text is the built-in bitmap face.** The engine's TTF support is not
  adopted: the owner's verdict against proportional display faces for dense
  information stands, and the feed is the densest surface in the game.

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
crypt without anybody being named. (Settlement's per-industry wage is the
same lever for camp work, and the two are one policy family: an industry
opens at the standing rate for its own kind of work.) A named posting is the
exception, not the routine. **The rates are content, not drawer rows**
(`asks::RATES`), exactly as an event class's default mode is: the panel is a
live write *and* a recorded input, where a drawer row would be a restart.
They ride every stamp.

**Hearing.** A posting binds nobody until it is heard (the petitions rule,
mirrored). Open postings are heard at camp: a character at home weighs the
board on their next rescore. A targeted posting to somebody standing at their
own door is heard *and answered* in the same world-minute, through the
ordinary rescore — otherwise "heard at once" would mean nothing the player
could see. Somebody who is out hears it at their **next arrival anywhere** —
the site they were walking to, or their own door on the way home — because
that is where a messenger catches them, and it lands on the one scheduler's
world-time address like everything else (asks travel, decided 2026-08-29).
Withdrawal is instant on the ledger, and heard the same way.

**Deciding.** The scorer weighs a heard posting as a candidate beside
everything else it weighs — the same one function, the ask as a
heavily-weighted candidate (`Action::Answer { posting, job }`, a fourth arm of
`weigh` that hands to `answers::terms`, because the arithmetic over a posting
belongs with the module that owns the record). Its terms, from data: the
wage's pull (**the ask replaces the pot term rather than adding to it** — a
posted job pays its taker the wage and the pot is the player's, so an ask's
money term is the wage, felt by `pot_affinity` **and** desperation, which is
what "a wage is a pot the player fills" means); a **regard-for-player term**
(the loyal comply for less; the cold need paying); fit (the task's aptitude
row); a **targeted bonus** (being asked by name weighs more than a notice on a
board — a drawer constant; this is the "obligation" of an ask without making
it an order). Three outcomes:

- **agree** — the character takes the job through the ordinary dispatch loop;
  the event says why in words ("Ludo took the mushroom haul — asked, and needs
  the money");
- **decline** — the scorer chose something else, said in the scorer's own
  words; a targeted decline emits `ask-declined` with the reason ("Tim won't —
  the pay was short and he's owed already"), and it is the first class in this
  game that stops the world; an open posting simply goes unfilled;
- **drop** — a character on a posting's job abandons it when something
  presses harder (`ask-dropped` is the class and the seam; needs and petitions
  supply the pressure).

**Regard.** Asking is free and declining is free (the ask-spam question stays
open; nothing here spends regard for the asking). Payment moves regard through
the wage-vs-expectation rule in §4.2: expectation is the standing rate for the
task type **at posting**; paying above it small +, below it small −, whether
or not they took it for less. Shirk and subvert — the betrayal-ladder rungs —
are parked fills until marks are common.

**Surfaces.** The site panel's job row is where a posting is made: with a
character selected, the one-tap gesture on an open row **posts to them at the
wage the footer shows** (the standing rate, until stepped), and the row reads
what the scorer makes of it. The **postings ledger** is a drawer and a view of
`Sim::postings`: every posting, its who/what/wage/until, who heard it, who
answered and why; withdraw from the row. The **standing rates** panel is four
rows. The player's own hand makes job-shaped and task-shaped postings; the
record carries site-shaped ones too, exercised by the batteries and without a
surface (§10). No messenger is drawn on the map (§10).

**The record and the rules are `src/asks.rs`; the answering and the money are
`src/answers.rs`** — two files, one module, because the record and what
happens once somebody has heard it are two subjects. `Postings` is a private
vector on the `Sim` with named writes, the way `stores.rs` holds regard: a
posting exists by `post`, ends by `withdraw`, and is answered by nothing else.
`sim::dispatch` is reached only by somebody who decided to go — their own
idea, or an ask they agreed to.

**Degrades to** (asks off): pure observation; no ledger, no postings; the site
panel is read-only. Verify runs the matrix that way.

## 4. Shared state (deep specs)

### 4.1 Wealth (gold; the only v1 currency)

Holders: player treasury, character wallets. **Mint at sources, burn
at sinks, conserved between holders**; the ports, exhaustively:

- MINT: a site pot (→ treasury, on a success — the remainder after the
  share); a self-chosen job's **share** (→ the worker's wallet, on a success:
  `share_pct` of the pot); an industry shift's wage (→ the worker's wallet,
  with the levy minted into the treasury beside it). A failed job mints
  nothing.
- TRANSFER: a posting's wage (treasury → wallet, on the job's completion);
  a petition reward (petitioner wallet → satisfier, never an overdraft — and
  **no shipped template pays one**, T1 to T6 all pay in regard, so the port's
  first exerciser is a staged row in the battery); a petition gift (treasury →
  the petitioner's purse, the card's GIVE, debiting exactly the `{n}` the card
  shows — a gift does not satisfy anything, the predicate does);
  `gives-away` (half a purse → `{other}`).
- BURN: upkeep (wallets; trait-modulated by `traits::upkeep_of`, the
  motivators' multipliers); industry construction (treasury,
  `settlement::build` — the first real sink the player has); declared
  consequences where stated (`broke`'s purse, gone).
- The industry levy knob is the `industry_levy` drawer row, default 0
  (passive income is upgraded into).

**One payout function.** `resolution::payout` is the one answer to "what does
this job pay, by port, if it resolves at this tier", and `resolution::settle`
moves exactly that onto the ledger. **A job pays one way**: the **wage** if it
was posted — in full and never in part, *even on a failure*: the promise was
the posting's, paying what was left rather than what was owed would be a
silent failure with a debtor's face, and the player ate the risk of sending a
bad fit — the **share** if it was their own idea (on a success), and a shift
pays the industry's wage, never rolled. A site's pot and an industry's shift
are the same `work_done`, and which port the gold moves through is read off
`Site::industry` rather than branched on a name. The scorer weighs what the
payout function says (`autonomy::pay_for`): a posting by its wage, a
self-chosen job by its share, a shift by its wage — never the pot — through
the one money term (`autonomy::money`), felt by pot affinity **and**
desperation.

**The share is three percent**, and that number is a finding rather than a
taste (`FINDINGS.md` G-048): Steve's first-interval claim, which is `CAST.md`
content, holds in every world only up to about a gold a job. The standing
rate competes with the share — a stingy posting can lose to a fat pot
somebody would rather take on their own terms — and at three percent that
competition is real in the arithmetic and small in practice. The owner prices
it.

**Treasury-margin**: pots land in the treasury; promised shares and wages pay
out; the remainder is the player's income — the player as contractor. **The
treasury may go negative**, which is a fact the top bar shows rather than a
rule anybody enforces: a settlement that has promised more than it has is a
state this game should be able to be in, and needs are what make it press.
**What is in the purse is what is taken**: upkeep never overdraws anybody —
the shortfall is what could not be found, and what it does is press
desperation and rewrite the source line, not go on the books as a debt. That
is the limp-floor's arithmetic half.

**`Sim::ports` is the ledger** — every port totalled, as a record nothing in
the simulation reads. The conservation identity is checkable rather than
argued: the treasury plus every purse, less what the purses opened holding,
equals pots + shares + wages − upkeep − building, with gifts, rewards, declared
burns and `gives-away` moving through named ports too; the economy sweeps and
the petition-horizon sweeps assert it on every world they run, and
`petitioned::conservation` stages each petition port and asserts it after.
A movement belonging to no named port shows up as the difference. **It is
the only honest source for a flow figure on a surface**: the state-of-the-camp
line carries no income and no net, because this build records no *window* to
derive one over, and `needs::judge_at` asserts the line says neither word.

### 4.2 Regard (the master currency)

Directed integer edges, char→player and char→char, default 0, range
bounded by facts (below). **Operations** (magnitudes are drawer
constants; classes are the spec):

- **Petition satisfied**: the petitioner's edge toward the player up by
  `plea_regard` **when the player's hand is in it** — a gift, a posting its
  subject answered since it was voiced, or a building put up since that the
  condition is about or that the meeting work was done at
  (`pleas::players_hand`). A petition met by the world alone is met all the
  same, with full credit (owner, 2026-10-02: incidental credit stands), and
  moves nobody's regard, because no character acts on another's petition yet
  and nobody is the satisfier.
- **Voiced petition failed**: the petitioner's edge toward **the player and
  nobody else** down by the same step — every consequence sours, `sours`
  included. **Obligation is voicing**: nothing here reads a need; a petition
  never raised, or raised and still on its messenger, moves no regard.
- **Wage offer vs expectation**: paying a posting's wage moves the worker's
  edge toward the player by `wage_regard`, up when the wage beat the standing
  rate **the posting recorded** and down when it fell short — the expectation
  is the rate at posting, so moving the rates afterwards cannot retroactively
  make somebody feel cheated. An industry shift's wage is judged against the
  standing rate for its own kind of work through the same
  `answers::wage_regard`, which is what makes the per-industry wage and the
  standing rates one policy family rather than two rules that happen to agree.
- **Witnessed acts**: via trait×mark reactions (giri v2 table carried);
  dormant until marks are common.
- **Slow drift toward a fact-set baseline**: a bond raises an edge's floor, a
  grudge lowers its ceiling — the scalar is the mood, the facts bound its
  range.

Regard is also the information network (transitive knowledge), dormant
until the knowledge module.

**The store** is `src/stores.rs`: directed integers, sparse (absent is zero),
a target either another character or the player. There is one general write,
`adjust_regard`, which holds the result inside the pair's bounds — so no
caller can push an edge past what the facts allow, and the bound is decided in
one place. **The bounds**: with no facts an edge runs `±regard_span`; a bond
raises the floor to `bond_floor`; a grudge lowers the ceiling to
`-grudge_ceiling`. **The baseline** is zero, held inside the bounds: a mood
with nothing behind it decays to indifference; a bond stops that decay at its
floor and a grudge at its ceiling. A pair holding **both** facts — a friend
who wronged you, a real state — has crossed bounds, and its baseline is the
midpoint of the interval between them; that is the arithmetic answer, and it
is why the crossed case needs no special rule anywhere else. **The drift** is
integer and bounded: one `drift_step` toward the baseline every `drift_hours`,
never past it, never outside the interval, with a fixed point at the baseline
so a world nobody touches settles instead of oscillating; it runs through the
one scheduler with a world-time address, so it is speed-invariant like every
other occurrence, and it emits no event — nothing *happened* to anybody, and
the feed is for things that did. Bounds are asserted at their bounds: over
every fact-set and every value from well outside the widest span, one drift
never leaves the interval and never moves away from the baseline.

### 4.3 Bonds & grudges (pair-facts) and Marks (person-facts)

Written by events, never by drift: bonds from repeated shared success
plus high mutual regard (threshold event); grudges from betrayal-class
acts, acting against a character's petition, or egregious/repeated
petition failure. Facts do not decay; goal-completion-style erasure
rules are post-MVP design. Marks carry giri v2 semantics unchanged
(public facts; trait×mark reaction table; title-marks arrive with
aspirations). Party outcome effects of bonds/grudges are the parties
module (wave 4); the *stores and write rules* are foundation.

- **The three vectors are private to `src/stores.rs`.** There is no `&mut`
  accessor and no eraser — *facts do not decay* is the absence of a function,
  not a rule somebody has to remember — so the only ways in are the write
  functions below.
- **Bonds**: `record_shared_success` counts a pair's successes and writes the
  bond the first time they reach `bond_after` **and** both edges stand at or
  above `bond_regard`. Both halves are load-bearing and both are asserted:
  repeated success at cold regard writes nothing. A bond is written **both
  ways**, because a one-sided bond is a category error. Nothing registered
  reads `Sim::resolved` (the record of every resolution) to write one yet;
  `resolution::went_well_means` says so, derived from the registry.
- **Grudges**: `record_grudge` is directed and one-sided — the wronged hold
  it, and what the wrongdoer feels is their own business. It takes a cause
  (betrayal / acted against their petition / petition failed) as **data on
  the row, never a branch**: nothing decides differently for one cause than
  another, which is what keeps a fourth source a data change. Writing the
  fact re-holds the edge inside its new ceiling at once: a betrayal *is* a
  grudge when it happens, and letting drift walk warmth down over the next
  several hours would be the sim lying about what just occurred. **A failed
  petition writes** `grudge(petitioner → player, PetitionFailed)` when it is a
  **repeat** (they have been failed before) or **egregious** (it was already
  the second ask of an arc — a `next` link's petition, the collector's second
  visit).
- **Marks** are person-facts, idempotent (wearing one twice would double every
  reaction to it), with giri v2's tones and the trait×mark reaction table
  carried unchanged — including the `(pragmatic, skimmer, +2)` cell that makes
  a known skimmer preferable to a stranger, so reactions open doors as well as
  close them. No shipped act writes a mark yet (§10).

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
| events-director | mvp | 1 (minimal) | clock, petitions | — | — |
| asks | mvp | 1 | autonomy, grid | regard, wealth | regard, wealth |
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

**The wave-1 modules, as built.** The seven rows above the line are in; each
is a row of `modules.rs` and each is green with itself off (§9).

**autonomy** — `src/autonomy.rs` is the scorer, and it is one function:
`choose(sim, tuning, now, who, candidates) -> Judged` — the action, the
score, every term of it, and **the words**. The candidates are the caller's,
which is how the ask arrives (a fourth `Action`, not a fork). Three of its
own: **seek work** (claim a named job and go), **socialize** (walk to
somebody's door, stay `visit_minutes`, small regard both ways by a drawer
row) and **idle** (the floor everything else has to beat).

- **The candidates go per open job**: `Action::SeekWork` names a
  `sim::JobId` — a `(site, slot)` — and `candidates` pushes one per open job
  rather than one per site, so each row is weighed on its own task type and a
  crafter can take the craft job standing behind a fight job. The judge
  battery pins it with a staged board.
- **The terms, each from data**: desperation opens the sum as it did in giri;
  a want's `pressure` applies where its `favors` field covers the candidate's
  task type; the aptitude is the row whose id *is* that task type
  (`traits::competence_at`, which is also what the job board's fit column
  prints); the pay pulls through the one money term (§4.1); regard toward
  whoever is at the door is weighed by the visitor's own bond and grudge
  multipliers; and a rest term keeps somebody who just finished a job at home
  for `rest_hours`. **No term branches on a trait id** — the neutrality rule
  is what makes multiplying every row in safe.
- **One dispatch loop.** A character the scorer sends out goes through
  `sim::dispatch`, and `sim::begin_journey` is the only place
  `Activity::Outbound` is written. The player cannot dispatch anybody (§3b).
- **Cadence**: every `scorer_hours`, staggered by roster index by
  `scorer_stagger` minutes, on the one scheduler with a world-time address.
  **Somebody who is out is not rescored**; the occurrence fires, finds them
  abroad and reschedules.
- **Two classes**, `action-started` (log) and `action-done` (ignore). The
  journey itself is told in the five movement classes the substrate had — one
  story per movement — and these two carry the *decision*, with the reason on
  the note: `Ludo took the mushroom haul at the Deep Cave - needs the money`.
  An idle choice emits nothing.
- **Relationship presets** are `bonds_preset`, a drawer row on every stamp:
  0 is flat, 1 writes `CAST.md` §5's seeds through the store APIs at scenario
  open.
- **The board is six jobs a site** (twenty-four), because ten people looking
  for work empty a seven-job board before the first day is out. Sites still
  run dry — they take longer.

**asks** — §3b. `src/asks.rs` is the record, the standing rates and the
hearing; `src/answers.rs` is the candidates, the terms, the row's read and
the money. The demo character `CAST.md` §4.1 names is Tim, the proud refuser;
what the shipped set produces is the *fits* rather than the pride (`CAST.md`
§4.1).

**needs and settlement** — `src/needs.rs` is the list, the burn and the
shortfall; `src/settlement.rs` is the industry table, the state and the
levers; `src/camp.rs` is the panel the camp's own marker opens.

- **A need is a row and a constant.** `needs::NEEDS` is §6's format with one
  entry, and both of its numbers are **fields on the row** (`Field::UpkeepCoin`,
  `Field::UpkeepHours`) — so the list stays data, the drawer stays the one
  place a number lives, and a second need is a row here plus a constant there.
  The cost for one person is `traits::upkeep_of` and nothing else.
- **A shortfall presses and rewrites.** Desperation moves one step, held
  inside `people::DESPERATION_MAX` by the one writer there is (`people::press`),
  and the `source` line is **composed onto** the line they were generated
  with rather than replacing it — `Character::origin` is kept beside it for
  exactly that. Two people at desperation five are two different problems.
- **Desperation is the scorer's opening term**, so a sliding person takes
  worse work with nothing added to `autonomy.rs`; `needs::judge_module`
  asserts the difference is exactly `need_weight` a step.
- **An industry is a site.** Its standing slots are one more entry of
  `Sim::sites`, standing at the camp itself (`sim::SITE_LOCATIONS` is a
  table), so the scorer, the dispatch, the journey and the completion are the
  ones that already existed and no term was added to the scorer: a shift is a
  job, and the wage is the job's pot. What makes a slot *standing* is that
  finishing it opens it again. **A shift is never rolled** (`resolution::rolls_at`
  reads `Site::industry`), and the settlement panel says so in a derived line
  (`camp::shift_odds`).
- **The wage lever moves people**: a shift's wage is felt by need as well as
  by greed (§4.1's one money term), and over the camp at its authored
  desperation the panel's ladder changes who takes a shift
  (`economy::judge_the_wage`; `FINDINGS.md` G-035, closed).
- **A shift is not postable.** It pays the industry's own wage, set on the
  settlement panel, and a posting over it would be a second wage for one
  shift and a second surface for one decision. No gesture produces one — the
  camp's marker opens the panel rather than a board — and `asks::post`
  refuses one anyway, because a door nobody can reach is still a door.
- **The staged start is content made mechanical.** Every roster row carries a
  `present_from` world-minute (the scenario file's); the camp opens with the
  four founders and the six who came later arrive as occurrences on the one
  scheduler (`CAST.md` §4). **The lens filters by presence** — `Lens::roll` is
  the one derivation — so the map, the roster, the meters, the candidate
  picker and the camp line all follow it, and a person who has not arrived
  has no party, no token, no chip and no row.

**resolution** — `src/resolution.rs` is the curve, the roll and the payout;
`src/outcomes.rs` is the battery.

- **Three tiers** — *went well*, *done*, *failed* — and **the outcome is a
  seeded roll with the odds shown**. `resolution::odds(tuning, fit)` is the
  fit-to-odds curve, a function of fit alone (`fail_base`, `fail_fit`,
  `well_fit`: at the shipped set a strong fit fails 11% and goes well 24%, a
  poor fit fails 35% and never goes well), and `resolution::resolve` is the
  roll that reads it. **No difficulty stat**: the seam is named in §10.
- **The roll is addressed by the occurrence** — the scenario seed (`Sim::seed`,
  planted by `flow::load_scenario`), the world-minute the work completes and
  the job's `(site, slot)` — mixed into one number and drawn once
  (`resolution::roll`). Never by call order and never by frame, so the
  speed-invariance sweep holds over it with nothing added (`FINDINGS.md`
  G-016's rule; G-045 for why the game mixes before seeding).
- **Every surface that shows fit shows the odds in a word** — `safe`,
  `chancy`, `risky`, at the `odds_safe`/`odds_risky` thresholds — `fit 2 safe`
  on the board row, the candidate picker and the work list, through one
  formatter (`resolution::fit_cell`) over `Lens::odds`, which is
  `resolution::odds_for`, the roll's own function. The fit chip's explanation
  is derived from the same curve and the registry (`resolution::fit_means`,
  `FINDINGS.md` G-046).
- **"Went well" pays as done, honestly.** Its effect is the record —
  `Sim::resolved`, every resolution with who, which job, the tier and whether
  it was posted — which §4.3's bonds will read; until something that reads it
  is registered, `resolution::went_well_means` says so, derived from the
  registry.
- **Failure is economic only**: no pot, no share, the posted wage paid anyway,
  no desperation write and nobody hurt. **The job goes back on its board** —
  open, pot intact, same identity, no retry limit — and the worker walks home
  as any returner does. The line says it is back: *Steve botched the second
  seal - paid 24g anyway, no pot; the job is back on the board*.
- **Two classes**: `task-failed` (a posted job; **pause-and-focus**, on the
  targeted decline's precedent — a contract you wrote going wrong is yours to
  see) and `own-job-failed` (a self-chosen job; `log`). A completion stays
  `quest-complete` and carries its tier on the note and on `Event::tier`.
- **Degrades to** the landed stub: with the module off nothing is rolled,
  nothing fails or goes well, the seed reaches nothing
  (`verify::seed_independence` asserts that half), and the pay is unchanged —
  the share is a wealth port, not a roll.

**petitions** — `src/petitions.rs` is the content (the template table, the
consequence vocabulary, the predicates); `src/pleas.rs` is the record and what
happens to one; `src/card.rs` and `src/plead.rs` are the card, its two
placements and their taps; `src/petitioned.rs` and `src/pleashots.rs` are the
battery.

- **Obligation is voicing** (owner, 2026-10-02). A petition is raised by a
  check and binds at delivery: at once to somebody in the camp, and by the
  messenger the asks module rides to somebody out — voiced at their next
  arrival anywhere, site or home. The deadline counts from that minute.
  There is **no accept, no decline and no promise**; decline and promise stay
  recorded variants in §10.
- **Every step is an occurrence**: the check (`Occ::Plea`, every `plea_hours`,
  staggered as the scorer is), the cliff (`Occ::Deadline`, addressed at
  voicing) and a walk-out's return (`Occ::Return`). Satisfaction is judged
  after every occurrence fires and after a gift — never per frame — so "met at
  any point before the deadline" is a world-time fact and the invariance sweep
  holds over all three sources.
- **One active petition per character**, in the data path: `pleas::raise`
  refuses anybody whose `active_petition` slot is taken.
- **The consequence fired is the card's reference** — `Template::consequence`
  points into `petitions::DECLARED`, the card prints that pointer and the
  cliff fires it, and the battery asserts the two are one address.
- **`walks-out` is presence, not a party.** It drops `Character::present`
  exactly as the staged start holds it before an arrival, so the map, the
  roster, the candidate picker and the work list follow through the lens's
  roll with no edit of their own; the absent are not rescored, not checked and
  not charged upkeep; and they come back unpaid. A walk-out declared while
  somebody is out on the road waits for them to come home — nobody walks out
  of a job half done.
- **A shortfall answered is a shortfall spent**: `thin-days` counts only the
  shortfalls since its last petition for that person was met or failed, or
  the same three would voice it again six hours after it was met.
- **Three source classes** (`petitions::SOURCES`): motivator, shortfall and
  director; the check raises the first two and the injector the third, and
  the vocabulary check requires the injector's registry row for any
  director-sourced template.
- **Degrades to**: nobody asks the player for anything — no petition, no
  deadline, no consequence; regard moves only through asks, wages, visits and
  drift; and nothing lowers desperation. `petitioned::module_off` asserts it
  over twelve attentive and idle days.

**events-director** — the minimal injector. `src/director.rs` is the module;
D1–D3 are rows of `petitions::TEMPLATES` (`CAST.md` §6); `src/scenario.rs`
and `games/ninjo/scenarios/` are the scenario file (§6); `src/directed.rs`,
`src/pressed.rs` and `src/eventshots.rs` are the battery.

- **It speaks only through petitions.** A firing raises a petition through
  `pleas::raise`, the one door, and from there it is a petition like any other
  — the same card, ledger, cliff and vocabulary of four. The registry row
  requires `petitions`, and `director::runs` reads both flags, so with
  petitions off the injector is silent too and the queue holds nothing of it.
- **Every firing is an occurrence**: `Occ::Director` on the one scheduler. The
  calm window ends with an unarmed occurrence at `calm_days`; each firing then
  draws the next gap at its own address — seed, world-minute, salt — between
  half and one and a half `director_hours`, so the mean is the drawer row and
  nothing is per frame or in call order. The target, the template, `{site}`
  and `{n}` are drawn the same way. A scenario's **pins** are `Occ::Pin`, at
  exactly their minute, past the calm window and the cap.
- **A firing reaches somebody or passes.** The eligible set is (person,
  template) in roster then table order — present, carrying no petition,
  holding a trait the row's `Carries` trigger names; a firing with nobody
  eligible, or with `director_max` already unresolved, passes with a feed line
  at `ignore`.
- **One class, `event`** (`ignore`): the firing is bookkeeping and the voicing
  it causes is what stops the world. Its id is the source table's display word
  for the class, so the player is never shown the word `director`; the card's
  source chip reads `event - from outside the camp - not their own want`,
  derived from `petitions::SOURCES` and asserted drawn whole.
- **The pressure params are `calm_days` / `director_hours` / `director_max`**,
  named for the drawer's fourteen-glyph name cell (`FINDINGS.md` G-058). The
  director is throttled by the cast's own petition density (G-057), and D1 is
  often met on the doorstep (G-056) — both in §10.
- **Degrades to** the quiet world: no event fires, no pin fires, no
  director-sourced petition is raised, and T1–T6 run exactly as before —
  asserted byte for byte by the scenario equality (§9).

**The registry as machinery.** `src/modules.rs` holds the table's shape
(`ModuleSpec`: id, tier, wave, degrades-to), the per-module disable flags
(`ModuleSet`, a bitmask planted as a resource before Startup like the
constants), and the matrix §9 iterates — seven rows, eight passes. Adding a
module is adding a row and reading `ModuleSet::enabled` where the module's
systems and data are installed; the matrix, the stamp and the reports all
walk the table, so nothing else changes.

## 6. Data formats (all data-defined, drawer-tunable, ASCII)

- **Trait row** (`traits::TraitDef`): id, name, kind
  (personality|motivator|aptitude), icon role, description (one
  stranger-facing ASCII line, validated), modifier set split by kind — bond
  and grudge multipliers plus the pot's pull for a personality, an upkeep
  multiplier, a scorer pressure and `favors` for a motivator, a competence
  value for an aptitude — with the reaction rows in their own table keyed by
  trait id. Every row carries every field; the neutrality rule (§3) is what
  makes that safe.
- **Petition/event template** (`petitions::Template`; one format — the
  director speaks in petitions): id, source class (motivator|shortfall|
  director), trigger (`Trigger`'s state predicates, `opens_day` as the
  world-time window, `rolled` for the seeded roll at `plea_odds` — addressed
  by occurrence as resolution's is — or `Carries` for the director's), the
  body (text with `{name}`/`{other}`/`{site}`/`{n}`/`{deadline}` and the
  bench's `{building}`/`{industry}`, **deadline in world-days on the row**,
  `Reward`, and the declared consequence as a reference into `DECLARED`),
  and `next` links both ways (this is quest chaining: a quest is a template
  arc). `TEMPLATES` is `CAST.md` §6 — T1 to T6, their chains, and D1 to D3.
  **Deadlines are template data, not drawer rows**: seven deadline rows
  would have filled the drawer the stamp needs, and a template is content
  the way the standing rates and the class table are; the drawer carries
  the petition numbers that *are* tuning. `petitions::vocabulary` asserts
  the table's own claims.
- **Scenario file** (`src/scenario.rs`; ASCII files under
  `games/ninjo/scenarios/`, one statement a line, compiled in with
  `include_str!`): `scenario <id>`, `seed`, `map`, `treasury`, `director
  on|off`, one `person` line per cast member in registry order (wallet,
  desperation, arrival minute), and `pin <template> at <minute> [for
  <person>]`. `scenario::parse` refuses a file it cannot read whole with the
  file, the line, what was wrong and the fix. **Freeplay** is the authored
  start (the same ten, their purses, desperations and arrival minutes, the
  treasury empty, the seed zero, the director on); **`pinned-collector`** is
  a test scenario — freeplay's camp, the director off, and one pin, D1 for
  Bob at minute 90 — loaded on the page with `?scenario=<id>`, refused loudly
  when no file has that id. The scenario id rides the opening log line, the
  tuning drawer's stamp and the verify report's stamp. **What a scenario does
  not carry**: the pressure params and the relationship preset stay drawer
  rows — the scenario says whether the director speaks and the drawer says
  how hard, because a number with two homes is two ways to move it; who the
  cast *are* stays `people::cast` (`CAST.md`'s content), and a scenario must
  open all ten in registry order (the party list and the roster are one
  list); a **predicate pin** is recorded in the format and refused by the
  parser by name — the tutorial that needs one is post-MVP. The tutorial is
  the most-pinned scenario; freeplay is the least.
- **Needs list** (`needs::NEEDS`): kind, interval, base cost — one row,
  `coin`, every `upkeep_hours`, at a base of `upkeep_coin`, both numbers
  `constants::Field` values on the row rather than literals in it, so the
  list is data and every number in it rides every stamp.
- **Petition card** (`card::card`, one card in two placements — `UI.md` §3h):
  who is asking (portrait, name, the source chip: the motivator row's own
  name or the class's display word, and the template id), the resolved
  words, a timer bar against the deadline (ember inside its last day), the
  reward — **"pays in regard" said honestly** where it pays no gold — the
  **"met when" line, derived from the variant the deadline's predicate
  matches** (`Condition::met_when` beside `Condition::met`), and the declared
  consequence as a chip whose explanation is derived from the vocabulary row's
  fields (`petitions::explain`), never written per card. No need-state on the
  card: the faces, the roster and the character panel carry desperation and
  the source line. The card carries no assign-picker: ARRANGE navigates to
  the surfaces that already carry one (the work list, the board and its
  candidate picker), and the willingness hint is their verdict column.
- **Posting record** (`asks::Posting`): id, who (character id | any), what
  (job id | site id | task type), until (done | withdrawn), wage,
  standing-rate-at-posting, made-at (world-minute), heard-by (character →
  world-minute), answered-by (character → agreed | declined + reason), status
  (open | filled | withdrawn). Sim state; replay-carried; the ledger drawer is
  a view of the vector they live in — there is no second list — and every
  posting is recorded input (a tap on a job row, a tap on WITHDRAW). The
  standing rates are sim state beside them and ride the scenario's opening
  stamp.

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

## 8. The wave plan (sessions are sequential within a wave; one session per handoff stands)

- **0b People** — the people substrate, the stores, the lens seam, the
  rename to ninjo. **Done.**
- **0a Attention** — mockup first, then the handoff. **Done.**
- **Wave 1 — done** (2026-10-02): 1.1 autonomy → 1.2 asks → 1.3 needs +
  settlement → 1.4 resolution → 1.5 petitions → 1.6 the minimal injector and
  the scenario file — all seven rows of §5's wave-1 column, each disableable
  and each off-pass green; closed by the wave-1 sanitation pass (the UI
  exemplar audit, then the history-bleed sweep). Each landed into a running
  world; the owner sanity-played between sessions but fun was not judged.
- **MVP gate playtest** — next. §7's question, under 1.6's own: does the
  world feel like it moves without you, at pressure mild enough that the gate
  stays about the loop?
- **3+** aspirations / threats / arrival (any order) → **4 parties**
  → knowledge when the experiment is wanted. Re-derive waves at each
  GDD refresh; the registry is the source.

## 9. Verify strategy

The claims, and what holds each. Every expectation in every battery is a
**shipped literal** — a check that derives its expectation from the constant
under test cannot see that constant move — and `target/verify/report.json`
is the verdict.

- **Module-off matrix** (`modules::matrix`): eight passes — the everything-on
  baseline, which also asserts the authored timeline, and one world per
  module switched off, each asserting the world moved and came to rest and
  the module's own degrades-to sentence: autonomy off, nobody decides anything
  and everybody idles at their own door; asks off, pure observation — no
  ledger, no postings, the site panel read-only; needs off, nothing is paid
  for — no upkeep burned, no wallet falls; settlement off, the camp stays a
  camp with nothing to build; resolution off, every worked job succeeds and
  the seed reaches nothing; petitions off, nobody asks the player for anything
  (`petitioned::module_off`); the injector off — and petitions off — the quiet
  world, in freeplay and in the pinned scenario (`pressed::module_off`). Green
  is the definition of modular.
- **Speed-invariance sweep** (`sweep.rs`): one authored scenario, one fixed
  script of recorded inputs at fixed world-minutes, under three speed scripts
  (all-1x, all-4x, a mix with pauses), judged over a world-time window
  (`sweep::WINDOW`, the scenario's first day — a world with people deciding
  things in it never comes to rest, and two speed scripts are only
  comparable over the same span of world-time): byte-identical transcripts,
  **sentences included**, because a replay that reproduced the choices and
  not the reasons would be reproducing half a decision. An input is addressed
  by `When::Approaching` — the conductor simulates the clock forward the
  ticks a tap takes and starts it early enough to *land* on the minute it
  names — and `sweep::orders_are_addressable` asserts every scripted minute
  is one the clock reads at 1x, 2x and 4x (`FINDINGS.md` G-016). The sweep
  runs again under a config that stops the world at every completion,
  resumed by the key the script is already running at: a pause stretches
  wall time and moves no world-time address. It is extended over drift, the
  scorer, postings and their messengers (a hearing asserted at the arrival
  minute it rides), and — to minute 16000, every stop a petition makes
  resumed by the player's own key — the petition check, the cliffs, the
  returns and a director firing inside the window
  (`petitioned::invariance`).
- **Economy sweeps** (`src/economy.rs`): **the population is the order in
  which ten people meet a finite board, times the seed** — world *k* opens
  with every scheduled first rescore rotated per roster place
  (`Sim::stagger_first_looks`) and rolls its jobs at seed *k*, and nothing
  else differs. Sixty-four worlds under each of two players over three
  world-days, driven through `sim::advance_to` (the same one door
  `sim::fire_due` fires occurrences through — there is no second
  simulation). **The bands** (`IDLE_TREASURY_BAND`, `IDLE_SHORTFALL_BAND`,
  `IDLE_WALLET_BAND`): the idle treasury 1233g, shortfalls nine to twelve,
  the median purse nothing. **The limp floor**, asserted in every idle
  world: no purse overdrawn, nobody past desperation 8 of a possible 10, every
  job finished eventually (a failed job goes back on the board), and —
  **the owner's own decision** — the slide is *uneven*: some of the camp goes
  short and some does not, and a world that pressed everybody or nobody
  fails. **Steve goes short first in every world** (`economy::PARIAH`,
  `CAST.md` §4.1). **The attention differential**: the same worlds under a
  scripted attentive player whose whole policy is three deliberately dumb
  rules (build the first industry once the treasury can pay; post the
  best-fitting open job to everybody idle at the standing rate; raise the
  most-refused rate a step when a day's refusals outran its agreements),
  measured as upkeep shortfalls over three world-days, **worst against
  every**: the attentive player's worst world goes short at least
  `ATTENTION_MARGIN_SHORTFALLS` (four) fewer times than neglect's best, and
  leaves the median purse at least `ATTENTION_MARGIN_WALLET` (17g) better.
  Worst-against-every, not median-against-median, because what is claimed is
  that competence beats neglect and not that it usually does. Every
  attentive world stands the industry up and therefore has work after the
  board is spent — that is the limp floor the settlement module is for, said
  as a fact about the sweep. Median desperation separates nothing and is
  reported rather than asserted: the person in the middle of the camp is not
  the person who slides. **The wage lever** (`economy::judge_the_wage`) walks
  the panel's ladder in the panel's own step and asserts it changes who takes
  a shift over the camp at its authored desperation.
- **Distribution sweeps**: over the compliance ladder (`src/compliance.rs`) —
  a seven-rung wage ladder against one job of each task type for every
  character, 280 answers a pass, with the four bands (loyal, cold, greedy,
  desperate) taking a pinned number of offers each; the shape is the
  design's: the loyal and the desperate comply most, the cold least; and the
  standing-rate walk, which changes somebody's *work* at pinned rungs and
  takes three steps from the shipped fight rate to move anybody — a question
  for the playtest, not a bug (`FINDINGS.md` G-021). Over outcomes
  (`src/outcomes.rs`) — every authored site job rolled at sixty-four seeds
  and four occurrence minutes, 6144 rolls a fit, with the tallies pinned and
  three bands (a strong fit fails under 15%, a poor fit over 28%, a strong fit
  goes well at least 18% of the time, and went-well is rarer than done
  everywhere), and the same bands over every played world of the economy
  sweep with at least one poor-fit job taken — desperate people must still
  rationally take work they are not fit for.
- **The petition-horizon sweep** (`src/petitioned.rs`, `src/pressed.rs`):
  sixteen worlds, both players, twelve days, director on. Voiced, failed,
  walked-out and grudge counts in pinned bands, met none by an idle player —
  consequences land and somebody always walks out and comes back; **the limp
  floor at this horizon is the shortfalls'**: the idle ceiling falls at
  minute 7200 with the petitions on or off (G-050), so the floor's three-day
  form is where it holds. No firing and no director petition before
  `calm_days` in any world; never more than `director_max` director petitions
  standing (`most_standing`); director petitions idle 1–2, attentive 3–8.
  **The attention differential, at this horizon**: the attentive worst world
  fails at least `PETITION_MARGIN` (six) fewer petitions and walks
  `WALKOUT_MARGIN` (four) fewer people out than neglect's best, and read
  against the same worlds with the injector off the petition margin is four:
  **external pressure widens the margin**, and `pressed::sweeps` asserts it
  does not narrow. The escalation pipe end to end (`petitioned::pipe`): Tim,
  empty-pursed, three shortfalls, `thin-days` voiced, failed, walked out,
  home; and the arranged twin, a job posted to him at the voicing and met by
  the player's hand, desperation down a step, the source line rewritten to
  *found paying work*. Both transcripts are literals.
- **One-source checks**: the feed equals the transcript filtered by the
  config (`attention::feed_is_a_view`, at both settings of the ignored
  toggle, with a check that the filter is hiding something so the assertion
  cannot pass vacuously); the `short` chip's set is the set the burn is about
  to press (`meters::registry` runs the burn and compares); the camp line's
  daily burn is the roster's own costs; every job row's, candidate row's and
  work row's odds word is the sim's odds for that pair, and moving `fail_base`
  moves the row and the roll together (`outcomes::judge_one_function`); over
  all 280 staged offers the job row's verdict and what the character does
  about the same posting agree; the breakdown band's terms are the recorded
  `Judged`, term for term; every consequence fired over twelve idle days is
  the card's own reference, and a mutated deadline, `{n}` or consequence
  moves the card and the firing together.
- **Determinism**: the same seed replays the choices, the sentences, the
  petitions and the director's firings word for word, and another seed parts
  them; the seed reaches the roll, the petition check's roll and the
  director's draws and nothing else (`verify::seed_independence`: at seeds 7
  and 7,777,777 the transcripts part, and with resolution off they are
  identical); the pinned scenario's D1 for Bob fires at minute 90 exactly
  — its feed line, its raise and its voicing — under all three speed scripts
  (`pressed::pinned`); and **the scenario equality**
  (`directed::scenario_equality`): the freeplay file's opening and twelve
  worlds of twelve days (seeds 0, 7, 42; idle and attentive; the injector
  switched out *and* the file's own `director off`) fingerprint
  byte-identical to the authored start the file replaced (FNV-1a over every
  event, person, petition and port).
- **Floors and captures**: every surface's rows in the `Panel`, judged by
  `floors.rs` and found on the frame by `frames.rs`; forty-one photographs
  per run, each asserted to be what it says. `UI.md` §4–§6 own the floors,
  the set and the rule for a new surface. Stamps carry seed, scenario,
  constants and the module set — the verify report and the scenario's
  opening log line both.
- **The mutation round** (`src/mutation.rs`): every drawer constant moved
  once and noticed by some instrument written with shipped literals — 53 of
  53 — plus D1–D3's row literals (`directed::rows_round`: deadline, `{n}`,
  spread, consequence, traits, site slot, condition and words each moved
  once, 24 of 24). A mutated wage, upkeep, odds or pressure constant must
  break a band.

## 10. Confidence & open ledger

**Wave 1 is built and swept, and not yet played as a whole.** Every module of
§5's wave-1 column is in, each is disableable and green with itself off, and
the sweeps say the economy holds its bands, the limp floor holds, competence
beats neglect by a stated margin, and the director widens that margin rather
than narrowing it. Whether any of it *reads* — whether a player feels the camp
needing them, sees what looking away cost, and finds being asked things a loop
worth keeping — is the MVP gate's question (§7), and nothing here can answer
it.

Foundation: grid/clock/pathfinding **played**; the people substrate
**played**; attention **owner-playable**. The wave-1 modules are
**speculative until the MVP gate** — correct and expected; the gate converts
speculation to played evidence.

**The director, beyond the minimum.** Wave 1.6 is the *minimal* injector —
three canned templates at a fixed mean cadence, capped, after a calm window.
**The full director is post-MVP**: pressure curves, storyteller pacing, threats
(§5's `threats` row, wave 3) and templates beyond D1-D3 are deliberately not
built, and the MVP gate is meant to measure the character loop under mild
pressure, not a storyteller. The tutorial — the most-pinned scenario — is
post-MVP too; the scenario file carries world-time pins and records predicate
pins without building them.

Open, from the economy and resolution (all in `FINDINGS.md`): **the camp runs
out of work before the band is whole** (G-032; the industry answers it and an
idle player never builds one); **the share Steve's claim allows is three
percent** (G-048); **the scorer does not price risk** (G-049); **the difficulty
seam** — odds are a function of fit alone, and a per-job difficulty would be
one more input to `resolution::odds`, deliberately not built; and **should fit
matter at an industry?** — a shift is never rolled.

Open, from the petitions and the director: **an idle camp is at the
desperation ceiling before any cliff falls** (G-050); **T3 asks for paying
work and is judged on desperation** (G-051); **ARRANGE on `thin-days` opens an
empty work list** once the board is spent (G-052); **nearly everybody carries
a petition by day four** (G-053), which is also why **the director is
throttled by the cast's own petitions** (G-057) — one or two director
petitions in an idle camp's twelve days, three to eight in an attended one;
and **D1 is often paid on the doorstep** (G-056), a pressure event that
relieves pressure when the purse already holds the debt. Recorded, not built:
**decline and promise** stay variants of the card (obligation is voicing —
owner, 2026-10-02); **knowledge-gated credit** waits on the knowledge lens;
`proud`'s refusal of a gift has no field, because no proud character carries a
money-shaped petition.

Open (deliberately): **the expectation model beyond the standing rate**
(fit-adjusted? regard-adjusted?) · **open postings travelling** (heard at camp
only) · **standing-posting fatigue** · **the ask-spam question** (does asking
spend regard?) · **whispers' rumor vocabulary and spread model** (capsule) ·
**a surface for site-shaped postings**, and a messenger anybody can see ·
whether the wave-1 class defaults survive a real petition load (the MVP
playtest) · **the trait vocabulary is locked** (owner, 2026-10-02): petition
copy is written against the shipped words, and a change now is a rename, not
a data edit — `CAST.md` §7 carries the question the playtest asks of them ·
aptitude-change mechanism (two candidates recorded) · bond/grudge erasure
rules · quest authoring surface beyond template `next` links · settlement
stock list beyond gold-only (bound to a famine/siege design need) · **a
scenario with fewer than the ten** (the party list and the roster are one
list) · map generation (post-GDD session; requirements now stateable: one
town, sites, terrain variety, readable at 8–12 characters' scale).
