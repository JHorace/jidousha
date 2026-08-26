# giri — design (v2)

Home: `games/giri/DESIGN.md` — the game's design doc travels with the game
(ADR-0038: games are workspace members; this file is game-side, not
engine-side, so `docs/internal/` shape rules do not apply to it).

v1 was drafted in the design session of 2026-08-22 and built across four
sessions. This revision was drafted in the design session of 2026-08-24, from
the owner's v1 playtest verdict, approved by the owner the same day, and
landed with the P1 implementation (2026-08-26). Sections whose systems are
later phases carry their design ahead of their implementation — the document
states the design, and **Implemented (P1)** notes mark what code exists;
§15 is the phase map.

Status legend: **DECIDED** (owner-settled, dated), **PROPOSED** (this
document's synthesis awaiting sign-off), **OPEN** (future design work).

義理 (giri): duty, obligation, the web of what people owe each other.
ぎりぎり (girigiri): barely scraping by.

---

## 1. Concept, and what v1 established

An auto-battler where the pieces have interests. The player sees a job's
parameters in advance, assembles a party from a roster, and resolution is
automatic — no attack/defend verbs. The inversion: roster members are not
inert units. They consent, refuse, betray, bond, and remember. The player's
verbs are entirely social — selection and juxtaposition under pressure.

Second inversion, protected deliberately: **the player's profit and the
characters' welfare pull apart.** The tutorial's second beat teaches the
player to profit from a death they could foresee. The player is complicit,
and the game does not soften this.

**v1's hypothesis** — inter-character dynamics alone can carry play —
returned its verdict in the owner's playtests (2026-08-24): *they carry
dilemmas, not yet a game.* Three findings, and they are what v2 answers:

1. The willingness arithmetic is legible but the player should not be doing
   it; the judgment must become rich enough that reading it beats computing
   it.
2. Deterministic betrayal is avoidable-by-arithmetic, so it never happens
   except on purpose, and when it does it feels dictated rather than risked.
3. Characters are constraint-bundles: nothing individuates them, so their
   deaths are logistics, not losses.

Positive finding: desperation is the keeper — the pressure mechanic the rest
of v2 builds around.

## 2. Design invariants (DECIDED, amended 2026-08-24)

1. **Social dynamics only.** No strength/int-style stats. Traits, marks and
   edges are the same species as v1's relational attributes — still social,
   still not combat math.
2. **The game never lies and never hides — restated, not weakened.** All
   rules and all state remain inspectable. What changed is the *surface*:
   the game shows judgments (will join / reluctant / refuses / blocked) and
   their **reasons as words** ("won't work with a comrade-killer" · "needs
   the money" · "trusts Alex"); the numeric machinery sits behind inspection
   rather than on the card. Exact mental computability stops being a design
   goal — the heuristic era is entered deliberately, through depth (more
   interacting causes), never through concealment. Hidden character
   knowledge (undisclosed traits) remains a **reserved variant experiment**,
   not v2 behavior: v2 sheets are complete.
3. **The authored puzzle chain is superseded by the open run** (§10). Beats
   survive as *seeded scenarios*: the tutorial and the tuning/regression
   harness, no longer the game's body. (P1 still plays the four-beat chain;
   the run is P3.)
4. **Resolution is pressure application, not combat simulation.** A dungeon
   exists to force the social rules to fire. Deepening resolution later
   means more moments where the decision function runs — never more stat
   math.
5. **Determinism — the sim-phase clause activates in P2.** Seeded randomness
   enters simulation, exclusively via the engine's `Rng` (PCG32,
   replay-exact everywhere). Outcomes are a pure function of (scenario,
   player choices, constants, variant, **seed**). Recordings carry the seed;
   replay identity is unchanged in kind. No other randomness source, ever.
   **P1 has no randomness at all** — its outcome is a pure function of (beat
   state, player assignments, constants), exactly as v1's was.
   *Implemented (P2):* the `Rng` is read at resolution and nowhere else —
   the willingness surface is asserted identical under far-apart seeds every
   verify run — and the seed machinery is §8d.

## 3. The character sheet v2 (DECIDED in shape; caps DECIDED)

A character is: **portrait · traits (≤3) · desperation, with its source ·
wealth · reputation marks · bonds/grudges (regard edges) · one active goal ·
biography**. Everything on the sheet is one of: who they are (traits), what
they've lived (marks, biography, edges), what presses on them (desperation +
source, wealth), what they want (goal).

**Caps are design decisions, not UI accommodations** (owner direction,
2026-08-24): at most **three traits**; exactly **one active goal**; marks
made rare by the ladder's construction (§8) rather than by display
truncation. If the sheet overwhelms at these caps, that is a presentation
problem; without caps it would have been a design failure wearing a UI
costume.

**Desperation's source is bound at character generation** — a short phrase
on the sheet naming *why* this one is hungry. Flavor-plus-data in P1; the
goal machinery that makes sources mechanical is P3, and two identical
numbers already read as two different management problems.

*Implemented (P1):* traits, desperation + source, wealth, marks and edges
are on every sheet; the trait cap is enforced by data validation over every
authored roster (`traits::vocabulary`), not by prose. Goal and biography are
P3.

## 4. Traits (DECIDED: visible on hire; the shipped list is tuning content)

A small, closed, data-defined vocabulary — order of 12–20 — each trait: an
id, a display name, an icon role, and a set of **modifiers to the decision
function's terms**. Sketch of the register (final list is tuning content,
not architecture):

