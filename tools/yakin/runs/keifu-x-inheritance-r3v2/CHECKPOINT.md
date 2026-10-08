# CHECKPOINT — keifu-x-inheritance-r3v2

task: keifu-x-inheritance-r3v2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:50 PDT

## Done so far
- claimed the branch (b5d64bf)
- read mainline Keifu (spec, constants, gaps, findings, the modules the base spec names) and docs/api; drafted DESIGN.md whole, every section filled (70fdc80)
- reviewed DESIGN.md against the mainline source and released it: stage implement (this commit)

## Exact next step
Worker tick (WORKER.md §3, V2): read tools/yakin/runs/keifu-x-inheritance-r3v2/DESIGN.md whole with
the spec, then DESIGN.md S0 — the fork commit: `cp -r games/keifu games/keifu-x-inheritance-r3v2`,
the eight renames listed there, `cargo check --workspace`, the fast gate,
`python3 tools/verify keifu_x_inheritance_r3v2` and `python3 tools/verify keifu` both pass, commit
`yakin(keifu-x-inheritance-r3v2): fork keifu` with nothing else in it, push. Then S1 onward.

## Deviations
- none
