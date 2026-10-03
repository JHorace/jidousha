# Keifu — spec gaps

Places where `spec/SPEC.md`, `spec/CONSTANTS.md` or `spec/content/` did not determine
something this port needed. Each entry records the smallest conservative choice the port
made, the code site (grep for `SPEC-GAPS KG-n`), and the question for the spec-amendment
pass, which has the original and adjudicates every entry. Entries are never deleted; an
adjudicated one gets a **Resolved:** line.

Session 1 (W0 + W1): 6 entries. Session 2 (W2): 6 entries, KG-7 to KG-12. Session 3 (W3):
12 entries, KG-13 to KG-24. Session 4 (W4): 5 entries, KG-25 to KG-29. Session 5 (W5): 4
entries, KG-30 to KG-33. Session 6 (W6): 7 entries, KG-34 to KG-40. Session 7 (W7): 6 entries,
KG-41 to KG-46. Session 8 (W8): 10 entries, KG-47 to KG-56. Session 9 (W9): 8 entries, KG-57 to
KG-64.

---

## KG-1 — the order of a hero's bonds on the sheet, within "living first"

- **Spec says:** §19.1 "BONDS: up to six shown bonds (not companions), living first, each
  with its gendered title and power (or "gone")". §3.2: bonds are an "ordered list".
- **Underdetermined:** the order *within* the living group (and within the gone group).
  Formation order (§4, `household.json` `bonds`) gives Garrick's list as Elsbeth (spouse),
  Maren (child), Odo (friend), so living-first-in-formation-order reads **Daughter Maren +2,
  Friend Odo +1, Wife Elsbeth, gone**. MODULES.md's W1 oracle lists them **Friend Odo +1,
  Daughter Maren +2, Wife Elsbeth gone** — which matches a reverse walk, or the oracle may
  simply not be stating an order.
- **Port's choice:** formation order inside each group (`src/sheet.rs` `shown_bonds`). The
  W1 oracle assertion checks that both living bonds are present and come after BONDS and
  before the bond to the dead, and deliberately does not assert Odo-before-Maren.
- **Question:** in what order does `lineage/sheet.jai:218-234` walk the bonds?

## KG-2 — the "year/years" word in the top bar's Door countdown

- **Spec says:** `ui.top_bar.door_countdown` = "The Sealed Door opens in % %. It will ask
  for four." with args "years until the Door", "year/years".
- **Underdetermined:** `ui-text.json` holds no "year"/"years" word. `lines.json` has
  `turning.year_word` ("year") and `turning.years_word` ("years"), used by the turning's
  own countdown line.
- **Port's choice:** the `lines.turning.*_word` pair, "year" at exactly 1 and "years"
  otherwise (`src/family.rs` `top_bar`). Year 25 reads "opens in 1 year".
- **Question:** does `lineage/panels.jai:354` use the same words, and the same singular rule?

## KG-3 — the family screen's "The Seer said" for an unspoken destiny

- **Spec says:** §19.2 the living's remembrance carries `ui.family.living_*` lines
  including the prophecy; `living_seer` = ` The Seer said: "%"`.
- **Underdetermined:** whether a hero whose destiny is UNSPOKEN (prophecy "Not yet
  spoken.") gets that sentence, which would read `The Seer said: "Not yet spoken."`.
- **Port's choice:** omitted for UNSPOKEN (`src/family.rs` `remembrance`). Pip, Wren read
  without it.
- **Question:** what does `lineage/family-tree.jai:380` condition the line on?

## KG-4 — an adult with no dream

- **Spec says:** §19.1 shows "Too young to know it yet." (`ui.hero_sheet.too_young`) in
  the DREAM section; §19.2 has `living_too_young`. Wren (a child) is the only undreamt hero
  at founding.
- **Underdetermined:** what the sheet and the family screen show for an undreamt **adult**
  (the spec's own conditions for the two too-young lines are not given).
- **Port's choice:** a child without a dream gets the too-young line on both screens; an
  undreamt adult gets no DREAM section and no dream sentence (`src/sheet.rs`
  `dream_section`, `src/family.rs` `remembrance`). Unreachable at founding.
- **Question:** the conditions at `lineage/sheet.jai:118` and `lineage/family-tree.jai:361`.

## KG-5 — the current stage's count on the sheet

- **Spec says:** §19.1 "all three stages marked done/current/upcoming". §9.2 defines
  "Progress" as the told task plus " (count/goal)" when goal > 1, for tellings.
- **Underdetermined:** whether the sheet shows the count for a current stage with goal > 1
  (e.g. "Succeed against the Undead twice (1/2)"), and whether its tasks are told or raw.
- **Port's choice:** raw tasks (as the W1 oracle quotes them: "Win a triumph at the
  Barrow"), with " (count/goal)" appended to the current stage only when its goal > 1
  (`src/sheet.rs` `dream_lines`). Garrick's current stage has goal 1, so the oracle is
  unaffected.
- **Question:** what `lineage/sheet.jai` prints per stage.

## KG-6 — DESTINY for UNSPOKEN: the doom line

- **Spec says:** §19.1 "DESTINY ...: prophecy (with "Blood of X:"), doom, gift; "Not yet
  spoken." for UNSPOKEN". `destinies.json` gives UNSPOKEN a doom ("The Seer speaks at the
  coming of age.") and an empty gift.
- **Underdetermined:** whether an UNSPOKEN sheet shows that doom line under "Not yet
  spoken.".
- **Port's choice:** prophecy only, read literally from §19.1 (`src/sheet.rs`
  `destiny_section`).
- **Question:** does `lineage/sheet.jai:183-` print doom and gift for UNSPOKEN?

---

## KG-7 — the glue between a fear-line item and "steadied"

- **Spec says:** §5.4 "Fear: <name> -<penalty> [steadied], ..."; `ui.quest_card.fearful_item`
  = "% -%" and `ui.quest_card.steadied` = "steadied", two separate strings. MODULES.md's W2
  oracle quotes the items as **"Maren -2, steadied"** and **"Garrick -3, steadied"**.
- **Underdetermined:** what joins the item and the word. §5.4's bracket notation reads as a
  space; the oracle reads as ", ". No content string carries the glue.
- **Port's choice:** ", " (the lore's name-list separator), as the oracle quotes it, so the
  whole line reads "Fear: Maren -2, steadied, Garrick -3, steadied" (`src/power.rs`
  `fear_items`). The W2 oracle asserts exactly the quoted items.
- **Question:** how does `lineage/quest-card.jai:148-149` join `fearful_item` and `steadied`
  — and are the items then joined with ", "?

## KG-8 — what makes a card's fear item "steadied", and who is listed

- **Spec says:** §5.4 lists "seated heroes who fear it" with "[steadied]". §10.1 step 3
  defines the companion who steadies at resolution: "the first *living* other member whose
  bond with m steadies (power > 0)".
- **Underdetermined:** (a) whether the card's "steadied" is that same test over the seated
  party; (b) whether "fear it" means "the fear costs power" (not conquered; a broken fear
  still counts, §6 line 3) or something narrower.
- **Port's choice:** (a) the same function as resolution, `bonds::steadying_companion`, so
  the card cannot say "steadied" when the roll would not steady; (b) listed iff the quest
  carries the tag and the fear is not conquered — the hero whose fear line costs power
  (`src/power.rs` `fear_items`, `src/fear.rs` `fears`).
- **Question:** the condition at `lineage/quest-card.jai:144-150`.

## KG-9 — Form's replacement and `since`; Change with no bond

- **Spec says:** §12.1 a bond records "since (year formed or last changed)"; Form replaces
  a bond of strictly lower rank "keeping the bond's place in each list"; Change "overwrites
  both sides regardless of rank **and sets since**".
- **Underdetermined:** (a) whether Form's replacement also sets `since` (only Change is said
  to) — it decides "no birth in the wedding's year" (§17.3) for companions or friends who
  wed; (b) what Change does when the two have no bond.
