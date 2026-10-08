# stack-card-game-r3v2 — round-three V2 run of `stack-card-game`

Kind: game · Variant: V2 · Size: M · Window: burn-down

Round-three paired run. Follow `tools/yakin/tasks/stack-card-game.md` exactly,
with the overrides below. That spec is this spec: its brief, steps,
decision-surface table and `## Done when` lines bind this task as written,
read through these overrides wherever they name the task, its game, its
branch, its run folder, its variant or its PR.

Round three runs `stack-card-game` twice from the same spec: as V1
(`stack-card-game-r3v1` — one worker tick lineage designs inline and
implements) and as V2 (`stack-card-game-r3v2` — a designer tick writes
DESIGN.md per the template, and a later worker tick builds from it). The two
runs are a pair, and comparing them is the experiment. Earlier rounds ran this
idea as night one's `stack-card-game`
([#125](https://github.com/JHorace/jidousha/pull/125)), round two's
`stack-card-game-r2` (branch `claude/yakin-stack-card-game-r2`); those are the
comparison baselines, never sources.

## Goal

The goal of `tools/yakin/tasks/stack-card-game.md`, unchanged, built fresh
from the base spec alone as a V2 run, so it can be compared with its V1/V2
counterpart `stack-card-game-r3v1`.

## Overrides

- Task id `stack-card-game-r3v2`; branch `claude/yakin-stack-card-game-r3v2`;
  run folder `tools/yakin/runs/stack-card-game-r3v2/` — wherever the base
  names `tools/yakin/runs/stack-card-game/`, read this one.
- Game id / folder / crate take the `r3v2` suffix, so every round's results
  coexist in the workspace: the game is `games/stack-card-game-r3v2/`, its
  crate (package name) `stack_card_game_r3v2`. Wherever the base names its
  game — a path `games/stack-card-game/…`, the crate `stack_card_game`, or a
  name given to `tools/verify`, `tools/build-web` or `tools/serve-web` — use
  the r3 folder for paths and the r3 crate's playable (its default binary,
  `stack_card_game_r3v2`) for the tools.
- PR title `[yakin:V2:r3] Stack card game: a two-player duel fought over the
  stack`.

## Variant: V2

The base spec is written for V1 — its brief's first bullet ("V1: design
inline, then implement") and its first `## Done when` line. This run is
**V2**, and these replace them:

- **Design stage** (DESIGNER.md): a designer tick writes
  `tools/yakin/runs/stack-card-game-r3v2/DESIGN.md` from
  `tools/yakin/templates/DESIGN.md`. Besides the template's sections it
  carries everything the base's design note must hold — the rules, the card
  pool and why stack manipulation dominates. The designer writes nothing under
  `games/` (DESIGNER.md §2).
- **Implement stage** (WORKER.md §3, V2): a later worker tick builds from that
  DESIGN.md. There is no second design note: the game folder carries no
  DESIGN.md, and wherever the base says `games/stack-card-game/DESIGN.md`,
  read `tools/yakin/runs/stack-card-game-r3v2/DESIGN.md`.

## Comparison hygiene

Do not open, read, or consult any other round's or the paired variant's
branches, PRs, designs, run folders or `games/` output for this idea — not its
V1/V2 counterpart `stack-card-game-r3v1` (branch
`claude/yakin-stack-card-game-r3v1`), night one's `stack-card-game`
([#125](https://github.com/JHorace/jidousha/pull/125)), round two's
`stack-card-game-r2` (branch `claude/yakin-stack-card-game-r2`). If one of
them has landed on `main`, its folders are in your clone: leave them unopened.
Work fresh from the base spec alone. The pairing is the experiment; cribbing
voids it. Looking up a PR's URL for the PR body (below) is not consulting it;
opening it is.

## The PR

Title `[yakin:V2:r3] Stack card game: a two-player duel fought over the
stack`; body WORKER.md §4's, with these three lines after its `Task:` line:

- `V1/V2 counterpart: stack-card-game-r3v1`
- `Comparison baselines: #125 (night one); <round two's PR URL, or "round two:
  no PR">` — find round two's URL with WORKER.md §4 step 1's lookup on head
  `claude/yakin-stack-card-game-r2`
- `At most one PR per idea-family ever merges: this PR, its counterpart and
  every other round's run of stack-card-game are one family.`

## Decisions

The base's `## Decisions this task adds` table, incorporated whole and
unchanged — it is this task's decision-surface table, and DOCTRINE §6's check
runs on it there.

## Fence

The base's `## Fence`, with its paths renamed: write access to
`games/stack-card-game-r3v2/**` (new), `Cargo.lock` for the
`stack_card_game_r3v2` crate's own entry only, and
`tools/yakin/runs/stack-card-game-r3v2/`. Nothing else — in particular not
`games/stack-card-game/`, `games/stack-card-game-r2/` or
`games/stack-card-game-r3v1/`. No new dependencies.

## Done when

Every `## Done when` line of `tools/yakin/tasks/stack-card-game.md`, read
through the overrides above — in particular:

- In place of the base's first line:
  `tools/yakin/runs/stack-card-game-r3v2/DESIGN.md` exists, written by the
  design stage, and names the rules, the card pool and why stack manipulation
  dominates.
- `git diff --stat origin/main...` names only `games/stack-card-game-r3v2/`,
  `Cargo.lock` (the `stack_card_game_r3v2` entry only) and
  `tools/yakin/runs/stack-card-game-r3v2/`.
- `python3 tools/verify stack_card_game_r3v2` reports `pass` in
  `target/verify/stack_card_game_r3v2.json` with a check for each decision
  row, and `python3 tools/build-web stack_card_game_r3v2 && python3
  tools/serve-web stack_card_game_r3v2 --check` passes — in place of the
  base's commands for `stack-card-game`.
- A PR titled `[yakin:V2:r3] Stack card game: a two-player duel fought over
  the stack` is open with the WORKER.md body and the three lines of "The PR"
  above, its Deviations listing every departure from DESIGN.md.
