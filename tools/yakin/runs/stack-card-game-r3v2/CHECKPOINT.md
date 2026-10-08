# CHECKPOINT — stack-card-game-r3v2

task: stack-card-game-r3v2
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:17 PDT

## Done so far
- claimed (32f5e58)
- DESIGN.md drafted whole: rules, pool, why manipulation dominates, systems, 13 gates, decisions, open calls (eafe24f)
- FINDINGS.md entry 1: `Flat` undefined in jidousha-ui.md's judge_frame example (eafe24f)
- DESIGN.md re-read and fixed: G4 restaged so passes = 1 is reachable, G5 pass count, resolution-order cell, cell widths vs string lengths, blurbs <= 16 chars, says lines <= 20 chars; design released (this commit)

## Exact next step
Implement stage (WORKER.md §3, V2): build games/stack-card-game-r3v2/ from tools/yakin/runs/stack-card-game-r3v2/DESIGN.md, in the Systems order there (cards.rs, rules.rs, duel.rs, players.rs, main.rs, screen.rs, verify.rs + checks.rs, capture.rs, mutants/round1.txt). First command: cp -r the crate layout from crates/jidousha/examples/prototype_kit as the shape, write Cargo.toml (package stack_card_game_r3v2, jidousha by path only, [lints] workspace = true), then `cargo check -p stack_card_game_r3v2` after each file. Do not open any other round's branches, PRs or games/ folders for this idea (spec, Comparison hygiene).

## Deviations
- none at design stage. The game's FINDINGS.md (make-game §C) should carry the run folder's FINDINGS.md entry 1 forward.
