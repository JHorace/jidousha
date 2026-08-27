# VARIANT — giri-rt

Tier-3 crate fork of `games/giri/` (variants policy, DESIGN §8b).
Created 2026-08-27 from the P2 playtest verdict.

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
