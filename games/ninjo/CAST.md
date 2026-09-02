# ninjo - the cast (founding band, vocabulary, first templates)

The content pass GDD s10 left open: who lives in the camp, what the
trait words are, and the first five petition templates. The roster and
the vocabulary become data (`people.rs`, `traits.rs`, the template
table) with the **wave-1.1** session, and this document stays as the
living cast bible - rewritten state, per the living-docs convention.
Where a number appears it is a drawer starting value, not a decision.

**Sections 1 to 8 landed with wave 1.1**, as the plan below says: the trait
vocabulary is `src/traits.rs`, the ten sheets and their homes are
`src/people.rs`, the task types are on every quest row, the relationship
presets are the `bonds_preset` drawer row, and the coverage matrix and the
no-dead-motivator rule are assertions the registry runs. What 1.1 did *not*
build is s6's template table — petitions are wave 1.3 — so the no-dead
rule is asserted against a declared list of the five motivators s6 covers
(`traits::TEMPLATED_MOTIVATORS`), and 1.3 repoints that constant at the real
table without the assertion changing. The `*Implemented*` notes below are
per-section.

**Landed early, on 2026-09-02, by the cast-art session** rather than
with wave 1.1 as the plan above says. That session picked and imported
the fifteen art roles s9 asks for, and s9 is its record; a document
whose s9 is the landed state cannot be a document that does not exist
yet, and three committed files (`src/sprites.rs`,
`art/sprite_defs.py`, `art/picks_sheet.py`) now cite it by section. No
other section was touched: sections 1 to 8 are the handoff's text
verbatim, unbuilt, and wave 1.1 writes their `*Implemented (w1.1):*`
marks. This is the one deviation that session made from its fences,
and its reason.

**Vocabulary status: PROVISIONAL through the wave-1 close.** The
aptitude and motivator words below are the owner's leaned choice made
before the context that would test them exists (party building is
wave 4; traits become consequential across waves 1.1-1.4). They are
ids in one table with display names beside them; renaming is a data
edit. The wave-1.5 playtest carries an explicit vocabulary question
(s7). Template text and source lines refer to traits by display name
through the row, never by prose - so a rename touches one cell.

All player-facing text here is printable ASCII (the registry asserts
it).

## 1. Setting bible

**Kawaza** is a river crossing, not yet a town. A ford, a toll-house
that predates everyone, a ridge with a watchtower nobody mans, a cave
that goes deeper than anyone has bothered to find out, and an old
crypt on the far bank. The band arrived a season ago because the
crossing was unclaimed and a crossing earns. They are **a new
adventurers guild - really a band of mercenaries** - and the player is
its guildmaster: responsible for everyone, in command of no one. At
game start Kawaza is **a camp**: tents on the near bank, one fire, a
tally kept in a ledger. It becomes recognizable as a settlement only
when industry starts running; the first standing building is a beat
(GDD s1, the phasing arc).

Facts the source lines and templates lean on: the **toll-house** is
where the collector works - the loan shark of the director's canned
template; the **four authored sites** stand as landed (watchtower,
deep-cave, old-crypt, and the fourth); the camp's **fire** is Rin's and
is the ancestor of the first industry; money is gold and only gold.

The 0a build called the town by a placeholder name; the owner's name is
**Kawaza**. *Implemented (w1.1):* renamed everywhere — `grid::LOCATIONS`, the
docs, the page title, the screenshots — and the old name is grepped for and
gone from the tree, this sentence included.

## 2. Task taxonomy (four types; aptitude ids double as task ids)

| task id | what it is | where it shows up at MVP |
|---|---|---|
| `fight` | clear a site of what is in it | site jobs at the crypt and the cave |
| `labor` | camp work; the generic industry's shifts | industry slots (1.2); haul/survey site jobs |
| `scout` | travel-heavy; go and look | the far sites; the fiction asks-travel rides on |
| `craft` | mend, build | rare at camp; grows with industry; the first building |

