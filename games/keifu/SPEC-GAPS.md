# Keifu — spec gaps

Places where `spec/SPEC.md`, `spec/CONSTANTS.md` or `spec/content/` did not determine
something this port needed. Each entry records the smallest conservative choice the port
made, the code site (grep for `SPEC-GAPS KG-n`), and the question for the spec-amendment
pass, which has the original and adjudicates every entry. Entries are never deleted; an
adjudicated one gets a **Resolved:** line.

Session 1 (W0 + W1): 6 entries.

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
