# CHECKPOINT — brolf

task: brolf
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 11:31 PDT

## Done so far
- claimed (d21cb8f)
- DESIGN.md drafted whole: every section filled (fe9658f)
- DESIGN.md re-read against the spec's four rows; staged sessions freeze unnamed NPCs, NPCs gather items, the hole opens late (this commit) — design released

- game rules, systems, text and draw written; unit tests green (3e98b1b)
- --verify written (sessions A-F, three players, capture); `tools/verify brolf` verified, clippy clean (this commit)

## Exact next step
Write games/brolf/mutants/m0.txt (DESIGN.md "Mutation round") and run `python3 tools/mutate brolf mutants/m0.txt` (background), file games/brolf/FINDINGS.md, then the full gate (WORKER.md §2), `python3 tools/build-web brolf && python3 tools/serve-web brolf --check`, then the PR (WORKER.md §4).

## Deviations
- (to be filled from the list in the PR body; see git log for this branch)
