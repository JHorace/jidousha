# CHECKPOINT — keifu-x-inheritance-r2

task: keifu-x-inheritance-r2
variant: V2
stage: implement
tick: released
resumes: 0
gates-green-at: none
updated: 2026-10-07 18:20 PDT

## Done so far
- branch claimed; design stage started (designer tick)
- mainline Keifu read: SPEC §3, §5-§7, §9-§15, §17-§18, §20; CONSTANTS; hero, house, heirs, death_page, births, newcomers, wanderer, quest_card, quest_sheet, plans, resolve, harm, witness, legacy, coming_of_age, verify, w8, play, capture; make-game §A, §C-§E; §D check passed (the spec carries its three-row table)
- DESIGN.md written and released (3ff185c draft; this commit final): family flag, courtship threshold, family renown, family-only heirs, marks, the oath, inheritance (conceive + succession), sheet, checks, mutants, pictures

## Exact next step
Implement stage (WORKER.md): first act, its own commit — `cp -r games/keifu games/keifu-x-inheritance-r2`, rename the crate to `keifu_x_inheritance_r2` (package, binary, window title, message prefix, doc lines, capture file prefix `keifu-x-inheritance-r2-`), change nothing else, and get `python3 tools/verify keifu_x_inheritance_r2` green on that commit. Then build DESIGN.md's Systems in their order, starting with `src/family.rs`.

## Deviations
- none
