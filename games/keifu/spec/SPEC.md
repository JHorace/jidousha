# Lineage — gameplay specification

This is the rules contract for porting **Lineage** (shipped title; program title in
`build.jai:40`). It is written so that a reader who has never seen the source can build a
rules-identical game. Numbers live in [CONSTANTS.md](CONSTANTS.md); hand-authored words live
in [content/](content/README.md). Ambiguities are numbered in
[OPEN-QUESTIONS.md](OPEN-QUESTIONS.md) and referenced here as **OQ-n**.

## 0. Conventions

- **Provenance.** Paths are relative to `source/application/game/` unless they start with
  `source/`. `file:a-b` is a line range. Every rule cites where it lives.
- **`[emergent]`** marks behaviour that falls out of code structure (ordering, fallthrough,
  rank rules, clamps) rather than a stated rule. Port the observable behaviour.
- **Text keys.** `lines.<key>`, `epitaph.<key>`, `ui.<path>` refer to entries in
  `content/lines.json`, `content/epitaph.json`, `content/ui-text.json`. Every narrative
  line the rules emit has a key; the format convention (`%`, `%N`) is in
  `content/README.md`.
- **Order matters.** "In creation order" means the order heroes were created
  (`game.heroes`, `lineage/household.jai:11-30`). Almost every loop over the household walks
  that order, and many rules are order-sensitive. Ids are assigned 0,1,2... in creation
  order (`entity/entity.jai:27-28`).
- **Integer division** truncates toward zero everywhere (all operands are non-negative where
  it is used).
- **Base vs effective aptitude.** Base = stored value. Effective = base + phase adjustment,
  floored at 0 (`entity/entities/hero.jai:140-143`). Power reads effective; teaching,
  learning, newborns, "best aptitude" and coming of age read base.

## 1. What the game is

A house of heroes is kept for 25 years, one turn per year. Each year is: a **summer** in
which up to four quests are posted and the player drags heroes onto them; a **telling** that
shows what happened; a **winter** in which the player seats heroes at ten winter seats; and a
**turning of the year** in which everyone ages and births, deaths, comings of age and arrivals
happen. After year 25 comes **the last summer**: the only quest is the Sealed Door, three
locks tried by one party of up to four. Then an **ending**: a verdict and the family tree with
every hero's epitaph. The run also ends early if house renown reaches 0.

Heroes have three aptitudes, an age phase, one dream (three stages), one fear, one destiny
(prophecy), bonds to each other, at most one heirloom, blessings, scars and a record of deeds.
Death is permanent. What is inherited: aptitudes (a share), fears, blessings, heirlooms (by
player choice), unfinished dreams (by player choice) and the Door's promise.

There is **no saving**: a run lives only as long as the process (`save.jai:1-9` defines an
empty save that nothing calls; see INVENTORY.md).

## 2. Flow and screens

### 2.1 Scene state machine

```
            (release builds only)
 [Title] --any press--> [Summer] --Set out / Stay home / Try the Door--> [Telling]
                           ^                                                |
                           |                        Door open or renown<=0  |  otherwise
                           |                                 v              v
                     "Summer comes"                      [Ending]      [Winter]
                           |                                 |              |
                       [Turning] <----"Let the winter pass"--+--------------+
                                                             |
                                       "Begin another house" -> new run (new seed)
```

- Start of a run: found the household, prepare year 1's summer, enter Summer
  (`game.jai:147-164`). The title screen is shown first only in release builds
  (`game.jai:152-154`); any button or pointer press closes it (`game.jai:82-86`).
- Summer → Telling: the set-out button resolves the summer, then switches
  (`scene/scenes/summer.jai:39-49`).
- Telling → Ending if the Door stands open **or** house renown <= 0; otherwise begin winter
  (seat the hearth) and enter Winter (`scene/scenes/telling.jai:184-196`). This is the
  **only** place house closure is checked [emergent] (OQ-16).
- Winter → Turning: "Let the winter pass" resolves the winter and the whole turn of the
  year, then switches (`scene/scenes/winter.jai:39-42`).
- Turning → Summer: "Summer comes" (or Skip with no undecided heir) advances the calendar to
  next summer and prepares the board (`scene/scenes/turning.jai:45-74`,
  `lineage/passage.jai:114-117,126-138`).
- Ending → new run: "Begin another house" resets the whole game state and reseeds the RNG
  from the current game RNG (`scene/scenes/ending.jai:26-27,38-40`,
  `source/core/initialize.jai:78-113`).

Overlays that can open over Summer, Telling, Winter and Turning (all four draw the top bar
with the two buttons, `lineage/panels.jai:109-121`):

- **How to play** (`modal-screens/guide.jai`): two pages of eight entries
  (`ui.guide.entries`), "Next page"/"First page", "Close".
- **The family** (`modal-screens/family.jai`): the family tree plus a remembrance panel
  (§19). "Back to the house".
- **Pause** (`modal-screens/pause-menu.jai`): engine settings only (resume, scale,
  fullscreen, sfx and music volume, quit; demos in debug builds). Opened by a corner pause
  button when no keyboard/gamepad is present (`game.jai:211-221`).

While any overlay is open, nothing in the game advances (`game.jai:114-120`); the only thing
that advances with time anyway is the telling's typewriter.

### 2.2 The calendar

The calendar holds a year index (0-based) and a month that is used as the season
(`calendar.jai:16-21,30-33`). The game is in SUMMER (month 1) or WINTER (month 3).
`current_year = year index + 1` (`lineage/household.jai:7-9`). Begin winter sets the month
to WINTER in the same year; begin summer sets SUMMER and increments the year
(`calendar.jai:51-55`). The final day is 25 years after the start
(`calendar.jai:35-41`, `game.jai:13`); `years_until_door = final.year - today.year`
(`lineage/door.jai:52-58`). So: summers 1..25 are ordinary, the 26th summer is the Door's.
The turning in the winter of year 25 already announces "The Sealed Door stands open"
(`lineage/passage.jai:103-111`).

## 3. State

### 3.1 The house (`game.jai:47-71`)

| Field | Meaning |
| --- | --- |
| calendar | §2.2 |
| heroes | every hero ever created, in creation order (living, dead, departed) |
| mourned | heroes who died since the last turning and still await their death page |
| roster | 12 seats: "the household" in summer, "the hall" in winter |
| board | 4 quests |
| hearth | winter seats: fire[2], training[2] (learner, teacher), courting[2], tellers[2], benches[4] (child, teacher, child, teacher), yard[6] |
| tale | this summer's telling (prologue, quest pages, meanwhile lines) |
| passage | this turning's pages |
| places[7] | per place: visits, triumphs, disasters, trouble (0..2), fallen (heroes who died there) |
| patrons | number of heroes ever crowned |
| renown | house renown, starts 15, floored at 0 |
| writing | per-pool "last line used" memory and the last epitaph frame (§21) |
| tales | house tales (title, about whom, since) |
| ghosts | unfinished dreams left to no one (hero, dream copy, place) |
| blades_named | count of blades forged this run |
| locks_opened | Door result |
| generation | name/house bags (§17.1) and, per place, the last quest template used |

### 3.2 A hero (`entity/entities/hero.jai:1-56`)

| Field | Meaning |
| --- | --- |
| name, house, pronoun (HE/SHE) | identity; full name = "name house" |
| vocation | calling: Knight, Warrior (Might), Ranger, Scholar (Wits), Priest, Sage (Spirit) |
| age, born_year | |
| aptitudes[3] | base Might, Wits, Spirit, 0..9 |
| dream, burden | own dream and a carried (inherited) dream; a dream is "dreamt" iff its title is non-empty |
| fear | tag, dread 0..5, courage 0..3, conquered, broken, born_brave |
| destiny | kind, fulfilled, blood_of (name of the original Door promisee) |
| bonds | ordered list of bonds to others (§12) |
| heirloom | name (empty = none), sprite, aptitude, bonus, provenance |
| blessings | list (title, scope, tag/place, power) |
| scars | list of strings |
| deeds | ordered list (kind, year, age, place, weight, other, telling) |
| legacy | kind (NONE/HEIRLOOM/TALE/BLESSING) + telling (name of what was left) |
| renown | personal renown |
| wounded, settled | |
| fate | LIVING, DEAD, DEPARTED (crowned); fate_year, fate_age, fate_telling |
| death | how a death happened: telling, questing (bool), place, tag |
| grieved | grief already applied |
| bequest | decided, heir, heirloom name, dream fate (UNDECIDED, NEVER_DREAMT, FULFILLED, PASSED_ON, LEFT_TO_NO_ONE, LAID_TO_REST), laid_year |
| wording, epitaph | epitaph frame/variants and the composed text (§20) |
| parents[2] | may be null |
| roads_walked | set of places quested at |
| quests_faced, fears_faced, winters_taught | counters |

Derived (`entity/entities/hero.jai:85-151`, `lineage/lore.jai:96-102`):

- **Phase** from age: Child <12, Youth 12-19, Prime 20-39, Veteran 40-54, Elder 55+.
- **Adult** iff age >= 12. **Living** iff fate == LIVING.
- **Best aptitude**: highest base value; ties go to the lower index (Might, then Wits, then
  Spirit) (`entity/entities/hero.jai:145-151`).
- **Kin** (`lineage/bond.jai:146-155`): a PARENT/CHILD bond between them, or they share any
  non-null parent. Nothing else is kin: grandparents, aunts, uncles and cousins are not
  [emergent] (OQ-11).
- **Descends from** (`lineage/legacy.jai:103-109`): via parents, recursively.
- **Firstborn** (`lineage/bond.jai:137-144`): among the hero's CHILD bonds (living or dead),
  the one with the smallest born_year (first found on ties).

## 4. The founding (`lineage/household.jai:57-225`)

