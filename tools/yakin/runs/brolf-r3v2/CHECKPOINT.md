# CHECKPOINT — brolf-r3v2

task: brolf-r3v2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 22:03 PDT

## Done so far
- branch claimed (claim commit)
- make-game §D check on the spec: the decision-surface table is present with four rows; well-formed
- DESIGN.md drafted whole, every template section filled (597d906)
- shipped literals verified numerically against the specified ball_step and ZONE_TABLE (roll 3.04/9.98/20.91/19.63, t_out 3335, gate excluded 2722); design released
- implement: games/brolf-r3v2/ built per DESIGN.md Systems; --verify passes (4 decision gates, players, layout, staged, contracts, capture); two design flaws fixed and logged: the cup opens at tick 7200 (PIKE's first DRIVE holed out at tick 83), the Hunter only clubs un-disabled rivals with loot (stun-lock) (this commit)

## Exact next step
Write games/brolf-r3v2/mutants/r1.txt (DESIGN.md's fourteen faults plus CUP_OPENS and the Hunter's prey filter), run `python3 tools/mutate brolf_r3v2 mutants/r1.txt`, fix escapes; write games/brolf-r3v2/FINDINGS.md (G-073 cup, G-074 stun-lock, G-075 good player loses at the opening); full gate; build-web/serve-web --check; PR.

## Deviations
- cup opens at tick 7200 (CUP_OPENS); roll_out takes the first tick so the aim line knows; HUD line 2 shows the cup state. DESIGN.md left the cup open from tick 1 and PIKE holed out with its first shot on tick 83.
- Hunter prey: un-disabled rivals holding loot a club knocks loose (DESIGN.md: any rival within 8, which stun-locks).
- gates 2-4 freeze the NPCs not under test (a staged change) so no rival interferes.
- design stage: none
