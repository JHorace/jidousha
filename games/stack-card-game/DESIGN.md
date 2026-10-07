# Stack Duel — design note (yakin task `stack-card-game`)

A two-player card duel fought entirely over **one shared stack**. The joy to
protect: *reading the stack and outsmarting it.* One full match against a
scripted NPC. Only the idea of a stack is borrowed from anywhere; every card,
name and number here is this game's own.

## Rules

- 20 life each. 5-card hand, draw 1 at the start of your own turn (the first
  player skips the first draw). Decks are seeded shuffles; the two decks differ.
- **Mana.** On your turn mana refills to `min(your turn number, 6)`. Unspent
  mana *stays* through the opponent's turn — it is what you answer with. Mana
  you spend in response is mana you do not have next to cast on your turn
  (it refills only at your own turn start), so holding mana open is a bet.
- **Speed.** *Sorcery* cards can be played only by the active player, with the
  stack empty. *Instant* cards can be played whenever you hold priority.
- **The stack.** Every played card is an item on one shared stack, with its
  owner, an optional target (another item), and an *aim* (which player it will
  land on). The top item resolves first (last in, first out).
- **Priority.** After a card is played the *other* player gets priority.
  A pass hands priority over. When both players pass in a row: if the stack
  is non-empty the top item resolves and the active player gets priority
  again; if it is empty the turn ends. Every turn therefore ends only when
  both players are done.
- **Resolution** is one function (`rules::resolve_top`). The stack panel's
  preview is that same function run on a copy until the stack is empty; the
  real thing is the same function run on the real stack. They cannot disagree.
- **Targets** are stack items. One legality function (`rules::legal_targets`)
  answers both "which items may I choose" (the panel marks them) and "is the
  target still legal when this resolves" (if not, the card *fizzles* and says
  so). No effect may be a silent no-op: a card with no legal target cannot be
  cast.
- **Winning.** Life 0 or less loses. After round 12 (24 turns) higher life
  wins; equal life is a draw.

## The card pool (what each is for)

| card | cost | speed | effect | it is for |
|---|---|---|---|---|
| Ember | 1 | instant | 2 damage to the opponent | cheap reach; the thing a Hush answers |
| Cleaver | 3 | sorcery | 6 damage | the honest threat — and a target |
| Siege | 5 | sorcery | 10 damage | half a life bar; the greediest thing to have redirected |
| Mend | 2 | instant | you gain 4 | the thing Redirect turns into a gift to the other side |
| Negate | 2 | instant | counter target item | the clean answer; its cost is the mana you did not spend elsewhere |
| Hush | 1 | instant | counter target item with cost 2 or less | cheap, narrow: answers Ember / Mend / Hush and tempo plays |
| Flip | 1 | instant | reverse the order of everything below it | turns "I resolve first" into "I resolve last" |
| Bury | 2 | instant | move target item to the bottom of the stack | delays a threat past a Mend, or tucks your own item away from a counter |
| Raise | 1 | instant | move target item to the top | pulls your own item above theirs |
| Redirect | 2 | instant | flip target damage/heal item's aim to the other player | their Siege lands on them |
| Echo | 2 | instant | copy target (not an Echo) onto the top, owned by you | their Cleaver, aimed back at them |

## Why stack manipulation dominates

- The biggest numbers are sorcery-speed: they can only be played on an empty
  stack, so the opponent *always* gets priority with them on the stack, and
  every one of them is a single card an answer can turn around (Redirect, Echo)
  or erase (Negate). Raw power therefore sits exposed for one full priority
  window; the instants (1-2 mana) that answer it cost a fifth of a Siege.
- Mana carries across turns, so tapping out for a Siege on your own turn is
  paying everything for a card that loses to a 2-mana answer. Sequencing — what
  to cast, what to hold, in which order — is what separates the players.
- Order inside the stack changes outcomes without any card being bigger:
  `Mend` under `Cleaver` resolves after it; Flip moves it above; Bury sends a
  threat below your heal. The preview shows the resolved order and the life
  totals it leads to, so the skill is *reading* it.
- Asymmetry: the player's deck ("Weaver") is built from manipulation (two each
  of Flip, Bury, Raise, Redirect, Negate, Hush) and little raw damage; the NPC's
  ("Hammer") is heavy (Siege x2, Cleaver x3) with a few answers. A player who
  only plays damage loses to Hammer's bigger damage; one who plays the stack
  wins — `--verify` asserts exactly that over a set of seeds.

## The NPC

`npc::choose` is a pure function of the visible game state (no randomness):
it counters or redirects incoming damage of 4+, Hushes cheap spells that
matter, Echoes its own damage once the player has passed, heals when low,
and otherwise casts its biggest affordable sorcery. It plays the stack: it
holds mana open on your turn.

## Controls (keyboard)

`1`-`9` select a card in hand · `Enter` play the selected card (a targeted card
opens target choice: `Up`/`Down` cycle through the *legal* targets, marked on
the stack panel, `Enter` confirms, `Esc` cancels) · `Space` pass priority ·
`Enter`/`Space` on the end screen restarts.

## Decision surfaces (spec table), and what asserts them

See `tools/yakin/tasks/stack-card-game.md`. Surface: stack panel (every item,
top first, owner, effect, resolution order, projected life) with the hand beside
it and a priority indicator. Checks: `decision-1`, `decision-2`, `decision-3`
in `src/verify.rs`.

## Files

`src/rules.rs` the pure rules · `src/cards.rs` the pool and decks · `src/npc.rs`
the opponent · `src/main.rs` the ECS shell · `src/ui.rs` drawing and text ·
`src/verify.rs`, `src/checks.rs`, `src/capture.rs` the check · `mutants/` the
mutation round lists.
