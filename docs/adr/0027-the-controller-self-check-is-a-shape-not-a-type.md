# ADR-0027: The controller self-check is a shape the document names, not a type the engine ships

Status: accepted · 2026-08-19

> **Accepted as recommended, which means `jidousha::testing` gains nothing.**
> What changed is that *Testing your game* now names **three** contracts a
> controller has instead of one, with the reading that tells them apart. The
> proposal to ship an accumulator that collects and prints them is declined, and
> the reasons are recorded here because the question will be asked again.

## Context

Five consecutive E0 runs have been sent into their own game's constants by a
fault in the controller that drives their `--verify` mode (F-037, F-047, F-056,
F-074, and now run 7). The document's response has been prose, four times, each
time rewritten after the next run walked into it. F-056's fourth attempt added
the thing that finally worked in one direction — a contract check the run
performs on the numbers it actually picked — and run 6 discharged the warning in
one step with it.

Run 7 wrote that check, reported `met 27 of 27 approaches`, and mis-tuned two of
its game's constants anyway. The check was healthy and the controller was broken:
meeting a ball and threatening with it are different contracts, and `met N of M`
covers only the first. The run's own conclusion is narrower than the document's
and more actionable:

> a controller self-check has to be a check the run performs, on the same
> numbers, in the same output, or it is prose again. Mine was. The thing that
> failed was that I only had *one* number and it was healthy.

Which raises the question this decision answers: if what works is an instrument
rather than a paragraph, should `jidousha::testing` **ship** the instrument —
something that accumulates approaches, met, planned threat and planned-versus-
actual, and prints the block — rather than describing it?

## Decision

**No. Name the three contracts in the document; add nothing to the engine.**

*Testing your game* now carries the three numbers, what each one clears, and the
four-way reading that says which half of the program to open. That is the whole
change.

### Why not the type

*The plumbing was never what failed.* Run 7 built its instrument without
difficulty; three floats, a counter and a `println!`. What it did not have was
the knowledge that there were three numbers rather than one. An accumulator
cannot tell a run what to accumulate, so shipping one closes the cheap half of
the problem and leaves the expensive half exactly where it was. The failure is a
**catalogue** failure, and a catalogue is a document.

*The vocabulary is the game's, not the engine's.* "Approach", "met", "planned
landing", "actual landing" are Pong's words. A racing game's contract is apexes
and braking points; a fighting game's is blocked versus whiffed. A type general
enough to hold all of them is a named `Vec<f32>` — a second way to spell
something a game already has, which is the one thing this repository's top
convention forbids. A type specific enough to be useful is an engine that knows
what a paddle is.

*It would be a second way to do something that already has one.* A `--verify`
mode already prints a summary in the shape `tools/verify` parses, and the three
numbers are three more indented lines in it. A type whose output is those lines
competes with `println!` and wins nothing.

*The cost of being wrong is asymmetric, and it points at the milestone.* A
declined API costs a paragraph. An accepted one costs public surface that every
future E0 run has to read, inside a document whose token budget is nearly spent
— and E0 measures whether that document is sufficient. Growing the surface to
patch a documentation failure moves the measurement in the wrong direction
(ADR-0025, public-api.md §4).

## Consequences

- **The three numbers are named where the warning lives**, with what each one
  clears and what their combinations mean. A maintainer reading a run's verdict
  block can now tell a broken controller from a broken game without opening
  either.
- **`met N of M` stops being described as *the* contract check.** It was, in the
  document, and that is what let run 7 stop looking.
- **The question is reopenable on evidence, and the evidence is specific.** If a
  run writes all three numbers, prints them, and still cannot act on them, that
  is evidence about the catalogue rather than the plumbing and this decision does
  not cover it. If two runs in genuinely different genres converge on the same
  accumulator shape, the vocabulary objection weakens and the proposal should be
  reheard.
- No signature changes; nothing to regenerate beyond the two documents.

## Alternatives considered

**Ship `ControllerLog` (or `ContractLog`) in `jidousha::testing`.** The proposal.
Rejected on all four grounds above; the catalogue argument is the one that would
still hold even if the type were free.

**Ship nothing and say nothing.** The status quo, and it is what produced a fifth
run steered into its game's constants by its own driver. Rejected for the reason
ADR-0020 and ADR-0024 give: a silence that manufactures a confident wrong move is
not neutral.

**Spend the lever run 4 named — a worked controller in a game unlike Pong.**
Held, and the reason is that run 7's failure is not the failure that lever was
reserved for. Runs 1–5 did not do what the prose said. Run 7 did *exactly* what
the prose said: it wrote "take the return that lands furthest from the middle"
against an opponent that chases the ball, for which that objective is close to
the worst available. That is a false sentence rather than an unheeded one, and a
second worked instance of a false prescription would have propagated it into
another genre rather than caught it. The prescription is now stated as a
principle with its reduction labelled as one, which is what a second genre would
have forced anyway. Revisit if run 8 writes a controller that honours the
principle and still ends up in its game's constants.
