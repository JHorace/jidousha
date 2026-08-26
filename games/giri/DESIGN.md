# giri — design (prototype #1)

Proposed home: `games/giri/DESIGN.md` — the game's design doc travels with the
game (ADR-0038: games are workspace members; this file is game-side, not
engine-side, so `docs/internal/` shape rules do not apply to it).

Drafted in a design session 2026-08-22. Items marked **DECIDED** were settled
with the owner in that session; items marked **PROPOSED** are this document's
synthesis awaiting owner sign-off; **OPEN** items are future design work.

義理 (giri): duty, obligation, the web of what people owe each other.
ぎりぎり (girigiri): barely scraping by.

---

## 1. Concept

An auto-battler where the pieces have interests. The player sees a dungeon's
parameters in advance, assembles a party from a roster, and resolution is
automatic — no attack/defend verbs. The inversion: roster members are not
inert units. They consent, refuse, betray, bond, and remember. The player's
verbs are entirely social — selection and juxtaposition under pressure.

Second inversion, protected deliberately: **the player's profit and the
characters' welfare pull apart.** The tutorial's second beat teaches the
player to profit from a death they could foresee. The player is complicit,
and the game does not soften this.

**The hypothesis prototype #1 tests (DECIDED):** inter-character dynamics
*alone* can carry play. No traditional combat stats, no content breadth —
if hand-authored five-character dilemmas are interesting, the rules are
worth scaling into a simulation; if not, content won't save them.

## 2. Design invariants (DECIDED)

1. **Social dynamics only.** No strength/int-style stats in v1. Relational
   attributes (compatibility predicates like "godly + priest") are the same
   species as infamy-gating and are the natural *second* content axis —
   still social, still not combat math.
2. **The game never lies and never hides.** All stats and edges are numeric
   and inspectable. Difficulty comes from combinatorics, not concealment —
   chess, not poker. Early beats are exactly computable (that is what makes
   them teachable); the intended endgame is that roster × edges × dungeon
   stream outgrows exhaustive evaluation and players shift to heuristics and
   long-term strategy. That shift is a *content threshold, not a systems
   change* — hiding information later would be a UI decision, and this
   invariant says: don't. Inspectability is sacred.
3. **Scope: authored puzzle chain.** Hand-built beats in the tutorial's
   style, each a designed dilemma, each introducing one concept. Sim/endless
   modes come later, on proven rules.
4. **Resolution is pressure application, not combat simulation.** A dungeon
   exists to force the social rules to fire. Deepening resolution later
   means more moments where the decision function runs — never more stat
   math.
