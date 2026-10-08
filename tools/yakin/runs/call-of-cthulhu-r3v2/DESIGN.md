# DESIGN — call-of-cthulhu-r3v2

task: call-of-cthulhu-r3v2
variant: V2
model-as-configured: claude-fable-5-1 (the session's stated configuration; its context also names fallbacks claude-opus-5-5[1m] and claude-opus-5[1m], and only the run page says which served)
date: 2026-10-07 21:17 PDT

This is the whole design note the base spec asks for: the beings (§Systems,
"beings"), the call structure (§The player-facing loop, "the night"), the day
loop (§The player-facing loop) and the sanity arithmetic (§Decisions already
made, "the arithmetic"). The implementer builds `games/call-of-cthulhu-r3v2/`
(package `call_of_cthulhu_r3v2`) from this file and `docs/api/` alone, with the
`make-game` skill's §A as the build order. Every string a screen draws is
printable ASCII (space through `~`): no em dash, no curly quote, no middle dot
anywhere in game text — the built-in font draws a box for each and no quad
check can see it.

## What the game is

Eldritch beings telephone you at night, and every exchange on the line costs
sanity, so a call must be ended as fast as it can be without offending the
caller. It is a dating sim inverted: each morning you study a being's lore,
steady your nerves or attend a cult, and what you learned is what lets you give
the answers that make a being hang up satisfied instead of lingering or
screaming. Five days, three beings; lose when sanity reaches zero, win by
hearing the dawn of day six.

## The player-facing loop

The game is turn-based: nothing happens until the player presses a key, and an
idle player is never charged. Keys are the digit row (`Key::Digit1` ..
`Key::Digit7`) to pick a numbered option and `Key::Enter` to continue past a
Dawn or End screen; nothing else does anything. Every screen is one `Panel`
(`jidousha::ui`) built by one pure function from the game state, drawn by the
one Draw system, and appended line by line to the game's **transcript** (a
`Vec<String>` in the game state) the moment it is shown, with `> <option text>`
appended when a choice commits. The screens, in order:

1. **Morning** (`== morning d ==`). Rows: the title; `DAY d of 5 - MORNING`;
   `sanity S/90` with the sanity bar; `tonight: X (<per> sanity an exchange,
   temper t/3)` and, on nights 4 and 5, `, then Y (...)` — read from
   `tonight()` with `cult: None` (no cult is attended yet when the morning opens); one status row per being, `X - temper t/3 - secrets known
   k/3`; then the numbered options, each with its **effect line** indented
   under it (the stated effect on tonight, from `effect_line()`); footer hint
   `press the number of a choice`. Pressing a digit commits that option; the
   effect is applied (lore learned, nerves steadied, cult attended) and the
   night begins at once.
2. **Exchange** (`== night d call c exchange e ==`), one per question. Rows:
   title; `DAY d of 5 - NIGHT`; sanity row and bar; `call c of n - <Being>,
   <epithet>`; a status line `each exchange: -<per> sanity | temper t/3 |
   patience p`; on the first exchange of a call the being's opening line,
   afterwards the **reaction line** of the previous exchange (what it did,
   what it cost); `what you know of <Being>:` followed by one gold row per
   known secret (the fact text) or one dim row `- nothing`; `it says:
   "<question>"`; the numbered **options** from `call_options()` — the lore
   reply first and marked `*` **only if its secret is known**, then the two
   other replies in the question's authored order, then `Hang up.` last; and
   the footer hint, every number of which comes from `answer_outcome()`: `* a
   reply from its lore costs 1 sanity. any other costs <per>; one that offends
   it costs <2per> and +1 temper. hanging up adds +2 temper. at temper 3 it
   screams: -25 sanity and the call ends.` Pressing a digit answers; the
   outcome is applied; the next exchange, the next call, the Dawn or the End
   follows on the same tick.