Every authored quest/site job carries a task type from wave 1.1 on
(data on the quest row). Resolution (1.4) reads the aptitude whose id
equals the task's type; until then the landed stub ignores it and the
scorer already weighs it.

*Implemented (w1.1):* `traits::TaskType`, and `Quest::task` on every authored
row. `TaskType::aptitude()` and `TaskType::of_aptitude()` are the round trip,
and the vocabulary's validation asserts it both ways — every type has exactly
one aptitude row and every aptitude row is some type's. **The board grew from
seven jobs to twenty-four**, six a site, each site leaning toward the work its
fiction implies and carrying at least one of every type: ten people looking
for work empty a seven-job board before the first day is out, and an aptitude
with nothing to do is a chip that means nothing. Sites still run dry.

## 3. Trait vocabulary (content)

### 3.1 Aptitudes (kind `aptitude`; one per task type)

| id (= task) | chip | line (stranger-facing) | aptitude |
|---|---|---|---|
| `fight` | fighter | "stands where the trouble is" | 2 |
| `labor` | laborer | "does the long work without being asked twice" | 2 |
| `scout` | scout | "knows the way, or finds it" | 2 |
| `craft` | crafter | "fixes it, or builds the thing that replaces it" | 2 |

The placeholder rows `strong`, `deft`, `learned` retire (not on any
sheet; delete the rows - nothing else references them).

*Implemented (w1.1):* deleted, and these four are the rows. The aptitude id
**is** the task id, so `competence_at(task, traits)` reads one row rather
than summing everything the carrier can do.

### 3.2 Motivators (kind `motivator`; five, each with its template)

| id | chip | line | upkeep | pressure | favors |
|---|---|---|---|---|---|
| `indebted` | indebted | "owes somebody, and the somebody is not patient" | 5/4 | 3 | any |
| `renown` | renown | "wants a name people say" | 1/1 | 2 | fight |
| `caring` | caring | "somebody else's trouble is their trouble" | 3/2 | 2 | any |
| `restless` | restless | "wants to be somewhere else, for a while" | 1/1 | 2 | scout |
| `maker` | maker | "wants to make something that lasts" | 5/4 | 2 | craft |

**`favors` is a new field on the motivator row** (neutral: none): the
task type this want's pressure applies to, `any` meaning any paid work.
The scorer (1.1) adds `pressure` to a candidate whose task type matches
(or to every paid candidate for `any`). Needs (1.2) reads `upkeep`.
This is the row's whole mechanical surface; the *petition* half of each
motivator is s6.

*Implemented (w1.1):* `favors` is a field on the row (`Favors::None` neutral,
`Any`, or `Task(t)`), asserted neutral on every non-motivator row like every
other kind-owned field, and read by `traits::pressure_toward` — which is the
only place a want reaches the scorer, and which never looks at an id.

The placeholder rows `provider`, `ambitious`, `homesick` retire.
`caring` absorbs provider's upkeep idea (feeding somebody else costs);
`renown` replaces ambitious; `restless` replaces homesick with the
direction reversed (away, not home).

**No-dead-motivator rule** (decided): every motivator row has at least
one template in s6 whose source class is `motivator` and whose trigger
names it. Checkable as data validation - add the check.

*Implemented (w1.1):* `traits::vocabulary` asserts it against
`TEMPLATED_MOTIVATORS`, the declared list of the five s6 writes a template
for, and asserts besides that no motivator has zero pressure. Wave 1.3
replaces the constant with a walk over the real table.

### 3.3 Personalities (giri's nine, audited)

Audit question: *does it shape daily scorer choices or ask verdicts at
MVP?* Kept rows are on the founding sheets; parked rows stay in the
vocabulary (the trait x mark reaction table references them and stays
whole) but appear on no sheet until marks are common - that is the
betrayal ladder's era (asks, wave 2, and after).

