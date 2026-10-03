# ninjo - the cast (founding band, vocabulary, templates)

The cast bible: who lives in Kawaza, what the trait words are, the seeded
relationships, and the petition templates. It is content where `GDD.md` is
design; where the two disagree about a decided thing, the GDD wins. The
roster and the vocabulary are data - the ten sheets and their homes are
`src/people.rs`, the trait vocabulary `src/traits.rs`, the templates
`petitions::TEMPLATES`, the standing rates `asks::RATES`, the relationship
presets the `bonds_preset` drawer row - and this document is the living
record of that content, rewritten state per the living-docs convention
(`docs/conventions.md` §Documents). Where a number appears it is a drawer
starting value or a row's content, not a decision about tuning. A live
wave's landings carry `*Implemented (wN):*` notes per section; wave 1's are
folded below and live in git.

**Vocabulary status: LOCKED** (owner, 2026-10-02). Petition copy is
written against the shipped words, so a change from here is a rename, not a
data edit. What follows is the record of how they were chosen. The
aptitude and motivator words below are the owner's leaned choice made
before the context that would test them exists (party building is
wave 4). They are ids in one table with display names beside them;
renaming is a data edit. The MVP gate playtest carries an explicit
vocabulary question (s7). Template text and source lines refer to traits by
display name through the row, never by prose - so a rename touches one cell.

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

## 2. Task taxonomy (four types; aptitude ids double as task ids)

| task id | what it is | where it shows up at MVP |
|---|---|---|
| `fight` | clear a site of what is in it | site jobs at the crypt and the cave |
| `labor` | camp work; haul and stores | haul/survey site jobs |
| `scout` | travel-heavy; go and look | the far sites; the fiction asks-travel rides on |
| `craft` | mend, build | the camp works' standing shifts; the first building |

Every authored job carries a task type (`Quest::task`, data on the row),
and it is **drawn on the row too**: every job on a site's board carries its
task-type chip, in the aptitude icon whose id is the task's, beside the fit
the reader has for it (UI.md s3c). `TaskType::aptitude()` and
`TaskType::of_aptitude()` are the round trip, and the vocabulary's
validation asserts it both ways - every type has exactly one aptitude row
and every aptitude row is some type's. Resolution reads the aptitude whose
id equals the task's type, and the scorer weighs it.

**The board is twenty-four jobs**, six a site, each site leaning toward the
work its fiction implies and carrying at least one of every type: ten
people looking for work empty a seven-job board before the first day is
out, and an aptitude with nothing to do is a chip that means nothing. Sites
still run dry.

## 3. Trait vocabulary (content)

### 3.1 Aptitudes (kind `aptitude`; one per task type)

| id (= task) | chip | line (stranger-facing) | aptitude |
|---|---|---|---|
| `fight` | fighter | "stands where the trouble is" | 2 |
| `labor` | laborer | "does the long work without being asked twice" | 2 |
| `scout` | scout | "knows the way, or finds it" | 2 |
| `craft` | crafter | "fixes it, or builds the thing that replaces it" | 2 |

These four are the rows. The aptitude id **is** the task id, so
`competence_at(task, traits)` reads one row rather than summing everything
the carrier can do. Every aptitude row is 2, so the cast has exactly two
odds per job - a fit of 2 is `safe` (fails 11%, goes well 24%) and a fit of
0 is `risky` (fails 35%, never goes well) through `resolution::odds`; a
third would need a third aptitude value, which is content.

### 3.2 Motivators (kind `motivator`; five, each with its template)

| id | chip | line | upkeep | pressure | favors |
|---|---|---|---|---|---|
| `indebted` | indebted | "owes somebody, and the somebody is not patient" | 5/4 | 3 | any |
| `renown` | renown | "wants a name people say" | 1/1 | 2 | fight |
| `caring` | caring | "somebody else's trouble is their trouble" | 3/2 | 2 | any |
| `restless` | restless | "wants to be somewhere else, for a while" | 1/1 | 2 | scout |
| `maker` | maker | "wants to make something that lasts" | 5/4 | 2 | craft |

**`favors`** (neutral: none) is the task type this want's pressure applies
to, `any` meaning any paid work. It is a field on the row (`Favors::None`,
`Any`, or `Task(t)`), asserted neutral on every non-motivator row like every
other kind-owned field, and read by `traits::pressure_toward` - the only
place a want reaches the scorer, and it never looks at an id. **`upkeep`**
is read by `needs::cost_of` through `traits::upkeep_of`: the multipliers are
what make one interval of the camp's upkeep cost Steve 7g, Bob 6g and Alex
the base 5g, and the whole gradient of who slides and who does not comes
out of this column and the purses in s4 - nothing else in the economy
distinguishes one person from another. This is the row's whole mechanical
surface; the *petition* half of each motivator is s6.

