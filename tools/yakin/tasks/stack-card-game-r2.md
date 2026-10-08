# stack-card-game-r2 — A/B re-run of `stack-card-game`

Kind: game · Variant: V1 · Size: M · Window: burn-down (the same
values as the original's row)

This is a deliberate A/B re-run. Follow `tools/yakin/tasks/stack-card-game.md`
exactly, with the overrides below. That spec is this spec: its brief, steps,
decision-surface table and `## Done when` lines bind this task as written,
read through these overrides wherever they name the task, its game, its
branch, its run folder or its PR. Night one's run of it is
[#125](https://github.com/JHorace/jidousha/pull/125).

## Goal

The goal of `tools/yakin/tasks/stack-card-game.md`, unchanged, built fresh from the
spec alone, so its result can be compared with night one's.

## Overrides

- Task id is `stack-card-game-r2`; branch `claude/yakin-stack-card-game-r2`; run folder
  `tools/yakin/runs/stack-card-game-r2/`.
- Game id / folder / crate take an `-r2` suffix, so both runs' results can
  coexist in the workspace: the game is `games/stack-card-game-r2/`, its crate (package
  name) `stack_card_game_r2`. Wherever the original names its game — a path
  `games/stack-card-game/…`, or a name given to `tools/verify`, `tools/build-web` or
  `tools/serve-web` — use the r2 folder for paths and the r2 crate's playable
  (its default binary, `stack_card_game_r2`) for the tools.
- **Comparison hygiene:** do not open, read, or consult the night-one
  branches, PRs, or any `games/` content the original task produced. Work
  fresh from the spec alone. Reading the prior solution contaminates the
  comparison that is this task's entire purpose.
- PR title `[yakin:V1:r2] Stack card game: a two-player duel fought over the stack`; the PR body names its night-one
  counterpart PR, #125, as the comparison baseline.

## Decisions

The original's `## Decisions this task adds` table, incorporated whole and unchanged — it is
this task's decision-surface table, and DOCTRINE §6's check runs on it there.

## Fence

The original's `## Fence`, with its paths renamed: write access to
`games/stack-card-game-r2/**` (new), `Cargo.lock` for the `stack_card_game_r2` crate's own entry
only, and `tools/yakin/runs/stack-card-game-r2/`. Nothing else — in particular not
`games/stack-card-game/` (night one's, if it ever lands). No new dependencies.

## Done when

Every `## Done when` line of `tools/yakin/tasks/stack-card-game.md`, read through the
overrides above — in particular:

- `git diff --stat origin/main...` names only `games/stack-card-game-r2/`, `Cargo.lock`
  (the `stack_card_game_r2` entry only) and `tools/yakin/runs/stack-card-game-r2/`.
- `python3 tools/verify stack_card_game_r2` and `python3 tools/build-web stack_card_game_r2 && python3 tools/serve-web stack_card_game_r2 --check`
  pass, in place of the original's commands for `stack-card-game`.
- A PR titled `[yakin:V1:r2] Stack card game: a two-player duel fought over the stack` is open with the
  WORKER.md body, naming #125 as its comparison baseline.
