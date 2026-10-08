# CHECKPOINT — call-of-cthulhu-r3v1

task: call-of-cthulhu-r3v1
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:34 PDT

## Done so far
- claimed
- design inline: games/call-of-cthulhu-r3v1/DESIGN.md + run DESIGN.md
- the game, its --verify (both decision rows, three players, staged screens) and capture; tools/verify call_of_cthulhu_r3v1 passes
- mutation list games/call-of-cthulhu-r3v1/mutants/r1.txt written; FINDINGS.md (G-070..G-073)
- mutation round 1: 18/21, three escapes closed (C4, C13, C17), rerun 3/3; build-web + serve-web --check pass

## Exact next step
Full gate (WORKER.md §2): doctor, tools/test (background), check-claude-md, yakin check; then open the PR.

## Deviations
- none