**No-dead-motivator rule** (decided): every motivator row has at least
one template in s6 whose source class is `motivator` and whose trigger
names it. `traits::vocabulary` asserts it by walking
`petitions::templated_motivators()`, the motivator rows the checked
templates name, and asserts besides that no motivator has zero pressure.

### 3.3 Personalities (giri's nine, audited)

Audit question: *does it shape daily scorer choices or ask verdicts at
MVP?* Kept rows are on the founding sheets; parked rows stay in the
vocabulary (the trait x mark reaction table references them and stays
whole) but appear on no sheet until marks are common - that is the
betrayal ladder's era.

| id | verdict | why |
|---|---|---|
| `greedy` | keep | pot pull is a scorer term |
| `loyal` | keep | bonds x2 shape whom they work beside and whom they obey |
| `proud` | keep | the ask refuser; refuses charity when gifts exist - the field for that has no reachable surface yet (s4.1) |
| `craven` | keep | danger terms x2 once fight tasks carry danger; dormant - failure is economic only, and whether a character weighs their own odds is open (`FINDINGS.md` G-049) |
| `vengeful` | keep | grudges x2, never decay - the repeat-refusal story |
| `cold` | keep | edges x1/2 both ways - the one who cannot be bought with regard |
| `pious` | park | reacts to marks by kind; marks are rare before the ladder |
| `pragmatic` | park | prefers a known skimmer; mark-dependent |
| `upright` | park | refuses the dark-marked; mark-dependent |

No personality was added: the scorer did not ask for one. A personality
that owns a scorer field (a work/idle bias, say) is proposed when the scorer
wants one, not before. The parked three are `people::PARKED` - declared
beside the roster rather than as a field on the trait row, because being on
a sheet is a casting decision and not a property of the word - and the
registry asserts they are in the vocabulary and on nobody.

Authoring norm: two or three traits per sheet (practice, not rule; the
list cap is gone).

## 4. The founding band (ten)

Mongrel names on purpose: a band drawn from everywhere.

| id | name | role | traits | wallet | desp. | arrives | source line |
|---|---|---|---|---|---|---|---|
| `bob` | Bob | founder, fighter | fighter, greedy, indebted | 6 | 4 | d1 00:00 | owes the collector at the toll-house, who counts days |
| `steve` | Steve | founder, laborer | laborer, loyal, caring | 3 | 5 | d1 00:00 | sends half of everything to a sister whose hands gave out |
| `alex` | Alex | founder, scout | scout, cold, restless | 12 | 1 | d1 00:00 | has not slept a full month in one place since childhood |
| `tim` | Tim | founder, quartermaster (camp-follower) | laborer, proud, vengeful | 20 | 2 | d1 00:00 | keeps the tally, and is owed by half the camp |
| `rin` | Rin | cook (camp-follower) | crafter, maker, loyal | 5 | 2 | d1 10:00 | cooks for ten on a fire built for three |
| `goro` | Goro | fighter | fighter, renown, proud | 9 | 3 | d1 18:00 | left home to be talked about, and nobody is talking yet |
| `hana` | Hana | scout | scout, caring, vengeful | 7 | 2 | d2 08:00 | came for her brother; stays exactly as long as he does |
| `ludo` | Ludo | laborer, fights when asked | laborer, fighter, indebted | 2 | 4 | d2 16:00 | works off a debt that was his father's before it was his |
| `ines` | Ines | crafter | crafter, maker, craven | 10 | 2 | d3 07:00 | mends what breaks, and would rather be far from what breaks it |
| `odd` | Odd | fighter | fighter, renown, restless | 8 | 3 | d3 18:00 | took the same job as Goro twice, and only one of them got paid |

All ten, in this order, are `people::roster`; the wallets, desperations and
arrival minutes are the scenario file's (`scenarios/freeplay.txt`, GDD s6).
**Homes are two rows of tents south of the road and east of the ford** -
y=15 at x 12/16/20/24/28 and y=17 at x 6/10/14/18/22 - passable, unshared,
off named locations (the registry asserts it), and spaced so ten names, ten
figures and the town's own marker do not collide, which `floors.rs` asserts
rather than this sentence. A character leaves from the doorstep they are
standing on and comes home to it, which is why the journey minutes differ
per character.

