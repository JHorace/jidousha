# CHECKPOINT — restaurant-nemesis-r3v2

task: restaurant-nemesis-r3v2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: 42ebee3
updated: 2026-10-07 22:15 PDT

## Done so far
- branch claimed; run folder created (f3402c3)
- DESIGN.md written whole, FINDINGS.md with two doc-gap entries (598e6c4)
- DESIGN.md consistency pass (G5/G6 wording, viability order, log-line formats, scenario player) and design released: stage implement
- worker tick (continue): game crate per DESIGN §Systems (db834cd); --verify G1-G10 + G8 tests (7dc5d9a)
- FINDINGS G-075..G-079 in games/restaurant-nemesis-r3v2/FINDINGS.md; mutation 18/21 then 21 of 21 noticed; web build + serve-web --check PASS
- full gate green at 42ebee3: doctor ENV_OK, tools/test pass 1495/0/0, check-claude-md ok, yakin check ok, fast gate clean

## Exact next step
Full gate (WORKER.md §2) and fast gate; then open the PR (WORKER.md §4) listing the deviations below.

## Deviations
- SEED_SPAWN precondition relaxed: no seed in 0..1024 keeps the first nemesis alone through day 5; now "one nemesis after day 1, $50 on night 1, first in the queue on days 3 and 5"; G6 tracks it by id (G-077)
- ledger nemesis row split into three rows, follower's +5 fol moved to line 1: the design's strings overflow 960 at 12px (G-079)
- capture 960x540, not 480x270 (G-078)
- mutate invoked with the package name restaurant_nemesis_r3v2 (the design's folder name is refused by tools/mutate)
- overwhelm reputation read as +2 served +5 overwhelmed (the design lists both without saying they stack)
