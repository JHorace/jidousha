# VARIANT — giri-rt — **CLOSED: CONFIRMED, adopted as ninjo**

Tier-3 crate fork of `games/giri/` (variants policy, DESIGN §8b).
Created 2026-08-27 from the P2 playtest verdict. **Closed 2026-08-30**
(wave 0b): the hypothesis below is CONFIRMED, the substrate is adopted, and
the crate is now `games/ninjo/`.

This file is kept, not deleted: it is the record of a question that was
asked properly and answered. The outcome is at the bottom; everything
between here and there is what was written before the answer was known, and
it is left in its original tense on purpose.

## Hypothesis

giri's information-overload problem is a **delivery** problem, not a
systems problem. The P2 ladder failed not because probability is wrong
but because (a) a four-beat horizon cannot absorb bad outcomes and
(b) everything happens simultaneously on one screen. A real-time-with-
pause world map — where events have time and place addresses, and pause
gives the player consent over when to think — should make the same
social machinery legible when it reintegrates.

This fork builds **the substrate only**: map, clock, moving parties,
addressed events, attention machinery. The social layer is stubbed
(parties succeed, pots pay, no refusal, no betrayal). giri mainline is
parked but stays green; its mechanics return here if the substrate
proves out.

## Exit criteria (what "decided" looks like)

- **Adopt**: the owner playtests S1/S2 and the loop of dispatching,
  watching the world run, and being interrupted at the right moments
  feels fundamentally sound → the reintegration design pass begins
  (S3), and giri's social systems migrate onto the substrate's event
  classes. giri mainline then retires to `attic/`.
- **Reject**: the substrate feels dead or the pacing can't be tuned
  into shape → giri-rt retires to `attic/` with a FINDINGS-style
  postmortem, and giri mainline resumes as the live line.

Either way this variant does not stay alive indefinitely — the ≤2-alive
budget holds.

## Outcome (2026-08-30, wave 0b) — **CONFIRMED · adopt**

**Verdict: adopt.** The owner played S1. Dispatching parties, watching the
world run, and being interrupted at the right moments read as fundamentally
sound — the delivery structure is the thing giri's social machinery was
missing, exactly as the hypothesis says. The information-overload problem
was a delivery problem.

What the verdict set in motion, and where each half now lives:

- **The reintegration design pass happened** and produced a game rather
  than a patch: `GDD.md`, ninjo (人情), where giri's mechanics return as
  want-mechanics rather than obligation-mechanics. giri's social systems
  migrate onto this substrate's event classes over waves 1–4; the registry
  in GDD §5 is the schedule, and GDD §8 is the wave plan.
- **The substrate itself is unchanged and is the foundation.** `DESIGN.md`
  stays as its technical doc, in its own voice, cross-linked from the GDD.
- **The crate renamed** `games/giri-rt` → `games/ninjo` in wave 0b, with
  the page, the stamps, the capture names and the sync filters. Wave 0b also
  landed the first half of the reintegration: the people substrate —
  characters, the trait vocabulary, regard, bonds and grudges, marks —
  ported from giri mainline by copy-adapt, with no dependency on the giri
  crate.
- **giri mainline is untouched and still green.** Its retirement to
  `attic/` is a separate decision on a separate day; nothing in wave 0b
  needs it gone, and its `--verify` run is the guarantee the port did not
  reach back into it.

**The ≤2-alive variant budget clears.** giri mainline plus this fork was the
pair; the fork stopped being a variant the moment it became the line. There
is one live prototype in this lineage again, and it is ninjo.

Nothing below this line is open. A new question about the substrate is a new
document, not an edit to this one.
