# CHECKPOINT — call-of-cthulhu-r3v1

task: call-of-cthulhu-r3v1
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:30 PDT

## Done so far
- claimed
- design inline: games/call-of-cthulhu-r3v1/DESIGN.md + run DESIGN.md
- the game, its --verify (both decision rows, three players, staged screens) and capture; tools/verify call_of_cthulhu_r3v1 passes
- mutation list games/call-of-cthulhu-r3v1/mutants/r1.txt written

## Exact next step
Run `python3 tools/mutate call-of-cthulhu-r3v1 mutants/r1.txt` (background), tighten any escape, then FINDINGS.md, full gate, build-web/serve-web --check, PR.

## Deviations
- none
