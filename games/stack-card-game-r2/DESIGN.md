# Stack duel — design note (stack-card-game-r2)

A two-player card duel fought entirely over one shared stack. You play the
**Weaver**; a scripted NPC plays the **Brute**. The joy to protect: *reading
the stack and outsmarting it*.

## Rules

- **Life 20** each. The match ends when a player reaches 0 life, or after
  **20 turns** (10 each), when the higher life wins (equal is a draw).
- **Turns alternate**, you first. At the start of every turn **both** players
  refill to **3 energy** and draw up to **4 cards** (a deck that runs out is
  reshuffled from its discards with the duel's seeded RNG). Unspent energy is
  lost, so energy held back on your own turn is energy for responses.
- **Priority.** The active player holds priority first. Holding priority you
  may play any card you can afford (with a legal target, if it needs one) or
  **pass**. Playing a card puts it on top of the stack and hands priority to
  the other player. Passing hands priority to the other player; **two passes
  in a row** with an item on the stack resolve the **top** item (last in,
  first out), and priority returns to the active player. Two passes in a row
  on an empty stack end the turn.
- **Targets are checked twice**: when a card is played (only legal targets
  can be chosen) and when it resolves (an item whose target has left the
  stack, or is no longer legal, *fizzles*). One function, `duel::legal`,
  answers both.
- A player with nothing playable passes automatically after a short beat, so
  the only priority windows that wait for you are ones where you have a
  choice.

## Input

| action | keys | pointer / touch |
|---|---|---|
| pick a card from hand | `1`–`6` | tap the card |
| choose its target (stack cards) | `Up`/`Down` among the **marked** legal items, `Enter` to play, `Esc` to put the card back | tap a marked stack row |
| play a card with no stack target | `Enter` (or pick it again) | tap the card again |
| pass priority | `Space` | tap PASS |
| next match (result screen) | `R` | tap the result banner |

## The card pool

| card | cost | target | when it resolves | what it is for |
|---|---|---|---|---|
| Bolt | 1 | — | 2 damage to the caster's opponent | cheap raw damage — the bait |
| Blast | 3 | — | 6 damage to the caster's opponent | **raw power**: the Brute's best card, and the most expensive thing to have turned against you |
| Surge | 2 | — | 1 damage, **+1 per other item still on the stack** | rewards a deep stack and *early* resolution — sinking it or letting it resolve last is the counterplay |
| Mend | 1 | — | the caster gains 3 life | the only non-stack defence; loses to Cancel like everything else |
| Cancel | 2 | any item | removes the target from the stack | the counter; a Cancel can itself be cancelled |
| Mirror | 1 | a damage item (Bolt, Blast, Surge) | that item's damage now goes to **its own caster** | **redirect**: one energy turns a three-energy Blast around — the clearest case of sequencing beating power |
| Echo | 2 | any item but an Echo | puts a **copy** of the target on top, controlled by Echo's caster (damage aimed at the new controller's opponent) | **copy**: a response that becomes your own threat, and that resolves *before* the original |
| Sink | 1 | any item | moves the target to the **bottom** of the stack | **delay / reorder**: makes a threat resolve last, after your answers — or drains a Surge |
| Flip | 1 | — (needs 2+ items under it) | reverses the order of everything below it | **reorder**: the whole resolution order inverted for one energy |

Decks (16 cards each, deliberately asymmetric):

- **Weaver (you):** Bolt ×3, Surge ×2, Mend ×1, Cancel ×3, Mirror ×3, Echo ×2, Sink ×1, Flip ×1.
- **Brute (NPC):** Blast ×5, Bolt ×5, Mend ×2, Cancel ×3, Mirror ×1.

## Why stack manipulation dominates

Raw power is priced high and answered cheap: Blast costs 3 and Mirror costs 1,
so a Brute who plays Blast into an open Mirror loses 6 life and its turn's
energy at once. Every manipulation card is a *response*, and responses are
free tempo, because energy refills for **both** players every turn — the
player who holds energy back on the opponent's turn and spends it on the
stack wins the exchange. Ordering effects (Surge, Sink, Flip, Echo's copy
landing on top) mean that *which item resolves first* changes the outcome, so
the resolution order the panel shows is the thing the player reads and the
thing they play against. The verify run plays the Weaver three ways — the
stack reader, a "play my cards and never respond" first-timer, and a player
who passes everything — and the reader has to beat the Brute while the
first-timer's power alone does not reliably do it.

## The opponent

The Brute's choice is the free function `npc::choose(&Duel, Side)`: for every
legal action (each affordable card at each legal target, and pass) it runs
the stack forward with `resolve::preview` and scores the result (its life minus
yours, a little against energy spent); it takes the best, preferring pass on
ties. So it responds when a response pays — it counters your Cancel, mirrors
your Bolt, cancels your Surge — and it plays its Blasts into an empty stack.
It waits **30 ticks** (half a second) before every action so a person can
see what it did.

## Decision surfaces (the spec's table, as built)

| decision | surface | one function | asserted by (`--verify`) |
|---|---|---|---|
| respond, and with which card | the stack panel (top first, each item with what it will do and the order it resolves in) beside the hand row | `resolve::resolve_top`, the one resolution step: `resolve::preview` folds it over a copy for the panel, the real resolution runs it on the duel | `row 1 respond` (`scenarios.rs`): three or more items, every item and its resolution order found in the panel and on the frame at the priority window; then resolved and compared with the preview |
| pass and let it resolve | the stack panel, plus the PRIORITY line saying whether the opponent can still respond | the same step and fold | `row 2 pass`: passes at a window and asserts the resolution log equals the preview exactly |
| where to aim a counter / reorder / redirect | the stack panel with legal targets marked `>` while choosing | `duel::legal` — the marking, the play and the resolution check all read it | `row 3 aim`: on fixed hands a Sink is aimed by keys and a Cancel by taps, the marked rows equal `legal_targets`, and the new order shown equals the order that resolves |

Layout: a 960x540 design space mapped 1:1 onto a 540-unit-tall camera at
16:9 (`jidousha::ui` `Panel` for every row of text; boxes drawn beside it).
Every match the verify run plays also checks each resolution against the
preview taken at the priority window before it (`N of N resolutions matched`).
