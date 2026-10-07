# DESIGN — keifu-x-inheritance

task: keifu-x-inheritance
variant: V2
model-as-configured: claude-fable-5-1 (the routine's setting as this session's context states it; not verified — the run page is the authority on what served)
date: 2026-10-07 08:59 PDT

This is a **delta design** over mainline Keifu (`games/keifu/`, a faithful port of
Lineage). Every rule below is written as "mainline does X; the variant does Y"; where a
section says nothing about a rule, the variant keeps mainline's rule and mainline's code.
Mainline's `spec/SPEC.md` section numbers are cited as §n. `games/keifu/` is read-only for
you too (the spec's Fence); you copy it and change only the copy. **You never touch
`crates/**`**, and nothing here needs you to.

Read before building, in this order: this file whole; the spec
`tools/yakin/tasks/keifu-x-inheritance.md`; mainline's `spec/SPEC.md` §3, §5.4, §6, §7.1,
§7.3, §9.6, §11.3, §11.6, §15.1–§15.2, §17.2–§17.3, §18; then the mainline modules named
in **Systems** below, each whole, before you edit its copy. `docs/api/jidousha-testing.md`
once, top to bottom, before the first check.

## What the game is

A house of heroes kept for 25 years, one turn a year — summer quests, a winter at the
hearth, the turn of the year — ending at the Sealed Door, exactly as mainline Keifu. The
variant heightens what a family *inherits*: a failed personal quest puts a heritable black
mark on the family name, outsiders are cheap hands who earn the house nothing until they
marry in, and a child is born with one parent's gift or flaw. Everything else — the board,
the dice, the hearth, the Door, the epitaphs — is mainline's, unchanged.

## The player-facing loop

Mainline's loop, with three new things to read and weigh:

1. **Summer, the board.** The player drags heroes onto quest cards and reads the card and
   its sheet in the dock. *New:* when a seated **family** hero's own dream needs this quest
   won, the card reads `Marked if it fails: <names>` and the sheet's `PERSONAL:` line gives
   the chance it fails and exactly what a failure costs now and later. "Set out" commits.
   A lost personal quest marks the name on its quest page; the house loses renown now, the
   hero loses renown now, and the mark weighs on the house a renown a year for eight years
   and passes to the blood at every death and birth meanwhile.
2. **Winter, the hearth.** *New:* the garden's note refuses an **outsider** (a wanderer who
   never married in) whose personal renown is under the threshold — `Odo unproven: renown
   3 of 5` — and reads `will wed` at the threshold. Marrying in makes them family: they take
   the family name, the house takes half their renown as a dowry, and from then their
   children are family and their quests pay the house. Until then an outsider's won quests
   and tales pay only themselves. "Let the winter pass" commits.
3. **The turn of the year, the death page.** *New:* every candidate on an undecided death
   page shows, beside their kinship, their **traits** and the **marks** they would carry if
   chosen; "No one" says where the marks fall instead (to the firstborn, or into the
   ground). Choosing commits, as mainline. A birth page names the trait a child was born
   with and whose it is, and the marks the child is born under.
4. The top bar's house renown is where the marks are felt: the "Year N begins" page says
   `The name carries 2 marks: -2 renown.` each year they weigh.

## Systems

