# Lineage — proposed module cut for the port

Dependency-ordered waves. Each module lists what it covers (with SPEC sections), what it
depends on, and one **oracle question**: a single thing the owner checks by playing the
original side by side with the port. Oracles use the authored opening so the answer is the
same on every seed where possible; where a roll is involved, the question is about the
shape, not the sequence.

Order: W0 → W1 → W2 → W3 → W4 → W5 → W6 → W7 → W8 → W9 → W10. W7 can be built in parallel
with W6 once W3 exists; W9 can start after W3 with stub inputs.

---

## W0 — Foundations and content

- **Covers:** loading `content/*.json`; lore tables (aptitudes, tags, places, phases,
  vocations, outcomes, pronouns); the year/season counter and "years until the Door"
  (SPEC §2.2); RNG primitives and the fresh/unclaimed pickers (§22.1); text conventions —
  name lists, year tellings, count words, capitalise/lower, writing pools with per-pool
  memory (§21); the Jai format convention (`content/README.md`).
- **Depends on:** nothing (the engine's PRNG, text and window).
- **Oracle:** Start a run. Does the top bar read "Year 1 of 25", "Summer", "House renown 15"
  and "The Sealed Door opens in 25 years. It will ask for four." with "Dark, Cold. Locks:
  Might 34, Wits 34, Spirit 34."?

## W1 — Heroes and the household

- **Covers:** the hero model (§3.2), derived quantities (phase, effective aptitude, adult,
  best aptitude, kin, descent, firstborn); the founding household (§4,
  `content/household.json`) in creation order; reseating the roster (§5.1); the hero card
  and hero sheet as information (§19.1); the family screen's membership (§19.2) without
  epitaphs.
- **Depends on:** W0.
- **Oracle:** Point at Garrick in year 1. Does his sheet show Might 5 (-2), Wits 5 (+1),
  Spirit 4, the dream "To lay the Barrow's dead to rest" with two steps done and "Win a
  triumph at the Barrow" current, fear Water with 2 dread and -3 power, "DESTINY, COME" for
  "Your child will surpass you.", Friend Odo +1, Daughter Maren +2, Wife Elsbeth gone, and
  the heirloom Thornfall +1 Might?

## W2 — Bonds, fears and destinies (state and pure rules)

- **Covers:** bond kinds, ranks, mirroring, form/change rules (§12.1); steadying; fear
  penalty, conquered bonus, refusal (§10.4); the dread rule, shedding, breaking, conquering
  (§10.2-10.4); grief (§12.5); destiny lore and predicates (§13): shields, claims, may-still-
  learn, speaking with the unclaimed rule.
- **Depends on:** W1.
- **Oracle:** Seat Maren and Garrick together on "The bell under the tide" in year 1. Does
  the card's fear line list both, in seat order, as "Maren -2, steadied" and "Garrick -3,
  steadied", and does it say "you bring 4" (1 + 1 + 2 for parent and child)?

## W3 — Dreams and legacies

- **Covers:** the dream model and the nine dreams (§9.1, `content/dreams.json`), predicates
  and moments, witnessing with "own dream first, one stage per moment" (§9.3), told
  titles/tasks (§9.2), fulfilment and settling (§9.4), dream calls (§9.6), legacies:
  heirlooms (blade names in order), tales, blessings and their recipients, giving a new
  heirloom and the heir-of-the-blood rule (§14.1-14.3), dream rivals (§12.3).
- **Depends on:** W2.
- **Oracle:** With Garrick seated on a Barrow quest, does the card's "Dream:" line include
  Garrick (Ysolde is also called to any place she has not walked) and the quest sheet say "Garrick's dream: Win a triumph at the Barrow. He must triumph."; and
  when he does triumph there, is he settled with the blessing "Garrick's rest: +2 against
  Undead" laid on Garrick, Maren and Pip?

## W4 — Quests and the forecast

- **Covers:** the quest model and stakes formula with trouble (§5.2 formula), power
  computation with every line in order and the per-member floor (§6), the 36-pair forecast
  and its bands, the quest card and quest sheet as information (§5.4), the live drag preview,
  the place history panel.
- **Depends on:** W3 (dream calls on cards), W2.
- **Oracle:** Seat Garrick and Brannoc on "Grave goods" in year 1. Does the card show "Needs
  Might" with a demand of 9, 10 or 11 (lower only if the board was eased), "you bring 12", and percentages matching the table
  in CONSTANTS.md §3 for power minus demand (+3, +2 or +1)?

## W5 — Board generation

- **Covers:** planning, reading (likely parties, ordered pairs), scoring, the welcome rule
  and the 16-attempt loop, easing, template memory, place sorting, forced opening quests,
  the first-ghost slot (§5.2, §14.4 quest part).
- **Depends on:** W4.
- **Oracle:** Restart several runs. Is year 1 always "Grave goods" at the Barrow and "The
  bell under the tide" at the Coast plus two other places, in place order (Barrow, Coast,
  Pass, Emberfall, Court, Deepwood), and does at least one quest each summer usually carry a
  "Dream:" mark for someone with a fair chance?

