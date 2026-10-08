# CHECKPOINT — brolf-r2

task: brolf-r2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 20:22 PDT

## Done so far
- claimed the branch (design stage)
- read brolf.md + brolf-r2.md, ran make-game §D on the spec's table (well-formed), read all five docs/api/ files and the prototype_kit, slalom and ui_kit examples
- DESIGN.md written and released: tools/yakin/runs/brolf-r2/DESIGN.md (every section filled; no engine change needed, no FINDINGS owed at design time)

## Exact next step
Implement stage (WORKER.md): read tools/yakin/runs/brolf-r2/DESIGN.md whole, then create games/brolf-r2/ (Cargo.toml with package name brolf_r2, src/main.rs with #![allow(missing_docs)]) in the build order DESIGN.md §Systems gives: main.rs, rules.rs, sim.rs, npc.rs, screen.rs, checks.rs, players.rs, verify.rs, capture.rs, mutants/r1.txt, FINDINGS.md. Comparison hygiene: never read claude/yakin-brolf, PR #128 or games/brolf/.

## Deviations
- none