- **Port's choice:** (a) yes — a replacement is a change of kind, and §11.3 says the
  spouses' bond is "formed this year" (`src/bonds.rs` `form`); `taught` and shared successes
  are kept. (b) a loud panic: every Change the spec names (§11.3 rivals make peace, §12.2
  rivals to friends and companions to rivals, §4 Brannoc and Ysolde) changes a bond that
  exists (`src/bonds.rs` `change`).
- **Question:** `lineage/bond.jai:84-121` — does the replacing branch write `since`, and
  does Change append when there is no bond?

## KG-10 — the order of a grief line and the break it causes

- **Spec says:** §12.5 "else `lines.grief.dread` and dread += grief (may break M; the scar
  names D)"; `lines.grief.dread`'s argument is "mourner dread after grief, capped at 5".
  §10.1 step 7, for facing a fear, orders it explicitly: dread, then the line, then the break.
- **Underdetermined:** whether `lines.fear.broken` comes before or after the grief line on
  the page.
- **Port's choice:** the grief line first, then the break's line, the order §12.5 lists them
  and the order §10.1 uses (`src/grief.rs` `grieve`).
- **Question:** `lineage/bond.jai:265-275` — is the line written before `add_dread`?

## KG-11 — shedding dread from zero

- **Spec says:** §10.2 "resting by the fire sheds 1 (not if broken)"; §11.3 the fire's rest
  writes "one of three lines ..., or nothing if neither applied".
- **Underdetermined:** whether a hero at dread 0 "sheds" (which decides whether the rest
  "applied" and so which rest line is written, W7), and whether dread can go below 0.
- **Port's choice:** nothing is shed at 0, and dread never goes below 0; `shed_dread`
  reports whether it shed (`src/fear.rs`).
- **Question:** `lineage/hearth.jai:324-340`.

## KG-12 — the fear deeds' fields

- **Spec says:** §10.4 "a BROKEN deed (with `other` = the mourned when grief)"; §10.3 "a
  CONQUERED_FEAR deed"; §3.2 a deed is (kind, year, age, place, weight, other, telling);
  `lines.deed.broken` / `deed.broken_by_grief` / `deed.conquered_fear` give the tellings
  ("occasion" = `common.at_place`). `lines.grief.occasion` exists but its own note says grief
  uses `scar.broken_by_grief`.
- **Underdetermined:** each deed's place and weight, and where `grief.occasion` is used.
- **Port's choice:** year = the current year, age = the hero's age; place = the quest's
  place for a break or a conquest at a place, none for a break by grief; weight 0
  (`src/fear.rs`). `grief.occasion` is not read: a break by grief takes
  `scar.broken_by_grief` and `deed.broken_by_grief`, which name the dead directly.
- **Question:** `lineage/fear.jai:51-78` — what place and weight the two deeds record, and
  whether `grief.occasion` reaches any string the player can read.

---

## KG-13 — the "dream"/"burden" word in the quest sheet's call line

- **Spec says:** §5.4 "<Name>'s dream|burden: <current stage task>. <He> must ...";
  `ui.quest_sheet.dream_call` = "%'s %: %. % must %." with argument 2 "burden/dream".
- **Underdetermined:** no content key holds the lower-case words "dream" and "burden". The
  nearest are the sheet's headings `ui.hero_sheet.dream` ("DREAM") and `.burden` ("BURDEN");
  `lines.dream.bearing_own` is "% dream" (a phrase, not the word).
- **Port's choice:** the heading lowered whole — "dream", "burden" — so the word still comes
  from content by key (`src/calls.rs` `call_line`). MODULES.md's W3 oracle quotes the result
  exactly: "Garrick's dream: Win a triumph at the Barrow. He must triumph."
- **Question:** is the word a literal at `lineage/quest-card.jai:195`?

## KG-14 — whom a dream line's task is told about

- **Spec says:** §9.2 "Told task: ... a trailing ' you' becomes ' him'/' her' of the hero it is
  told about". `lines.dream.counted` and `lines.dream.stage_done` take "stage task (told)"; only
  `lines.heir.takes_dream` says whom ("told about the dream owner").
- **Underdetermined:** for a carried dream, whether the progress lines tell the task about
  the bearer or the owner. Only WORTHY_STUDENT's "See a student succeed without you" ends in
  " you", so it shows only in "What is left: see a student succeed without him|her".
- **Port's choice:** the bearer — the hero the line is about (`src/witness.rs` `advance`).
- **Question:** which hero `lineage/tale.jai:426,431` passes to the told-task call.

## KG-15 — the dream deeds' fields

- **Spec says:** §9.3 "a DREAM_STEP deed"; §9.4 "a DREAM_FULFILLED deed"; §14.1 "a LEFT_LEGACY
  deed"; a deed is (kind, year, age, place, weight, other, telling); the tellings are
  `lines.deed.dream_step`, `.dream_fulfilled`, `.left_legacy`.
- **Underdetermined:** each deed's place, weight and other. Nothing the spec describes reads
  them (the epitaph, W9, reads other deed kinds).
- **Port's choice:** year now, the hero's age; DREAM_STEP and DREAM_FULFILLED take the quest's
  place when a quest moment moved the dream and none for a winter or a turning;
  LEFT_LEGACY no place; weight 0 and no other hero for all three, as KG-12 chose for the fear
  deeds (`src/witness.rs`, `src/legacy.rs`).
- **Question:** `lineage/tale.jai:432,451` and `lineage/legacy.jai:184` — the recorded fields.

## KG-16 — the order of a blessing's recipients

