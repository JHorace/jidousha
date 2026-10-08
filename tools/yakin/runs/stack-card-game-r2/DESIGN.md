# DESIGN — stack-card-game-r2 (V1, inline)

The game's design note is `games/stack-card-game-r2/DESIGN.md` (rules, pool,
why stack manipulation dominates, decision surfaces). This file is the build plan.

## Files (all inside the Fence)

- `games/stack-card-game-r2/Cargo.toml` — package `stack_card_game_r2`, jidousha by path only.
- `src/main.rs` — config, `register`, the ECS shell (Startup, player input, NPC, restart, draw).
- `src/cards.rs` — the card table and the two decks.
- `src/duel.rs` — the pure rules: `Duel`, `play`, `pass`, `legal`, `preview`.
- `src/npc.rs` — `choose`, the Brute's (and the good controller's) decision.
- `src/screen.rs` — layout constants, the `Panel` builder, controls, hit-testing.
- `src/checks.rs`, `src/verify.rs`, `src/capture.rs` — the `--verify` mode.
- `mutants/r1.txt` — the mutation round's list.
- `DESIGN.md`, `FINDINGS.md`.
- `Cargo.lock` — the new crate's entry only.

## Order

1. Rules core (duel + cards) with preview and legality as free functions.
2. NPC on top of preview.
3. Screen + input + main loop; play headless.
4. `--verify`: layout/bounds checks, the three decision-row checks, three players, staged result screens, schedule order.
5. Capture; mutation round; full gate; web check; PR.

## Done-when → how checked

- DESIGN.md states rules, pool, why → the file above.
- Full match end-to-end, verify reaches result screen → verify's good-player match reports a winner and the result frame is judged.
- `tools/verify stack_card_game_r2` pass with a check per decision row → `target/verify/stack_card_game_r2.json`, check names in the summary.
- diff names only the fenced paths → `git diff --stat origin/main...`.
- build-web + serve-web --check → run both.
- PR open → WORKER.md §4.