3. **Dawn** (`== dawn d ==`): `NIGHT d is over.`; one row per call `<Being>:
   e exchanges, -s sanity`; `sanity S/90`; `press Enter`. Enter opens the
   next Morning, or the End after night 5.
4. **End** (`== end won ==` / `== end lost ==`). Won: `DAWN OF DAY 6. You pull
   the cord from the wall.` and `sanity S/90 remains.` Lost: `Your sanity is
   gone.` and `<Being> is still talking.` Both: `press Enter to begin again`.
   Enter rebuilds the game (a fresh rotation drawn from the world `Rng`, so
   the run stays replayable).

**The night** (the call structure). Night d has one call on days 1-3 and two on
days 4-5. A call opens with the being's `patience` at 4 and runs exchange by
exchange; the being asks its three questions in authored order, cycling
(exchange e asks question `e % 3`). Each reply has an outcome (below); the call
ends when patience reaches 0 ("it is satisfied; the line goes dead"), when the
player hangs up, when the being screams, or at the sixth exchange whatever was
said ("the line dies at dawn"). Sanity reaching 0 or below at any moment ends
the run on the lost End screen immediately, mid-call.

## Systems

In build order. All paths are under `games/call-of-cthulhu-r3v2/`; the manifest
names `jidousha = { path = "../../crates/jidousha" }` and nothing else, with
`#![allow(missing_docs)]` at the crate root (make-game §A.2).

- **beings** — the three callers as data: `enum BeingId { Cthulhu,
  Nyarlathotep, Hastur }` (index 0..3), `struct Question { ask, fact,
  replies: [(AnswerKind, &'static str); 3] }` (the lore reply, the wrong reply
  and the offending reply in a per-question authored order), `struct Being {
  id, name, epithet, opening, drain: u32, questions: [Question; 3] }`, `const
  BEINGS: [Being; 3]`. Content is §"The beings" below, copied verbatim. ·
  `src/beings.rs` / touches nothing in `docs/api/` (plain data).
