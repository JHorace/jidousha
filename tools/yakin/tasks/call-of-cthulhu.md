# call-of-cthulhu — eldritch beings phone you; hang up before your sanity goes

Kind: game · Variant: V1 · Size: M · Window: burn-down (the handoff said
"night"; `burn-down` is the schema's value for the Wednesday window)

## Goal

A new text-driven game, `games/call-of-cthulhu`. The player receives phone
calls from eldritch beings. Staying on the line drains sanity, so calls must
be ended as fast as possible — a dating sim inverted: learning each being's
lore is what lets you answer their questions in the ways that end the
conversation quickly. Actively angering a being is worse than the drain.
Prototype scope: a run of several days with 2–3 distinct beings, win or lose
on sanity.

## Fence

- **Grants write access to `games/call-of-cthulhu/**`** (new) and `Cargo.lock`
  for that crate's own entry only, plus the run folder
  `tools/yakin/runs/call-of-cthulhu/`.
- Grants nothing else. No new dependencies: the manifest names `jidousha` by
  path and nothing else. The beings and their lore are written for this game
  (Lovecraft's public-domain mythos may be drawn on; no text from any
  commercial game).

## The brief

- **V1: design inline, then implement.** The tick writes a compact design
  note in the game folder (`games/call-of-cthulhu/DESIGN.md`) — the beings,
  the call structure, the day loop, the sanity arithmetic — before the code.
  The run folder's DESIGN.md (WORKER.md §3) may be short and point at it.
- **Evening: the calls.** A being calls; each exchange on the line costs
  sanity. The being asks questions; answers drawn from its lore end the call
  sooner; wrong answers prolong it; some answers anger it, and anger costs
  more than the drain. The design says how much more, and what anger does.
- **Morning: preparation.** A morning phase to boost skills, study lore, and
  interact with cults. Daytime choices **visibly** shape the night's calls —
  which being calls, what it asks, which answers you have — and the player
  can see that link, not just infer it.
- **The run.** Several days, 2–3 distinct beings with their own lore and
  temper. Lose when sanity runs out; win by surviving the run (the note may
  add a better ending).
- **House pattern.** Deterministic with seeded calls (CLAUDE.md convention 4:
  the same seed and inputs replay the same run), a `--verify` mode with a
  scripted player and transcript gates (make-game §A.4–A.5), the mutation
  round (§A.6), a captured picture (§A.7).

## Decisions this task adds

| decision | must know | surface | action | one function | asserted by |
|---|---|---|---|---|---|
| Which answer to give the being on the line | the question; the answer options; which lore I have learned about this being; my current sanity and the per-exchange drain; the being's current temper | the call screen — question, options, sanity, temper and my known lore for this being all on screen while choosing | choosing an answer (the note names the input) | one answer-outcome function (call length change, anger change, sanity cost) that both any on-screen hint and the call's resolution read | a scripted `--verify` check that asserts the question, options, sanity and known lore are in the transcript before answering, then answers on a fixed seed and asserts the sanity and temper change equal what that function returns |
| How to spend the morning: train a skill, study a being's lore, or deal with a cult | what each option gives; which being it bears on; how it changes tonight's calls | the morning screen, with each option's effect on tonight stated on the option | choosing a morning action (the note names the input) | one function that derives tonight's call schedule and question pool from the day's state — the morning preview and the evening both read it | a `--verify` check where two seeded runs differ only in one morning choice, asserting the stated effect is in the transcript at choosing and the night's calls differ exactly as stated |

## Done when

- `games/call-of-cthulhu/DESIGN.md` exists and names the beings, the call
  structure, the day loop and the sanity arithmetic.
- A run of several days with 2–3 distinct beings plays end to end, and
  scripted `--verify` runs reach both a win and a sanity loss.
- `python3 tools/verify call-of-cthulhu` reports `pass` in
  `target/verify/call-of-cthulhu.json`, with a check for each decision row
  above.
- `git diff --stat origin/main...` names only `games/call-of-cthulhu/`,
  `Cargo.lock` (that crate's entry only) and
  `tools/yakin/runs/call-of-cthulhu/`.
- `python3 tools/build-web call-of-cthulhu && python3 tools/serve-web call-of-cthulhu --check`
  passes.
- A PR titled `[yakin:V1] <this task's queue title>` is open with the WORKER.md
  body.
