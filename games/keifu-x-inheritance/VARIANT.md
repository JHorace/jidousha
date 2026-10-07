# Keifu X Inheritance — the variant's canon

A fork of mainline Keifu (`games/keifu/`, a faithful port of Lineage) that diverges on what a
family *inherits*. Mainline is never touched by this crate's task; this file is the variant's
canon beside mainline's `spec/` (copied here whole, as mainline's reference). Every rule below
is a delta: where it says nothing, the variant keeps mainline's rule and mainline's code.

The design is `tools/yakin/runs/keifu-x-inheritance/DESIGN.md`; this file is the deltas as
built, with the departures named.

## What the game is

A house of heroes kept for 25 years, one turn a year — summer quests, a winter at the hearth,
the turn of the year — ending at the Sealed Door, exactly as mainline. The variant adds a
heritable black mark for failed personal quests, a real distinction between family and
outsiders, and rudimentary trait genetics.

## Deltas, rule by rule

**Family and outsiders** (`src/outsiders.rs`, `Hero::family`). The nine founders are family; a
wanderer arrives an outsider; a newborn is family iff either parent is (the house name is the
family parent's, the first-created's when both are); an outsider becomes family only by
marrying in.

- *Outsiders earn the house nothing.* A won quest's renown (and the carriers' bonus, counted over
  family only) goes to the house only if a family member is in the party; otherwise the page
  reads `lines.quest.reward_outsiders` and only the members are paid. The first *family* adult at
  the long table tells it for the house; an outsider is `Teller::Own`.
- *Outsiders never inherit.* The heir list is family only; a crowned outsider keeps their
  heirloom (`nearest_kin` is none); an outsider's death page never waits — it says
  `lines.death.outsider`, buries the heirloom and raises an undone dream's ghost
  (`heirs::settle_with_no_one`, the code "No one. Let it lie." shares).
- *Marrying in.* The garden's checks gain UNPROVEN after WED_ALREADY: exactly one of the pair an
  outsider with personal renown below `MARRY_IN_RENOWN` (5) reads `<name> unproven: renown R of
  5`. At the threshold it reads `will wed`; marrying in makes them family, gives them the house
  name of their spouse, and the house takes `renown / DOWRY_SHARE` (2) as a dowry
  (`lines.winter.married_in`). The dead's epitaphs are composed again so the new name reads
  everywhere. `outsiders::may_marry_in` is the one eligibility function.

**Traits** (`src/traits.rs`, `Hero::traits`). Six: Strong/Frail (Might), Sharp/Dull (Wits),
Steadfast/Faint (Spirit); at most one per aptitude. A gift adds `TRAIT_POWER` (1) to a quest of its
aptitude and a flaw takes it off, as a power line after the heirloom line and before the fear line
(`  is strong`). Not read at the Door. Founders: Garrick Strong, Elsbeth Sharp, Maren Sharp,
Brannoc Strong, Odo Steadfast. At birth, after every mainline roll, per aptitude (Might, Wits,
Spirit): both parents alike → that trait, no roll; one parent → `TRAIT_PASS_CHANCE` (0.5); a gift
and a flaw → a coin for which; then, if the child has none, `TRAIT_SPRING_CHANCE` (0.125) and a coin
for gift or flaw. A spring that lands on a parent's own trait is told as that parent's. The
birth page tells each (`lines.birth.trait_like` / `trait_sprung`); the sheet has a TRAITS section.

