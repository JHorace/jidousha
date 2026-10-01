# yakin — the Wednesday burn-down

Unattended Claude Code cloud routines ("ticks") spend the usage week's remainder
(**Wed 01:00 → Thu 05:00 America/Los_Angeles**; ticks stop work at 04:30) building
experimental games and backlog systems — **PRs only**, never merged by a tick.
ADR-0045 is the why.

**Lifecycle:** `queued` (a row in `queue.json` on `main`) → **claimed** (branch
`claude/yakin-<id>` exists) → **checkpointed** (every push carries
`tools/yakin/runs/<id>/CHECKPOINT.md`) → **PR** (`[yakin:<variant>] <title>`)
— or `BLOCKED.md` on the branch. **State lives on branches, never in the
queue:** each tick is a fresh clone that cannot write `main`, so what it pushed
is all the next tick knows. `tools/yakin/yakin next` reads that state.

**Queue** — `queue.json`, validated by `tools/yakin/yakin check`:
`{"schema": 1, "tasks": [<task>, …]}`, queue order is priority. A task has
exactly: `id` (lowercase words joined by `-`, ≤40) · `title` (≤72, the PR
title) · `kind` `game|system` · `variant` `V1|V2|V3` · `size` `S|M|L` · `spec`
(`tools/yakin/tasks/<id>.md`) · `status` `queued` (the only value: claim state
is not stored here) · optional `window` `burn-down` (default) `|any` (size S
only; runs outside the week — the canary). A spec carries `## Goal`, `## Fence`
(the only paths the task may write) and `## Done when`; `tasks/m0-canary.md` is
the model. **V1** designs and builds in one tick lineage; **V2** splits design
and build across ticks; **V3** is V2 plus the review routine's FINDINGS comment.

**Owner:** retire a task by deleting its row; re-run one by deleting its
branch. Blocked work has no PR — find it with
`git ls-remote origin 'refs/heads/claude/yakin-*'` and `BLOCKED.md` on the
branch. The routines are configured at claude.ai, not here; the real prompts
are `WORKER.md` and `REVIEW.md`, so they version and PR like everything else.

Worker routine prompt (cron, every 2h through the window):

```
You are a yakin worker tick.
Read tools/yakin/WORKER.md from the cloned repo and follow it exactly.
It is your entire instruction.
```

Review routine prompt (GitHub `pull_request.opened`, head branch starts with `claude/yakin`):

```
You are a yakin review tick.
Read tools/yakin/REVIEW.md from the cloned repo and follow it exactly.
It is your entire instruction.
```
