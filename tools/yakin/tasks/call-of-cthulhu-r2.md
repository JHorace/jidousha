# call-of-cthulhu-r2 — A/B re-run of `call-of-cthulhu`

Kind: game · Variant: V1 · Size: M · Window: burn-down (the same
values as the original's row)

This is a deliberate A/B re-run. Follow `tools/yakin/tasks/call-of-cthulhu.md`
exactly, with the overrides below. That spec is this spec: its brief, steps,
decision-surface table and `## Done when` lines bind this task as written,
read through these overrides wherever they name the task, its game, its
branch, its run folder or its PR. Night one's run of it is
[#127](https://github.com/JHorace/jidousha/pull/127).

## Goal

The goal of `tools/yakin/tasks/call-of-cthulhu.md`, unchanged, built fresh from the
spec alone, so its result can be compared with night one's.

## Overrides

- Task id is `call-of-cthulhu-r2`; branch `claude/yakin-call-of-cthulhu-r2`; run folder
  `tools/yakin/runs/call-of-cthulhu-r2/`.
- Game id / folder / crate take an `-r2` suffix, so both runs' results can
  coexist in the workspace: the game is `games/call-of-cthulhu-r2/`, its crate (package
  name) `call_of_cthulhu_r2`. Wherever the original names its game — a path
  `games/call-of-cthulhu/…`, or a name given to `tools/verify`, `tools/build-web` or
  `tools/serve-web` — use the r2 folder for paths and the r2 crate's playable
  (its default binary, `call_of_cthulhu_r2`) for the tools.
- **Comparison hygiene:** do not open, read, or consult the night-one
  branches, PRs, or any `games/` content the original task produced. Work
  fresh from the spec alone. Reading the prior solution contaminates the
  comparison that is this task's entire purpose.
- PR title `[yakin:V1:r2] Call of Cthulhu: hang up on eldritch callers before sanity runs out`; the PR body names its night-one
  counterpart PR, #127, as the comparison baseline.

## Decisions

The original's `## Decisions this task adds` table, incorporated whole and unchanged — it is
this task's decision-surface table, and DOCTRINE §6's check runs on it there.

## Fence

The original's `## Fence`, with its paths renamed: write access to
`games/call-of-cthulhu-r2/**` (new), `Cargo.lock` for the `call_of_cthulhu_r2` crate's own entry
only, and `tools/yakin/runs/call-of-cthulhu-r2/`. Nothing else — in particular not
`games/call-of-cthulhu/` (night one's, if it ever lands). No new dependencies.

## Done when

Every `## Done when` line of `tools/yakin/tasks/call-of-cthulhu.md`, read through the
overrides above — in particular:

- `git diff --stat origin/main...` names only `games/call-of-cthulhu-r2/`, `Cargo.lock`
  (the `call_of_cthulhu_r2` entry only) and `tools/yakin/runs/call-of-cthulhu-r2/`.
- `python3 tools/verify call_of_cthulhu_r2` and `python3 tools/build-web call_of_cthulhu_r2 && python3 tools/serve-web call_of_cthulhu_r2 --check`
  pass, in place of the original's commands for `call-of-cthulhu`.
- A PR titled `[yakin:V1:r2] Call of Cthulhu: hang up on eldritch callers before sanity runs out` is open with the
  WORKER.md body, naming #127 as its comparison baseline.
