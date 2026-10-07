# CHECKPOINT — keifu-fixes

task: keifu-fixes
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 02:06 PDT

## Done so far
- claimed
- outlook.rs (shared ranges), telegraph on card+sheet, foresight in idle dock, tutorial text rewrite, verify checks in outlook_checks.rs; tools/verify keifu passes

## Exact next step
Run tools/mutate keifu on a new mutants/outlook.txt (see mutants/*.txt shape), add capture pictures of foresight + telegraph (src/capture.rs, screens/), write FINDINGS, then full gate + build-web/serve-web check, then PR.

## Deviations
- none