- **Spec says:** §14.1 "Recipients: F, all descendants of the dreamer, all descendants of F,
  and for WORTHY_STUDENT everyone F taught; each living recipient receives it unless they
  already have a blessing with the same title. `lines.legacy.blessing` names the newly blessed."
- **Underdetermined:** the order within "all descendants" and "everyone F taught", which is the
  order the name list reads in.
- **Port's choice:** the groups in the order listed; descendants in creation order; the taught
  in F's bond order; a hero already given it earlier in the walk is skipped by the title rule
  (`src/blessing.rs` `bless`). The W3 oracle — "Garrick, Maren and Pip" — is creation order,
  which every reading of the rule gives for the founding household.
- **Question:** how `lineage/legacy.jai:160-178` walks descendants.

## KG-17 — the place word in a blessing's "at" effect

- **Spec says:** `legacies.json` `blessing_effects.AT_PLACE` = "+% at %" with no argument notes.
- **Underdetermined:** the place's `title` ("The Drowned Coast") or `name` ("the Drowned
  Coast").
- **Port's choice:** `name`, the mid-sentence form `content/README.md` defines it as — "+2 at
  the Drowned Coast" (`src/blessing.rs` `blessing_effect`).
- **Question:** `lineage/legacy.jai` — which place field the effect prints.

## KG-18 — the order of a new heirloom's lines

- **Spec says:** §14.1 HEIRLOOM: "forge it ... Give it to F (§14.2). Line
  `lines.legacy.heirloom`." §14.2's giving writes `legacy.hands_full` or
  `legacy.hung_over_hearth`.
- **Underdetermined:** whether the giving's line comes before or after "It leaves an heirloom".
- **Port's choice:** the order §14.1 lists them: the giving's line, then the heirloom's
  (`src/legacy.rs` `leave_legacy`).
- **Question:** in `lineage/legacy.jai:130-185`, is the give call (`:111-128`) before or after
  the line at `:146`?

## KG-19 — the heir of the blood on equal born years

- **Spec says:** §14.2 "among all candidates found choose the earliest born_year".
- **Underdetermined:** a tie (twins, or two cousins of one year).
- **Port's choice:** the first found in the walk (children in bond order, depth first), as
  `firstborn` (§3.2) keeps the first on ties (`src/legacy.rs` `heir`).
- **Question:** `lineage/legacy.jai:281-304`'s comparison, `<` or `<=`.

## KG-20 — which dreams a dream rival is judged by

- **Spec says:** §12.3 "When a hero gains a dream ... if that dream is dreamt and
  unfulfilled: every other living adult with an unfulfilled dream of the same kind ...".
- **Underdetermined:** (a) whether "an unfulfilled dream" of the other hero includes a burden
  they carry; (b) for a hero who gains a burden (a passed dream), which dream is compared.
- **Port's choice:** (a) the other's own dream only (`Hero::dream`, the field the model calls
  "dream"); (b) the dream gained, whichever slot it lands in — the caller passes it
  (`src/rivals.rs` `dream_rivals`). The rival line's title is told for the gainer (owner's
  pronoun if owned).
- **Question:** `lineage/bond.jai:221-239` — does it read `hero.dream` only?

## KG-21 — an heirloom hung over the hearth, at the start of a sentence

- **Spec says:** `lines.legacy.hung_over_hearth` = "% is hung over the hearth. ..." with
  argument "old heirloom name"; `lines.heir.buried_with` says "heirloom name, first letter
  capitalized" for the same position.
- **Underdetermined:** whether a lower-case name ("the Hale cradle-ring") is capitalised here.
- **Port's choice:** as given, uncapitalised, since this key's argument does not say otherwise
  (`src/legacy.rs` `give_heirloom`). Every authored or forged name but the cradle-ring starts
  upper-case.
- **Question:** `lineage/legacy.jai:123`.

## KG-22 — the card's "Dream:" names: order and separator

- **Spec says:** §5.4 "'Dream: <names>' for every adult whose dream (or burden) this quest would
  advance"; `ui.quest_card.dreamers` argument "called dreamers, comma-separated". MODULES.md's
  W3 oracle: the line "include[s] Garrick (Ysolde is also called ...)".
- **Underdetermined:** the order of the names, and whether "comma-separated" is ", " or the
  name-list's "a, b and c".
- **Port's choice:** living adults in creation order, joined with ", " — the lore's separator,
  as KG-7 chose for the fear line (`src/calls.rs` `dreamers_line`). Garrick seated on "Grave
  goods" in year 1 reads "Dream: Garrick, Ysolde".
- **Question:** `lineage/quest-card.jai:130-139`.

## KG-23 — the legacy promise under a fulfilled dream

- **Spec says:** §19.1 DREAM: "... or 'Fulfilled.'; hover gives the legacy promise"
  (`legacies.json` `promises`: "If it is ever done, it will leave ...").
- **Underdetermined:** whether a fulfilled dream still offers its promise, which reads oddly
  once it is done.
- **Port's choice:** unchanged from session 1 — the promise is shown under every dream, done
  or not (`src/sheet.rs` `dream_lines`); W3 made "Fulfilled." reachable without deciding this.
- **Question:** `lineage/sheet.jai`'s hover condition on the dream section.

## KG-24 — a house tale's "about whom" and "since"

- **Spec says:** §3.1 tales are "(title, about whom, since)"; §14.1 "append a house tale titled
  by the dream kind's format with the dreamer's name".
- **Underdetermined:** whether "about whom" is the dreamer or the fulfiller, and what "since"
  records. Nothing W3 builds reads either (the tally counts; §14.3 reads the newest title).
- **Port's choice:** the dreamer, as the title names them; since = the year it was left
  (`src/legacy.rs` `leave_legacy`).
- **Question:** `lineage/legacy.jai:150-155`.

---

## KG-25 — which odds the card's four percentages are

- **Spec says:** §5.4 the card shows "an odds bar and the four outcome percentages";
  `ui.quest_card` has "Succeed %\%", "triumph %\%", "Setback %\%", "disaster %\%".
  CONSTANTS §3's table ends in a column "Success or better (as shown)". §5.4's quest sheet
  shows "each outcome with its percentage and consequence".
- **Underdetermined:** whether the card's "Setback" is setback alone or setback or worse
  (the table pins only "Succeed" as success or better), and whether the sheet's four are
  each band alone. Rounding each band separately and rounding a sum differ (+1: success
  alone 56%, triumph 17%, success or better 72%).
- **Port's choice:** the card reads as two halves, each a sum over its side and then its
  extreme alone: "Succeed" = percent(success + triumph), the table's "as shown" column;
  "triumph" = percent(triumph); "Setback" = percent(setback + disaster); "disaster" =
  percent(disaster) (`src/forecast.rs` `card_percentages`). The sheet prints each band
  alone beside its consequence, and 0% for all four with nobody going (SPEC §6: an empty
  party has all chances 0) (`src/quest_sheet.rs`). W4's oracle asserts the card against
  the table at the card's margin; the "Setback" figure is the one this choice decides.
