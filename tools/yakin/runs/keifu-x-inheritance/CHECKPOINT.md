# CHECKPOINT — keifu-x-inheritance

task: keifu-x-inheritance
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 09:29 PDT

## Done so far
- branch claimed (32c3d1d); mainline Keifu read; DESIGN.md written and released by the designer
- S0 landed (075ff00): pure copy-and-rename, `tools/verify keifu-x-inheritance` passes on it
- S1 landed: Hero.family, src/outsiders.rs (`may_marry_in`, `marrying_in`), Courtship::Unproven, marrying in (winter::court), reward/tellers/heirs/nearest_kin/legacy::heir/death page for outsiders, sheet + arrival lines; unit tests in outsiders_tests.rs; W8 stirred oracle rewritten (seven buttons); W9 battery tolerates a renamed spouse's epitaph being recomposed; verify passes, fast gate clean

## Exact next step
S2 (traits) per DESIGN.md: add the six traits to the fork's spec/content/lore.json `traits` table, `src/traits.rs` (`Trait`, `trait_from`), `Hero.traits`, household.json `trait` keys, births rolls and lines, power.rs/power_lines.rs trait line, sheet TRAITS section, constants TRAIT_*; then S3 marks, S4 inheritance, S5 checks/floors/pictures/mutants/VARIANT.md. Add words with `W` variants only as used (clippy -D warnings rejects unused).

## Deviations
- DESIGN.md names a new `src/family.rs`; mainline already has `family.rs` (the family screen), so the one eligibility function lives in `src/outsiders.rs` (`is_family`, `may_marry_in`, `marrying_in`). A FINDINGS entry will be filed (design misled).
- Epitaphs of the dead are composed again when an outsider marries in (the name changes everywhere it is drawn); the W9 battery's "changed with nothing moved" accepts a rename.
- `lines.json`/`ui-text.json` entries carry `source: "keifu-x-inheritance VARIANT.md"` each instead of one `_variant` key per file.