**The arrival column is the backstory, in order.** The camp opens with the
four founders, because s1 says a band arrived a season ago and it is *that*
band; the other six are "the six who came later" made mechanical rather
than a new fiction, and the order is the fiction's own:

- **Rin first**, and soonest - the cook follows the fire she built, and the
  camp is hers to feed before it is anything else.
- **Goro next**, drawn by what an unclaimed crossing pays a fighter.
- **Hana after Goro**, because she came for her brother and cannot have
  arrived before him. The arrival column is where "stays exactly as long as he
  does" first means something.
- **Ludo**, following the debt, at the first place that was hiring.
- **Ines** when there is something worth mending, which is after the band has
  been breaking things for two days.
- **Odd last**, arriving to find Goro already here and already talked about -
  the grudge s5 seeds is a thing that has now had time to happen.

The times are drawer-invariant content: each row carries a `present_from`
world-minute and the arrival is an occurrence on the one scheduler, so it is
world-time addressed and speed-invariant like everything else, and the lens
filters every surface by presence off one derivation (`Lens::roll`). The
registry asserts the column is monotonic, that presence agrees with it,
that the founding band is exactly these four names, and - separately,
because a camp whose opening four cannot do any of the opening work is a
camp whose first decisions are not decisions - that **each founder has at
least one open job they have an aptitude for** on the opening board. The
coverage matrix in s7 applies to the whole roster.

### 4.1 Demo characters (each MVP module names the person it is proved on)

| module | demo character | why this one |
|---|---|---|
| needs (1.3) | **Steve** - the pariah-candidate | highest upkeep multiplier (caring 3/2), lowest wallet among earners, labor pays least: first to shortfall with the player idle |
| autonomy (1.1) | **Ludo** - the eager worker | indebted pressure 3, favors any, no pride: takes whatever work is open without being asked; the character the scorer is most visibly alive on |
| asks (1.2) | **Tim** - the proud refuser | proud refuses; vengeful turns a repeat into a grudge; and he holds the tally, so the refusal costs the player something |
| petitions (1.5) | **Goro** - the petition fountain | renown fires the proving-job template most often; Odd makes every answer to it a social problem |
| settlement (1.3) | **Rin** - the industry seed | maker; her petition asks for the first building; the camp fire becoming a kitchen is the first-building beat and the baker-dream's ancestor |
| events-director (1.6) | **Bob** - the loan-shark debtor | indebted + greedy; the collector's canned template has a natural target from minute one |

What each claim is, as the batteries hold it:

- **Steve shortfalls first in every world** (`economy::PARIAH`), and it is
  the staged start that makes the claim true rather than merely likely. On
  day one the camp is the four founders; Steve's `caring` multiplies the base
  upkeep to 7g and he opens with 3g, and he is the only one of the four who
  cannot meet the first interval - Bob pays his in full with a gold left
  over. It is not registry order: Bob is index 0. The economy sweep asserts it
  in **every** world it runs, a stronger claim than the median one the
  handoff asked for - and **it is what sets the share**: his 3g purse against
  a 7g first interval holds the self-chosen share at three percent
  (`FINDINGS.md` G-048).
- **Ludo's claim is staged, not played**, and that is a deviation worth
  naming: he is one of the six who came later, and by the time he walks in on
  day two the board has been claimed (`FINDINGS.md` G-032). What the claim is
  about is what he does *with an offer*, and a world with no offer left
  cannot say - so `autonomy::judge_alive` stages a full board and asserts he
  takes work at the first minute he is asked to think.
- **Tim agrees at the standing rates like every other founder.** The
  refusals the shipped set produces are the *fits*, not the pride: Alex will
  not leave scouting, and Ines and Rin will not leave crafting. Pride's own
  refusal needs the gift-refusal field, and that field has no reachable
  surface: the gift is GIVE, it exists only on a money-shaped condition, and
  the only one is T1's - carried by Bob and Ludo, neither of them proud. It
  waits for a proud character with a money-shaped petition (GDD s10). Until
  then the demo character for this module is a promise rather than a
  demonstration.
- **Rin is the industry seed**, mechanically: the camp works are `craft`
  work (s2), the fire she cooks on with a roof over it, and she is the only
  crafter in Kawaza for two world-days - she arrives on day one and Ines on
  day three - so the first shift the works ever open is hers to take. T5 is
  her petition for the building.
