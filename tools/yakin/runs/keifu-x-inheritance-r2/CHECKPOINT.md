# CHECKPOINT — keifu-x-inheritance-r2

task: keifu-x-inheritance-r2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 19:21 PDT

## Done so far
- branch claimed; design stage started (designer tick)
- mainline Keifu read: SPEC §3, §5-§7, §9-§15, §17-§18, §20; CONSTANTS; hero, house, heirs, death_page, births, newcomers, wanderer, quest_card, quest_sheet, plans, resolve, harm, witness, legacy, coming_of_age, verify, w8, play, capture; make-game §A, §C-§E; §D check passed (the spec carries its three-row table)
- DESIGN.md written and released (3ff185c draft; this commit final): family flag, courtship threshold, family renown, family-only heirs, marks, the oath, inheritance (conceive + succession), sheet, checks, mutants, pictures

- implement stage: copy-and-rename commit (games/keifu -> games/keifu-x-inheritance-r2, crate keifu_x_inheritance_r2); tools/verify keifu_x_inheritance_r2 pass on it (2375505 checks) — 87f1ddf
- the variant's rules (family, outsiders, courtship, family renown, heirs/bequeath, marks, oath, inheritance, sheet) — 2aaae99; cargo test -p keifu_x_inheritance_r2 green (356)

## Exact next step
Run `python3 tools/verify keifu_x_inheritance_r2`, rewrite any mainline oracle the variant breaks (w8::stirred's heir list etc.; each a PR deviation), then write src/xi.rs (+ xi_stages.rs): DESIGN.md's checks (oath, oath eligibility, marry-in, neither family, succession, outsider death, family renown, drain), register them in verify::run, three floors stages, three capture rows, mutants/xi.txt.

## Deviations
- the outsider module is `src/outsiders.rs`, not `src/family.rs`: mainline already has a family.rs (the top bar and family screen)
- `Posted.sworn` holds `Oath { by, need }` (still one value), and an oath is withdrawn after every drop whose party no longer lets its swearer swear it as sworn (`oath::keep_oaths`) — stricter than "left the card", so a party change that moves the call cannot leave a stale need
- the oath button is drawn by board_view (a plate at MARK, its label at TEXT), not `summer::button`, whose label band would land on the overlay's layer when drawn on a card; absent when nobody may swear
- NEWBORN_APTITUDE_SHARE removed (replaced by DOMINANT_SHARE)
