# CHECKPOINT — stack-card-game-r3v2

task: stack-card-game-r3v2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:32 PDT

## Done so far
- claimed (32f5e58)
- DESIGN.md drafted whole: rules, pool, why manipulation dominates, systems, 13 gates, decisions, open calls (eafe24f)
- FINDINGS.md entry 1: `Flat` undefined in jidousha-ui.md's judge_frame example (eafe24f)
- DESIGN.md re-read and fixed: G4 restaged so passes = 1 is reachable, G5 pass count, resolution-order cell, cell widths vs string lengths, blurbs <= 16 chars, says lines <= 20 chars; design released (90882b5)
- implement: games/stack-card-game-r3v2/ built whole per DESIGN.md Systems; --verify passes all gates (G1-G11, G13), sequencer 7/12, brute 0/12, nothing 0/12; fast gate clean (this commit)

## Exact next step
Mutation round (G12): write games/stack-card-game-r3v2/mutants/round1.txt with DESIGN.md's ten faults and run `python3 tools/mutate stack-card-game-r3v2 mutants/round1.txt`; then games/stack-card-game-r3v2/FINDINGS.md, full gate, build-web/serve-web --check, PR.

## Deviations
- hand panel title shortened to `HAND - 1-6 plays`: DESIGN.md's `HAND - 1-6 plays, Space passes` at TITLE size runs 78 units off the right edge (floors caught it).
- verify split across verify.rs, checks.rs, decisions.rs, sweep.rs to keep files under ~500 lines.
- design stage: none. The game's FINDINGS.md (make-game §C) should carry the run folder's FINDINGS.md entry 1 forward.
