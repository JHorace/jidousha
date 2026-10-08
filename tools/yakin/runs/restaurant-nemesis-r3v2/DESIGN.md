# DESIGN — restaurant-nemesis-r3v2

task: restaurant-nemesis-r3v2
variant: V2
model-as-configured: claude-fable-5-1 (the session's context states this as the configured model, with fallbacks claude-opus-5-5[1m] then claude-opus-5[1m]; not verified — only the run page says what served)
date: 2026-10-07 21:22 PDT

Spec: `tools/yakin/tasks/restaurant-nemesis-r3v2.md`, which wraps
`tools/yakin/tasks/restaurant-nemesis.md`. Every number the base spec's
§"The design" asks for is fixed in §Decisions already made, in one table.
The implementer reads this file, `docs/api/` (all five) and
`crates/jidousha/examples/` and nothing else of the engine (make-game §0.1).

## What the game is

A silly, turn-based restaurant sim: each day the player spends five units of
kitchen capacity across six customers whose demands outrun it, so some orders
fail whatever they do. A failure bad enough spawns a titled problem customer
("Mustard Monster") who is harder to satisfy in that theme, comes back every
second day, and whose social-media followers turn tomorrow's ordinary customers
into themed copies of them. The player wins by surviving seven days with money
and reputation intact, defeating nemeses by serving them twice, overwhelming
them once, or ratioing their followers to zero with between-days spending.

## The player-facing loop

Everything happens on a key press; nothing moves between presses. Three
screens, one `Phase` value:

1. **Service (day d of 7).** The queue is listed one order per two rows:
   returning nemeses first (rows 1..k), then the six ordinary customers. Each
   row states, *while the player is choosing*, what serving pays and what
   leaving it unmet costs — the failure's theme, its severity, and whether that
   failure `SPAWN`s a nemesis, `feeds` an existing one (cap full), or does
   nothing. The header shows day, money, reputation, capacity left.
   - `Digit1..Digit9` on an ordinary row: serve it, if capacity left ≥ its
     need. Money and reputation change at once; the row flips to `SERVED`. Not
     enough capacity: the row is unchanged and a notice line says
     `not enough capacity: needs 2, 1 left` (a refused action is shown, never
     silent).
   - `Digit1..Digit9` on a nemesis row: opens the **nemesis card** over the
     queue (`Phase::Service { focus: Some(row) }`). The card shows their title,
     theme, sensitivity, tier and followers, and their progress on all three
     defeat paths with what each costs *now*. `S` serves them (3 units, banks
     one toward the tally), `O` overwhelms them (5 units, defeats them at
     once), `Escape` closes the card. Either serve key also closes it.
   - `Enter` closes the kitchen: every unserved order is resolved unmet, in
     queue order (penalties, spawns, follower growth), nightly effects apply,
     then the viability check runs. Day 7 passing → **End (won)**; any day
     failing → **End (lost)**; otherwise → **Ledger**.
2. **Ledger (night d).** Shows the night's log (what was served, what failed,
   who spawned), money and reputation against their lose lines, each active
   nemesis with their followers, tier, tomorrow's follower share, and exactly
   what one social-media spend would do to all three — and the six customers
   drawn for tomorrow with follower marks.
   - `Digit1..Digit3`: spend $25 on that nemesis (rows in spawn order) — cut
     15 followers; at 0 they are defeated on the spot. Refused with a notice
     when money < 25.
   - `P`: prep, $15 for +1 capacity tomorrow, at most twice a night.
   - `Enter`: open tomorrow — the queue is built and day d+1's Service shows.
3. **End.** `SURVIVED 7 DAYS - THE REVIEWS ARE MIXED` or
   `CLOSED DOWN - <reason>`, with the run's tally (days, money, rep, nemeses
   spawned/defeated, the titles). `Enter` starts a fresh run with the same seed.

## Systems

In build order. All paths under `games/restaurant-nemesis-r3v2/`. Keep each
file under ~500 lines (CLAUDE.md convention 5); split where it grows.

