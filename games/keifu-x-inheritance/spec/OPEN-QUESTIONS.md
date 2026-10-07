# Lineage — open questions

Every place where the code's behaviour is ambiguous, looks unintended, or was a judgement
call in this spec. SPEC.md specifies **what the code does** in every case below; these
entries ask whether that is what the owner wants the port to do. Paths are relative to
`source/application/game/` unless they start with `source/`.

Resolution kinds: **run** (play the original and watch), **owner** (a design decision),
**read** (a closer read or a toolchain fact this session could not establish statically).

Severity: **R** = a rules difference the player can feel; **T** = text/presentation only;
**L** = low, edge case.

---

### OQ-1 — Range of the engine's uniform random number (L, read)

- **Where:** `lineage/chance.jai:1-30`, `source/core/distribution/choice.jai:12` call
  `random_get_zero_to_one_open` from the Jai standard `Random` module, which is not in this
  repository.
- **Unclear:** SPEC assumes u is uniform on [0, 1). If it could return exactly 1.0,
  `roll_between` could return `high + 1`; if it excluded 0, nothing visible changes.
- **Resolves it:** a look at the Jai beta 0.2.030 `Random` module. Distribution-wise the
  assumption is surely right; the port should use any uniform [0, 1) source.

### OQ-2 — Epitaph names the own dream even when the burden was passed (T, owner)

- **Where:** `lineage/epitaph.jai:271-310` reads only `hero.dream`; `lineage/passage.jai:140-144`
  passes the *burden* first.
- **Unclear:** a hero who carried a burden and dies with both dreams undone has the burden
  passed to the heir, but the epitaph says "His dream, <own dream>, was left to X". It
  misnames the dream.
- **Resolves it:** owner decision (port as-is, or name the passed dream). A run can confirm:
  give a hero with an unfinished own dream a burden, let them die, choose an heir.

### OQ-3 — Only one unfinished dream survives a death (R, owner)

- **Where:** `lineage/passage.jai:140-144,250-291`.
- **Unclear:** if a hero dies with an unfinished burden *and* an unfinished own dream, only
  the burden passes (or becomes a ghost). The own dream vanishes: no heir, no ghost, and its
  epitaph sentence reads as in OQ-2.
- **Resolves it:** owner decision.

### OQ-4 — A crowned hero's unfinished dream just ends (R, owner)

- **Where:** `lineage/tale.jai:317-352` (no dream handling), `lineage/epitaph.jai:305-307`.
- **Unclear:** crowning leaves the heirloom with the nearest kin but does nothing with the
  dream: no heir choice, no ghost. The epitaph says it was "left undone for a crown".
- **Resolves it:** owner decision; likely intended.

### OQ-5 — An heir's own heirloom is destroyed when they inherit another (R, owner)

- **Where:** `lineage/passage.jai:261-273` (heir choice) and `lineage/tale.jai:332-348`
  (crown) overwrite the heir's heirloom ("lays X aside, over the hearth"), while
  `lineage/legacy.jai:111-128` (a new legacy heirloom) passes the old one on to an heir.
- **Unclear:** the two paths disagree about what "laid aside" means. The heir button warns
  "(lays one aside)".
- **Resolves it:** owner decision. SPEC follows the code: the laid-aside heirloom is gone.

### OQ-6 — Odds shown at "Set out" can differ from the roll (R, run/owner)

- **Where:** `lineage/tale.jai:46-67,94` (each quest forecasts when it resolves, in board
  order).
- **Unclear:** a death in an earlier quest grieves heroes on later quests (more dread, lower
  power, possibly breaking them mid-summer); a crown adds a patron to later quests; renown
  changes alter later unanswered costs. The player committed against the earlier numbers.
- **Resolves it:** owner decision whether the port keeps sequential resolution (rules-
  identical) — recommended — or snapshots. A run with a disaster death in the Barrow and a
  grieving spouse on a later quest shows it.

### OQ-7 — Fire destiny on a setback kills every fire-doomed member (R, owner)