*greedy* (pot terms weigh on them; skim-prone when the ladder lands) ·
*loyal* (bonds weigh double; betrayal floor higher toward bonded — P2) ·
*proud* (will not stand with a thief; refuses charity — P3) · *craven*
(fears a killer; danger terms weigh double — P2) · *vengeful* (grudges weigh
double and never decay; eager toward grudge-resolving quests — P3) · *pious*
(reacts to marks by kind, not size) · *pragmatic* (prefers known quantities
— a marked skimmer over a stranger) · *cold* (edges weigh half, both ways) ·
*upright* (refuses the dark-marked, whatever it costs) …

Traits are per-character multipliers/filters on shared rules — the decision
function stays one function receiving its inputs; **traits parameterize,
they never branch gameplay code** (the §8b tier-2 discipline, applied to
people: a trait is data the function reads, never an `if` in a system).
Assignment: rolled at character generation from templates; **mutated only by
history** (§8, §9). Visible from hire, always (DECIDED; hidden/discovered
versions are reserved variant experiments).

*Implemented (P1):* nine traits ship (`traits.rs::TRAITS`) — the eight of
the register plus *upright*, which §5's own example calls for. Each row
carries exact rational multipliers for bonds and grudges, a pot affinity,
and its cells in the trait×mark table; the icon roles are category icons
from the existing library (interim, UI.md §13). Traits whose registers name
P2/P3 systems (craven's danger, vengeful's eagerness, proud's charity) carry
table cells now and grow their real modifiers with those systems.

*Implemented (P2):* the ladder-era trait effects landed as two more data
tables — severity biases (greedy → skim, craven → abandon, vengeful →
sabotage and murder; `ladder::SEVERITIES`) and pressure biases (greedy
under a fat pot, vengeful beside a grudge; `pressure::PRESSURES`). Loyal's
"betrayal floor higher toward bonded" arrives through the mechanism rather
than a table cell: loyal doubles bonds, bonds at `K_loyal` suppress (§8),
so the loyal hold the line sooner. Every row also carries a one-line
behavioral description, shown on chip hover (the P1-playtest warm-up;
UI.md §14).

## 5. Reputation marks (DECIDED — replaces scalar infamy)

**Infamy the number is retired.** Public knowledge becomes **marks**:
qualitative, earned, plural, written by witnessed events. The v1
public/personal split survives sharpened: regard is what *this* character
feels; marks are what *everyone* knows.

- Marks come from the betrayal ladder (§8) and from conduct: *skimmer ·
  deserter · saboteur · comrade-killer* on the dark side; *reliable (N clean
  jobs) · kept-the-line* on the light; *survivor — parties die around this
  one* for the ambiguous.
- **Reactions are trait×mark, and they open doors as well as close them**
  (DECIDED): the upright refuse a comrade-killer; the desperate swallow it;
  the pragmatic *prefer* a known skimmer to an unknown; certain quests (the
  underworld register) require a dark mark on someone in the party — the v1
  "infamy only closes doors" problem is resolved structurally.
- Marks are hard to lose: **goal completion is the only eraser** (§9). No
  passive decay in v2 (OPEN for later: slow decay of minor marks).
- **Guild marks** exist as a reserved hook (*keeps its word*, *spends its
  people*): written by player-visible conduct, read by recruits and quest
  givers. Minimal in v2 (§17 — the player-as-legible-actor question).

*Implemented (P1):* the seven-mark vocabulary and the trait×mark table are
data (`traits.rs`); a reaction is the mark's tone base (`mark_dark` /
`mark_light`, ambiguous 0) plus the looker's table cells. This phase's
writers: **murder writes comrade-killer** (where v1 wrote infamy +N) and
**clean-job counting writes reliable** at `reliable_after` jobs; the other
marks are written by the ladder (P2) and appear in P1 only as authored
backstory. The quest predicates migrated: needs-a-known-face became
**needs-a-dark-mark** (`Requirement::NeedsDarkMark` / `NoDarkMarks`). A mark
is written once — it is a fact, not a counter.

## 6. The decision function v2 (DECIDED in shape) — *Implemented (P1)*

One function, as ever, now richer inputs:

```text
willingness(c, party, quest) =
    desperation(c)
  + sum over m in party, mark on m: reaction(traits(c), mark)
  + sum over m in party: regard(c->m), as traits(c) weigh it
  + the pot, as filtered by traits(c)
  [+ eagerness — the quest's relevance to c's goal — P3]
```

Output: a **verdict** (will join / reluctant / refuses) with its **margin**,
and the **reasons** — the top contributing causes, rendered as words (§14).
The margin's boundaries: negative refuses; non-negative but under
`reluctant_below` is *reluctant* — in, but barely.

- **Reasons are a fixed vocabulary, as data** — one ASCII template per cause
  kind (needs the money · won't work with a `<mark>` · prefers a known
  `<mark>` · trusts `<name>` · despises `<name>` · the money is good ·
  nothing pulls either way), never free-form debug text. Every rendered
  verdict carries at least one reason; a sum with no causes falls back to
  the indifferent template by construction.
- **The door rule stands unchanged** and evaluates through the v2 function.
  A character `c` may be added to party `P` iff (1) `willingness(c, P+{c},
  q) >= 0` — the newcomer consents — and (2) no incumbent's willingness
  would go negative; an incumbent who would blocks the arrival, and the UI
  names the blocker and their reason. **Consent is evaluated at the door
  only** (owner, 2026-08-23): later departures do not re-evaluate, the send
  gate checks headcount and the composition predicate and nothing else, and
  a member pushed negative by a departure stays, in ember, with their reason
  on the card.