- **Crate scaffold** — `Cargo.toml` (`[package] name = "restaurant_nemesis_r3v2"`, the four `.workspace = true` lines, `jidousha = { path = "../../crates/jidousha" }`, `[lints] workspace = true`), `src/main.rs` with `#![allow(missing_docs)]`, `GameConfig { title: "Restaurant Nemesis", seed: WINDOW_SEED, window_size: WINDOW, .. }`, the `--verify` dispatch, `mod` lines · `src/main.rs` · touches `jidousha-api.md`: `run`, `GameConfig`, `PhysicalSize`, `App::add_system`, `Startup`, `Update`, `Draw`, `RunError` (print `Display`).
- **Sim data + constants** — `Game` resource (phase, day, money, rep, capacity_left, preps_tonight, queue: `Vec<Order>`, nemeses: `Vec<Nemesis>` (active, spawn order), defeated: `Vec<Nemesis>`, spawned_per_theme: `[u32; 4]`, tomorrow_base: `Vec<Customer>`, log: `Vec<String>`, notice: `Option<String>`); `Theme`, `Customer { theme, need, temper }`, `Kind { Ordinary, Follower { leader: NemesisId }, Nemesis(NemesisId) }`, `Order { kind, customer, served: bool }`, `Nemesis { id, theme, title_index, followers, tally, spawned_day, next_visit }`, `Phase { Service { focus: Option<usize> }, Ledger, Over { won: bool, reason: &'static str } }`; every constant of the table below as a named `const` · `src/sim.rs` · touches `jidousha-api.md`: `Resource`, `Rng` (`below`), `World` (`insert_resource`, `resource`, `resource_mut`).
- **Rules: the three decision functions + their helpers** — `outcome_of`, `defeat_progress`, `social_outlook` (§Decisions already made, "The three functions"), plus `tier_of(followers) -> Tier`, `title_for(theme, index) -> &'static str`, `severity(customer) -> u32`, `cut_followers(&mut Nemesis)`, `draw_base_customers(&mut Rng) -> Vec<Customer>`. Pure functions over plain data; no `World` in any signature · `src/rules.rs` · touches `jidousha-api.md`: `Rng`.
- **Turn resolution** — `apply(game: &mut Game, action: Action)` where `Action { Serve(row), Overwhelm, Focus(row), Unfocus, CloseKitchen, Spend(slot), Prep, OpenTomorrow, Restart }`; `close_kitchen` resolves unmet orders in queue order through `outcome_of`, applies nightly effects, runs the viability check, draws `tomorrow_base`; `open_tomorrow` builds the queue from `social_outlook` and the nemeses whose `next_visit == day + 1` · `src/turn.rs` · touches `jidousha-api.md`: `Rng`.
- **Input → action** — `advance(world: &mut World)`: the one `Update` system; `find_resource::<Input>()`, `just_pressed` on `Key::Digit1..=Digit9`, `S`, `O`, `Escape`, `Enter`, `P`, mapped to an `Action` by phase (`fn action_for(phase, key) -> Option<Action>`), then `apply`. At most one action per tick (first matching key in that fixed order) · `src/input.rs` · touches `jidousha-api.md`: `Input`, `Key`, `World::find_resource`.
- **Screen** — `screen(game: &Game) -> Panel<Art>` with `enum Art {}` (no icons; `impl Icon for Art { fn size_at(self, _: f32) -> Vec2 { match self {} } }`); one function builds the Service panel (plus the card lifted onto the overlay band when `focus` is `Some`), the Ledger panel, the End panel; `struct Flat; impl Mapping for Flat` (identity: `to_world(ui) = ui`, `scale() = 1.0`); `draw(ctx: &mut DrawCtx)` builds the panel and calls `Panel::draw`; `mod layers { BACK = 0, CHROME = 1, OVERLAY = 2 }`; the camera in `Startup`: `Camera { center: Vec2::new(480.0, 270.0), height: 540.0, clear_color: palette::INK, ..Camera::default() }` so one world unit is one design unit at 16:9 · `src/screen.rs` (strings and jokes in `src/lines.rs` if it grows past 500) · touches `jidousha-ui.md`: `Panel` (`text`, `block`, `hint`, `lifted`, `draw`, `all_strings`), `TextRun`, `Cell`, `clipped`, `Icon`, `Mapping`; `jidousha-api.md`: `Camera`, `TextStyle`, `Depth`, `Color`, `DrawCtx`.
- **Scripted players** — `src/players.rs`: the three of §Gates (sleeper, order-taker, greedy chef), each a `fn(&Game) -> Option<Key>` policy driven by `SnapshotBuilder` events (press on one tick, release the next), plus `play(seed, policy, record: bool) -> Session` that builds `headless(GameConfig { seed, .. }, setup)`, ticks, and keeps the panels and frames it was asked for · touches `jidousha-testing.md`: `headless`, `HeadlessSim`, `SnapshotBuilder`, `InputEvent`, `InputScript`, `FrameRecorder`, `FrameRecord`.
- **Verify** — `src/verify.rs` (the mode: failures collected into a `Vec<String>`, `verified ` verdict line, indented summary, then one frame transcript) and `src/checks.rs` (bounds + clearance, clear-colour requirement, floors, ASCII, schedule order) · touches `jidousha-testing.md`: `FrameRecorder`, `FrameRecord` (`quads`, `covering`, `plan.clear_color`, `transcript`), `find_bounds`, `HeadlessSim::schedule_debug`; `jidousha-ui.md`: `judge_panel`, `judge_frame`, `frame_text_floor`, `Floors`, `Breach`.
- **Capture** — `src/capture.rs`: `examples/prototype_kit/capture.rs` minus the asset half (pure shapes and text), 480x270, `capture: <path> written to ...` line exactly as `jidousha-capture.md` words it; `RenderError::NoAdapter` → skipped-and-said · touches `jidousha-capture.md`: `WgpuBackend::offscreen`, `create_builtin_textures`, `upload_text_atlases`, `encode_png`, `RenderError`.
- **Mutants** — `mutants/r1.txt`, ≥ 12 faults (§Gates, G12), run with `tools/mutate restaurant-nemesis-r3v2 mutants/r1.txt`.
- **FINDINGS** — `games/restaurant-nemesis-r3v2/FINDINGS.md`: carry over the two entries of `tools/yakin/runs/restaurant-nemesis-r3v2/FINDINGS.md` (G-numbers continue the sequence across games) and add the build's own.

