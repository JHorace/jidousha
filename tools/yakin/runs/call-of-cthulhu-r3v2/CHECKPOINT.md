# CHECKPOINT — call-of-cthulhu-r3v2

task: call-of-cthulhu-r3v2
variant: V2
stage: implement
tick: released
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:21 PDT

## Done so far
- claimed the branch (designer tick, mode start)
- make-game §D check on the spec: the base's decision-surface table (two rows, six columns) is present and incorporated by the r3v2 spec; not malformed
- DESIGN.md written whole from tools/yakin/templates/DESIGN.md: beings, call structure, day loop, sanity arithmetic, systems in build order, eleven gates plus a twenty-fault mutation list, non-goals, decisions, open calls
- design released: stage implement, tick released (this commit)

## Exact next step
A worker tick (WORKER.md §3, V2) builds games/call-of-cthulhu-r3v2/ from
tools/yakin/runs/call-of-cthulhu-r3v2/DESIGN.md, starting with Cargo.toml and
src/beings.rs (the content is in DESIGN.md §Systems "The beings", copy it
verbatim), then src/rules.rs (answer_outcome, tonight), src/flow.rs (step),
src/screen.rs, src/main.rs, then verify/players/checks/capture and mutants/r1.txt.

## Deviations
- none
