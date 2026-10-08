# CHECKPOINT — call-of-cthulhu-r2

task: call-of-cthulhu-r2
variant: V1
stage: done
tick: released
resumes: 0
gates-green-at: 1219f19
updated: 2026-10-07 20:59 PDT

## Done so far
- claimed
- design: games/call-of-cthulhu-r2/DESIGN.md + run DESIGN.md
- game crate playable; --verify passes (players, decision rows, floors, staged screens, capture) at 389188d
- mutants/r2.txt (27 faults), FINDINGS.md G-070..G-074
- mutation round: 19/27 first pass; rule_checks.rs closes the 8 escapes; 27/27
- tools/verify pass; build-web + serve-web --check pass
- full gate green at 1219f19: doctor ENV_OK, tools/test pass 1487/0/0, fast gate clean
- PR #134 opened

## Exact next step
None — done; PR https://github.com/JHorace/jidousha/pull/134 awaits review.

## Deviations
- run DESIGN.md's file list grew: play.rs, conductor.rs, decisions.rs, screen_checks.rs, rule_checks.rs (verify split under the ~500-line rule)
- tuning moved from the first design note: wrong answers +1 (not -1), composure max 1 (not 2), cults unheld at temper 2 (G-073, G-074)