House renown = 15. Nine heroes are created in this order, with exactly the values in
`content/household.json`: Elsbeth (dead), Garrick, Maren, Pip, Ysolde, Brannoc, Odo, Aud
(dead), Wren. Notable details:

- Elsbeth is DEAD (fate year -18, "was lost at the Drowned Coast") and is listed among the
  Drowned Coast's fallen. Aud is DEAD (fate year -5). Both are marked grieved and decided, and
  get an epitaph wording roll and an epitaph at founding (Elsbeth first)
  (`lineage/household.jai:214-225`).
- Garrick holds **Thornfall** (+1 Might). His destiny is CHILD_WILL_SURPASS_YOU, already
  fulfilled. His dream is QUIET_THE_BARROW at stage 3 (index 2; the first two stages count
  as done).
- Maren's dream is AVENGE_THE_LOST built for Elsbeth (Drowned Coast, Water), at stage 2.
  Elsbeth's own SEE_THE_SEA is at stage 2; Aud's SEE_A_CHILD_GROWN at stage 3.
- Pip (10) and Wren (8) are children. Pip already dreams SEE_THE_SEA; Wren has no dream.
  Neither has a vocation set, so it is the default **Knight** until coming of age replaces it
  (invisible: children show as children).
- Bonds, in formation order: Brannoc-Aud spouses; Wren's parents Brannoc and Aud; Garrick-
  Elsbeth spouses; Maren's parents Garrick and Elsbeth; Pip's parent Maren; Garrick-Odo
  friends; Brannoc-Ysolde companions then rivals.
- Year 1's board is half authored (§5.2): "Grave goods" (Barrow) and "The bell under the
  tide" (Drowned Coast).

`advance_dream_to_stage(d, s)` sets the current stage to s and marks every earlier stage's
count as its goal (`lineage/dream.jai:295-298`).

## 5. Summer

### 5.1 Preparing a summer (`lineage/passage.jai:126-138`)

If the Door stands open: the board is cleared and slot 0 becomes the Might lock of the Door
(4 seats, §16). Otherwise the board is generated (§5.2), with the two opening quests forced in
year 1. Then the household is reseated: the roster is cleared and every living adult is
pushed into the first free roster seat in creation order; all quest seats and all hearth seats
are emptied (`lineage/household.jai:32-40`). Children are not seated in summer; they are shown
in "the yard" (first six, creation order) and cannot be dragged
(`scene/scenes/summer.jai:65-70`).

### 5.2 Board generation (`generation/quest-context/quest-context.jai:54-217`)

The board is the best of up to 16 planned boards.

**Planning one board** (`:161-190`):
1. Available places = the six questing places (not the Door).
2. Each forced (opening) quest is generated from its named template and its place removed.
3. If there is at least one ghost and fewer than 4 quests are planned, take **the first ghost
   in the ghost list** (§14.4). If its place is still available, add its ghost quest (§14.4)
   and remove the place. Only that one ghost is considered; others wait.
4. While fewer than 4 quests: draw a place uniformly without replacement from the remaining
   places; pick a template uniformly among that place's four templates **excluding the
   template used at that place on the last board** (if one is remembered); generate it.

**Generating a quest from a template** (`:192-217`, `lineage/quest.jai:45-53`): title,
premise, aptitude, tags and four endings come from the template (`content/quests.json`) —
note a quest's tags are the template's, which may be a subset of its place's tags. Then:

```
calm_seats  = uniform(seats_low .. seats_high)
trouble     = the place's current trouble (0..2)
seats       = max(calm_seats - trouble, 1)
danger      = min(template danger + trouble, 4)
renown      = template danger + trouble
demand      = seats * (3 + template danger + trouble + (year - 1) / 6)
demand     += uniform(-1, 0, +1)
```

**Reading a board** (`:93-116`): for every ordered pair of distinct quests (i, j): choose a
likely party for i from all living adults, then a likely party for j from the adults not in
i's party; take `min(success_i, success_j)` where success is P(SUCCESS or TRIUMPH) of the
forecast (§6) with the house's patrons. *Answerability* = the maximum over pairs; the pair
that first attains it is remembered (first, second). The board *calls a dreamer who could go*
if any quest does (§9.6).

A **likely party** (`lineage/quest.jai:100-126`): start from an optional leader; repeatedly
add the candidate (in candidate order) with the highest **solo** forecast power (patrons 0),
skipping the wounded, those who refuse the quest (§10.4) and those already chosen; ties keep
the earliest; stop when seats are full or no candidate remains.

Because success-or-better is always k/36 with k in {0,1,3,6,10,15,21,...}, a quest's chance is >= 0.5 exactly when the party's power >= demand, and >= 0.35 exactly when power >= demand - 1 (CONSTANTS §4).

**Choosing** (`:61-73`): `score = min(answerability / 0.5, 1) * 2 + (1 if calls a dreamer)`.
Plan up to 16 boards; keep a new plan only if its score is strictly higher than the kept one;
stop early as soon as the kept board is *welcome* (answerability >= 0.5 and calls a dreamer).
Before each attempt the "last template per place" memory is restored, so attempts are
independent.

**Easing** (`:75-77,150-159`): if the kept board's answerability < 0.5 and it has at least two
quests, repeat up to 30 times: recompute the remembered pair's chances (parties re-chosen
each time); stop if both >= 0.5; otherwise lower the demand of the weaker quest of the pair
(the first quest of the pair unless its chance is not lower than the second's) by 1, unless
its demand is already <= its seats (then stop).

**Finishing** (`:79-90`): the per-place template memory becomes: the remembered memory,
overwritten for each non-ghost quest on the kept board with that quest's template. The quests
are sorted by place (Barrow, Coast, Pass, Emberfall, Court, Deepwood) and fill board slots
0..3. This sorted order is the resolution order (§7).

### 5.3 Seating (`scene/scenes/summer.jai:18-50`, `lineage/panels.jai:266-321`)

- Seats are drag-and-drop slots. The roster has 12; each offered quest has exactly its
  `seats` slots.
- Dragging a hero onto an empty slot moves them; onto an occupied slot **swaps** (the
  displaced hero goes to the slot the dragged hero came from); releasing anywhere else returns
  the hero to where they came from (`source/core/ui/slot-group.jai:13-59`).
- Any adult may be seated on any quest, including the wounded. After every frame, any hero
  sitting on a quest they **refuse** (broken fear of one of its tags, §10.4) is sent back to
  the first free roster seat (`scene/scenes/summer.jai:153-170`).
- The set-out button reads "Set out" if anyone is seated, "Stay home" if nobody is, and "Try
  the Door" in the last summer, where it is disabled until at least one hero is seated
  (`scene/scenes/summer.jai:39-46`).

### 5.4 What the player sees in summer

Gameplay-level only; layout is free.

- **Top bar** (`lineage/panels.jai:331-372`): "Year N of 25" (or "The last summer"),
  season, house renown (warning colour at <= 4; hover explains renown and shows the current
  wanderer chance), the Door countdown, the Door's tags and the three lock numbers, and "Your
  best four today bring M, W, S: all three open P in 100" (§16.4). The Door hover names the
  best four.
- **Household and yard**: hero cards (name, age, sprite by phase/vocation, tint: dead grey,
  wounded red, elder grey; one pip per point of dread; a gold pip if settled)
  (`entity/entities/hero.jai:165-198`).
- **Quest card** per offered quest (`lineage/quest-card.jai:63-164`): place, title, tags
  (highlighted when a seated or pointed-at hero fears them), danger pips, "Needs <Aptitude>
  <demand>", "you bring <power>" (if anyone seated), an odds bar and the four outcome
  percentages; with nobody seated it shows the place's trouble line if troubled, else "No one
  is going. Room for N."; "Renown +R" and "unanswered -C" (C computed with the current house
  renown); then either "The ghost of <name>" or "Dream: <names>" for every adult whose dream
  (or burden) this quest would advance given the current party (§9.6); then either
  "<watched> will not go." if the pointed-at hero refuses it, or "Fear: <name> -<penalty>
  [steadied], ..." for seated heroes who fear it.
- **Live preview**: while a hero is being dragged over a quest card that has room and they
  would not refuse, the card's forecast includes them (`lineage/quest-card.jai:31-46`).
- **Quest sheet** (point at a card) (`lineage/quest-card.jai:166-240`): premise; trouble
  stakes text if troubled; for each called adult, "<Name>'s dream|burden: <current stage
  task>. <He> must <go|succeed|triumph|stay behind>."; the full power breakdown (§6); "Two
  dice, less 7, are added to that."; each outcome with its percentage and consequence; the
  unanswered cost; and the place's history (visits, triumphs, disasters, and every hero who
  fell there with their fate telling).
- **Hero sheet** (point at a hero) (§19.1).
- **Door card and Door sheet** in the last summer (§16.4).

## 6. Power and odds (`lineage/quest.jai:156-224`)

For a party P (seated heroes in seat order) on quest Q:

Per member m, the lines added in this order (a line of 0 is omitted):
1. effective aptitude of m in Q's aptitude;
2. heirloom bonus, if m holds an heirloom of Q's aptitude;
3. `-(2 + dread/2)` if m fears a tag Q carries and the fear is not conquered (broken fears
   still count here);
4. `+2` if m's fear is conquered (including born brave) and Q carries that tag;
5. `-2` if m is wounded;
6. `+5` if Q is a Door lock and m's destiny is OPEN_THE_SEALED_DOOR (including "blood of");
7. `+power` for each of m's blessings that applies (against a tag Q carries; at Q's place;
   or everywhere).

**Floor**: if m's lines sum to less than 0, an extra line raises m's contribution to exactly
0 ("can do no worse than nothing").