- **Question:** what `lineage/quest-card.jai:107-112` passes to the four strings, and what
  `:218-226` prints beside each consequence.

## KG-26 — the power breakdown's arguments and numbers

- **Spec says:** §6 lists the seven member lines, the floor, the bonds and the patrons; §5.4
  "the full power breakdown (§6)". `ui.quest_sheet.power_line_*` give the strings:
  `power_line_member` "%, % %" (name, aptitude title, value) has its arguments; "  carries %",
  "  fears %", "  has conquered %", "  blessed: %" do not say what fills them; nothing says
  how a line's number is shown, and `quest_sheet.you_bring` ("You bring") takes no argument.
- **Underdetermined:** the four arguments, and where each line's number and the total go.
- **Port's choice:** the heirloom's name ("  carries Thornfall"), the tag's `noun` ("  fears
  deep water", "  has conquered the dark" — `content/README.md` calls `noun` "the lower-case
  phrase used in sentences"), the blessing's title ("  blessed: Garrick's rest"). The member
  line carries its number in its own text; every other line has its signed number in a
  right-hand column, and "You bring" has the total there (`src/power_lines.rs`,
  `src/board_view.rs`). The power is unchanged by any of this: `party_lines` asserts its
  lines add up to `party_power`.
- **Question:** `lineage/quest.jai:159-210` — the arguments of `:163,167,171,183` and how
  the number of each line is printed.

## KG-27 — a place's history before the house has quested there

- **Spec says:** §5.4 the sheet ends with "the place's history (visits, triumphs, disasters,
  and every hero who fell there with their fate telling)"; `quest_sheet.not_quested` is "The
  house has not quested here yet." and `quest_sheet.history` "Quested here %: % in triumph, %
  in disaster." (args: count words of visits, triumphs, disasters).
- **Underdetermined:** whether the fallen are listed when there have been no visits — the
  Drowned Coast took Elsbeth before the first year and has never been quested — and whether
  triumphs and disasters are count words too.
- **Port's choice:** "not quested here yet" when visits are 0, then the fallen whatever the
  visits (the Coast reads "The house has not quested here yet." then "Elsbeth Thorne was lost
  at the Drowned Coast."); triumphs and disasters as numerals, since only the visits'
  argument says "count words" (`src/quest_sheet.rs` `place_history`).
- **Question:** `lineage/quest-card.jai:230-240` — is the fallen loop inside the visits branch?

## KG-28 — the drag: the hand's own seat, the card's body, and what the preview covers

- **Spec says:** §5.3 a drop onto an empty slot moves, onto an occupied slot swaps,
  "releasing anywhere else returns the hero to where they came from". §5.4 "while a hero is
  being dragged over a quest card that has room and they would not refuse, the card's
  forecast includes them".
- **Underdetermined:** (a) whether the hero in hand still counts on the quest they were
  lifted from; (b) what a release over a card with room, but not over one of its seats,
  does — §5.4 previews them there and §5.3 would return them; (c) which seat the preview puts
  them in, which orders the fear line; (d) whether "the forecast" means only "you bring" and
  the odds, or the card's other party-derived lines too (the Dream: and Fear: lines, the
  highlighted tags).
- **Port's choice:** (a) the hand has left its seat: the quest it came from forecasts without
  it until it is released (and over its own card it lands back in its own seat); (b) one
  answer for the preview and the release, `House::landing`: a seat lands on that seat, a
  seated hero's tile on their seat (a swap), a card's body on its first free seat — so a
  card never previews a hero the release would send back (`src/board.rs`); (c) the hovered
  empty seat, else the first free one; (d) the whole card reads the previewed party, so the
  Fear: line and the tags light up for the hero in hand (`src/quest_card.rs`
  `preview_seats`). A full card previews nothing, per "has room", though a release on a
  seated hero there still swaps.
- **Question:** `source/core/ui/slot-group.jai:13-59` and `lineage/quest-card.jai:31-46` —
  is the card's body a drop target, and does the preview add the hero to the party the whole
  card reads?

## KG-29 — the glue between "No one is going." and "Room for N."

- **Spec says:** §5.4 "No one is going. Room for N."; `quest_card.no_one` and
  `quest_card.room_for` are two strings.
- **Underdetermined:** whether they share a line, and with what between them.
- **Port's choice:** one line, joined by a space, as §5.4 quotes it (`src/quest_card.rs`).
- **Question:** `lineage/quest-card.jai:120-121`.

---

## KG-30 — the remembered pair when no pair can be answered, and the order pairs are read in

- **Spec says:** §5.2 "for every ordered pair of distinct quests (i, j) ... *Answerability* =
  the maximum over pairs; the pair that first attains it is remembered (first, second)";
  easing then works on "the remembered pair". OQ-33: comparisons are strict, ties resolve to
  the first candidate. Sorting by place happens at "Finishing", after reading and easing.
- **Underdetermined:** (a) whether a pair is remembered when every pair reads 0 — "first
  attains" a maximum of 0 is (0, 1) if the running best starts below 0, and no pair at all if
  it starts at 0, which would leave easing nothing to ease; (b) which order "first" walks:
  the quests as planned, or as they will be sorted.
- **Port's choice:** (a) the first pair is remembered even at 0, so an unanswerable board is
  eased (`src/reading.rs` `read_board`: the running best starts empty, and the first pair
  takes it); (b) planning order — forced quests, then the ghost's, then drawn places in
  draw order — with i outer and j inner, since the board is sorted only when it is finished.
  The W5 checks' staged all-wounded house (every pair 0) is eased on (0, 1).
- **Question:** `generation/quest-context/quest-context.jai:93-116` — the running best's
  initial value, and the loop order.

## KG-31 — whom a ghost premise's task is told about

- **Spec says:** `content/README.md` "`{task}` = the told current stage task of the ghost's
  dream (SPEC §9.2)"; §9.2's told task turns a trailing " you" into " him"/" her" "of the hero
  it is told about"; `{He}`, `{he}`, `{him}` are the dead hero's.
- **Underdetermined:** the hero the task is told about — the dead hero, or the dream's owner
  when the ghost carried someone else's dream (KG-14's question, for a ghost).
- **Port's choice:** the dead hero, whose pronouns the rest of the premise uses (`src/ghost.rs`
  `ghost_text`). Only WORTHY_STUDENT's "See a student succeed without you" ends in " you".
- **Question:** `lineage/ghost.jai:55-76` — which hero the told-task call is given.

## KG-32 — whether "a dreamer who could go" counts the house's patrons

