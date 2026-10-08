# CHECKPOINT — brolf-r3v2

task: brolf-r3v2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:48 PDT

## Done so far
- branch claimed (claim commit)
- make-game §D check on the spec: the decision-surface table is present with four rows; well-formed
- DESIGN.md drafted whole, every template section filled (597d906)
- shipped literals verified numerically against the specified ball_step and ZONE_TABLE (roll 3.04/9.98/20.91/19.63, t_out 3335, gate excluded 2722); design released

## Exact next step
Worker (WORKER.md §3, V2): read tools/yakin/tasks/brolf-r3v2.md, tools/yakin/tasks/brolf.md and tools/yakin/runs/brolf-r3v2/DESIGN.md whole, then create games/brolf-r3v2/Cargo.toml and src/main.rs per DESIGN.md §Systems, building in the listed order starting with src/rules.rs (zone_at, ball_step, roll_out).

## Deviations
- none