In build order. Every path is under `games/keifu-x-inheritance/` (the spec's Fence); "copy
of" names the mainline file it starts from. The `docs/api/` surface each touches is the one
mainline already uses: `docs/api/jidousha-api.md` `Rng` (every roll goes through the fork's
`chance.rs` on the run's generator, SPEC §22), `Rect`, `TextStyle`; the checks use
`docs/api/jidousha-testing.md` `HeadlessSim`, `FrameRecorder`, `SnapshotBuilder`,
`InputEvent`, `MemorySource`; the pictures use `docs/api/jidousha-capture.md`
`WgpuBackend::offscreen`, `encode_png`. The fork does **not** adopt `jidousha::ui`
(`docs/api/jidousha-ui.md`): mainline draws every screen through its own `screen::Page`,
and one copied game keeps one way of drawing (see Decisions).

- **S0 The copy** — `cp -r games/keifu games/keifu-x-inheritance`, then the renames: the
  package name (`Cargo.toml` `name = "keifu-x-inheritance"`; the manifest names
  `jidousha = { path = "../../crates/jidousha" }` and nothing else, as mainline's does), the
  window title (`main.rs` `config`: `title: "Keifu X Inheritance"`), the verdict line
  (`verify.rs` `run`: `verified keifu-x-inheritance: ...`), every capture file name
  (`capture.rs`: `keifu.png` → `keifu-x-inheritance.png`, `keifu-w3-garrick.png` →
  `keifu-x-inheritance-w3-garrick.png`, and so on — `target/verify/` is shared with
  mainline and the names would collide), the asset root string
  (`asset_source("games/keifu/assets")` → `"games/keifu-x-inheritance/assets"`; grep
  `games/keifu` across the copy), and the crate doc header of `main.rs` (one paragraph:
  "Keifu X Inheritance: a fork of Keifu that diverges on what a family inherits; the
  rules it changes are `VARIANT.md`'s"). The `[keifu]` prefix on panic messages may stay.
  `mutants/*.txt` need no change (paths are relative to the game). `spec/` is copied whole
  and left as mainline's reference; `SPEC-GAPS.md` and `FINDINGS.md` are copied and
  continued, not restarted (G-numbers continue from mainline's last, G-069; KG-numbers
  from KG-74), under a heading `## Keifu X Inheritance`. This is the spec's first commit:
  it changes nothing else, and `python3 tools/verify keifu-x-inheritance` must pass on it.
  · `games/keifu-x-inheritance/**`, `Cargo.lock` (the new crate's entry only) · touches no
  `docs/api/` item beyond mainline's.
- **S1 Family and outsiders** — one flag and the rules that read it · `src/hero.rs`
  (`Hero.family: bool`), `src/household.rs` (`found`: every founding hero `family = true`),
  `src/wanderer.rs` (`arrive`: `family = false`, the arrival line), `src/births.rs`
  (`born`: `family = either parent's family`; the house name is the family parent's, the
  first-created parent's when both are, as mainline), `src/plans.rs` (`courtship`: the new
  verdict `Courtship::Unproven { outsider, renown }` and its note; `tellers`: the first
  **family** adult is `Teller::House`, an outsider adult is `Teller::Own`), `src/winter.rs`
  (`court`: marrying in), `src/reward.rs` (`reward`: house renown only through family),
  `src/heirs.rs` (`heirs`: family only; `nearest_kin`: `None` for an outsider dead),
  `src/legacy.rs` (`heir`: family only), `src/death_page.rs` (an outsider's page), new
  `src/family.rs` (the one predicate `is_family(hero) -> bool` and the one eligibility
  function `may_marry_in(heroes, a, b) -> Option<(HeroId, i32)>`: the outsider and their
  renown when exactly one of the pair is an outsider below `MARRY_IN_RENOWN`), `src/sheet.rs`
  (the outsider line) · `Rng`: none new.
- **S2 Traits** — the six gifts and flaws, authored on founders, rolled at birth, read by
  the power sum · new `src/traits.rs` (`Trait { aptitude: Aptitude, gift: bool }`, its id
  and words from the fork's `spec/content/lore.json` `traits` table, `trait_from(f, m,
  rng)`), `src/hero.rs` (`Hero.traits: Vec<Trait>`, at most one per aptitude),
  `src/household.rs` (a `trait` key per founder, optional), `src/births.rs` (the rolls,
  the birth lines), `src/power.rs` (`member_power`: the trait line), `src/power_lines.rs`
  (the sheet's trait line), `src/sheet.rs` (`TRAITS`) · `Rng`: the birth rolls.
- **S3 Marks** — the black mark: earned on a lost personal quest, carried by heroes,
  weighing on the house · new `src/marks.rs` (`Mark`, `personal(..)`, `mark_the_name(..)`
  — the one writer of a mark, step 7b — and `weigh(..)` — step 8b, lapse then drain,
  returning its lines), `src/house.rs`
  (`marks_carried`), `src/resolve.rs` (step 7b), `src/quest_card.rs`
  (`CardReading.personal`), `src/quest_sheet.rs` (the `PERSONAL:` line),
  `src/turning.rs` (step 8b), `src/sheet.rs` (`MARKS`), `src/hero.rs`
  (`Hero.marks: Vec<Mark>`, `DeedKind::Marked`) · `Rng`: none new.
- **S4 Inheritance** — the one function the death page's preview, the heir choice and the
  birth all call · new `src/inheritance.rs` (`inherit(heroes, from, to) -> Carried`),
  `src/heirs.rs` (`choose`: marks to the chosen heir, or to the blood on "No one"; the
  page waits on marks too), `src/death_page.rs` (`leaves`), `src/births.rs` (the child's
  marks), `src/turning_view.rs` (`lay_out_choice`: the candidate lines and the room they
  take), `src/passage.rs` (nothing new: `Bequest.leaves` already carries "waits") · `Rng`:
  none new.
- **S5 The variant's checks** — new `src/x1.rs` (the three decision checks, played through
  the scripted pointer), `src/x1_stages.rs` (the stagings the checks, the floors and the
  pictures share), the rewritten mainline oracles (named in Gates), the floors over the new
  surfaces (`src/floors_x1.rs`, counted into `floors::battery`), the pictures
  (`src/capture.rs`), the mutation list `mutants/x1.txt`, and `VARIANT.md` at the crate
  root (the Deltas and Decisions of this file, as built, so a later session has the
  variant's canon beside mainline's `spec/`) · `HeadlessSim`, `FrameRecorder`,
  `SnapshotBuilder`, `WgpuBackend::offscreen`.

### The deltas, rule by rule

**Family.** Mainline has no notion of family: every hero is "of the house". The variant
holds `Hero.family`: the nine founders are family (all of them, whatever their house name
— Thorne, Vane, Hale and Fenn are all the family); a wanderer arrives an **outsider**
(`family = false`); a newborn is family iff either parent is (after a marrying-in, both are);
an outsider becomes family only by marrying in. "The name" in prose means the family; a
hero's `house` string stays what mainline makes it, except at marrying in (below).

**Outsiders earn the house nothing.** Mainline (§7.3): a won quest adds `renown (+1
triumph) + carriers` to house renown whoever went. Variant: the house gains that **only if
at least one family member is in the party**, and carriers are counted over family members
only; with no family member in the party the house gains nothing and the page reads
`lines.quest.reward_outsiders` in place of `lines.quest.reward` (personal renown to every
member as mainline; at the Door the same rule, no line, as mainline writes none). Mainline
(§11.3 step 5): the first adult teller gives the house +1. Variant: the first **family**
adult teller does (`Teller::House`); an outsider adult at the table is `Teller::Own`
(preview `own +`), whichever seat. Disaster costs, unanswered costs, patrons from a crowned
outsider, grief: mainline, unchanged — the house posted the quest and the bonds are bonds.

**Outsiders never inherit.** Mainline (§15.1): the heir list is every living hero but the
dead, ranked. Variant: every living **family** hero but the dead, ranked by mainline's
table. Mainline (§14.2): an heirloom passed on in life goes to the heir of the blood, else
a living empty-handed spouse or student. Variant: the same walk over family members only
(an outsider spouse before marrying in cannot exist, since the garden refuses; an outsider
student can, and is skipped). Mainline (§15.3): a crowned hero's heirloom goes to their
nearest kin. Variant: a crowned **outsider** keeps it (`nearest_kin` is `None`;
`lines.crown.heirloom_taken`). Mainline (§15.1): a death page waits when the dead holds an
heirloom or leaves a dream. Variant, an outsider's death page: it never waits. Its first
line is `lines.death.outsider`; then exactly what mainline does when "No one. Let it lie."
is chosen — the heirloom is buried (`lines.heir.buried_with`), an undone dream raises a
ghost (`lines.ghost.raised`, dream fate LEFT_TO_NO_ONE) — written on the page as it is
made, no heirs gathered, `leaves = false`. An outsider has no marks to pass (only family
can be marked) and no traits to pass by anything but blood.

**Marrying in.** Mainline (§11.6): the garden's checks, in order, are NOBODY, WAITING, KIN,
TOO_YOUNG, TOO_FAR_APART, RIVALS, WED_ALREADY, then WILL_WED. Variant: after WED_ALREADY
and before WILL_WED, **UNPROVEN**: when exactly one of the two is an outsider and that
outsider's personal renown is below `MARRY_IN_RENOWN` = 5, the verdict is
`Courtship::Unproven { outsider, renown }` and the garden's note reads `<name> unproven:
renown <renown> of 5` (`ui.winter.courtship_notes.UNPROVEN` = `"% unproven: renown % of %"`).
Two outsiders may wed each other as mainline (both stay outsiders; their children are
outsiders). At the threshold or above the verdict is WILL_WED, and when the winter resolves
a WILL_WED with one outsider, after mainline's wedding line and deeds: the outsider's
`family = true`, their `house` becomes the spouse's `house` (the full name changes
everywhere it is drawn; the first name does not), the house gains `outsider.renown /
DOWRY_SHARE` (integer division, `DOWRY_SHARE` = 2; at renown 5 that is +2) and the page
reads `lines.winter.married_in`. `may_marry_in` (`family.rs`) is the one function:
`courtship` asks it for the verdict, `court` asks it again to know whom to make family, and
the arrival line and the sheet print its threshold. The wanderer's arrival page gains, after
`lines.arrival.fear`, `lines.arrival.outsider`; an outsider's sheet gains, after its renown
line, `ui.hero_sheet.outsider`.

**Traits.** Mainline has aptitudes, a fear, blessings and a destiny; a child's aptitudes
average the parents' (§17.3). Variant adds **traits**, the rudimentary genetics; the
aptitude formula, the fear rolls and the blessings stay exactly mainline's. There are six
traits, a gift and a flaw per aptitude, in the fork's `lore.json` `traits` table in this
order: `STRONG` (Might, gift), `FRAIL` (Might, flaw), `SHARP` (Wits, gift), `DULL` (Wits,
flaw), `STEADFAST` (Spirit, gift), `FAINT` (Spirit, flaw); each row `{id, aptitude, gift,
title, telling}` with titles Strong/Frail/Sharp/Dull/Steadfast/Faint and tellings
strong/frail/sharp/dull/steadfast/faint. A hero holds at most one trait per aptitude. **What
a trait does:** on a quest of its aptitude, a gift adds `+TRAIT_POWER` (= 1) and a flaw
`-TRAIT_POWER` to that member's lines (§6), as a new line **after the heirloom line and
before the fear line**, told `ui.quest_sheet.power_line_trait` = `"  is %"` with the
telling and the signed value; the floor at 0 applies as mainline. Nothing else reads a
trait (not training, not old age, not the Door). **Founders:** `household.json` gains an
optional `trait` key (a trait id) — Garrick STRONG, Elsbeth SHARP, Maren SHARP, Brannoc
STRONG, Odo STEADFAST; Pip, Ysolde, Aud and Wren none. (Elsbeth and Aud are dead; their
traits are authored so a reader sees where Maren's comes from; they pass nothing.)
Wanderers arrive with no trait. **At birth**, after mainline's rolls (pronoun, name, the
three aptitude coins, the fear tag, the fear inheritance rolls), for each aptitude in order
Might, Wits, Spirit, with `f` the first parent's trait there and `m` the second's:
- both the same (both `None` included): the child gets it, no roll;
- exactly one parent has one: one roll, `chance(rng, TRAIT_PASS_CHANCE)` (= 0.5): the
  child gets it or nothing;
- one gift and one flaw: one roll, `index(rng, 2)`: 0 the first parent's, 1 the second's;
- then, only if the child got nothing above: one roll `chance(rng, TRAIT_SPRING_CHANCE)`
  (= 0.125) and, if it hits, one roll `index(rng, 2)`: 0 the gift, 1 the flaw.
`traits::trait_from(f, m, rng) -> Option<Trait>` is that rule, once, called three times;
the birth page tells each trait the child has: `lines.birth.trait_like` =
`"%1 is %2, like %3 %4."` (He/She, telling, his/her, father/mother — the parent it came
from; when both had it, the second parent, as mainline attributes a shared fear) or
`lines.birth.trait_sprung` = `"%1 is %2, and neither parent is."`; no line for an aptitude
with no trait. The hero sheet gains a `TRAITS` section after the fear section, one line per
trait: `ui.hero_sheet.trait_line` = `"%: % % on quests."` (Strong, +1, Might); no section
when there are none (as mainline's DREAM for an undreamt adult, KG-4). The family screen's
remembrance and the epitaph say nothing of traits.

**Personal quests and the black mark.** Mainline has no personal quest; its nearest thing
is a dream call (§9.6). Variant: a posted quest is **personal** to a seated hero when (a)
the hero is family, and (b) the hero's **own** dream (never a burden) calls this quest for
the party as seated and the call's telling is **succeed** or **triumph** — the stage needs
the quest won. A "go" call (reach a place, walk a new road) makes nothing personal; an
"apart" call cannot apply to a seated hero. `marks::personal(content, heroes, facts, party)
-> Vec<HeroId>` returns those members in seat order, from `calls::dream_call`; the card,
the sheet and the resolution all call it with the party they are reading. A personal quest
**fails** when its outcome is SETBACK or DISASTER. Resolution (§7.1) gains **step 7b**,
after step 7 (the disaster's house renown) and before step 8 (facing the fear): for each
member `personal(..)` names, with the dream as it stands at that moment (an earlier quest
this summer may have moved it, as mainline's OQ-6 lets odds move): push `Mark { origin:
member, year, place: the quest's, generation: 0 }` onto that hero's `marks`; house renown
`-MARK_HOUSE_COST` (= 2); the member's personal renown `-MARK_PERSONAL_COST` (= 2),
floored at 0; the line `lines.mark.fallen`; a `DeedKind::Marked` deed (place the quest's,
weight the danger, telling `lines.deed.marked`). A member who died in the same resolution
is marked all the same (step 7b runs before steps 9 and 10; the mark then passes at their
death page the same turning). **How marks stack:** a hero carries any number; two failures
are two marks; identity is `(origin, year, place)`, and no hero carries one identity twice.
**How they weigh:** the turning (§18) gains **step 8b**, after step 8 (the tales): first,
every mark on every hero whose `current_year - mark.year >= MARK_YEARS` (= 8) is removed,
with `lines.mark.lapsed` per distinct identity removed from a living family member; then
house renown `-= MARK_YEARLY` (= 1) × the number of **distinct** marks carried by living
family members (`House::marks_carried`, distinct by identity, in creation order of the
first carrier), with `lines.mark.weighs_one` / `lines.mark.weighs_many`; nothing when none.
So a mark earned in year 1 weighs at the turnings of years 1 through 8 and is forgotten at
the turning of year 9: "weighs through year 8" is `year + MARK_YEARS - 1`. **How they
decay across generations:** by years only — a passed mark keeps its year, so the blood
carries it until the same lapse; `generation` counts how many times it was passed and is
read only by the sheet's wording. **Renown, both:** the house pays at the failure and
yearly; the failer pays personally at the failure; whoever takes a mark at a death page
pays `MARK_HEIR_COST` (= 1) personal renown per mark newly taken, floored at 0 (a newborn
pays nothing: born at 0). The sheet gains a `MARKS` section after `TRAITS`, one line per
mark: `ui.hero_sheet.mark_own` = `"Failed at % in year %. Weighs on the name through year
%."` for `generation == 0`, else `ui.hero_sheet.mark_inherited` = `"%'s failure at % in
year %. Weighs on the name through year %."` (origin's name, the place's mid-sentence
`name`); no section when none. **On the board:** `CardReading` gains `personal:
Option<String>`, drawn as one row after the dream row: `ui.quest_card.personal` =
`"Marked if it fails: %"` with the names as a name list; `quest_sheet` gains, after the
call lines and before `NEEDS`, one line per personal member: `ui.quest_sheet.personal` =
`"PERSONAL: %1's dream. If it fails (%2 in 100) the name is marked: -%3 house renown and -%4
to %1 now, then -%5 a year to the house through year %6, and the blood carries it."` where
`%2` is `percent((setback ways + disaster ways) / 36)` from the same `forecast` the card's
odds are counted from (one fraction, then `percent`), and `%6` is the current year `+
MARK_YEARS - 1`. The card and the sheet read the previewed party mid-drag exactly as
mainline's card does (KG-28).

**Inheritance.** Mainline passes, at the death page, an heirloom and an undone dream to
the chosen heir (§15.2), and at birth a share of aptitudes, maybe a fear, and blessings
(§17.3). Variant: **one function**, `inheritance::inherit(heroes, from, to) -> Carried {
traits: Vec<Trait>, marks: Vec<Mark> }` — what `to` would carry having taken `from`'s
inheritance: `traits` are `to`'s own (succession passes no trait; traits are blood, rolled
at birth), `marks` are `to`'s own plus each of `from`'s whose identity `to` does not carry,
at `generation + 1`, in `from`'s order after `to`'s. `Carried::taken` counts the marks that
are new to `to`. Three callers, no other code merges marks: (1) **the death page's
preview** — `turning_view::lay_out_choice` draws, between the prompt and the buttons, one
line per candidate in list order, at draw time (so a choice on an earlier page the same
turning is reflected, as mainline reads its "(not the dream)" marks at draw time, KG-47):
`lines.heir.carries` = `"%1, %2: %3; marks %4"` — name, kinship (`heirs::kinship_to_dead`),
the candidate's traits' titles as a name list or `lines.heir.no_trait` = `"no trait"`, and
`"<total> (+<taken>)"` when `taken > 0` else `"<total>"`, all from `inherit(heroes, dead,
candidate)`; then, only when the dead carries marks, one line for "No one":
`lines.heir.no_one_marks_blood` = `"No one: the marks fall to %, blood of the name."`
naming the dead's earliest-born living child (mainline's firstborn-living rule,
`death_page::door_promise`'s search) or `lines.heir.no_one_marks_ground` = `"No one: the
marks go into the ground."` when there is none. `CHOICE_H` grows by one `PITCH` per line
drawn (candidates, plus the "No one" line when present); every leaf rule of
`turning_view::leaves` and the long-page check hold as before. (2) **The choice** —
`heirs::choose(.., Some(h))`: after mainline's heirloom and dream, `h.marks =
inherit(heroes, dead, h).marks`, `h.renown -= MARK_HEIR_COST × taken` (floored 0), and
`lines.heir.takes_marks` = `"% takes the name's shame with the rest: % mark(s)."` written
as `lines.heir.takes_marks_one` = `"% takes the name's shame with the rest: a mark."` /
`lines.heir.takes_marks_many` = `"% takes the name's shame with the rest: % marks."`, only
when `taken > 0`; `choose(.., None)`: the marks go to the earliest-born living child by the
same `inherit` (lines `lines.mark.passes_one` = `"%'s mark falls to %, blood of the name."`
/ `lines.mark.passes_many` = `"%'s % marks fall to %, blood of the name."`; that child pays
`MARK_HEIR_COST` per mark taken) or, with no living child, lapse with the dead
(`lines.mark.buried` = `"%'s shame goes into the ground with %."`, name and object pronoun).
The dead's own `marks` are left as they are (the dead act no more). A death page **waits**
when the dead holds an heirloom, leaves an undone dream, **or carries a mark**
(`Bequest.leaves`); an outsider's never waits (above). (3) **Birth** — after the child is
pushed with its rolled traits and no marks, `child.marks = inherit(heroes, first,
child).marks`, then the same for `second`; the birth page then reads
`lines.birth.marks_one` = `"% is born under a mark on the name."` / `lines.birth.marks_many`
= `"% is born under % marks on the name."` when the child carries any. The Ending (§23)
passes no marks: the final summer's dead get no page, as mainline.

**Constants** (`src/constants.rs`, each with its CONSTANTS-style doc line; these are the
mutation round's targets): `MARRY_IN_RENOWN: i32 = 5`, `DOWRY_SHARE: i32 = 2`,
`TRAIT_POWER: i32 = 1`, `TRAIT_PASS_CHANCE: f64 = 0.5`, `TRAIT_SPRING_CHANCE: f64 = 0.125`,
`MARK_HOUSE_COST: i32 = 2`, `MARK_PERSONAL_COST: i32 = 2`, `MARK_YEARLY: i32 = 1`,
`MARK_YEARS: i32 = 8`, `MARK_HEIR_COST: i32 = 1`.

**Text** lives in the fork's `spec/content/ui-text.json` and `lines.json` under the keys
named above (every new key under `_variant` notes, so a reader can tell them from
Lineage's verbatim strings), each read through a `W` variant in `words.rs` so a missing
key stops the game at load as mainline's do. Every new string is printable ASCII (the
floors check it). The strings above are the shipped literals; the checks copy them by hand.

## Gates to add

Every gate is a `--verify` check (`Checks::require`, numbers in the specifics) or a test
named as a sentence; all seeded; no wall-clock. `python3 tools/verify
keifu-x-inheritance` is ground truth (`target/verify/keifu-x-inheritance.json`).

- **G0 The pure copy** — input: the S0 commit · asserts: `tools/verify keifu-x-inheritance`
  reports `pass` on the copy before any variant rule; `tools/verify keifu` still passes;
  `git diff --stat origin/main...` names nothing under `games/keifu/` · covers Done-when
  lines 1, 3, 4.
- **G1 The personal quest and its mark (decision row 1)** — `x1::check_personal`, input:
  seed `X1_SEED` (a shipped literal you choose, see below) · the scripted pointer drags
  Garrick **alone** onto "Grave goods" (board slot 0; his dream "Win a triumph at the
  Barrow" calls it *triumph*, so it is personal to him; alone he brings 7 — Might 5 after
  the Elder's -2, Thornfall +1, Strong +1 — against a demand of 9..11, so he fails on
  most seeds), points at the card: asserts the card's rows contain `Marked if it fails:
  Garrick` and the dock's lines contain the `PERSONAL:` line with `%2` equal to the card's
  setback + disaster percentages as one fraction, `-2`, `-2`, `-1`, `through year 8`,
  every number a shipped literal; then "Set out", the telling read to its end: asserts the
  quest page carries `lines.mark.fallen` for Garrick, house renown equals the shipped
  literal for that seed (15, less the unanswered three, less the disaster's danger if it
  was one, less 2), Garrick's renown `6 - 2 = 4`, and `House::marks_carried()` is exactly
  `[Mark { origin: Garrick, year: 1, place: Barrow, generation: 0 }]` (he may be wounded or
  dead by it; the mark stands either way). `X1_SEED` is a seed on which that resolution is
  a SETBACK or a DISASTER; choose it by sweeping seeds 0..64 off-screen once, then ship it
  and its outcome as literals, never computed in the check. A second, seed-free assertion
  stages the same seating off-screen and resolves with `resolve::resolve_rolled(.., dice
  [1, 1])`: a disaster (margin 7 + 2 - 7 - demand), the same single mark, house renown `15
  - 2 - 2 = 11` (the disaster's danger 2, then the mark; `resolve_rolled` takes no
  unanswered cost), Garrick's `4` · covers Done-when line 2 (row 1).
- **G2 Not personal, no mark** — in `x1`: the same board with only Brannoc on "Grave goods"
  (no call that needs it won) resolved on dice `[1, 1]`: no mark, no `lines.mark.fallen`,
  house renown falls only by the disaster; and with Garrick's seat held by an outsider
  carrying a copy of his dream (stage a wanderer with `QUIET_THE_BARROW` at stage 3): no
  mark — only family is marked · covers row 1's rule.
- **G3 The mark weighs and lapses** — two parts. A unit test in `marks.rs`, named as a
  sentence: a founded house with one mark `(Maren, 1, Barrow, 0)` on Maren, the calendar
  set to each year in turn, `marks::weigh(content, house)` (step 8b as one function:
  lapse, then drain; returns its lines) gives `-1` and `The name carries a mark: -1 renown.`
  for years 1 through 8 and `0` with `lines.mark.lapsed` at year 9, and nothing at year
  10; a second mark of the same identity on Pip changes nothing; a different identity
  makes it `-2` and `The name carries 2 marks: -2 renown.`; a mark on an outsider counts
  nothing. And `x1::check_weighing` through the screen on `SEEDS[0]`: year 1 left at home
  into the winter, Maren given that mark, the winter let pass: the "Year 2 begins" page
  carries `The name carries a mark: -1 renown.` and house renown is `11 - 1 = 10` (W6's
  stay-home 11, less the mark) · covers the weighing rule.
- **G4 Marrying in (decision row 2)** — `x1::check_marrying_in`, input: seed `SEEDS[0]`;
  year 1 left at home into the winter (`w7::stay_home_into_winter`); `wanderer::arrive`
  staged on the house as `w8::stir` does, the wanderer's renown set to 4; the pointer drags
  Ysolde and the wanderer into the garden: asserts the garden's panel reads
  `<name> unproven: renown 4 of 5` and `winter::plan`'s verdict is `Unproven`; the
  wanderer's renown set to 5 (the house untouched otherwise): the panel reads `will wed`;
  "Let the winter pass": asserts the winter page carries the wedding line and
  `lines.winter.married_in`, the wanderer's `family` is true, their `house` is `"Vane"`,
  house renown rose by `+2` beyond what the same winter gives without the marrying-in, and
  their sheet in the dock no longer carries `ui.hero_sheet.outsider` · covers Done-when
  line 2 (row 2).
- **G5 Outsiders earn nothing and inherit nothing** — in `x1`, off-screen: a staged
  wanderer alone on board slot 1 with its demand set to 1 (as mainline's unit tests set a
  demand), resolved on dice `[6, 6]`: the page carries `lines.quest.reward_outsiders`,
  house renown unchanged by the reward, their renown up by the quest's; the same with
  Maren beside them: `lines.quest.reward`, the house up by the quest's renown; a wanderer
  at table seat 0 and Odo at seat 1: `tellers` reads `[Own, House]` and the house gains
  `+1` once;
  Garrick's death page (`w8::stage_garricks_winter` with a wanderer staged): the heir list
  omits the wanderer; the wanderer aged to 94 instead: their page carries
  `lines.death.outsider`, waits for no one, and their staged heirloom is buried · covers the
  outsider rules.
- **G6 Traits at birth** — a unit test in `traits.rs` named as a sentence per branch, on
  fixed seeds: both parents Strong → Strong, no roll consumed; one Strong → Strong on
  about half of 400 seeds and the other half nothing; Strong and Frail → one of the two,
  never none; neither → a trait on about one in eight seeds, gift and flaw about evenly;
  and the roll count per branch (0, 1, 1, 1 or 2) measured by the generator's position ·
  covers the genetics rule.
- **G7 The trait on the board** — in `x1`: Garrick on "Grave goods": the sheet's breakdown
  carries `  is strong` `+1` after `  carries Thornfall` and before any fear line, and the
  card's `you bring` is mainline's `12` for Garrick and Brannoc **plus 2** (both Strong);
  the W4 played oracle's `you bring 12` is rewritten to `you bring 14` (Deviations) ·
  covers the trait effect.
- **G8 The heir's inheritance (decision row 3)** — `x1::check_heir_inheritance`, input: seed
  `SEEDS[0]`; `w8::stage_garricks_winter`, then Garrick given one mark `(Garrick, 1,
  Barrow, 0)` on the house, the winter let pass, "Go on" to the choice: asserts the choice
  block's lines are, in order, `Maren, daughter: Sharp; marks 1 (+1)`, `Pip, grandson: no
  trait; marks 1 (+1)`, `Odo, friend: Steadfast; marks 1 (+1)`, `Ysolde, of the house: no
  trait; marks 1 (+1)`, `Brannoc, of the house: Strong; marks 1 (+1)`, `Wren, of the house:
  no trait; marks 1 (+1)`, `No one: the marks fall to Maren, blood of the name.`, then the
  seven buttons mainline's W8 oracle lists; Maren is chosen: asserts her `traits` is
  `[Sharp]` and her `marks` is `[(Garrick, 1, Barrow, 1)]` — equal to what her line showed
  — her renown `4 - 1 = 3`, the page carries `lines.heir.takes_marks_one`, and
  `marks_carried()` is that one mark; a second run choosing "No one": Maren carries it all
  the same (the blood), with `lines.mark.passes_one`; a third with Maren dead before the
  page: `lines.mark.buried` and `marks_carried()` empty · covers Done-when line 2 (row 3).
- **G9 Born under a mark** — in `x1`, off-screen: Maren and Brannoc wed and Maren marked,
  `births::births` on a seed that bears a child: the child's marks are `[(Garrick, …,
  generation 2)]` or `[(Maren, …, 1)]` as staged, the birth page carries
  `lines.birth.marks_one`, and `marks_carried()` still counts the identity once · covers
  the birth path of `inherit`.
- **G10 Mainline oracles the variant rewrites** — each kept as a check with its literal
  rewritten and listed in the PR's Deviations: W8's stirred heir list (`w8::stirred`) no
  longer names the wanderer; W4's `you bring 12` → `14` and the sheet's breakdown gains the
  two trait lines (`w4.rs`, `w4_rules.rs`, `quest_sheet.rs`'s unit test); W1's Garrick
  sheet oracle gains `TRAITS` / `Strong: +1 Might on quests.` between FEAR and DESTINY
  (`oracles.rs`); W7's played-winter renown literal `12` if a trait or family rule moves it
  (it should not: Maren and Brannoc are family); W6's and W8's batteries hold (they assert
  mainline rules the variant keeps) — run them and report, and if one moves, the rule that
  moved it is a Deviation to name, not a check to loosen · covers Done-when line 2.
- **G11 Floors** — `floors_x1::x1_surfaces`, at every size in `floors::SIZES`: the summer
  with Garrick and Brannoc on "Grave goods" and the card pointed at (the `PERSONAL:` sheet,
  scrolled whole); the garden with an unproven outsider; Garrick's death page at the choice
  with the candidate lines; a marked, traited hero's sheet; an outsider's sheet; counted
  into `floors::battery`'s surface total · covers §A.4.
- **G12 Pictures** — in `capture.rs`, four more: `keifu-x-inheritance-x1-personal.png` (the
  card and sheet of G1 before "Set out"), `keifu-x-inheritance-x1-unproven.png` (the garden
  of G4 at renown 4), `keifu-x-inheritance-x1-heirs.png` (G8's choice), and
  `keifu-x-inheritance-x1-sheet.png` (Maren's sheet after G8). Open each and name what you
  see in the PR · covers §A.7.
- **G13 The mutation round** — `mutants/x1.txt`: one fault per new constant (ten) and per
  rule line: the personal rule's `succeed|triumph` → `go` included and the family test
  dropped; step 7b before step 7 and after step 8; the mark identity without the year; the
  lapse at `>` instead of `>=`; the yearly drain over all marks instead of distinct; the
  reward's family gate inverted and the carriers counted over everyone; the tellers' family
  rule; UNPROVEN placed before WED_ALREADY; `<` → `<=` on the threshold; the dowry to the
  outsider instead of the house; the house name not changed; the heir filter dropped;
  `nearest_kin` for an outsider; the outsider's page waiting; each of the four trait
  branches; the power line before the heirloom line; the flaw as a gift; `inherit` dropping
  the dedup, the generation not incremented, traits merged from `from`; `choose(None)` to
  the chosen-less ground always; the page not waiting on marks; the candidate line's `(+n)`
  from the total. Run `python3 tools/mutate keifu-x-inheritance mutants/x1.txt --fast`,
  then `mutants/*.txt --fast --changed-since <the S0 commit>` for every earlier list over
  the files you changed; report `N of N noticed` and fix every escape before the PR ·
  covers §A.6.
- **G14 The web** — `python3 tools/build-web keifu-x-inheritance && python3
  tools/serve-web keifu-x-inheritance --check` passes · covers Done-when line 5.
- **G15 `cargo check --target wasm32-unknown-unknown`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo fmt --all`, `tools/check-game-deps`,
  `tools/check-assets`, `tools/test`** — all clean; `target/verify/report.json` is the
  verdict · covers the definition of done.

## Non-goals

Cut so the implement stage fits the window, or because the spec fences them:

- **Redeeming a mark** by a later triumph at its place: out. Marks lapse by years only.
- **Marks on the family screen, the remembrance or the epitaph**, a top-bar count, a
  card tint for the marked: out. The sheet, the quest sheet, the death page and the "Year N
  begins" page are the whole surface.
- **A burden making a quest personal**: out; only the own dream does.
- **Traits beyond the six**, traits read by training, old age, courtship or the Door, a
  trait passed at succession: out. Blood only, power only.
- **Outsiders' own marks, outsiders as heirs of any kind, an outsider's dream passing to
  the family**: out by rule (above), not by omission.
- **Any change to board generation, the forecast, the Door, the Ending, or the epitaphs.**
  The variant's renown movements flow through `House::add_renown` as mainline's do.
- **The `jidousha::ui` kit**: not adopted. Mainline keifu draws every screen through its
  own `screen::Page`; a fork that drew one line through the kit would have two ways of
  drawing. No engine change is needed anywhere in this design, so there is no FINDINGS
  entry naming a missing surface.
- **Mainline `games/keifu/`**: never written, by the spec's ruling.

## Decisions already made

Settled here; do not relitigate. The spec's three decision rows are elaborated below, each
naming its panel, its commit input, its one function and its gate; the design adds no
decision and drops none.

| decision | must know | surface (the panel) | action (the commit) | one function | asserted by |
|---|---|---|---|---|---|
| Whether to send a family member on a personal quest | the chance it fails (the card's setback + disaster, the sheet's `%2 in 100`); the mark a failure puts on the name and what it does now (-2 house, -2 them) and to heirs later (-1 a year through year N; it passes to the blood) | the quest card (`board_view::quest_rect(slot)`, the `Marked if it fails:` row) and its sheet in the dock (`summer::SHEET`, the `PERSONAL:` line), read at the moment of seating, mid-drag included | the set-out control, `Target::SetOut` ("Set out"), after seating by drag | `marks::personal(content, heroes, facts, party)` — the card, the sheet and resolution step 7b all call it; the failure's cost is `marks::mark_the_name(..)`, the one writer of a mark | G1, G2 |
| Whether to let an outsider marry into the family | the outsider's personal renown; the threshold (5); what marrying in changes (family, the name, the dowry, their children family, their quests paying the house) | the hearth's garden group (`hearth_view::group_rect(Group::Garden)`), its note `<name> unproven: renown R of 5` / `will wed` | "Let the winter pass", `Target::LetWinterPass` | `family::may_marry_in(heroes, a, b)` — `plans::courtship` reads it for the verdict and `winter::court` reads it to make the outsider family | G4, G5 |
| Which heir to choose, given what they inherit | each candidate's traits and the marks they would carry (`total (+taken)`), and where the marks fall on "No one" | the death page's choice block (`turning_view::lay_out_choice`): one line per candidate above the heir buttons | the heir button, `Target::Heir(page, Some(heir))` or `Target::Heir(page, None)` | `inheritance::inherit(heroes, dead, candidate)` — the preview, `heirs::choose` and `births::born` all call it | G8, G9 |

- **The copy is a pure copy and its own commit** (spec): every later diff reads as
  divergence from a known-identical crate. Reason: reviewability.
- **Family is a flag set at three sites** (founding, arrival, birth) and changed at one
  (marrying in). Reason: a predicate over house names would make the founders four
  families; the spec's distinction is family versus outsider, not Thorne versus Hale.
- **The personal rule is "own dream, needs it won"** rather than "any call". Reason: a
  "go" call (Ysolde's road at stage 1 calls every quest) would make nearly every seated
  dreamer's quest personal and a mark common; the decision has to be rare enough to be a
  decision.
- **A mark is worn for eight years whoever carries it, at one renown a year, and passes to
  the blood at death and birth.** Reason: a per-generation weight with a years clock is two
  rules where one suffices; eight years at -1 beside -2 at the failure is a cost on the
  order of two won quests, felt but survivable, and the player chooses the risk.
- **Marks pass with the heir choice, and to the blood on "No one"**, and a marked death
  page waits. Reason: the choice has to be able to move the shame for the preview to be a
  decision surface; shame that could be buried for free would always be buried.
- **The yearly weight counts distinct marks**, not carriers. Reason: a parent and a child
  carrying one failure is one failure.
- **Traits are six, one per aptitude, ±1 on quests, and the only genetics.** Reason: the
  aptitude average is mainline's and stays; a trait is visible on a card the first time it
  matters and nowhere else, so its whole cost is one power line.
- **The trait roll order is fixed** (above) and runs after mainline's birth rolls. Reason:
  mainline's roll sequence (§22.2) stays a prefix of the variant's, so a mainline seed's
  pronoun, name, aptitudes and fear are the same in the fork.
- **Founders' traits are authored, five of nine.** Reason: the oracles need a trait on the
  board and on a candidate in year 1 (Garrick, Maren); dead founders carry theirs so the
  reader sees descent.
- **Outsiders earn the house nothing on quests or at the table, cost it as anyone, and
  never inherit.** Reason: "cheap bodies" has to mean something on every surface renown
  moves through, or the threshold is a formality.
- **The threshold is 5 and the dowry half.** Reason: a wanderer arrives at 0..2; a won
  quest pays 2..5 personally; 5 is one good quest or a quest and two tales — reachable in
  two or three years, not the first winter.
- **New text is in the fork's content files under the existing keys' files**, read through
  `W`. Reason: mainline's one way to show a string.
- **Mainline oracles the variant breaks are rewritten to the variant's rule, each named a
  Deviation** (spec): W8 stirred, W4 `you bring`, W1's sheet, and whichever batteries
  move. Reason: the spec's instruction; a check loosened to pass both is a check of
  neither.

## Open calls delegated to the implementer

- **`X1_SEED`** (G1): choose by one off-screen sweep, ship as a literal with its outcome.
- **The exact wording of `lines.deed.marked`, `lines.death.outsider`, `lines.arrival.outsider`
  and `lines.mark.lapsed`**: write them in Lineage's voice; keep every other string above
  as given, since the checks copy them.
- **Where `settle_with_no_one` lives**: the code an outsider's death page and
  `heirs::choose(.., None)` share (bury the heirloom, raise the ghost) — factor it once in
  `heirs.rs` or `death_page.rs`, never twice.
- **`CHOICE_H` as a function of the lines drawn**, and whether the candidate lines take two
  columns when eight are offered: your call, under the floors (no row crowds a control, no
  two rows cross) and the long-page check.
- **How `Courtship::Unproven` carries the outsider**: `{ outsider: HeroId, renown: i32 }` is
  the shape this design names; if `Courtship::note` then needs the heroes to print the name,
  change its signature once and every caller.
- **Whether the card's `Marked if it fails:` row replaces or follows the `Dream:` row** on a
  card that is out of room at the web canvas (`floors::SIZES`): follow it if it fits at
  every size; otherwise fold the names into the dream row as `Dream: Garrick (marked if it
  fails)` and say so in the PR.
- **The `_variant` provenance notes** in the content files: one `_variant` key per file
  listing the added keys is enough.
- **Anything a battery reveals**: the W6, W8 and W10 batteries print distributions; if
  marks close houses far more often than mainline's survival shape, report the numbers in
  the PR under Findings and do not retune — the constants are this design's, and a change
  is the owner's.