- **Spec says:** §5.2 reading: "success is P(SUCCESS or TRIUMPH) of the forecast (§6) with the
  house's patrons"; §9.6 the board reader's call: "the likely party built with d as leader ...
  has P(SUCCESS or better) >= 0.35", with no word on patrons. A likely party is chosen by solo
  power with patrons 0.
- **Underdetermined:** whether the 0.35 test adds the patrons the answerability test does.
- **Port's choice:** yes — the same success function as the reading, patrons included
  (`src/reading.rs` `could_go`), since the paragraph defines "success" once and the card the
  player reads counts the patrons. No house has a patron before W6 crowns one.
- **Question:** `generation/quest-context/quest-context.jai:118-132` — the patrons argument.

## KG-33 — a ghost's quest that also calls a dreamer

- **Spec says:** §5.4 the card shows "then either 'The ghost of <name>' or 'Dream: <names>'".
- **Underdetermined:** which a ghost's quest shows when it also calls a living dreamer (a
  ghost's quest at the Barrow calls Garrick's "triumph at the Barrow" like any Barrow quest).
- **Port's choice:** a ghost's quest shows the ghost line and no Dream: line; any other quest
  the Dream: line (`src/quest_card.rs` `read_card`). The quest sheet's call lines are not
  affected: it lists every call either way. The board reader still counts a ghost's quest's
  callers (§9.6 makes no exception).
- **Question:** `lineage/quest-card.jai:130-139` — the condition between the two lines.

---

## KG-34 — the summer's deeds' fields

- **Spec says:** §7.1 step 4 a FIRST_QUEST deed "(place, weight = danger)"; §7.3 a TRIUMPH deed
  "(place, weight = danger)"; §7.4 WOUNDED, SURVIVED_DISASTER, MENDED and CROWNED deeds; §12.2
  BEFRIENDED deeds "both"; §14.4 LAID_GHOST for each living member. A deed is (kind, year, age,
  place, weight, other, telling); `lines.deed.*` give the tellings.
- **Underdetermined:** the place, weight and other of the six deeds the spec gives no fields for.
  INVENTORY.md lists all six as written and never read.
- **Port's choice:** dated now at the hero's age, at the quest's place, weight 0; BEFRIENDED's
  other is the friend (its telling names them), every other deed's none — as KG-12 and KG-15
  chose for the fear and dream deeds (`src/harm.rs` `deed`, `src/road.rs`).
- **Question:** the `record_deed` calls at `lineage/tale.jai:253,294,314,330`,
  `lineage/bond.jai:184` and `lineage/ghost.jai:103`.

## KG-35 — a hero broken mid-summer, seated on a later quest

- **Spec says:** §7.1 step 1 forecasts "with the current party"; OQ-6: an earlier quest's death
  "grieves (adds dread, can break) heroes seated on later quests". §10.4: a broken hero's fear
  "still costs power if they are somehow on such a quest (only possible if they break during the
  Door, §16.2)". §5.3 bounces refusers "after every frame" of the summer scene.
- **Underdetermined:** whether a hero broken by grief on an earlier page, and seated on a later
  quest carrying the tag, still goes — §10.4's "only possible at the Door" says it cannot happen,
  OQ-6 says it can.
- **Port's choice:** the smallest: each party is read off its seats as it resolves and nothing
  bounces between quests (resolution is one call, no frame passes), so they go; the fear costs
  power, and facing it skips them as broken (`src/resolve.rs` `set_out`).
- **Question:** does `lineage/tale.jai:46-80` re-check refusal per quest?

## KG-36 — when the Meanwhile page exists, and the closing line

- **Spec says:** §8 pages: "then 'Meanwhile' (unanswered lines, unanswered total, home healing)
  ... if there is nothing at all, a 'A quiet summer' page"; "If the house has closed, the Meanwhile
  page ends with `ui.telling.house_closed`." The house closes only on leaving the telling (§2.1).
- **Underdetermined:** (a) whether Meanwhile is shown when it holds no line; (b) what "has
  closed" means on the screen that comes before the closing; (c) where the closing line goes when
  every quest was answered and nobody healed (a disaster can spend the renown with no Meanwhile
  line).
- **Port's choice:** (a) Meanwhile exists when it has a line; (b) "has closed" is renown at 0,
  which leaving will act on; (c) the closing line counts as a line, so such a summer gets a
  Meanwhile page holding only it. "A quiet summer" only when there is no quest page and no
  Meanwhile line (`src/telling_view.rs` `meanwhile_lines`, `leaves`).
- **Question:** `scene/scenes/telling.jai:145-172,229-237`.

## KG-37 — the typewriter, the leaf buttons and the skip

- **Spec says:** §8 the story is "typed out at 90 letters per second; the next-button first
  completes it", "then the outcome word, 'Needed ...', then the page's lines"; "'Go on' (reveal,
  then next leaf), numbered page buttons (free navigation), 'Skip ahead' (leave immediately)".
- **Underdetermined:** (a) whether the outcome, the roll and the lines show while the story is
  still typing; (b) whether a leaf shown again types its story again; (c) whether the numbered
  buttons number pages or leaves; (d) the typewriter's clock.
- **Port's choice:** (a) they wait until the story is whole — "then" read as sequence; (b) yes,
  typing restarts whenever a leaf comes on screen, by "Go on" or by a button; (c) leaves, so every
  leaf can be reached; (d) the frame clock's ticks since the leaf came on screen, at
  `TYPEWRITER_LETTERS_PER_SECOND` — render-side only, the house never reads it
  (`src/telling_view.rs`, `src/pointer.rs` `press`). "Skip ahead" leaves from any leaf, typing or not.
- **Question:** `scene/scenes/telling.jai:67-69,97` and the page-button handler.

## KG-38 — the crowned's bequest without an heirloom

- **Spec says:** §7.4 Crowned: "If they hold an heirloom: the heir is their nearest kin (§15.3);
  the bequest records the heirloom, the heir and decided."
- **Underdetermined:** whether a crowned hero with no heirloom has a decided bequest (W8's death
  page and W9's epitaph read it; the crowned never get a death page).
- **Port's choice:** as written — recorded and decided only with an heirloom (`src/harm.rs` `crown`).
- **Question:** `lineage/tale.jai:332-348` — is `decided` set inside the heirloom branch?

## KG-39 — a hero several heir ranks fit

- **Spec says:** §15.1's heir table: children 0, other descendants 1, spouse 2, "kin (siblings by
  a shared parent) and D's parents" 3, those D taught 4, a steadying bond 5, everyone else 6;
  "ordered by rank, then creation order".
- **Underdetermined:** the rank of a hero two rows fit (a spouse D taught; a parent who is also a
  friend), and how "D's parents" is known (a PARENT bond, or the parents field).
