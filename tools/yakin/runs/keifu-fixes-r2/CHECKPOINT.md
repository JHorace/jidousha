# CHECKPOINT — keifu-fixes-r2

task: keifu-fixes-r2
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 22:36 PDT

## Done so far
- claimed; DESIGN.md written (inline V1)
- implementation: outlook.rs, card/sheet telegraph, NEXT SUMMER dock, text rewrites, r2_checks.rs (3de31be)
- tools/verify keifu green with r2 checks (2,637,508 checks; main baseline 2,375,505, every summary line kept)
- mutation r2.txt: 18/19 on 8df7278, R16 escape fixed by a unit assertion (noticed on 22454f1)
- mutation: r2 19/19, filtered round 175/176 (K8 known equivalent); screens recaptured (495653b)
- w6 U1/R9/R10 re-cut at resolve::risen_trouble/eased_trouble (22454f1)

## Exact next step
Full gate on 495653b running (tools/test → target/yakin/test.log); then tools/check-claude-md, yakin check, build-web + serve-web --check keifu, then open the PR (WORKER.md §4).

## Deviations
- The port has no tutorial screen (lineage's guide never ported): rewrote the shown help/dice/stakes strings instead (G-070).
- Foresight is deliberately partial (trouble per place, the ghost, demand step); next board not drawn early.
- Telegraph shows the need as "up to N" (easing only lowers), not a two-sided range.
