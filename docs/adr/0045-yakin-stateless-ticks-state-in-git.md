# ADR-0045: yakin — stateless scheduled ticks, state in git, PRs only

Status: accepted · 2026-10-01 · supersedes nothing

> **The week's unspent usage is burned down by unattended cloud routines that
> keep every bit of state in git.** A tick is a fresh clone with no memory; a
> task is claimed by its branch existing; progress is a CHECKPOINT.md pushed
> before every risk; output is a pull request the owner merges or doesn't.

## Context

The subscription's usage week resets Thursday 05:00 America/Los_Angeles, and
most weeks end with quota unspent. Claude Code routines can run unattended on
a schedule (or on a GitHub event) on cloud infrastructure, configured by the
owner at claude.ai. Three facts about them shape everything here:

- **Each run is a fresh clone of the default branch.** No session memory, no
  shared sandbox between runs.
- **A run that hits the usage limit is halted where it stands**, and its
  sandbox is unreachable afterwards. Burning the remainder means *expecting*
  to be halted, every week, mid-command.
- **A tick cannot write the default branch** — it pushes `claude/*` branches
  and opens PRs.

So the next tick knows exactly what was pushed, and nothing else.

## Decision

1. **State lives in git, on the task's branch.** A task is claimed iff
   `claude/yakin-<task-id>` exists. `queue.json` on `main` holds only queued
   work (`status: queued` is its only legal value); a claim recorded there
   would be one no tick could write.
2. **A lease, not a lock.** A branch whose last commit is under 90 minutes old
   may have a live tick; ticks push at least hourly. Older, and the next tick
   resumes it from its CHECKPOINT; a third resume becomes a `BLOCKED.md`.
   `tools/yakin/yakin next` is the rule's single implementation, so no tick
   re-derives it.
3. **Checkpoint discipline is a template, not a value.** CHECKPOINT.md has fixed
   fields (stage, tick, resumes, last-green commit, exact next step) and is true
   at every pushed commit; every green gate is a commit and every commit is
   pushed. The push is the write-ahead log.
4. **PRs only.** Ticks never merge, approve, or touch `main`. Engine crates,
   CI, `.claude/` and `CLAUDE.md` are read-only to them; an engine fault is a
   FINDINGS entry. No new dependencies.
5. **The window is local time.** Wednesday 01:00 → Thursday 04:30
   America/Los_Angeles; the half hour before the reset is margin so no tick
   spends the fresh week's quota.
6. **The pipeline is an experiment, V1/V2/V3, tagged per task.** V1 designs and
   builds in one tick lineage. V2 splits design and build across ticks, with
   `DESIGN.md` as the only channel between them. V3 is V2 plus a review routine
   that posts one FINDINGS comment — never an approval. The comparison is what
   the burn-down buys besides the code.
7. **The prompts live in the repo.** The routine forms hold three-line
   pointers to `tools/yakin/WORKER.md` and `REVIEW.md`, so the instructions
   version, diff and review like code.

## Consequences

- A halted tick costs at most the work since its last push, plus a 90-minute
  wait before resumption. Heartbeat pushes are the price of that bound.
- Abandoned or blocked branches accumulate; the owner retires tasks by
  deleting queue rows and re-runs them by deleting branches.
- The run folder (`tools/yakin/runs/<id>/`) lands on `main` with a merged PR;
  that record is also how `next` knows a task is finished.
- The canary (`m0-canary`) is the end-to-end proof, and its cold-build timing
  decides whether the routine environment needs a setup script.

## Alternatives rejected

- **Claim state in `queue.json`.** Ticks cannot write `main`, and a claim file
  on a side branch is a second source of truth beside the branch itself.
- **One long session instead of ticks.** A halt loses the sandbox either way;
  short ticks bound the loss and re-read the queue each time.
- **An agent-approved merge path.** Contradicts agent-practices §2.5; the
  review arm comments only.
