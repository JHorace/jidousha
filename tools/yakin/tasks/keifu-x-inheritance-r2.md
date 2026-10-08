# keifu-x-inheritance-r2 — A/B re-run of `keifu-x-inheritance`

Kind: game · Variant: V2 · Size: L · Window: burn-down (the same
values as the original's row)

This is a deliberate A/B re-run. Follow `tools/yakin/tasks/keifu-x-inheritance.md`
exactly, with the overrides below. That spec is this spec: its brief, steps,
decision-surface table and `## Done when` lines bind this task as written,
read through these overrides wherever they name the task, its game, its
branch, its run folder or its PR. Night one's run of it is
[#129](https://github.com/JHorace/jidousha/pull/129).

## Goal

The goal of `tools/yakin/tasks/keifu-x-inheritance.md`, unchanged, built fresh from the
spec alone, so its result can be compared with night one's.

## Overrides

- Task id is `keifu-x-inheritance-r2`; branch `claude/yakin-keifu-x-inheritance-r2`; run folder
  `tools/yakin/runs/keifu-x-inheritance-r2/`.
- Game id / folder / crate take an `-r2` suffix, so both runs' results can
  coexist in the workspace: the game is `games/keifu-x-inheritance-r2/`, its crate (package
  name) `keifu_x_inheritance_r2`. Wherever the original names its game — a path
  `games/keifu-x-inheritance/…`, or a name given to `tools/verify`, `tools/build-web` or
  `tools/serve-web` — use the r2 folder for paths and the r2 crate's playable
  (its default binary, `keifu_x_inheritance_r2`) for the tools.
- **Comparison hygiene:** do not open, read, or consult the night-one
  branches, PRs, or any `games/` content the original task produced. Work
  fresh from the spec alone. Reading the prior solution contaminates the
  comparison that is this task's entire purpose.
- PR title `[yakin:V2:r2] Keifu X Inheritance: fork Keifu and deepen what a family inherits`; the PR body names its night-one
  counterpart PR, #129, as the comparison baseline.

## Decisions

The original's `## Decisions this task adds` table, incorporated whole and unchanged — it is
this task's decision-surface table, and DOCTRINE §6's check runs on it there.

## Fence

The original's `## Fence`, with its paths renamed: write access to
`games/keifu-x-inheritance-r2/**` (new), `Cargo.lock` for the `keifu_x_inheritance_r2` crate's own entry
only, and `tools/yakin/runs/keifu-x-inheritance-r2/`. Nothing else — in particular not
`games/keifu-x-inheritance/` (night one's, if it ever lands), nor mainline `games/keifu/`, which stays read-only as the original says. No new dependencies.

## Done when

Every `## Done when` line of `tools/yakin/tasks/keifu-x-inheritance.md`, read through the
overrides above — in particular:

- `git diff --stat origin/main...` names only `games/keifu-x-inheritance-r2/`, `Cargo.lock`
  (the `keifu_x_inheritance_r2` entry only) and `tools/yakin/runs/keifu-x-inheritance-r2/`.
- `python3 tools/verify keifu_x_inheritance_r2` and `python3 tools/build-web keifu_x_inheritance_r2 && python3 tools/serve-web keifu_x_inheritance_r2 --check`
  pass, in place of the original's commands for `keifu-x-inheritance`.
- A PR titled `[yakin:V2:r2] Keifu X Inheritance: fork Keifu and deepen what a family inherits` is open with the
  WORKER.md body, naming #129 as its comparison baseline.
