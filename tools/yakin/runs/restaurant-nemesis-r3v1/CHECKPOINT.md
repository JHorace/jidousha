# CHECKPOINT — restaurant-nemesis-r3v1

task: restaurant-nemesis-r3v1
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 22:07 PDT

## Done so far
- claimed
- inline DESIGN.md written (V1), every number fixed
- games/restaurant-nemesis-r3v1 plays end to end; tools/verify passes (188 checks): good wins, first-timer and idle lose, 3 nemeses at once; fast gate clean

## Exact next step
Write mutants/r3v1.txt and run `python3 tools/mutate restaurant_nemesis_r3v1 mutants/r3v1.txt`; FINDINGS.md; build-web/serve-web --check; full gate; PR.

## Deviations
- 5 customers a round plus a 3-customer lunch rush in one seeded round (23 a day), not 4 a round (16): with 4 a round, standard-for-everyone never failed anyone
- kitchen capacity 4 a round, not 5
- an unserved (skipped) order's failure is 1 more severe (SKIP_INSULT); the spawn threshold is severity 4, not 3 — so only a skipped level-3 demand spawns, as the design intended
- a nemesis spawns with 10 + 5 x severity followers (30 at severity 4), not 15 + 10 x severity
- refunds are $1 per severity, not $2; a satisfied nemesis pays $10, not $12
- a satisfied nemesis loses 5 followers (a grudging good review); not in DESIGN.md
- tomorrow's followers are stated "of 23", the day's customer count
- order rows are drawn at the 12-unit floor and the queue pages at 7 rows (a rush round with visits is up to 11)
