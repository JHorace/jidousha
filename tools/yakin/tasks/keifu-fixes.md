# keifu-fixes — three conservative usability fixes on mainline Keifu

Kind: game · Variant: V1 · Size: M · Window: burn-down (the handoff said
"night"; `burn-down` is the schema's value for the Wednesday window)

## Goal

Make mainline Keifu easier to learn and to plan in, without redesigning it.
Mainline `games/keifu` is a faithful port of lineage (`games/keifu/spec/`);
these are usability fixes, so every delta from the original stays small,
deliberate and listed. Three improvements, in no particular order, each
independently landable:

1. **Tutorial language.** Rewrite the tutorial and help text for clarity,
   above all the dice-roll explanations — the current text reads like an
   unnatural Claude-ism. Voice: a human teaching a friend. Mechanics unchanged:
   every number, odd and rule the text states stays exactly as the sim computes
   it.
2. **Dungeon foresight.** Let the player see next round's incoming dungeons —
   in Keifu's terms, next summer's board of quests and places (SPEC §5.2) — so
   they can plan this summer against it. The smallest UI surface that achieves
   it. Partial foresight (for example: which places are coming, or their
   trouble, but not every roll) is acceptable if it is designed deliberately
   and the design note says what is shown and why.
3. **Danger telegraphing.** Make rot/escalation consequences legible before
   the player commits. In Keifu's terms: what leaving a quest unanswered does
   to its place's trouble and so to that place's next quest (SPEC §7.2 cost and
   trouble rise; §5.2's seats, danger, renown and demand formulas). Show it as
   a telegraphed range — "its next level could be X–Y" — not a hidden outcome.
   It need not be fully deterministic; it must be visible at the moment of
   seating.

If one of the three balloons, ship the others and name the remainder in the
PR body (Deviations) as not done, with what was found.

## Owner ruling carried by this spec

Confirmed by the owner 2026-10-06: there is no open Keifu PR, so this task may
edit mainline `games/keifu` directly. It is Keifu's own task, so DOCTRINE §9's
"a game is touched only by its own task" is satisfied. `keifu-x-inheritance`
(a separate task) forks Keifu into its own folder and never writes here.

## Fence

- **Grants write access to `games/keifu/**`** (source, its `spec/content/`
  text where the tutorial lives, `screens/`, `mutants/`, its ledgers) and the
  run folder `tools/yakin/runs/keifu-fixes/`.
- Grants nothing else: no other game, no engine crate, no workspace manifest,
  no `Cargo.lock` change (this task adds no crate). A diff outside those paths
  is a defect.
- No new dependencies.
- **Determinism and the existing record stay intact.** Keifu's existing
  `--verify` battery and its oracles are the port's evidence of faithfulness.
  A fix that would change an existing oracle's outcome (for example by drawing
  next summer's board early and so moving every later draw on a shared RNG
  stream) is the wrong fix: derive foresight from state that already exists,
  or from a separately seeded stream, and keep the old run identical. Text
  assertions that quote rewritten tutorial lines are updated to the new
  wording — that is the one kind of existing assertion this task may change,
  and each such change is named in the PR.

## Steps

1. Design inline (WORKER.md §3, V1): `tools/yakin/runs/keifu-fixes/DESIGN.md`
   names, for each of the three fixes, what changes on screen, the files
   touched, and how it is checked. Read Keifu's own docs first —
   `games/keifu/spec/SPEC.md` (§5 summer, §6 power and odds, §7 resolution),
   `spec/content/ui-text.json`, `SPEC-GAPS.md` and `FINDINGS.md` — then the
   source it touches.
2. Tutorial: rewrite the text; keep every stated number true to the sim (a
   dice explanation that quotes odds reads them from the same function the
   resolution uses — CONSTANTS §4's k/36 table is the ground truth).
3. Foresight: add the smallest surface that shows next summer's incoming
   board (or the deliberately partial version DESIGN.md chose).
4. Telegraph: add the range shown at seating time for each quest — what its
   place's next quest could be if it is left unanswered.
5. Gates: existing `tools/verify keifu` stays green; extend the transcript
   gates (the `--verify` assertions that read the frame transcript, make-game
   §A.4) to cover the foresight and telegraph surfaces where feasible; run the
   mutation round on the new checks (make-game §A.6, `tools/mutate`); recapture
   any screen whose picture changed (make-game §A.7) and add one picture each
   of the foresight and telegraph surfaces.

## Decisions this task changes

| decision | must know | surface | action | one function | asserted by |
|---|---|---|---|---|---|
| Which quests to answer this summer, given what is coming next summer | next summer's incoming places/quests (or the deliberately partial set DESIGN.md chose) | the summer board, readable while seating — not a separate screen | dragging heroes into quest seats, as today | the board-generation path that will produce next summer's board (or the trouble state it reads) — the preview and the real generation read the same function, so the preview cannot promise a board the sim then does not deal | a scripted `--verify` check that reaches a seating summer and asserts the foresight facts are in the frame transcript, and a later check that the board dealt matches what was previewed (to the precision the design promised) |
| Whether to leave a quest unanswered and let its place's trouble rise | the place's current trouble; the range its next quest's danger, seats, renown and demand fall in if trouble rises; the renown cost of leaving it (SPEC §7.2) | on or beside that quest's card during seating | leaving its seats empty and ending the summer, as today | the §5.2 quest-generation formula, evaluated over the template range for trouble + 1, and the §7.2 unanswered cost — the telegraph and the resolution call the same functions | a scripted `--verify` check that asserts the telegraphed range is in the transcript at seating, then leaves the quest unanswered and asserts the next summer's quest at that place lies inside the range shown |

## Done when

- The three fixes are on the branch, or the PR names which did not land and
  why.
- `python3 tools/verify keifu` reports `pass` in `target/verify/keifu.json`;
  every pre-existing check is still present and passing (only rewritten
  tutorial-text quotes changed, each listed in the PR).
- The two decision rows above each have the check named in "asserted by", or
  the PR says why that one was infeasible and what checks it instead.
- `git diff --stat origin/main...` names only paths under `games/keifu/` and
  `tools/yakin/runs/keifu-fixes/`.
- `python3 tools/build-web keifu && python3 tools/serve-web keifu --check`
  passes.
- A PR titled `[yakin:V1] <this task's queue title>` is open with the WORKER.md
  body; its Deviations list every behavioral delta from lineage this task
  introduced, one line each.
