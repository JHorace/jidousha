# brolf-r2 — A/B re-run of `brolf`

Kind: game · Variant: V2 · Size: M · Window: burn-down (the same
values as the original's row)

This is a deliberate A/B re-run. Follow `tools/yakin/tasks/brolf.md`
exactly, with the overrides below. That spec is this spec: its brief, steps,
decision-surface table and `## Done when` lines bind this task as written,
read through these overrides wherever they name the task, its game, its
branch, its run folder or its PR. Night one's run of it is
[#128](https://github.com/JHorace/jidousha/pull/128).

## Goal

The goal of `tools/yakin/tasks/brolf.md`, unchanged, built fresh from the
spec alone, so its result can be compared with night one's.

## Overrides

- Task id is `brolf-r2`; branch `claude/yakin-brolf-r2`; run folder
  `tools/yakin/runs/brolf-r2/`.
- Game id / folder / crate take an `-r2` suffix, so both runs' results can
  coexist in the workspace: the game is `games/brolf-r2/`, its crate (package
  name) `brolf_r2`. Wherever the original names its game — a path
  `games/brolf/…`, or a name given to `tools/verify`, `tools/build-web` or
  `tools/serve-web` — use the r2 folder for paths and the r2 crate's playable
  (its default binary, `brolf_r2`) for the tools.
- **Comparison hygiene:** do not open, read, or consult the night-one
  branches, PRs, or any `games/` content the original task produced. Work
  fresh from the spec alone. Reading the prior solution contaminates the
  comparison that is this task's entire purpose.
- PR title `[yakin:V2:r2] Brolf: battle-royale golf prototype against NPC opponents`; the PR body names its night-one
  counterpart PR, #128, as the comparison baseline.

## Decisions

The original's `## Decisions this task adds` table, incorporated whole and unchanged — it is
this task's decision-surface table, and DOCTRINE §6's check runs on it there.

## Fence

The original's `## Fence`, with its paths renamed: write access to
`games/brolf-r2/**` (new), `Cargo.lock` for the `brolf_r2` crate's own entry
only, and `tools/yakin/runs/brolf-r2/`. Nothing else — in particular not
`games/brolf/` (night one's, if it ever lands). No new dependencies.

## Done when

Every `## Done when` line of `tools/yakin/tasks/brolf.md`, read through the
overrides above — in particular:

- `git diff --stat origin/main...` names only `games/brolf-r2/`, `Cargo.lock`
  (the `brolf_r2` entry only) and `tools/yakin/runs/brolf-r2/`.
- `python3 tools/verify brolf_r2` and `python3 tools/build-web brolf_r2 && python3 tools/serve-web brolf_r2 --check`
  pass, in place of the original's commands for `brolf`.
- A PR titled `[yakin:V2:r2] Brolf: battle-royale golf prototype against NPC opponents` is open with the
  WORKER.md body, naming #128 as its comparison baseline.
