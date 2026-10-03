# yakin — the Wednesday burn-down

Unattended Claude Code cloud routines ("ticks") spend the usage week's remainder
(**Wed 01:00 → Thu 05:00 America/Los_Angeles**; ticks stop work at 04:30) building
experimental games and backlog systems — **PRs only**, never merged by a tick.
ADR-0045 is the why.

**Lifecycle:** `queued` (a row in `queue.json` on `main`) → **design** (V2/V3:
a designer tick writes `runs/<id>/DESIGN.md`) → **implement** → **PR**
(`[yakin:<variant>] <title>`) — or `BLOCKED.md` on the branch. A task is
claimed iff branch `claude/yakin-<id>` exists; every push carries
`tools/yakin/runs/<id>/CHECKPOINT.md`. **State lives on branches, never in the
queue:** each tick is a fresh clone that cannot write `main`, so what it pushed
is all the next tick knows. `yakin next --role design|implement` reads it.

**Queue** — `queue.json`, validated by `tools/yakin/yakin check`:
`{"schema": 1, "tasks": [<task>, …]}`, queue order is priority. A task has
exactly: `id` (lowercase words joined by `-`, ≤40) · `title` (≤72, the PR
title) · `kind` `game|system` · `variant` `V1|V2|V3` · `size` `S|M|L` · `spec`
(`tools/yakin/tasks/<id>.md`) · `status` `queued` (the only value: claim state
is not stored here) · optional `window` `burn-down` (default) `|any` (size S
only; runs outside the week — the canary). A spec carries `## Goal`, `## Fence`
(the only paths the task may write) and `## Done when`; `tasks/m0-canary.md` is
the model. **V1**: the worker designs inline and builds; **V2**: the designer
routine designs, the worker builds; **V3**: V2 plus the review routine's
FINDINGS comment. **Stage is derived, never queued:** a V2/V3 task with no
branch, or whose CHECKPOINT says `stage: design`, is at design and only
`next --role design` prints it; once the CHECKPOINT says `implement`, only
`next --role implement` does — as it does every V1 task (V1 has no design stage).

**Owner:** retire a task by deleting its row; re-run one by deleting its
branch. Blocked work has no PR — find it with
`git ls-remote origin 'refs/heads/claude/yakin-*'` and `BLOCKED.md` on the
branch. The routines are configured at claude.ai, not here; the real prompts
are `WORKER.md`, `DESIGNER.md` (both reading the shared `DOCTRINE.md`) and
`REVIEW.md`, so they version and PR like everything else. DESIGN.md's shape is
`templates/DESIGN.md`; its model field is the routine's setting, not proof —
the run page is the authority on what ran.

Worker routine prompt (cron, every 2h through the window):

```
You are a yakin worker tick.
Read tools/yakin/WORKER.md from the cloned repo and follow it exactly.
It is your entire instruction.
```

Designer routine prompt (cron `7 1,3 * * 3` — Wed 01:07 and 03:07):

```
You are a yakin designer tick.
Read tools/yakin/DESIGNER.md from the cloned repo and follow it exactly.
It is your entire instruction.
```

Review routine prompt (GitHub `pull_request.opened`, head branch starts with `claude/yakin`):

```
You are a yakin review tick.
Read tools/yakin/REVIEW.md from the cloned repo and follow it exactly.
It is your entire instruction.
```