- **Where:** `lineage/tale.jai:128-133`, `lineage/destiny.jai:126-130`.
- **Unclear:** on a Fire quest's setback, every member doomed by fire dies, not only the one
  hero a setback normally wounds. Matches the doom text ("a failed Fire quest kills") but not
  the quest sheet's "Miss by up to 4: one is wounded."
- **Resolves it:** owner confirmation (almost certainly intended).

### OQ-8 — Settled heroes can still gain courage (R, owner)

- **Where:** `lineage/fear.jai:97-108`: the courage branch precedes the "can dread" check.
- **Unclear:** a hero whose dream is fulfilled has "dread has no hold", yet can still gain
  courage and conquer their fear (and then get +2). Probably harmless, possibly unintended.
- **Resolves it:** owner decision.

### OQ-9 — Children can be grieved into breaking (R, owner)

- **Where:** `lineage/fear.jai:36-38,40-45`, `lineage/bond.jai:241-278`.
- **Unclear:** nothing exempts children from grief. A child whose parent and sibling die can
  reach 5 dread and break, refusing that tag for life before ever questing. Children can
  shed dread only by being seated at the fire.
- **Resolves it:** owner decision; a run where a parent of a young child dies shows the
  dread pips on the child's card.

### OQ-10 — A parent who teaches their own child is not the child's "first teacher" (R, owner)

- **Where:** `lineage/hearth.jai:306-322` (MENTOR bond blocked by the PARENT rank,
  `lineage/bond.jai:98-111`), `generation/hero-context/hero-context.jai:158-177` (calling and
  +1 come from the first MENTOR bond).
- **Unclear:** the guide says "their first teacher decides their calling". A parent who
  teaches their child on the bench gets the credit (taught, the TEACH dream count) but the
  child's calling is rolled from their best aptitude and the +1 does not happen. A later
  non-parent teacher becomes the "first teacher" instead.
- **Resolves it:** owner decision. Run: teach Pip with Maren, then with Odo; see Pip's calling.

### OQ-11 — Who counts as kin (R, owner)

- **Where:** `lineage/bond.jai:146-155`.
- **Unclear:** kin = parent/child or a shared parent. Grandparents, aunts/uncles, nieces/
  nephews and cousins are not kin: they may court and wed if within 15 years of age; they are
  not grieved as kin; heirs ranks treat them as non-kin. An aunt and nephew close in age is
  reachable in 25 years.
- **Resolves it:** owner decision.

### OQ-12 — A child's house is the house of the parent created first (T/R, owner)

- **Where:** `lineage/passage.jai:353-367`, `generation/hero-context/hero-context.jai:105`.
- **Unclear:** the newborn takes `first.house`, where `first` is the spouse with the lower id
  (created earlier). So a Thorne marrying a wanderer always has Thorne children; Wren Hale
  marrying a later wanderer has Hale children; but a wanderer of id 12 marrying a wanderer
  of id 15 gives id 12's house regardless of sex. Rules impact: house colour only, and
  epitaph cradle-ring names ("the <house> cradle-ring").
- **Resolves it:** owner decision.

### OQ-13 — Newborn born_year is one less than the authored convention (L, owner)

- **Where:** authored heroes use `born_year = year - age` (`lineage/household.jai`);
  newborns get `born_year = this year` at the turning but appear (age 0) in the next year
  (`lineage/passage.jai:367`, `generation/hero-context/hero-context.jai:107`).
- **Unclear:** "born in year N" in epitaphs names the year whose turning they were born at.
  Only text and firstborn tie-breaking read born_year.
- **Resolves it:** owner decision (port as-is is safe).

### OQ-14 — "Best four" for the Door includes the wounded and those who would refuse (T, owner)

- **Where:** `lineage/door.jai:87-125`.
- **Unclear:** the top bar's best four and its odds can include a broken hero who refuses
  Dark/Cold and would be bounced from the Door, or a wounded hero (counted at -2).
- **Resolves it:** owner decision; display only.

### OQ-15 — "All three locks open" assumes independent, unchanging locks (T, owner)

