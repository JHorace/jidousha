# CHECKPOINT — keifu-fixes-r2

task: keifu-fixes-r2
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:08 PDT

## Done so far
- claimed; DESIGN.md written (inline V1)
- implementation: outlook.rs, card/sheet telegraph, NEXT SUMMER dock, text rewrites, r2_checks.rs (3de31be)
- tools/verify keifu green with r2 checks (2,637,508 checks; main baseline 2,375,505, every summary line kept)
- mutation r2.txt: 18/19 on 8df7278, R16 escape fixed by a unit assertion (noticed on 22454f1)
- w6 U1/R9/R10 re-cut at resolve::risen_trouble/eased_trouble (22454f1)

## Exact next step
Wait for / rerun `python3 tools/mutate keifu "mutants/*.txt" --fast --changed-since e2e1c21 --jobs 2` (log target/yakin/mutate2.log); then `python3 tools/verify keifu`, copy changed PNGs (dock-idle, dock-scrolled, w0-w1-garrick, w1-wren-child, w10-door-card, w10-door-empty, w10-reset, + new r2-outlook, r2-telegraph) from target/verify/keifu-*.png into games/keifu/screens; record the mutation score in FINDINGS; full gate; build-web + serve-web --check; PR.

## Deviations
- The port has no tutorial screen (lineage's guide never ported): rewrote the shown help/dice/stakes strings instead (G-070).
- Foresight is deliberately partial (trouble per place, the ghost, demand step); next board not drawn early.
- Telegraph shows the need as "up to N" (easing only lowers), not a two-sided range.
