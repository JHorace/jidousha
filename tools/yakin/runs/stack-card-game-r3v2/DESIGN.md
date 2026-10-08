# DESIGN — stack-card-game-r3v2

task: stack-card-game-r3v2
variant: V2
model-as-configured: claude-fable-5-1 (the session's context states this as the configured model, with Opus fallbacks; not verified — only the run page says what served)
date: 2026-10-07 21:12 PDT

The base spec is `tools/yakin/tasks/stack-card-game.md`, read through the
overrides in `tools/yakin/tasks/stack-card-game-r3v2.md`. This document is the
V1 design note the base asks for *and* the V2 handoff: the rules, the card
pool, why stack manipulation dominates, and everything an implementer with no
memory of this tick needs. Game id `games/stack-card-game-r3v2/`, crate
`stack_card_game_r3v2`. Engine surface: `docs/api/` only (all five files) and
`crates/jidousha/examples/` — never `crates/*/src/`.

## What the game is

A two-seat card duel, You against a scripted Rival, in which every card played
goes onto one shared stack that resolves top-first only when both seats pass in
a row. Half of each deck is cards that act on the stack itself — void an item,
push it to the bottom, flip whom it hits, copy it — and those cards are cheap,
so the winner is the seat that reads what the stack *will* do and plays the
right card at the right moment, not the seat with more damage. One match, up to
twelve turns, ends on a result screen.

## The player-facing loop

1. The screen is one table: a header with the turn and whose turn it is, two
   seat lines (life and energy for You and the Rival), the **stack panel** on
   the left listing every item top-first with what each will do when it
   resolves, the **hand panel** on the right listing your cards numbered 1-6
   with cost and a one-line blurb, a **priority line** under the stack saying
   who holds priority and what a pass would do, and a three-line log of what
   last resolved.
2. When you hold priority you press `1`-`6` to play that hand slot (if you can
   afford it and it has a legal target) or `Space` to pass. An effect card
   (Bolt, Blast, Mend) goes straight onto the stack. A stack card (Counter,
   Delay, Redirect, Copy) opens **choosing**: legal stack items are marked,
   `Up`/`Down` move the cursor among them, `Enter` commits, `Esc` cancels.
3. Every play puts the card on top of the stack, refreshes the forecast column
   ("will: ...") for every item, and hands priority to the Rival. The Rival
   thinks for 45 ticks (so you can read the stack), then responds or passes;
   its play appears on the stack and priority returns to you.
4. When both seats pass in succession the top item resolves: life totals move,
   the log gains a line, the item leaves the stack, and priority returns to the
   active seat. When both pass on an empty stack the turn ends: both seats'
   energy refills to 3, both draw a card, the other seat becomes active.
5. The match ends when a seat's life reaches 0 or below, or at the end of turn
   12. A banner says `YOU WIN`, `RIVAL WINS` or `DRAW` with both life totals,
   over the final table; `Enter` starts a new match from the same seed.

## Systems

In build order. Every path is under `games/stack-card-game-r3v2/`, the spec's
Fence; the crate layout is `examples/prototype_kit`'s (make-game §A.2): one
`Cargo.toml` naming `jidousha = { path = "../../crates/jidousha" }` and nothing
else, `[lints] workspace = true`, and `#![allow(missing_docs)]` at the top of
`src/main.rs`.

- cards — the seven `Card` kinds with `cost()`, `name()`, `blurb()` (the hand's
  one-liner), `is_effect()`, and the two deck constants `YOUR_DECK` and
  `RIVAL_DECK` (Decisions, "The pool") · `src/cards.rs` · touches nothing in
  `docs/api/` (plain data).
- rules — `Side`, `Item`, the stack, and the three pure functions every other
  system calls: `legal_targets(card, stack) -> Vec<ItemId>`, `forecast(duel)
  -> Forecast`, and `resolve_top(duel)` which applies `forecast(duel).steps[0]`
  (Decisions, "One function per decision row") · `src/rules.rs` · touches
  nothing in `docs/api/`.
- duel — `Seat` (life, energy, hand, deck), `Duel` (seats, stack, turn, active
  seat, priority holder, passes in a row, next item id, phase `Live |
  Over(Outcome)`, the `log`), `Duel::new(&mut Rng)` (shuffle and deal) and
  `Duel::apply(Action) -> Result<(), Illegal>` (play, pass, resolution, turn
  end, match end — Decisions, "The rules") · `src/duel.rs` · touches
  `jidousha-api.md`: `Rng` (`below`) for the shuffle. Unit tests for the rules
  live beside the code they test (Gates, G10).
- players — `rival_action(&Duel) -> Action` (the NPC), and the three scripted
  check players `sequencer_action`, `brute_action`, `nothing_action`
  (Decisions, "The Rival" and "The sequencer") · `src/players.rs` · touches
  nothing in `docs/api/` (pure functions over `Duel`).
- flow — the one game resource `Flow { duel: Duel, choosing:
  Option<Choosing>, rival_wait: u32, seed: u64 }`, the `Startup` system
  `set_the_table` (takes a `Flow` a harness inserted before tick 1, else builds
  one from `Rng`), and the two `Update` systems `read_player_input` (keys ->
  `Action`, the choosing state) and `let_the_rival_act` (the 45-tick wait, then
  `rival_action`) · `src/main.rs` · touches `jidousha-api.md`: `run`,
  `headless`, `GameConfig` (`seed`, `title`, `window_size`), `App::add_system`,
  `Startup`, `Update`, `Draw`, `World` (`insert_resource`, `find_resource`,
  `resource`, `resource_mut`), `Input` (`just_pressed`), `Key` (`Digit1`..
  `Digit6`, `ArrowUp`, `ArrowDown`, `Enter`, `Escape`, `Space`), `Camera`,
  `Time` (nothing is timed by seconds; the Rival's wait counts ticks).
- screen — `layers`, `palette`, the layout constants, the empty `Art` icon
  enum, the `Flat` mapping, `screen(&Flow) -> Panel<Art>` (the one reader the
  draw system, the floors and the frame check all take), and the two `Draw`
  systems `draw_table` (the background rects) and `draw_chrome`
  (`Panel::draw`) · `src/screen.rs` · touches `jidousha-ui.md`: `Panel`
  (`text`, `all_strings`, `lifted`, `draw`), `TextRun`, `Cell`, `Icon`,
  `IconRun` (type only), `Mapping`, `clipped`; `jidousha-api.md`: `DrawCtx`,
  `Submit` (`rect`, `text` via the kit), `TextStyle` (`width_of`), `Rect`,
  `Color`, `Depth`, `Camera` (`visible_bounds`).
- verify — the `--verify` mode: the gates below, the three players' verdict
  lines, the `verified` line and summary, then one frame transcript ·
  `src/verify.rs` and `src/checks.rs` · touches `jidousha-testing.md`:
  `FrameRecorder` (`new`, `draw`, `font_texture`), `FrameRecord` (`quads`,
  `covering`, `transcript`, `plan`), `DrawnQuad` (`bounds`, `texture`),
  `find_bounds`, `SnapshotBuilder` (`record`, `first_tick_snapshot`),
  `InputEvent` (`KeyPressed`, `KeyReleased`), `InputSnapshot` (`new`),
  `HeadlessSim` (`tick`, `world`, `world_mut`, `schedule_debug`), `FramePlan`
  (`clear_color`); `jidousha-ui.md`: `Floors`, `judge_panel`, `judge_frame`,
  `frame_text_floor`, `glyph_run`, `Breach`.
- capture — one PNG of the three-item priority window at 480x270 ·
  `src/capture.rs` · touches `jidousha-capture.md`: `WgpuBackend`
  (`offscreen`, `poll`, `is_ready`), `RenderBackend` (`render`, `capture`),
  `create_builtin_textures`, `TextureTable`, `FONT_TEXTURE`, `encode_png`,
  `RenderError` (`NoAdapter` is the only skip), `RawImage`. The shapes-and-text
  path: `examples/prototype_kit/capture.rs` minus its asset lines.
- mutants — `mutants/round1.txt`, the faults of G12, run with `tools/mutate`.

- Assets: none. Every shape is `ctx.rect`; every string is the built-in face.

## Gates to add

All in `--verify` unless marked *test* (a `#[cfg(test)]` function named as a
sentence, run by `cargo test -p stack_card_game_r3v2`). Every assertion
reports the numbers it judged; failures are collected, not exited on. Seeds
below are `GameConfig::seed` values. "Staged" means a `Duel` built by hand and
inserted as a `Flow` before tick 1 (jidousha-testing.md, "the window between
building a game and running it"). The summary under `verified` carries one
line per gate, and the three decision-row gates are named `decision 1`,
`decision 2`, `decision 3` so the report shows a check per row.

- G1 bounds — input: every frame drawn below · asserts: every quad inside
  `Camera::visible_bounds()` of the camera rebuilt with the headless viewport,
  and prints the closest-edge clearance · covers Done-when: verify reports pass.
- G2 floors — input: the live frame of G3 and the result frame of G6 ·
  asserts: `judge_panel(screen, FLOORS, &[], &[])` returns no breach;
  `judge_frame` finds every row of `screen(&flow)` on the frame; `frame_text_floor(.., 12.0)` is empty; every string of `all_strings()` is printable ASCII (`' '..='~'`); no live-screen row ends in `...` (a clipped forecast is a hidden fact) · plus one staged panel with two rows two units apart, whose `judge_panel` breach is `"two rows of chrome text overlap"` by name · covers Done-when: verify pass.
- G3 decision 1 (respond, and with what) — input: staged, seed irrelevant: turn 3, You active and holding priority, Rival energy 0, hand You = [Counter, Bolt, Redirect, Mend], stack top-first = #3 Counter (Rival, target #1), #2 Blast (Rival, affects You), #1 Bolt (You, affects Rival); life 15 / 15 · asserts: `forecast` has two steps in the order #3 then #2 (`Counter: voids #1`, `Blast: You 15 -> 10`), final life You 10, Rival 15, and its `says` column reads top-first `Counter: voids #1`, `Blast: You 15 -> 10`, `voided by #3` (shipped literals, not arithmetic over the constants); the stack panel's rows are in that order top-first, each carrying the item's id, card, owner and its `says` line, and `glyph_run` finds every character of every row on the frame; the hand panel marks all four cards playable (Counter and Redirect have targets, #2 and #1 for Redirect); then the script presses `Space` and, since the Rival at 0 energy must pass, the next resolution is exactly `steps[0]` (same item, same life after, same stack after), and so on until the stack is empty — the sequence of actual resolutions equals the forecast taken at the window, step by step · covers Done-when: "a check for each decision row" (row 1).
- G4 decision 2 (pass) — input: the staged duel of G3 after one `Space` from You (passes = 1) and the Rival's forced pass · asserts: while passes = 1 and You hold priority the priority line reads `YOU HOLD PRIORITY - Rival passed: Space resolves #3`; while passes = 0 it reads `YOU HOLD PRIORITY - Rival may still respond`; on an empty stack with passes = 1 it reads `... Space ends the turn`; after the resolving `Space` the life totals and the stack equal `forecast.steps[0].life_after` and `.stack_after` exactly, and the log's newest line equals that step's outcome line · covers Done-when: row 2.
- G5 decision 3 (target) — input: staged, You hold priority on your turn with Delay as slot 1 and Redirect as slot 2, stack top-first = #3 Counter (Rival, target #2), #2 Bolt (You, affects Rival), #1 Mend (Rival, affects Rival), Rival energy 0, life 15 / 15 · asserts: pressing `2` (Redirect) marks exactly the effect items #2 and #1 (their mark cells read `>` for the cursor row #2 and `+` for #1; #3 reads ` ` and is `DIM`) and the marked set equals `legal_targets(Redirect, stack)`; `Esc` clears every mark; pressing `1` (Delay) marks all three with the cursor on #3; `Down` three times wraps the cursor back to #3; `Enter` commits Delay on #3 — the forecast now reads top-first #4 `Delay: #3 to bottom`, #2 `Bolt: Rival 15 -> 12`, #1 `Mend: Rival 12 -> 15`, #3 `Counter: fizzles`; after two `Space`s (the Rival passing) Delay has resolved and the panel shows top-first #2, #1, #3, which equals `forecast.says`' order and equals the order the three then actually resolve in under further passes, with life ending You 15, Rival 15 · covers Done-when: row 3.
- G6 the match — input: seed 7, one headless sim driven through the keyboard (`SnapshotBuilder`, one key event per tick, `keys_for(action)` translating the sequencer's `Action` into slot digit, `Down` presses and `Enter`) with the Rival playing its own seat through `let_the_rival_act` · asserts: the match reaches `Over(..)` before tick 6000; the result frame carries the banner, both life totals and `Enter: play again`; `Enter` then yields a `Live` duel at turn 1 with 4 cards in each hand · covers Done-when: "one full match against the scripted NPC plays end to end, and a scripted --verify run reaches its result screen".
- G7 three players — input: seeds 1..=12, `Duel` driven directly by `apply` (no sim; the Rival's wait is irrelevant), each of `nothing_action`, `brute_action`, `sequencer_action` on Your seat against `rival_action` · asserts, one verdict line each, in the controllers document's shape: nothing wins 0 of 12; brute wins strictly fewer than sequencer; sequencer wins at least 7 of 12; and prints the three controller numbers for the sequencer (plays made, responses made, forecast margin at each play averaged) · covers the brief: "raw card power should lose to good sequencing".
- G8 the screens the run never reaches — input: staged `Over(RivalWins)`, `Over(Draw)`, a full 10-item stack, a 6-card hand, and the choosing state of G5 · asserts: G1 and G2 over each; the two end banners differ from each other and from `YOU WIN` (name the pair); the banner's glyphs are what `covering(banner centre)[0]` returns (the band test) · covers Done-when: verify pass.
- G9 order of systems — input: `schedule_debug()` · asserts: `set_the_table` is in Startup; in Update `read_player_input` precedes `let_the_rival_act`, both found (two `None`s compare equal — assert both are `Some`) · covers Done-when: verify pass.
- G10 contracts (*test*, in `src/rules.rs` and `src/duel.rs`) — one sentence each: a counter whose target already resolved fizzles and changes no life; redirect lists only effect items as legal; delay moves its target to the bottom and nothing else moves; copy of a bolt owned by the rival makes a bolt owned by you that hits the rival; a stack of ten refuses an eleventh play; both passing on an empty stack ends the turn and refills both energies to three; a draw into a hand of six is skipped; the forecast of an empty stack is empty; the forecast stops at the step that ends the match; the same seed deals the same hands twice.
- G11 clear colour — input: any frame · asserts: `frame.plan.clear_color == palette::TABLE` and, independently, its brightest channel is below 0.25 at alpha above 0.99.
- G12 the mutation round — `mutants/round1.txt`, every fault noticed: forecast resolves bottom-first; Bolt deals 4; starting life 20; energy refills to 4; two passes become one; Delay moves its target to the top; Redirect's legal list includes stack cards; the Rival always passes; the stack panel lists bottom-first; the priority line always says `Rival passed`. Report `N of N noticed` in the PR.
- G13 capture — the G3 window at 480x270, `capture: <path> written to ...` worded as jidousha-capture.md gives it; `NoAdapter` skips with a summary line and nothing else does · covers Done-when: the captured PNG in the report.

The remaining Done-when lines are the gate commands themselves: `python3
tools/verify stack_card_game_r3v2` (pass, with the three decision lines),
`python3 tools/build-web stack_card_game_r3v2 && python3 tools/serve-web
stack_card_game_r3v2 --check`, the `git diff --stat` fence, and the PR.

## Non-goals

- Pointer and touch input: keyboard only. The kit's `judge_panel` gets no
  controls because there are none to click.
- Sound, art, a loaded typeface, icons: the `Art` enum is empty.
- Any card beyond the seven; deck building; mana growth; a hand bigger than
  six; a stack deeper than ten; a best-of series; an undo.
- A Rival that looks ahead. Its rule is one greedy function on purpose
  (Decisions, "The Rival"): the skill tested is reading that rule off the
  stack, and a smarter Rival would need its own tuning round this window does
  not have.
- `Time::alpha` interpolation: nothing moves between ticks.
- The kit's feed, meters, chips and class table: the log is three `TextRun`s
  over `Flow::duel.log`; nothing pauses the world.
- Engine changes: none needed. The one doc gap met (`Flat` in the UI doc's
  `judge_frame` example is never defined) is `FINDINGS.md` entry 1 and is
  designed around by defining the identity mapping in `src/screen.rs`.

## Decisions already made

### The rules

- Two seats, `Side::You` and `Side::Rival`. Life starts at 15 (no cap on
  healing). Energy is 3 per seat, set to 3 for **both** seats at the start of
  every turn and never carried over.
- Each seat has its own 16-card deck (below), shuffled once at match start by
  Fisher-Yates over the engine `Rng` (`rng.below(n)`), Your deck first, then
  the Rival's. Opening hand 4 cards each, drawn from the top. Hand cap 6: a draw
  into a full hand is skipped and the card stays on the deck. A draw from an
  empty deck draws nothing.
- Turns 1..=12. You are active on odd turns, the Rival on even. At the start of
  a turn both energies refill and both seats draw one card, active seat first.
  The active seat holds priority over an empty stack with passes = 0.
- The holder of priority either **plays** or **passes**. A play requires
  `cost <= energy`, a stack of fewer than 10 items, and — for a stack card — a
  legal target; it takes the card from the hand, pays the energy, pushes an
  `Item` on top of the stack (ids count up from 1 within a match), sets passes
  to 0 and gives priority to the other seat. A pass adds 1 to passes and gives
  priority to the other seat; when passes reaches 2: a non-empty stack resolves
  its top item (one item only), passes resets to 0 and priority goes to the
  **active** seat; an empty stack ends the turn.
- After every resolution, if a seat's life is 0 or below the match is over
  (the rest of the stack is discarded unresolved): the other seat wins; both
  at or below 0 is a draw. After turn 12 ends, the higher life wins; equal is
  a draw. `Over(Outcome)` with `Outcome::{YouWin, RivalWins, Draw}` and the
  final life totals; `Enter` rebuilds the match from the same seed.
- `Item { id, card, owner, affects: Side, target: Option<ItemId> }`. `affects`
  is set at play: Bolt and Blast affect the owner's opponent, Mend affects the
  owner; stack cards carry `affects = owner` and it means nothing for them.
- Resolution of the top item, exactly: Bolt — `affects` loses 3. Blast —
  `affects` loses 5. Mend — `affects` gains 3. Counter — removes its target
  from the stack. Delay — moves its target to the bottom of the stack. Redirect
  — flips its target's `affects` to the other seat. Copy — pushes a new item on
  top of the stack with a fresh id: the same card, owner = Copy's owner,
  `affects` re-derived for that owner (so your copy of the Rival's Bolt hits
  the Rival), the same `target`. A stack card whose target is not in
  `legal_targets(card, stack without itself)` at resolution **fizzles**: it
  leaves the stack and does nothing. Life changes apply at once.
- Every resolution appends one outcome line to `duel.log` (kept whole; the
  screen shows the last three).

### The pool

Seven cards. The two decks are deliberately unequal, and that inequality is
the experiment the base spec asks for: raw damage in the Rival's deck is 35
(5 Bolt, 3 Blast) against 14 in yours (3 Bolt, 1 Blast), and your deck wins
anyway when sequenced.

| card | cost | effect | hand blurb | in your deck | in the Rival's |
|---|---|---|---|---|---|
| Bolt | 1 | opponent loses 3 | `3 to the other seat` | 3 | 5 |
| Blast | 2 | opponent loses 5 | `5 to the other seat` | 1 | 3 |
| Mend | 1 | you gain 3 | `3 back to you` | 1 | 2 |
| Counter | 1 | target item leaves the stack unresolved | `voids an item` | 4 | 4 |
| Delay | 1 | target item moves to the bottom | `sends an item to the bottom` | 3 | 0 |
| Redirect | 1 | target effect item hits the other seat instead | `flips whom an item hits` | 2 | 2 |
| Copy | 2 | a copy of the target, yours, on top | `copies an item, as yours` | 2 | 0 |

`YOUR_DECK` = 3 Bolt, 1 Blast, 1 Mend, 4 Counter, 3 Delay, 2 Redirect, 2 Copy
(16). `RIVAL_DECK` = 5 Bolt, 3 Blast, 2 Mend, 4 Counter, 2 Redirect (16).
Legal targets: Counter, Delay and Copy target any item on the stack; Redirect
targets only effect items (Bolt, Blast, Mend); effect cards have no target
and never open choosing.

### Why stack manipulation dominates

- **Price.** Every stack card costs 1 and acts on a card that cost 1 or 2. A
  Counter on a Blast trades 1 energy for 2 and 5 life; a Redirect on a Blast
  is a 10-life swing for 1 energy; a Copy of a Blast is 5 damage for 2 energy
  without owning a Blast. No effect card can match a swing per energy of
  more than 2.5.
- **Order.** Resolution is last-in-first-out, so the seat that plays *last*
  before the passes decides what happens. A Counter aimed at a Bolt is itself
  a target: Counter the Counter and the Bolt lands; Delay the Counter below
  its Bolt and it fizzles for the loss of a Delay. Reading the order is the
  whole game, and the forecast column shows it without hiding it — the skill
  is choosing, not computing.
- **The Rival's rule is readable.** It answers the single biggest threat on
  the stack with Counter first and never saves an answer (below), so a cheap
  Bolt in front of a Blast draws its Counter, and the Blast behind it lands
  once its energy is gone. Three energy a turn is three answers at most; the
  sequencer wins by making the Rival spend them on the wrong items.
- **Raw power loses on its own.** The brute (plays its biggest damage card
  every turn, never responds) walks every Bolt into a Counter or a Redirect
  and takes every Blast the Rival plays; G7 asserts it wins fewer seeds than
  the sequencer, which plays the same deck.

### One function per decision row

The spec's decision-surface table is incorporated whole; this elaborates each
row and invents no surface.

| decision | one function | the surface reads it | the sim reads it | asserted by |
|---|---|---|---|---|
| respond, and with which card | `rules::forecast(&Duel) -> Forecast`; `Forecast { steps: Vec<Step>, says: Vec<(ItemId, String)> }` with `Step { item: ItemId, card, says: String, life_after: [i32; 2], stack_after: Vec<ItemId> }`: the stack resolved top-first with no further plays, stopping at the step that ends the match; `says` has exactly one line per item on the stack, top-first — the item's step line, or `voided by #n`, or `not reached` | the stack panel's forecast column prints the item's `says` line beside each item, top-first; the hint row prints what the chosen card would change by calling `forecast` on a copy with that card applied | `rules::resolve_top` applies `forecast(duel).steps[0]` — same stack after, same life after, same log line | G3 |
| pass priority | the same `forecast`, plus `Duel::passes` and `Duel::priority` | the priority line: `Rival may still respond` while passes = 0; `Rival passed: Space resolves #<top id>` while passes = 1 and the stack is non-empty; `Rival passed: Space ends the turn` on an empty stack | `Duel::apply(Action::Pass)` | G4 |
| where to aim a counter, delay, redirect or copy | `rules::legal_targets(card, &[Item]) -> Vec<ItemId>` | choosing marks every id in the list (`+`, cursor `>`), draws the rest dim, and the cursor only visits the list | play refuses a target not in the list; resolution fizzles an item whose target is not in `legal_targets(card, stack without itself)` | G5 |

The hint row for a stack card while choosing: `Counter on #3: ` followed by
the first step's `says` of the forecast with that play applied. For an effect
card the hint row shows `Bolt: Rival 15 -> 12` (the forecast with it applied).

### The Rival

`players::rival_action(&Duel) -> Action`, a pure function, first rule that
applies:

1. **Stack non-empty** (any turn): let `margin(d)` = Rival life minus Your life
   at the end of `forecast(d)`. For each item `i` compute `margin` of the duel
   with `i` removed; the **threat** is the item whose removal raises `margin`
   the most, ties to the top-most, and only if the rise is positive. No
   threat → Pass. Threat → play Counter on it if Counter is in hand and
   affordable; else Redirect on it if the threat is an effect item and
   Redirect is affordable; else Pass.
2. **Own turn, empty stack**: Blast if affordable, else Bolt if affordable,
   else Mend if affordable and Rival life ≤ 9, else Pass.
3. **Otherwise** (Your turn, empty stack): Pass.

It acts `RIVAL_THINK_TICKS = 45` ticks after receiving priority
(`Flow::rival_wait`), in the window and headless alike. Its documented
weaknesses, which the hand panel's blurbs do not spell out but the log makes
visible: it never Delays or Copies (it owns none), never plays an effect card
off-turn, never saves an answer, and judges a stack only by its current
forecast — so a Counter you aim at its Blast is a threat it answers with a
Counter of its own if it has one, which you can then Counter back.

### The sequencer, the brute and the nothing player

Check players only (`src/players.rs`), each `fn(&Duel) -> Action`:

- `nothing_action`: always Pass. Proves the match can be lost.
- `brute_action`: on its own turn with an empty stack, play the highest-damage
  affordable effect card (Blast, Bolt), else Mend if affordable, else Pass;
  never respond. Raw power with no reading.
- `sequencer_action`: enumerate Pass and every legal play (each affordable
  playable card × each legal target). Score each by applying it to a copy,
  then applying one `rival_action` reply if the Rival then holds priority,
  then `score = 10 * (Your life - Rival life at the end of forecast) + 2 *
  (energy the Rival spent in that reply) - (energy this action costs)`. Take
  the highest; ties go to the action that leaves the Rival with fewer cards
  in hand, then to Pass, then to the lowest slot and lowest target id. This
  is "run the opponent's own rule forward" from jidousha-controllers.md.

G7's thresholds (nothing 0, brute < sequencer, sequencer ≥ 7 of 12) are the
requirement. If the sweep misses them, change the sequencer's scoring or the
Rival's rule 2 thresholds — never the pool, the costs or the stack rules — and
record it under Deviations.

### Controls

`Digit1`..`Digit6` play that hand slot; `ArrowUp`/`ArrowDown` move the
choosing cursor through the legal list, wrapping; `Enter` commits;
`Escape` cancels choosing; `Space` passes; `Enter` on the result screen
restarts. Keys are read with `just_pressed` only, and ignored while the Rival
holds priority. `Flow::choosing` is `Option<Choosing { slot: usize, cursor:
usize }>` — one value, never a flag beside it (jidousha-ui.md).

### Layout, in constants

- `WINDOW = PhysicalSize::new(1280, 720)`; camera `center (480, 270)`, `height
  540`, `clear_color palette::TABLE`, `viewport` left to the driver; the
  headless recorder uses `WINDOW`. So **one world unit is one design unit** and
  the kit's 960x540 design rect is the visible world; the `Flat` mapping is
  `to_world(ui) = ui`, `scale() = 1.0`.
- `FLOORS = Floors { min_text: 12.0, chrome: 0..960 x 0..540, world: the
  same }`. Text sizes: `ROW = 14.0`, `TITLE = 18.0`, `BANNER = 32.0`. Row
  leading 20.
- `layers`: `TABLE = 0` (background rects), `CHROME = 1` (every panel row),
  `BANNER_BACK = 2` (the result backing rect), `BANNER = 3` (the result rows,
  `lifted`).
- `palette`: `TABLE = Color::rgb(0.05, 0.06, 0.10)`, `PANEL = rgb(0.10, 0.12,
  0.18)`, `INK = WHITE`, `DIM = rgb(0.45, 0.47, 0.55)`, `YOURS = rgb(0.45,
  0.90, 1.0)`, `THEIRS = rgb(1.0, 0.50, 0.45)`, `MARK = rgb(1.0, 0.85, 0.25)`.
- Header row y 12: `TURN 3 of 12 - YOUR TURN` / `- RIVAL'S TURN` at x 20,
  `TITLE`. Seat rows y 36, `ROW`: `YOU    life 15   energy 3` at x 20 in
  `YOURS`; `RIVAL  life 15   energy 3` at x 580 in `THEIRS`.
- Stack panel: rect x 20..560, y 64..396 in `PANEL`; title `STACK - top
  resolves first` at (28, 70) `TITLE`; rows from y 96, cells as offsets from
  the row: mark `Cell { at: (8, 0), width: 14 }`, id `(22, 0) w 36` (`#10`),
  card and owner `(60, 0) w 170` (`Redirect  Rival`), target `(232, 0) w 58`
  (`-> #10`), forecast `(292, 0) w 240` (`step.says`). Owner colours the row
  `YOURS`/`THEIRS`; while choosing, illegal rows are `DIM` and legal rows
  `MARK`. Empty stack: one row `(empty - nothing to resolve)` in `DIM`.
- Priority line at (28, 408), `ROW`, `INK`: the three texts of the decision
  table, or `RIVAL HOLDS PRIORITY - thinking` while it waits, or `CHOOSE A
  TARGET for Counter - Up/Down, Enter, Esc` while choosing.
- Hand panel: rect x 580..940, y 64..396 in `PANEL`; title `HAND - 1-6 plays,
  Space passes` at (588, 70); rows from y 96: slot `(8, 0) w 20`, name `(30,
  0) w 90`, cost `(124, 0) w 20`, blurb `(150, 0) w 200`; playable rows `INK`,
  others `DIM`. The hint row at (28, 500) `ROW` `MARK` while choosing or when a
  slot is playable-with-preview, else the controls line `1-6 play   Up/Down
  target   Enter commit   Esc cancel   Space pass` in `DIM`.
- Log: three rows at y 432, 450, 468, x 28, `ROW`, `DIM`, newest last.
- Result: rect x 200..760, y 196..332 on `BANNER_BACK` in `TABLE` colour;
  `YOU WIN  12 - 0` (or `RIVAL WINS  0 - 7`, `DRAW  3 - 3`; Your life first)
  centred by `width_of` at y 220 `BANNER`; `Enter: play again` centred at y
  284 `TITLE`; both on `BANNER` via `lifted`. The live table stays drawn
  beneath.
- Outcome lines (`Step::says`), ASCII, at most 22 characters: `Bolt: Rival 15
  -> 12`, `Blast: You 15 -> 10`, `Mend: You 12 -> 15`, `Counter: voids #3`,
  `Delay: #1 to bottom`, `Redirect: #2 -> Rival` (now affects the Rival),
  `Copy: #2 as #6`, and `<card>: fizzles` for any stack card with no target.

### Determinism and the house pattern

- All randomness is the engine `Rng` at match start; nothing reads the clock;
  the Rival's wait counts ticks. Same seed, same match — G10's last test.
- `Duel` is plain `Clone` data with no world in it, so `forecast`, the Rival
  and the sequencer roll copies forward (jidousha-api.md, "Write the two
  decisions a check will want as free functions").
- Update order is `read_player_input` then `let_the_rival_act`, asserted by
  G9. Draw order is `draw_table` then `draw_chrome`; bands, not order, put
  text over rects.
- `Startup` takes a `Flow` already in the world (a harness staged it) or builds
  one from `Rng` and `GameConfig::seed`, as jidousha-testing.md's `Tuning`
  pattern does.

## Open calls delegated to the implementer

- Exact wording of the hand blurbs and the log lines beyond what the tables
  above fix, provided every string is ASCII, fits its cell unclipped on the
  live screens (G2), and the `Step::says` forms keep the card name, the seat
  and the before/after life.
- The internal representation of the hand (a `Vec<Card>` indexed by slot is
  enough) and of `Flow::rival_wait`.
- `keys_for(action, &Duel) -> Vec<Key>` in verify: the digit, then one `Down`
  per step from the first legal target to the chosen one, then `Enter`; one
  key event per tick, release the tick after.
- Whether the sequencer's one-ply reply also applies the Rival's rule 2 (an
  own-turn play) — allowed either way; say which in a comment.
- The seed sweep count may rise above 12 if it runs under a second; the
  thresholds scale with it (nothing 0, brute < sequencer, sequencer ≥ 7/12 of
  the count).
- Any constant not fixed above (panel inset, log leading) within the floors.
