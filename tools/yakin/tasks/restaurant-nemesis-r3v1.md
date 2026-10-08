# restaurant-nemesis-r3v1 — round-three V1 run of `restaurant-nemesis`

Kind: game · Variant: V1 · Size: L · Window: burn-down

Round-three paired run. Follow `tools/yakin/tasks/restaurant-nemesis.md`
exactly, with the overrides below. That spec is this spec: its brief, steps,
decision-surface table and `## Done when` lines bind this task as written,
read through these overrides wherever they name the task, its game, its
branch, its run folder, its variant or its PR.

Round three runs `restaurant-nemesis` twice from the same spec: as V1
(`restaurant-nemesis-r3v1` — one worker tick lineage designs inline and
implements) and as V2 (`restaurant-nemesis-r3v2` — a designer tick writes
DESIGN.md per the template, and a later worker tick builds from it). The two
runs are a pair, and comparing them is the experiment.

## Goal

The goal of `tools/yakin/tasks/restaurant-nemesis.md`, unchanged, built fresh
from the base spec alone as a V1 run, so it can be compared with its V1/V2
counterpart `restaurant-nemesis-r3v2`.

## Overrides

- Task id `restaurant-nemesis-r3v1`; branch
  `claude/yakin-restaurant-nemesis-r3v1`; run folder
  `tools/yakin/runs/restaurant-nemesis-r3v1/` — wherever the base names
  `tools/yakin/runs/restaurant-nemesis/`, read this one.
- Game id / folder / crate take the `r3v1` suffix, so every round's results
  coexist in the workspace: the game is `games/restaurant-nemesis-r3v1/`, its
  crate (package name) `restaurant_nemesis_r3v1`. Wherever the base names its
  game — a path `games/restaurant-nemesis/…`, the crate `restaurant_nemesis`,
  or a name given to `tools/verify`, `tools/build-web` or `tools/serve-web` —
  use the r3 folder for paths and the r3 crate's playable (its default binary,
  `restaurant_nemesis_r3v1`) for the tools.
- PR title `[yakin:V1:r3] Restaurant Nemesis: bad service breeds themed
  nemeses who come back`.

## Variant: V1

The base spec is variant-neutral. This run is **V1**: one worker tick lineage
designs inline and implements (WORKER.md §3, V1). Its inline design step
writes the base's §"The design" —
`tools/yakin/runs/restaurant-nemesis-r3v1/DESIGN.md` from
`tools/yakin/templates/DESIGN.md`, header `variant: V1`, every number the base
lists — before any code, and sets `stage: implement` in the same commit.
Wherever the base says "the designer" or cites DESIGNER.md (its "Size" rule),
read: this worker, at its inline design step. The open calls the template asks
for are yours to decide and to record there.

## Comparison hygiene

Do not open, read, or consult any other round's or the paired variant's
branches, PRs, designs, run folders or `games/` output for this idea — not its
V1/V2 counterpart `restaurant-nemesis-r3v2` (branch
`claude/yakin-restaurant-nemesis-r3v2`). If one of them has landed on `main`,
its folders are in your clone: leave them unopened. Work fresh from the base
spec alone. The pairing is the experiment; cribbing voids it. Looking up a
PR's URL for the PR body (below) is not consulting it; opening it is.

## The PR

Title `[yakin:V1:r3] Restaurant Nemesis: bad service breeds themed nemeses who
come back`; body WORKER.md §4's, with these three lines after its `Task:`
line:

- `V1/V2 counterpart: restaurant-nemesis-r3v2`
- `Comparison baselines: none — round three is this idea's first run`
- `At most one PR per idea-family ever merges: this PR, its counterpart and
  every other round's run of restaurant-nemesis are one family.`

## Decisions

The base's `## Decisions this task adds` table, incorporated whole and
unchanged — it is this task's decision-surface table, and DOCTRINE §6's check
runs on it there.

## Fence

The base's `## Fence`, with its paths renamed: write access to
`games/restaurant-nemesis-r3v1/**` (new), `Cargo.lock` for the
`restaurant_nemesis_r3v1` crate's own entry only, and
`tools/yakin/runs/restaurant-nemesis-r3v1/`. Nothing else — in particular not
`games/restaurant-nemesis/` or `games/restaurant-nemesis-r3v2/`. No new
dependencies.

## Done when

Every `## Done when` line of `tools/yakin/tasks/restaurant-nemesis.md`, read
through the overrides above — in particular:

- `tools/yakin/runs/restaurant-nemesis-r3v1/DESIGN.md` exists, header
  `variant: V1`, and fixes every number the base's §"The design" lists.
- `git diff --stat origin/main...` names only
  `games/restaurant-nemesis-r3v1/`, `Cargo.lock` (the
  `restaurant_nemesis_r3v1` entry only) and
  `tools/yakin/runs/restaurant-nemesis-r3v1/`.
- `python3 tools/verify restaurant_nemesis_r3v1` reports `pass` in
  `target/verify/restaurant_nemesis_r3v1.json` with a check for each decision
  row, and `python3 tools/build-web restaurant_nemesis_r3v1 && python3
  tools/serve-web restaurant_nemesis_r3v1 --check` passes — in place of the
  base's commands for `restaurant-nemesis`.
- A PR titled `[yakin:V1:r3] Restaurant Nemesis: bad service breeds themed
  nemeses who come back` is open with the WORKER.md body and the three lines
  of "The PR" above, its Deviations listing every departure from DESIGN.md.