5. **Determinism, fully.** v1 outcome is a pure function of (authored beat
   state, player assignments). **No randomness at all in v1** — not even
   seeded. The puzzle chain must be exactly reproducible and exactly
   assertable. Seeded RNG (the engine's `Rng` resource) enters, if ever,
   with the sim phase, and replay-safely.

## 3. The social model (PROPOSED)

### 3.1 State

Per-character scalars:

- **desperation** — need. Rises when a character fails to profit in a round;
  falls when they profit. The willingness override and the betrayal motive.
- **infamy** — *public* knowledge: the global projection of witnessed acts.
  Everyone sees it; it feeds everyone's evaluations of this character.
- **wealth** — what profit accumulates into; the input that drives
  desperation down. (May stay implicit in v1 — desperation adjustments can
  encode it — but naming it now keeps the economy extensible.)

Between characters, **directed personal edges**:

- **regard(A→B)** — one signed number per ordered pair (sparse; absent =
  zero). Positive is a bond, negative is a grudge. Directed and asymmetric
  on purpose: "Tim trusts Alex more than Alex trusts Tim" is where stories
  live.

The infamy/regard split is the load-bearing distinction: **infamy is what
strangers know; regard is what *this* character knows.** Bob's infamy gates
him with everyone; Alex's regard for Bob overrides it for Alex alone.

### 3.2 One decision function, three firing moments

Refusal, betrayal, and bonding are one computation — a character evaluating
"what do I do, given my needs and my relations to these people?" — fired at
three moments:

**Willingness** (party assembly): character `c` asked to join party `P`
computes

```
willingness(c, P) = desperation(c)
                  + Σ_{m ∈ P} regard(c→m)
                  − Σ_{m ∈ P} incompat(c, m)
joins iff willingness ≥ 0
incompat(c, m) = K_inf × max(0, infamy(m) − infamy(c))
```

Tim refuses Bob (infamy gap, no desperation to override it). Alex accepts
(no gap — he has infamy too). Tim later accepts (desperation rose). "Everyone
has a price" is a theorem of this function, not a scripted event.

**Joining is gated at the door, in both directions (DECIDED, 2026-08-23).** A
character `c` may be added to party `P` iff:

1. `willingness(c, P ∪ {c}) ≥ 0` — the newcomer consents, and
2. for every incumbent `m ∈ P`: `willingness(m, P ∪ {c}) ≥ 0` — no incumbent's
   willingness would go negative. An incumbent whose willingness would go
   negative **blocks** the arrival; the UI names the blocker and shows their
   arithmetic. (Tim in the party blocks Bob for the same numbers by which Bob
   in the party makes Tim refuse — the rule is order-symmetric about who is at
   the door.)

**Consent is evaluated at the door only.** Once a member is in, they stay
until the player removes them or the party is sent — later arrivals cannot be
admitted over a veto (rule 2), and later *departures* do not trigger
re-evaluation, even though removing a bonded partner can push a remaining
member's willingness negative. The alternative — members walking out when the
party changes under them — was considered and rejected (owner, 2026-08-23):
blocking is more legible, and party state staying monotonic under the player's
own actions is worth more than the extra drama.

*Implementation note (2026-08-23):* the rule's second half decides something
the first half does not state. Because consent is not re-evaluated, **the send
gate does not check willingness either** — it checks headcount and the
composition predicate and nothing else. Gating the send on "every member is
still willing" would leave a player with a party they assembled legally, cannot
send, and can only fix by removing somebody, which is the re-evaluation this
rule declines to do arriving by the back door. The member's own card still
states their current sum, in ember; the game does not hide it, it just does not
ask them again.

**Betrayal** (resolution): the economy is the motive engine (§4). After
success, each member evaluates, against each partymate:

```
betray(c, t) iff desperation(c) ≥ K_kill
           and shareGain(c | t dead) > 0
           and regard(c→t) < K_loyal
```

Deterministic; evaluation order is the party's roster order (stated, so it
is predictable and assertable). Bob kills Steve: desperation high, share
doubles, no regard. Bob and Alex spare each other: neither is desperate.

**Bond drift** (aftermath): shared success without betrayal raises mutual
regard between all surviving pairs. Betrayal: the betrayer's infamy rises
(public — there were witnesses, or the outcome speaks); each surviving
witness's regard toward the betrayer drops (personal grudge). If a character
had positive regard toward a victim, they acquire a grudge against the
killer, and (OPEN) possibly grief effects on desperation. Round end: every
roster member who did not profit has desperation rise; participants who
profited have it fall.

These formulas are accepted as starting points (owner, 2026-08-22). All
constants (`K_inf`, `K_kill`, `K_loyal`, drift magnitudes) are tuning values,
named in one place in the game code, with the puzzle beats as their test
suite: a constant change that breaks a beat's intended dilemma fails verify —
the beats *are* the tuning harness. Runtime tuning is a first-class
requirement; §8a owns it.

Three points the first implementation had to settle, because the formulas above
do not (prototype #1, 2026-08-22 — see §12):

- **Bond drift is per run, not per pair.** "Shared success without betrayal" is
  read as *this dungeon had no betrayal in it*: a job somebody was killed on
  leaves no bonds behind at all, not even between two survivors who had nothing
  to do with it.
- **Desperation has a floor**, `desperation_floor`, a constant like the others
  and 0 in the shipped set. Without it `desperation_fall` takes a character
  below zero the first time they profit, and a character at −2 refuses a clean
  job with nothing wrong with it. At the floor they still take clean work and
  nothing that costs them.
- **Betrayal is evaluated in roster order at both levels**: `c` walks the party
  in roster order and, for each `c`, `t` walks it in roster order too, with each
  kill taking effect immediately. A character killed before their own turn never
  evaluates.

### 3.3 Bonds, mechanically (DECIDED 2026-08-22)

A bond (positive regard) does four things — this is the mechanical
definition the concept was missing:

1. **Overrides public information**: regard enters willingness alongside
   incompat, so a strong bond outweighs an infamy gap (Alex joins Bob).
2. **Suppresses betrayal**: the `regard(c→t) < K_loyal` clause.
3. **Propagates consequences**: harm to a bonded character creates a grudge
   in the survivor toward the killer. Relationships make events *travel*.
4. **(OPEN, likely v1.1) Package deals**: above a threshold, willingness
   couples — the pair joins together or not at all. This makes bonds a
   constraint the player *plans around*, not just a buff.

## 4. Economy (DECIDED 2026-08-22 — including the direction that infamy
must not only close doors; the underworld-track mechanism below stays OPEN)

