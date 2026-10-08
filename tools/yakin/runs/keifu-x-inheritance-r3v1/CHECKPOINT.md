# CHECKPOINT — keifu-x-inheritance-r3v1

task: keifu-x-inheritance-r3v1
variant: V1
stage: done
tick: released
resumes: 0
gates-green-at: fb74e5d
updated: 2026-10-07 22:31 PDT

## Done so far
- claimed
- inline DESIGN.md written (V1) (db95fcf)
- fork: pure copy-and-rename of games/keifu, verify pass (277a44d)
- variant rules: genes.rs, marks.rs, outsiders.rs, inheritance.rs wired into births, wanderers, power, resolve, reward, courtship, wedding, tellers, heirs, turning toll, card/sheet/heir-button surfaces (4dd4007)
- inherit_checks.rs: the three decision rows; mainline oracles rewritten (W3 sheet, stir staging, heir_labels first line); verify pass
- rule tests (outsiders.rs), three captures, mutants/inherit.txt (34 faults), FINDINGS G-070

- mutation round: 34 of 34 noticed over three passes (escapes fixed with tests); verify keifu + fork pass; build-web/serve-web --check pass; fast gate clean
- full gate green at fb74e5d: doctor ENV_OK, tools/test pass (1865/0/0), check-claude-md ok, yakin check ok
- PR opened: https://github.com/JHorace/jidousha/pull/144

## Exact next step
None — done; the PR waits on the owner.

## Deviations
- see the PR body Deviations (sheet blood line only when non-empty; personal-quest word on the call line; two-line heir buttons; card fail % from the card's setback figure; mainline oracles rewritten)
