# m0-canary — prove clone → build → gates → push → PR, touching nothing real

Kind: system · Variant: V1 · Size: S · Window: any (the owner runs it with
**Run now**, outside the burn-down week)

## Goal

Prove the yakin pipeline end to end on a task whose only product is a record:
a fresh routine clone builds, runs the full gate, pushes its branch, and opens
a PR. The record it writes also answers one owner question — how long a cold
`tools/test` takes in the routine environment — which decides whether the
environment's setup script stays empty.

## Fence

- **Grants write access to exactly one path beyond the run folder:**
  `tools/yakin/canary.md` (new). The run folder
  `tools/yakin/runs/m0-canary/` holds CHECKPOINT.md and DESIGN.md as usual.
- Grants nothing else: the workspace manifest, `Cargo.lock`, every crate and
  game stay untouched. A diff outside those two paths is a defect.
- No new dependencies. No source edits to make a gate pass: a red gate is
  recorded in `canary.md` as the result, not fixed — this spec relaxes
  WORKER.md's "full gate green" for this task, and only that.

## Steps

1. Record `started:` — the output of
   `TZ=America/Los_Angeles date '+%Y-%m-%d %H:%M:%S %Z'` — as the tick's
   first act on this task.
2. Record, verbatim, the output of each of:
   `rustc --version`, `cargo --version`, `python3 --version`, `git --version`,
   `TZ=America/Los_Angeles date '+%a %Y-%m-%d %H:%M %Z'`, `nproc`,
   `df -h . | tail -1`, and the last line of `python3 tools/doctor`.
3. Push the CHECKPOINT (stage `implement`, next step: "time tools/test"),
   then run the full suite timed, from a clean build directory so the
   number is a cold build. It outlasts the 10-minute foreground limit, so it
   runs in the background (WORKER.md §5's shape; cargo leaves `target/yakin/`
   alone):

   ```
   rm -rf target && mkdir -p target/yakin && nohup sh -c 'start=$(date +%s); python3 tools/test > target/yakin/test.log 2>&1; echo "exit=$? seconds=$(( $(date +%s) - start ))" >> target/yakin/test.log' >/dev/null 2>&1 &
   tail -2 target/yakin/test.log   # repeat every few minutes until the exit= line appears
   ```

4. From `target/verify/report.json` (ground truth, not the terminal) record
   `status`, the pass/fail/ignored counts, and each phase's name, status and
   `duration_s`. The `build` phase's `duration_s` is the cold-build cost;
   say it in one line beside the wall time.
5. Write `tools/yakin/canary.md` (shape below), commit, push, open the PR.

## canary.md shape

```
# yakin canary — <date, America/Los_Angeles>

Tick: branch claude/yakin-m0-canary · resumes <N> · started <step 1> · finished <same command, now>

## Toolchain
<the step-2 outputs, one per line>

## tools/test (cold)
exit: <n> · wall: <seconds>s · report status: <status>
counts: <passed> passed · <failed> failed · <ignored> ignored
cold build (the `build` phase): <seconds>s

| phase | status | seconds |
|---|---|---|
<one row per phase from report.json>

## Verdict
<one line: does a cold build fit comfortably inside one 2h tick, and does the
setup script need to pre-warm anything? Evidence, not opinion.>
```

## Done when

- `tools/yakin/canary.md` exists on the branch in the shape above, with every
  number taken from a command run in this tick.
- `git diff --stat origin/main...` names only `tools/yakin/canary.md` and
  `tools/yakin/runs/m0-canary/`.
- A PR titled `[yakin:V1] M0 canary: record toolchain and a full tools/test timing`
  is open with the WORKER.md body, its gate line quoting `report.json`'s status.
- A red `tools/test` does not stop this task: it is recorded, the PR's Gates
  line shows it, and its Deviations line says it was recorded, not fixed.
