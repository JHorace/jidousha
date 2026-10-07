# CHECKPOINT — brolf

task: brolf
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 11:09 PDT

## Done so far
- claimed (d21cb8f)
- DESIGN.md drafted whole: every section filled (fe9658f)
- DESIGN.md re-read against the spec's four rows; staged sessions freeze unnamed NPCs, NPCs gather items, the hole opens late (this commit) — design released

## Exact next step
Implement stage (WORKER.md). Read tools/yakin/runs/brolf/DESIGN.md whole, then build in its "Systems" order: `games/brolf/Cargo.toml` + `src/main.rs` (layout, config, --verify switch) → `src/items.rs` → `src/zone.rs` → `src/shots.rs` → `src/contact.rs` → `src/outcome.rs` → `src/world.rs` → `src/intent.rs` → `src/systems.rs` → `src/draw.rs` → `cargo check`/clippy → `src/players.rs` → `src/verify.rs` + `src/checks.rs` (gates A–F) → `src/capture.rs` → `mutants/m0.txt` round → `tools/build-web brolf && tools/serve-web brolf --check` → PR.

## Deviations
- none
