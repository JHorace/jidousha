# yakin worker tick — the implement role

You are one **yakin worker tick**. **Read `tools/yakin/DOCTRINE.md` next, whole,
before anything else** — it is the law every yakin tick shares (startup, the
lease, the CHECKPOINT, BLOCKED, the deadline, the fences, your last lines), and
this file only adds what the implement stage does. This file plus DOCTRINE.md
is your entire instruction; where they cite a section, `§n` is DOCTRINE's and
`WORKER.md §n` is this file's.

## 1. Your role: `implement`

Your role is **`implement`**: DOCTRINE §1's picker line is, for you,

```
python3 tools/yakin/yakin next --role implement
```

It prints only implement work — a V1 task (design inline, §3 below), or a
V2/V3 task whose branch's CHECKPOINT says `stage: implement`. It never prints a
V2/V3 task at the design stage: that is the designer routine's
(`tools/yakin/DESIGNER.md`), and a worker on it would race the designer on one
branch. Act only on what it prints (DOCTRINE §1).

## 2. Work discipline and the gates — these exact commands

**Fast gate** — after each coherent code edit, and before every commit of code:

```
cargo fmt --all --check
cargo check --workspace
cargo check --workspace --target wasm32-unknown-unknown
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**Full gate** — before the PR, and whenever you want a new `gates-green-at`:

```
python3 tools/doctor               # last line must be ENV_OK
python3 tools/test                 # verdict = "status" in target/verify/report.json; must be "pass"
python3 tools/check-claude-md
python3 tools/yakin/yakin check
```

`tools/test` took 7 minutes in a cloud session on 2026-10-01 and takes longer
cold, so run it — and any command that may pass 10 minutes, such as a cold
`cargo clippy` — in the background and poll; its report file, not the
terminal, is the verdict:

```
mkdir -p target/yakin && nohup sh -c 'python3 tools/test > target/yakin/test.log 2>&1; echo "exit=$?" >> target/yakin/test.log' >/dev/null 2>&1 &
tail -2 target/yakin/test.log      # repeat every few minutes until a line reads exit=<n>
```

(For another command, substitute it and a log name: the shape is the same.)

`tools/test` already runs every game's `tools/verify`, `tools/check-assets`,
`tools/check-game-deps`, `tools/check-api-coverage` and
`tools/gen-api-doc --check`. **Game tasks** also run, for their game `<name>`:

```
python3 tools/verify <name>        # verdict = "status" in target/verify/<name>.json; must be "pass"
python3 tools/build-web <name> && python3 tools/serve-web <name> --check
```

Commit and push per DOCTRINE §5.

**When a gate fails** — CLAUDE.md's protocol: a plain compile error in code you
just wrote, debug it. Anything else, `python3 tools/doctor` first and obey it
(`ENV_OK` your code · `ENV_FIXABLE: <cmd>` run exactly that · `ENV_BROKEN` →
DOCTRINE §7). The same command failing the same way twice after a fix attempt
→ DOCTRINE §7. A command killed by the 10-minute foreground limit has not
failed; rerun it in the background. Never `#[ignore]`, delete or weaken a test to go green.

## 3. Stages and variants

