# CHECKPOINT — keifu-fixes

task: keifu-fixes
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: 96cc81d
updated: 2026-10-07 04:59 PDT

## Done so far
- claimed
- outlook.rs (shared ranges), telegraph on card+sheet, foresight in idle dock, tutorial text rewrite, verify checks in outlook_checks.rs; tools/verify keifu passes
- mutants/outlook.txt 20 of 20; pictures re-taken; FINDINGS written; full gate green (1501 passed) at 96cc81d; web check passes

## Exact next step
Wait for the filtered mutation round (target/yakin/mutate2.log; if the sandbox is new, rerun: python3 tools/mutate keifu mutants/*.txt --fast --changed-since e2e1c21), put its numbers in games/keifu/FINDINGS.md where it says MUTATION2, then open the PR (WORKER.md §4) and set stage done.

## Deviations
- none