- **Bob** is D1's natural target from minute one; the pinned test scenario
  fires it at him at world-minute 90 (GDD s6).

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

`sim::seed_relationships` writes them, gated on the `bonds_preset` drawer
row (0 flat, 1 authored) and so on every stamp. Every seed is written
through the ordinary store API - `adjust_regard`, `record_shared_success`,
`record_grudge` - so a seeded world is a world that could have got there by
living, and no vector is written directly. Regard is written before the
facts, so a bond's floor and a grudge's ceiling are seen to hold what
follows them. The founders' warmth toward the player is +2 each; the six who
came later open at nothing. `autonomy::judge_presets` is the flip test: with
the board spent, somebody goes to see somebody they think well of under the
authored seeds and stays home under the flat ones.

The seeds are chosen so that the two **deliberate gaps** in s7 have
faces: the fighters who should team up hold a grudge, and the pair who
would cover each other (Hana scouts, Goro fights) are the pair Hana
will not be parted from.

## 6. The petition templates (one per motivator, the shortfall's, and the director's three)

Format per GDD s6: id, source class, trigger, body (text, deadline,
reward, consequence), `next`. Text is a template with `{name}`,
`{other}`, `{site}`, `{n}`, `{deadline}` slots; the card shows the
resolved text, the timer bar, the reward, and the declared consequence
(UI.md s3h). `petitions::TEMPLATES` is this section as data, every number
on the row. **Deadlines and `{n}`s are on the row, not in the drawer**:
seven deadline rows would have filled the drawer, and a template is
content; the drawer carries what *is* tuning - the regard step, the relief,
the check's cadence and roll, and the thin-days window. **No template pays
gold**; the reward port's first exerciser is a staged row in the battery.
The lookbacks (T2, T4, T5) are **2** world-days, and the person has to have
been in the camp that long.

**Consequence vocabulary** (`petitions::KINDS`, the four, asserted; a
consequence is a template reference, GDD s6):

