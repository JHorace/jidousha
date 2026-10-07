# CHECKPOINT — stack-card-game

task: stack-card-game
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 03:29 PDT

## Done so far
- claimed
- design note written (games/stack-card-game/DESIGN.md)
- rules, cards, npc, players, game, ui, main compile (skilled 71% vs NPC, power 0%, blind 0% over 200 seeds)

## Exact next step
Write src/verify.rs, checks.rs, capture.rs (decision rows 1-3 scenarios, rule checks, three players, determinism); then mutants/*.txt, tools/mutate, capture PNG, full gate.

## Deviations
- none