Then, for every pair (i < j in party order) with a bond that is not COMPANION: add that bond
kind's power (friend +1, mentor/student +1, spouse +2, parent/child +2, rival -1). Bonds are
not floored. Then, if the party is non-empty and patrons > 0: `+patrons`.

`power` = the sum. The forecast enumerates all 36 dice pairs and buckets
`margin = power + d1 + d2 - 7 - demand` into DISASTER (<= -5), SETBACK (-4..-1), SUCCESS
(0..3), TRIUMPH (>= 4), each pair weighing 1/36. An empty party has all chances 0. The
displayed and the rolled outcome use the same formula (table in CONSTANTS.md §3).

**Odds can move between "Set out" and the roll** [emergent] (OQ-6): quests are resolved in
board order, each forecasting at the moment it resolves. Earlier quests in the same summer
can change later ones: a death grieves (adds dread, can break) heroes seated on later quests;
a crowning adds a patron to every later quest; house renown changes alter later unanswered
costs.

## 7. Summer resolution (`lineage/tale.jai:46-80`)

On set out:

1. Reset the telling; record the year.
2. If the Door stands open: resolve the Door (§16.2) and stop here — no unanswered costs, no
   healing at home.
3. For each board slot in order (sorted by place), if offered:
   - **No one seated**: the quest is unanswered (§7.2); its cost is accumulated.
   - Otherwise resolve it (§7.1) on a new quest page.
4. If the accumulated unanswered cost > 0: house renown -= cost; meanwhile line
   `lines.summer.unanswered_total`.
5. Every hero still sitting in a roster seat who is wounded is healed:
   `lines.summer.mended_at_home` (roster seat order).

### 7.1 Resolving one quest (`lineage/tale.jai:93-161`)

In exactly this order:

1. Forecast with the current party and the house's patrons (§6).
2. Roll two dice (uniform 1..6 each). `margin = power + d1 + d2 - 7 - demand`; outcome by
   the bands in §6. The page records power, dice, margin, outcome, members, and the story =
   the quest's ending for the outcome with `%1` = the party's first names joined (name list,
   §21) (the Door writes its own story, §16.2).
3. Place history: visits +1; triumphs +1 or disasters +1 if so.
4. For each member in order: quests_faced +1; the first time ever, record a FIRST_QUEST deed
   (place, weight = danger).
5. On SUCCESS or TRIUMPH: **reward** (§7.3).
6. Ease the place's trouble: SUCCESS/TRIUMPH set it to 0; SETBACK/DISASTER lower it by 1
   (floor 0).
7. On DISASTER: house renown -= danger; line `lines.quest.disaster_renown` (not at the Door).
8. For each member in order: **face the fear** (§10.1).
9. On SETBACK: pick one member uniformly at random ("unlucky"). Every member other than the
   unlucky one whom the fire destiny claims (§13) is burned; then if the fire destiny claims
   the unlucky one they are burned, otherwise they are **wounded** (§7.4). So on a Fire
   quest's setback, *every* fire-doomed member dies [emergent].
10. On DISASTER: each member in order **suffers the disaster** (§7.4).
11. Unless this is the Door: for each pair (i < j) of members **both still living**, they
    **share the road** (§12.2).
12. Every living hero in the house (creation order, members or not, children included)
    witnesses the quest as a dream moment (§9.3). Present = they were in the party.
13. Every member (living or dead) adds this place to roads_walked.
14. On TRIUMPH: each living member whom the crown claims (§13) is crowned.
15. If this is a ghost quest and the outcome is SUCCESS or TRIUMPH: the ghost is laid (§14.4).

### 7.2 Unanswered (`lineage/tale.jai:82-91`, `lineage/quest.jai:41-43`)

`cost = 1 + (1 if the quest was generated with trouble > 0) + house_renown / 20`, using the
house renown **at the moment this quest is processed** (earlier quests this summer may have
changed it) [emergent]. The place's trouble rises by 1 (max 2). A meanwhile line
`lines.summer.unanswered` gives the place's trouble line (`content/lore.json`
`places[].trouble_line`, filled with `year_counts[new trouble]`: "a year", "two years").

### 7.3 Reward (`lineage/tale.jai:171-232`)

- `renown = quest renown (+1 on TRIUMPH)`; `carried = number of CARRY_THE_HOUSE members`.
- House renown += renown + carried.
- Not at the Door: line `lines.quest.reward`; one `lines.quest.carrier_bonus` per carrier.
- Every member: personal renown += renown (carriers get no extra personally). On TRIUMPH and
  not at the Door, each member records a TRIUMPH deed (place, weight = danger).
- **Triumph lesson** (TRIUMPH, not at the Door): among members who can still grow in the
  quest's aptitude (base < 9 and may still learn, §13), the one with the **lowest base** in
  that aptitude (earliest in party order on ties) gains +1 (`lines.quest.lesson`).
- **Youth lesson** (SUCCESS or TRIUMPH, not at the Door): each member in Youth phase who can
  still grow and whose base in the quest aptitude is < 5 (checked after the triumph lesson)
  gains +1 (`lines.quest.lesson`). A youth can gain twice in one triumph [emergent].

### 7.4 Wounds, deaths, mending, burning

**Wound** (`lineage/tale.jai:234-254`), skipped for the dead:
- Already wounded and the destiny shields on quests (FIRE_WILL_END_YOU or DIE_IN_YOUR_BED):
  nothing but `lines.quest.wound_shielded`.
- Already wounded otherwise: dies (`lines.quest.second_wound`; fate
  `lines.fate.died_of_wounds`).
- Otherwise: wounded = true; a WOUNDS pool line; WOUNDED deed.

**Suffer a disaster** (`lineage/tale.jai:265-296`), skipped for the dead:
1. Fire destiny claims → burned; stop.
2. BREAK_AND_BE_MENDED and not yet fulfilled → mended; stop (no death roll).
3. Roll death with chance `min(danger * 0.15, 1)`.
4. If the roll hits and the destiny does not shield on quests: dies
   (`lines.quest.death` with a QUEST_DEATHS pool line; fate `lines.fate.fell`); stop.
5. If the roll hit but was shielded: line `destiny.shielded_bed` or `destiny.shielded_fire`.
6. Wound (above) — so an already-wounded unshielded hero who survived the roll still dies.
7. If still living: SURVIVED_DISASTER deed.

**Mended** (`lineage/tale.jai:298-315`): destiny fulfilled; wounded = true (even if already
wounded; no death); dread +2 through the normal dread rule (§10.2, may break them); every base
aptitude +1 (cap 9); scar `lines.scar.mended`; line `lines.quest.mended`; MENDED deed.

**Burned** (`lineage/tale.jai:256-263`): `lines.quest.burned`; destiny fulfilled; dies with
fate `lines.fate.burned`.

**Crowned** (`lineage/tale.jai:317-352`): destiny fulfilled; fate DEPARTED (fate year/age,
`lines.fate.crowned`); patrons +1; removed from every seat; `lines.quest.crowned`; CROWNED
deed. If they hold an heirloom: the heir is their nearest kin (§15.3); the bequest records the
heirloom, the heir and decided. With an heir: if the heir already holds an heirloom, that one
is **discarded** (`lines.crown.heir_lays_aside`, OQ-5); the heir receives it
(`lines.crown.heirloom_left`); the crowned hero no longer holds it. Without an heir: they keep
it (`lines.crown.heirloom_taken`). Then an epitaph wording is rolled and the epitaph composed
(it is never recomposed afterwards). The crowned are not mourned, not grieved, never offered
as heirs, and their unfinished dream simply ends (OQ-4).

**Death** (`lineage/tale.jai:361-389`): fate DEAD with year, age at death and fate telling;
the death's place/tag (questing deaths: the quest's place and its first tag); a questing death
adds the hero to the place's fallen; removed from every seat; added to `mourned`. If
CARRY_THE_HOUSE: house renown -4 (with `lines.death.carrier_loss` only when there is a page
to write it on, i.e. quest deaths; an old-age carrier death loses the renown silently
[emergent]). Quest deaths are grieved immediately on the quest page (§12.5); old-age deaths
are grieved on their death page.

## 8. The telling (`scene/scenes/telling.jai`)

Pages, in order (`:145-172`): the Door prologue (last summer only), one page per resolved
quest in board order, then "Meanwhile" (unanswered lines, unanswered total, home healing).
Long pages split into continuation leaves; if there is nothing at all, a "A quiet summer"
page (practically unreachable, see INVENTORY.md).

A quest page shows: place, title, premise, the members' cards, the story (typed out at 90
letters per second; the next-button first completes it), then the outcome word, "Needed
<Apt> <demand>. Brought <power>. Dice a and b, less 7: <margin telling>.", then the page's
lines in the order they were produced. If the house has closed, the Meanwhile page ends with
`ui.telling.house_closed`.

Navigation: "Go on" (reveal, then next leaf), numbered page buttons (free navigation),
"Skip ahead" (leave immediately). The last leaf's button reads "Winter comes", "After the
Door" (Door open) or "The last of it" (house closed). Leaving goes to Winter or Ending
(§2.1). Nothing is decided on this screen.

## 9. Dreams

### 9.1 Model (`lineage/dream.jai:1-57,186-291`)

A dream has a kind, a setup (place, tag, lost hero — used by AVENGE_THE_LOST), an optional
**owner** (the hero who first dreamt it; set when a dream is passed on or raised as a ghost),
a title, three stages and a current stage index (3 = fulfilled). Each stage has a task text,
a requirement tree (ALL of predicates, or one predicate), a goal (1, or 2/3 for some) and a
count. The nine dreams, their stages and requirements are in `content/dreams.json`.

Requirement predicates over a **moment** (`lineage/dream.jai:110-178`):

