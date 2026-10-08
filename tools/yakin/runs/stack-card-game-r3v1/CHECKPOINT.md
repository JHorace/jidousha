# CHECKPOINT — stack-card-game-r3v1

task: stack-card-game-r3v1
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: 6fb32f6
updated: 2026-10-07 21:47 PDT

## Done so far
- claimed
- design inline: games/stack-card-game-r3v1/DESIGN.md + run DESIGN.md
- game crate: rules, cards, rival, screen, main (a973c6c)
- --verify: three players, four decision-row checks, floors, capture; rules tests (d13f1b9)
- mutation list mutants/round1.txt (632d386); FINDINGS.md G-070..G-074 (cb361fb)
- mutation round: 23/27 first pass, escapes closed, rerun 28 of 28 noticed (eded35a); web build + serve-web --check PASS
- full gate green at 6fb32f6: doctor ENV_OK, tools/test pass 1495/0/0, check-claude-md ok, yakin check ok, fast gate clean

## Exact next step
Open the PR (WORKER.md §4) with body target/yakin/pr-body.md's shape; then stage: done, tick: released.

## Deviations
- none
