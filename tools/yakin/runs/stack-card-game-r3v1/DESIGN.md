# DESIGN — stack-card-game-r3v1 (V1, inline)

The design is the game's own note: `games/stack-card-game-r3v1/DESIGN.md`
(rules, card pool, why stack manipulation dominates, screen, input).

## What will be built, in order

1. `games/stack-card-game-r3v1/` crate: `Cargo.toml` (jidousha only), `src/main.rs`.
2. `src/rules.rs` — pure duel model: `Duel`, `play`, `pass`, `resolve_top`,
   `preview`, `legal_targets`, deck shuffle from the seed.
3. `src/rival.rs` — `rival_choice` (one-ply over `preview`).
4. `src/screen.rs` — the `Panel` (jidousha::ui) for the stack, priority, hand,
   log, result; hit-test rects shared with drawing.
5. `src/verify.rs`, `src/checks.rs`, `src/players.rs`, `src/capture.rs` — the
   `--verify` mode: three players (reader / raw power / idle), the three
   decision-row checks, floors, staged screens, schedule order, capture.
6. `mutants/round1.txt` — run `tools/mutate`.

## How each Done-when line is checked

- DESIGN.md exists → the file above.
- Full match end to end + result screen → `--verify` plays the reader to a
  result and asserts the result screen draws.
- `tools/verify stack_card_game_r3v1` pass with one check per decision row →
  named checks `respond-window`, `pass-resolves-as-previewed`,
  `target-reorder` / `target-counter`, printed in the summary.
- diff stat → `git diff --stat origin/main...` before the PR.
- web → `tools/build-web stack_card_game_r3v1 && tools/serve-web stack_card_game_r3v1 --check`.
