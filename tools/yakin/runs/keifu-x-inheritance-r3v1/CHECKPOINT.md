# CHECKPOINT — keifu-x-inheritance-r3v1

task: keifu-x-inheritance-r3v1
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:48 PDT

## Done so far
- claimed
- inline DESIGN.md written (V1) (db95fcf)
- fork: pure copy-and-rename of games/keifu, verify pass (277a44d)
- variant rules: genes.rs, marks.rs, outsiders.rs, inheritance.rs wired into births, wanderers, power, resolve, reward, courtship, wedding, tellers, heirs, turning toll, card/sheet/heir-button surfaces (4dd4007)
- inherit_checks.rs: the three decision rows; mainline oracles rewritten (W3 sheet, stir staging, heir_labels first line); verify pass
- rule tests (outsiders.rs), three captures, mutants/inherit.txt (34 faults), FINDINGS G-070

## Exact next step
Mutation round running (target/yakin/mutate.log, not committed); on escapes add checks; then tools/verify keifu, build-web/serve-web --check, full gate (tools/test in background), PR.

## Deviations
- none
