# brolf — battle-royale golf, a 2D prototype against NPC opponents

Kind: game · Variant: V2 · Size: M · Window: burn-down (the handoff said
"night"; `burn-down` is the schema's value for the Wednesday window)

## Goal

A new 2D game, `games/brolf`: golf as a battle royale. A safe zone shrinks
over the course; you and your ball must both stay inside it. Players can
club each other and each other's balls, gather equipment that shapes how
they play, and win in more than one way — including extracting with some of
what they gathered. Prototype scope: one full match loop against NPC
opponents, end-to-end playable. No multiplayer.

## Fence

- **Grants write access to `games/brolf/**`** (new) and `Cargo.lock` for that
  crate's own entry only, plus the run folder `tools/yakin/runs/brolf/`.
- Grants nothing else. No new dependencies: the manifest names `jidousha` by
  path and nothing else.

## The brief (the design stage designs from this)

- **The zone.** A gradually shrinking safe zone that the player **and** their
  ball must remain inside. Elimination = you or your ball outside the zone
  beyond a grace period. The design fixes the shrink schedule, the grace
  period and how both are shown.
- **Contact.** Players can club other players — a temporary disable, never a
  direct permanent kill — and can strike other players' balls.
- **Equipment.** Gatherable equipment and power-ups that shape playstyle —
  for example a ball that is hard for others to hit, or resilience to being
  clubbed. The design picks a small set that pulls play in different
  directions.
- **Why be aggressive.** Aggression needs an incentive. Multiple win
  conditions, extraction-shaped: a successful extraction keeps some of your
  stuff. **Design this explicitly** — what the win conditions are, what
  extraction is, what is kept, and why clubbing or ball-striking pays.
- **Scope.** One full match loop against NPC opponents, end to end: start,
  play, zone closes, someone wins or extracts, result shown. NPCs good enough
  to make the zone, contact and extraction matter.
- **House pattern.** Deterministic sim (seeded RNG, fixed timestep, replayable
  input — CLAUDE.md convention 4), a `--verify` mode with scripted players and
  transcript gates (make-game §A.4–A.5), the mutation round (§A.6), a captured
  picture (§A.7).

The designer cuts anything that will not build in the night's remainder into
Non-goals (DESIGNER.md §3, "Size"); the brief's scope line is the floor, not a
wish list.

## Decisions this task adds

| decision | must know | surface | action | one function | asserted by |
|---|---|---|---|---|---|
| Where to hit my ball next | the zone now and where it will be by the time the ball lands and I reach it; the grace time left if I am outside; the shot's reach | the course view during aiming — current zone, next zone, the aim line and its landing area, all on screen while aiming | aim and commit the shot (the design names the input) | one zone-membership-at-time function that the drawn zone, the grace countdown and elimination all read (the design names it) | a scripted `--verify` check asserting the current and next zone and the grace countdown are in the transcript while aiming, and that a ball left outside past grace is eliminated on the tick the function says |
| Whether to club a nearby player or strike their ball, or keep playing my own ball | who is in reach; how long a club disables them; what striking their ball does to them; what it costs me (time, zone, exposure) | the course view when a target is in reach (the design names the cue) | the contact input (the design names it) | one contact-resolution function the reach cue and the resolution both read | a `--verify` check where a scripted player clubs an NPC: the reach cue is in the transcript first, then the NPC is disabled for exactly the stated time and never permanently removed by the club |
| Whether to pick up a piece of equipment | what it does, and whether it replaces something I hold | the course view at the pickup, readable before taking it | moving onto or taking it (the design names the input) | one equipment-effect function that the description and the sim read | a `--verify` check that takes one item on a fixed seed and asserts its stated effect is what the sim then does |
| Whether to extract now or fight on | what extracting keeps; what the other win conditions are and how close each is | an always-visible match status (the design names it) | the extract input at an extraction point (the design names it) | one outcome function that the status and the end-of-match result both read | a `--verify` run that extracts and asserts the result screen keeps exactly what the status promised |

The design stage elaborates these rows; it does not add or drop a decision
(DOCTRINE §6).

## Done when

- One full match loop against NPC opponents plays end to end, and a scripted
  `--verify` run reaches its result screen.
- `python3 tools/verify brolf` reports `pass` in `target/verify/brolf.json`,
  with a check for each decision row above.
- `git diff --stat origin/main...` names only `games/brolf/`, `Cargo.lock`
  (that crate's entry only) and `tools/yakin/runs/brolf/`.
- `python3 tools/build-web brolf && python3 tools/serve-web brolf --check`
  passes.
- A PR titled `[yakin:V2] <this task's queue title>` is open with the WORKER.md
  body; its Deviations list every departure from DESIGN.md.