| Predicate | True when |
| --- | --- |
| went_to_place(p) | quest moment, present, quest place == p |
| faced_tag(t) | quest moment, present, the quest carries t |
| fared_at_least(o) | quest moment, present, outcome >= o (order DISASTER < SETBACK < SUCCESS < TRIUMPH) |
| walked_a_new_road | quest moment, present, place is not the Door, and the hero has not quested at this place before (roads_walked is updated after the witness) |
| walked_every_road | quest moment, present, not the Door, and roads_walked plus this place covers all six questing places |
| spent_the_winter(a) | winter moment with action a |
| taught_the_young | winter moment with action TEACH or MIND_A_CHILD |
| stood_beside_a_student | quest moment, present, and someone in the party is a student the hero taught (the hero's bond to them has taught = true) |
| student_fared_alone | quest moment, **not** present, outcome >= SUCCESS, and someone in the party is a student the hero taught |
| renown_is_at_least(n) | the hero's personal renown >= n — **any** moment kind |
| is_wed | the hero has a SPOUSE bond (living or dead) — any moment |
| has_a_child | the hero has a CHILD bond (living or dead) — any moment |
| has_a_grown_child | the hero has a living adult child — any moment |

The AVENGE_THE_LOST title is "To avenge " + "my father"/"my mother" by the lost hero's
pronoun ("the lost" if none); its second and third tasks name the setup place and tag.

### 9.2 Telling titles and tasks (`lineage/dream.jai:67-92`)

- **Told title**: the title with its first letter lowered, and every " my " replaced by
  " his "/" her " of the dream's owner (or the bearer if no owner).
- **Told task**: lowered first letter; a trailing " you" becomes " him"/" her" of the hero it
  is told about.
- **Progress**: the told task, plus " (count/goal)" when goal > 1 (count capped at goal).

### 9.3 Moments and witnessing (`lineage/tale.jai:391-454`, `lineage/dream.jai:307-319`)

Moments are offered:
- after every resolved quest, to **every living hero** (§7.1 step 12);
- for each winter action taken by an adult (§11.3);
- at every turning, to every living hero (§18 step 9) (YEAR_TURN; only the "any moment"
  predicates can match).

Witnessing a moment: test the hero's **own dream**'s current stage; if it matches, count +1;
if the count reaches the goal, the dream advances one stage (fulfilled at 3). **If the own
dream made any progress (counted, stage done or fulfilled), the burden is not tested this
moment**; otherwise test the burden the same way. So at most one stage of one dream moves
per moment.

Because the "any moment" predicates hold at every moment once true, such stages complete at
the next moment of any kind, and several such stages can complete in one summer (one per
resolved quest) [emergent].

Progress lines: counted `lines.dream.counted`; stage done `lines.dream.stage_done` + a
DREAM_STEP deed; fulfilled §9.4.

### 9.4 Fulfilment (`lineage/tale.jai:434-452`)

The hero becomes **settled** (permanently), dread drops to 0, a line
(`lines.dream.fulfilled_owned` if the dream has an owner, else `lines.dream.fulfilled`), a
DREAM_FULFILLED deed, and the dream **leaves its legacy** (§14). The legacy is named for the
*dreamer* (owner, else the fulfiller) but given to/placed by the fulfiller.

Settled heroes can no longer gain dread (§10.2), but can still gain courage and conquer
their fear [emergent] (OQ-8).

### 9.5 Getting a dream

- Founders: authored. Wanderers: rolled on arrival (§17.2).
- Children: none, unless handed one by an heir choice (§15.2).
- At coming of age, in this priority (`generation/hero-context/hero-context.jai:179-223`):
  1. already has a dream (own or passed on) → keep it;
  2. a parent (parent 0 checked before parent 1) is DEAD by a questing death not at the Door
     → AVENGE_THE_LOST for that parent, setup = the parent's death place and death tag (the
     quest's first tag);
  3. a ghost whose hero is an ancestor (first such in the ghost list) → take the ghost's
     dream (keeping its owner and stage); the ghost is removed (§14.4); the dead hero's
     bequest becomes PASSED_ON to this hero and their epitaph is recomposed;
  4. otherwise roll a dream (§17.4).
  Then dream rivalry is checked (§12.3).

### 9.6 Dream calls (which quests "call" a dreamer) (`lineage/dream.jai:328-410`)

For hero h, quest q and a party P, check h's own dream first; if it does not call, check the
burden (a call from the burden is marked "burden"). For one dream with current stage s:

1. No current stage → no call.
2. If s is already met by an idle year-turn moment (an "any moment" predicate already true)
   → no call (it will complete on its own).
3. `going` = h is in P. If not going and s would be met by a quest moment *anywhere* (a
   fictitious quest at the Door with no tags, outcome TRIUMPH, h present alone) → no call.
   (In practice this excludes only "Carry it to a triumph": it calls only the quest the
   dreamer is seated on.)
4. Test s against a present moment with party P ∪ {h} and outcomes DISASTER, SETBACK,
   SUCCESS, TRIUMPH in that order; the first outcome that satisfies it is what is *needed*.
   If one does, or if h is going → return (calls = found).
5. Otherwise test an absent moment (h not present, party P) the same way; if satisfied it is
   an "apart" call.

Telling of a call: apart → "stay behind"; needs TRIUMPH → "triumph"; SUCCESS → "succeed";
otherwise "go". Cards list every adult whose call holds for the current party; the board
reader uses the call with an empty party (§5.2):

**Calls a dreamer who could go** (`generation/quest-context/quest-context.jai:118-132`): some
living adult d has a call (empty party) that is not "apart", is not wounded, does not refuse
q, and the likely party built with d as leader (from all adults) has P(SUCCESS or better) >=
0.35.

## 10. Fears

### 10.1 Facing the fear (`lineage/fear.jai:88-126`)

For each member m of a resolved quest (Door locks included), in party order:
1. Skip unless m's fear is not conquered, the quest carries m's tag, and m is not broken.
2. fears_faced +1.
3. `companion` = the first *living* other member whose bond with m steadies (power > 0:
   friend, spouse, mentor, student, parent, child).
4. If the outcome is SUCCESS/TRIUMPH and there is a companion: courage +1
   (`lines.fear.courage` with a COURAGES pool line); if courage >= 3 the fear is conquered
   (§10.3). Stop. (Settledness is not checked here.)
5. If m cannot dread (settled, conquered or broken): stop.
6. `amount = 1 (+1 if the outcome was SETBACK/DISASTER) (-1 if companion)`. If <= 0:
   `lines.fear.steadied`; stop.
7. dread = min(dread + amount, 5); `lines.fear.dread` (DREADS pool); if dread >= 5 the hero
   breaks (§10.4).

### 10.2 Dread from other sources

All go through one rule (`lineage/fear.jai:40-45`): if the hero can dread (not settled, not
conquered, not broken) and amount > 0, dread = min(dread + amount, 5); at 5 they break.
Sources: mending (+2), parent/child failure (+1 each, §12.2), grief (§12.5).

**Shedding** (`lineage/hearth.jai:324-340`): resting by the fire sheds 1 (not if broken).
Fulfilling a dream sets dread to 0. Conquering sets it to 0.

Children can gain dread from grief and can break before they ever quest [emergent] (OQ-9).

### 10.3 Conquering (`lineage/fear.jai:68-78`)

conquered = true, dread = 0, CONQUERED_FEAR deed, `lines.fear.conquered`. From then: no
penalty, +2 on quests with the tag, no dread from any source, and children may be born brave
(§17.3).

### 10.4 Breaking (`lineage/fear.jai:51-66`)

broken = true; a scar ("Broken by grief for X" if caused by grief, else "Broken by <tag
noun> <occasion>"), a BROKEN deed (with `other` = the mourned when grief), and
`lines.fear.broken`. A broken hero **refuses** every quest carrying the tag: they are bounced
from such seats (§5.3) and skipped when likely parties are built. Their fear still costs
power if they are somehow on such a quest (only possible if they break during the Door, §16.2).
Children of a broken parent inherit the fear at 80% (§17.3).

## 11. Winter

### 11.1 Opening the hearth (`lineage/hearth.jai:190-207`)

Reseat the household (§5.1: roster = living adults in creation order, all hearth seats
empty), then put the first six living children (creation order) into the six yard seats,
then move up to two **wounded** heroes from the roster (roster seat order) to the two fire
seats.

### 11.2 Seating

Every winter seat — roster ("the hall"), yard, fire, training (learner, teacher), garden,
long table, benches (child, teacher x2) — is a drag-and-drop slot with the swap rules of
§5.3 (`scene/scenes/winter.jai:16-43`). **Any hero can be put in any seat**; eligibility is
judged only when the winter resolves. Heroes left in the hall or yard do nothing.

The winter screen previews each seat's result (`scene/scenes/winter.jai:48-138`): fire —
"heals"/"calms"/"rests"; training — the planned lesson ("+N Aptitude" or the excuse);
garden — the courtship verdict; table — "house +" for the first adult, "own +" for the
second, "a child" for children; benches — the planned lesson or excuse.

### 11.3 Resolving the winter (`lineage/hearth.jai:209-287`)

Happens when the player lets the winter pass, **before** anyone ages. In order:

1. **Fire** (seat order): rest (heal the wound; shed 1 dread unless broken; one of three lines
   with a RESTS pool line, or nothing if neither applied). If the hero is an adult: winter
   moment REST.
2. **Training yard** (learner L, teacher T):
   - L present: plan the yard lesson (§11.4) and train L; then if T is present **and adult,
     credit T for teaching** (§11.5) — even when the lesson was wasted [emergent] (OQ-21).
   - Only T present: `lines.winter.waits_for_learner`.
3. **Benches**, bench 0 then bench 1 (child C, teacher M):
   - C present: plan the bench lesson and train C; credit M only if M is present and the
     lesson's amount > 0 (action MIND_A_CHILD).
   - Only M present: `lines.winter.bench_no_child`.
4. **Garden** (seat A, seat B), courtship verdict (§11.6):
   - WILL_WED: they become spouses (bond formed this year); a WEDDINGS pool line; WED deeds.
   - RIVALS: both bonds become FRIEND (`lines.winter.rivals_make_peace`).
   - any other verdict except NOBODY: `lines.winter.courting_failed` about the first occupied
     seat.
   - Then winter moment COURT for each adult occupant (no dream uses it).
5. **Long table** (seat order): each **adult** teller gains +1 personal renown and records
   TOLD_THE_TALE; the first adult teller also gives the house +1 renown
   (`lines.winter.tale_told` with a TALES_TOLD line); a second adult teller gets
   `lines.winter.tale_told_again`. Each adult teller: winter moment TELL_THE_TALE. Children
   at the table are ignored entirely.

**Training** (`lineage/hearth.jai:289-304`): if the lesson's amount > 0, base aptitude +=
amount (`lines.winter.trained`); otherwise the lesson's wasted line (`ui.winter.lesson_excuses`
`*_wasted`, `%1` learner, `%2` teacher). Then, if L is an adult: winter moment TRAIN — **even
if nothing was learned** [emergent] (OQ-20).

### 11.4 Lessons (`lineage/hearth.jai:37-135`)

**Yard lesson** (training yard): a child learner → wasted "belongs on a bench". Otherwise
the general plan below.

**Bench lesson**: an adult in the child seat → wasted "is no child"; no teacher → wasted
"needs a teacher"; otherwise the general plan below.

**General plan** (learner L, teacher T or none), checks in order:
1. T present and T is a child → wasted "a child cannot teach".
2. L may not learn (§13: TEACH_A_GREATER, or CHILD_WILL_SURPASS_YOU with an adult firstborn)
   → wasted "learns nothing more".
3. No T:
   - L is a child → wasted "needs a teacher" (unreachable through the seats).
   - aptitude = L's vocation aptitude; known = L's base in it.
   - known >= 6 → wasted "needs a teacher now".
   - amount = 1 (+1 if L is a Youth), at most 6 - known.
4. With T: aptitude = T's best aptitude (base); known = L's base; taught = T's base in it, or
   9 if T is TEACH_A_GREATER.
   - taught <= known → wasted "teacher knows no more".
   - L is a child: age < 6 → wasted "too young to learn"; known >= 4 → wasted "is a child
     yet"; else amount = 1.
   - L is an adult: amount = 1 (+1 if T is Veteran/Elder) (+1 if L is a Youth), capped at 2;
     then +1 if T is TEACH_A_GREATER; then at most taught - known; then at most 9 - known.

### 11.5 Crediting a teacher (`lineage/hearth.jai:306-322`)

1. "Newly bound" = T and L had no bond at all.
2. Form a bond L→T of kind MENTOR (T→L becomes STUDENT) **subject to the rank rule**
   (§12.1): it replaces companion (0), friend or rival (1); it does not replace spouse (3) or
   parent/child (4).
3. T's bond to L gets **taught = true** whatever its kind.
4. T's winters_taught +1; the first time, a TAUGHT deed.
5. If newly bound: `lines.winter.new_student`.
6. Winter moment for T: TEACH (training yard) or MIND_A_CHILD (bench).

Consequences [emergent]: a parent teaching their own child sets taught but forms no MENTOR
bond, so does not count as the child's "first teacher" at coming of age (OQ-10). Teaching a
rival turns the rivalry into mentorship.

### 11.6 Courtship (`lineage/hearth.jai:156-169`)

Checks in order: both seats empty → NOBODY; one empty → WAITING; kin → KIN; either < 18 →
TOO_YOUNG; age difference > 15 → TOO_FAR_APART; they are rivals → RIVALS; either already has
a **living** spouse → WED_ALREADY; else WILL_WED. Widowed heroes may remarry.

## 12. Bonds

### 12.1 Forming and changing (`lineage/bond.jai:84-121`)

Bonds are stored on both heroes, mirrored (MENTOR↔STUDENT, PARENT↔CHILD, others symmetric).
"A's bond to B is PARENT" means B is A's parent. Each bond records kind, other, since (year
formed or last changed), taught (on the teacher's side) and shared successes.

**Form(A, B, kind)**: if A == B, nothing. If no bond exists, append to both lists. If a bond
exists, replace it only if the new kind's rank is strictly higher (ranks: companion 0;
friend, rival 1; mentor, student 2; spouse 3; parent, child 4), keeping the bond's place in
each list. **Change(A, B, kind)** overwrites both sides regardless of rank and sets since.

