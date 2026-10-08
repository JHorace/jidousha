# keifu-fixes-r2 — A/B re-run of `keifu-fixes`

Kind: game · Variant: V1 · Size: M · Window: burn-down (the same
values as the original's row)

This is a deliberate A/B re-run. Follow `tools/yakin/tasks/keifu-fixes.md`
exactly, with the overrides below. That spec is this spec: its brief, steps,
decision-surface table and `## Done when` lines bind this task as written,
read through these overrides wherever they name the task, its game, its
branch, its run folder or its PR. Night one's run of it is
[#126](https://github.com/JHorace/jidousha/pull/126).

## Goal

The goal of `tools/yakin/tasks/keifu-fixes.md`, unchanged, built fresh from the
spec alone, so its result can be compared with night one's.

## Overrides

- Task id is `keifu-fixes-r2`; branch `claude/yakin-keifu-fixes-r2`; run folder
  `tools/yakin/runs/keifu-fixes-r2/`.
- The game is mainline `games/keifu` — no game-id suffix, by design: this
  task targets the same game as its original, and the exception below is
  why that is safe. Its tool name stays `keifu`.
- **Comparison hygiene:** do not open, read, or consult the night-one
  branches, PRs, or any `games/` content the original task produced. Work
  fresh from the spec alone. Reading the prior solution contaminates the
  comparison that is this task's entire purpose.
- PR title `[yakin:V1:r2] Keifu: clearer tutorial, next-summer foresight, trouble telegraphs`; the PR body names its night-one
  counterpart PR, #126, as the comparison baseline.

## Owner-sanctioned exception

Owner-sanctioned exception to the one-direct-edit-per-game rule
(2026-10-07): this task edits mainline `games/keifu` while its night-one
counterpart PR is open. That is the point — the two PRs are an A/B pair.
State in the PR body, prominently: "A/B counterpart of PR #126 - at most ONE
of the pair may ever merge." Do not rebase onto, read, or resolve conflicts
with the counterpart.

## Decisions

The original's `## Decisions this task changes` table, incorporated whole and unchanged — it is
this task's decision-surface table, and DOCTRINE §6's check runs on it there.

## Fence

The original's `## Fence`, unchanged except for the run folder: write
access to `games/keifu/**` and `tools/yakin/runs/keifu-fixes-r2/`, nothing else. No
`Cargo.lock` change, no new dependencies, and determinism and the existing
`--verify` record stay intact exactly as the original says.

## Done when

Every `## Done when` line of `tools/yakin/tasks/keifu-fixes.md`, read through the
overrides above — in particular:

- `git diff --stat origin/main...` names only `games/keifu/` and
  `tools/yakin/runs/keifu-fixes-r2/`.
- A PR titled `[yakin:V1:r2] Keifu: clearer tutorial, next-summer foresight, trouble telegraphs` is open with the
  WORKER.md body, naming #126 as its comparison baseline, and carrying the exception's A/B line prominently.
