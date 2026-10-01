# Keifu — spec gaps

Places where `spec/SPEC.md`, `spec/CONSTANTS.md` or `spec/content/` did not determine
something this port needed. Each entry records the smallest conservative choice the port
made, the code site (grep for `SPEC-GAPS KG-n`), and the question for the spec-amendment
pass, which has the original and adjudicates every entry. Entries are never deleted; an
adjudicated one gets a **Resolved:** line.

Session 1 (W0 + W1): 6 entries. Session 2 (W2): 6 entries, KG-7 to KG-12. Session 3 (W3):
12 entries, KG-13 to KG-24.

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