## W6 — Summer resolution and the telling

- **Covers:** set-out rules (§5.3), resolution order and every step (§7, §7.1-7.4):
  dice, history, rewards and lessons, trouble easing, disaster renown, facing fears, setbacks
  (unlucky pick, fire), disasters (fire, mending, death roll, wounds), sharing the road,
  witnessing, roads, crowns, ghost laying; unanswered costs (§7.2); healing at home; the
  telling pages and their line order (§8); house closure on leaving the telling.
- **Depends on:** W5, W3, W2.
- **Oracle:** Press "Stay home" in year 1. Does the telling's Meanwhile list four "No one
  went." trouble lines in place order and "-4 renown", leaving the house at 11; and next
  summer does any quest at one of those places show one seat fewer (never below one), +1
  danger, "Renown +" one higher, and "unanswered -2"?

## W7 — Winter

- **Covers:** opening the hearth (§11.1), seating anywhere (§11.2), resolution order
  (§11.3), lesson planning with every excuse (§11.4), teacher credit and the rank-limited
  mentor bond (§11.5), courtship (§11.6), tales at the table, rest, winter dream moments; the
  seat previews.
- **Depends on:** W3, W2 (bonds), W1.
- **Oracle:** In year 1's winter, put Odo on a bench as Pip's teacher. Does the preview read
  "+1 Spirit", and after the winter does Pip's Spirit rise to 3, Odo "has taken Pip as a
  student", and Odo's dream read "Teach the young two winters (1/2)"?

## W8 — The turn of the year, heirs and inheritance

- **Covers:** the turning order (§18): ageing, old age, death pages (§15.1), the Door
  promise (§15.4), births (§17.3), comings of age (§17.5), wanderers (§17.2), tales, phase
  lines, year-turn moments; the turning screen and heir choice (§15.2, §18.1); ghosts raised
  and taken up at coming of age (§9.5, §14.4); crowned heroes' nearest kin (§15.3); name
  and house bags (§17.1); rolled dreams (§17.4).
- **Depends on:** W7, W6, W3.
- **Oracle:** When Garrick dies of old age, does his death page offer, in this order, Maren
  (daughter), Pip (grandson), Odo (friend), then the rest of the living house in founding
  order (Ysolde, Brannoc, Wren, then any wanderers), each marked "(not the dream)" only if
  they already carry a burden — and does the year refuse to turn until one is chosen?

## W9 — Epitaphs and remembrance

- **Covers:** wording rolls with the no-repeat frame (§20), the nine parts and their
  selection rules, priority and the six-sentence budget, frame ordering, naming the subject,
  every recomposition point; the family screen's remembrance panel (§19.2).
- **Depends on:** W3 (dream fates), W8 (bequests), W1.
- **Oracle:** Open the family screen in year 1 and point at Elsbeth. Is her epitaph made of
  exactly these sentences, in one of the three frame orders, with the first one naming
  "Elsbeth Thorne": that she was lost at the Drowned Coast before the first year, aged 41;
  that she wanted to see the sea and died before it was done; that she loved Garrick Thorne
  and Maren; one of the two fear-of-high-places wordings; and "went out eleven times in
  all" only when the fear wording is the one-sentence one — with nothing about where she
  came from?

## W10 — The Sealed Door and the ending

- **Covers:** the last summer (§16.1), the Door's three-lock resolution with the bearer rule
  and carry-over between locks (§16.2), the Door outlook and best four (§16.4), the ending's
  remembering of the fallen and the living (§23), verdicts (Door 0-3 locks, or closed),
  "Begin another house".
- **Depends on:** W6, W8, W9.
- **Oracle:** In the last summer, seat four heroes on the Door. Do the Door card's three
  per-lock "you bring" numbers and "All three locks open: P in 100" match the original for
  the same four, and after trying, does the verdict's title match the number of locks that
  gave ("The Door stayed shut" / "opened a hand's breadth" / "stood half open" / "is open")?