| id | what fires |
|---|---|
| `sours` | regard(petitioner -> player) large -; grudge on repeat/egregious (the petitions rule) |
| `broke` | petitioner's wallet to 0 (burned), desperation +2, source line rewritten to the event |
| `walks-out` | petitioner leaves camp for `{n}` days (presence, not a party - the staged start's own away-state, through the lens), unpaid; returns |
| `gives-away` | petitioner transfers half their wallet to `{other}` (conserved), desperation +1 |

Every consequence fires `sours` as well unless it *is* `sours`; a
failed voiced petition always costs regard (GDD s4.2).

### T1 `collectors-visit` (indebted)

- trigger: has `indebted`; wallet < `{n}`; seeded roll per interval.
- text: "{name}: I owe {n} gold to a man who counts days. Find me work
  that pays before {deadline}, or he takes it out of me."
- deadline: 6 world-days. reward: none (pays in regard). condition:
  `{name}`'s wallet >= `{n}` at any point before the deadline. `{n}` is
  **30g**.
- consequence: `broke`.
- next (on failure): `collectors-visit-again` - same text with "He
  came once already." prepended, deadline 3 days, consequence
  `walks-out` (dragged off, {n}=4 days). This is the quest chain in
  miniature; the director's canned loan-shark template *is* T1
  fired with source class `director` on anyone indebted - D1 below.

### T2 `proving-job` (renown)

- trigger: has `renown`; no `fight` task in the last `{n}` days.
- text: "{name}: Send me somewhere that matters. {site} - not the safe
  one. People should hear about it."
- deadline: 5 world-days. reward: none. condition: `{name}` dispatched
  to a `fight` task whose pot is **55g** or more (on the row; 55 is the
  richest fight pot on three of the four boards, so "not the safe one"
  means something). `{site}` names the authored site with the richest fight
  job.
- consequence: `sours`. next (on repeat failure): `proving-job-again` - the
  same ask a second time, declaring `walks-out` ("gone looking for a name
  somewhere else", 5 days) - so the repeat's consequence is the one its own
  card printed, never a different one than the first card showed.

### T3 `look-after-them` (caring)

- trigger: has `caring`; some `{other}` with desperation >= threshold.
- text: "{name}: {other} has not eaten properly in days. Get {other}
  paying work before {deadline}, or I will feed {other} out of my own
  pocket."
- deadline: 4 world-days. reward: none. condition: `{other}`'s
  desperation below the threshold at the deadline. **The threshold is the
  `desperate` chip's**, `needs::DESPERATE_AT` (6) - one predicate, so a
  person the chip names is a person T3 can speak for. The condition is hard
  to meet (`FINDINGS.md` G-051, open).
- consequence: `gives-away` (to `{other}`). Failure feeds the other
  anyway - the consequence is conserving, and the caring one is poorer
  and colder toward you for it.

### T4 `the-far-road` (restless)

- trigger: has `restless`; not dispatched in the last `{n}` days.
- text: "{name}: I have been looking at the same tents for too long.
  Send me to {site} before {deadline}. Anywhere I have not been."
- deadline: 6 world-days. reward: none. condition: `{name}` dispatched
  to a site they have not visited - the per-character visited set is
  `Character::memory`, written on arrival, replay-carried; `{site}` is the
  first site in registry order its petitioner has never reached.
- consequence: `walks-out` (wandered off, 3 days). The cheap fiction
  asks-travel rides on: the restless are who you send far.

### T5 `a-proper-bench` (maker)

- trigger: has `maker`; no `craft` task or industry shift in `{n}`
  days.
- text: "{name}: I can make things this camp needs, if I have somewhere
  to make them. Put up {building} or give me {n} days at the {industry}
  before {deadline}."
- deadline: 8 world-days. reward: none. condition: an industry built,
  or `{name}` works **2** shifts. `{building}` and `{industry}` are the
  settlement's own content.
- consequence: `sours`, desperation +1 - `sours` with one more step on the
  declared consequence, not a fifth kind: the vocabulary is four. next (on
  satisfaction): `first-order` - "{name}: It is up. Give me something to
  make." - 3 days, met by any craft work, `sours`. The treasury-paid craft
  task at camp the chain describes is the seed of the industry arc and the
  baker dream; the link is recorded and exercised, and that task is built
  when aspirations arrive.

### T6 `thin-days` (shortfall) - the shortfall exemplar

The escalation pipe's second rung: the first template whose source class is
`shortfall` rather than a motivator, so it can speak for anybody - Tim, who
carries no motivator, included.

- trigger: three `upkeep-shortfall` events on `{name}` inside the thin-days
  window (`thin_window`, a drawer row: three world-days); one active petition
  per character, as ever. A shortfall answered is a shortfall spent: the window
  opens no earlier than this template's last petition for them was met or
  failed.
- text: "{name}: Three intervals short now. I need paying work by {deadline}
  or I am done waiting for it."
- deadline: 4 world-days. reward: none (pays in regard). condition: a paid job
  or shift completed by `{name}` before the deadline.
- consequence: `walks-out` ({n}=3 days).

### D1-D3 - the director's, source class `director`

The capsule's own three examples, carried in from outside the camp by the
minimal injector (GDD s5's events-director; `src/director.rs`). They are
petitions like any other - the same card, ledger, cliff and vocabulary of
four - and the only difference the player sees is the source chip, which
reads **`event`**: "from outside the camp - not their own want". None is
raised by the petition check; the injector fires them after the calm
window, at most `director_max` unresolved at once, on somebody eligible
(present, carrying no petition, holding a trait the `Carries` trigger
names), drawn by the firing's own address. All three pay in regard. The
no-dead check runs over them too (`petitions::vocabulary`: every director
row reaches somebody in the cast, and requires the injector's registry
row).

**D1 `the-collector-comes`** (eligible: `indebted`, wallet regardless)

- The canned loan shark T1's note promised: T1's text, deadline (6 days),
  condition (`{name}`'s wallet >= `{n}`, {n} = 30) and consequence (`broke`),
  with T1's chain (`collectors-visit-again` on failure). What differs is who
  it reaches: anyone indebted, purse regardless - the collector does not wait
  for poverty, he arrives. **When the purse already holds 30g it is met at
  its voicing**: the collector arrives, is paid, and goes - a pressure event
  that relieves pressure (`FINDINGS.md` G-056, open).

**D2 `rival-offer`** (eligible: `renown` or `greedy`)

- text: "{name}: The Grey Banners came through {site} offering {n} a week.
  Show me what I am worth here before {deadline}, or I will go find out."
- `{site}`: any authored site, drawn at the firing. `{n}`: 30 to 50 gold,
  drawn at the firing (`spread` 20 on the row).
- deadline: 5 world-days. condition: `{name}` is paid at least `{n}` - wages
  and shares, summed since voicing - before the deadline.
- consequence: `walks-out` ({n}=5 days - trying them out).

**D3 `word-from-the-road`** (eligible: `restless` or `renown`)

- text: "{name}: Travelers say {site} is worth somebody's time again - good
  pots for whoever moves first. We should be first."
- `{site}`: an authored site with open work on its board, drawn at the
  firing; with none, D3 reaches nobody. (The site names carry their own
  "the", so the text carries none.)
- deadline: 4 world-days. condition: any job at `{site}` completed, by
  anybody, before the deadline (the player's hand if that job was posted).
- consequence: `sours`.

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

Rules the matrix satisfies, asserted in `people::registry` as rules rather
than as numbers - so a future edit that breaks coverage fails at the row
that caused it: every aptitude and every motivator on at least two sheets;
every kept personality on at least one; every parked personality on none;
every motivator has a template (s3.2's rule).

**Deliberate gaps** (the matrix is complete but inconvenient):

1. **Nobody is both fighter and scout.** The far sites want someone who
   can get there and someone who can handle what is there; that is two
   people, and the two people have opinions about each other.
2. **The two fighters who should pair hold a grudge** (goro, odd). The
   obvious fight party is a social problem from the first dispatch.

Bonus friction, not counted: two crafters, one of them the cook who is
always busy - the maker's petition bites.

**The vocabulary question for the MVP gate playtest**, beside the gate's
own: *do the words on the chips match what you watched them do?* It is
answerable from the screen: **tapping a trait chip anywhere it appears** - a
sheet, a roster row - shows one line derived from the row itself, so the
words and what they do can be compared without reading the source. The
vocabulary is locked (above): a rename now is a data edit on the row plus
the prose written against the word.

## 9. Art - the roles, and what fills them

*(There is no s8: the per-wave format notes that stood there were folded into
the sections they annotated at the wave-1 close. The number is kept because
the code and the art tooling cite this section as s9.)*

Twenty-eight portrait and icon roles are filled and committed; this section
is their record.

**The roles.** Ten portraits - the four founders' and `portrait_rin`,
`portrait_goro`, `portrait_hana`, `portrait_ludo`, `portrait_ines`,
`portrait_odd` - and nine trait-chip icons - `icon_fight`, `icon_labor`,
`icon_scout`, `icon_craft`, `icon_indebted`, `icon_renown`, `icon_caring`,
`icon_restless`, `icon_maker`. All are in `Art::ALL` and in
`Gallery::load` (`src/sprites.rs`), so `tools/check-assets` and
`library.rs`'s art contract both know the names and an unfilled role is a
failure before the game runs. The portraits are drawn on the map, the roster
and the character panel; the chips on every trait chip (UI.md s3, s7).

The Coin, Heart, Eye, Flame and Skull icons stay with the
personalities, untouched.

**Where they came from.** Every portrait is a Tiny Dungeon bust, the
same pack and the same drawing style, so the ten faces read as one cast.
Every icon is Micro Roguelike at 8x8, the same pack as the four
personality-chip icons. Sizes and drawn scales are `UI.md` s7's table;
provenance is `assets/CREDITS.md`, one row per file; which pack region fills
which role is `art/kenney-manifest.json`, with three to five shortlisted
candidates recorded per role.

**Picking was delegated this once** (owner decision 2026-09-01: away
from the machine with the packs). The cast-art session picked against this
section's written criteria and committed the picks sheet
`art/picks/cast-2026-09.png` - the ten portraits at 1x, at map scale
and at 4x, and the nine chips at the 16-unit chip and at 4x, on the
game's own ground and panel colours - and the approval moved to the PR by
way of that sheet. **The curation model (owner picks) is unchanged for
everything after**; DESIGN s7 and `art/role_sheet.py`'s docstring still say
what they said.

**The veto path.** A veto is one line: edit `chosen` in
`art/kenney-manifest.json`, and any later session applies it with
`art/extract.py` then `art/import_pack.py` - no code changes, because the
role is the contract and not the picture.

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
shortlists so the trade is on the record. The floor is a shipped
assertion: `library::portraits_are_tellable_apart` fails the verify run if
any pair falls under 15%, so a veto that picks a near-duplicate is caught
before it is a picture (`UI.md` s7). Tim is a quartermaster who does not
fight and Odd is a fighter, so the pair rarely stands together in a party -
but they do both stand at home on the map, and that is where a veto would
be aimed.