- **Where:** `lineage/door.jai:67-85`.
- **Unclear:** the shown chance is the product of three per-lock chances for the starting
  four. In play, deaths, wounds and dread at one lock change the next lock's party and power,
  so the true chance is lower. Display only.
- **Resolves it:** owner decision; port as-is for identity.

### OQ-16 — House closure is checked only after the summer telling (R, owner)

- **Where:** `scene/scenes/telling.jai:184-196`.
- **Unclear:** renown can reach 0 at a turning (a CARRY_THE_HOUSE hero dying of old age:
  -4). The house then plays another summer and closes only if renown is still 0 after it.
- **Resolves it:** owner decision; port as-is for identity.

### OQ-17 — An old-age death of a house carrier costs renown silently (T, owner)

- **Where:** `lineage/tale.jai:377-383` writes the line only when `lines` is non-null;
  `lineage/passage.jai:69-72` passes null.
- **Unclear:** the -4 happens with no text anywhere.
- **Resolves it:** owner decision (add the line in the port, or not).

### OQ-18 — Birth fear attributed to a parent by coincidence (T, owner)

- **Where:** `lineage/passage.jai:401-422`.
- **Unclear:** "has her father's fear of fire" is printed whenever the child's tag equals a
  parent's tag, including when the tag was rolled at random (1 in 8 per parent) and no
  inheritance roll succeeded.
- **Resolves it:** owner decision.

### OQ-19 — Several dream stages can complete in one summer (R, run/owner)

- **Where:** `lineage/dream.jai:153-170` ("any moment" predicates), `lineage/tale.jai:147-150`.
- **Unclear:** stages like "Earn 4 renown", "Court a winter and wed", "Have a child" are
  tested at every moment, and every resolved quest offers a moment to every living hero, so a
  wed parent can tick "wed" and "have a child" on the first two quests of one summer. The
  PROTOTYPE notes say "one stage of a dream per moment", which holds, but moments are many.
- **Resolves it:** owner confirmation.

### OQ-20 — A wasted training winter still counts as "Train a winter" (R, owner)

- **Where:** `lineage/hearth.jai:289-304`.
- **Unclear:** the TRAIN moment fires for every adult in the learner seat, even when the
  lesson was refused ("needs a teacher now", "learns nothing more"). Dream stages "Train a
  winter" and "Train a winter at the forge" complete regardless.
- **Resolves it:** owner decision.

### OQ-21 — Training-yard teachers are credited even for a wasted lesson; bench teachers are not (R, owner)

- **Where:** `lineage/hearth.jai:221-225` vs `:234-238`.
- **Unclear:** an adult in the training-yard teacher seat always gets the teaching credit
  (bond, taught flag, TEACH moment counting toward "Teach the young two winters"), even when
  the learner learns nothing — including when the learner is a child ("belongs on a bench"),
  which also makes that adult the child's first teacher (calling + 1 at coming of age). A
  bench teacher is credited only when the child actually learns.
- **Resolves it:** owner decision.

### OQ-22 — Teaching a rival erases the rivalry (L, owner)

- **Where:** `lineage/bond.jai:98-111` (MENTOR rank 2 replaces RIVAL rank 1).
- **Unclear:** two rivals sharing the training yard become mentor and student (+1 together
  instead of -1). Also true for dream rivals.
- **Resolves it:** owner decision.

### OQ-23 — PROTOTYPE.md disagrees with the code in places (R, owner)

- **Where:** `PROTOTYPE.md` (repo root) vs the cited code.
- **Unclear:** the design notes are partly stale. Examples: wanderer standings "under 10,
  10 to 24, 25 and over" vs code 0/12/30 (`generation/hero-context/templates-hero.jai:42-49`);
  the age table says youth from 14 vs code 12 (`lineage/lore.jai:83`); "trouble ... raises
  demand 1 a step" vs the final formula; births 40% vs 60%; Door locks 24 vs 34. SPEC follows
  the code throughout.
- **Resolves it:** owner confirms the code values are the intended final ones.

### OQ-24 — A hero who breaks at the Door keeps going (L, owner)

