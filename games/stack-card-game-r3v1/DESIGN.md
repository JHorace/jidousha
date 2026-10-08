# Stack card game (r3v1) — design note

A two-player card duel fought over one shared stack. **The joy to protect:
reading the stack and outsmarting it.** You play one full match against a
scripted rival who plays the stack too.

## The rules

- **Two duellists**, *you* and *the rival*, each start at **12 life**. Bring the
  other to 0 and you win. Both at 0 on the same resolution is a draw.
- **Each owns a 14-card deck** of the same list (below), shuffled from the game
  seed. You open with 5 cards; at the start of every round each duellist draws
  **2** (hand limit 7 — a draw past it is lost). Drawing from an empty deck
  costs **1 life** instead (exhaustion), so every match ends.
- **Focus.** Every round each duellist has **3 focus**; a card costs focus to
  play. Unspent focus is lost at round end.
- **Shield.** Some cards give shield; shield soaks damage before life and is
  gone at round end.
- **The round.** The *leader* (alternates: you lead round 1) gets priority.
  Whoever holds priority either **plays a card** — it goes on top of the stack
  and priority passes to the other duellist — or **passes**.
  - Both pass in a row with something on the stack → **the top item resolves**,
    and priority returns to the leader.
  - Both pass in a row on an empty stack → the round ends.
- **Last in, first out**, unless a card changes the order. A card that names a
  target picks **a stack item** when it is played; if that item has left the
  stack by the time the card resolves, the card *fizzles* and does nothing.

## The card pool

Seven cards. Three are *payloads* (they change life or shield); four are
*manipulations* (they change the stack). The deck is 3 Strike, 2 Haymaker,
2 Ward, 2 Cancel, 2 Bury, 2 Turn, 1 Echo.

| card | cost | does, on resolving | what it is for |
|---|---|---|---|
| **Strike** | 1 | 2 damage to the opponent of its controller | cheap, steady pressure; bait |
| **Haymaker** | 3 | 5 damage to the opponent of its controller | raw power — a whole round's focus on one item, and the biggest target on the stack |
| **Ward** | 1 | its controller gains 3 shield | answers damage **only if it resolves first** — played in response to a hit it saves you, played before one it does nothing |
| **Cancel** | 2 | removes target item from the stack | the clean answer, and the most expensive |
| **Bury** | 1 | moves target item to the bottom of the stack | changes *when*: a Ward buried under a Strike resolves too late; a hit buried under your Ward is soaked |
| **Turn** | 2 | target payload changes controller | changes *who*: their Haymaker hits them, their Ward shields you |
| **Echo** | 2 | puts a copy of target payload, under Echo's controller, on top of the stack | changes *how many*: their best payload becomes yours too |

Legality — **one function** (`legal_targets`), read by the target marks on the
stack panel and by resolution alike:

- Cancel: any item on the stack.
- Bury: any item that is not already at the bottom.
- Turn and Echo: payload items only (a Strike, Haymaker or Ward).

## Why stack manipulation dominates

- **Payloads are inert until they resolve, and the stack decides what they do.**
  Damage goes to "the opponent of its controller" and shield to "its
  controller", both read at resolution — so Turn and Echo rewrite a payload's
  meaning after it was committed.
- **Raw power is a liability.** A Haymaker costs a full round of focus; the
  answer to it costs 2 (Turn: 5 to its caster instead, a 10-life swing; Echo:
  both sides take 5; Cancel: nothing). A duellist who plays the biggest card
  every round loses to one who waits for it — the check's middle player is
  exactly that duellist, and it loses.
- **Order is a resource.** Ward does nothing unless it resolves before the hit;
  Bury flips that. So *when* you play matters as much as *what*, and the
  stack panel's resolution preview is the thing you read before choosing.
- **Leading is a disadvantage you manage.** The leader commits first; the
  responder sees the stack. Leadership alternates, so every other round you are
  the one reading.

## The rival

`rival_choice` is a pure function of the duel: for every card it can afford and
every legal target, it runs the **same resolution-preview function** forward
and scores the outcome (its life plus shield-adjusted margin minus yours);
it plays the best option if it beats passing, else passes. It responds,
counters, buries and turns — it plays the stack, one move deep. It thinks for
`RIVAL_THINK` ticks before acting so a person can watch it.

## The screen (design space 960x540, one world unit = one design unit)

- **The stack panel** (left, always visible): every item top first — its slot
  letter (A = top), card, controller, target, and **what it will do and in
  which order** ("resolves 1st: rival takes 2"). Under it, the outcome line if
  both pass all the way. Legal targets are outlined and tagged `<- target`
  while you are choosing.
- **The priority indicator** (right): who holds priority, and whether the
  other duellist has already passed (so a pass now resolves the top item).
- **Your hand** (bottom): numbered cards with cost and one-line effect, beside
  the stack panel. Unaffordable cards are dimmed.
- **The log** (right, under priority): the last resolutions, newest first.

## Input

- **Keyboard:** `1`–`7` plays that hand card; if it needs a target the legal
  stack items light up and `A`–`H` picks one (`Esc` cancels). `Space` passes.
  `Enter` starts a rematch from the result screen.
- **Mouse / touch:** click a hand card, click a lit stack item, click PASS.

## Decision surfaces (the task's table, as built)

| decision | surface | action | one function | asserted by |
|---|---|---|---|---|
| respond, and with what | stack panel + hand | `1`–`7` / click a card | `preview` (panel and `resolve_top` both) | `--verify` check `respond-window` |
| pass and let it resolve | stack panel + priority indicator | `Space` / PASS | `preview` | `--verify` check `pass-resolves-as-previewed` |
| where to aim a counter/reorder | stack panel, legal targets marked | `A`–`H` / click a lit item | `legal_targets` | `--verify` check `target-reorder` and `target-counter` |