- **Port's choice:** the first row that fits, top down — the lowest rank; a parent by either the
  bond or the dead's parents field (`src/harm.rs` `heir_rank`). W6 reads the list only for the
  crowned's nearest kin (§15.3); W8's death page will read it whole.
- **Question:** `lineage/passage.jai:186-217`.

## KG-40 — "the first time ever" for a founder

- **Spec says:** §7.1 step 4 "quests_faced +1; the first time ever, record a FIRST_QUEST deed".
  Founders arrive with `quests_faced` from `household.json` (Garrick 30) and no deeds; the
  epitaph's ROADS part has a branch for "No FIRST_QUEST deed" (§20).
- **Underdetermined:** whether a founder who quested before year 1 records a FIRST_QUEST on their
  first quest of the run.
- **Port's choice:** no — the first time ever is the count reaching 1, which a founder with
  quests behind them never does; the epitaph's no-deed branch is then theirs
  (`src/resolve.rs` `resolve_rolled`).
- **Question:** `lineage/tale.jai:163-169` — the condition on the deed.

---

## KG-41 — the hearth's preview mid-drag, and what a release lands on

- **Spec says:** §11.2 "Every winter seat ... is a drag-and-drop slot with the swap rules of §5.3"; "The
  winter screen previews each seat's result". §5.3: releasing anywhere else returns the hero. KG-28
  settled the summer's card preview.
- **Underdetermined:** (a) what the previews read while a hero is in hand — the seating as it is, or as
  the release would make it; (b) whether a group of seats (the panel round a pair) is a drop target, as a
  quest card's body is (KG-28 b).
- **Port's choice:** (a) the seating the release would make: the hand at its landing, a swap's displaced
  hero where the hand came from, or — landing nowhere — the hand lifted out of its seat, as KG-28 (a) has
  it; every note on the screen is read off that seating's plan (`src/hearth_view.rs` `previewed`); (b) no:
  only a seat or a seated hero is a target, so a release on a panel returns the hero (`House::landing`).
- **Question:** `scene/scenes/winter.jai:16-138` — what the seat notes read during a drag, and whether
  the groups take drops.

## KG-42 — a fire seat that would both heal and calm

- **Spec says:** §11.2 the fire previews "heals"/"calms"/"rests"; §11.3 step 1 a rest heals the wound and
  sheds dread, with three lines for healed, calmed, both.
- **Underdetermined:** which word a wounded hero with dread to shed previews.
- **Port's choice:** "heals" — the wound first, then "calms", else "rests" (`src/plans.rs` `Rest::note`).
  The resolution is unaffected: both happen, and `winter.rest_both` is written.
- **Question:** `lineage/hearth.jai:183-188` — the order of the three notes' conditions.

## KG-43 — the winter's deeds' fields

- **Spec says:** §11.5 step 4 "the first time, a TAUGHT deed"; §11.3 step 4 "WED deeds"; step 5
  "records TOLD_THE_TALE". A deed is (kind, year, age, place, weight, other, telling); `lines.deed.taught`
  names the learner, `lines.deed.wed` the spouse.
- **Underdetermined:** each deed's place, weight and other; and "the first time" for a teacher.
- **Port's choice:** dated now at the hero's age, no place, weight 0; TAUGHT's other is the learner and
  WED's the spouse (their tellings name them, as BEFRIENDED's names the friend, KG-34), TOLD_THE_TALE's
  none. "The first time" is `winters_taught` reaching 1, as KG-40 reads "the first time ever"
  (`src/winter.rs` `credit`, `court`, `tell_the_tale`).
- **Question:** the `record_deed` calls at `lineage/hearth.jai:251,284,318`.

## KG-44 — the training yard's and a bench's note with no learner

- **Spec says:** §11.2 "training — the planned lesson ('+N Aptitude' or the excuse) ... benches — the
  planned lesson or excuse"; `ui.winter.no_learner` ("no learner") and `ui.winter.no_child` ("no child")
  exist; §11.3 writes `winter.waits_for_learner` / `bench_no_child` for a teacher alone.
- **Underdetermined:** when the two "no ..." notes show, and where a pair's one note is drawn.
- **Port's choice:** a pair's note is drawn beside its two seats: the lesson when the learner's (child's)
  seat is filled, "no learner" ("no child") when only the teacher sits, nothing when both are empty — the
  same cases §11.3 tells (`src/hearth_view.rs` `notes`).
- **Question:** `scene/scenes/winter.jai:68-106`.

## KG-45 — a crowned spouse, in the garden

- **Spec says:** §11.6 "either already has a **living** spouse → WED_ALREADY ... Widowed heroes may
  remarry." §3.2 fate is LIVING, DEAD or DEPARTED (crowned, gone to Court).
- **Underdetermined:** whether a hero whose spouse was crowned is wed already.
- **Port's choice:** "living" is fate LIVING (`Hero::is_living`), as everywhere else the spec says living,
  so a hero whose spouse departed to Court may wed again (`src/plans.rs` `has_living_spouse`).
- **Question:** `lineage/hearth.jai:166` — the spouse test.

## KG-46 — "ten winter seats" and the twelve the hearth lists

- **Spec says:** §1 "a **winter** in which the player seats heroes at ten winter seats";
  `ui.winter.help` "There are ten seats and one winter." §3.1 `hearth` lists fire[2], training[2],
  courting[2], tellers[2], benches[4] — twelve — and CONSTANTS §9 gives the same counts.
- **Underdetermined:** whether the original has ten seats (and §3.1's list overcounts), or twelve
  (and "ten" counts something else — a bench as one seat, say).
- **Port's choice:** twelve, as §3.1 and CONSTANTS §9 list them and §11.3 resolves them; the help is
  shown as shipped, "ten" and all (`src/hearth.rs` `Seat::all`).
- **Question:** `lineage/hearth.jai:1-21` — the hearth's arrays, and what the help's "ten" counts.

---

## KG-47 — the heir list fixed, its marks read when shown

- **Spec says:** §15.1 "The heir list is fixed when the page is made (before this turning's births,
  comings of age and arrivals). Each heir button reads `lines.heir.prospect` ... plus '(not the dream)'
  if the dead leaves a dream the candidate cannot take, else '(lays one aside)' if both hold heirlooms."
- **Underdetermined:** whether the marks are fixed with the list or read when the buttons are drawn. They
  differ when one turning has two death pages: the heir chosen on the first page takes a dream as a burden
  (or an heirloom), and on the second page can then no longer take one — a mark fixed earlier would say
  they can, and the choice would raise a ghost instead.
- **Port's choice:** the list (who, in what order) is fixed when the page is made; the kinship words and the
  marks are read each time the page is drawn, from the same `can_take_dream` the choice obeys, so a button
  never promises what pressing it will not do (`src/heirs.rs` `heir_buttons`).
