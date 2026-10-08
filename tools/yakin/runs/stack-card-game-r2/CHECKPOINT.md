# CHECKPOINT — stack-card-game-r2

task: stack-card-game-r2
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 20:40 PDT

## Done so far
- claimed
- design written: games/stack-card-game-r2/DESIGN.md + run DESIGN.md

- game built: rules, NPC, screen, --verify (three players, three decision rows, result screens, capture) passing; fast gate clean

## Exact next step
Mutation round: write games/stack-card-game-r2/mutants/r1.txt and run `python3 tools/mutate stack_card_game_r2 mutants/r1.txt` (check its usage first); then FINDINGS.md, full gate, build-web/serve-web --check, PR.

## Deviations
- none
