# CHECKPOINT — keifu-x-inheritance

task: keifu-x-inheritance
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 09:56 PDT

## Done so far
- branch claimed (32c3d1d); mainline Keifu read; DESIGN.md written and released by the designer
- S0 landed (075ff00): pure copy-and-rename, `tools/verify keifu-x-inheritance` passes on it
- S1 landed: Hero.family, src/outsiders.rs (`may_marry_in`, `marrying_in`), Courtship::Unproven, marrying in (winter::court), reward/tellers/heirs/nearest_kin/legacy::heir/death page for outsiders, sheet + arrival lines; unit tests in outsiders_tests.rs; W8 stirred oracle rewritten (seven buttons); W9 battery tolerates a renamed spouse's epitaph being recomposed; verify passes, fast gate clean
- S2 landed: ids::Trait + src/traits.rs (`trait_from`, `birth_traits`, power), lore.json `traits`, household.json `trait` keys, births rolls + lines, power/power_lines trait line (not at the Door), sheet TRAITS; rewritten mainline oracles (W1 sheet, W3 power line 7, W4 you bring 14 + odds + lines, w4_rules patrons, W6 played page) and unit tests; verify passes, fast gate clean
- S3+S4 landed: src/marks.rs (`personal`, `cost`, `mark_the_name`, `weigh`), Hero.marks, DeedKind::Marked, House::marks_carried, resolve step 7b, turning step 8b, card `Marked if it fails:` row, sheet `PERSONAL:` line and MARKS section; src/inheritance.rs (`inherit`), heirs::choose/pass_marks, heirs::heir_lines + turning_view candidate lines (choice_height), death page waits on marks, births under marks; unit tests marks_tests/inheritance_tests; W4 history read with the wheel at the end; verify passes, fast gate clean

## Exact next step
S5: src/x1.rs + src/x1_stages.rs (the three decision checks G1/G4/G8 played through the scripted pointer, plus G2, G3 `check_weighing`, G5, G9, wired into verify.rs), choose `X1_SEED` by an off-screen sweep, src/floors_x1.rs (G11, counted into floors::battery), four pictures in capture.rs (G12), mutants/x1.txt and `python3 tools/mutate keifu-x-inheritance mutants/x1.txt --fast` (G13), VARIANT.md at the crate root, FINDINGS.md/SPEC-GAPS.md continued, the battery numbers for the PR; then the full gate (doctor, tools/test in the background, check-claude-md, yakin check), web build + serve-web --check, PR.

## Deviations
- DESIGN.md names a new `src/family.rs`; mainline already has `family.rs` (the family screen), so the one eligibility function lives in `src/outsiders.rs` (`is_family`, `may_marry_in`, `marrying_in`). A FINDINGS entry will be filed (design misled).
- `Trait` is the `ids!` enum (Strong..Faint) with `aptitude()`/`gift()`, not a `{aptitude, gift}` struct; `trait_from(first, second, aptitude, rng)` returns the source too; a spring that lands on a parent's own trait is told as that parent's.
- Traits are not read at the Door (`QuestFacts::door_lock`), per "nothing else reads a trait"; W10 oracles are unchanged.
- Floors: `floors::look` does not wheel-scroll a held hero's sheet (a wheel over the dock lets go of the hero; the TRAITS section made Brannoc's sheet longer than the dock).
- Mainline unit tests that count Garrick and Brannoc's power without the traits use `testkit::house_without_traits()` (easing, reading x2, resolve x2, telling); the breakdown tests (power, power_lines, quest_card, quest_sheet) are rewritten to the variant's literals.
- Epitaphs of the dead are composed again when an outsider marries in (the name changes everywhere it is drawn); the W9 battery's "changed with nothing moved" accepts a rename.
- `lines.json`/`ui-text.json` entries carry `source: "keifu-x-inheritance VARIANT.md"` each instead of one `_variant` key per file.
