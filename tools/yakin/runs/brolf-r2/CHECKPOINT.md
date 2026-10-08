# CHECKPOINT — brolf-r2

task: brolf-r2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 20:35 PDT

## Done so far
- claimed the branch (design stage)
- read brolf.md + brolf-r2.md, ran make-game §D on the spec's table (well-formed), read all five docs/api/ files and the prototype_kit, slalom and ui_kit examples
- DESIGN.md written and released: tools/yakin/runs/brolf-r2/DESIGN.md (every section filled; no engine change needed, no FINDINGS owed at design time)
- implement: games/brolf-r2/ crate drafted whole (rules, sim, npc, screen, players, verify, gates, capture); --verify runs, 6 gates red

## Exact next step
Run `cargo run -q -p brolf_r2 -- --verify` and fix the red gates: first-out tick off by one (judge reads post-stage positions), N1 holds Heavy at the sledge (clear its held in a_after_300), Idle survives because NPCs all extract (make Survived require every rival Eliminated), planner landed error counts holed shots, two-overlays floor staging. Then mutants/r1.txt, FINDINGS.md, full gate, PR.

## Deviations
- none
