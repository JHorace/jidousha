# stack-card-game — a two-player card duel fought entirely over the stack

Kind: game · Variant: V1 · Size: M · Window: burn-down (the handoff said
"night"; `burn-down` is the schema's value for the Wednesday window)

## Goal

A new two-player card game, `games/stack-card-game`, that goes all in on
Magic: The Gathering's notion of the stack: every played card enters a shared
stack, and the whole game is about manipulating it — resolution order,
timing, countering, reordering. **The joy to protect: reading the stack and
outsmarting it.** Prototype scope: one full match against a scripted NPC
opponent, end-to-end playable.

## Fence

- **Grants write access to `games/stack-card-game/**`** (new) and `Cargo.lock`
  for that crate's own entry only, plus the run folder
  `tools/yakin/runs/stack-card-game/`.
- Grants nothing else. No new dependencies: the manifest names `jidousha` by
  path and nothing else. Card data and art are the game's own (text and
  primitive shapes are fine; any asset is CC0 or made here and credited).
  No Magic: The Gathering names, card text or art — the stack is the idea
  borrowed, nothing else.

## The brief

- **V1: design inline, then implement.** The tick writes a compact design
  note in the game folder (`games/stack-card-game/DESIGN.md`) — the rules,
  the card pool, why stack manipulation dominates — before the code. The run
  folder's DESIGN.md (WORKER.md §3) may be short and point at it.
- **The stack is the game.** Every played card goes onto one shared stack;
  both players can respond; it resolves last-in-first-out unless a card
  changes that. Cards that counter, reorder, delay, redirect or copy what is
  on the stack are the core of the pool, not a garnish.
- **A small, deliberately asymmetric card pool** that makes stack
  manipulation the dominant skill: raw card power should lose to good
  sequencing. The note says what each card is for.
- **The opponent.** A scripted NPC that plays the stack too (it responds, it
  counters), so outsmarting it is the skill being tested.
- **House pattern.** Deterministic sim (seeded shuffles, replayable input —
  CLAUDE.md convention 4), a `--verify` mode with a scripted player and
  transcript gates (make-game §A.4–A.5), the mutation round (§A.6), a
  captured picture (§A.7).

## Decisions this task adds

| decision | must know | surface | action | one function | asserted by |
|---|---|---|---|---|---|
| Whether to respond to what is on the stack, and with which card | the whole stack in order, top first; what each item will do when it resolves; the order it will resolve in; my playable responses and what each would change | the stack panel, always visible during a priority window, with my hand beside it | play a card from hand onto the stack while I hold priority (the note names the input) | one resolution-preview function that computes what the stack will do in order — the panel and the actual resolution both call it | a scripted `--verify` check that, with three or more items on the stack, asserts every item and the resolution order are in the transcript at the priority window, then resolves and asserts the outcome matches the preview |
| Whether to pass priority and let the stack resolve | the same as above, plus whether the opponent can still respond | the stack panel and a visible priority indicator | the pass input | the same resolution-preview function | a `--verify` check that passes and asserts the resolution matches the preview exactly |
| Where to target a reorder, counter or redirect | which stack items it can affect and the resulting order | the stack panel, with legal targets marked while choosing | selecting a stack item as the target | one legality function the target marking and the resolution both read | a `--verify` check that counters or reorders on a fixed seed and asserts the new order shown equals the order that resolves |

## Done when

- `games/stack-card-game/DESIGN.md` exists and states the rules, the card
  pool and why stack manipulation dominates.
- One full match against the scripted NPC plays end to end, and a scripted
  `--verify` run reaches its result screen.
- `python3 tools/verify stack-card-game` reports `pass` in
  `target/verify/stack-card-game.json`, with a check for each decision row
  above.
- `git diff --stat origin/main...` names only `games/stack-card-game/`,
  `Cargo.lock` (that crate's entry only) and
  `tools/yakin/runs/stack-card-game/`.
- `python3 tools/build-web stack-card-game && python3 tools/serve-web stack-card-game --check`
  passes.
- A PR titled `[yakin:V1] <this task's queue title>` is open with the WORKER.md
  body.
