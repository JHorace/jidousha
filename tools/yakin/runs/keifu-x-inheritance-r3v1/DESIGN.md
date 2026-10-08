# DESIGN — keifu-x-inheritance-r3v1

task: keifu-x-inheritance-r3v1
variant: V1
model-as-configured: claude-opus-5-5 (the session's configured model; configured, not verified — the run page is the authority)
date: 2026-10-07 21:55 PDT

Read for this design: `games/keifu/spec/SPEC.md` §1–§18 (§3 state, §5.3–5.4
seating and the card, §6 power, §7 resolution, §9 dreams and §9.6 calls, §11.6
courtship, §12 bonds, §13–15 destinies, legacies, death pages and heirs, §17
births and wanderers), and mainline source `hero.rs`, `births.rs`, `calls.rs`,
`plans.rs` (courtship), `heirs.rs` (buttons, choose), `resolve.rs`
(`resolve_party`), `quest_card.rs`, `verify.rs`, `capture.rs`. Mainline
`games/keifu/` is read-only; nothing of it is written.

## What the game is

Keifu X Inheritance is a fork of Keifu (a port of Lineage: a house of heroes kept
for 25 years, then the Sealed Door) that leans into what a family carries
forward. Three things are added on top of a byte-identical copy: a heritable
**black mark** for failing a personal quest, a hard line between **family and
outsiders** (outsiders earn renown only for themselves until they marry in), and
rudimentary **trait genetics** (two trait slots per hero, one passed from each
parent by a seeded coin). Everything else plays as mainline.

## The player-facing loop

As mainline, with three new things read and decided:

1. **Summer, seating a quest.** When a seated hero's dream is called by the quest
   with "must succeed" or "must triumph" (mainline SPEC §9.6), the quest is that
   hero's **personal quest**. The quest card then shows a mark line: who would be
   marked, the chance of failure (setback + disaster %), the mark (+1 on a setback,
   +2 on a disaster), and what it costs the house's renown now; the quest sheet adds
   that heirs carry half. The player decides whether to send them, and commits with
   **Set out**.
2. **Winter, the garden.** An outsider seated in the garden with a family member
   needs personal renown >= 6 to wed. The garden preview reads either
   "<renown> of 6 renown" (refused) or "marries in: house +N" (accepted). The
   player commits with **Let the winter pass**.
3. **The turning, the death page.** Each heir button reads, beside mainline's
   label, what that heir would hold after the choice: their traits and their marks
   after inheriting. Choosing the button commits it.
4. Everywhere: the hero sheet shows blood (family/outsider), traits and marks.

## Systems

Build order. All files are under `games/keifu-x-inheritance-r3v1/` (the fork,
crate `keifu_x_inheritance_r3v1`); the engine surface touched is only what
mainline already uses (`docs/api/jidousha-api.md`: `Rng`, `HeadlessSim`,
`FrameRecorder`; `docs/api/jidousha-testing.md`: transcripts).

- **Fork** — copy-and-rename, its own commit, verify green (fork rule).
- **Hero state** — `hero.rs`: `blood: Blood { Family, Outsider }`, `genes:
  [Option<Trait>; 2]`, `marks: i32`. Founders are Family with authored genes
  (`genes.rs` table: the two founding children only — Pip `[Strong, -]`, Wren
  `[Bold, Clever]` — so no founding adult's power changes); wanderers are
  Outsiders; a child is Family if either parent is.
- **Genes** — `genes.rs`: `Trait { Strong, Clever, Devout, Bold }`; `traits(hero)`
  (distinct expressed traits, a trait in both slots is "true-bred" and counts
  twice); `child_genes(seed, child_id, a, b)` and `wanderer_genes(seed, id)`. All
  gene rolls draw from a **sub-generator** `Rng::from_seed(house.seed ^
  mix(hero id))`, never from the run's generator, so mainline's roll inventory
  (SPEC §22.2) is unchanged and every mainline seed replays the same events.
  Child: one slot from each parent, each chosen by a coin; then a 15% mutation
  rewrites slot (coin) with a uniform trait. Wanderer: each slot 50% a uniform
  trait. Effect (per copy): Strong/Clever/Devout +1 power on a quest of Might/
  Wits/Spirit; Bold reduces an unconquered fear's penalty by 1 (not below 0).
  Read in `power.rs::member_power` (and so in every forecast, preview and roll)
  and itemised in `power_lines.rs`.
- **Marks** — `marks.rs`: `personal(content, heroes, quest, party) -> Vec<HeroId>`
  (seated members whose dream/burden the quest calls with Succeed/Triumph,
  read through `calls::dream_call`); **the one function** `stain(heroes, hero,
  outcome) -> Option<Stain>` (`Stain { weight, house_loss, personal_loss }`:
  weight 1 setback / 2 disaster / none otherwise; house_loss = weight for family,
  0 for an outsider; personal_loss = min(weight, renown)); `mark_line` for the
  card and sheet; `apply` in `resolve.rs::resolve_party` after step 7, using the
  party called **at step 1** (before anything this quest changes). The house
  loses `house_loss`, the hero `personal_loss` and gains `weight` marks, a line
  `"<Name> fails <his> own quest: a black mark on the name (+W)."` goes on the
  quest page, and a FailedQuest deed is recorded. Never at the Door.
  - Stacking: marks add. Decay: a newborn carries `max(parents' marks) / 2`; an
    heir chosen on a death page gains `dead.marks / 2`; an outsider's marks are
    their own and never pass to the house or a child... (outsiders' children are
    family only through a family parent, so the max rule already covers it).
  - Renown over time: at each turning, after the tales (§18 step 8), the house
    loses `sum(marks of living Family) / 4` with a turning line.
- **Outsiders** — `outsiders.rs`: `MARRY_IN_RENOWN = 6`; **the one function**
  `marry_in(heroes, a, b) -> MarryIn { None, Refused{outsider, renown}, Accepted{
  outsider, transfer} }` (transfer = renown / 2). `plans::courtship` returns a
  new verdict `Unproven` between WedAlready and WillWed when `marry_in` refuses;
  the garden note shows `Refused`/`Accepted` facts; the wedding (`winter.rs`)
  makes the outsider Family and adds `transfer` to the house. Two outsiders
  cannot wed (`Unproven` with the first outsider's facts).
  Outsiders confer no family renown: a quest's house reward (§7.3) is paid only
  if a Family member is in the party; a carrier bonus only for Family carriers;
  the first teller's house +1 only for Family; the CARRY_THE_HOUSE death loss only
  for Family. Outsiders are never on a heir list. An outsider's death leaves its
  heirloom and dream to the family's heirs as mainline; its personal renown and
  marks die with it. A crowned outsider still adds a patron.
- **Inheritance** — `inheritance.rs`: **the one function** `bequeathed(heroes,
  dead, heir) -> Inherited { traits: Vec<Trait>, marks: i32 }` — the heir's
  traits (genes do not change on succession) and `heir.marks + dead.marks / 2`.
  `heirs::heir_buttons` appends its reading; `heirs::choose` applies its marks.
- **Sheet** — `sheet.rs`: three lines (blood, traits, marks).
- **Verify** — `inherit_checks.rs`: the three decision-row checks plus rule
  checks; `capture.rs`: three new pictures.
- Assets: none — every new surface is text on existing panels.

## Gates to add

- **Row 1, the black mark** — input: verify session on `SEEDS[0]`, Maren seated
  alone on "The bell under the tide" (her AVENGE_THE_LOST calls it, must
  succeed) · asserts: the page transcript before Set out carries the mark line
  naming Maren, +1/+2, house -1/-2; then the quest is rolled with dice that give a
  setback and Maren carries exactly 1 mark, house renown dropped by exactly the
  card's figure on top of mainline's, the quest page carries the mark line ·
  covers Done-when: verify pass with a check per decision row.
- **Row 2, marrying in** — input: winter staged with Odo turned Outsider (renown
  set to 5, then 6) in the garden with a family member of age · asserts: the page
  shows "5 of 6" and after the winter they are not wed and Odo is Outsider; at 6
  the note shows "marries in: house +3", after the winter they are spouses, Odo is
  Family and house renown rose by 3 · covers Done-when: as above.
- **Row 3, the heir** — input: mainline's staged Garrick's winter death page on
  `SEEDS[0]`, Garrick given 3 marks and the heir given genes · asserts: each
  button's shown traits/marks equal `bequeathed`, and after choosing, the heir's
  traits and marks equal what their button showed · covers Done-when: as above.
- **Rules** — child genes come one from each parent on a fixed sub-seed; a
  wanderer is Outsider; outsiders are off the heir list; an all-outsider party
  earns no house renown; the turning toll; Bold and Strong in the power sum.
- **Pictures** — the quest card with a mark line, the garden refusing an outsider,
  the death page with inheritance labels — opened and named.
- **Mutation round** — `games/keifu-x-inheritance-r3v1/mutants/inherit.txt`,
  every new constant and rule line, run with `tools/mutate`.
- **Fork hygiene** — `tools/verify keifu` pass; `git diff --stat origin/main...`
  names only the fork, Cargo.lock's entry and the run folder · covers those
  Done-when lines. `tools/build-web` + `tools/serve-web --check` on the fork.

## Non-goals

- No new art, no new screen: every surface is a line on an existing panel.
- No personal quest separate from dreams (no new quest generator): a dream call
  is the personal quest. Mainline's board generation is untouched.
- Marks do not affect the Ending's verdict or epitaphs (no epitaph part for
  marks); traits do not affect lessons, old age or fear rolls.
- No content-JSON edits for new words: new strings live in `inherit_words.rs`.
- Mainline's SPEC/SPEC-GAPS/FINDINGS are copied, not rewritten; the variant's
  rules are this file plus the code's comments.

## Decisions already made

- Personal quest = a dream call needing success (reuses §9.6's one function, so
  the card and the roll cannot disagree about who is at stake).
- Gene rolls on a per-hero sub-generator: mainline's whole roll sequence stays
  intact, so the copied oracles keep their meaning.
- Founding adults carry no traits, so W1–W10's founding power numbers stand.
- Threshold 6, transfer half, mark weights 1/2, toll divisor 4, decay halving.

## Open calls delegated to the implementer

(V1: the implementer is this tick; each decided here or recorded in the PR.)

- Exact wording and placement of the card's mark line — must fit the card's
  floors; if it does not, it may go on the card's warning row when no fear
  warning is shown and always on the sheet (decided at build).
- Which mainline oracles the variant rules break — each rewritten and named in
  the PR's Deviations.