### 12.2 Sharing the road (`lineage/bond.jai:163-219`)

For each pair of living members after a non-Door quest:
1. Form a COMPANION bond if they have none.
2. On SUCCESS/TRIUMPH: shared successes +1 (both sides). If they are rivals and it was a
   TRIUMPH → friends (`lines.bond.rivals_to_friends`). Else if companions with >= 2 shared
   successes → friends (FRIENDSHIPS pool line; BEFRIENDED deeds both). Stop.
3. On DISASTER and they are companions → rivals (`lines.bond.companions_to_rivals`). Stop.
4. On SETBACK/DISASTER and they are parent and child: each who can dread gets +1
   (`lines.bond.parent_saw_child`, `lines.bond.child_saw_parent`, may break).

### 12.3 Dream rivals (`lineage/bond.jai:221-239`)

When a hero gains a dream (wanderer arrival, coming of age, receiving a passed dream), if
that dream is dreamt and unfulfilled: every *other* living **adult** with an unfulfilled dream
of the **same kind**, who is not kin and has no bond other than companion, becomes a rival
(`lines.bond.dream_rivals`). Two AVENGE_THE_LOST dreams for different people are the same
kind [emergent]. The hero receiving the dream may be a child.

### 12.4 What bonds do

- Power in a shared party (§6).
- Steadying: a bond with power > 0 lets the companion steady fear (§10.1).
- Grief (§12.5); heir ranking (§15.1); epitaph "loves" (§20).
- Spouses can have children (§17.3).

### 12.5 Grief (`lineage/bond.jai:241-299`)

When hero D dies (immediately for quest deaths, on the death page for old-age deaths), for
each of D's bonds in order, to a **living** mourner M, using **M's** bond kind to D:
- RIVAL → `lines.grief.rival`, no dread.
- grief 0 (companion) → nothing.
- grief = 2 (spouse, parent, child) or 1 (friend, mentor, student); +2 if M is
  OUTLIVE_THOSE_YOU_LOVE.
- If M cannot dread → M is collected as a "bearer"; else `lines.grief.dread` and dread +=
  grief (may break M; the scar names D).
Bearers are told together at the end: one → `lines.grief.borne_one`; several →
`lines.grief.borne_many`.

## 13. Destinies (`lineage/destiny.jai`, `content/destinies.json`)

**Speaking** (`lineage/destiny.jai:95-104`): only if the hero's destiny is UNSPOKEN. Among the
eight speakable destinies (not UNSPOKEN, not OPEN_THE_SEALED_DOOR), those held by any living
hero are *claimed*; pick uniformly among the unclaimed; if all are claimed, uniformly among
all eight. Spoken at coming of age and on arrival. OPEN_THE_SEALED_DOOR is only ever
authored (Ysolde) or inherited (§15.4).

| Destiny | Rules |
| --- | --- |
| FIRE_WILL_END_YOU | Claimed by a SETBACK or DISASTER on a quest carrying Fire: dies (burned), fulfilled. Shields: never dies of old age (chance 0); on quests a second wound does not kill and a disaster death roll is ignored. |
| OUTLIVE_THOSE_YOU_LOVE | Grief +2 whenever grief applies. Old-age chance x0.5. |
| WEAR_A_CROWN | On a TRIUMPH at the King's Court with personal renown >= 8 (after this quest's reward), the hero is crowned (§7.4). |
| OPEN_THE_SEALED_DOOR | +5 power at each Door lock. On death, passes to the firstborn living child (§15.4). |
| CHILD_WILL_SURPASS_YOU | The first child born to the hero gets +2 to all base aptitudes (checked at birth: no child bond yet); fulfilled then. Once the hero's firstborn (any CHILD bond, living or dead, earliest born) is an adult, the hero may not learn anything. |
| BREAK_AND_BE_MENDED | The first disaster (while unfulfilled) mends instead of the death roll (§7.4). |
| DIE_IN_YOUR_BED | Shields on quests like the fire destiny (no disaster death, no second-wound death). Old-age chance x2. |
| CARRY_THE_HOUSE | +1 house renown per won quest per carrier. House -4 renown when they die. |
| TEACH_A_GREATER | May not learn anything. As a teacher: `taught` counts as 9 and adult learners get +1. |

"May still learn" (`lineage/destiny.jai:138-144`) gates the triumph lesson, the youth lesson
and every winter lesson; it does **not** gate the coming-of-age +1 or mending.

The epitaph's prophecy sentence reads the fulfilled flag (§20).

## 14. Legacies, blessings, tales, ghosts

### 14.1 Leaving a legacy (`lineage/legacy.jai:130-185`)

The dreamer = the dream's owner if any, else the fulfiller F. By the dream's legacy kind:

- **HEIRLOOM** (FORGE_A_BLADE, WALK_EVERY_ROAD, SEE_A_CHILD_GROWN): forge it
  (`content/legacies.json` `heirlooms`): a blade takes the next blade name in order (cycling
  after ten); the road-book is named "<dreamer>'s road-book"; otherwise "the <F's house>
  cradle-ring". Bonus +2; aptitude Might/Wits/Spirit respectively; provenance names F and the
  year. Give it to F (§14.2). Line `lines.legacy.heirloom`.