Each dungeon has a **pot**, split among surviving participants after the
player takes a stated cut. Fixed pot + division among survivors is what
makes desperate betrayal *economically rational* — no separate betrayal
mechanic exists, only arithmetic and the decision function.

The designed-dilemma knob: **a dungeon that requires N characters but pays
fewer than N worthwhile shares.** The player can read that math and send the
party anyway. (Beat design leans on this.)

Non-participants don't profit, so their desperation rises each round —
the roster decays toward willingness. Refusal is always temporary.

**OPEN — infamy needs a positive use.** If infamy only closes doors, optimal
play quarantines infamous characters and the mechanic dead-ends. Proposal to
explore: infamy-gated dungeons (an underworld track that *requires* infamy),
so infamous rosters open different doors rather than fewer. Not in the first
beats; the chain should surface the problem before the answer.

## 5. Dungeons (PROPOSED)

v1 dungeon = requirements + pot + payout rule:

- **Requirements**: headcount, plus composition predicates from the social
  vocabulary ("at least one infamous member", "no infamous members",
  "must include a bonded pair", ...). Predicates are the growth axis.
  *Theme note (owner, 2026-08-22):* predicates like "at least one infamous
  member" are accepted as gameplay-first and thematically artificial —
  requirements will likely change in later versions, and **theme is mutable
  and secondary during prototyping**. (Observed in passing: the predicate
  that feels wrong for fantasy-RPG dressing is perfectly natural for an
  organized-crime frame — a job that *needs* a known face. Theming is a
  skin to choose after the gameplay proves out, and this is a candidate.)
- **Pot and player's cut**: visible before assembly, like everything else.
- **Resolution order** (stated, deterministic): willingness checks happen at
  assembly time in the UI (refusals are *feedback*, not failures); if
  requirements are met the dungeon succeeds; betrayal evaluation; payout;
  bond drift; round-end desperation drift.
- **v1 has no resolution failure** (accepted for now, owner 2026-08-22,
  with the expectation that requirements change in future versions): an
  under-filled party cannot be sent, and a sent party succeeds. Puzzle
  purity — stakes come from *what success costs*, not from whether the dice
  come up. Failure semantics enter with the sim phase (OPEN), where they'll
  need design (death? empty-handed return with a desperation spike?).

## 6. The puzzle chain (PROPOSED structure; beats TBD with owner)

Beats 1–4 are the owner's tutorial, verbatim (Steve; Bob kills Steve; Tim
refuses / Alex joins; Tim's price is met). Then one new concept per beat,
roughly: grudge consequences (the survivor of a betrayal meets the betrayer
again) · bond formation as a plannable asset · a shares-less-than-headcount
dilemma · an infamy-gated requirement · package-deal bonds · grief ·
a closing beat that requires exploiting everything at once, ideally with a
genuinely uncomfortable optimal solution. Target ~10–15 beats.

Authoring format: each beat = (initial roster state, dungeon(s), the
intended dilemma stated in a sentence, expected-outcome assertions). The
fourth field is the verify scenario (§8). Win = complete the chain.

