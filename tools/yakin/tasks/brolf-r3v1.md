# brolf-r3v1 — round-three V1 run of `brolf`

Kind: game · Variant: V1 · Size: M · Window: burn-down

Round-three paired run. Follow `tools/yakin/tasks/brolf.md` exactly, with the
overrides below. That spec is this spec: its brief, steps, decision-surface
table and `## Done when` lines bind this task as written, read through these
overrides wherever they name the task, its game, its branch, its run folder,
its variant or its PR.

Round three runs `brolf` twice from the same spec: as V1 (`brolf-r3v1` — one
worker tick lineage designs inline and implements) and as V2 (`brolf-r3v2` — a
designer tick writes DESIGN.md per the template, and a later worker tick
builds from it). The two runs are a pair, and comparing them is the
experiment. Earlier rounds ran this idea as night one's `brolf`
([#128](https://github.com/JHorace/jidousha/pull/128)), round two's `brolf-r2`
(branch `claude/yakin-brolf-r2`); those are the comparison baselines, never
sources.

## Goal

The goal of `tools/yakin/tasks/brolf.md`, unchanged, built fresh from the base
spec alone as a V1 run, so it can be compared with its V1/V2 counterpart
`brolf-r3v2`.

## Overrides

- Task id `brolf-r3v1`; branch `claude/yakin-brolf-r3v1`; run folder
  `tools/yakin/runs/brolf-r3v1/` — wherever the base names
  `tools/yakin/runs/brolf/`, read this one.
- Game id / folder / crate take the `r3v1` suffix, so every round's results
  coexist in the workspace: the game is `games/brolf-r3v1/`, its crate
  (package name) `brolf_r3v1`. Wherever the base names its game — a path
  `games/brolf/…`, the crate `brolf`, or a name given to `tools/verify`,
  `tools/build-web` or `tools/serve-web` — use the r3 folder for paths and the
  r3 crate's playable (its default binary, `brolf_r3v1`) for the tools.
- PR title `[yakin:V1:r3] Brolf: battle-royale golf prototype against NPC
  opponents`.

## Variant: V1

The base spec is written for V2 — a designer routine designs, a worker builds.
This run is **V1**, so one worker tick lineage does both:

- **Inline design** (WORKER.md §3, V1): before any code, the worker writes
  `tools/yakin/runs/brolf-r3v1/DESIGN.md` from
  `tools/yakin/templates/DESIGN.md` (header `variant: V1`), and it carries
  everything the base asks of its design stage — every call its brief leaves
  to the design (the shrink schedule, the grace period and how both are shown;
  the contact rules; the small equipment set; the win conditions, what
  extraction is, what is kept and why clubbing or ball-striking pays), and its
  cuts to Non-goals (the brief's scope line is the floor, not a wish list).
  Set `stage: implement` in the same commit and keep going.
- Wherever the base says "the designer", "the design stage" or cites
  DESIGNER.md, read: this worker, at its inline design step. DESIGNER.md §3's
  "Size" rule still binds the design: built and through the full gate by
  Thursday 04:30. The open calls the template asks for are yours to decide;
  record each decision in DESIGN.md.
- The base's "Deviations list every departure from DESIGN.md" still holds —
  the departures from your own inline design.

## Comparison hygiene

Do not open, read, or consult any other round's or the paired variant's
branches, PRs, designs, run folders or `games/` output for this idea — not its
V1/V2 counterpart `brolf-r3v2` (branch `claude/yakin-brolf-r3v2`), night one's
`brolf` ([#128](https://github.com/JHorace/jidousha/pull/128)), round two's
`brolf-r2` (branch `claude/yakin-brolf-r2`). If one of them has landed on
`main`, its folders are in your clone: leave them unopened. Work fresh from
the base spec alone. The pairing is the experiment; cribbing voids it. Looking
up a PR's URL for the PR body (below) is not consulting it; opening it is.

## The PR

Title `[yakin:V1:r3] Brolf: battle-royale golf prototype against NPC
opponents`; body WORKER.md §4's, with these three lines after its `Task:`
line:

- `V1/V2 counterpart: brolf-r3v2`
- `Comparison baselines: #128 (night one); <round two's PR URL, or "round two:
  no PR">` — find round two's URL with WORKER.md §4 step 1's lookup on head
  `claude/yakin-brolf-r2`
- `At most one PR per idea-family ever merges: this PR, its counterpart and
  every other round's run of brolf are one family.`

## Decisions

The base's `## Decisions this task adds` table, incorporated whole and
unchanged — it is this task's decision-surface table, and DOCTRINE §6's check
runs on it there.

## Fence

The base's `## Fence`, with its paths renamed: write access to
`games/brolf-r3v1/**` (new), `Cargo.lock` for the `brolf_r3v1` crate's own
entry only, and `tools/yakin/runs/brolf-r3v1/`. Nothing else — in particular
not `games/brolf/`, `games/brolf-r2/` or `games/brolf-r3v2/`. No new
dependencies.

## Done when

Every `## Done when` line of `tools/yakin/tasks/brolf.md`, read through the
overrides above — in particular:

- `tools/yakin/runs/brolf-r3v1/DESIGN.md` exists, header `variant: V1`,
  carrying what the base asks of its design stage.
- `git diff --stat origin/main...` names only `games/brolf-r3v1/`,
  `Cargo.lock` (the `brolf_r3v1` entry only) and
  `tools/yakin/runs/brolf-r3v1/`.
- `python3 tools/verify brolf_r3v1` reports `pass` in
  `target/verify/brolf_r3v1.json` with a check for each decision row, and
  `python3 tools/build-web brolf_r3v1 && python3 tools/serve-web brolf_r3v1
  --check` passes — in place of the base's commands for `brolf`.
- A PR titled `[yakin:V1:r3] Brolf: battle-royale golf prototype against NPC
  opponents` is open with the WORKER.md body and the three lines of "The PR"
  above, its Deviations listing every departure from DESIGN.md.