| id | verdict | why |
|---|---|---|
| `greedy` | keep | pot pull is a scorer term from 1.1 |
| `loyal` | keep | bonds x2 shape whom they work beside and whom they obey |
| `proud` | keep | the ask refuser; refuses charity when gifts exist (1.3 - needs a field then; see s8) |
| `craven` | keep | danger terms x2 once fight tasks carry danger (1.4); dormant until then, harmless |
| `vengeful` | keep | grudges x2, never decay - the repeat-refusal story |
| `cold` | keep | edges x1/2 both ways - the one who cannot be bought with regard |
| `pious` | park | reacts to marks by kind; marks are rare before the ladder |
| `pragmatic` | park | prefers a known skimmer; mark-dependent |
| `upright` | park | refuses the dark-marked; mark-dependent |

No personality is added. The scorer does not exist yet; a personality
that owns a scorer field (a work/idle bias, say) is proposed when 1.1
finds the scorer wants one, not before.

*Implemented (w1.1):* none was added, and the scorer did not ask for one. The
parked three are `people::PARKED` — declared beside the roster rather than as
a field on the trait row, because being on a sheet is a casting decision and
not a property of the word — and the registry asserts they are in the
vocabulary and on nobody.

Authoring norm: two or three traits per sheet (practice, not rule; the
list cap is gone).

## 4. The founding band (ten)

Mongrel names on purpose: a band drawn from everywhere. The four
0b-landed characters survive as founders with revised sheets. `home`
tiles are the 1.1 session's to place (passable, unshared, off named
locations - the registry asserts it).

| id | name | role | traits | wallet | desp. | source line |
|---|---|---|---|---|---|---|
| `bob` | Bob | founder, fighter | fighter, greedy, indebted | 6 | 4 | owes the collector at the toll-house, who counts days |
| `steve` | Steve | founder, laborer | laborer, loyal, caring | 3 | 5 | sends half of everything to a sister whose hands gave out |
| `alex` | Alex | founder, scout | scout, cold, restless | 12 | 1 | has not slept a full month in one place since childhood |
| `tim` | Tim | founder, quartermaster (camp-follower) | laborer, proud, vengeful | 20 | 2 | keeps the tally, and is owed by half the camp |
| `rin` | Rin | cook (camp-follower) | crafter, maker, loyal | 5 | 2 | cooks for ten on a fire built for three |
| `goro` | Goro | fighter | fighter, renown, proud | 9 | 3 | left home to be talked about, and nobody is talking yet |
| `hana` | Hana | scout | scout, caring, vengeful | 7 | 2 | came for her brother; stays exactly as long as he does |
| `ludo` | Ludo | laborer, fights when asked | laborer, fighter, indebted | 2 | 4 | works off a debt that was his father's before it was his |
| `ines` | Ines | crafter | crafter, maker, craven | 10 | 2 | mends what breaks, and would rather be far from what breaks it |
| `odd` | Odd | fighter | fighter, renown, restless | 8 | 3 | took the same job as Goro twice, and only one of them got paid |

*Implemented (w1.1):* all ten, in this order, in `people::roster`. **Homes are
two rows of tents south of the road and east of the ford** — y=15 at x
12/16/20/24/28 and y=17 at x 6/10/14/18/22 — spaced so ten names, ten figures
and the town's own marker do not collide, which `floors.rs` asserts rather
than this sentence. A dispatched party leaves from the doorstep it is standing
on and comes home to it, which is why the journey minutes differ per
character.

### 4.1 Demo characters (each MVP module names the person it is proved on)

