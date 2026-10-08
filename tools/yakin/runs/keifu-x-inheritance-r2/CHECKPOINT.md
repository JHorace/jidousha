# CHECKPOINT — keifu-x-inheritance-r2

task: keifu-x-inheritance-r2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 19:13 PDT

## Done so far
- branch claimed; design stage started (designer tick)
- mainline Keifu read: SPEC §3, §5-§7, §9-§15, §17-§18, §20; CONSTANTS; hero, house, heirs, death_page, births, newcomers, wanderer, quest_card, quest_sheet, plans, resolve, harm, witness, legacy, coming_of_age, verify, w8, play, capture; make-game §A, §C-§E; §D check passed (the spec carries its three-row table)
- DESIGN.md written and released (3ff185c draft; this commit final): family flag, courtship threshold, family renown, family-only heirs, marks, the oath, inheritance (conceive + succession), sheet, checks, mutants, pictures

- implement stage: copy-and-rename commit (games/keifu -> games/keifu-x-inheritance-r2, crate keifu_x_inheritance_r2); tools/verify keifu_x_inheritance_r2 pass on it (2375505 checks)

## Exact next step
Build DESIGN.md's Systems in order on games/keifu-x-inheritance-r2/src: Hero.family + Hero.marks fields (hero.rs, household.rs, newcomers.rs; births set family), then the outsiders module (`outsiders.rs` — mainline already has a family.rs), courtship, family renown, heirs, marks, oath, inheritance, sheet, then checks in xi.rs.

## Deviations
- none
