# CHECKPOINT — keifu-x-inheritance-r3v1

task: keifu-x-inheritance-r3v1
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:37 PDT

## Done so far
- claimed
- inline DESIGN.md written (V1) (db95fcf)
- fork: pure copy-and-rename of games/keifu, verify pass (277a44d)
- variant rules: genes.rs, marks.rs, outsiders.rs, inheritance.rs wired into births, wanderers, power, resolve, reward, courtship, wedding, tellers, heirs, turning toll, card/sheet/heir-button surfaces; unit tests green; verify not yet green

## Exact next step
Run python3 tools/verify keifu_x_inheritance_r3v1 and fix remaining mainline-oracle breaks; then write src/inherit_checks.rs (three decision-row checks) and call it from verify::run.

## Deviations
- none