| module | demo character | why this one |
|---|---|---|
| needs (1.2) | **Steve** - the pariah-candidate | highest upkeep multiplier (caring 3/2), lowest wallet among earners, labor pays least: first to shortfall with the player idle |
| autonomy (1.1) | **Ludo** - the eager worker | indebted pressure 3, favors any, no pride: takes whatever work is open without being asked; the character the scorer is most visibly alive on |
| asks (2) | **Tim** - the proud refuser | proud refuses; vengeful turns a repeat into a grudge; and he holds the tally, so the refusal costs the player something |
| petitions (1.3) | **Goro** - the petition fountain | renown fires the proving-job template most often; Odd makes every answer to it a social problem |
| settlement (1.2) | **Rin** - the industry seed | maker; her petition asks for the first building; the camp fire becoming a kitchen is the first-building beat and the baker-dream's ancestor |
| events-director (1.5) | **Bob** - the loan-shark debtor | indebted + greedy; the collector's canned template has a natural target from minute one |

## 5. Seeded relationships (the `authored` preset)

Two presets, exposed as a drawer/scenario choice (decided): **flat** -
every edge 0, no facts; **authored** - the seeds below. Playtests
compare lived-in against clean-room.

Facts (pair-facts, written through the store APIs at scenario open):

- **bond(hana, goro)** - siblings; she came for him.
- **bond(bob, steve)** - Bob covered Steve's sister's winter; it is why
  Bob is in debt. (Steve does not know the size of it.)
- **grudge(goro, odd)** - the same job twice, one payment. Rivals.

Regard (directed, small magnitudes; drawer scale):

- steve -> bob +; rin -> steve + (she feeds him extra); ludo -> tim +
  (Tim keeps his father's debt honest); tim -> bob - (Bob owes the
  tally); ines -> odd - (he breaks what she mends); alex -> nobody
  (cold, and the newest to the band).
- Toward the player: founders (bob, steve, alex, tim) small +; the rest
  0. The guildmaster has to earn the six who came later.

*Implemented (w1.1):* `sim::seed_relationships`, gated on the `bonds_preset`
drawer row (0 flat, 1 authored) and so on every stamp. Every seed is written
through the ordinary store API — `adjust_regard`, `record_shared_success`,
`record_grudge` — so a seeded world is a world that could have got there by
living, and no vector is written directly. Regard is written before the facts,
so a bond's floor and a grudge's ceiling are seen to hold what follows them.
The founders' warmth toward the player is +2 each; the six who came later open
at nothing. `autonomy::judge_presets` is the flip test: with the board spent,
somebody goes to see somebody they think well of under the authored seeds and
stays home under the flat ones.

The seeds are chosen so that the two **deliberate gaps** in s7 have
faces: the fighters who should team up hold a grudge, and the pair who
would cover each other (Hana scouts, Goro fights) are the pair Hana
will not be parted from.

## 6. The first five petition templates (one per motivator)

Format per GDD s6: id, source class, trigger, body (text, deadline,
reward, consequence), `next`. Text is a template with `{name}`,
`{other}`, `{site}`, `{n}`, `{deadline}` slots; the card shows the
resolved text, the timer bar, the reward, and the declared consequence
(the 0a-recorded card anatomy). Deadlines are drawer rows.

**Consequence vocabulary** (data; a consequence is a template
reference, GDD s6):

| id | what fires |
|---|---|
| `sours` | regard(petitioner -> player) large -; grudge on repeat/egregious (the petitions rule) |
| `broke` | petitioner's wallet to 0 (burned), desperation +2, source line rewritten to the event |
| `walks-out` | petitioner leaves camp for `{n}` days (an autonomy away-state, not a party), unpaid; returns |
| `gives-away` | petitioner transfers half their wallet to `{other}` (conserved), desperation +1 |

Every consequence fires `sours` as well unless it *is* `sours`; a
failed voiced petition always costs regard (GDD s4.2).

### T1 `collectors-visit` (indebted)

- trigger: has `indebted`; wallet < `{n}`; seeded roll per interval.
- text: "{name}: I owe {n} gold to a man who counts days. Find me work
  that pays before {deadline}, or he takes it out of me."
- deadline: 6 world-days. reward: none (pays in regard). condition:
  `{name}`'s wallet >= `{n}` at any point before the deadline.