- **TALE** (SEE_THE_SEA, ROOF_OF_THE_WORLD, KNOWN_AT_COURT): append a house tale titled by
  the dream kind's format with the dreamer's name. `lines.legacy.tale`.
- **BLESSING** (QUIET_THE_BARROW, AVENGE_THE_LOST, WORTHY_STUDENT): word it — "<dreamer>'s
  rest" (+2 against Undead), "<dreamer>'s debt, paid" (+2 at the avenged place), "<dreamer>'s
  patience" (+1 on every quest). Recipients: F, all descendants of the dreamer, all
  descendants of F, and for WORTHY_STUDENT everyone F taught; each **living** recipient
  receives it unless they already have a blessing with the same title. `lines.legacy.blessing`
  names the newly blessed.

F's legacy is recorded (kind + telling) and a LEFT_LEGACY deed is added. Children born later
to blessed parents inherit their blessings (§17.3).

### 14.2 Receiving a new heirloom while holding one (`lineage/legacy.jai:111-128`)

The old heirloom passes to F's **heir** (below); with no heir it is lost
(`lines.legacy.hung_over_hearth`); with an heir, that heir's own heirloom (if any) is simply
overwritten. F then holds the new one.

**Heir** (`lineage/legacy.jai:281-304`): first the heir of the blood: over F's CHILD bonds,
take each child if living and empty-handed, otherwise recurse into that child's own line;
among all candidates found choose the earliest born_year. If none: the first bond (in F's
bond order) to a living, empty-handed hero who is F's spouse or whom F taught. Else none.

### 14.3 Tales (`lineage/legacy.jai:251-266`)

At every turning, house renown += number of house tales (each +1), with `lines.tales.one`
or `lines.tales.many`. Tales are never removed.

### 14.4 Ghosts (`lineage/ghost.jai`)

**Raising** (§15.2): a copy of the undone dream (owner set to the dead if unset), the dead
hero, and a place: the dream kind's place (Coast, Pass, Emberfall, avenged place, Court,
Barrow); for WORTHY_STUDENT, WALK_EVERY_ROAD, SEE_A_CHILD_GROWN, the place of the hero's
questing death if they died questing and not at the Door, else the Barrow. Appended to the
ghost list. `lines.ghost.raised`.

**The ghost quest** (`:55-76`): title "Lay <name>'s ghost", premise and endings from
`content/ghost.json` (`{name}`, `{task}` = told current stage task, `{He}`, `{he}`, `{him}`
substituted; `%1` = party names), Spirit, the place's tags, calm seats 2 and calm danger 2
with the place's trouble (§5.2 formula, no wobble). It occupies its place on the board (§5.2)
every summer until laid.

**Laying** (`:83-111`): on SUCCESS/TRIUMPH: append the tale "The laying of <name>'s ghost";
the dead hero's bequest becomes LAID_TO_REST with the year; their epitaph is recomposed;
`lines.ghost.laid`; each living member records LAID_GHOST; the ghost is removed. The dream
is **not** fulfilled and leaves no other legacy.

**Removal** is a *swap remove*: the last ghost in the list moves into the removed one's
index. This determines which ghost is "first" next summer [emergent].

## 15. Death pages, heirs and inheritance

### 15.1 The death page (`lineage/passage.jai:146-184`)

For each hero in `mourned`, in order of death (§18 step 4):
1. Roll an epitaph wording (§20).
2. The undone dream = the **burden** if dreamt and unfulfilled, else the own dream if dreamt
   and unfulfilled, else none. Only one dream can pass [emergent] (OQ-3).
3. Lines: `lines.death.leaves_heirloom` if holding one (and the bequest records its name);
   `lines.death.leaves_dream` if a dream is undone.
4. Pass the Door promise (§15.4).
5. Grieve if not already grieved (old-age deaths) (§12.5).
6. Dream fate: NEVER_DREAMT if no own dream; FULFILLED if the own dream is fulfilled.
7. If holding an heirloom or leaving a dream: gather heirs and leave the page **undecided**;
   otherwise decided.
8. Compose the epitaph (shown at the top of the page).

**Heirs** (`lineage/passage.jai:186-217`): living heroes other than the dead, ordered by
rank, then creation order, at most 8:

| Rank | Who (relative to the dead D) |
| --- | --- |
| 0 | D's children |
| 1 | D's other descendants |
| 2 | D's spouse |
| 3 | kin (siblings by a shared parent) and D's parents |
| 4 | those D taught |
| 5 | anyone D has a steadying bond with |
| 6 | everyone else, children included |

The heir list is fixed when the page is made (before this turning's births, comings of age
and arrivals). Each heir button reads `lines.heir.prospect` with kinship (child's
son/daughter, grandson/granddaughter for other descendants, the shown bond's kinship, brother/
sister, else "of the house") plus "(not the dream)" if the dead leaves a dream the candidate
cannot take, else "(lays one aside)" if both hold heirlooms. There is always "No one. Let it
lie."

### 15.2 Choosing (`lineage/passage.jai:250-311`)

One choice per undecided page, irrevocable. The turning cannot proceed past an undecided
page (§18.1). On choice of heir H (or none):
- bequest decided, heir recorded.
- Heirloom: with H — H's own heirloom, if any, is **discarded** (`lines.heir.lays_aside`,
  OQ-5); H receives the heirloom; INHERITED deed. Without H — buried
  (`lines.heir.buried_with`). D no longer holds it.
- Undone dream: if H exists and **can take a dream** (H has no dream, or H's burden is empty
  or fulfilled) → pass it (below), dream fate PASSED_ON. Otherwise (no one, or an H who
  cannot) → raise a ghost (§14.4), dream fate LEFT_TO_NO_ONE.
- The new lines are inserted right after the bequest lines; the epitaph is recomposed with
  the same wording.

**Passing a dream** (`:293-311`): copy it at its current stage and counts; owner = D if
unset (an inherited dream keeps its original owner). If H has no dream it becomes H's dream;
otherwise H's burden (replacing a fulfilled burden). `lines.heir.takes_dream`; TOOK_UP_DREAM
deed; dream rivalry check (§12.3).

### 15.3 Nearest kin (crowned heroes) (`lineage/passage.jai:219-227`)

Gather the heir list as above; the first without an heirloom, else the first, else none.

### 15.4 The Door's promise (`lineage/passage.jai:313-334`)

If D was OPEN_THE_SEALED_DOOR (and not fulfilled — never set in practice): the firstborn
**living** child (CHILD bonds, earliest born_year) gets OPEN_THE_SEALED_DOOR with blood_of =
D's blood_of if any, else D's name, replacing whatever destiny they had, spoken or not
(`lines.promise.passes`). No living child: `lines.promise.no_child` and the promise ends.

## 16. The Sealed Door

### 16.1 The last summer

After the turning of year 25, the summer board holds only the Door's Might lock (4 seats,
§5.1). The player seats up to four (refusing heroes — broken fear of Dark or Cold — are
bounced). "Try the Door" needs at least one.

### 16.2 Resolution (`lineage/door.jai:127-162`)

1. locks_opened = 0. The party = the heroes seated.
2. Prologue lines: `lines.door.prologue`; per member `lines.door.brought` with appended
   fragments (blood/promise, heirloom with provenance, Dark/Cold fear, everywhere-blessings,
   wounded); per member who has shown bonds to later members, `lines.door.went_beside`; then
   `lines.door.summary` with the party's power at each lock (patrons included).
   - `lines.door.brought` per member (party order): "<Name> was <age>, <descent>." Descent:
     an ARRIVED deed → `lines.door.descent_arrived`; no parents → `lines.door.descent_none`; parents but
     no grandparents → `lines.door.descent_parents`; else `lines.door.descent_grandparents` (son/daughter,
     grandson/granddaughter by the member's pronoun; names as name lists). Then, appended in
     order when they apply: Door destiny (`lines.door.brought_blood` if blood_of else
     `lines.door.brought_promise`); heirloom (`lines.door.brought_heirloom`, the heirloom name with
     "<own name>'s " turned into "<his/her> own ", then its provenance); a fear of Dark or Cold
     (`lines.door.brought_born_brave`, else conquered with the conquest year
     `lines.door.brought_conquered_year` or without `lines.door.brought_conquered`, else
     `lines.door.brought_afraid` with the penalty); each EVERYWHERE blessing
     (`lines.door.brought_blessing`); wounded (`lines.door.brought_wounded`).
   - `lines.door.went_beside`, per member in party order that has a shown bond (not
     companion) to any **later** member: one `lines.door.companion` item per such later member
     (kinship between, §21; signed bond power), joined as a name list.
3. For each lock in order Might, Wits, Spirit (`content/door.json`; demand 34, danger 3,
   renown 5, tags Dark+Cold, 4 seats):
   - standing = party members still living; if none, stop.
   - bearer = the standing member with the highest **solo** power for this lock (patrons 0);
     first on ties.
   - Resolve it as a quest (§7.1) with the standing party. At the Door: no reward/disaster
     lines, no triumph or youth lessons, no triumph deeds, no sharing the road; everything
     else applies (renown, trouble history of the Door place, fears and courage, setbacks and
     disasters with 45% death, dream witnessing, roads bit, ghosts and crowns cannot apply).
   - The page story is the lock's ending for the outcome with `%1` = standing names and `%2`
     = bearer's name.
   - SUCCESS/TRIUMPH: locks_opened +1; if the bearer is living, OPENED_A_LOCK deed (weight =
     lock index).
4. Every living member of the party records STOOD_AT_THE_DOOR (weight = locks opened).

Wounds, deaths, dread and breaking at one lock carry into the next [emergent]. A hero who
breaks at the Door keeps going and keeps paying the fear penalty.

### 16.3 After the Door

The telling ends with "After the Door" → Ending (§23) whatever the renown.

### 16.4 The Door outlook (display only) (`lineage/door.jai:67-125`, `lineage/panels.jai:353-372`)

For a party: for each lock, power and P(success or better) with the house's patrons; "all
three open" = the product of the three chances (as if independent and unchanged between
locks; the real resolution is not independent, OQ-15). The **best four** shown in the top bar
from summer 1: if there are <= 4 living adults, all of them; otherwise every combination of
four living adults (creation-order combinations), scored `P(all three) + sum(P_i)/1000`,
keeping the first strictly best. Wounded and refusing heroes are not excluded [emergent]
(OQ-14). The Door card and sheet show the same numbers for the seated party, each member's
solo power per lock, the bond total and the patron bonus.