- **Question:** `scene/scenes/turning.jai:120-146` and `lineage/passage.jai:229-240` — are the button
  labels built once with the page, or every frame?

## KG-48 — the turning's deeds' fields

- **Spec says:** §15.2 an INHERITED deed and a TOOK_UP_DREAM deed; §17.3 CHILD_BORN deeds for both parents;
  §17.5 a CAME_OF_AGE deed; §17.2 "ARRIVED deed dated next year". A deed is (kind, year, age, place,
  weight, other, telling); the `lines.deed.*` tellings name the giver, the dream's owner and the child.
- **Underdetermined:** each deed's place, weight and other; the year of all but ARRIVED.
- **Port's choice:** dated the turning's year (ARRIVED the next, as stated) at the hero's age, no place,
  weight 0; `other` is the hero the telling names — the dead for INHERITED, the dream's owner for
  TOOK_UP_DREAM, the child for CHILD_BORN — and none for CAME_OF_AGE and ARRIVED, as KG-43 chose for the
  winter's (`src/heirs.rs` `deed`). The epitaph (W9) reads ARRIVED's year and age, which the spec fixes.
- **Question:** the `record_deed` calls at `lineage/passage.jai:267,308,377,436,471`.

## KG-49 — which dreams a rolled dream counts as claimed

- **Spec says:** §17.4 "claimed = kinds dreamt by any other living hero; WORTHY_STUDENT also counts as
  claimed if the hero is younger than 35."
- **Underdetermined:** whether "dreamt by" reads a hero's own dream only or a carried burden too, and
  whether a fulfilled dream still claims its kind (a dream is "dreamt" iff its title is non-empty, §3.2,
  which a fulfilled dream's still is).
- **Port's choice:** the own dream, fulfilled or not (`src/newcomers.rs` `rolled_dream`) — the field the
  model calls "dream", as KG-20 read §12.3, and "dreamt" as §3.2 defines it.
- **Question:** `generation/hero-context/hero-context.jai:46-59` — does the claimed loop read `hero.dream`
  only, and test `is_dreamt` or `!is_fulfilled`?

## KG-50 — the count in "born under N blessings"

- **Spec says:** `lines.birth.blessing_many` = "% is born under % blessings: %." with argument "blessing count".
- **Underdetermined:** numeral or count word ("2" or "two").
- **Port's choice:** the numeral, as `lines.tales.many`'s "tale count" (`src/births.rs` `born`). The count
  words of §21 are "never", "once", "twice" — times, not quantities.
- **Question:** `lineage/passage.jai:398`.

## KG-51 — how far the turning lets the reader go while a page waits

- **Spec says:** §18.1 "While a page is undecided: 'Go on' is disabled, page buttons beyond it do nothing,
  and 'Skip ahead' jumps to the first undecided page instead of leaving."
- **Underdetermined:** (a) whether "Go on" is disabled on every leaf while any page waits (which would leave
  a reader on the winter's page unable to reach the death page but by "Skip ahead") or only where going on
  would pass the waiting page; (b) "beyond it" for a death page of several leaves; (c) which leaf of the page
  "Skip ahead" lands on.
- **Port's choice:** the reader may go anywhere up to the first undecided page's last leaf — where its choice
  is drawn — and no further: "Go on" is greyed and does nothing there, a leaf button past it does nothing,
  "Skip ahead" lands on it (`src/pointer.rs` `press`, `src/turning_view.rs` `furthest`).
- **Question:** `scene/scenes/turning.jai:45-74,150-170`.

## KG-52 — the vengeance line names the place twice

- **Spec says:** `lines.age.avenge_dream` = "% has a dream, and the whole house knows why: %. % % at %. It was
  %." — the lost parent's full name, fate telling, the place's name, the tag's noun.
- **Underdetermined:** every questing fate telling already names the place (`fate.fell` "fell at %",
  `fate.died_of_wounds`, `fate.burned`), so the line reads "Brannoc Hale fell at Emberfall at Emberfall."
- **Port's choice:** the arguments as listed, the repetition and all (`src/coming_of_age.rs`
  `take_a_dream`); content is identical by contract.
- **Question:** `generation/hero-context/hero-context.jai:196` — is the second argument the fate telling, or
  something shorter (a verb alone)?

## KG-53 — dream rivals at a coming of age for a dream already held

- **Spec says:** §9.5 lists the coming of age's dream by priority, "1. already has a dream ... keep it; ...
  Then dream rivalry is checked (§12.3)." §12.3 is checked "when a hero gains a dream".
- **Underdetermined:** whether a kept dream is checked for rivals again — a child handed a dream by a death
  page is checked when it is passed (§15.2), and a child's own dream may meet a new rival since.
- **Port's choice:** checked whichever priority gave the dream, as §9.5's "then" reads (`src/coming_of_age.rs`
  `come_of_age`); a rival already made is not made twice (§12.3 needs no bond but a companion's).
- **Question:** `generation/hero-context/hero-context.jai:179-223` — is the rival check inside each branch
  or after them all?

## KG-54 — the Door's "firstborn living child"

- **Spec says:** §15.4 "the firstborn **living** child (CHILD bonds, earliest born_year) gets
  OPEN_THE_SEALED_DOOR".
- **Underdetermined:** the earliest born of the living children, or the firstborn child only if living
  (falling to "no child" when the firstborn is dead and a younger child lives).
- **Port's choice:** the earliest born among the living, first found on ties (`src/death_page.rs`
  `door_promise`), as the emphasis on "living" reads and KG-19 breaks ties.
- **Question:** `lineage/passage.jai:313-334` — is `is_living` inside the earliest-born loop?

## KG-55 — a death page with something to leave and no one living

- **Spec says:** §15.1 step 7 "If holding an heirloom or leaving a dream: gather heirs and leave the page
  undecided"; "There is always 'No one. Let it lie.'"
- **Underdetermined:** a page whose heir list is empty (the last of the house).
- **Port's choice:** it waits all the same, on "No one. Let it lie." alone, which buries the heirloom and
  raises the ghost (`src/passage.rs` `Bequest::leaves`). Reachable only when the house has no one living.
- **Question:** none needed unless the original decides such a page itself.

## KG-56 — a phase line for a hero new this turning

- **Spec says:** §18 step 9 "For each living hero: if their phase changed this turning and the new phase is
  not Youth, `lines.phase.changed`".
- **Underdetermined:** a newborn or a wanderer had no phase before the turning.
- **Port's choice:** no phase before, no change (`src/turning.rs` `turn_the_year`); a newborn is a Child at 0
  and a wanderer arrives into whatever phase their age is, told on their own page.
- **Question:** `lineage/passage.jai:90-98` — where the previous phase is read from (a saved age, or
  `age - 1`, which would tell a wanderer of 20 that they are "in the prime of life now").

---

## KG-57 — the remembrance of the dead who await their death page

- **Spec says:** §19.2 "Pointing at a node shows the remembrance: the epitaph if composed (the dead, the
  crowned, and everyone at the Ending), otherwise for the living their station and `ui.family.living_*`
  lines." §15.1 composes a summer's dead's epitaph on their death page, at the turning.
