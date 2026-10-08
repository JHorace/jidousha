# DESIGN — call-of-cthulhu-r2 (V1, inline)

The game's own design note is `games/call-of-cthulhu-r2/DESIGN.md` (beings,
call structure, day loop, sanity arithmetic). This file is the build plan.

## Files (all inside the Fence)

- `games/call-of-cthulhu-r2/Cargo.toml` — package `call_of_cthulhu_r2`, jidousha by path only.
- `src/main.rs` — config, `register`, the input and draw systems.
- `src/lore.rs` — the beings, facts, questions, answers (content).
- `src/rules.rs` — constants and the pure functions: `answer_outcome`, `drain`, `tonight`, `morning_options`, `apply_morning`, `effect_line`.
- `src/screens.rs` — one `jidousha::ui::Panel` per phase plus the option rows (draw and hit-test read the same rows).
- `src/verify.rs`, `src/players.rs`, `src/checks.rs`, `src/capture.rs` — the `--verify` mode.
- `mutants/r2.txt` — the mutation round's list.
- `DESIGN.md`, `FINDINGS.md`.
- `Cargo.lock` — the crate's own entry.

## Order

Content and rules first (pure, testable), then screens, then systems, then
verify with three players (scholar / first-timer / silent), decision-row checks,
floors + judge_frame, capture, mutation round, web check.

## Done-when → how checked

- DESIGN.md names beings/call/day loop/sanity → the file above.
- Plays end to end, win and loss → verify's scholar run wins, silent run loses; both asserted.
- `tools/verify call_of_cthulhu_r2` pass with a check per decision row → `decision row 1` / `decision row 2` checks in verify.rs, report file.
- diff names only the fenced paths → `git diff --stat origin/main...`.
- build-web + serve-web --check → run both.
- PR → WORKER.md §4.