## 17. Generation

### 17.1 Names and houses (`generation/hero-context/hero-context.jai:21-44`)

Three bags (`content/names.json`): 32 names for him, 32 for her, 16 houses. Each draw is
uniform **without replacement**; an empty bag is refilled with the full list. Authored names
are not removed, so a new hero can share a founder's name (e.g. "Wren") [emergent]. Pronoun:
HE or SHE, 50/50.

### 17.2 Wanderers (`lineage/passage.jai:439-472`, `generation/hero-context/hero-context.jai:74-98`)

At each turning, after births and comings of age: if living >= 10, none. If living adults <
5, one arrives for certain; else with chance `min(0.2 + 0.01 * house renown, 0.6)`. At most
one per year. Generation, in roll order: pronoun; name; house; age 17..36; standing by house
renown (CONSTANTS §10); gift aptitude uniform; all three aptitudes rolled in the standing's
"other" range, then the gift overwritten with a roll in the "gift" range; vocation uniform
among the two of the gift aptitude; fear tag uniform over 8; dream (§17.4); personal renown
0..2; destiny spoken (§13). born_year = next year - age. The arrival page: an ARRIVALS pool
line, `lines.arrival.who`, `lines.arrival.dream`, dream-rival lines, `lines.arrival.fear`,
`lines.arrival.seer`, the doom and gift lines; ARRIVED deed dated next year.

### 17.3 Births (`lineage/passage.jai:336-422`, `generation/hero-context/hero-context.jai:100-146`)

For each living hero F in creation order (the list taken before any birth): let S be F's
first living spouse. Skip unless F's id < S's id (each pair once), the marriage's since <
this year (no birth in the wedding's year), both are 18..45 (after this turning's ageing), and
the pair has fewer than 3 children together (living or dead). Then: if living >= 12 → stop
all births; if living children >= 6 → stop all births; roll 60% → a child.

The child: pronoun; name; **house = F's house** (the parent created first) [emergent];
age 0; born_year = this year; parents [F, S].
- Each base aptitude = `max((F + S) / 4 + uniform{0,1}, 1)` (base values).
- For each parent (F then S) with destiny CHILD_WILL_SURPASS_YOU and no child yet: +2 to all
  three; the parent's destiny is fulfilled.
- Fear: roll a tag uniformly over 8. Then for each parent F then S: if that parent's fear is
  conquered (including born brave): roll 50% → the child is born brave with that tag
  (conquered + born brave), stop; on failure go on to the next parent. Otherwise roll 80% if
  the parent is broken, else 40% → the child takes that tag, stop.
- Blessings: every blessing of F, then of S (no duplicate titles).
- Bonds: child→F PARENT, child→S PARENT (since = this year).

Birth page: `lines.birth.title`, a BIRTHS pool line, `lines.birth.named`, one fear line
(born brave / "has her father's fear" / "afraid in the cradle" — the attributed parent is
whichever parent's tag equals the child's, preferring S, even if the tag was rolled at random
[emergent]), blessing lines; CHILD_BORN deeds for both parents.

### 17.4 Rolled dreams (`generation/hero-context/hero-context.jai:46-59`)

From the eight wanderer dreams (`content/wanderers.json`, not AVENGE_THE_LOST): claimed =
kinds dreamt by any other living hero; WORTHY_STUDENT also counts as claimed if the hero is
younger than 35. Uniform among unclaimed; if all are claimed, uniform among all eight.

### 17.5 Coming of age (`lineage/passage.jai:424-437`, `generation/hero-context/hero-context.jai:155-225`)

At each turning, every living hero whose age (after ageing) is exactly 12:
1. Vocation: roll uniformly among the two vocations of their best base aptitude; if they have
   a MENTOR bond (first MENTOR-kind bond in their bond list, living or dead teacher), the
   vocation becomes that teacher's vocation instead (the roll is still made).
   `lines.age.calling`.
2. If that teacher exists and the child's base in the teacher's best aptitude is < 9: +1
   (`lines.age.teaching_shows`).
3. Dream (§9.5) with its line; dream rivals.
4. Destiny spoken (§13).
5. Page: the Seer line, doom, gift; CAME_OF_AGE deed.

## 18. Turn of the year (`lineage/passage.jai:56-112`)

When the winter is let pass, in this exact order:

1. The winter page ("The winter of year N"): winter resolution lines (§11.3), or "A quiet
   winter. ..." if none.
2. Every living hero ages +1.
3. Old age: for each living hero (creation order), roll death with chance
   `min((0.04 + 0.025 * (age - 55)) * factor, 1)` for age >= 55 (0 below, 0 for
   FIRE_WILL_END_YOU; factor 0.5 OUTLIVE, 2 BED). A death uses a SLEEP_DEATHS pool line as
   its fate telling and no page lines.
4. Death pages for everyone in `mourned` (summer deaths first, then old age), §15.1; then
   `mourned` is cleared.
5. Births (§17.3).
6. Comings of age (§17.5).
7. Wanderer (§17.2).
8. Tales: house renown += tale count (§14.3).
9. For each living hero: if their phase changed this turning and the new phase is not Youth,
   `lines.phase.changed`; then a YEAR_TURN dream moment.
10. If steps 8-9 produced lines: a "Year N+1 begins" page with them.
11. The last page gets a closing line: "The year turns. The Sealed Door opens in R years."
    (R = years until the Door - 1) or, when R <= 0, "The year turns. The Sealed Door stands
    open. It asks for four."

### 18.1 The turning screen (`scene/scenes/turning.jai`)

Pages in the order produced (each long page split into leaves). Kinds: WINTER, DEATH (skull
portrait, epitaph at the top), BIRTH, COMING_OF_AGE, ARRIVAL, YEAR. On an undecided death
page the heir buttons (up to 8) and "No one. Let it lie." are shown under "WHO IS HIS/HER
HEIR? One choice. It cannot be unmade." While a page is undecided: "Go on" is disabled, page
buttons beyond it do nothing, and "Skip ahead" jumps to the first undecided page instead of
leaving. After a choice the leaves are rebuilt and the view stays on that page. The last
leaf's button is "Summer comes".

## 19. Information the player can read

### 19.1 Hero sheet (`lineage/sheet.jai`)

Full name; station ("Child of house H, aged A" or "<Vocation>, <phase>, aged A"); condition
("Died <year telling>, aged A", "Left ...", or "Renown R" with Wounded/Settled); heirloom
sprite. Effective aptitudes with the phase adjustment noted (hover gives base "of 9"); "Another
wound will kill. Rest to heal." if wounded. DREAM (and BURDEN): title (told, if owned), "Taken
up from <owner>", all three stages marked done/current/upcoming (only the current stage for an
owned dream), or "Fulfilled."; hover gives the legacy promise. FEAR: state label (FEAR / FEAR,
CONQUERED / FEAR, BROKEN / BORN BRAVE), tag and noun; for an active fear, dread pips (of 5),
courage pips (of 3) and the penalty, or "Settled. Dread has no hold." DESTINY (or DESTINY,
COME when fulfilled): prophecy (with "Blood of X:"), doom, gift; "Not yet spoken." for
UNSPOKEN. BONDS: up to six shown bonds (not companions), living first, each with its
gendered title and power (or "gone"), then "and N more". HEIRLOOM (provenance on hover),
BLESSED lines, LEAVES (legacy), SCAR lines.

### 19.2 The family screen (`lineage/family-tree.jai`)

Every hero who ever lived is a node (dead greyed, departed gold). Layout is free, but the
relations shown are: parents above children (a line from a parent pair or single parent);
spouses linked. Pointing at a node shows the remembrance: the epitaph if composed (the dead,
the crowned, and everyone at the Ending), otherwise for the living their station and
`ui.family.living_*` lines (dream and progress, burden, fear state, prophecy). A tally reads
"L living, G gone. <tales sentence> House renown R." (`:313-321`).

## 20. Epitaphs (`lineage/epitaph.jai`, `content/epitaph.json`)

**Wording** (`:62-70`): a frame 0..2 rolled uniformly but never equal to the previous frame
rolled anywhere in the run, and nine independent fair coins (one per part, 0 or 1).

**When composed**: at founding for the two dead founders; at crowning; on the death page
(before the heir); after the heir choice (same wording); when the hero's ghost is laid; when
their ghost's dream is taken up at a coming of age; at the Ending for deaths not yet given a
page (new wording) and for every living hero (new wording).

**Parts** (each may be empty): ORIGIN, ROADS, TRIUMPH, FEAR, DREAM, PROPHECY, LOVE, END,
LEFT. Their selection rules, with `vN` = that part's coin (`epitaph.<key>` names):

- **ORIGIN** (`:154-177`): has an ARRIVED deed → `origin.arrived_v` (year, age from that deed).
  Not living and fate year < 1 → empty. born_year >= 1 and has a parent → `origin.born_v`.
  Has a parent → `origin.child_of_old`. Else `origin.founder_v`.
