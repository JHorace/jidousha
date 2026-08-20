# ADR-0029: E0's bar counts *novel* findings; a re-tread does not reset the streak

Status: accepted · 2026-08-20

> **The bar moves from "no new `engine` or `docs` findings" to "no `engine`
> finding and no *novel* `docs` finding".** A finding whose cross-run column
> names a prior `F-` number is still recorded and still fixed; it no longer
> restarts the count. The streak restarts at zero, which costs nothing, because
> it has been zero for eight runs.

## Context

E0 is the project's definition of working, and §2 of `e0-findings.md` has said
since run 1 that it passes when **two consecutive runs produce no new `engine`
or `docs` findings**. Eight runs in, that series has not moved:

| run | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|
| blocking findings (`engine` + `docs`) | 14 | 13 | 8 | 14 | 8 | 10 | 15 | 14 |
| — novel | 14 | 13 | 8 | 14 | 6 | 8 | 7 | **3** |
| — a re-tread of a recorded shape | 0 | 0 | 0 | 0 | 2 | 2 | 8 | **11** |
| `engine` findings | 5 | 3 | 0 | 3 | 1 | 1 | 0 | 0 |

The count is flat and the **composition has inverted**. Novel findings averaged
12.2 a run over runs 1–4 and 6.0 over runs 5–8, reaching 3 in run 8. Engine
findings are gone: zero in three of the last four, and run 8's one engine-shaped
item (F-097) was a doc comment that contradicted its own function rather than
behaviour anybody wanted changed.

Author cost tracks the novel series rather than the raw one. Run 1's log says
the document "did not survive it"; run 8's says it was never blocked, never
wanted the source to learn what a function *did*, and caught 23 of 23 injected
faults. What run 8 lost to friction was one cycle.

**Two structural facts explain the flat line better than any story about the
engine.**

**The prompt selects against the bar.** `e0-prompt.md` tells every run, in the
text that must not change between runs:

> A run that reports no friction and produces a working Pong is a less useful
> run than one that limps and says why.

and §1 of this file converts *any* friction into a `docs` finding until proven
otherwise. So the instrument is calibrated to produce findings and the bar
requires none. Passing would take a fresh agent reading a 15,000-token document
cover to cover and having no comment on it — not a realistic state, and not one
the exercise should be steering toward, because the run that has no comment is
also the run that teaches nothing.

**A re-tread is a different claim from a gap.** F-102 is F-068's rule escaping
its worked instance; F-110 is F-072's list of lints being one short. Each is
worth recording and each got fixed. But neither says *the document is
inadequate in a way nobody knew about* — which is the question E0 exists to
answer. Counting them the same way means a run that discovers nothing new still
resets the streak, so the bar can be held open for ever by findings that are, by
construction, already understood.

## Decision

**E0 passes when two consecutive runs produce no `engine` finding and no novel
`docs` finding.**

- A `docs` finding is **novel** unless its "Also found by" names a prior `F-`
  number or an already-recorded shape. That column is already mandatory in every
  triage table, so this adds no bookkeeping.
- A re-tread is recorded, classified and fixed exactly as now. It does not reset
  the streak.
- `engine` findings still reset it outright, novel or not. An engine defect
  found twice is worse than one found once.
- An `author` finding still never resets it, as before.

**The streak restarts at zero.** `e0-prompt.md`'s ledger says any change that
makes a run easier invalidates the streak and restarts the count. This is such a
change and the rule is honoured — at a price of nothing, because the count has
been zero since run 1. That is the argument for doing it now rather than after a
run that would have to give something up.

## Rationale

The bar should measure the thing the milestone is about. E0 asks whether
`docs/api/` is enough on its own for an author who cannot read the source. A
novel finding is direct evidence that it is not. A re-tread is evidence about a
*previous fix's* reach, which is a real and different question — one the triage
already tracks, and one that a growing corpus makes easier to find rather than
harder. With 112 findings on file, almost any friction has a relative; with 15,
none did. That alone would have made the raw count drift upward over time
regardless of the engine's health, which is a bad property for a bar.

The novel series is also the one that behaves like a convergence: 14, 13, 8, 14,
6, 8, 7, 3. Under the new bar run 8 scores 3 — not a pass, and close enough that
"one or two more runs" is a defensible sentence rather than a hope.

## Consequences

- Run 8's triage is re-read against this bar in §4e; it does not become a pass.
- §2 of `e0-findings.md`, §3 of `implementation-plan.md` and after-the-run step 5
  of `e0-prompt.md` all state the bar and all change together. A bar stated three
  ways that drift apart is worse than a bar stated once.
- **The prompt does not change.** It is right to ask for every friction, and the
  conflict is resolved on the counting side rather than by asking runs to
  self-censor — which would cost the exercise the entries it is most valuable
  for. The ledger's comparability argument is untouched.
- The re-tread count becomes a number worth watching in its own right: it is a
  measure of how well fixes generalise, and eight consecutive re-treads of the
  controller advice is what spent run 4's lever (see `docs/e0/` and §4e).

## Alternatives rejected

- **Keep the bar and keep running.** The data gives no reason to expect a zero,
  and each run costs a session plus a triage. A milestone that cannot be reached
  is not a bar, it is a treadmill — and the failure mode is that somebody
  eventually declares victory informally, which is worse than moving it on the
  record.
- **Bar on cost instead of count** — "no friction that cost the run a cycle".
  Closer to what anybody actually wants, and unmeasurable: a run's own estimate
  of what a friction cost it is the least reliable number in its log, and it is
  not a number a maintainer can check.
- **Drop the "do not soften these" instruction from the prompt**, so runs report
  less. This trades the exercise's whole output for a number, and it changes the
  prompt, which the ledger says makes runs incomparable.
- **Declare E0 passed on run 8's substance.** Defensible — four runs with zero
  engine findings, and run 8 shipped without opening the source. Rejected
  because the second clean run is what distinguishes a fixed engine from a lucky
  one, and that argument is as good now as it was at run 1.