- Assets: none. Built-in font, `ctx.rect` for the card's backing and the row
  separators, text for everything else. No `Assets` resource, no `assets/`
  directory.

## Gates to add

All seeds are shipped literals (`const SEED_*: u64`), found by the implementer
scanning `0..256` for the first seed meeting the scenario's precondition
(§Open calls). Every assertion reports the numbers it judged. "Transcript" below
means the `Panel` returned by `screen(&game)` on the tick *before* the acting key
is pressed, **and** `judge_frame` of that panel against the frame recorded on
that tick (so what was asserted is what was drawn). The run's summary prints
each player's verdict line and the three numbers of G4.

- **G1 schedule** — input: `sim.schedule_debug()` · asserts: `advance` is in `Update` and `draw` in `Draw`, both found (two `None`s compare equal — assert `is_some()` first) · covers Done-when: "A multi-day run plays end to end" (the shape it runs in).
- **G2 bounds + clearance + ink** — input: every recorded frame of every session below and every staged screen · asserts: every quad inside `camera.visible_bounds()` (camera rebuilt with `viewport: WINDOW` as the testing doc shows), the closest-to-edge clearance printed; `frame.plan.clear_color == palette::INK` and the requirement form `brightest channel < 0.25, alpha > 0.99` · covers Done-when: `tools/verify ... pass`.
- **G3 floors, staged** — input: five staged screens: Service day 1; Service with the card open on a nemesis; Ledger with 2 nemeses; End won; End lost. Staged *correctively* (set every field of `Game` the screen depends on) · asserts: `judge_panel(&panel, &FLOORS, &[], &[("NEMESIS", "NEMESIS CARD")])` empty; `judge_frame` empty; `frame_text_floor(frame, font, 12.0)` empty; every string of `panel.all_strings()` in `' '..='~'`; and one deliberately staged breach — two rows 2 px apart — fails the overlap floor **by name** (`"two rows of chrome text overlap"`) · covers Done-when: `tools/verify ... pass`.
- **G4 three players** — input: `SEED_WIN` for the greedy chef, seed 0 for the sleeper, `SEED_TWO` for the order-taker; each run ticks until `Phase::Over` or 2000 ticks (a failure, reported with the phase and day it stalled in) · asserts: sleeper → `Over { won: false, reason: "money" }` on day 1 with money < 0; greedy chef → `Over { won: true }` on day 7; order-taker → its line reports the outcome without asserting it, and this run is the one that must see ≥ 2 nemeses active on one day (assert `max over days of nemeses.len() ≥ 2`, print the day). Three numbers printed per player, the turn-based analogues of the controllers doc's three: `served N of M orders`, `spawn-risk orders left unmet R` (the objective), `capacity used U of C` (the aim) · covers Done-when: "scripted --verify runs reach both a win and a viability loss, and at least one of them has 2 or more problem customers active on the same day".
- **G5 decision row 1 (capacity)** — input: `SEED_SPAWN`, the greedy chef's day-1 service, up to the last order it cannot serve whose `outcome_of(...).if_unmet.severity ≥ 4` (the precondition the seed scan looks for) · asserts: the transcript row for that order contains `sev <s>` with `s == outcome_of(..).if_unmet.severity` and the word `SPAWN`; after `Enter`, `nemeses.len() == 1`, `nemeses[0].theme == that order's theme`, `title_for(theme, 0)` equals the title in the night-1 ledger log line `SPAWNED: <title> (<theme>), sensitivity +2 <theme>, 30 fol, returns day 3`, and on day 3 that nemesis's queue row reads `need 3` (`NEMESIS_BASE_NEED + SENSITIVITY`) with the card reading `sensitivity +2 <theme>`; money after equals money before minus `REFUND_ORDINARY` per unmet row exactly as the rows previewed · covers Done-when: "a check for each decision row" (row 1).
- **G6 decision row 2 (the card), three runs** — input: `SEED_SPAWN` for all three, driven by the *scenario player* (§Open calls: the greedy chef's service rule, `Enter` only on every night, and the card choice the run names), spawn on day 1, first visit day 3 · asserts:
  - *tally*: day 3 card shows `won over 0 of 2`; `S`; day 5 card shows `won over 1 of 2 - S serves (3 units), DEFEATS them`; `defeat_progress(..).defeated_if_served == true` on day 5 and `false` on day 3; after `S` on day 5 the nemesis is in `defeated` and the day is exactly 5.
  - *overwhelm*: day 3 card shows `overwhelm - O (5 units), leaves 0 of 5 for the rest, DEFEATS them` (capacity untouched before the card); `defeat_progress(..).overwhelm == Some(5)`; after `O`, defeated on day 3, `capacity_left == 0`.
  - *followers cut*: night 1 ledger row shows `30 fol` and `2 spends to ratio`; `defeat_progress(..).spends_to_ratio == 2`; two `Digit1` presses → defeated in the ledger on night 1 (day still 1), `money` down by exactly 50, day 2's queue has no follower of that id and no nemesis row.
  · covers Done-when: "a check for each decision row" (row 2).
- **G7 decision row 3 (the ledger), two runs** — input: `SEED_SPAWN`, both runs identical through day 1's `Enter`; run A presses `Enter` on night 1, run B presses `Digit1` once then `Enter` · asserts: B's night-1 ledger transcript row 1 reads `spend $25: 30 -> 15 fol, Local Menace -> Grumbler, 2 -> 1 followers` and that string is built from `social_outlook` of a copy with `cut_followers` applied (the check recomputes it and compares); day 2 queues: A has exactly 2 rows of `Kind::Follower { leader }`, B exactly 1, the follower rows are the lowest-index ordinary slots, and every other row is equal field for field (same base draw: `tomorrow_base` is drawn before any spend); `money_A - money_B == 25`; B's nemesis tier is `Grumbler` · covers Done-when: "a check for each decision row" (row 3).
- **G8 outcome arithmetic** (`cargo test -p restaurant_nemesis_r3v2`, names as sentences) — `tier_of answers Grumbler at 29, Local Menace at 30 and 59, Trending Terror at 60`; `severity is need plus temper and four is the first value that spawns`; `a spawn at the cap feeds the same-theme nemesis severity times five followers, else the most followed, oldest on ties`; `title_for cycles through the theme's three titles by spawn count`; `cut_followers floors at zero`; `social_outlook with three Trending Terrors assigns 4, 2 and 0 of six slots in spawn order`; `a follower of a defeated leader is resolved as ordinary` · covers Done-when: `tools/verify ... pass` (the functions the rows rest on).
- **G9 unmet at the cap** — input: a staged `Game` with 3 nemeses and one dangerous unmet order · asserts: the row previews `feeds <title> +20 fol` (severity 4 × 5), after `Enter` that nemesis has +20 and `nemeses.len()` is still 3 · covers Done-when: DESIGN "what a spawn does at the cap".
- **G10 capture** — input: the staged Service-with-card screen of G3 · asserts: `capture: target/verify/restaurant_nemesis_r3v2.png written to ...`, 480x270, ids checked; `NoAdapter` → `capture skipped: no GPU` in the summary, run stays green · covers Done-when: `tools/verify ... pass` (the picture field).
- **G11 web** — `python3 tools/build-web restaurant_nemesis_r3v2 && python3 tools/serve-web restaurant_nemesis_r3v2 --check` · covers Done-when: that line.
- **G12 mutation round** — `tools/mutate restaurant-nemesis-r3v2 mutants/r1.txt` with at least these faults, expectations shipped as literals: `SPAWN_THRESHOLD 4→5`; `SENSITIVITY 2→1`; `REPEAT_TALLY 2→3`; `OVERWHELM_EXTRA 2→1`; `T2_AT 30→31`; `FOLLOWERS_AT_SPAWN 30→20`; `SOCIAL_CUT 15→16`; `SOCIAL_COST 25→20`; `RETURN_EVERY 2→1`; `BASE_CAPACITY 5→6`; `REFUND_ORDINARY 8→7`; share table `[1,2,4]→[1,2,3]`; the viability test `money < 0`→`money < -10`; card string `DEFEATS`→`defeats` (judge_frame + transcript must notice). Target: every fault noticed; report `N of N noticed` in the PR · covers Done-when: `tools/verify ... pass` (that the checks are instruments).