- **ROADS** (`:187-219`): quests_faced == 0: living child → `roads.childhood_teachers` (names
  of MENTOR bonds; empty if none); dead child → `roads.never_old_enough`; else v1
  `roads.kept_house_1` / v0 `roads.never_went_0`. No FIRST_QUEST deed →
  `roads.count_only`. Exactly one → `roads.once`. Else `roads.many_v`.
- **TRIUMPH** (`:221-229`): the TRIUMPH deed of greatest weight (later wins ties); empty if
  none; v1 `triumph.spoken_of_1`, v0 `triumph.best_day_0`.
- **FEAR** (`:231-264`): born brave → `fear.born_brave`. A CONQUERED_FEAR deed →
  `fear.conquered_v`. A BROKEN deed with another hero → `fear.broken_by_grief`. A BROKEN
  deed → `fear.broken_v`. fears_faced > 0 → `fear.faced_v`. Child → `fear.child`. Else
  `fear.kept_from`.
- **DREAM** (`:271-310`), from the **own dream only** (OQ-2): empty if none. Fulfilled →
  `dream.fulfilled_v`. Fate PASSED_ON with an heir → `dream.passed_v`. LEFT_TO_NO_ONE →
  `dream.left`. LAID_TO_REST → `dream.laid`. Living → `dream.living` with
  `progress_tellings[min(current, 2)]`. Departed → `dream.crowned`. Else `dream.died`. In
  every template whose `args` list "inherited suffix or \"\"", that argument is
  `dream.inherited_suffix` when the dream has an owner, else empty.
- **PROPHECY** (`:328-382`): empty if UNSPOKEN. FIRE, CROWN, SURPASS and MENDED have a
  `*_came` sentence used when the destiny is fulfilled and a plain one otherwise; OUTLIVE,
  BED and CARRY have one sentence each. The Door: with blood_of → `prophecy.door_blood_stood`
  if the hero has a STOOD_AT_THE_DOOR deed else `prophecy.door_blood`; without →
  `prophecy.door_stood` / `prophecy.door`. TEACH_A_GREATER: `prophecy.greater_came` naming the
  first student they taught (a bond with taught = true, in bond order) whose best base
  aptitude value exceeds the teacher's best base value, else `prophecy.greater`.
- **LOVE** (`:384-417`): spouses (all, full names) and/or children (all, first names) →
  `love.spouses_children` / `love.spouses` / `love.children`; else the **last** FRIEND bond:
  kin → `love.sibling_friend`, else `love.friend_v`; else empty.
- **END** (`:419-448`): living → stood at the Door (`end.door_opened` listing the locks they
  bore open, or `end.door`), else `end.child` / `end.living`. Dead or departed: fate year < 1
  → `end.before_first_year`; else `end.dead_v`.
- **LEFT** (`:450-484`): living → `living.made_*` by legacy kind, else `living.carried_heirloom`
  if holding one, else empty. Departed → `left.departed`. Has a legacy and bequeathed an
  heirloom to an heir → `left.legacy_and_heirloom`. TALE → `left.tale`. BLESSING →
  `left.blessing`. HEIRLOOM legacy with an heir → `left.heirloom_made`. Bequeathed heirloom:
  with heir → `left.heirloom_went`; decided without heir → `left.buried_with`. Else empty.

**Composition** (`:86-148`): walk the priority order ORIGIN, END, DREAM, PROPHECY, LEFT, LOVE,
TRIUMPH, FEAR, ROADS; choose each non-empty part whose sentence count (number of "."
characters) still fits within 6 in total (a part that does not fit is skipped and later ones
still considered). Then emit the chosen parts in the frame's order, joined by single spaces.
The first emitted part names its subject: a leading "He "/"She " becomes the full name; a
leading "His "/"Her " becomes "<full name>'s"; otherwise the first " he "/" she " inside
becomes " <full name> "; otherwise unchanged.

Helpers: "own making" replaces "<name>'s " by "his/her own " in the legacy telling; "year
telling" (§21).

## 21. Text conventions (`lineage/writing.jai`, `lineage/legacy.jai:268-279`)

- **Name list**: none → "no one yet"; one → it; else "a, b and c".
- **Party telling**: no one → "No one"; else the name list of first names.
- **Year telling**: < 1 → "before the first year"; > 25 → "in the last summer"; else "in
  year N".
- **Count words**: 0..12 → "never", "once", "twice", "three times" ... "twelve times"; then
  "N times".
- **Capitalized / lowered**: first byte upper/lower case.
- **Writing pools** (`content/writing.json`): each pick is uniform over the pool **excluding
  the line picked last time from that same pool** in this run (the first pick is uniform over
  all) (`lineage/writing.jai:157-163`, `lineage/chance.jai:15-21`).
- Pronouns: he/him/his, she/her/her.
- **Phase effect sentence** (`lineage/lore.jai:125-157`; fragments in `content/lore.json`
  `phase_effect_fragments`), used by `lines.phase.changed` and `lines.age.calling` (always
  for Youth there): if all three adjustments of the phase are equal and non-zero → "<He>
  brings <|n|> <less|more> in every aptitude" + (Youth only) " until <he> is 20" + ". ";
  otherwise, for each non-zero adjustment in aptitude order, "<His> <Aptitude> <grows|fades>
  by <|n|>" for the first and " and <his> <Aptitude> <grows|fades> by <|n|>" for the next,
  then ". " if there was any clause. Then the phase note with `{He}`/`{his}` substituted.
  Youth: "He brings 1 less in every aptitude until he is 20. He learns fast: ..."; Prime:
  "He is at full strength."; Veteran: "His Might fades by 1 and his Wits grows by 1. He
  teaches well now."
- **Kinship between** two party members (Door prologue, `lineage/bond.jai:66-72`): spouse,
  parent or child bond → that kinship (husband/wife, father/mother, son/daughter, by the other's
  pronoun); else kin by a shared parent → brother/sister; else any bond → its kinship telling
  (teacher, student, rival, friend); else "companion".
- **Kinship to the dead** (heir buttons, `lineage/passage.jai:198-205`): the dead's child →
  son/daughter; other descendant → grandson/granddaughter; any shown bond → its kinship
  telling from the dead's side; kin → brother/sister; else "of the house".

## 22. Randomness

### 22.1 Generator and seeding

One generator for all gameplay (`game.jai:29`), seeded per run from the clock (or `-seed`)
(`source/main.jai:57-72`, `source/core/initialize.jai:82-109`); a new house reseeds from a
draw of the previous run's generator. The port needs only matching distributions.

Primitives (`lineage/chance.jai`), with u uniform on [0, 1) (the engine's
`random_get_zero_to_one_open`; OQ-1):
- `between(lo, hi)` = lo + floor(u * (hi - lo + 1)), i.e. uniform integer.
- `chance(p)` = u < p.
- `index(n)` = between(0, n - 1).
- `fresh_index(n, prev)` = uniform over {0..n-1} \ {prev} (two draws when the first hits prev).
- `unclaimed_index(claimed)` = uniform over unclaimed indices, else uniform over all.
- Bag draw (`source/core/distribution/choice.jai:5-20`) = uniform index, removed if without
  replacement.

### 22.2 Roll inventory, in sequence

- **Founding**: two epitaph wordings (frame + 9 coins each). Then board generation for year 1.
- **Board generation** (per attempt, up to 16): place draws, template picks, seats, wobble —
  for each non-forced, non-ghost quest in planning order.
- **Summer** (per answered quest in board order): two dice; pool picks as lines are written;
  on SETBACK one unlucky index; on DISASTER one death roll per member reaching step 3 of §7.4
  (members claimed by fire or mended roll nothing).
- **Winter**: pool picks only (rest, wedding, tale).
- **Turning**: one old-age roll per living hero (even at chance 0); per death page one
  wording; per eligible pair one birth roll; per newborn: pronoun, name, three 0/1, fear tag,
  then up to two inheritance rolls; per coming of age: vocation, maybe a dream roll, destiny;
  wanderer chance (unless certain), then pronoun, name, house, age, gift index, three others,
  gift, vocation, fear, dream, renown, destiny; pool picks throughout.
- **Ending**: one wording per un-mourned dead and per living hero.

### 22.3 Correlations to know about

- Text generation (pool picks, epitaph wordings) draws from the **same** generator as the
  rules, interleaved. This shifts sequences but not distributions; a port that uses a separate
  text RNG remains distribution-identical.
- Board planning draws a variable number of times (up to 16 boards) before the summer's dice.
- No roll is reused for two decisions. Pool picks are deliberately anti-correlated (never the
  same line twice running per pool); epitaph frames likewise across the whole run.

## 23. The Ending (`scene/scenes/ending.jai`)

On entering (`:12-15`):
1. **Remember the fallen**: for each hero still in `mourned` (deaths in the final summer —
   the Door, or the summer the house closed): roll a wording; bequest decided; if holding an
   heirloom, it goes to their heir (§14.2 heir rule, no choice) or is recorded as buried;
   dream fate NEVER_DREAMT/FULFILLED where applicable (undone dreams are neither passed nor
   ghosted); compose the epitaph. No death pages, no further grief.
2. **Remember the living**: every living hero gets a new wording and an epitaph.

The verdict page: if the Door stood open, title and text by locks opened (0..3,
`content/door.json`), then one line per member of the first lock's party
(`lines.door.verdict_*`: the dead "fell at <last lock they stood at>", bearers who opened
locks "opened X and Y, and came home", the rest together "came home [too]"). If the house
closed: `door.closed_title`, `door.closed_verdict`, "It was year Y, with the Door still R
years off." Then "N lived under this roof. <tally>". Buttons: "The family" (tree, with "The
verdict" to return) and "Begin another house".

There is no score.
