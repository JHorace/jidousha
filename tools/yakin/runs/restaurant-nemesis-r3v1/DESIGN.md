# DESIGN — restaurant-nemesis-r3v1

task: restaurant-nemesis-r3v1
variant: V1
model-as-configured: claude-opus-5-5 (the session's configured model; configured, not verified — the run page is the authority)
date: 2026-10-07 21:55 PDT

## What the game is

Restaurant Nemesis is a turn-based restaurant sim told in panels: seven days of
service, four rounds a day, a kitchen too small for every order. A bad enough
failure turns an ordinary diner into a titled nemesis ("Mustard Monster") who
is extra picky about the thing you got wrong, comes back every day, and has
followers who show up as ordinary customers carrying the nemesis's demand.
Beat nemeses by pleasing them three times, by blowing them away once, or by
buying down their following; keep the money above zero through day 7 to win.

## The player-facing loop

1. **Ledger (before each day).** The money, tonight's rent, and one card per
   active nemesis: followers, tier, and the forecast for tomorrow — how many
   of tomorrow's 16 customers follow them and what demand they carry — with
   and without a takedown. Toggle spends with **1/2/3** (a takedown on nemesis
   1/2/3) and **T** (a temp cook). The margin line says what is left after
   spends and rent. **Enter** opens the day.
2. **Service (four rounds).** Each round four customers (plus any nemesis
   visiting this round) wait in the queue. Each row shows who they are
   (diner, follower of X, or nemesis), their demands per theme, the serve you
   have assigned and what it earns, and — always — what happens if the order
   goes unmet: the failure's theme, its severity, and whether it **spawns a
   nemesis** or feeds a following. **Up/Down** picks a row, **0/1/2/3** sets
   its serve (skip / standard / careful / all-out), the kitchen meter says how
   much of the round's capacity is used, **Enter** cooks the round.
3. While a nemesis's row is selected, the **nemesis card** on the right shows
   their title, theme, sensitivity, tier, followers, all three defeat paths'
   progress, and what each serve would do to them.
4. After the round the feed reports results and the nemeses' posts; after the
   fourth round rent is paid and the ledger opens. Money below zero after rent
   loses the run; finishing day 7 wins it. **Enter** on the end screen starts
   a new run on the next seed.

## Systems

All under `games/restaurant-nemesis-r3v1/`, crate `restaurant_nemesis_r3v1`.
The whole run is one `Game` value (`Clone`, owns its `Rng`); every rule is a
free function over it.

- Content — themes, title tables, names, post templates · `src/content.rs` ·
  none.
- Rules — `outcome(game, order, serve) -> Outcome` (**the one order-outcome
  function**: satisfaction, failure theme and severity, spawn or follower
  growth, money; the row preview and `cook_round` both read it);
  `progress(nemesis) -> Progress` and `visit_result(nemesis, serve) ->
  VisitResult` (**the one defeat-progress function** pair: the card prints
  them, the resolution applies `visit_result`); `forecast(game, nemesis,
  takedown) -> Forecast` (**the one follower function**: followers → tier →
  tomorrow's follower count and adopted demand; the ledger prints it and
  `open_day` generates customers from it) · `src/rules.rs` · `Rng`.
- Run state and steps — `Game`, `Order`, `Nemesis`, `Phase { Ledger, Service,
  Over }`; `open_day`, `cook_round`, `close_day`, spawning, posts · `src/sim.rs`
  · `Rng`.
- Screens — each screen is a `jidousha::ui::Panel` built by a pure function
  of the `Game` (`ledger_panel`, `service_panel`, `end_panel`) and drawn
  through one `Mapping`; backgrounds are `ctx.rect` · `src/screens.rs` ·
  `jidousha-ui.md`: `Panel`, `TextRun`, `Cell`, `clipped`, `wrap`,
  `Mapping`, `Floors`, `judge_panel`, `judge_frame`, `frame_text_floor`.
- Input — keys to `Command`s, applied by `apply(game, command)` · `src/main.rs`
  · `Input`, `Key`.
- Verify — players (good / first-timer / idle), the decision-row scenes, the
  floors, the mutation list, the capture · `src/verify.rs`, `src/scenes.rs`,
  `src/players.rs`, `src/checks.rs`, `src/capture.rs`, `mutants/r3v1.txt`.
- Assets: none — panels of text and flat rectangles only.

## The numbers

**Themes (4):** CONDIMENT, TEMPERATURE, WAIT, PORTION.

**Customers.** 7 days × 4 rounds × 4 customers = 16 a day, plus each active
nemesis visiting once a day. An ordinary diner has 1 theme demand (60%) or 2
(40%), distinct themes, each at level 1 (40%), 2 (35%) or 3 (25%), drawn from
the run's `Rng`. A follower is an ordinary diner whose demand in the leader's
theme is replaced by level `1 + tier`.

**Serves.** skip: quality 0, cost 0 · standard: quality 2, cost 1 · careful:
quality 4, cost 2 · all-out: quality 7, cost 4. Kitchen capacity: 5 per round
(+1 per round on a temp-cook day).

**Outcome.** Satisfied iff quality ≥ demand in every theme the order has.
Otherwise the failure's theme is the one with the largest shortfall (ties by
theme order) and its severity is that shortfall. Money: satisfied diner or
follower +$6, satisfied nemesis +$12; served but unsatisfied +$2; every
failure refunds `$2 × severity`.