## Non-goals

- Nothing from the base spec's own Non-goals: no nemesis-to-nemesis interaction, no mutation of theme/title/sensitivity after spawn, no multiplayer, no real-time or dexterity play, no meta-progression, never more than 3 active nemeses.
- No pointer or touch input (keys only), no `Time::alpha` interpolation (nothing moves), no sound, no art files, no loaded fonts.
- No mid-day events: a day is a list of orders resolved by key presses and one `Enter`.
- No reputation gain from defeating by followers (only the two served paths give a rep bonus) — one fewer number to balance.
- No engine change is needed: everything above is built from `docs/api/`. The two FINDINGS entries in `tools/yakin/runs/restaurant-nemesis-r3v2/FINDINGS.md` are doc gaps worked around in the game (`Flat` defined locally; `judge_panel`'s overlay tuple read as (name, exact title row)).
- No seed sweep UI, no `Tuning` resource: constants are `const`s (the testing doc's own advice for a game with a dozen numbers).

## Decisions already made

### The numbers (the base spec's §"The design", every item)

| name | value | what it fixes |
|---|---|---|
| `DAYS` | 7 | run length; surviving day 7's check wins |
| `CUSTOMERS_PER_DAY` | 6 | ordinary customers each day |
| `BASE_CAPACITY` | 5 | units of service per day; `PREP_BONUS` 1 per prep, `MAX_PREPS` 2 per night (max 7) |
| `THEMES` | Condiment, Temperature, Wait, Portion (4, in this order) | the failure themes |
| customer generation | per customer: `theme = below(4)`, `need = [1, 1, 2][below(3)]`, `temper = below(3) + 1`, drawn in that order, six times, from the world `Rng`, **only** at `close_kitchen` (for tomorrow) and in `Startup` (for day 1); nothing else draws | how a day's customers and demands are generated |
| severity | `need + temper` (2..=5; a follower's need already carries `FOLLOWER_EXTRA_NEED`) | failure severity |
| `SPAWN_THRESHOLD` | 4 (severity ≥ 4 spawns) | the spawn threshold |
| `MAX_NEMESES` | 3; at the cap no spawn: the failure feeds `severity * CAP_FEED_PER_SEVERITY` (5) followers to the active nemesis of the same theme, else to the most followed (oldest on ties) | what a spawn does at the cap |
| title | `TITLES[theme][spawned_per_theme[theme] % 3]` — Condiment: Mustard Monster, The Ketchup Kaiser, Mayo Mayhem · Temperature: The Lukewarm Baron, Count Tepid, The Scalding Sultan · Wait: The Hangry Hourglass, Sir Waits-a-Lot, The Ticking Tyrant · Portion: The Crumb Countess, Big Plate Pete, Marquis de Morsel | title derivation (pure: theme and spawn count; no Rng) |
| `SENSITIVITY` | 2 extra units of need in their theme; `NEMESIS_BASE_NEED` 1, so a nemesis order needs 3 | the sensitivity's size |
| `RETURN_EVERY` | 2: first visit `spawned_day + 2`, then every 2 days; a visit happens whether served or failed, and `next_visit += 2` at that day's close | the return schedule |
| `FOLLOWERS_AT_SPAWN` | 30 | follower growth: start |
| `FAIL_LEADER_FOLLOWERS` | +30 when the nemesis's own order is unmet | follower growth: failing the leader |
| `FAIL_FOLLOWER_FOLLOWERS` | +5 when a follower's order is unmet (leader still active) | follower growth: failing a follower |
| tiers | `T2_AT` 30, `T3_AT` 60: 0..=29 **Grumbler**, 30..=59 **Local Menace**, 60+ **Trending Terror** | follower → tier |
| `SHARE` | `[1, 2, 4]` by tier: that many of tomorrow's 6 customers are this nemesis's followers, assigned to the lowest-index non-follower slots in spawn order, stopping at 6 | what each tier does to the day's customers |
| follower demand | theme := leader's theme; need += `FOLLOWER_EXTRA_NEED` (1); temper unchanged | the adopted themed demand |
| `TIER3_NIGHTLY_REP` | 3: every Trending Terror costs 3 reputation at each day's close | the top tier's extra |
| defeat 1: repeat | `REPEAT_TALLY` 2 satisfactions (a served visit = 1); defeated at the serving key press of the 2nd | defeat path condition |
| defeat 2: overwhelm | one visit served with `need + OVERWHELM_EXTRA` = 3 + 2 = 5 units; only offered when `capacity_left ≥ 5`; the extra units earn nothing | defeat path condition |
| defeat 3: ratio | followers reach 0 (only spends reduce followers); defeated at the spend | defeat path condition |
| on defeat | moved to `defeated`; no more visits; followers revert (the overlay only reads active nemeses; a follower order already in the queue is resolved as ordinary) | — |
| money | served ordinary/follower: `+PRICE_PER_UNIT (10) * need`; served nemesis: +30 (either way); unmet ordinary/follower: `-REFUND_ORDINARY` (8); unmet nemesis: `-REFUND_NEMESIS` (20) | service earns, failures cost |
| reputation | served ordinary/follower `+1`; unmet ordinary/follower `-temper`; nemesis served `+2`, won over `+5` more, overwhelmed `+5`; nemesis unmet `-5`; tier 3 nightly `-3`; clamped to 0..=100 | — |
| `SOCIAL_COST` / `SOCIAL_CUT` | $25 cuts 15 followers from one chosen nemesis; any number of spends a night while money ≥ 25 | what a social-media spend buys |
| `PREP_COST` | $15 → +1 capacity tomorrow only, max 2 a night | the other sink |
| `START_MONEY` / `START_REP` | 40 / 30 | — |
| viability | checked once per day at `close_kitchen`, after unmet penalties and nightly effects: lose if `money < 0` (reason `"money"`, checked first) or `rep <= 0` (reason `"reputation"`); spends never drive money below 0 | viability; the lose condition |
| win | day 7's check passes → `Over { won: true }`; no ledger after day 7 | the win condition |
| `WINDOW` / camera | 1280x720; `Camera { center: (480, 270), height: 540 }`, design rect 960x540, `Flat` identity mapping, `FLOORS.min_text` 12 (header 18, rows 12) | presentation |
| `WINDOW_SEED` | 7 | the seed the windowed run plays |

### The three functions (one per decision row; the display and the outcome both call them)

- `outcome_of(order: &Order, game: &Game) -> OrderOutcome { if_served: Reward { money, rep }, if_unmet: Unmet { theme, severity, consequence: Consequence::{Spawns, Feeds { id, followers }, Nothing}, money, rep, leader_followers } }` — the service row's two lines are formatted from it, and `close_kitchen` applies `if_unmet` for every unserved order, `Serve` applies `if_served`. Nemesis orders: `if_unmet.consequence` is always `Nothing` and `leader_followers` is 30; a nemesis failure never spawns.
- `defeat_progress(n: &Nemesis, capacity_left: u32) -> DefeatProgress { tally: (u32, u32), defeated_if_served: bool, overwhelm: Option<u32> (the cost, `None` when capacity is short), leaves_for_rest: u32, followers: u32, tier: Tier, spends_to_ratio: u32 }` — the card's lines are formatted from it; `Serve`, `Overwhelm` and `Spend` resolve the defeat by reading the same value (`defeated_if_served`, `overwhelm.is_some()`, `spends_to_ratio == 1` for the spend that lands).
- `social_outlook(nemeses: &[Nemesis], base: &[Customer]) -> Outlook { per: Vec<(NemesisId, followers, Tier, share)>, customers: Vec<(Kind, Customer)> }` — the ledger's nemesis rows and tomorrow list are formatted from it; the spend preview calls it on a clone with `cut_followers` applied; `open_tomorrow` calls it on the real list and takes `customers` as the day's ordinary orders.

### Surfaces and inputs (the spec's three rows, elaborated)

| decision | surface (strings the gates look for) | action | one function | asserted by |
|---|---|---|---|---|
| Which order gets the next capacity | Service rows: line 1 `[n] <who> wants <dish> (<theme>), need <k>`; line 2 `  served: +$<m> +1 rep \| unmet: <theme> sev <s> -> SPAWN \| feeds <title> +<f> fol \| no spawn, -$8 -<t> rep`; nemesis rows: `[n] <TITLE> - <tier>, <f> fol - wants <theme>, need 3` / `  unmet: +30 fol -$20 -5 rep \| press <n> for the card` | `Digit<n>` | `outcome_of` | G5 (and G9 at the cap) |
| Bank a satisfaction or go all-out | the card: `NEMESIS CARD` / `<TITLE> - <theme>, sensitivity +2 <theme>` / `<tier>, <f> followers, <k> spends to ratio` / `won over <t> of 2 - S serves (3 units)[, DEFEATS them]` / `overwhelm - O (5 units), leaves <c-5> of <c> for the rest, DEFEATS them` or `overwhelm - O needs 5 units, only <c> left` / `<complaint>` / `Esc back` | `S` / `O` while the card is open | `defeat_progress` | G6 (three runs) |
| How to spend the night's money | Ledger: `NIGHT <d> - LEDGER`; the night's `tonight:` line and event lines; `money $<m> (closes below $0)  rep <r> (closes at 0)`; per nemesis `[n] <TITLE> - <tier>, <f> fol - tomorrow <share> of 6 customers are followers (<theme>, +1 need)` / `  spend $25: <f> -> <f'> fol, <tier> -> <tier'>, <share> -> <share'> followers[, DEFEATS them]`; `P prep $15: capacity tomorrow <c> -> <c+1> (max 7)`; `tomorrow:` then six rows `<i>. <theme> need <k> temper <t>[ F:<TITLE>]` | `Digit1..3`, `P`, `Enter` | `social_outlook` | G7 |

### Everything else settled here

- **Queue order**: nemeses visiting today first (spawn order), then ordinary slots 0..5. Row numbers are 1-based and are the digit keys.
- **Rewards apply at the key press** (serve/overwhelm/spend); **penalties apply at `Enter`** in queue order; spawns are checked against the cap per failure in that order; nightly effects after all orders; then the viability check; then `tomorrow_base` is drawn (not on day 7).
- **Turn system**: one `Update` system `advance`, one `Draw` system `draw`; `Startup` `set_the_scene` inserts the camera and the `Game` (day 1's customers drawn from the world `Rng` there).
- **Tone**: every string printable ASCII. Dishes: Condiment `fries, mustard NOW`; Temperature `soup, hot means HOT`; Wait `the special, in a hurry`; Portion `a heroic portion`. Card complaints: Condiment `"Last time the mustard was a rumour."`; Temperature `"I have had warmer handshakes."`; Wait `"I aged. Visibly."`; Portion `"I have seen bigger croutons."`. Ledger post lines by tier: Grumbler `posted a one-star review. Nobody liked it.`; Local Menace `the whole street has seen the video.`; Trending Terror `trending. A news van is outside.`
- **Notices** are one `Option<String>` on `Game`, cleared on the next action; drawn as the last row before the hint.
- **Log lines** (`Game.log`; the ledger shows `tonight: served <n>, unmet <m> (-$<x>, -<r> rep)` and then every SPAWNED/FED/WON OVER/OVERWHELMED/RATIOED/trending line of that night in order, never the per-order lines; the End screen shows nothing from it; exact formats the gates read): `served <who> (+$<m>, +<r> rep)`; `unmet <who>: <theme> sev <s>, -$<m>, -<r> rep`; `SPAWNED: <title> (<theme>), sensitivity +2 <theme>, 30 fol, returns day <d>`; `FED: <title> +<f> fol (cap full)`; `WON OVER: <title>`; `OVERWHELMED: <title>`; `RATIOED: <title>`; `<title> is trending: -3 rep`. `<who>` is `customer <i>`, `follower of <title>`, or the title.
- **End reasons**: `"money"` / `"reputation"`; the End screen's second line is `CLOSED DOWN - out of money` / `CLOSED DOWN - reputation hit zero`; win line `SURVIVED 7 DAYS - THE REVIEWS ARE MIXED`.
- **Verify layout**: `main.rs` dispatches `--verify` to `verify::run() -> ExitCode`; failures collected; the verdict line is `verified restaurant_nemesis_r3v2: <n> checks, <k> players, <m> frames`; summary lines indented; then one frame transcript (the capture's frame).

## Open calls delegated to the implementer

- **Seeds**: `SEED_SPAWN` (first seed in 0..256 where the greedy chef's day 1 leaves unmet an order with severity ≥ 4, exactly one nemesis spawns on day 1, and that nemesis is alone through day 5 under the tally policy), `SEED_WIN` (first seed where the greedy chef wins), `SEED_TWO` (first seed where the order-taker has ≥ 2 nemeses active on one day). Ship them as literals and print them in the summary. If any scan comes back empty, widen to 0..1024 before touching a number.
- **The scenario player** for G5-G7: the greedy chef's service rule below (nemesis rows first, then ordinary by severity descending), `S`/`O`/`Digit` on the card or ledger exactly as the gate names, otherwise `Enter` on every night (no spends, no prep).
- **The greedy chef's policy**, within this shape: on service, serve nemesis rows with `S` first (`O` instead when `overwhelm.is_some()` and `money >= 50`), then ordinary rows by `if_unmet.severity` descending, ties by row, while capacity allows; `Enter`. On the ledger, spend on the nemesis with the most followers while `money >= SOCIAL_COST + 15`; then prep once if `money >= 40`; `Enter`. The order-taker serves rows 1..9 in order while capacity allows, never spends. The sleeper only presses `Enter`.
- **One tuning knob, bounded**: if the greedy chef wins on no seed in 0..1024, raise `START_MONEY` in steps of 10 up to 80, and record the change as a deviation in CHECKPOINT and the PR. No other number moves.
- **Row layout numbers** inside the 960x540 rect (header at y 12 size 18; rows start y 48, 12 px text, 14 px leading; card rect centred, 640x200, `OVERLAY` band, `ctx.rect` backing at `Depth { layer: OVERLAY, z: 0.0 }` and text above it) — adjust so `judge_panel` is clean, keeping `min_text` 12 and every row inside the rect.
- **Colours** (`mod palette`): `INK` dark (brightest channel < 0.25), text white, nemesis gold, `SPAWN` red, `no spawn` green, notice magenta — exact values are the implementer's.
- **Mutant list wording** and any extra faults beyond G12's fourteen.
- Whether `turn.rs` and `rules.rs` are one file or two, if both stay under ~500 lines.
