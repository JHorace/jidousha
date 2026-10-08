# keifu-x-inheritance-r3v2 — round-three V2 run of `keifu-x-inheritance`

Kind: game · Variant: V2 · Size: L · Window: burn-down

Round-three paired run. Follow `tools/yakin/tasks/keifu-x-inheritance.md`
exactly, with the overrides below. That spec is this spec: its brief, steps,
decision-surface table and `## Done when` lines bind this task as written,
read through these overrides wherever they name the task, its game, its
branch, its run folder, its variant or its PR.

Round three runs `keifu-x-inheritance` twice from the same spec: as V1
(`keifu-x-inheritance-r3v1` — one worker tick lineage designs inline and
implements) and as V2 (`keifu-x-inheritance-r3v2` — a designer tick writes
DESIGN.md per the template, and a later worker tick builds from it). The two
runs are a pair, and comparing them is the experiment. Earlier rounds ran this
idea as night one's `keifu-x-inheritance`
([#129](https://github.com/JHorace/jidousha/pull/129)), round two's
`keifu-x-inheritance-r2` (branch `claude/yakin-keifu-x-inheritance-r2`); those
are the comparison baselines, never sources.

## Goal

The goal of `tools/yakin/tasks/keifu-x-inheritance.md`, unchanged, built fresh
from the base spec alone as a V2 run, so it can be compared with its V1/V2
counterpart `keifu-x-inheritance-r3v1`.

## Overrides

- Task id `keifu-x-inheritance-r3v2`; branch
  `claude/yakin-keifu-x-inheritance-r3v2`; run folder
  `tools/yakin/runs/keifu-x-inheritance-r3v2/` — wherever the base names
  `tools/yakin/runs/keifu-x-inheritance/`, read this one.
- Game id / folder / crate take the `r3v2` suffix, so every round's results
  coexist in the workspace: the game is `games/keifu-x-inheritance-r3v2/`, its
  crate (package name) `keifu_x_inheritance_r3v2`. Wherever the base names its
  game — a path `games/keifu-x-inheritance/…`, the crate
  `keifu_x_inheritance`, or a name given to `tools/verify`, `tools/build-web`
  or `tools/serve-web` — use the r3 folder for paths and the r3 crate's
  playable (its default binary, `keifu_x_inheritance_r3v2`) for the tools.
- PR title `[yakin:V2:r3] Keifu X Inheritance: fork Keifu and deepen what a
  family inherits`.

## Variant: V2

The base spec is written for V2, and this run is **V2**: a designer tick
(DESIGNER.md) writes `tools/yakin/runs/keifu-x-inheritance-r3v2/DESIGN.md`
from `tools/yakin/templates/DESIGN.md`, carrying everything the base asks of
its design stage, and releases it; a later worker tick (WORKER.md §3, V2)
builds from it. Neither stage's role changes.

## Fork rule (restated from the base spec)

- **First act of the implement stage, and its own commit:** copy mainline
  `games/keifu` to `games/keifu-x-inheritance-r3v2/` and rename the crate and
  its ids to `keifu_x_inheritance_r3v2` — package and binary name, the
  `tools/verify` / `tools/build-web` name, window title, and any asset or path
  string naming `keifu` that would collide or mislead. That commit changes
  nothing else, and `python3 tools/verify keifu_x_inheritance_r3v2` passes on
  it. The only commits that may precede it on the branch touch nothing but the
  run folder (the claim; the design stage's commits).
- **Mainline `games/keifu` is never touched** — read-only for every stage of
  this task; not one byte of it is written.
- The design stage reads mainline Keifu's docs and source, as the base spec
  lists them. That is base-spec knowledge, not a hygiene violation: the ban
  below covers other yakin runs' output, not the engine's public docs or
  mainline games.

## Comparison hygiene

Do not open, read, or consult any other round's or the paired variant's
branches, PRs, designs, run folders or `games/` output for this idea — not its
V1/V2 counterpart `keifu-x-inheritance-r3v1` (branch
`claude/yakin-keifu-x-inheritance-r3v1`), night one's `keifu-x-inheritance`
([#129](https://github.com/JHorace/jidousha/pull/129)), round two's
`keifu-x-inheritance-r2` (branch `claude/yakin-keifu-x-inheritance-r2`). If
one of them has landed on `main`, its folders are in your clone: leave them
unopened. Work fresh from the base spec alone. The pairing is the experiment;
cribbing voids it. Looking up a PR's URL for the PR body (below) is not
consulting it; opening it is.

Mainline `games/keifu` is not a yakin run's output: reading it is required
(the fork rule above), not a breach.

## The PR

Title `[yakin:V2:r3] Keifu X Inheritance: fork Keifu and deepen what a family
inherits`; body WORKER.md §4's, with these three lines after its `Task:` line:

- `V1/V2 counterpart: keifu-x-inheritance-r3v1`
- `Comparison baselines: #129 (night one); <round two's PR URL, or "round two:
  no PR">` — find round two's URL with WORKER.md §4 step 1's lookup on head
  `claude/yakin-keifu-x-inheritance-r2`
- `At most one PR per idea-family ever merges: this PR, its counterpart and
  every other round's run of keifu-x-inheritance are one family.`

## Decisions

The base's `## Decisions this task adds` table, incorporated whole and
unchanged — it is this task's decision-surface table, and DOCTRINE §6's check
runs on it there.

## Fence

The base's `## Fence`, with its paths renamed: write access to
`games/keifu-x-inheritance-r3v2/**` (new), `Cargo.lock` for the
`keifu_x_inheritance_r3v2` crate's own entry only, and
`tools/yakin/runs/keifu-x-inheritance-r3v2/`. Nothing else — in particular not
`games/keifu-x-inheritance/`, `games/keifu-x-inheritance-r2/`,
`games/keifu-x-inheritance-r3v1/` or mainline `games/keifu/`. No new
dependencies.

## Done when

Every `## Done when` line of `tools/yakin/tasks/keifu-x-inheritance.md`, read
through the overrides above — in particular:

- `tools/yakin/runs/keifu-x-inheritance-r3v2/DESIGN.md` exists, header
  `variant: V2`, carrying what the base asks of its design stage.
- `games/keifu-x-inheritance-r3v2/` exists as a workspace member, and its
  first code commit on the branch is the pure copy-and-rename with its verify
  green.
- `python3 tools/verify keifu` still reports `pass`, and `git diff --stat
  origin/main...` names no path under `games/keifu/`.
- `git diff --stat origin/main...` names only
  `games/keifu-x-inheritance-r3v2/`, `Cargo.lock` (the
  `keifu_x_inheritance_r3v2` entry only) and
  `tools/yakin/runs/keifu-x-inheritance-r3v2/`.
- `python3 tools/verify keifu_x_inheritance_r3v2` reports `pass` in
  `target/verify/keifu_x_inheritance_r3v2.json` with a check for each decision
  row, and `python3 tools/build-web keifu_x_inheritance_r3v2 && python3
  tools/serve-web keifu_x_inheritance_r3v2 --check` passes — in place of the
  base's commands for `keifu-x-inheritance`.
- A PR titled `[yakin:V2:r3] Keifu X Inheritance: fork Keifu and deepen what a
  family inherits` is open with the WORKER.md body and the three lines of "The
  PR" above, its Deviations listing every departure from DESIGN.md.
