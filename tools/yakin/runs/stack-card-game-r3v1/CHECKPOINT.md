# CHECKPOINT — stack-card-game-r3v1

task: stack-card-game-r3v1
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:27 PDT

## Done so far
- claimed
- design inline: games/stack-card-game-r3v1/DESIGN.md + run DESIGN.md
- game crate: rules, cards, rival, screen, main (a973c6c)
- --verify: three players, four decision-row checks, floors, capture; rules tests (d13f1b9)
- mutation list mutants/round1.txt (632d386); FINDINGS.md G-070..G-074

## Exact next step
Run `python3 tools/mutate stack_card_game_r3v1 games/stack-card-game-r3v1/mutants/round1.txt`, fix escapes; then build-web + serve-web --check; then the full gate (WORKER.md §2) and the PR.

## Deviations
- none