**Spawn.** An ordinary diner's failure of severity ≥ 3 spawns a nemesis in
that theme (only a skipped level-3 demand gets there). A follower's failure
gives its leader +8 followers. A nemesis's failure gives them +25.
**At the cap (3 active)** a would-be spawn instead adds +20 followers to the
active nemesis with the most followers (their repost goes viral) and the feed
says so.

**Title.** From a per-theme table of four titles, index `rng.below(4)` at
spawn, skipping titles already active; a run that has used all four of a
theme's titles appends " II". **Sensitivity:** +2 to their demand in their
theme (a nemesis's order: their theme at `2 + 2 = 4`, plus one other theme at
level 1).

**Return schedule.** Every day after the spawn day, in round
`(spawn round + days since spawn) % 4`, until defeated.

**Followers and tiers.** A nemesis spawns with `15 + 10 × severity`
followers. Overnight (`close_day`) every active nemesis gains +10 (their
posts). Tier: 1 for 10–49, 2 for 50–99, 3 for 100+. Tomorrow's followers of a
nemesis = `2 × tier` of the 16 customers (2, 4 or 6), capped so all nemeses'
followers together are at most 8 of 16 (earlier-spawned nemeses first). Their adopted demand is
level `1 + tier` in the leader's theme (tier 3 = 4, which only careful or
better meets).

**Defeat paths** (`progress`):
1. *Tally* — satisfied on 3 visits (not necessarily in a row): defeated on the
   third satisfying visit.
2. *Overwhelming* — one visit whose quality beats their theme demand by 3 or
   more (all-out, 7 vs 4): defeated on that visit. Costs 4 of the round's 5.
3. *Following* — followers below 10 when the day opens: defeated that
   morning.
A defeated nemesis stops visiting; their followers are ordinary diners from
the next generated day.

**Money and viability.** Start $40. Rent $25 at each day's close. Spends at
the ledger: takedown $15 (−30 followers on one nemesis, one per nemesis per
night), temp cook $20 (+1 capacity every round tomorrow). Viability is money:
below $0 after rent → **lose** ("the bank changed the locks"). Close day 7
with money ≥ 0 → **win**.

**Inputs.** Ledger: 1/2/3 toggle takedown on nemesis 1/2/3; T toggles temp
cook; Enter opens the day. Service: Up/Down select; 0/1/2/3 set serve; Enter
cook. End: Enter new run.

## Gates to add

- Row 1 (which order gets capacity) — input: seed 3, day 1, the first round
  with an order whose unmet outcome spawns · asserts: the service frame shows
  that row's `if unmet:` line with the severity and `SPAWNS A NEMESIS` (found
  on the frame with `judge_frame`), the order is skipped, then a nemesis
  exists in that theme with a title from that theme's table and a theme
  demand of 4 on their next visit. · covers Done-when: a check per row.
- Row 2 × 3 (how to serve a nemesis) — input: staged nemesis in the queue
  with tally 2 (careful), tally 0 (all-out), followers 35 at the ledger
  (takedown) · asserts: the card's text before the act says that act defeats
  them, and the nemesis is defeated on exactly that visit / that morning, not
  before. · covers: a check per row.
- Row 3 (spend) — input: staged ledger with one nemesis at 80 followers, two
  runs differing only in one takedown · asserts: each run's ledger preview
  (followers, tier, follower count, adopted level) is in the transcript, and
  the next day's generated customers carry exactly that many followers of
  that nemesis at exactly that level. · covers: a check per row.
- Full runs — good player wins on seed 3; idle player loses (viability); the
  first-timer's ending is printed; a run has ≥ 2 nemeses active on one day.
  · covers: multi-day run, win and loss, 2+ concurrent.
- Floors — every screen's panel through `judge_panel` (min text 12, chrome
  inside 960×540), `judge_frame` on the recorded frames, `frame_text_floor`,
  nothing off camera, every string printable ASCII.
- Mutation round — `tools/mutate restaurant_nemesis_r3v1 mutants/r3v1.txt`.
- Web — build-web and serve-web --check.

## Non-goals

Everything in the spec's Non-goals; plus pointer/touch input (keyboard only),
animation, art files, sound, a second sink beyond the temp cook, reputation
as a second viability axis, and save/load.

## Decisions already made

Every number above. Keyboard only. Screens are `jidousha::ui::Panel`s in a
960×540 design space mapped 1:1 to world units (camera height 540 centred on
the design rect).

## Open calls delegated to the implementer

- Tuning (start money, rent, capacity) inside the constraint: the good player
  wins seed 3 and the idle player loses; record any change as a deviation.
- Exact wording of posts and complaints — silly, ASCII only.
