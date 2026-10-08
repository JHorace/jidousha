# CHECKPOINT — call-of-cthulhu-r2

task: call-of-cthulhu-r2
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 20:41 PDT

## Done so far
- claimed
- design: games/call-of-cthulhu-r2/DESIGN.md + run DESIGN.md
- game crate playable; --verify passes (players, decision rows, floors, staged screens, capture)

## Exact next step
Look at target/verify/call_of_cthulhu_r2.png; write games/call-of-cthulhu-r2/mutants/r2.txt and run `python3 tools/mutate call_of_cthulhu_r2 mutants/r2.txt`; then FINDINGS.md, web build, full gate.

## Deviations
- none