- **The margin is no longer discarded at the door**: it is computed and
  stored on every answer as **strain** — how reluctantly this party holds
  together — and strain is a primary betrayal input for the ladder (§8).
  *Implemented (P2):* the margin gained its reader — §7a's strain component
  maps it onto named constants. The deterministic variant (§8e) still does
  not see it, which is part of what it preserves.
- **Bonds, mechanically** (DECIDED 2026-08-22, unchanged): a bond (positive
  regard) overrides public information (it enters the same sum the mark
  reactions do), suppresses betrayal (`regard >= K_loyal`), and propagates
  consequences (harm to a bonded character creates a grudge in the survivor
  toward the killer). Package deals stay OPEN.

**The retained deterministic betrayal (v1's rule, now the preserved
variant).** Through P1, resolution kept v1's exact rule: after success, in
roster order at both levels, with kills taking effect immediately —

```text
betray(c, t) iff desperation(c) >= K_kill
           and shareGain(c | t dead) > 0
           and regard(c->t) < K_loyal
```

*Implemented (P2):* the ladder (§8) replaced it as the shipped rule, and
this rule survives verbatim as the `deterministic` variant (§8e) — the same
function, still called, still deterministic, for comparison playtests.
Regard here is the raw edge, unweighted by traits — the trait weights are
the *decision function's*, and this rule is v1's economy arithmetic,
preserved rather than polished.

**Aftermath (unchanged in rule, extended in pen):** a clean run bonds every
surviving pair (+`bond_gain` both ways, per run — a job somebody died on
bonds nobody) and counts one clean job for every survivor; a betrayal writes
the killer's mark, drops every surviving witness's regard toward them
(`witness_grudge`, plus `bonded_grudge` if the witness was bonded to the
victim); round end, every living non-profiter's desperation rises and every
profiter's falls, floored at `desperation_floor`.

*Implementation notes (P1, 2026-08-26) — choices the formulas leave open:*

- **The pot term is trait-only in P1**: base pot weight is zero, and the pot
  pulls through a trait's pot affinity at `pot_pull` per gold of share. "The
  pot as filtered by traits" is implemented literally; a base pull for
  everyone is a tuning decision available any time (it is one constant).
- **The share the pot term reads is the job's at its stated headcount** —
  what the sheet promises — not the split among the party staged so far, so
  an answer does not wobble while the party is assembled.
- **With no quest taken, no pot pulls**: the quest is part of the question,
  and the door evaluates against the taken quest or against none.
- **Reasons sort by absolute contribution**, stable, with build order
  (desperation, then each member's marks and edges in roster order, then the
  pot) breaking ties — deterministic like everything else.

## 7. Dungeons and resolution (carried from v1; predicates migrated)

v2 dungeon = requirements + pot + payout rule, everything visible before
assembly:

- **Requirements**: headcount, plus composition predicates from the social
  vocabulary. The mark predicates (§5) are the current axis: a job that
  needs a dark mark on somebody, a job that cannot be seen with one. (The
  organized-crime read of "a job that needs a known face" survives the
  migration intact — it needs a known *mark* now.)
