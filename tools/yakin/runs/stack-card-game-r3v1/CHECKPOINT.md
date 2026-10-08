# CHECKPOINT — stack-card-game-r3v1

task: stack-card-game-r3v1
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:32 PDT

## Done so far
- claimed
- design inline: games/stack-card-game-r3v1/DESIGN.md + run DESIGN.md
- game crate: rules, cards, rival, screen, main (a973c6c)
- --verify: three players, four decision-row checks, floors, capture; rules tests (d13f1b9)
- mutation list mutants/round1.txt (632d386); FINDINGS.md G-070..G-074 (cb361fb)
- mutation round: 23/27 first pass, escapes closed, rerun 28 of 28 noticed (eded35a); web build + serve-web --check PASS

## Exact next step
Run the full gate (WORKER.md §2: doctor, tools/test in background, check-claude-md, yakin check) and the fast gate; then open the PR (WORKER.md §4).

## Deviations
- none
