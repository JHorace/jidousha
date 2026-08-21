# ADR-0036: E0 closes on substance rather than on its bar

Status: accepted · 2026-08-21 · **supersedes ADR-0029; the milestone ends without meeting the condition it stated**

> **E0 is complete after eleven runs.** There is no run 12. The bar ADR-0029
> set — two consecutive runs with no `engine` finding and no *novel* `docs`
> finding — was never met and is not met now: the streak at closure is **zero**.
> The exercise ends because its own instrumentation says the number it was
> counting stopped measuring the document, not because the number reached zero.
> This is a decision to stop, and it is deliberately not called a pass.

## Context

E0 asks one question: is `docs/api/` enough on its own for an author who cannot
read the source? Eleven fresh sessions have now answered it, and the series looks
like this.

| run | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| novel `docs` | 14 | 13 | 8 | 14 | 6 | 8 | 7 | **3** | **3** | **3** | **1** |
| re-tread of a recorded shape | 0 | 0 | 0 | 0 | 2 | 2 | 8 | 11 | 5 | 6 | 3 |
| `engine` | 5 | 3 | 0 | 3 | 1 | 1 | 0 | 0 | **1** | **0** | **1** |

**Three things in that table decide this.**

**The novel count measures the author, not the document.** §6 of
`e0-findings.md` reached this after run 10 and said so before this decision was
taken: three consecutive runs at exactly three novel findings — across two
document splits, an ADR, twelve fixes and a change to the bar itself — with the
re-tread count wandering (11, 5, 6). *"Three is roughly what a fresh author
meeting fifteen thousand tokens of prose produces, and it is not a measurement of
this document's remaining gaps."* Run 11 scored 1, below the floor that
assessment named, and it did so in a run that also produced an `engine` finding —
so even the movement does not read as convergence. The number the bar counts has
stopped tracking the thing the bar is about.

**No run since run 4 has found the engine doing the wrong thing.** Seven runs.
The two `engine` findings since are both a missing affordance rather than a
defect in behaviour, and both were found by trying to write one line:

- **F-116** (run 9) — `find_bounds`, the fold six checks were writing out by
  hand. ADR-0032. Nothing computed a wrong answer; six files computed the right
  one at length.
- **F-137** (run 11) — `PhysicalSize::aspect` not being a `const fn`, so a layout
  in constants could not take its shape from its window. Nothing computed a wrong
  answer; one ratio was typed by hand.

Each was fixed in the session that triaged it, in one line plus a test. Run 5's
one engine finding was *declined* on the merits (ADR-0024) and runs 3, 7, 8 and
10 found none at all. The last run to report engine behaviour anybody wanted
changed was run 4, and its three findings were settled by ADRs 0021–0023 seven
runs ago.

**What the last four runs actually did.** Each shipped a playable Pong that
compiled clean, ran `--verify` green, and was played — in a window and in a
browser. None was blocked. None wanted `src/` to learn what a function *did*;
run 11 wanted to look something up twice, went neither time, and reports that
both times the document had it and the run had misread. Their own mutation rounds
scored 23 of 23, 18 of 20, 22 of 23 and 19 of 19. That is the substance the
question was asking about, and it has been steady for longer than any streak the
bar would have counted.

## Decision

**E0 is complete. The milestone is ticked and no run 12 is scheduled.**

- ADR-0029's pass condition is **retired**, not moved. It is not replaced by a
  looser bar, because a second bar chosen after seeing the data it will be
  applied to is not a bar.
- The claim being recorded is: **`docs/api/` is sufficient for an author who
  cannot read the source to build, check, capture and ship a working Pong** —
  demonstrated eleven times, four of them consecutively without a blocker, by
  authors who had not seen the document before.
- The claim **not** being recorded is that the streak was achieved. It was not.
  §2 of `e0-findings.md` now states the count at closure — zero — rather than
  quietly dropping it.
- `docs/internal/e0-prompt.md` is kept intact and unchanged. E0 remains runnable
  as a deliberate regression check; what ends is the *repeat-until-clean* loop,
  not the harness.

## Rationale

**ADR-0029 asked for exactly this, on the record.** Its own alternatives section
rejected "keep the bar and keep running" with: *"A milestone that cannot be
reached is not a bar, it is a treadmill — and the failure mode is that somebody
eventually declares victory informally, which is worse than moving it on the
record."* Three runs later the bar has still not been reached and §6 has
explained why it will not be. Writing this down is the behaviour ADR-0029 asked
for; another run would be the treadmill it named.

