# CHECKPOINT — stack-card-game-r2

task: stack-card-game-r2
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 20:51 PDT

## Done so far
- claimed
- design written: games/stack-card-game-r2/DESIGN.md + run DESIGN.md

- game built: rules, NPC, screen, --verify (three players, three decision rows, result screens, capture) passing; fast gate clean
- mutation round r1: 24 of 25 noticed (L3 equivalent, noted in the list); FINDINGS.md G-070..G-072

## Exact next step
Full gate (doctor, tools/test in background, check-claude-md, yakin check), then `python3 tools/build-web stack_card_game_r2 && python3 tools/serve-web stack_card_game_r2 --check`, then open the PR (WORKER.md §4).

## Deviations
- none