- **Underdetermined:** what the family screen shows for a hero who died this summer — dead, so not "the
  living", and with no epitaph composed until the turning. The family can be opened from the telling and
  the hearth, where such a hero is pointed at.
- **Port's choice:** their name and their sheet's first lines — station, and the condition "Died in year N,
  aged A" — the stand-in session 1 used for every dead hero, now kept only for the dead the turning has not
  yet reached (`src/family.rs` `remembrance`). Once the page composes the epitaph the remembrance shows it.
- **Question:** `lineage/family-tree.jai:340-380` — what it shows when the epitaph is empty and the hero is
  not living (the epitaph's empty string, or the `living_*` lines regardless).

## KG-58 — the order of a wording's draws, and which coin is which part's

- **Spec says:** §20 "a frame 0..2 rolled uniformly but never equal to the previous frame rolled anywhere in
  the run, and nine independent fair coins (one per part, 0 or 1)"; `epitaph.json` `parts` lists the nine.
- **Underdetermined:** whether the frame is drawn before the coins, how a fair coin is drawn (`index(2)` or
  `chance(0.5)`), and whether the coins are indexed by `parts` or by another list (`priorities`, a frame).
  None of it moves a distribution; all of it moves the sequence after a wording.
- **Port's choice:** the frame first (`index(3)` for the run's first, `fresh_index(3, last)` after), then
  nine `index(2)` draws, coin `i` for `parts[i]` (`src/epitaph.rs` `roll_wording`); the loader refuses a
  `parts` list out of SPEC §20's order, since the coins are indexed by it.
- **Question:** `lineage/epitaph.jai:62-70`.

## KG-59 — ROADS's "exactly one"

- **Spec says:** §20 ROADS "No FIRST_QUEST deed → `roads.count_only`. Exactly one → `roads.once`. Else
  `roads.many_v`."
- **Underdetermined:** exactly one of what — quests faced, or FIRST_QUEST deeds (of which a hero has at most
  one, which would make `roads.many_v` unreachable).
- **Port's choice:** `quests_faced` is exactly one (`src/epitaph_parts.rs` `roads`), the reading that leaves
  every template reachable and matches `roads.once`'s words ("went out once").
- **Question:** `lineage/epitaph.jai:199-208`.

## KG-60 — the naming rule's pronouns and case

- **Spec says:** §20 "a leading 'He '/'She ' becomes the full name; a leading 'His '/'Her ' becomes
  '<full name>'s'; otherwise the first ' he '/' she ' inside becomes ' <full name> '".
- **Underdetermined:** whether the rule looks for both pronouns or the subject's own, and whether the inside
  search is case-sensitive (a later sentence's "She" is preceded by ". ", not " ").
- **Port's choice:** the subject's own forms, from the lore (`He `/`His `/` he ` for him, `She `/`Her `/` she `
  for her), and the inside search lower-case and case-sensitive, first occurrence only (`src/epitaph.rs`
  `name_the_subject`). No template carries another hero's pronoun, so the first half cannot change a word.
- **Question:** `lineage/epitaph.jai:118-140`.

## KG-61 — `ui.family.departed`, never shown

- **Spec says:** §19.2 the remembrance shows "the epitaph if composed (the dead, the crowned, ...)"; §7.4
  every crowning rolls a wording and composes. `ui-text.json` has `family.departed` = "% %, in year %, aged %.
  The house has had a patron at Court since." (`lineage/family-tree.jai:345`).
- **Underdetermined:** when `family.departed` is shown, since every departed hero has an epitaph.
- **Port's choice:** never: the crowned show their epitaph, as §19.2 says (`src/family.rs` `remembrance`).
  If the original shows `departed` in place of the epitaph, the crowned's remembrance is the one to change.
- **Question:** `lineage/family-tree.jai:340-346` — the condition on the `departed` line.

## KG-62 — the greater student, as they stand when the epitaph is composed

- **Spec says:** §20 PROPHECY "TEACH_A_GREATER: `prophecy.greater_came` naming the first student they
  taught ... whose best base aptitude value exceeds the teacher's best base value, else `prophecy.greater`."
- **Underdetermined:** a student who passes a dead teacher later — learning, a coming of age's teaching that
  shows, a mending — moves no recomposition point, so the epitaph keeps the sentence composed at the death.
- **Port's choice:** read when the epitaph is composed, and not recomposed for (`src/epitaph_ends.rs`
  `prophecy`). The W9 battery counts how often a held epitaph and its hero's composition now differ this
  way (0 in 160 houses).
- **Question:** does the original recompose epitaphs anywhere §20 does not list (the Ending's "deaths not
  yet given a page" suggests not)?

## KG-63 — ORIGIN's "has a parent", and the parents' names

- **Spec says:** §20 ORIGIN "born_year >= 1 and has a parent → `origin.born_v`. Has a parent →
  `origin.child_of_old`"; the templates' argument is "parent names".
- **Underdetermined:** whether a parent is known by the `parents` field or a PARENT bond (KG-39 met this for
  the heir ranking), and whether "names" are first or full.
- **Port's choice:** the `parents` field, in its order, first names as a name list — `content/README.md`'s
  "name" (`src/epitaph_parts.rs` `origin`). Every parent the rules make (§4, §17.3) is in both, so the two
  readings agree on every hero a run can make.
- **Question:** `lineage/epitaph.jai:163-173`.

## KG-64 — the heir a ghost taken up names, read by LEFT

- **Spec says:** §9.5 at a coming of age "the dead hero's bequest becomes PASSED_ON to this hero and their
  epitaph is recomposed"; §20 LEFT reads "an heir" for `left.legacy_and_heirloom`, `left.heirloom_made` and
  `left.heirloom_went`, and "decided without heir" for `left.buried_with`.
- **Underdetermined:** whether "PASSED_ON to this hero" records the taker as the bequest's heir (W8 chose
  yes, so DREAM's "fate PASSED_ON with an heir" can name them). If it does, a dead hero whose heirloom was
  buried and whose ghost a grandchild later takes up reads "<heirloom> went to <grandchild>" from then on.
- **Port's choice:** the heir is the bequest's, whoever recorded it (`src/epitaph_ends.rs` `left`), as both
  rules read it literally.
- **Question:** `generation/hero-context/hero-context.jai:200-210` — does the take-up set `bequest.heir`?

