# ADR-0047: the mutation round is an engine tool — `tools/mutate`, for adopted games

Status: accepted · 2026-10-04 · supersedes nothing · **takes the decision
e0-findings F-140 deferred (citing F-137's streak), for adopted games only**

> **A mutation round on a game under `games/` is `tools/mutate <game>
> <list>...`.** The fault lists stay with the game; the harness, its two hard
> errors and its cost model are the engine's. Worktrees persist so a round
> stops paying a cold build per worker; `--fast` and `--changed-since` make a
> session-time round cheaper without changing what any fault scores. E0 is
> untouched: its authors still write their own harness.

## Context

A mutation round injects one-line faults into a game and demands the tests or
`--verify` notice each. The technique is in `docs/api/jidousha-testing.md`;
the instrument has been written at least five times and committed once. E0
runs 7, 8, 9 and 11 each built a harness in scratch and discarded it
(e0-findings F-123, F-140); keifu sessions 1 and 2 built one and lost it
(keifu G-044); session 3 wrote `games/keifu/mutants/mutate.py` and committed
it. The shape is game-independent — only the fault list belongs to a game.

e0-findings deliberately declined to build `tools/mutate` (the paragraph sits
in F-140 and argues from F-137's streak), for two reasons, and asked that the
decision be taken "between runs and on purpose": handing an E0 author the
instrument changes what the run measures, and the hazard a tool would remove
was one run's word.

The cost is now measured rather than reported. Keifu session 9 reran all
eight lists — 856 faults — in about 2h50m on the cloud sandbox's four cores;
session 10's round of nine lists passed six hours. The owner reproduced the
w9 list (120 faults) on an 8-core / 16-thread desktop on 2026-10-04: 19m00s
wall at 8 workers (144 core-min), 18m24s at 16 workers (228 core-min).
Solving the two runs: **~0.7 core-minutes per fault** (the sandbox's rate is
~0.8), **~7 core-minutes per cold build of the dependency graph**, which every
worktree pays every round because the harness removes its worktrees at the
end, and **SMT worth ~0** on this load. Per-fault work is single-thread bound
(one incremental build, `cargo test -p`, `--verify`, in series); throughput is
linear in physical cores; the round grows linearly with the port's length
because every session reruns every prior list.

## Decision

1. **`tools/mutate` is the one mutation harness for adopted games.** Stdlib
   Python, extensionless, beside `tools/test` and `tools/verify`;
   game-agnostic (`tools/mutate <game> <list>...`). Per fault it builds
   `-p <package>` (tests, then the binary), runs `cargo test -p <package>`,
   and runs the game's verify **the way `tools/test` does — through
   `tools/verify`'s own command and judgement**, loaded from that file rather
   than restated, so "verified" means one thing in both places. It carries
   the testing document's two hard errors as hard errors: a find that matches
   other than once stops the round before anything is written; a mutation
   that does not build is NOT BUILT and never noticed. Files are restored
   from the bytes read; every worktree is checked clean after; a check that
   does not end in ten minutes is a noticed fault, and its whole process
   group is killed rather than left running. Several lists in one call are
   one round, labels prefixed by the list's stem (`w9:C1`), which is what
   sessions 8–10 did by hand.
2. **Fault lists stay with the game**, in `games/<id>/mutants/*.txt`, in the
   format `mutate.py` established (label / `@` file / `-` find / `+` replace,
   `⏎` for a line break, `#!` for a header line). `games/keifu/mutants/mutate.py`
   is retired in favor of the tool; its lists are unchanged and must score
   identically under the tool — the same noticed / NOT BUILT / ESCAPED verdict
   per fault, on the same commit. A change to the tool that moves any verdict
   is a bug in the tool, not a finding about a list.
3. **Worktrees persist across rounds.** `--jobs N` workers run in N worktrees
   the tool keeps at `target/mutants/w<n>` — detached, moved to HEAD and
   checked clean at round start, kept at round end — so each worker's
   `target/` stays warm and the ~7 core-minute cold build is paid once per
   worktree, not once per round. `--jobs 1` is one worktree too: the round
   never writes into the checkout a person is working in. Each worktree
   builds HEAD unmutated before its first fault, so a cold build is never
   charged to a fault's ten-minute clock and HEAD failing to build is a hard
   error rather than a list of NOT BUILTs. `--fresh` removes and recreates
   them. The workers draw faults from one queue rather than a fixed share.
4. **`--fast` short-circuits.** When `cargo test` notices a fault, `--verify`
   is skipped for that fault; the per-fault line shows `verify=-`, and the
   summary withholds the "verify alone" count, saying why, rather than
   reporting it wrong. The default remains the full pair, so the two-column
   score sessions have been reporting is still available when asked for.
5. **`--changed-since <rev>` filters to faults whose `@` file changed between
   `<rev>` and HEAD** (`git diff --name-only`), and prints how many were kept
   and how many skipped. It filters by file, never by label. A session-time
   round is the new list plus the filtered prior lists; the full round still
   runs, as a job on `main`, because a test regression can un-notice an old
   fault in an untouched file. The CI job is a second session, not this one.
6. **E0 is unchanged.** `tools/` stays off the E0 may-read list; E0 authors
   keep writing their own harness, and that remains part of what E0 measures.
   This ADR takes the deferred decision for adopted games only, and
   e0-findings records it as such.
7. **The cost model lives in `docs/internal/tooling.md`**, beside CI wall
   time, as maintained numbers: core-minutes per fault, core-minutes per cold
   build, SMT's contribution, and the rule that throughput scales with
   physical cores. Anything sized later — a cloud environment, a CI runner, a
   VPS — starts from that paragraph. The before/after measurement of the
   session that built the tool is recorded there, not here.

## Consequences

A full keifu round on the sandbox drops from ~6h toward ~3h with no
infrastructure change (cold builds gone, `--fast` halving the rest);
session-time rounds drop further under `--changed-since`. A later move to
bigger compute is a `runs-on` or environment change and nothing else, and its
sizing is arithmetic from the tooling paragraph. ninjo and any future adopted
game get the instrument without writing it. The "verify alone" column becomes
opt-in. Persistent worktrees cost disk — one warm `target/` per worker under
`target/mutants/`, removed by `--fresh` or by deleting the directory and
running `git worktree prune`. E0's measurement is untouched, and the
e0-findings paragraph that deferred this decision gets its closing entry.
