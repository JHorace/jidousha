# CHECKPOINT — keifu-x-inheritance-r3v2

task: keifu-x-inheritance-r3v2
variant: V2
stage: done
tick: released
resumes: 0
gates-green-at: 6d08146
updated: 2026-10-07 22:45 PDT

## Done so far
- claimed the branch (b5d64bf)
- read mainline Keifu (spec, constants, gaps, findings, the modules the base spec names) and docs/api; drafted DESIGN.md whole, every section filled (70fdc80)
- reviewed DESIGN.md against the mainline source and released it: stage implement (this commit)
- S0 fork commit 5981641: verify keifu_x_inheritance_r3v2 pass, verify keifu pass, fast gate clean
- S1-S7 rules in (is_family, family_house, marks.rs, inheritance.rs, reward/tellers/heirs gates, step 7b and 8b, death page + choose, courtship Unproven/MarriesIn, sheets, quest-sheet stake, content words, constants); unit tests 365/365 after rewriting 5 mainline unit oracles the variant breaks
- S8 checks xi_marks / xi_outsiders / xi_inheritance wired into verify; w8::stirred rewritten; floors W4 mid-drag -> look_held; floors_w8 stirred page wanderer made family; VARIANT.md; plans_tests +3; verify keifu_x_inheritance_r3v2 pass
- mutants/xi.txt: 21/26 first pass; G7 unit tests + birth tests added; rerun 4/5, X12 equivalent (25/26); three pictures; FINDINGS G-075..G-077
- full gate green at 6d08146: doctor ENV_OK, tools/test pass (1860/0/0, verify keifu and the variant pass), check-claude-md ok, yakin check ok; build-web + serve-web --check pass

## Exact next step
None — PR https://github.com/JHorace/jidousha/pull/145 is open.

## Deviations
- 5 mainline unit tests rewritten to variant rules (births_tests x3: lean bonus; quest_sheet grave goods: stake line; resolve_tests disaster: Ysolde's mark) - DESIGN.md said no other mainline check moves
- birthright takes the child's own best aptitude as a parameter (DESIGN.md's signature has no way to know it)
- quest.outsiders_reward names the family house by argument instead of the literal "Thorne"
- the birth page's lean line sits after the fear line, before blessings (DESIGN.md names no position)
- floors.rs W4 mid-drag judged with look_held (the lean line makes Brannoc's held sheet longer than the dock)
- floors_w8 stirred page: the wanderer made family so the floor still judges eight buttons