**The evidence that closes this is not the same evidence ADR-0029 saw.** That
matters, because ADR-0029 rejected "declare E0 passed on run 8's substance" and
this decision must not pretend to be a new argument for an old position. Two
things are genuinely new. First, §6's floor finding, which is a claim about the
*instrument* and could not be made until three runs had scored the same number
through structural change. Second, three further runs of engine data: at run 8
the "zero engine findings" streak was four runs old and untested by anything
since; it is now seven runs and has survived two runs that went looking hard
enough to find affordance gaps and reported nothing worse.

**The residue is a known shape and it is not an engine question.** §4g and §4h
name it: a previous fix answering the question asked and not the one beside it.
Three of run 11's four `docs` findings are that, and so is its `engine` finding —
F-069 wrote the const-fn rule down and did not apply it to the list the rule
itself created. That residue is real and worth working on. It is a documentation
review practice, and it does not need a fresh Pong every time to surface one
instance of itself.

## Consequences

- `implementation-plan.md` §4 ticks E0. `make-game` remains unticked and is the
  next thing: practices §3 says it is written from what E0 taught, and eleven
  triages are what it is written from.
- **`crates/jidousha/examples/pong/` stays in the tree**, and stays registered in
  `tools/test`. The delete-before-the-next-run step (`e0-prompt.md` step 2,
  F-020) exists so a worked solution is not sitting in the next author's allowed
  reading; with no next author it protects nothing, and run 11's game becomes a
  permanent worked example beside `slalom/` and `prototype_kit/`.
- **The run 12 watch list in §6 is kept and marked unrun.** It is a maintainer's
  prediction about fixes that did land, and deleting it would remove the only
  record of what those fixes were expected to buy. If E0 is ever re-run, it is
  the list to score against.
- §2 of `e0-findings.md`, §3 of `implementation-plan.md` and after-the-run step 5
  of `e0-prompt.md` all stated the bar and all change together, as ADR-0029's
  consequences require. A bar stated three ways that drift apart is worse than a
  bar stated once — and that holds for a retired bar too.
- The findings file stops growing and becomes a source rather than a ledger.
  F-141 is the last finding.

## What this decision does not establish

Recorded because a future reader will otherwise read "E0 complete" as more than
it is.

- **The bar was never met.** Consecutive clean runs at closure: zero. Every run
  produced at least one novel `docs` finding or one `engine` finding, and two of
  the last three produced an `engine` finding.
- **The second clean run was the thing that distinguished a fixed engine from a
  lucky one**, and that argument — made at run 1, restated in ADR-0029 — is not
  refuted here. It is given up, knowingly, in exchange for seven runs of engine
  quiet and a diagnosis of why the docs number will not fall.
- **This is a claim about Pong.** §5's caveat stands and is the most important
  sentence in this ADR: the three documents are *visibly* aimed at Pong, so
  friction no run hit is not evidence about a different game. "The document is
  sufficient for Pong, demonstrated by authors who had not seen it before" is the
  claim. "The document is sufficient for a game" is not, and no number of Pongs
  would have made it one.
- **A run 12 would very likely have found something.** On the series it would
  have found roughly three `docs` findings, one of them novel, and possibly an
  affordance gap on a type nobody has tried to write a constant of yet. Stopping
  is a judgement that those are worth less than what the same session spends on
  `make-game`, not a prediction that they do not exist.

## Alternatives rejected

- **Run E0 once or twice more and close either way.** The honest version of "one
  more run" — but §6 already predicts what it returns, and a decision to stop
  that is taken after two more confirmations of a known number is the same
  decision taken later and more expensively. If the prediction is wrong the
  harness is intact and the run can be made.
- **Loosen the bar to "no `engine` finding" alone and pass on that.** Run 10
  would clear it and run 11 would not, so it would mean waiting for one more run
  anyway — and a bar rewritten to fit the data in front of it is not evidence.
  Retiring the bar openly is more honest than lowering it.
- **Declare a pass.** Rejected. The word has a definition in §2 and this does not
  meet it. "Complete" and "passed" are different claims and the file should not
  spend eleven runs of careful counting and then blur them in the last entry.
- **Delete the harness and the watch list as finished business.** Rejected: the
  cost of keeping them is a directory nobody reads, and the cost of losing them
  is that a future regression check has to reinvent a prompt whose comparability
  ledger took five revisions to get right.
