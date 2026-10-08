# CHECKPOINT — restaurant-nemesis-r3v2

task: restaurant-nemesis-r3v2
variant: V2
stage: implement
tick: released
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:24 PDT

## Done so far
- branch claimed; run folder created (f3402c3)
- DESIGN.md written whole, FINDINGS.md with two doc-gap entries (598e6c4)
- DESIGN.md consistency pass (G5/G6 wording, viability order, log-line formats, scenario player) and design released: stage implement

## Exact next step
A worker tick (WORKER.md §3, V2): read tools/yakin/runs/restaurant-nemesis-r3v2/DESIGN.md whole, then build games/restaurant-nemesis-r3v2/ in its §Systems order, starting with the crate scaffold (Cargo.toml, src/main.rs) and src/sim.rs + src/rules.rs (the three decision functions), cargo check after every edit.

## Deviations
- none
