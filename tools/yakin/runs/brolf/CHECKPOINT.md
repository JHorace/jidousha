# CHECKPOINT — brolf

task: brolf
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 11:22 PDT

## Done so far
- claimed (d21cb8f)
- DESIGN.md drafted whole: every section filled (fe9658f)
- DESIGN.md re-read against the spec's four rows; staged sessions freeze unnamed NPCs, NPCs gather items, the hole opens late (this commit) — design released

- game rules, systems, text and draw written; `cargo check` and 18 unit tests green; verify modules are stubs (this commit)

## Exact next step
Replace the stubs in games/brolf/src/{verify,checks,gates,gates_play,gates_end,gates_frames,players,capture}.rs with the real --verify (DESIGN.md "Gates to add": sessions A-F), then mutants/m0.txt, then build-web/serve-web check and the PR (WORKER.md §4).

## Deviations
- none