**Personal quests and the black mark** (`src/marks.rs`, `Hero::marks`). A quest is *personal* to a
seated **family** hero whose **own** dream (never a burden) calls it and needs it won (a "succeed" or
"triumph" call; a "go" call makes nothing personal); never at the Door (`marks::personal`, which the
card, the sheet and the resolution all call). A personal quest *fails* on a setback or a disaster:
resolution step 7b (after the disaster's house renown, before the fear is faced) marks the name —
the one writer is `marks::mark_the_name` — costing the house `MARK_HOUSE_COST` (2) and the failer
`MARK_PERSONAL_COST` (2, floored at 0), with `lines.mark.fallen` and a `DeedKind::Marked`.
`marks::cost` gives the numbers the sheet previews and the resolution pays.

- *How marks stack:* any number; identity is `(origin, year, place)` and no hero carries one twice.
- *How they weigh and lapse:* the turning's step 8b (`marks::weigh`, onto "Year N begins"): a mark
  with `current_year - mark.year >= MARK_YEARS` (8) is forgotten, then the house loses
  `MARK_YEARLY` (1) for each **distinct** mark the **living family** carries. A mark earned in
  year Y weighs at the turnings of years Y through Y+7.
- *Across generations:* by years only; a passed mark keeps its year, and `generation` counts the
  passes (read by the sheet's wording only).
- *On the board:* the card reads `Marked if it fails: <names>` after the dream row; the quest sheet
  has a `PERSONAL:` line per personal member, with the chance it fails, what failing costs now and
  yearly, and the last year it weighs.

**Inheritance** (`src/inheritance.rs`). `inherit(heroes, from, to)` is what `to` would carry
having taken `from`'s inheritance: their own traits (traits are blood; succession passes none)
and their own marks plus each of `from`'s that are new to them, at `generation + 1`. Three callers
and no other merge: the death page's candidate lines (`heirs::heir_lines`, drawn above the
buttons: `Maren, daughter: Sharp; marks 1 (+1)`, then where "No one" leaves them), the heir choice
(`heirs::choose` → `pass_marks`: the chosen heir, or on "No one" the dead's earliest-born living
child, or the ground; whoever takes a mark pays `MARK_HEIR_COST` (1) per mark new to them), and
birth (a child is born under both parents' marks, counted once). A death page waits when the dead
carries a mark.

## Decisions (the spec's three rows, as built)

| decision | surface | commit | one function | asserted by |
|---|---|---|---|---|
| Whether to send a family member on a personal quest | the quest card's `Marked if it fails:` row and the sheet's `PERSONAL:` line, mid-drag included | "Set out" | `marks::personal` / `marks::cost` / `marks::mark_the_name` | `x1::check_personal` |
| Whether to let an outsider marry in | the garden's note (`unproven: renown R of 5` / `will wed`) | "Let the winter pass" | `outsiders::may_marry_in` | `x1::check_marrying_in` |
| Which heir to choose given what they inherit | the death page's candidate lines | the heir button | `inheritance::inherit` | `x1::check_heir_inheritance` |

## Constants (`src/constants.rs`, the mutation round's targets)

`MARRY_IN_RENOWN` 5 · `DOWRY_SHARE` 2 · `TRAIT_POWER` 1 · `TRAIT_PASS_CHANCE` 0.5 ·
`TRAIT_SPRING_CHANCE` 0.125 · `MARK_HOUSE_COST` 2 · `MARK_PERSONAL_COST` 2 · `MARK_YEARLY` 1 ·
`MARK_YEARS` 8 · `MARK_HEIR_COST` 1.

## Text

New strings are in `spec/content/ui-text.json` and `lines.json`, each carrying `source:
"keifu-x-inheritance VARIANT.md"`, read through `W` variants in `words.rs`, so a missing key
stops the game at load as mainline's do. The six traits are `lore.json`'s `traits` table.

## Not in the variant

Redeeming a mark by a later triumph · marks on the family screen, remembrance or epitaph · a burden
making a quest personal · traits beyond the six, read by training, old age, courtship or the Door ·
outsiders' own marks · any change to board generation, the forecast, the Door, the Ending or the
epitaphs.

## Mainline's oracles the variant rewrote

W8's stirred heir list (the wanderer is never offered: seven buttons); W1's Garrick sheet
(TRAITS between FEAR and DESTINY); W3's power line 7 (Garrick's grave-goods power 7, with Strong);
W4's `you bring 12` → `14`, its odds rows (gaps 3..5), its breakdown lines, and its history panel
read with the wheel at the end (the PERSONAL line lengthens the sheet); `w4_rules`' patron card
(16); W6's played page (`Brought 14`); the power, power-lines, card and sheet unit tests; and the
epitaph watcher of W9's battery, which accepts a recomposition when a name changed. Mainline unit
tests that count a party's power without the variant's traits use
`testkit::house_without_traits()` (easing, reading, resolve, telling).