- **Where:** `lineage/door.jai:127-162` (no refusal check between locks), `lineage/fear.jai:89`.
- **Unclear:** breaking at the first lock (dread 5) does not remove them from the second and
  third; they still pay the fear penalty but no longer gain dread or courage there.
- **Resolves it:** owner decision; port as-is for identity.

### OQ-25 — Is UI wording part of "content-identical"? (T, owner)

- **Where:** `content/ui-text.json` (guide, help texts, hover notes, labels).
- **Unclear:** the contract says visual/UI presentation is free but hand-authored content is
  identical. Narrative lines (`lines.json`, `epitaph.json`, pools, templates) are clearly
  content. The guide and the rule-explaining hover texts sit in between.
- **Resolves it:** owner decision. They are extracted verbatim either way.

### OQ-26 — Name bags can repeat founders' names (T, owner)

- **Where:** `generation/hero-context/hero-context.jai:9-31`.
- **Unclear:** "Wren" is both a founder and in the girls' name bag; a newborn or wanderer can
  be a second Wren. Names also repeat once a bag is exhausted and refilled.
- **Resolves it:** owner decision; port as-is for identity.

### OQ-27 — Any two AVENGE_THE_LOST dreams make rivals (L, owner)

- **Where:** `lineage/bond.jai:221-239` compares dream kinds only.
- **Unclear:** two non-kin heroes avenging different people become rivals ("wants what X
  wants").
- **Resolves it:** owner decision.

### OQ-28 — Deed tellings are written but never shown (T, owner)

- **Where:** every `record_deed` call stores a telling; nothing reads `Deed.telling`
  (INVENTORY.md, "Dead or unreachable").
- **Unclear:** whether the port needs them. They are extracted (`lines.json` `deed.*`).
- **Resolves it:** owner decision; safe to keep as data.

### OQ-29 — Ghost order after removal (L, owner)

- **Where:** `lineage/ghost.jai:106-110`, `generation/hero-context/hero-context.jai:210-215`
  (Jai `remove it` is an unordered swap-remove), `generation/quest-context/quest-context.jai:175-181`
  (only the first ghost is offered each summer).
- **Unclear:** with three or more ghosts, which one appears next depends on swap-remove
  order. SPEC specifies swap-remove; the owner may prefer oldest-first.
- **Resolves it:** owner decision. Exact emulation is cheap.

### OQ-30 — Crown renown threshold includes the triumph that crowns (L, owner)

- **Where:** `lineage/tale.jai:154-156` runs after `reward_party` (`:192`).
- **Unclear:** a hero at 6 personal renown triumphing at a danger-1 Court quest (+2) reaches
  8 and is crowned by that same triumph. Doom text says "at 8 renown". Probably intended.
- **Resolves it:** owner confirmation.

### OQ-31 — Children's vocation before coming of age is "Knight" (L, read)

- **Where:** `lineage/household.jai:118-127,192-200` never set Pip's or Wren's vocation; the
  enum default is KNIGHT. Newborns likewise (`generation/hero-context/hero-context.jai:100-146`).
- **Unclear:** invisible in play (children are drawn as children and sheets say "Child of
  house"), and coming of age always sets a vocation. Listed only so the port does not invent
  a different default that leaks somewhere.
- **Resolves it:** none needed unless the port shows a child's vocation.

### OQ-32 — The "fulfilled" check on the Door promise is dead (L, read)

- **Where:** `lineage/passage.jai:314` tests `destiny.fulfilled`, which is never set for
  OPEN_THE_SEALED_DOOR.
- **Unclear:** nothing; the promise always passes on death. Listed so nobody hunts for the
  missing setter.
- **Resolves it:** none.

### OQ-33 — Board ties and float ordering (L, read)

- **Where:** `generation/quest-context/quest-context.jai:48-73,97-108`, `lineage/door.jai:96-105`.
- **Unclear:** comparisons are strict `>` on float32 sums of 1/36. Equal-k chances produce
  bit-identical floats, so ties are genuine and resolve to the first candidate; a port using
  exact fractions orders identically. The Door's best-four score adds `/1000` terms in float;
  ties there are display-only.
- **Resolves it:** none expected; flagged because ties decide which board/party is kept.