- **V1** — one tick lineage, design inline. The design stage writes
  `<R>/DESIGN.md`: what will be built, the files it will touch (all inside the
  spec's Fence), the order, and how each `## Done when` line will be checked.
  Design touches no code, so it needs no gate. DESIGN.md is working notes; set
  `stage: implement` in the same commit, push, keep going.
- **V2 / V3** — you take the task at `stage: implement`, after a designer tick
  wrote `<R>/DESIGN.md` from `tools/yakin/templates/DESIGN.md`. Read it whole
  with the spec. **DESIGN.md is authoritative**: build what it says, keep its
  "Decisions already made" as made, and decide its "Open calls" yourself.
  Where it and the spec disagree the spec wins, and the conflict is data: a
  FINDINGS entry attributed to the design stage. Every departure from
  DESIGN.md — the spec winning included — is one line in the PR body's
  Deviations, as is each line of the designer's CHECKPOINT Deviations.
  A design that **misled** you — it said something about the engine, the
  spec or the time the build needs that was not true — earns a FINDINGS entry
  (DOCTRINE §9) naming what it said, what was true, and what it cost; that
  signal feeds the variant evaluation, so file it even when the fix was cheap.
  The designer's own findings, if any, are in `<R>/FINDINGS.md`. On a game
  task, move each entry into `games/<name>/FINDINGS.md` noting its origin
  (`design stage, <R>/FINDINGS.md`) — the game's ledger stays canonical; a
  system task keeps them where they are. Count them in the PR's Findings line.
  V3 is identical on the worker side — the review routine comments on its PR
  by itself.

Game tasks follow DOCTRINE §6 at both stages, and §5 below when they need art.

## 4. Done — the PR

**Done** — when every `## Done when` line holds and the full gate is green (or
red and the spec says to record it):

1. Look for an existing PR:
   `gh api 'repos/JHorace/jidousha/pulls?head=JHorace:<B>&state=all' --jq '.[].html_url'`.
   One exists → step 3.
2. Open it: base `main`, head `<B>`, title `[yakin:<variant>] <queue title>`,
   body below. Use the GitHub MCP tool `create_pull_request` (deferred? load it
   with ToolSearch `select:mcp__github__create_pull_request`). If that tool is
   absent, write the body to `target/yakin/pr-body.md` and run
   `gh api repos/JHorace/jidousha/pulls -f title='<title>' -f head=<B> -f base=main -F body=@target/yakin/pr-body.md`.
   **Not `gh pr create`**: it uses GraphQL, which these sessions refuse (HTTP 403).
3. Set `stage: done`, `tick: released`; commit; push. **Never merge.**

```
Scope: yakin task <id>, PR only — <one line from the spec's Goal>.
Task: <id> · variant <V> · kind <k> · size <s> · spec tools/yakin/tasks/<id>.md
Resumes: <n>
Gates at <sha>: doctor <verdict> · tools/test <status> (<p> passed, <f> failed, <i> ignored) · fast gate <clean|what failed|n/a — no code changed> · verify <name> <status> (game tasks)
Design: <V1: "inline" · V2/V3: <R>/DESIGN.md, designer model as configured: <its header's value, quoted> (configured, not verified — the run page is the authority)>
Deviations:
- <one line each, or "none"; V2/V3: every departure from DESIGN.md, one line each>
Findings: <n> in <path> — or "0 findings: <why>"
Assets created: <game tasks: one line each, `<path>` — <what it is> (WORKER.md §5), or "none">
Owner actions: <game tasks: make-game §E's five lines · system tasks: "none" or the list>
```

## 5. Assets — game tasks

The depot supplies availability, not curation — and it is currently thin.
When a game task needs an asset (sprite, tile, icon, sound) and nothing
appropriate exists, **create one** rather than shipping a placeholder or
stalling:

- **Generate programmatically where feasible** — a script run in-session
  (Python/PIL and similar are available on the VM). Commit the output under
  the game's asset root per convention (`games/<name>/assets/`, ADR-0040);
  committing the generator script beside it is encouraged — a regenerable
  asset is an editable asset. Both stay inside the spec's Fence.
- **Simple, legible, stylistically consistent within the game.** Functional
  beats beautiful; a readable colored shape with intent is the bar, not art.
- **Never download or copy third-party assets from the network** —
  provenance and licensing cannot be reviewed unattended. The depot and your
  own generation are the only sources; no spec widens this.
- **Time-boxed in service of the task:** gameplay and gates come first. If
  art is eating the window, ship the honest placeholder, note it in the PR
  (Deviations), and move on.
- **List every created asset in the PR body** (§4's `Assets created:` line —
  path and a one-line description) so the owner can skim the night's art
  debt.
