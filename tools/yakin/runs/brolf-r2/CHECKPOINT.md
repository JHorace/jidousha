# CHECKPOINT — brolf-r2

task: brolf-r2
variant: V2
stage: done
tick: released
resumes: 0
gates-green-at: 5bd8da0
updated: 2026-10-07 20:55 PDT

## Done so far
- claimed the branch (design stage)
- read brolf.md + brolf-r2.md, ran make-game §D on the spec's table (well-formed), read all five docs/api/ files and the prototype_kit, slalom and ui_kit examples
- DESIGN.md written and released: tools/yakin/runs/brolf-r2/DESIGN.md (every section filled; no engine change needed, no FINDINGS owed at design time)
- implement: games/brolf-r2/ crate built whole; `tools/verify brolf_r2` passes (all gates green), fmt + clippy + wasm check clean

- mutation round r1: 19 of 19 noticed (3 escapes closed with gates); FINDINGS G-070..G-075; build-web + serve-web --check pass

- full gate green at 5bd8da0; PR opened: https://github.com/JHorace/jidousha/pull/133

## Exact next step
None: the task is done; the PR awaits review.

## Deviations
- end_match: Survived needs every rival Eliminated (design crowned a survivor whose rivals had extracted; Idle then won by doing nothing)
- run A stage after 300 also clears N1's held (it had looted a Heavy, halving the sledge the gate asserts at 13.5)
- Helmet/Heavy lines shortened to fit 27 columns; shrink row reads `shrinking, 4.0s left`
