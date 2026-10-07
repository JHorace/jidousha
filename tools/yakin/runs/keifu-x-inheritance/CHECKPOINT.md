# CHECKPOINT — keifu-x-inheritance

task: keifu-x-inheritance
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 09:44 PDT

## Done so far
- branch claimed (32c3d1d); mainline Keifu read; DESIGN.md written and released by the designer
- S0 landed (075ff00): pure copy-and-rename, `tools/verify keifu-x-inheritance` passes on it
- S1 landed: Hero.family, src/outsiders.rs (`may_marry_in`, `marrying_in`), Courtship::Unproven, marrying in (winter::court), reward/tellers/heirs/nearest_kin/legacy::heir/death page for outsiders, sheet + arrival lines; unit tests in outsiders_tests.rs; W8 stirred oracle rewritten (seven buttons); W9 battery tolerates a renamed spouse's epitaph being recomposed; verify passes, fast gate clean
- S2 landed: ids::Trait + src/traits.rs (`trait_from`, `birth_traits`, power), lore.json `traits`, household.json `trait` keys, births rolls + lines, power/power_lines trait line (not at the Door), sheet TRAITS; rewritten mainline oracles (W1 sheet, W3 power line 7, W4 you bring 14 + odds + lines, w4_rules patrons, W6 played page) and unit tests; verify passes, fast gate clean

## Exact next step
S3 (marks) per DESIGN.md: src/marks.rs (`Mark`, `personal(content, heroes, facts, party)`, `mark_the_name`, `weigh`), Hero.marks, DeedKind::Marked, House::marks_carried, resolve.rs step 7b, quest_card `personal` row, quest_sheet `PERSONAL:` line, turning.rs step 8b, sheet MARKS section, constants MARK_*; then S4 (src/inheritance.rs `inherit`, heirs::choose, death page preview lines in turning_view::lay_out_choice, births marks), then S5 (src/x1.rs checks, floors_x1.rs, pictures, mutants/x1.txt, VARIANT.md, FINDINGS).

## Deviations
- DESIGN.md names a new `src/family.rs`; mainline already has `family.rs` (the family screen), so the one eligibility function lives in `src/outsiders.rs` (`is_family`, `may_marry_in`, `marrying_in`). A FINDINGS entry will be filed (design misled).
- `Trait` is the `ids!` enum (Strong..Faint) with `aptitude()`/`gift()`, not a `{aptitude, gift}` struct; `trait_from(first, second, aptitude, rng)` returns the source too; a spring that lands on a parent's own trait is told as that parent's.
- Traits are not read at the Door (`QuestFacts::door_lock`), per "nothing else reads a trait"; W10 oracles are unchanged.
- Floors: `floors::look` does not wheel-scroll a held hero's sheet (a wheel over the dock lets go of the hero; the TRAITS section made Brannoc's sheet longer than the dock).
- Mainline unit tests that count Garrick and Brannoc's power without the traits use `testkit::house_without_traits()` (easing, reading x2, resolve x2, telling); the breakdown tests (power, power_lines, quest_card, quest_sheet) are rewritten to the variant's literals.
- Epitaphs of the dead are composed again when an outsider marries in (the name changes everywhere it is drawn); the W9 battery's "changed with nothing moved" accepts a rename.
- `lines.json`/`ui-text.json` entries carry `source: "keifu-x-inheritance VARIANT.md"` each instead of one `_variant` key per file.