*Implementation (prototype #1):* the beats live in `src/beats.rs` as a `CHAIN`
of `BeatSpec` values and are read by no code that names a beat number — beats 5+
are added there and nowhere else. The verify scenario needs a fifth field beside
the assertions, `send`: the party the scripted run assembles. Nothing in the
game reads it and a player may send anything the gate allows; it is the
scenario's "and then the player does this", which the assertions alone cannot
say. Beats 1–4 are the owner's four, with rosters, pots and cuts authored to the
formulas in §3.2 — beat 2's numbers are the ones §7's example line quotes.

## 7. Presentation (DECIDED unless marked)

2D, sprites and quads, pointer input — nothing the engine lacks. No audio,
no particles, no gamepads (standing policy: nothing pulled until needed).
A **post-v1 polish pass is anticipated as an option** (owner, 2026-08-22):
once the gameplay prototype is done, a deliberate polish step may pull
menu items like particles or audio — that pass, if taken, *is* the "need"
the standing policy asks for, and each pull gets its normal ADR.

**Text: the engine's built-in `ctx.text`** — the embedded 5×7 monospace
atlas (printable ASCII + fallback box, `width_of`, explicit `\n`; renderer.md
§6). No font asset exists or is created. Constraints accepted for v1:
ASCII-only names and copy, monospace, no wrapping (explicit line breaks).
giri remains the likeliest first customer for the TTF menu item — the pull
trigger is outgrowing 5×7 monospace ASCII, and that revisit is a menu pull
and an ADR (PROPOSED).

**Assets: curated or generated, never downloaded.** Sprites are generated by a
committed script (original art, deterministic, reviewable) or owner-provided —
**never downloaded from third-party sources by an agent** (provenance and
licensing; owner approval plus a recorded license is the only exception). The
curation model: role-named lowercase `snake_case` files, an import script
committed beside them, a `CREDITS.md` naming source and license per file, and a
license check against the repository's visibility before any purchased asset is
committed. Individual PNGs stay at or under 2048 on each axis.

*Landed 2026-08-23:* the library, generated by `art/make_art.py` from the grids
in `art/sprite_defs.py` — four portraits, four dungeon icons, five stat and
event icons. It was built as a placeholder set and the owner kept it after
reviewing the captures.

*Curated 2026-08-23, the same day:* the owner supplied seven Kenney packs and
**twelve of the thirteen slots are now a curated subset of them (DECIDED)**. The
swap cost no code: the role names held, and the only changes were sizes. What
the curation model bought is visible in that fact.

The packs are CC0 1.0 and live on the owner's machine; they have never been in
this repository. Kenney *requests* that whole packs not be redistributed, which
is not a licence term but is honoured by construction: only individually chosen,
role-named sprites are committed, and the **contact sheets are never committed**,
because a whole pack rearranged into sheets is still the whole pack. The tooling
that made choosing possible — `art/contact_sheet.py` to see a pack,
`art/role_sheet.py` to see one slot's shortlist, `art/kenney-manifest.json` to
record what each region is and which one the owner picked, `art/extract.py` to
cut the picks — writes everything but the manifest into `target/`.

Two things this session fixed in place, both worth keeping: classification is
**on demand** (only regions a slot actually needed are labelled; the rest are
recorded as unclassified with their extent, because labelling a whole pack is
hours spent on tags nobody will query), and the manifest is metadata rather than
art, so committing it is what makes looking at a pack a one-time cost. It lives
with giri until a second game wants it (second-consumer rule).

The generated path stays whole — `art/make_art.py` and the grids are still here,
still run, and still fill the one slot no pack could: **no eye glyph exists in
any of the seven packs**, so infamy keeps its generated icon rather than taking
a substitute, which would have meant editing UI.md §2's signifier table. A
stable signifier is not something an import gets to change.

**Presentation is owned by `games/giri/UI.md`** (2026-08-23) — screens,
signifiers, layout, readability floors, the display ladder, and the screenshot
process all live there, and a change to any of them is a UI.md edit. What stays
here is only what binds the UI to the game: what is previewed (the paragraph
below, unchanged), invariant 2's inspectability, the asset policy and the
curation model above, and the text constraints.

**The resolution report is the story surface.** Every consequence is
narrated mechanically, naming the rule inputs — as the game actually prints it,
in the ASCII this section mandates:

```text
Bob killed Steve - desperation 8 >= 6, share 2->4, regard 0 < 2
```

*(Corrected 2026-08-22: this line was drafted as "Bob killed Steve — desperation
8 ≥ 6, share 2→4, no regard", whose em dash, ≥ and → are three characters the
5×7 atlas draws as boxes. The engine's font covers space through `~` and nothing
else, and no assertion over drawn quads can see the difference — the string is
the only instrument. Every line the game draws is ASCII, and `--verify` checks
each one.)*

In v1 this is debugging output promoted to UI; it is also how players learn the
heuristics that invariant 2's endgame depends on. Flavor text can layer over it
later; the arithmetic stays reachable (the game never hides).

**Willingness is previewed; betrayal is not.** The preview shows each selected
character's sum before commitment because refusal is *feedback* the player acts
on (§5). Betrayal has no preview in v1: the player is shown every input to it —
desperation and regard on the sheets, the pot and the cut on the job, and the
constants themselves in a panel on screen — and does the arithmetic. That is
what makes beat 2 a death the player *could foresee* rather than one the game
warned them about, and it is inspectability rather than concealment (invariant
2: nothing is hidden, and the game does not do the sum for you). A betrayal
preview is a UI decision available any time, and would soften the second
inversion.

## 7a. The scaling contract (DECIDED 2026-08-23)

**Scaling contract:** the game view scales uniformly with the window — aspect
preserved, letterboxed, symmetric in both axes — down to a minimum scale.
Vertical-only or horizontal-only distortion is a defect. UI.md §6 carries the
reference resolution and the known browser defect to resolve.

*Resolved 2026-08-23:* the defect was game-side. `Camera::height` is the game's
and the driver stamps only `viewport`, so a game that names one height and
leaves it there scales uniformly on a vertical shrink and not at all on a
horizontal one. `src/scaling.rs` refits the height every frame from the
viewport the driver last stamped, and `src/floors.rs` asserts the four claims
the contract is — uniform, symmetric, whole, and clamped at the floor — at four
surfaces.

## 8. Verification (DECIDED in approach)

Each beat is scripted end-to-end via `InputScript` in the prototype_kit
pattern: drive assembly through the per-tick snapshot, assert on world state
(deaths, infamy, regard edges, refusals, desperation trajectories) and on
the null-backend transcript (sheets rendered, report shown). The tutorial
is the test suite; the beats are the tuning constants' regression harness.
Mutation round per practices §5.2 — break constants on purpose and check
the beats notice. `tools/verify giri` needs no registration (ADR-0038).

## 8a. Tuning and playtesting (DECIDED 2026-08-24)

*The mechanism below was PROPOSED when this section was drafted and is DECIDED
as of 2026-08-24: the beat-boundary rule, the stamping, and the live menu are
the ones built. `UI.md` §9a owns their presentation and §12 records what
building them corrected.*

Balance questions (§3.2's constants, beat difficulty, the heuristic-onset
point) are answered by playtesting, through two channels:

**Agent self-playtest.** As with the Pong runs that tuned their own
difficulty, agent sessions play giri via `InputScript` and sweep constants
against the beats. The verify report includes the constants in effect for
the run, so a tuning sweep is scriptable: same beat, varied weights,
machine-readable outcomes.

**Human playtest with a live tuning menu.** A debug UI exposes the §3.2
constants for on-the-fly adjustment — the owner demoing giri to another
person adjusts weights without switching builds. Built with the same quads,
sprite font, and pointer input as the rest of the game; no engine pull.

**The determinism interaction (the part that needs stating):** tuning
constants are *simulation inputs* — replay state is a pure function of
(beat state, assignments, constants). A constant silently changed mid-run
would make the recording a lie. v1's resolution: **the tuning menu applies
changes at beat boundaries — adjusting a constant restarts the current
beat with the new values.** Beats are short puzzles, so this costs seconds
and is actively useful (same beat, A/B'd across weights). The constants in
effect are stamped into every recording and verify report, so any run
remains exactly reproducible. This keeps the engine's replay contract
untouched and needs no engine changes. If the sim phase ever wants
*mid-run* tuning, constant-changes would have to enter the recorded stream
like input does — that is an engine conversation (new recorded channel,
own ADR) and is explicitly out of scope for v1.

Playtesting instrumentation should also serve the heuristic-onset question
(§11): logging what players inspect and how long assembly takes per beat is
cheap, and is the only honest way to locate where evaluation gives way to
heuristics — it varies per player; we look for the general point.

## 8b. Variants — how incompatible mechanics coexist (DECIDED 2026-08-23)

Iteration on giri will produce mechanics that cannot all be true at once. Which
mechanism carries a variant follows from **how deep the divergence goes**, in
three tiers.

**Tier 1 — different numbers: not a variant.** Same rules, different constants
is a *tuning preset* — a named constants set, handled entirely by the §8a
machinery (one constants module, stamped into recordings and reports, adjustable
at beat boundaries). Never a flag, never a binary.

**Tier 2 — different rules, same shape: one binary, variants as data.** A
variant that swaps *which rule* fires at the decision function's moments — a
different betrayal condition, package-deal bonds on, another bond-drift law —
while keeping the beat format, the screen flow and the state shape, lives in the
mainline crate as a `Variant` chosen at chain start.

Structural constraint, enforced by review: **variant selection happens in
exactly one module**, which assembles the rule set at startup — never inline
`if variant` branches through systems. One file states what every variant is,
and the decision function stays one function receiving its rules. That is the
same discipline §3.2 already imposes for the same reason: the moment the rule
set is readable in one place, a variant is a thing you can reason about; spread
across call sites it is a thing you can only test.

The variant id is a **simulation input** exactly as the tuning constants are:
part of replay identity, stamped into every recording and verify report. Verify
runs beats × variants the way §8a's mutation round sweeps constants. On the web,
the variant picker sits at chain start, beside where the tuning menu lives.

**Tier 3 — different loop, screens, or state shape: fork the crate.** When flags
would lie about the divergence, the variant becomes `games/giri-<name>/` — a
sibling workspace member, which ADR-0038 makes nearly free: it builds, verifies
and publishes automatically, and gets its own playtest URL. Two disciplines keep
forks honest:

- Every fork carries a `VARIANT.md` stating the hypothesis it exists to test and
  what "decided" looks like. A fork is an experiment with an exit, not a second
  product.
- Forks are short-lived and few (at most ~2 alive). When decided, the winning
  mechanics merge into mainline giri and the losing fork moves to `attic/` — the
  topology's built-in graveyard — with `VARIANT.md` updated to record the
  verdict.

**Deliberately deferred: a shared gameplay library.** Tier-3 forks duplicate code
against mainline, and the tempting fix — a `games/giri-core` library crate — is a
real topology question ADR-0038 has no story for (games depend only on the
facade; a non-runnable directory under `games/` breaks the
everything-under-`games`-is-a-game property). Fork short-livedness bounds the
duplication. If a long-lived multi-variant future arrives, that is the driving
use case for a games-shared-library ADR, and it is taken then, not now — the
second-consumer rule, applied to infrastructure.

**Decision procedure, compressed:** only numbers → tier 1. Expressible as
choosing rule implementations at startup, without touching beats, screens or
state shape → tier 2. Anything deeper → tier 3.

## 9. ECS representation (DECIDED 2026-08-22)

Characters are entities; scalar stats are components. **Regard edges are
entities** — `RegardEdge { from: Entity, to: Entity, value }` — the clean
ECS answer for sparse directed relations; queries over edges use the
read-pass/write-pass pattern (ADR-0013). Game flow (beat index, phase) is a
resource holding an explicit state machine. Facade-only, per ADR-0038 —
nothing here needs engine internals or new engine features.

## 10. Scope fences — what giri v1 does NOT do

No traditional stats · no randomness · no hidden information · no sim or
endless mode · no resolution failure · no audio, TTF, particles, gamepads ·
no downloaded art ·
no generated flavor text beyond mechanical narration · no player-reputation
system (characters do not yet remember what *the player* did — OPEN below).

## 11. Open questions (future design sessions)

- Infamy's positive use (§4) — likely the first post-chain design question.
- Failure semantics for the sim phase (§5).
- Package-deal bonds (§3.3.4) — v1.1 candidate.
- Grief: what a bonded survivor's desperation does, and whether grief and
  greed should be distinguishable states. (Owner: liked, explicitly deferred
  to later — do not design it into v1.)
- **The player is a guild-master** (owner, 2026-08-22) — but theming is
  secondary during prototyping and this frames rather than constrains. The
  live long-term question underneath it stands: does the player become a
  *legible actor* — do characters hold regard toward the player, remember
  being sent to die, refuse the player? That question is the heart of the
  complicity theme, for the sim phase.
- Where heuristic play begins: acknowledged unanswerable a priori — it
  varies per player. The goal is to identify a *general* onset point from
  playtesting (see §8a instrumentation), and then to decide whether the
  chain's difficulty curve should reach it by the final beats or only in
  the sim.

## 12. Implementation notes — prototype #1 (2026-08-22)

Written when the first slice landed (`games/giri/`, the social model, beats
1–4). Everything here is a place where implementing the document changed it;
the changes are inline above and this is the index.

- **§3.2** now states the desperation floor, the per-run reading of bond drift,
  and the roster order at both levels of the betrayal loop. All three were
  ambiguities rather than disagreements — the formulas do not decide them and an
  implementation must.
- **§6** now names the fifth beat field the verify scenario needs (`send`) and
  says where the chain lives.
- **§7**'s example narration line was not ASCII and the same section requires
  ASCII; corrected, with the reason.
- **§7** now says explicitly that betrayal is not previewed, which the section
  left open and beat 2 depends on.
- **§3.1**'s "wealth may stay implicit in v1" was taken the other way: wealth is
  a component, it accumulates shares, and it is on every sheet. Implicit wealth
  would have made the payout arithmetic unreadable to a player, and invariant 2
  says a number that decides an outcome is a number on screen.
- **§8a**'s tuning menu is not built (next session), but its two prerequisites
  are: every constant is one `Tuning` resource, and the set in effect is stamped
  into the UI and into every verify report. The menu bolts onto that, and the
  beat-boundary rule §8a settles is already how the game restarts a beat.

## 13. Implementation notes — the tuning drawer (2026-08-24)

§8a's menu is built (`src/tuning.rs`), and §8b's tier 1 with it
(`src/presets.rs`). Nothing in this document changed except §8a's status; what
follows is what building it settled that the document left open.

- **A preset is a `Tuning` with a name on it, and that is the whole of tier 1.**
  §8b says a tuning preset is "never a flag, never a binary"; the shape that
  makes that true is a table the drawer *walks*, so a preset added is a row added
  and no code anywhere names one. `DEFAULT` is `Tuning::SHIPPED` by reference,
  because two spellings of the shipped set is one spelling that can go stale.
- **The stepper rows are walked off the constants module too**, so a constant
  added to `Tuning` grows a row in the drawer, a key in the compact stamp, and a
  key `?constants=` accepts, without any of the three being edited. Beats 5–15
  will add beats and may add constants; this is the half that does not need a
  second visit when they do.
- **"Apply at a beat boundary" is one action, not two.** Swapping the resource
  and restarting the beat happen in one function and neither is reachable without
  the other, because a swap without a restart is exactly the recording that lies
  which §8a's resolution exists to prevent. `src/restart.rs` asserts the claim
  the resolution rests on: a beat played out after an apply is byte-identical to
  the same beat played from the start at those constants — and, so the comparison
  is an instrument rather than a tautology, that the same beat resolves
  *differently* at the shipped set.
- **The pending set is UI state and the active set is simulation state**, and the
  drawer reads the active one from the world's `Tuning` resource rather than a
  copy beside it — so the numbers it stamps cannot drift from the numbers the
  decision function reads. That is the one property the whole feature stands on.
- **§11's instrumentation is two counters and a log line** (`src/onset.rs`):
  assembly duration in ticks from the first roster interaction to SEND, and the
  number of arrivals of the pointer on a sheet. Arrivals rather than time spent,
  because every sheet is always on screen (invariant 2) and there is no inspect
  verb to count. Ticks rather than a clock, per invariant 5.
- **Not a deviation, recorded because it is a choice the document leaves open:**
  the drawer's steppers and its `?constants=` links accept 0 to 12, which is a
  bound on the *tuning surface* rather than on the type — the mutation round
  still moves a constant to 99 and the floor to −99, and has to, because a
  perturbation has to be one nobody would plausibly author.
- **Not deviations, recorded because they are choices the document leaves open:**
  the chain loops back to beat 1 from the completion screen rather than ending
  (a playtest convenience); a beat's dungeons are a list with a selectable row
  even though every beat here offers one, so a multi-dungeon beat needs no
  systems work; and the tutorial's rosters and pots are authored numbers, chosen
  to make each beat exactly computable from the sheets.