- consequence: `broke`.
- next (on failure): `collectors-visit-again` - same text with "He
  came once already." prepended, deadline 3 days, consequence
  `walks-out` (dragged off, {n}=4 days). This is the quest chain in
  miniature; the director's canned loan-shark template (1.5) *is* T1
  fired with source class `director` on anyone indebted.

### T2 `proving-job` (renown)

- trigger: has `renown`; no `fight` task in the last `{n}` days.
- text: "{name}: Send me somewhere that matters. {site} - not the safe
  one. People should hear about it."
- deadline: 5 world-days. reward: none. condition: `{name}` dispatched
  to a `fight` task whose pot >= the drawer threshold.
- consequence: `sours`. next (on repeat failure): `walks-out` ("gone
  looking for a name somewhere else", 5 days).

### T3 `look-after-them` (caring)

- trigger: has `caring`; some `{other}` with desperation >= threshold.
- text: "{name}: {other} has not eaten properly in days. Get {other}
  paying work before {deadline}, or I will feed {other} out of my own
  pocket."
- deadline: 4 world-days. reward: none. condition: `{other}`'s
  desperation below the threshold at the deadline.
- consequence: `gives-away` (to `{other}`). Failure feeds the other
  anyway - the consequence is conserving, and the caring one is poorer
  and colder toward you for it.

### T4 `the-far-road` (restless)

- trigger: has `restless`; not dispatched in the last `{n}` days.
- text: "{name}: I have been looking at the same tents for too long.
  Send me to {site} before {deadline}. Anywhere I have not been."
- deadline: 6 world-days. reward: none. condition: `{name}` dispatched
  to a site they have not visited (a per-character visited set - the
  scout's memory; small sim state, replay-carried).
- consequence: `walks-out` (wandered off, 3 days). The cheap fiction
  asks-travel rides on: the restless are who you send far.

### T5 `a-proper-bench` (maker)

- trigger: has `maker`; no `craft` task or industry shift in `{n}`
  days.
- text: "{name}: I can make things this camp needs, if I have somewhere
  to make them. Put up {building} or give me {n} days at the {industry}
  before {deadline}."
- deadline: 8 world-days. reward: none. condition: an industry built,
  or `{name}` works `{n}` shifts.
- consequence: `sours`, desperation +1. next (on satisfaction):
  `first-order` - "{name}: It is up. Give me something to make." - a
  `craft` task at camp with a small pot from the treasury; the seed of
  the industry arc and the baker dream (post-1.3 optional; record the
  link, build when aspirations arrive).

Before industries exist (1.1), T5 cannot be satisfied and is not fired;
the trigger's "industry shift" clause is what the 1.2 session turns on.

## 7. Coverage matrix and the deliberate gaps

Counts over the ten sheets:

| kind | id | on | count |
|---|---|---|---|
| aptitude | fight | bob, goro, ludo, odd | 4 |
| aptitude | labor | steve, tim, ludo | 3 |
| aptitude | scout | alex, hana | 2 |
| aptitude | craft | rin, ines | 2 |
| motivator | indebted | bob, ludo | 2 |
| motivator | renown | goro, odd | 2 |
| motivator | caring | steve, hana | 2 |
| motivator | restless | alex, odd | 2 |
| motivator | maker | rin, ines | 2 |
| personality | greedy | bob | 1 |
| personality | loyal | steve, rin | 2 |
| personality | proud | tim, goro | 2 |
| personality | craven | ines | 1 |
| personality | vengeful | tim, hana | 2 |
| personality | cold | alex | 1 |

Rules the matrix satisfies (assert them in the registry's validation,
so a future edit that breaks coverage is caught as data): every
aptitude and every motivator on at least two sheets; every kept
personality on at least one; every parked personality on none; every
motivator has a template (s3.2's rule).

**Deliberate gaps** (the matrix is complete but inconvenient):

1. **Nobody is both fighter and scout.** The far sites want someone who
   can get there and someone who can handle what is there; that is two
   people, and the two people have opinions about each other.
2. **The two fighters who should pair hold a grudge** (goro, odd). The
   obvious fight party is a social problem from the first dispatch.

Bonus friction, not counted: two crafters, one of them the cook who is
always busy - the maker's petition bites.

*Implemented (w1.1):* the counts above are asserted in `people::registry` as
rules rather than as numbers — every aptitude and motivator on at least two
sheets, every kept personality on at least one, every parked one on none — so
a future edit that breaks coverage fails at the row that caused it.

**The vocabulary question for the wave-1.5 playtest**, beside the
wave's own: *do the words on the chips match what you watched them
do?* Wave 1.1 made it answerable early: **tapping a trait chip anywhere it
appears** — a sheet, a roster row — shows one line derived from the row
itself, so the words and what they do can be compared without reading the
source. Renames happen before 1.3 writes petition copy against them if
the answer is no; after that a rename costs prose.

## 8. Format notes for the implementing sessions

- **1.1** (*done*): motivator rows gain `favors` (task type or `any`; neutral
  none). Quest rows gain a task type. Aptitude id = task id. Retired
  placeholder rows deleted; parked personalities stay as rows.
  Relationship presets as a drawer/scenario choice. The coverage
  assertions above.
- **1.2**: needs reads `upkeep`; the pariah-candidate check (Steve
  shortfalls first in the idle-player sweep at the shipped numbers - an
  assertion on the seeds, not a hope).
- **1.3**: the template table in this format; consequence vocabulary;
  `proud` needs a field for refusing gifts (the row currently has no
  numeric hook for it); the per-character visited set for T4;
  `walks-out` uses autonomy's away-state.
- **1.4**: resolution reads the aptitude whose id equals the task's
  type; `craven` starts to matter once fight tasks carry danger.
- **1.5**: T1 as the director's loan-shark canned template; the
  no-dead-motivator check runs over the director's templates too.

## 9. Art check - **landed 2026-09-02** (the cast-art session)

Twenty-eight portrait and icon roles are filled and committed. What
this section asked for is done, and what follows is the landed state
rather than the plan.

**The roles.** Six portraits - `portrait_rin`, `portrait_goro`,
`portrait_hana`, `portrait_ludo`, `portrait_ines`, `portrait_odd` -
and nine trait-chip icons - `icon_fight`, `icon_labor`, `icon_scout`,
`icon_craft`, `icon_indebted`, `icon_renown`, `icon_caring`,
`icon_restless`, `icon_maker`. All fifteen are in `Art::ALL` and in
`Gallery::load` (`src/sprites.rs`), so `tools/check-assets` and
`library.rs`'s art contract both know the names and an unfilled role
is a failure before the game runs. **Nothing draws them yet**: wave
1.1 lands the characters and the trait rows that wear them, and the
placeholder policy this section held in reserve is not needed.

The Coin, Heart, Eye, Flame and Skull icons stay with the
personalities, untouched.

**Where they came from.** Every portrait is a Tiny Dungeon bust, the
same pack and the same drawing style as the four that landed in wave
0b, so the ten faces read as one cast. Every icon is Micro Roguelike
at 8x8, the same pack as the four personality-chip icons. Sizes and
drawn scales are `UI.md` s7's table; provenance is
`assets/CREDITS.md`, one row per file; which pack region fills which
role is `art/kenney-manifest.json`, with three to five shortlisted
candidates recorded per role.

**Picking was delegated this once** (owner decision 2026-09-01: away
from the machine with the packs). The session picked against this
section's written criteria and committed the picks sheet
`art/picks/cast-2026-09.png` - the ten portraits at 1x, at map scale
and at 4x, and the nine chips at the 16-unit chip and at 4x, on the
game's own ground and panel colours. **The curation model (owner
picks) is unchanged for everything after this**; DESIGN s7 and
`art/role_sheet.py`'s docstring still say what they said.

**The veto path.** Approve or veto from the PR. A veto is one line:
edit `chosen` in `art/kenney-manifest.json`, and any later session
applies it with `art/extract.py` then `art/import_pack.py` - no code
changes, because the role is the contract and not the picture. The
roster in wave 1.1 does not wait on it.

**What the picks are, and why.** The reason is one line each; the
shortlist each was chosen from is in the manifest.

| role | pick | why this one |
|---|---|---|
| `portrait_rin` | `tinydungeon:99` | the only bust with no armour, no weapon and no working leathers - the camp-follower cook reads as not-a-fighter at a glance |
| `portrait_goro` | `tinydungeon:88` | bare-armed and unarmoured with nothing to hide behind: the man who left home to be talked about |
| `portrait_hana` | `tinydungeon:98` | bare-headed and lightly mailed, and brown-haired like Goro - the sibling bond is in the faces |
| `portrait_ludo` | `tinydungeon:86` | a work apron and a face worn hollow: the labourer working off a debt that was his father's |
| `portrait_ines` | `tinydungeon:100` | grey-white hair over a leather tunic - the oldest hands in the camp, and the one who mends |
| `portrait_odd` | `tinydungeon:97` | armoured and helmed where Goro is bare: the rival who took the same job and reads as his opposite |
| `icon_fight` | `microrl:70` | a sword, and the only steel implement whose blade stays one bright unbroken stroke at 16 units |
| `icon_labor` | `microrl:91` | a ladder - a lattice nothing else in the set shares, after the pick (`microrl:71`) proved indistinguishable from the hammer at chip size |
| `icon_scout` | `microrl:39` | a lantern: the light a traveller carries to find the way, neither pack having a boot, a footprint, a spyglass or a map |
| `icon_craft` | `microrl:74` | a hammer, the craft semantic, and with the pick gone the only other thing on a diagonal is the sword's solid blade |
| `icon_indebted` | `microrl:122` | a satchel - the purse that is owed, and the Coin is taken by `greedy` |
| `icon_renown` | `microrl:47` | a flag on a pole: a name people say, and a clean rectangle at chip size where the gold medallion would have collided with the Coin |
| `icon_caring` | `microrl:138` | a joint of meat - feeding somebody else, which is what `caring` costs, and the pale bone end keeps it off the Flame |
| `icon_restless` | `microrl:115` | a chevron pointing away: the road sign this section offered, and the boldest single shape in either pack at 16 units |
| `icon_maker` | `microrl:55` | a bench - and T5 is `a-proper-bench`; a solid slab nothing else resembles |

**The two families are told apart by weight.** An aptitude is
line-work - a steel-and-timber implement with the panel showing
through it; a motivator is one filled warm mass that fills its cell.
That is the cue a glance uses at 16 units, where an 8x8 picture's
*subject* is not yet legible and its weight already is
(`UI.md` s7).

**The ten at map scale.** Every pair of the ten differs by more than a
detail on the ground colour at native texel size. The tightest pair is
`portrait_tim` and `portrait_odd` - a closed helm and an open one -
at 19% of texels differing and a mean channel distance of 17.5; every
other pair is above 20%, and most are above 40%. That floor is what
the packs allow without putting a cyclops (`tinydungeon:109`) or a
red-eyed troll (`tinydungeon:111`) into a band of human mercenaries;
both scored better and both were rejected for it, and both stay on the
shortlists so the trade is on the record. The floor is now a shipped
assertion rather than a note: `library::portraits_are_tellable_apart`
fails the verify run if any pair falls under 15%, so a veto that picks
a near-duplicate is caught before it is a picture (`UI.md` s7). Tim is a quartermaster who
does not fight and Odd is a fighter, so the pair rarely stands
together in a party - but they do both stand at home on the map, and
that is where a veto would be aimed.