- **Pot and player's cut**: visible before assembly, like everything else.
- **Resolution order** (stated, deterministic in shape): willingness checks
  happen at assembly time in the UI (refusals are *feedback*, not
  failures); betrayal events in roster order under the chain's rule set
  (§8, or §8e's preserved rule); the desertion re-evaluation (§8c); payout
  through the skim and sabotage arithmetic; bond drift and clean-job
  counting; mark writes; round-end desperation drift.
- **P2 opens exactly one failure path**: a desertion re-evaluates the
  quest's success against the remaining party (§8c), and a job left short
  fails — no payout, no cut, said loudly on the takeover. An under-filled
  party still cannot be *sent*; every other failure semantics waits for the
  run era's design (OPEN).

## 7a. Pressure, strain and the foreshadowing bands (DECIDED 2026-08-26)

*Implemented (P2).* At resolution, each party member has **pressure**: an
integer computed from
already-visible state, by one function —

```text
pressure(c) = strain_component      (the door margin, mapped: reluctant or
                                     pushed-negative -> +strain_reluctant;
                                     comfortable -> 0; margins above
                                     eager_above -> -strain_eager)
            + desperation_component (desperation x hunger_weight)
            + trait_component       (per-trait biases, as data: greedy under
                                     a fat pot, vengeful beside a grudge)
            + opportunity_component (the gold a member would gain if one
                                     fewer split the pot x opportunity_pull
                                     - pot size relative to share, and party
                                     size, in one number)
```

floored at zero. **The foreshadowing bands derive from the same pressure
numbers the rolls consume** — *calm / uneasy / powder keg* are named cutoffs
(`uneasy_at`, `powder_keg_at`) on the party's highest member pressure: the
most dangerous person sets the mood. One source for the roll and the warning
is what makes the foreshadowing unable to lie (invariant 2); the bands are
UI vocabulary, the cutoffs are constants in the drawer, and the **party band
chip** is visible before SEND (UI.md §14). The strain mapping is the stored
margin's reader (§6): willingness stays deterministic — what it *risks* is
what became probabilistic.

## 8. The betrayal ladder (DECIDED, refined 2026-08-26) — *Implemented (P2)*

Betrayal is a seeded probabilistic event on a severity ladder:

**skim** (take an extra share) → **abandon** (walk mid-quest; the quest's
success re-evaluates without them) → **sabotage** (the pot is damaged, the
job soured) → **murder** (the v1 event, the rare summit).

**The roll — seeded, ordered, legible.** In **roster order**, each member
rolls once against their pressure (§7a): occurrence fires when a roll of
`0..occurrence_die` lands under `pressure - occurrence_calm` (the roll
forgives a calm party's pressure outright, which is what keeps their
betrayals rare rather than merely less common). On occurrence a second
bounded roll picks severity from the rungs *available at this pressure* —
each rung has a floor, held as data beside its base weight — weighted by
trait biases (greedy → skim, craven → abandon, vengeful → escalate).
Randomness decides *whether and how bad*; **target selection stays
deterministic** (legibility: the dice never choose the victim — the
relationships do). At most one event per member per quest; rolls use
start-of-resolution state; a member murdered before their turn never rolls,
and a member bonded (`regard >= K_loyal`) toward everybody still present
holds the line and never rolls either (§6's suppression clause).

- **Murder is structurally gated: unreachable below the powder-keg
  cutoff.** The murder rung's floor *is* `powder_keg_at` — the band chip's
  own top cutoff, one constant — so "visibly telegraphed before it can
  happen" is a property of the model, not a UI promise, and the sweep
  (§8f) asserts it as *exactly zero* murders below the floor, never as a
  rarity band. Murder is also infeasible without a v1-legal target: no
  profit or everybody held at `K_loyal` takes it off the table.
- Every rung writes: a mark (public), edges (the wronged), and a
  resolution-report line in the mechanical-narration style. The ladder is
  the reputation system's pen.
- Design intent named plainly: v1 punished a computable mistake; v2 charges
  for a chosen gamble. Pressing reluctant, hungry people into service is
  *priced in risk* — the strain component is that price — and benching
  them raises tomorrow's price (§11). Avoidance is never free.

## 8a. Tuning and playtesting (DECIDED 2026-08-24)

Balance questions (the constants, beat difficulty, the heuristic-onset
point) are answered by playtesting, through two channels:

**Agent self-playtest.** Agent sessions play giri via `InputScript` and
sweep constants against the beats. The verify report includes the constants
in effect for the run, so a tuning sweep is scriptable: same beat, varied
weights, machine-readable outcomes.

**Human playtest with a live tuning menu.** A debug UI exposes every
constant for on-the-fly adjustment. Built with the same quads, sprite font
and pointer input as the rest of the game; no engine pull. Built 2026-08-24
(`src/tuning.rs`); UI.md §9a owns its presentation.

**The determinism interaction:** tuning constants are *simulation inputs* —
replay state is a pure function of (beat state, assignments, constants). The
menu applies changes **at beat boundaries** — adjusting a constant restarts
the current beat with the new values — and the constants in effect are
stamped into every recording and verify report, so any run remains exactly
reproducible. Mid-run tuning would need constants to enter the recorded
stream like input does — an engine conversation, explicitly out of scope.

Playtesting instrumentation serves the heuristic-onset question (§17):
per-beat assembly duration and sheet-look counts are logged locally
(`src/onset.rs`), nothing leaves the machine.

*Implemented (P1):* the drawer covers the five v2 constants exactly as it
covered v1's — it walks the constants module by design, so the new rows,
stamp keys and `?constants=` keys appeared without the drawer being edited;
the presets were re-derived for the wider set (`src/presets.rs`).

*Implemented (P2):* the same walk carried the ten ladder-era constants in;
the drawer grew to hold twenty-three rows (UI.md §14 owns the geometry),
the presets were re-derived again, and the stamp gained the variant id and
the seed (§8d) — a recording says *everything* it ran with.

## 8b. Variants — how incompatible mechanics coexist (DECIDED 2026-08-23)

Iteration on giri will produce mechanics that cannot all be true at once.
Which mechanism carries a variant follows from **how deep the divergence
goes**, in three tiers.

**Tier 1 — different numbers: not a variant.** Same rules, different
constants is a *tuning preset* — a named constants set, handled entirely by
the §8a machinery (one constants module, stamped into recordings and
reports, adjustable at beat boundaries). Never a flag, never a binary.

**Tier 2 — different rules, same shape: one binary, variants as data.** A
variant that swaps *which rule* fires at the decision function's moments — a
different betrayal condition, package-deal bonds on, another bond-drift law
— while keeping the beat format, the screen flow and the state shape, lives
in the mainline crate as a `Variant` chosen at chain start.

Structural constraint, enforced by review: **variant selection happens in
exactly one module**, which assembles the rule set at startup — never inline
`if variant` branches through systems. One file states what every variant
is, and the decision function stays one function receiving its rules. That
is the same discipline §6 already imposes for the same reason — and §4
imposes it a third time, for traits.

The variant id is a **simulation input** exactly as the tuning constants
are: part of replay identity, stamped into every recording and verify
report. Verify runs beats × variants the way §8a's mutation round sweeps
constants. On the web, the variant picker sits at chain start, beside where
the tuning menu lives.

**Tier 3 — different loop, screens, or state shape: fork the crate.** When
flags would lie about the divergence, the variant becomes
`games/giri-<name>/` — a sibling workspace member, which ADR-0038 makes
nearly free. Two disciplines keep forks honest: every fork carries a
`VARIANT.md` stating the hypothesis it exists to test and what "decided"
looks like, and forks are short-lived and few (at most ~2 alive); the loser
moves to `attic/` with its verdict recorded.

**Deliberately deferred: a shared gameplay library** — the second-consumer
rule, applied to infrastructure (the reasoning is unchanged from when this
section was written; see the repository history for the long form).

**Decision procedure, compressed:** only numbers → tier 1. Expressible as
choosing rule implementations at startup, without touching beats, screens or
state shape → tier 2. Anything deeper → tier 3.

*Implemented (P2):* the machinery's first real instance is §8e — one module
(`src/variant.rs`) holds the id, the one `match`, and the picker's data;
the id is a resource stamped into every report and settable with
`?variant=`; verify runs the beats under both rule sets. The section's
design survived contact with one bend: the chain-start picker lives *inside*
the tuning drawer (its natural neighbours are the other simulation inputs),
and picking a different rule set restarts the chain from the top — rule-set
assembly is chain-start, so a new rule set is a new chain, immediately
rather than pending.

## 8c. The rungs — consequences, all public at resolution (DECIDED 2026-08-26)

*Implemented (P2).* All betrayals are known by the resolution report (the
game never hides from the *player*; whether characters in-world could have
missed a "quiet" skim is a reserved hidden-info variant, not P2):

- **Skim** — takes one share off the top before the split (the shipped
  arithmetic: the skimmer gets a full share extra, everybody splits what is
  left); mark *skimmer*; small regard hits from the shorted.
- **Abandon** — leaves mid-quest: the quest's success **re-evaluates
  against the remaining party** (headcount and predicates; the murdered
  still count — they did the work before they died, which is v1's own
  semantics kept). A job left short **fails**: no payout, no cut. The
  deserter takes no share; their hunger still rises; mark *deserter*;
  regard hits from those left holding the job.
- **Sabotage** — the pot is damaged by a named fraction (`sabotage_loss`
  twelfths of the pot) and the quest is soured; mark *saboteur*; strong
  regard hits.
- **Murder** — the v1 event, unchanged in its writes (comrade-killer,
  witness grudges, bonded-grudge propagation, death); target selection is
  the v1 deterministic rule, against the members still present.

Every rung writes its mark once, its edges, and a report line naming the
numbers the roll read (pressure, roll, die, and the rung's own arithmetic).
A quest with any betrayal in it bonds nobody and counts as clean for
nobody; events resolve in roster order and later members inherit only the
shrinking room (rolls use start-of-resolution state — drift happens after).

## 8d. Seeds (DECIDED 2026-08-26) — *Implemented (P2)*

giri reads the engine `Rng` — the first and only randomness source
(invariant 5) — **seeded per scenario**: every beat carries a fixed seed as
authored data, re-fixed at the beat boundary (so each scenario's outcome is
a pure function of scenario, choices, constants, variant and seed, whatever
came before it, and an APPLY's restart replays exactly). The web page
accepts `?seed=` beside `?constants=` and `?variant=` as a session-wide
override; a verify scenario rides its seed in on `GameConfig::seed`. The
seed joins every stamp, report and recording — the drawer's stamp block,
the per-beat log line, the verify report's beat rows — so a repro link is
`?constants=...&variant=...&seed=...`. **No `Rng` read happens outside
resolution**: willingness stays deterministic, and verify asserts the whole
assembly surface identical under far-apart seeds.

## 8e. The deterministic variant (DECIDED 2026-08-26) — *Implemented (P2)*

The §8b machinery's first customer. Two variants ship: `ladder` (default,
§8) and `deterministic` — the v1 betrayal rule, **preserved verbatim** for
comparison playtests: the same `model::betrayals` function v1 shipped,
still called, never reimplemented, its narration byte-identical and its
outcome seed-independent (asserted). The variant id is a simulation input —
a resource, stamped into recordings, reports and the `?constants=` link
family (`?variant=`), picked at chain start (the picker sits in the tuning
drawer; switching restarts the chain). Verify runs the beats under both:
the deterministic beats keep their v1 assertion lists; the ladder beats are
fixed-seed. Under the deterministic rule the band chip does not draw —
foreshadowing is the ladder's obligation, and v1's stance (the player does
the arithmetic) is part of what the variant preserves.

## 8f. Verify — fixed seeds and distributions (DECIDED 2026-08-26)

*Implemented (P2).*

- Beats: fixed-seed exact assertions, as established — two lists per beat
  (§8e), plus exact pressure and band assertions on the staged party (the
  numbers the rolls consume, hand-computed).
- **Distribution sweeps** (a verify phase): 200 seeds over authored
  scenarios asserting **bands** — the calm party betrays rarely, the
  pressed party sits between, the powder-keg party betrays in most runs,
  skims dominate the severities, murders are rare — and **zero murders
  below the floor** (exact, not statistical: the one hard count, because
  the gate is structural). Sweep results land in the report with constants,
  variant and seed range.
- The mutation round extends over the new constants and the band cutoffs —
  a perturbed cutoff must break a band assertion (every sweep scenario also
  asserts its band deterministically, which is what lets the round see a
  moved cutoff without paying for a sweep per perturbation).

## 9. Goals (DECIDED — the wonder layer; **P3**, design ahead)

Every character carries **one active goal**: discrete, named, stated in the
game's own vocabulary, with visible progress on the sheet — a wonder track
per person.

- **Templates, bound per character** (authoring economy): earn-N-for-self
  (debt, dowry, stake) · erase-mark-M (clear her name) · complete-quest-Q
  (see the Black Vault opened) · resolve-edge-with-P (settle the score,
  repay the debt of gratitude) · accumulate-and-leave (retire wealthy). The
  template set is data; new templates are design events.
- **Completion mutates the sheet meaningfully** (DECIDED): trait rewrites
  (*craven* → *steady*), desperation re-anchored or its source removed
  outright (the debt paid is a pump deleted), marks erased (the only
  eraser), bonds forged — the capstone events of history-as-progression.
  **accumulate-and-leave completes and the character retires** — off the
  roster, with a legacy (a recommended recruit, a parting gift, a guild
  mark). A wonder whose payoff is losing the piece, on purpose.
- **Goals can curdle** (DECIDED): failure conditions in the same vocabulary
  (the creditor calls it in; the accuser dies un-confronted), mutating the
  sheet darkly (the debt becomes *hunted*; the score becomes a trait). A
  goal is stakes, not a savings account.
- **Eagerness** (§6): quests advancing a goal pull that character in — they
  join parties they would otherwise refuse. The player may exploit this; the
  game notices (guild marks, §5 — OPEN how sharply).
- Desperation differentiation falls out: the goal names *why* this character
  is desperate, and two identical numbers become two different management
  problems. (P1 already binds the source as flavor-plus-data, §3.)

## 10. The run (DECIDED in shape — **P3**, design ahead; supersedes the chain
as the game's body)

giri v2 is an **open run**: rounds of quest selection → party assembly →
resolution → consequences, against a **generated quest stream** drawn from
quest templates — including **goal hooks** ("her accuser is garrisoned at
the Watchtower"), which is what makes the stream a landscape rather than a
job board (and gives the eventual overworld map destinations that matter).

- **The roster exceeds the work, always** (DECIDED): more mouths than shares
  is the standing pressure that makes every selection a portfolio decision
  (§11).
- **Recruitment**: a modest candidate stream (sheets visible — traits,
  marks, goal; the initial-assessment moment the sheet exists for) refills
  attrition from death and retirement. PROPOSED: candidates' quality and
  kind react to guild marks (reserved hook, minimal v2).
- Run length / end state: OPEN (survival? insolvency? a chain of guild
  goals?). v2 ships as an endless run; the loss condition is designed after
  the loop proves out.
- The **beat chain becomes the harness**: beats 1–4 re-authored as seeded
  scenarios (fixed roster, fixed seed) serving as tutorial and as the
  verify/tuning regression set. Beats 5–15 as previously imagined are
  **cancelled** — superseded by the run.

**Beat authoring (current, P1):** a beat = (initial roster state, dungeon(s),
the intended dilemma in a sentence, expected-outcome assertions), plus the
`send` field naming the party the verify scenario assembles. The chain lives
in `src/chain.rs` as data and is read by no code that names a beat number.
*Implemented (P1):* beats 1–4 re-authored minimally onto the v2 machinery —
same dilemmas, new causes (§18 lists every assertion that moved and why).

## 11. The portfolio economy (DECIDED in shape)

Each job has a **pot**, split among surviving participants after the
player's stated cut — fixed pot + division among survivors is what makes
desperate betrayal *economically rational*. The designed-dilemma knob
survives from v1: a job that requires N but pays fewer than N worthwhile
shares. Non-participants don't profit, so their desperation rises each round
— the roster decays toward willingness, and refusal is always temporary.

The long-term axis is **people plus goals** — no separate meta-currency:

- **Investment**: history writes value — bonds from shared work, *reliable*
  marks from clean jobs. Taking the unproven kid on the wrong job is how she
  becomes somebody: worse odds today, better roster tomorrow.
- **Upkeep**: v1's hunger rule stands — the unfed get hungrier — now with
  **explicit per-character wealth** as the buffer (how many benched rounds
  until dangerous). The player's treasury (the cut) can be **spent into
  people**: wages/gifts that reduce desperation and count toward earn-goals
  (P3). Farms versus wonders: feeding everyone is the farm; Rena's freedom
  is the pyramid.
- **Shaping**: which jobs you give which people decides which marks and
  traits they accumulate — the guild's character is the sum of what you made
  its people do.

*Implemented (P1):* wealth is a per-character component, earned from shares,
displayed on every sheet; the hunger rule's economics drive desperation
exactly as in v1. Treasury spending is P3.

## 12. Presentation (DECIDED sequencing — the owner's flag)

v2 asks the player to track much more, and the defense is layered:

1. **Caps at the source** (§3) — the sheet is bounded by design.
2. **Verdict + reasons-as-words** (§6) — the surface shows judgments and
   causes, not sums. This is display-ladder rung 3, reached by mechanics.
3. **Sequencing (DECIDED)**: P1 ships v2 mechanics under *interim
   presentation* — existing UI patterns extended without craft (trait chips,
   mark lines; the goal track when P3 lands), **still bound by the
   readability floors and the signifier table** (ugly is acceptable;
   unreadable is a regression against shipped assertions). One thing was NOT
   deferrable and shipped with P1: **the verdict-and-reasons line** — it is
   how v2 is playtestable at all. Immediately after the owner's first
   playtest, a **dedicated UI/UX design session** (working agreement 8:
   mockup-first) designs the real presentation, briefed by the owner's "what
   did I reach for and not find" notes. Polish implementation follows it.

Everything below carries from v1 unchanged in force:

**Presentation is owned by `games/giri/UI.md`** — screens, signifiers,
layout, readability floors, the display ladder, and the screenshot process.
What stays here is only what binds the UI to the game: what is previewed,
invariant 2's inspectability, the asset policy and curation model, and the
text constraints.

**Text: the engine's built-in `ctx.text`** — the embedded 5×7 monospace
atlas (printable ASCII + fallback box, explicit `\n`). ASCII-only names and
copy, monospace, no engine wrapping. giri remains the likeliest first
customer for the TTF menu item; that revisit is a menu pull and an ADR
(PROPOSED).

**Assets: curated or generated, never downloaded.** The curation model in
full: role-named lowercase `snake_case` files, a committed import script,
`assets/CREDITS.md` naming source and license per file, license check
against repository visibility before any purchased asset is committed, PNGs
at or under 2048 per axis. Twelve of thirteen slots are a curated subset of
the owner's Kenney packs (2026-08-23); the eye is generated
(`art/make_art.py`); contact sheets are never committed. The manifest and
tooling live with giri until a second game wants them.

**The resolution report is the story surface.** Every consequence is
narrated mechanically, naming the rule inputs, in ASCII the atlas can draw:

```text
Bob killed Steve - desperation 8 >= 6, share 2->4, regard 0 < 2
Bob is marked comrade-killer - a witnessed kill is public
```

Flavor text can layer over it later; the arithmetic stays reachable.

**Willingness is previewed; betrayal is foreshadowed.** The preview shows
each character's verdict and leading reason before commitment because
refusal is *feedback* the player acts on (§7). *Implemented (P2):* the
ladder era replaced P1's no-preview stance with §7a's bands — the **party
band chip**, visible before SEND, derived from the same pressures the rolls
consume. No percentages on the surface; the causes are inspectable
(pressures one step deep, constants in the drawer). Under the deterministic
variant the chip does not draw: v1's the-player-does-the-arithmetic stance
is part of what §8e preserves. The seed and variant id ride every stamp and
recording (§8d) — a repro link carries constants, variant and seed.

## 12a. The scaling contract (DECIDED 2026-08-23)

The game view scales uniformly with the window — aspect preserved,
letterboxed, symmetric in both axes — down to a minimum scale.
Vertical-only or horizontal-only distortion is a defect. UI.md §6 carries
the reference resolution; `src/scaling.rs` refits the camera every frame and
`src/floors.rs` asserts the contract's four claims at four surfaces.

## 13. ECS representation (DECIDED 2026-08-22, extended P1)

Characters are entities; per-character state is components (desperation,
source, wealth, traits, marks, clean-job count). **Regard edges are
entities** — `RegardEdge { from, to, value }` — the clean ECS answer for
sparse directed relations; queries use the read-pass/write-pass pattern
(ADR-0013). Game flow (beat index, phase) is a resource holding an explicit
state machine. Facade-only, per ADR-0038 — nothing here needs engine
internals or new engine features.

## 14. Verification and tuning under seeds

**Now (P1, deterministic):** each beat is scripted end-to-end via
`InputScript`; assertions cover world state (deaths, marks, regard edges,
verdicts, margins, reasons, desperation and wealth trajectories, clean-job
counts) and the null-backend transcript (sheets rendered, report shown). The
tutorial is the test suite; the beats are the tuning constants' regression
harness; the mutation round perturbs every constant and demands a beat or
contract notice. **The reasons-as-words surface is itself verifiable**: the
transcript asserts that every rendered verdict carries at least one reason,
and the floors apply to every new chip and line.

**Under seeds — *Implemented (P2)*, specified in §8f:** scenario assertions
at fixed seed (two lists per beat, one per variant), distribution sweeps in
bands with the murder floor exact, the mutation round extended over the new
constants and cutoffs, the drawer covering the whole set, and repro links
carrying constants, variant and seed (§8d). A CUTTHROAT world is now a
probability regime, as promised.

## 15. Phases (each phase = one handoff, one session)

- **P1 — People** *(this build)*: traits (data vocabulary + function
  modifiers), marks replacing infamy, explicit wealth, willingness v2 with
  verdict + margin + reasons-as-words (interim rendering), door rule intact.
  Deterministic betrayal temporarily retained. Beats re-authored minimally
  to stay green.
- **P2 — Risk** *(this build)*: seeded RNG (engine `Rng`, §8d); strain
  (§7a); the betrayal ladder with mark/edge writes (§8, §8c);
  foreshadowing bands and the party band chip (§7a, §12); distribution
  sweeps in verify (§8f); the drawer covers the new constants; the
  deterministic model preserved as the tier-2 variant `deterministic`
  (§8e). Biography lines wait for P3's sheet work.
- **P3 — Wants and the run**: goals (templates, progress, completion
  mutations, curdling, retirement/legacy, eagerness); the open run
  (generated stream with goal hooks, recruitment, roster > work; treasury
  spending). Beats 1–4 finalized as seeded tutorial scenarios.
- **UI design session** after the P2-or-P3 playtest (owner's call on
  timing), then the presentation implementation session it specifies.

Each phase lands green (floors, verify, screenshots-viewed) and playable;
the owner playtests between phases and redirects.

## 16. Scope fences — what this phase does NOT do

No traditional stats · randomness through the engine `Rng` only, read at
resolution only (§8d) · no hidden information · no goals, eagerness,
recruitment, treasury spending, or open run yet (P3) · no resolution
failure beyond §8c's desertion re-evaluation · no audio, TTF, particles,
gamepads · no downloaded art · no generated flavor text beyond mechanical
narration · no player-reputation system (guild marks are a reserved hook).

## 17. Open questions

- Run end-state and loss condition (§10).
- Guild marks' sharpness — when does the player's own conduct gate content,
  and do characters hold regard toward the player (the complicity /
  legible-actor question, now with a mechanism ready for it).
- Hidden/discovered traits — reserved variant experiments (DECIDED to
  reserve; tier-2/3 per §8b when tried).
- Mark decay for minor marks; multiple goals per character (post-v2 — the
  one-goal cap is deliberate for now).
- The overworld map (UI.md's ideal quest screen) — the goal-hooked stream is
  its content model; the camera work waits for the UI era.
- Grief (v1's open question, still liked, still deferred).
- Where heuristic play begins: located from playtesting (§8a's
  instrumentation), not a priori.

## 18. Implementation notes — P1, the People slice (2026-08-26)

Everything here is a place where implementing this document decided
something it left open, or moved a number the tutorial asserted. The changes
are inline above; this is the index.

- **§4**: nine traits ship, not twelve — the register's eight plus
  *upright*. The pot-affinity reading of *greedy* ("pot terms weigh double")
  became "the pot term exists through traits" because P1's base pot weight
  is zero (§6's first implementation note); when a base pull is tuned in,
  greedy doubling it becomes literal again.
- **§5**: mark base reactions are per *tone* (`mark_dark`, `mark_light`),
  with per-kind character coming from the trait×mark table — two constants
  the drawer can sweep instead of seven, and the table stays the place a
  mark's personality lives. *Kept-the-line* and the remaining dark marks
  have no P1 writer; they exist as authored backstory and table columns so
  the ladder writes into a vocabulary that already reacts.
- **§6**: the verdict boundary is `reluctant_below` (shipped 2), so a margin
  of 0 — Tim's met price — is *reluctant*, which is beat 4's new texture.
  The reason vocabulary's templates are shorter than this document's
  examples ("needs the money") because the party card wraps at sixteen
  columns and the floors bind (UI.md §7).
- **Beats, every assertion that moved**: Steve's willingness 1 → 5 (the pot
  now pulls him — greedy, share 4); Bob's beat-2 willingness 8 → 10 (share
  2); Alex's authored desperation 2 → 3 and his willingness stays 2 (the
  comrade-killer reaction −1 replaced the v1 gap −1... at his new need);
  Alex's end desperation is 0 in both versions; Tim's refusals stay −2 and
  his beat-4 price stays 0, now produced by upright × comrade-killer (−3)
  against the same desperations; Bob's beat-4 willingness 4 → 7 (pot 3);
  every `Infamy` assertion became a mark assertion (beat 2: Bob has
  comrade-killer; beat 4: Bob reaches *reliable* at his second clean job,
  which is new coverage, not migration). The kill line's narration is
  byte-identical to v1's.
- **Clean-job continuity is authored**: beat 4 starts Bob at one clean job —
  beat 3's — because each beat's roster is authored state; the counter is
  data like everything else on the sheet.
- **The eye signifier** now means reputation marks (UI.md §2 updated);
  trait chips borrow category icons from the existing five (interim, the UI
  session designs real ones).

## 19. Implementation notes — P2, the Risk slice (2026-08-26)

Everything here is a place where implementing the P2 specification decided
something it left open, or bent a stated shape and says so. The changes are
inline above; this is the index, and the PR that landed the slice lists the
deviations one line each.

- **§8: the murder floor is not a constant *at or above* the band boundary
  — it is the boundary.** The specification asked for `murder_floor >= the
  powder-keg cutoff`; the build makes the murder rung read `powder_keg_at`
  itself. One constant satisfies the `>=` trivially and is the strongest
  one-source form: the warning and the gate cannot be two numbers.
- **§8: `occurrence_calm` was added** — pressure the occurrence roll
  forgives. The specification's shape (roll once against pressure) made a
  genuinely calm party betray in a quarter of runs at any playable die;
  the grace term is what makes "a calm party betrays rarely" a true band.
  It is a named, drawer-tunable constant like the rest.
- **§8: rung floors, base weights, and the trait severity and pressure
  biases are data tables** (`ladder::RUNGS`, `ladder::SEVERITIES`,
  `pressure::PRESSURES`), not `Tuning` fields — the same species as the
  trait×mark table, and like it, tuning content rather than drawer rows.
  The scalar model constants (strain, hunger, opportunity, cutoffs, die,
  grace, sabotage fraction) are all in the drawer. The mutation round
  covers the fields; the tables are covered by the fixed-seed beats and
  the battery's hard-number checks.
- **§7a: opportunity is the betrayal gain** — the gold a member would gain
  if one fewer split the pot — rather than separate pot-size and
  party-size terms: "pot size relative to share, and party size, in one
  number", and it is the same arithmetic the v1 rule's `shareGain` reads.
- **§8c: a quest's completion counts the murdered.** The desertion
  re-evaluation counts `party - deserters` against headcount and
  predicates: the murdered did the work before they died, which is v1's
  own vault semantics (two went down, one came back, the job cleared).
- **§8c: the skim's arithmetic, made exact**: each skimmer takes one
  post-cut share off the top (`share_each` at survivor count), and the
  remaining pool splits among all paid survivors. With no skimmers this is
  v1's split to the gold.
- **§6/§8: the bond-suppression clause** landed as: a member bonded
  (`regard >= K_loyal`) toward *everybody still present* never rolls.
  Murder's per-target protection is unchanged inside target selection.
- **§8d: per-scenario seeding is a re-seed of the engine `Rng` at the beat
  boundary.** `GameConfig::seed` carries the scenario seed in verify's
  one-sim-per-beat runs; a windowed session spans scenarios, so the beat
  boundary re-fixes the seed (authored, or the `?seed=` override) — which
  is also what makes an APPLY's restart replay exactly.
- **§8e: the variant picker sits in the tuning drawer** and switching
  restarts the chain immediately (see §8b's implementation note).
- **The beats each carry two assertion lists and a fixed seed**; the
  ladder lists pin every pressure and band by hand-computed number, which
  is how the mutation round sees the pressure constants. Beat 3's ladder
  story is a skim (the common rung, taught early); beat 4's is a powder-keg
  warning that does not come true — a probability, not a promise.
- **The tutorial copy was rewritten stranger-facing** (the P1-playtest
  warm-up): dilemmas now say what to click and what to read, and every
  trait row carries a one-line behavioral description shown on chip hover
  (UI.md §14). Copy only; no assertion moved except the strings themselves.
