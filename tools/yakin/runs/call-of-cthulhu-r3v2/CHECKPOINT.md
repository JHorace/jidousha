# CHECKPOINT — call-of-cthulhu-r3v2

task: call-of-cthulhu-r3v2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:39 PDT

## Done so far
- claimed the branch (designer tick, mode start)
- make-game §D check on the spec: the base's decision-surface table (two rows, six columns) is present and incorporated by the r3v2 spec; not malformed
- DESIGN.md written whole from tools/yakin/templates/DESIGN.md: beings, call structure, day loop, sanity arithmetic, systems in build order, eleven gates plus a twenty-fault mutation list, non-goals, decisions, open calls
- design released: stage implement, tick released
- worker tick: game built from DESIGN.md (beings, rules, flow, screen, main, players, checks, capture, verify, verify_screens); --verify passes 763 checks on seeds 1928 and 7 (802352e)
- mutants/r1.txt: 20 of 20 noticed (verify alone 20); FINDINGS.md G-070, G-071

## Exact next step
Run the full gate (tools/doctor, tools/test in the background, check-claude-md, yakin check), then
`python3 tools/verify call_of_cthulhu_r3v2` and `python3 tools/build-web call_of_cthulhu_r3v2 && python3 tools/serve-web call_of_cthulhu_r3v2 --check`; then open the PR (WORKER.md §4).

## Deviations
- the Guesser studies in the morning instead of meditating: with meditation the design's own arithmetic gives it 68-76 sanity spent, so it survives; studying (and never using the lore) costs 96-104 and it loses on night 4 (FINDINGS G-070)
- mutation round run as `tools/mutate call_of_cthulhu_r3v2` (the binary's name); DESIGN.md wrote the folder's name, which the tool refuses (FINDINGS G-071)
