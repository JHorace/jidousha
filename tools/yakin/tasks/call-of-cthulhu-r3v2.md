# call-of-cthulhu-r3v2 — round-three V2 run of `call-of-cthulhu`

Kind: game · Variant: V2 · Size: M · Window: burn-down

Round-three paired run. Follow `tools/yakin/tasks/call-of-cthulhu.md` exactly,
with the overrides below. That spec is this spec: its brief, steps,
decision-surface table and `## Done when` lines bind this task as written,
read through these overrides wherever they name the task, its game, its
branch, its run folder, its variant or its PR.

Round three runs `call-of-cthulhu` twice from the same spec: as V1
(`call-of-cthulhu-r3v1` — one worker tick lineage designs inline and
implements) and as V2 (`call-of-cthulhu-r3v2` — a designer tick writes
DESIGN.md per the template, and a later worker tick builds from it). The two
runs are a pair, and comparing them is the experiment. Earlier rounds ran this
idea as night one's `call-of-cthulhu`
([#127](https://github.com/JHorace/jidousha/pull/127)), round two's
`call-of-cthulhu-r2` (branch `claude/yakin-call-of-cthulhu-r2`); those are the
comparison baselines, never sources.

## Goal

The goal of `tools/yakin/tasks/call-of-cthulhu.md`, unchanged, built fresh
from the base spec alone as a V2 run, so it can be compared with its V1/V2
counterpart `call-of-cthulhu-r3v1`.

## Overrides

- Task id `call-of-cthulhu-r3v2`; branch `claude/yakin-call-of-cthulhu-r3v2`;
  run folder `tools/yakin/runs/call-of-cthulhu-r3v2/` — wherever the base
  names `tools/yakin/runs/call-of-cthulhu/`, read this one.
- Game id / folder / crate take the `r3v2` suffix, so every round's results
  coexist in the workspace: the game is `games/call-of-cthulhu-r3v2/`, its
  crate (package name) `call_of_cthulhu_r3v2`. Wherever the base names its
  game — a path `games/call-of-cthulhu/…`, the crate `call_of_cthulhu`, or a
  name given to `tools/verify`, `tools/build-web` or `tools/serve-web` — use
  the r3 folder for paths and the r3 crate's playable (its default binary,
  `call_of_cthulhu_r3v2`) for the tools.
- PR title `[yakin:V2:r3] Call of Cthulhu: hang up on eldritch callers before
  sanity runs out`.

## Variant: V2

The base spec is written for V1 — its brief's first bullet ("V1: design
inline, then implement") and its first `## Done when` line. This run is
**V2**, and these replace them:

- **Design stage** (DESIGNER.md): a designer tick writes
  `tools/yakin/runs/call-of-cthulhu-r3v2/DESIGN.md` from
  `tools/yakin/templates/DESIGN.md`. Besides the template's sections it
  carries everything the base's design note must hold — the beings, the call
  structure, the day loop and the sanity arithmetic. The designer writes
  nothing under `games/` (DESIGNER.md §2).
- **Implement stage** (WORKER.md §3, V2): a later worker tick builds from that
  DESIGN.md. There is no second design note: the game folder carries no
  DESIGN.md, and wherever the base says `games/call-of-cthulhu/DESIGN.md`,
  read `tools/yakin/runs/call-of-cthulhu-r3v2/DESIGN.md`.

## Comparison hygiene

Do not open, read, or consult any other round's or the paired variant's
branches, PRs, designs, run folders or `games/` output for this idea — not its
V1/V2 counterpart `call-of-cthulhu-r3v1` (branch
`claude/yakin-call-of-cthulhu-r3v1`), night one's `call-of-cthulhu`
([#127](https://github.com/JHorace/jidousha/pull/127)), round two's
`call-of-cthulhu-r2` (branch `claude/yakin-call-of-cthulhu-r2`). If one of
them has landed on `main`, its folders are in your clone: leave them unopened.
Work fresh from the base spec alone. The pairing is the experiment; cribbing
voids it. Looking up a PR's URL for the PR body (below) is not consulting it;
opening it is.

## The PR

Title `[yakin:V2:r3] Call of Cthulhu: hang up on eldritch callers before
sanity runs out`; body WORKER.md §4's, with these three lines after its
`Task:` line:

- `V1/V2 counterpart: call-of-cthulhu-r3v1`
- `Comparison baselines: #127 (night one); <round two's PR URL, or "round two:
  no PR">` — find round two's URL with WORKER.md §4 step 1's lookup on head
  `claude/yakin-call-of-cthulhu-r2`
- `At most one PR per idea-family ever merges: this PR, its counterpart and
  every other round's run of call-of-cthulhu are one family.`

## Decisions

The base's `## Decisions this task adds` table, incorporated whole and
unchanged — it is this task's decision-surface table, and DOCTRINE §6's check
runs on it there.

## Fence

The base's `## Fence`, with its paths renamed: write access to
`games/call-of-cthulhu-r3v2/**` (new), `Cargo.lock` for the
`call_of_cthulhu_r3v2` crate's own entry only, and
`tools/yakin/runs/call-of-cthulhu-r3v2/`. Nothing else — in particular not
`games/call-of-cthulhu/`, `games/call-of-cthulhu-r2/` or
`games/call-of-cthulhu-r3v1/`. No new dependencies.

## Done when

Every `## Done when` line of `tools/yakin/tasks/call-of-cthulhu.md`, read
through the overrides above — in particular:

- In place of the base's first line:
  `tools/yakin/runs/call-of-cthulhu-r3v2/DESIGN.md` exists, written by the
  design stage, and names the beings, the call structure, the day loop and the
  sanity arithmetic.
- `git diff --stat origin/main...` names only `games/call-of-cthulhu-r3v2/`,
  `Cargo.lock` (the `call_of_cthulhu_r3v2` entry only) and
  `tools/yakin/runs/call-of-cthulhu-r3v2/`.
- `python3 tools/verify call_of_cthulhu_r3v2` reports `pass` in
  `target/verify/call_of_cthulhu_r3v2.json` with a check for each decision
  row, and `python3 tools/build-web call_of_cthulhu_r3v2 && python3
  tools/serve-web call_of_cthulhu_r3v2 --check` passes — in place of the
  base's commands for `call-of-cthulhu`.
- A PR titled `[yakin:V2:r3] Call of Cthulhu: hang up on eldritch callers
  before sanity runs out` is open with the WORKER.md body and the three lines
  of "The PR" above, its Deviations listing every departure from DESIGN.md.
