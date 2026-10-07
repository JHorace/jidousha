# CHECKPOINT — stack-card-game

task: stack-card-game
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 03:34 PDT

## Done so far
- claimed
- design note written (games/stack-card-game/DESIGN.md)
- rules, cards, npc, players, game, ui, main compile (skilled 71% vs NPC, power 0%, blind 0% over 200 seeds)
- verify mode: rule checks, 3 decision scenarios, layout x3 aspects, full match twice, 3 players; tools/verify passes; capture PNG written

## Exact next step
Write mutants/*.txt and run tools/mutate; tools/build-web + serve-web --check; full gate; open PR.

## Deviations
- none
