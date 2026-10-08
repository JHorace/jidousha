# CHECKPOINT — call-of-cthulhu-r2

task: call-of-cthulhu-r2
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 20:44 PDT

## Done so far
- claimed
- design: games/call-of-cthulhu-r2/DESIGN.md + run DESIGN.md
- game crate playable; --verify passes (players, decision rows, floors, staged screens, capture) at 389188d
- mutants/r2.txt (27 faults), FINDINGS.md G-070..G-074

## Exact next step
Run `python3 tools/mutate call_of_cthulhu_r2 mutants/r2.txt --jobs 2`, close escapes with checks; then build-web + serve-web --check, full gate, PR.

## Deviations
- none