- **rules** — the arithmetic, all pure: the constants (`START_SANITY 90`,
  `PATIENCE 4`, `MAX_EXCHANGES 6`, `LINE_NOISE 1`, `ANGER_MULT 2`,
  `TEMPER_LIMIT 3`, `WRATH 25`, `HANG_UP_TEMPER 2`, `DAYS 5`, `TWO_CALL_FROM
  4`); `enum AnswerKind { Lore, Wrong, Anger, HangUp }`; `struct Outcome {
  sanity_cost: u32, temper_delta: u8, progress: u8, hangs_up: bool }`; **`fn
  answer_outcome(being: BeingId, temper: u8, meditated: bool, kind:
  AnswerKind) -> Outcome`** (decision row 1's one function); `struct CallPlan {
  being: BeingId, per_exchange: u32 }`, `struct NightPlan { calls: Vec<CallPlan>
  }`; **`fn tonight(day: u8, rotation: [BeingId; 3], tempers: [u8; 3],
  meditated: bool, cult: Option<BeingId>) -> NightPlan`** (decision row 2's one
  function); `fn rotation(rng: &mut Rng) -> [BeingId; 3]` (a Fisher-Yates over
  the three ids with `Rng::below`). · `src/rules.rs` / touches
  `jidousha-api.md`: `Rng` (`below`).
- **flow** — the state machine over a plain struct, no `World` in it: `enum
  Screen { Morning, Exchange, Dawn, End { won: bool } }`; `enum MorningAction {
  Study(BeingId), Meditate, Cult(BeingId) }`; `enum Choice { Pick(usize),
  Continue }`; `struct Night { plan: NightPlan, call: usize, exchange: u8,
  patience: u8, summary: Vec<(BeingId, u8, u32)> }`; `struct Game { day: u8,
  sanity: i32, tempers: [u8; 3], known: [[bool; 3]; 3], meditated: bool, cult:
  Option<BeingId>, rotation: [BeingId; 3], screen: Screen, night:
  Option<Night>, last_line: String, transcript: Vec<String> }` with
  `Game::new(rotation)`; `fn morning_options(&Game) -> Vec<MorningAction>`
  (`Study(X)` for each being with an unknown secret, in id order; then
  `Meditate`; then `Cult(X)` for each being in id order); `fn effect_line(&Game,
  MorningAction) -> String`; `fn call_options(&Game) -> Vec<(AnswerKind,
  &'static str)>`; **`fn step(&mut Game, Choice)`**, which applies a choice,
  moves the screen, and appends to the transcript. `Game` is a `Resource`. ·
  `src/flow.rs` / touches `jidousha-api.md`: `Resource`.
- **screen** — `fn screen(&Game) -> Panel<Art>` with `enum Art {}` (no icons;
  `impl Icon for Art` is the empty match, as `jidousha-ui.md`'s `judge_panel`
  entry shows), the layout constants (§Layout below), `const FLOORS: Floors`,
  the identity `Mapping` (`to_world(ui) = ui`, `scale() = 1.0`), and the
  sanity bar as a `Rect` the draw system fills beside the panel. ·
  `src/screen.rs` / touches `jidousha-ui.md`: `Panel` (`text`, `hint`,
  `all_strings`, `draw`), `TextRun`, `Icon`, `Mapping`, `Floors`, `wrap`;
  `jidousha-api.md`: `TextStyle`, `Depth`, `Color`, `Rect`.
- **main** — `config()` (`GameConfig { title: "call of cthulhu", seed: 1928,
  window_size: 1280x720, ..default }`), `register(app)` with exactly three
  systems: `Startup set_the_scene` (camera `center (480, 270)`, `height 540`,
  `clear_color NIGHT`; draws `rotation()` from the world `Rng`; inserts
  `Game::new(rotation)`), `Update play` (reads `Input` with `find_resource`;
  the first `just_pressed` digit maps to `Choice::Pick(n - 1)` if `n` is
  within the current option count, `Enter` to `Choice::Continue`; calls
  `step`; on `End` + `Continue` rebuilds the game with a new rotation from the
  world `Rng`), `Draw draw_screen` (`screen(game).draw(ctx, &Identity, ..)`,
  then the sanity bar: a `DIM` track rect and a fill rect `BAR_W *
  sanity/START_SANITY` wide in `SEA`, `EMBER` when sanity < 25); `mod layers`
  (`BAR -1`, `TEXT 1`); `main` with the `--verify` branch first. ·
  `src/main.rs` / touches `jidousha-api.md`: `run`, `GameConfig`, `App`,
  `Startup`/`Update`/`Draw`, `Camera`, `Input` (`just_pressed`), `Key`,
  `Rng`, `Submit::rect`, `RunError`.
- **checks** — the failure accumulator (`Checks::require`, `verdict`,
  `greater`, `near`), copied in shape from
  `crates/jidousha/examples/prototype_kit/checks.rs`. · `src/checks.rs` ·
  touches `jidousha-api.md`: `message`.
- **players** — the three controllers of §Gates G5, each a function from `&Game`
  to the `Choice` it makes on the current screen, driven through
  `SnapshotBuilder` (`KeyPressed`/`KeyReleased` pairs, one press per screen,
  one idle tick between presses) and each printing its three numbers. ·
  `src/players.rs` / touches `jidousha-testing.md`: `SnapshotBuilder`,
  `InputEvent`, `Input`, `HeadlessSim` (`world`, `world_mut`, `tick`).
- **verify** — `--verify`: the gates below, the `verified ` verdict line, the
  indented summary (sanity left per player, clearance, capture line), then
  the last live frame's `transcript()` and the game transcript as evidence. ·
  `src/verify.rs` / touches `jidousha-testing.md`: `headless`, `FrameRecorder`
  (`new`, `draw`, `font_texture`), `FrameRecord` (`quads`, `covering`, `plan`,
  `transcript`), `InputScript` (`press`, `snapshot_at`), `find_bounds`;
  `jidousha-ui.md`: `judge_panel`, `judge_frame`, `frame_text_floor`,
  `glyph_run`.
- **capture** — the PNG of the last live Exchange frame at 480x270, the
  shapes-and-text path (`create_builtin_textures`, `WgpuBackend::offscreen`,
  `encode_png`; `NoAdapter` reported as skipped, every other error a fault),
  from `crates/jidousha/examples/prototype_kit/capture.rs` minus its asset
  half; prints `capture: <path> written to <dir>` exactly as
  `jidousha-capture.md` requires. · `src/capture.rs` / touches
  `jidousha-capture.md`: `create_builtin_textures`, `WgpuBackend`
  (`offscreen`, `poll`, `is_ready`), `RenderBackend` (`render`, `capture`),
  `RenderError`, `encode_png`, `FONT_TEXTURE`, `TextureTable`.
- **mutants** — `mutants/r1.txt`, the twenty faults of §Gates "the mutation
  round", in `tools/mutate`'s four-line block format, run with `python3
  tools/mutate call-of-cthulhu-r3v2 mutants/r1.txt`. · `mutants/r1.txt`.

- Assets: none. Shapes and the built-in font only; no `Assets` resource is
  ever inserted, no `assets/` directory exists.

### The beings

Lore is written for this game, drawing on the public-domain mythos; no text
from any commercial game. Each being: name, epithet, drain (sanity per
exchange), opening line, then three questions. Under each question: the
**fact** (what studying teaches, shown under "what you know"), then the three
replies in the order they are listed on screen, tagged L (lore), W (wrong),
A (offends). The lore reply is only listed when its fact is known.

**Cthulhu**, "the Sleeper of R'lyeh". drain **6**. Opening: `The line is full
of water. Something very large breathes at the other end.`
1. ask: `WHERE... DO I LIE, LITTLE VOICE?`
   fact: `It lies in R'lyeh, sunk beneath the Pacific, until the stars come right.`
   W `In a cave under the Himalayas, I think.` / L `In R'lyeh, beneath the sea, until the stars are right.` / A `Nowhere. You are a story the sailors tell.`
2. ask: `WHAT DO THE DREAMERS SEE WHEN I CALL THEM?`
   fact: `It speaks to artists in dreams: a city of wrong angles, a head of tentacles, wings.`
   L `A city of wrong angles, and your face: the tentacles, the wings.` / A `Nothing. Nobody dreams of you any more.` / W `Numbers. Long columns of numbers.`
3. ask: `WHAT DO MY PRIESTS SING?`
   fact: `Its cult chants that in his house at R'lyeh dead Cthulhu waits dreaming.`
   W `A sea shanty. Something about a whale.` / A `Wake up, old man. You have overslept.` / L `In his house at R'lyeh dead Cthulhu waits dreaming.`
   (What offends it: being told it does not exist, is forgotten, or is merely
   asleep.)

**Nyarlathotep**, "the Crawling Chaos". drain **4**. Opening: `A pleasant
voice, amused, as if it had been expecting you to pick up.`
1. ask: `Tell me, friend: what face am I wearing tonight?`
   fact: `He wears a thousand masks; on the line he is the tall dark man out of Egypt who shows machines.`
   L `The tall dark man out of Egypt, the one who shows the machines.` / W `A face like my uncle's. Round, kind.` / A `You have no face. You are nothing but a voice.`
2. ask: `Whose errand do you suppose I am running?`
   fact: `He is the messenger of the Outer Gods and the mouth of blind Azathoth; mistake him for Cthulhu and he will not forgive it.`
   W `The devil's, I would guess.` / L `Azathoth's. The blind idiot god at the centre of everything.` / A `Cthulhu's, like all of you.`
3. ask: `I put on a little show in your city tonight. What did they see?`
   fact: `He shows crowds sparks and cold flames, and afterwards nobody in the city can sleep.`
   A `Nothing worth watching. They went home early.` / W `A juggling act, wasn't it?` / L `Sparks and cold fire, and afterwards none of them could sleep.`
   (What offends him: denying his masks, belittling his show, confusing him
   with the fish.)

**Hastur**, "the Unspeakable". drain **5**. Opening: `A whisper under wind.
The line crackles every time you breathe.`
1. ask: `...you know who this is. Say who.`
   fact: `It is not to be named. Say "Hastur" on the line and it hears you say it.`
   W `Is this the gas company?` / A `Hastur. It's Hastur.` / L `I know. The one who is not to be named.`
2. ask: `...have you seen my sign? Describe it.`
   fact: `Its sign is the Yellow Sign, and the King in Tatters wears a pallid mask.`
   L `The Yellow Sign. I saw it and I will not draw it.` / W `A red triangle, on a hill.` / A `A sign? Hastur, you don't even have a sign.`
3. ask: `...where do the black stars hang?`
   fact: `It dwells by the lake of Hali, under black stars, in lost Carcosa.`
   A `Over your head, Hastur, wherever that is.` / L `Over the lake of Hali. Over Carcosa.` / W `Over Arkham, by the river.`
   (What offends it: its name. Every offending reply says it; the first fact
   teaches the rule.)

### Layout

Design space 960x540 = world space: the camera is `center (480, 270)`,
`height 540`, window 1280x720, so `visible_bounds()` is the design rect and
the `Mapping` is the identity, which is what lets `frame_text_floor` judge the
frame in text units. Rows run from `x = 24` to `x = 936`. Styles: `TITLE`
size 20 `SEA`; `BODY` size 14 `INK`; `SMALL` size 12 `DIM` (effect lines and
hints); lore rows and the `*` option `GOLD`; costs, temper and the low bar
`EMBER`. Leading: body 18, small 16. Title at `y 16`; the day/phase row at
`(600, 20)`; the sanity row at `y 48` with the bar track at `(200, 50)` size
`BAR_W 400 x 10`; body from `y 80`; options follow the body; the footer hint
is `Panel::hint` at `(24, 496)`, width 912, size 12, leading 14. Palette:
`NIGHT rgb(0.05, 0.04, 0.09)` (clear), `INK rgb(0.86, 0.83, 0.74)`, `DIM
rgb(0.52, 0.50, 0.58)`, `GOLD rgb(0.90, 0.72, 0.30)`, `EMBER rgb(0.85, 0.32,
0.22)`, `SEA rgb(0.35, 0.75, 0.70)`. Floors: `min_text 12`, `chrome` the
design rect, `world` the design rect. The implementer may move rows, never
cross a floor; at size 14 the built-in face gives 83 columns across 912
units, and no authored string above exceeds 70 characters before its `n. `
prefix.

## Gates to add

All run inside `--verify` (`python3 tools/verify call_of_cthulhu_r3v2`
reports `pass` in `target/verify/call_of_cthulhu_r3v2.json`). Failures are
collected, never exited on; every message prints the numbers it judged.
Seeds: `1928` (the shipped one) and `7`. "Done-when" lines are the r3v2
spec's; D2 is "several days, 2-3 beings, win and loss reached", D3 is "a
check for each decision row", D5 the web check.

- **G1 determinism** — input: two headless runs on seed 1928 driven by the
  Scholar · asserts: both game transcripts are byte-identical, and
  `tonight(d, ..)` called twice with equal arguments is equal; on each seed
  the first callers of nights 1-3 are the three distinct beings and the two
  callers of nights 4 and 5 are distinct · covers Done-when: D2.
- **G2 the arithmetic, as shipped literals** — input: direct calls to
  `answer_outcome` / asserts: Cthulhu, temper 0, not meditated: Lore -> (1,
  0, 2, false); Wrong -> (6, 0, 1, false); Anger -> (12, 1, 0, false); HangUp
  -> (0, 2, 0, true). Nyarlathotep Wrong -> 4, Hastur Wrong -> 5. Cthulhu
  meditated Wrong -> 5, Lore still 1. Cthulhu temper 3 Wrong -> 12, Anger ->
  24. Written as literals, never as arithmetic over the constants · covers
  Done-when: D3 (row 1's one function).
- **G3 decision row 1, the call screen** — input: seed 1928, an
  `InputScript` that presses the digit of `Study <tonight's first caller>` on
  tick 5, so tick 6 shows night 1's first exchange · asserts: the transcript
  since the last `==` marker contains the being's question 1 `ask`, every
  option text `call_options()` lists (four of them, the first beginning
  `1. *`), the row `sanity 90/90`, `temper 0/3`, and the known fact's text
  under `what you know of`; the footer hint contains `costs 1`, the Wrong
  cost and the Anger cost that `answer_outcome` returns for this being;
  `judge_frame` finds every row on the recorded frame. Then the script
  presses the digit of the Wrong reply on tick 8 and the check asserts
  `sanity == 90 - outcome.sanity_cost` and `temper == outcome.temper_delta`
  for `outcome = answer_outcome(being, 0, false, Wrong)`, and the reaction
  line names that cost · covers Done-when: D3 (row 1).
- **G4 decision row 2, the morning screen** — input: three headless runs on
  seed 1928 differing only in the morning-1 press: A `Study X` (X tonight's
  first caller), B `Meditate`, C `Cult Y` (Y the next being after X in id
  order, so not tonight's caller) · asserts: at the press, the option's
  effect line is in the transcript and reads exactly `effect_line()`'s text,
  which for A contains `question 1 costs 1, not <d_X>`, for B `X <d_X>-><d_X -
  1>`, for C `Y calls tonight instead of X`; then on the first exchange A
  lists four options with `1. *` and `each exchange: -<d_X>`, B lists three
  with `each exchange: -<d_X - 1>`, C's caller is Y with three options; A and
  B have the same caller X. Also: the Morning's `tonight:` row names the
  being the transcript then shows calling first, in every run of G5 · covers
  Done-when: D3 (row 2).
- **G5 three players** (`jidousha-controllers.md`, adapted to turns) — input:
  seeds 1928 and 7, each player through the sim via `SnapshotBuilder` ·
  asserts: **Scholar** (morning: `Study` the first of tonight's callers with
  fewer than two secrets known, else `Meditate`; night: the Lore reply when
  listed, else Wrong, never Hang up) reaches `End { won: true }` with sanity
  >= 20 and the transcript holds `DAY 5`; **Guesser** (morning: `Meditate`;
  night: always Wrong) reaches `End { won: false }` during night 4 or 5;
  **Blasphemer** (morning: `Meditate`; night: always Anger) reaches `End {
  won: false }` by the end of night 2. Each prints three numbers: `answered N
  of N questions asked`, `lore replies K of N`, and `predicted cost X, paid
  Y` where X sums `answer_outcome` for every reply it made before pressing,
  and asserts X == Y (the hint and the resolution are one function) · covers
  Done-when: D2 (win and loss reached, five days, three beings), D3.
- **G6 floors and the screens never reached** — input: every frame G3-G5
  record, plus staged screens: Exchange with four options and three known
  facts for each being; Dawn after a two-call night; End won; End lost;
  Morning on day 5 with all `Study` options gone · asserts: `judge_panel`
  returns no breach; `judge_frame` returns no breach; `frame_text_floor(..,
  12)` returns none; every quad inside `visible_bounds()`, clearance printed;
  `panel.all_strings()` all printable ASCII; the staged pair of rows at `y`
  and `y + 2` makes `judge_panel` report `two rows of chrome text overlap`
  by name · covers Done-when: D2.
- **G7 the schedule** — input: `sim.schedule_debug()` / asserts: it names
  `set_the_scene` under Startup, `play` alone under Update and `draw_screen`
  under Draw, each found (`Some`), so the verify runs the window's
  registration · covers Done-when: D2.
- **G8 the sanity bar** — input: a staged frame with sanity 45 · asserts: the
  fill quad found at the bar's centre is `BAR_W / 2` wide within 0.5 and the
  track quad is `BAR_W` wide; with sanity 20 the fill's tint is `EMBER` ·
  covers Done-when: D2.
- **G9 the clear colour** — input: any recorded frame · asserts: `plan.
  clear_color == NIGHT` and, in requirement form, its brightest channel is
  below 0.2 with alpha above 0.99 (ink has to read on it) · covers Done-when:
  D2.
- **G10 the picture** — input: the last live Exchange frame of the Scholar's
  run · asserts: a 480x270 PNG is written and the `capture:` line printed;
  `NoAdapter` is reported as skipped in the summary · covers Done-when: D2
  (make-game §A.7).
- **G11 exact zero loses** — input: a `Game` built with sanity equal to one
  Wrong cost, `step(Pick(Wrong))` directly · asserts: `screen == End { won:
  false }` / covers Done-when: D2.
- **the mutation round** — `mutants/r1.txt`, one block each, every one
  expected `noticed`: Cthulhu drain 6->5; `LINE_NOISE` 1->2; `ANGER_MULT`
  2->1; `WRATH` 25->5; `TEMPER_LIMIT` 3->4; `HANG_UP_TEMPER` 2->1; `PATIENCE`
  4->5; `START_SANITY` 90->120; Lore progress 2->1; the meditate subtraction
  dropped; the cult replacement dropped; the second caller's index `d % 3` ->
  `(d - 1) % 3`; the `if known` guard on the lore option dropped; the `*`
  marker dropped; the bar fill width using `START_SANITY` for sanity; `NIGHT`
  brightened to 0.5; temper not raised on Anger; `sanity <= 0` -> `< 0`; the
  win on `day >= 4`; study marking fact 2 instead of the lowest unknown. The
  PR reports `N of 20 noticed`.

The web gate (`python3 tools/build-web call_of_cthulhu_r3v2 && python3
tools/serve-web call_of_cthulhu_r3v2 --check`, D5) needs nothing beyond the
Morning screen drawing on frame one with no input, which it does.

## Non-goals

- **No real-time pressure.** Silence on the line costs nothing; the game is
  turn-based. Cut for size and so every gate is a count of presses, not of
  ticks.
- **No better ending.** Surviving is the win; the base spec allows a better
  ending and this build does not add one.
- **No pointer or touch input.** Digits and Enter only; the web build is
  keyboard-played. A tap surface would double the controls list and the
  floors' work.
- **No art, no sound, no loaded font.** Built-in face, rects and text.
- **No between-run persistence**, no difficulty settings, no title screen (the
  game opens on Day 1's morning).
- **Temper never cools on its own**; only a cult visit lowers it. One skill
  (steadying the nerves) and it lasts one night.
- **Nothing needs an engine change.** `GameConfig::seed` is not readable from
  the world, so the rotation is drawn from the seeded `Rng` resource in
  Startup instead; that is a design around a surface that exists, not a gap,
  so `FINDINGS.md` carries no entry from this stage (see "Decisions").

## Decisions already made

**The arithmetic** (decision row 1's one function, `answer_outcome`):

- `per = max(1, drain * (if temper >= 3 { 2 } else { 1 }) - (if meditated { 1 } else { 0 }))`.
- Lore reply: costs `LINE_NOISE = 1`, temper +0, progress 2.
- Wrong reply: costs `per`, temper +0, progress 1.
- Offending reply: costs `per * ANGER_MULT (2)`, temper +1, progress 0.
- Hang up: costs 0 now, temper +2, ends the call.
- After any reply: sanity -= cost; temper += delta; **if delta > 0 and the
  new temper >= TEMPER_LIMIT (3), the being screams: sanity -= WRATH (25) and
  the call ends** (so anger is worse than the drain twice over: double cost
  now, and a scream that costs more than a whole no-lore call, after which
  its drain is doubled until a cult visit brings temper below 3). Otherwise
  patience -= progress and the call ends at patience 0 or at the sixth
  exchange. Sanity <= 0 at any point: lost.
- Why these numbers: `START_SANITY 90`, `PATIENCE 4`, three questions a
  being. A no-lore call costs `4 * drain`; one secret makes it `1 + 2 *
  drain`; two secrets make it 2. Over the seven calls of a run the Scholar
  pays at most 61, the Guesser (meditating, never studying) at least 108 and
  dies on night 5, the Blasphemer dies on night 2 (two screams plus six
  doubled exchanges exceed 90 on every rotation). Those bands are G5's
  literals.

**The day loop and the schedule** (decision row 2's one function, `tonight`):

- `rotation` is a seeded permutation `[p0, p1, p2]` of the beings, drawn once
  per run from the world `Rng`. Night `d` (1-based): first caller
  `p[(d - 1) % 3]`; on nights 4 and 5 a second caller `p[d % 3]`. A cult
  attended today for a being not already calling tonight **replaces the
  first caller**; for one already calling, nothing in the schedule moves.
  `per_exchange` per call is `answer_outcome(.., Wrong).sanity_cost` with
  today's meditation and the being's temper.
- One morning action a day. `Study(X)` marks X's lowest unknown fact known
  (questions are asked in that same order, so the first secret is always the
  one tonight's first question wants). `Meditate` sets `meditated` for
  tonight only. `Cult(X)` lowers X's temper by 1 (never below 0) and draws
  X's call tonight. Effect lines, exactly: `learn secret k of 3: tonight X's
  question k costs 1, not <per>` / `learn secret k of 3: X does not call
  tonight` / `tonight each exchange costs 1 less: X <d>-><d-1>` (`, Y
  <d>-><d-1>` appended on a two-call night) / `X's temper <t>-><t-1>; X calls
  tonight instead of <first caller>` (`X's temper stays 0;` at 0; `X calls
  tonight anyway` when already calling).
- Five days; after night 5's Dawn, Enter shows the won End.

**Screens as data.** Every screen is one `Panel` from one function, the draw
system draws exactly it, and the transcript is its strings: so the thing
drawn, the thing judged and the thing asserted are one list (jidousha-ui.md's
first principle). The lore reply is listed only when its fact is known —
that is what makes learning lore the mechanic rather than a hint.

**Pure logic, thin ECS.** `step`, `tonight`, `answer_outcome`,
`call_options`, `morning_options` and `effect_line` take and return plain
values; the single Update system is the only place `Input` is read and the
only caller of `step` in the window build, so the controllers can call `step`
through the sim and the arithmetic gates can call the functions directly.

**Elaborations of the spec's two decision rows, never a third.** The Hang up
option, the `*` marker and the footer hint are the call-screen row's "options"
and "on-screen hint"; the effect lines and the `tonight:` preview are the
morning row's "effect on tonight stated on the option". No decision is added.

## Open calls delegated to the implementer

- **Exact row y-positions and wrapping** within §Layout, provided no floor is
  crossed and `judge_panel` is clean on every staged screen.
- **The reaction line's wording** per outcome (satisfied / lingers / offended
  / hung up / screams / line dies), provided each names the sanity cost and
  the new temper as numbers and stays printable ASCII.
- **How the three players read the option index**: by matching `AnswerKind` /
  `MorningAction` from `call_options()` and `morning_options()`, never by a
  hard-coded digit.
- **File split** if `flow.rs` or `verify.rs` passes ~500 lines (CLAUDE.md
  convention 5): split by screen or by gate group; the function names above
  stay.
- **The mutation list's exact find strings**, which depend on the code as
  written; the twenty faults themselves are fixed above.
