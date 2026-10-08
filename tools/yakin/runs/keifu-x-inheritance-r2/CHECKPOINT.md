# CHECKPOINT — keifu-x-inheritance-r2

task: keifu-x-inheritance-r2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 19:55 PDT

## Done so far
- branch claimed; design stage started (designer tick)
- mainline Keifu read: SPEC §3, §5-§7, §9-§15, §17-§18, §20; CONSTANTS; hero, house, heirs, death_page, births, newcomers, wanderer, quest_card, quest_sheet, plans, resolve, harm, witness, legacy, coming_of_age, verify, w8, play, capture; make-game §A, §C-§E; §D check passed (the spec carries its three-row table)
- DESIGN.md written and released (3ff185c draft; this commit final): family flag, courtship threshold, family renown, family-only heirs, marks, the oath, inheritance (conceive + succession), sheet, checks, mutants, pictures

- implement stage: copy-and-rename commit (games/keifu -> games/keifu-x-inheritance-r2, crate keifu_x_inheritance_r2); tools/verify keifu_x_inheritance_r2 pass on it (2375505 checks) — 87f1ddf
- the variant's rules (family, outsiders, courtship, family renown, heirs/bequeath, marks, oath, inheritance, sheet) — 2aaae99; cargo test -p keifu_x_inheritance_r2 green (356)
- XI checks (xi.rs, xi_heirs.rs, xi_stages.rs, xi_tests.rs), three floors surfaces, three pictures, mutants/xi.txt; mainline oracles rewritten (w8 stirred, floors_w8 count, w7_controls garden help) — cbf43e4; tools/verify keifu_x_inheritance_r2 pass (2423836 checks)
- FINDINGS G-070..G-073 in games/keifu-x-inheritance-r2/FINDINGS.md (G-070, G-071 attributed to the design stage)
- 12 mainline faults re-cut to the variant's sites (labels marked "re-cut keifu-x-inheritance-r2")

## Exact next step
Mutation round (running at this commit, in the background; rerun if lost): `python3 tools/mutate keifu-x-inheritance-r2 mutants/*.txt --fast --changed-since 87f1ddf` (log target/yakin/mutate.log); tighten any escape with a check; then FINDINGS entries (G-070+) in games/keifu-x-inheritance-r2/FINDINGS.md, full gate (doctor, tools/test, check-claude-md, yakin check), build-web + serve-web --check, PR.

## Deviations
- the outsider module is `src/outsiders.rs`, not `src/family.rs`: mainline already has a family.rs (the top bar and family screen)
- `Posted.sworn` holds `Oath { by, need }` (still one value), and an oath is withdrawn after every drop whose party no longer lets its swearer swear it as sworn (`oath::keep_oaths`) — stricter than "left the card", so a party change that moves the call cannot leave a stale need
- the oath button is drawn by board_view (a plate at MARK, its label at TEXT), not `summer::button`, whose label band would land on the overlay's layer when drawn on a card; absent when nobody may swear
- NEWBORN_APTITUDE_SHARE removed (replaced by DOMINANT_SHARE)
- the card carries one oath line ("Sworn by Garrick: triumph, or a mark of 2 on the name. Fails 72 in 100.") and its button sits at the card's top right ("Swear it" / "Withdraw"): the design's two card lines and foot button overlapped the seat tiles (floors flagged it); the full stakes are on the quest sheet's SWORN BY block
- WOULD INHERIT lines read "Thornfall: +1 Might on quests." / "Garrick's rest: +2 against Undead" (the content's own effect wording), not "Thornfall (+1 Might)"
- the drain and the sheet's yearly-loss wording take MARK_DRAIN as an argument rather than a literal "-1"
- an outsider at the long table always writes `winter.tale_told_outsider`, in any seat
