# DESIGN — brolf

task: brolf
variant: V2
model-as-configured: claude-fable-5-1
date: 2026-10-07 09:23 PDT

Read with: the spec `tools/yakin/tasks/brolf.md` (the decision table there is
binding; this file elaborates its rows and adds none), `docs/api/` all five,
`crates/jidousha/examples/prototype_kit/` (the four-file shape to copy) and
`crates/jidousha/examples/slalom/` (the controller shape). Never `crates/**/src`.

Every number in this file is a decision. Where the implementer finds one wrong
in practice, the number changes and the change is a Deviation in CHECKPOINT.md
and the PR — not a silent retune.

## What the game is

Brolf is top-down 2D golf played as a battle royale on one screen: six golfers
(you and five NPCs) each walk their own ball toward one hole while a circular
safe zone shrinks in five seeded steps, and anyone whose body **or** ball stays
outside it past a five-second grace is out. Players can swing at each other
(a three-second daze that drops an item, never a kill) and at each other's
resting balls (knocking them four units away), and gather four kinds of
equipment that trade reach, resilience and aggression. You win by holing out,
by being the last one standing, or by being closest to the pin when time runs
out; or you leave early by **extracting** on a pad near the hole, keeping every
item you hold while the others fight on.

## The player-facing loop

1. **Tee.** Six bodies stand on an ellipse round the course, each with its ball
   a step in front. The top band reads the zone (`ZONE r16.5 -> r8.0 in 20s`)
   and your standing; the bottom band says `walk to your ball`. The hole (a
   grey covered disc until it opens at 95 s, then dark with a flag) and the
   extraction pad (a dotted disc two units beyond it, grey until it opens at
   45 s) are visible from the first frame, so where the zone will end is
   never a secret: the final zone is the hole.
2. **Walk** with WASD. When you are within 1.2 units of your resting ball the
   aim preview appears: a dotted reach ring round the ball, a line from the
   ball to the pointer (clamped to your reach), a landing disc at its end
   whose colour says whether that spot is inside the zone *by the time the
   ball lands and you walk there* (green) or not (red), the current zone as a
   dotted blue ring and the next zone as a dotted white ring. Click to shoot.
   The ball flies straight there at 12 u/s and stops; shots scatter by up to
   12% of their length, so a hole-out is a two-unit putt, not a seven-unit
   drive. Walk after it; shoot again.
3. **The zone moves.** Five stops, 20 s holds, 15 s shrinks. While you or
   your ball are outside it the top band counts down `OUT 4.3s` in red; at
   zero you are eliminated and the result screen shows it.
4. **Contact.** When a rival body is within 1.5 units the bottom band reads
   `SPACE: club P3 (dazes 3.0s, drops helmet)` and an orange dotted ring
   marks them; when only a rival's resting ball is in reach it reads
   `SPACE: strike P3 ball (knocks it 4.0 away from the hole)`. Space does
   exactly what the line says. A dazed player cannot walk, shoot or swing for
   the stated time, drops their most recent item at the clubber's feet, and
   is never removed by the club. A struck ball flies 4 units straight away
   from the striker.
5. **Equipment.** Eight items sit on a ring round the course, two of each
   kind, labelled by three letters. Within 2.5 units the bottom band's second
   line reads what the nearest one does and what it replaces
   (`TAKE heavy ball: strikes on it x0.5, your reach x0.75 (replaces light ball)`);
   walk onto it to take it. Two slots: a ball mod and a body mod.
6. **Extract or fight on.** The top band's second line always reads what
   extraction keeps, whether the pad is open and for how long, how far your
   ball is from the hole, how many rivals are left and where your ball ranks
   by distance to the pin. Stand on the pad with your ball on it, press E,
   stay two seconds: extracted.
7. **Result.** One screen, centred: the outcome in a word or two, what you
   kept, your placing. A rival holing out first ends your match as `LOST`,
   keeping nothing. The match is over; the window is closed by the player
   (no restart — Non-goals).

NPCs run the same loop through the same `Intent` type as the keyboard, with
three personas (Golfer, Hunter, Extractor) so that the zone, contact and
extraction all happen to the player whether or not they go looking.

## Systems

All in `games/brolf/src/`, in build order. Every file ≤ ~500 lines. Pure rule
functions take plain values (never `&mut World`) so `verify.rs`, the NPCs and
the draw systems all call the same one.

- **layout** — `main.rs` constants: `WINDOW = PhysicalSize::new(1280, 720)`,
  `HALF_H = 9.0`, `HALF_W = HALF_H * WINDOW.aspect()` (`const`, the Concepts
  idiom); `COURSE_HALF = Vec2::new(14.0, 7.4)` (the playable rect, origin
  centred); the top band `y in [-9.0, -8.0]` and bottom band `y in [8.0, 9.0]`
  each hold two text lines of size 0.42 at `y = -8.95 / -8.50` and
  `8.08 / 8.50`; `mod layers { COURSE = -1, ZONE = 0, PLAY = 1, AIM = 2, UI = 3 }`.
  Camera: `center ZERO, height 18.0, clear_color COURT = rgb(0.10, 0.20, 0.12),
  viewport WINDOW`, inserted in Startup. · `main.rs` · touches `jidousha-api.md`:
  `GameConfig`, `PhysicalSize::aspect`, `Camera`, `Depth`, `run`, `headless`.
- **items** — `enum Item { HeavyBall, LightBall, Helmet, BigClub }`,
  `enum Slot { Ball, Body }`, `struct Kit { ball: Option<Item>, body:
  Option<Item>, last_taken: Option<Slot> }`, `struct Effects { shot_reach,
  strike_taken, strike_given, daze_taken, walk, swing_reach, swing_cooldown:
  f32, drops_when_clubbed: bool }` with `Item::effects(self) -> Effects`
  (**the one equipment-effect function**), `Kit::effects(&self)` (the
  component-wise product of the two slots over `Effects::NEUTRAL`),
  `Item::slot`, `Item::name() -> &'static str` ("heavy ball", "light ball",
  "helmet", "big club"), `Item::tag()` ("HVY", "LGT", "HLM", "BIG") and
  `Item::describe(self) -> String`, which formats the non-neutral fields of
  `effects()` and nothing else — so the label and the sim cannot disagree.
  Values (all exact, the mutation list walks them):
  HeavyBall: `strike_taken 0.5, shot_reach 0.75`; LightBall: `shot_reach
  1.25, strike_taken 1.5`; Helmet: `daze_taken 0.5, walk 0.85,
  drops_when_clubbed false`; BigClub: `swing_reach 1.5, strike_given 1.5,
  swing_cooldown 1.5`. `Kit::take(item) -> Option<Item>` fills the slot and
  returns what it displaced. · `src/items.rs` · touches nothing in the API.
- **zone** — `struct Zone { center: Vec2, radius: f32 }` with
  `contains(p) = (p - center).length_squared() <= radius²`;
  `struct Schedule { stops: [(u64, Vec2, f32); 6] }` built by `Schedule::seeded
  (rng, hole)`: ticks and radii are the literals below, centres are
  `hole * (k / 4)` plus a seeded jitter of length `≤ JITTER[k]`, which
  guarantees every stop lies inside the one before it (a unit test asserts
  it for forty seeds). `zone_at(&Schedule, tick) -> Zone` (**the one
  zone-membership-at-time function**: piecewise-linear in centre and radius
  between consecutive stops, constant before the first and after the last);
  `next_stop(&Schedule, tick) -> Option<(u64, Zone)>` (the first stop whose
  tick is > now, or `None` after the last); `landing_safe(&Schedule, landing,
  arrive_tick) -> bool = zone_at(arrive_tick).contains(landing)`;
  `pad_closes_at(&Schedule, pad) -> Option<u64>` (the first tick ≥ `PAD_OPENS`
  at which `zone_at` excludes the pad, searched tick by tick once in Startup).
  Stops `(tick, radius)`: `(0, 16.5)`, `(2100, 8.0)`, `(3900, 5.0)`,
  `(5700, 3.0)`, `(7500, 1.2)`, `(9000, 1.2)`; a shrink begins 900 ticks
  before its stop (`SHRINK_TICKS = 900`), so the holds are `0..1200`,
  `2100..3000`, `3900..4800`, `5700..6600`, `7500..9000`. The last stop's
  centre is the hole exactly. `JITTER = [0.0, 1.5, 0.75, 0.5, 0.0]`.
  `GRACE_TICKS = 300`. · `src/zone.rs` · touches `jidousha-api.md`: `Rng`,
  `Vec2` (`lerp`, `length_squared`, `clamp_length_max`), `rotate`, `Radians`.
- **shots** — `MAX_SHOT = 7.0`, `BALL_SPEED = 12.0`, `SHOT_SPREAD = 0.12`,
  `REACH_BALL = 1.2`. `aim_point(ball, want, reach) -> Vec2` = `ball +
  (want - ball).clamp_length_max(reach)` then clamped into the course rect;
  `flight_ticks(distance) -> u64 = ceil(distance / BALL_SPEED * 60)` (at
  least 1); `scatter(aim, ball, rng) -> Vec2` = `aim + rotate(X * (r₁ *
  SHOT_SPREAD * |aim - ball|), Radians(r₂ * TAU))` with `r₁, r₂ = rng.next_f32()`
  drawn in that order, clamped into the course rect. `struct Flight { from,
  to: Vec2, start: u64, ticks: u64 }`, `Flight::at(tick) -> Vec2` (lerp by
  `(tick - start) / ticks`, `to` once past). `strike_landing(striker, ball,
  distance) -> Vec2` = `ball + (ball - striker).normalize_or_zero() *
  distance`, `+X` when the two coincide, clamped into the course.
  `STRIKE_DISTANCE = 4.0`. · `src/shots.rs` · touches `jidousha-api.md`:
  `Vec2`, `Rng::next_f32`, `rotate`, `Radians::TAU`, `Rect`.
- **contact** — `SWING_REACH = 1.5`, `SWING_COOLDOWN = 120`, `DAZE_TICKS =
  180`. `enum Contact { Club { who: usize, daze_ticks: u64, drops:
  Option<Item> }, Strike { whose: usize, lands_at: Vec2 } }`;
  `contact_target(me: &PlayerSnap, rivals: &[PlayerSnap], tick) -> Option<Contact>`
  (**the one contact-resolution function**): `None` if I am dazed,
  extracting, or `tick < swing_ready_at`; else the nearest live rival body
  within `SWING_REACH * my.effects.swing_reach` wins (`Club`, with `daze_ticks
  = round(DAZE_TICKS * their.effects.daze_taken)` and `drops = their most
  recent item if their.effects.drops_when_clubbed`); else the nearest rival
  *resting* ball within the same reach (`Strike`, `lands_at =
  strike_landing(me.pos, ball, STRIKE_DISTANCE * my.strike_given *
  their.strike_taken)`); ties by lower player index. `PlayerSnap` is the
  plain `Copy` snapshot of one player (index, pos, ball pos, ball resting?,
  dazed_until, swing_ready_at, extracting?, kit, effects, alive) that every
  pure function here takes; `snap_players(&WorldView) -> Vec<PlayerSnap>` is
  the one reader, sorted by player index. · `src/contact.rs` · touches
  `jidousha-api.md`: `WorldView`, `World::view`.
- **outcome** — `enum Outcome { HoledOut, LastStanding, ClosestToPin,
  Extracted, Eliminated, Lost { to: usize } }` (`Lost`: an NPC holed out
  first); `kept_on(outcome, &Kit) -> Kit` (**the one outcome function**:
  `Eliminated` and `Lost` keep nothing, every other outcome keeps the kit as
  held); `kept_text(&Kit) -> String` ("heavy ball, helmet", or "nothing");
  `enum PadState { Closed { opens_at }, Open { closes_at: Option<u64> },
  Gone }` from `pad_state(&Schedule, pad, tick)`; `enum Placing { Won, Out
  { place: usize }, Left { still_in: usize }, Lost { to: usize } }`; `struct
  Standing { hole_distance: f32, hole_open: bool, rivals_left: usize,
  pin_rank: usize, pad: PadState }` from `standing(&[PlayerSnap], me, hole,
  tick)`; `settle(&[PlayerSnap], eliminated: &[usize], hole, tick) ->
  Option<(Outcome, Placing)>` decides the match-ending event for the human
  in this order: human extracted (`Left { still_in: rivals_left }`) → human
  eliminated (`Out { place: 7 - eliminated.len() }`, the human included) →
  `tick >= HOLE_OPENS` and a ball resting within `HOLE_RADIUS = 0.35` of the
  hole (lowest index wins a tie: the human → `HoledOut, Won`; an NPC `k` →
  `Lost { to: k }`) → no live rival left → `LastStanding, Won` → `tick >=
  MATCH_END` → `ClosestToPin, Won` if the human's resting ball is nearest
  the hole among live players, else `Lost { to: that player }`.
  `HOLE_OPENS = 5700` (**the hole is covered until stop 3**: a ball resting
  on it before then just rests, and can be struck away; this is what stops
  a Golfer ending the match in forty seconds before the zone has moved),
  `PAD_OPENS = 2700`, `PAD_RADIUS = 1.2`, `EXTRACT_TICKS = 120`, `MATCH_END =
  9000`. · `src/outcome.rs` · touches nothing in the API.
- **world** — components and resources: `Player { index: usize, persona:
  Persona, kit: Kit, dazed_until: u64, swing_ready_at: u64, exposure: u64,
  extracting_since: Option<u64>, shots: u32 }`, `Ball { owner: usize, flight:
  Option<Flight> }`, `Pickup(Item)`, all with `Transform`; resources
  `Course { hole: Vec2, pad: Vec2, schedule: Schedule, pad_closes: Option<u64> }`,
  `MatchState { Playing, Over { outcome: Outcome, kept: Kit, placing: Placing } }`,
  `Eliminated(Vec<usize>)` in order of leaving (for placing), `Intents(Vec<(usize,
  Intent)>)` (this tick's decisions, written by `decide`, read by every
  apply system), and `Pointer(Vec2)` (the pointer in world units, updated
  from `Input` each tick so Draw can read it). `reset_match(&mut World, seed)`
  spawns everything: human index 0, NPC personas `[Golfer, Hunter, Extractor,
  Golfer, Hunter]` for indices 1..5; bodies at `(12.0 cos θ, 6.2 sin θ)`,
  `θ = phase + k·60°`, `phase = rng.next_f32() * TAU`; each ball `0.8` toward
  the origin from its body; the hole at `polar(rng angle, 1.5 + 1.5 *
  rng.next_f32())`; the pad at `hole + hole.normalize() * 2.0`; eight pickups
  at `(7.0 cos φ, 4.2 sin φ)`, `φ = phase₂ + k·45°`, kinds cycling
  `[HeavyBall, LightBall, Helmet, BigClub]`. RNG draws happen in exactly this
  order (phase, hole angle, hole length, phase₂) so a seed names one course.
  · `src/world.rs` · touches `jidousha-api.md`: `Component`, `Resource`,
  `World::spawn/insert/insert_resource`, `Transform::at`, `Rng`.
- **intent** — `struct Intent { walk: Vec2, shoot: Option<Vec2>, swing: bool,
  extract: bool }` (`walk` is a direction, length ≤ 1; `shoot` is the
  wanted point in world units). `intent_from_input(&Input, &Camera) ->
  Intent`: WASD → `walk`, `pointer().screen` through `camera.screen_to_world`
  → the pointer, `just_pressed(PointerButton::Primary)` on the pointer →
  `shoot = Some(pointer)`, `just_pressed(Key::Space)` → `swing`,
  `just_pressed(Key::E)` → `extract`. `npc_intent(me: &PlayerSnap, all:
  &[PlayerSnap], course: &Course, tick) -> Intent` (the NPC decision as a
  free function), in priority order:
  1. dazed, extracting, or not alive → `Intent::NONE`.
  2. my ball is resting and `!landing_safe(schedule, ball, tick + 120)`, or my
     body is outside `zone_at(tick)`: walk to my ball; if within `REACH_BALL`,
     `shoot = Some(safe_aim(..))`.
  3. `contact_target(me, rivals, tick)` is `Some(Club)` and the target holds
     an item, or `Some(Strike)` and their ball is nearer the hole than mine →
     `swing`.
  4. Hunter: a rival body within 5.0 units → walk toward it.
  5. Extractor: I hold ≥ 1 item and the pad is `Open` → if my ball is not on
     the pad and I am within reach of it: `shoot = Some(pad)`; else walk to my
     ball, or to the pad once the ball is on it; `extract` when my body and
     ball are both on the pad.
  6. otherwise walk to my ball; within reach, `shoot = Some(safe_aim(..))`.
  `safe_aim(me, course, tick) -> Vec2`: `target = hole` once `tick >=
  HOLE_OPENS`, else the lay-up point `hole + (ball - hole).normalize_or_zero()
  * 1.5`; `candidate = aim_point(ball, target, reach * 0.9)`; if
  `landing_safe(schedule, candidate, tick + flight_ticks(|candidate - ball|)
  + 300)` then `candidate`, else `aim_point(ball, next-stop centre (or the
  current centre after the last stop), reach * 0.9)`. `reach = MAX_SHOT *
  effects.shot_reach`. · `src/intent.rs` · touches
  `jidousha-api.md`: `Input::held/just_pressed/pointer`, `Key`,
  `PointerButton`, `Camera::screen_to_world`.
- **systems** — the `Update` phase, registered in **exactly this order**
  (the schedule gate asserts it): `decide` (fills `Intents`: index 0 from
  `intent_from_input`, when an `Input` resource exists, else `NONE`; NPCs
  from `npc_intent` over one `snap_players` taken at the top of the tick) ·
  `walk` (`pos += walk * WALK_SPEED * effects.walk * fixed_dt`, `WALK_SPEED =
  4.0`, clamped to the course; dazed or extracting players do not move) ·
  `swing` (for each `swing` intent, `contact_target` again on this tick's
  snaps; `Club` sets the victim's `dazed_until = tick + daze_ticks`, cancels
  their extraction, drops `drops` as a `Pickup` at `victim + (me -
  victim).normalize_or_zero() * 1.0`; `Strike` gives the ball a `Flight` to
  `lands_at`; either sets my `swing_ready_at = tick + round(SWING_COOLDOWN *
  effects.swing_cooldown)`) · `shoot` (a `shoot` intent from a player within
  `REACH_BALL` of their own resting ball, not dazed, not extracting: `aim =
  aim_point(ball, want, reach)`, `to = scatter(aim, ball, rng)`, `Flight
  { from: ball, to, start: tick, ticks: flight_ticks(|to - ball|) }`,
  `shots += 1`) · `fly` (every ball with a flight moves to `flight.at(tick)`;
  at `tick >= start + ticks` it rests at `to` and the flight is cleared) ·
  `pickup` (a live, undazed player within `PICKUP_RADIUS = 0.6` of a pickup
  takes it — nearest player by index wins a tie — and the displaced item, if
  any, is respawned as a pickup at that spot) · `expose` (per live player:
  `outside = !zone_at(tick).contains(body) || !zone_at(tick).contains(ball
  now)`; `exposure = outside ? exposure + 1 : 0`; when `exposure ==
  GRACE_TICKS` the player is eliminated: pushed onto `Eliminated`, body and
  ball despawned via `commands`, items **not** dropped) · `extract`
  (`extract` intent with body and ball both within `PAD_RADIUS` of the pad and
  `pad_state == Open` starts `extracting_since = Some(tick)`; it ends at
  `tick - since >= EXTRACT_TICKS` if still on the pad and undazed, marking the
  player extracted; leaving the pad or a daze clears it) · `settle` (if
  `MatchState::Playing`, `settle(..)` → `MatchState::Over { outcome, kept:
  kept_on(outcome, human kit), placing }`; a `Stats` resource counts dazes,
  strikes, NPC extractions and NPC zone eliminations as they happen, for
  gate E5). · `src/systems.rs` · touches `jidousha-api.md`: `Time`,
  `Commands::despawn/spawn`, `query_mut`, `resource_mut`, `Rng`.
- **draw** — the `Draw` phase: `draw_course` (court rect `COURSE_HALF * 2` in
  `COURSE_FILL`, four edge lines, the hole as a `0.35` disc — grey while
  covered, near-black with a `0.9`-tall flag line once `tick >= HOLE_OPENS`
  — the pad as 16 dots at `PAD_RADIUS` in grey / yellow when open / dark red
  when gone) · `draw_zone` (the current zone as
  48 dots of side `0.18` in `ZONE_NOW = rgb(0.3, 0.6, 1.0)` and the next
  stop's zone as 36 dots in white, **skipping any dot whose rect is not
  inside the course rect** — so the opening 16.5 zone draws nothing and the
  "nothing outside the camera" check holds) · `draw_players` (bodies as
  `0.35` discs — human white, NPCs per-persona hues — a `0.4`-size tag
  `P1..P5` / `YOU` to the **right** of the body at `(x + 0.45, y - 0.2)`;
  dazed bodies drawn at half alpha with `"DAZED"` under the tag; balls as
  `0.18` discs in the owner's hue; pickups as `0.3` squares with the
  three-letter tag to their right; the contact target ringed by 12 orange
  dots at `0.5`) · `draw_aim` (only when `aiming(me)`: human alive, undazed,
  not extracting, ball resting and within `REACH_BALL`; the reach ring as
  24 dots at `MAX_SHOT * shot_reach` skipping dots outside the course, a
  `0.06` line from ball to `aim_point(ball, pointer, reach)`, a `0.15` disc
  there, and a translucent disc of radius `SHOT_SPREAD * |aim - ball|` in
  green if `landing_safe(schedule, aim, tick + flight_ticks + walk_ticks)`
  else red, `walk_ticks = ceil(|aim - body| / (WALK_SPEED * walk) * 60)`) ·
  `draw_bands` (the four band lines from the four string functions below,
  each `ctx.text` at its band line's `(-15.8, y)` left-aligned, size 0.42)
  · `draw_result` (when `Over`: a `0.9`-size three-line block centred on the
  course by each line's own `width_of`, over a `COURT`-coloured rect of the
  block's measured size plus `1.0` margin, on `layers::UI`). The string
  functions, free and over `&WorldView` (so verify and draw read one text):
  `status_line_1(view) -> String` =
  `ZONE r{r:.1} -> r{next:.1} in {s}s   OUT {grace:.1}s   SHOTS {n}   DAZED {d:.1}s`
  with the `OUT`, `DAZED` fields present only when non-zero and `-> ...`
  replaced by `final` after the last stop; `status_line_2(view)` =
  `HOLD {kept_text}   EXTRACT {open 41s | opens in 12s | gone} keeps {kept_text(kept_on(Extracted, kit))}   HOLE {d:.1} away, {open | opens in 23s}   RIVALS {n}   PIN #{rank}`;
  `cue_line(view)` = `SPACE: club P3 (dazes 3.0s, drops helmet)` /
  `SPACE: strike P3 ball (knocks it 4.0 away from the hole)` /
  `CLICK: shoot {len:.1} -> lands in zone` / `... -> lands OUTSIDE the zone`
  / `E: extract (keeps heavy ball, helmet)` when standing on an open pad
  with the ball / `walk to your ball ({d:.1} away)` / `DAZED` / `` while
  extracting `EXTRACTING {s:.1}s`; `pickup_line(view)` = `TAKE heavy ball:
  strikes on it x0.5, your reach x0.75 (replaces light ball)` for the
  nearest pickup within `LABEL_RADIUS = 2.5`, built from `Item::describe`,
  or empty; `result_lines(view) -> [String; 3]` = `{EXTRACTED | HOLED OUT |
  LAST STANDING | CLOSEST TO PIN | ELIMINATED | LOST}` /
  `kept: {kept_text}` / the `Placing`: `placed 1 of 6` (`Won`), `placed {n}
  of 6` (`Out`), `left with {n} still in` (`Left`), `P{k} holed out first`
  (`Lost`). Every literal ASCII. · `src/draw.rs`
  (strings in `src/text.rs` if `draw.rs` passes 500 lines) · touches
  `jidousha-api.md`: `Submit::rect/line/circle/text`, `TextStyle::width_of/
  measure`, `Depth`, `Color`, `WorldView`.
- **main** — `GameConfig { title: "brolf", seed: 7, window_size: WINDOW,
  ..default }`; `register(app)` adds `Startup: set_up` (camera, `reset_match
  (world, seed)` from `Rng`), the Update systems in the order above, then the
  six Draw systems; `main` takes `--verify` exactly as the testing document's
  closing convention shows, `#![allow(missing_docs)]` at the crate root.
  `Cargo.toml`: the `prototype_kit`/`keifu` shape — four `.workspace = true`
  lines, `jidousha = { path = "../../crates/jidousha" }` and nothing else,
  `[lints] workspace = true`. · `games/brolf/Cargo.toml`, `src/main.rs` ·
  touches `jidousha-api.md`: `run`, `headless`, `App::add_system`, `RunError`.
- **players** — the three controllers of `jidousha-controllers.md`, each a
  `fn(&WorldView, tick, &mut SnapshotBuilder, &Camera)` that emits
  `InputEvent`s for player 0 (events, never states: track what is held) and
  a shared `Report { shots, in_zone_at_arrival, aimed_from_hole: f32,
  landed_from_plan: f32 }`: `Idle` (nothing, ever) · `Golfer` (walk to own
  ball; in reach, move the pointer to `safe_aim(..)` through
  `camera.world_to_screen` and click) · `Full` (Golfer, plus Space whenever
  `contact_target` is `Some`, plus the Extractor rule from `npc_intent` once
  holding two items). Pointer events: `PointerMoved { id: PRIMARY, screen }`
  then `ButtonPressed`/`ButtonReleased { Primary }` on the next tick.
  · `src/players.rs` · touches `jidousha-testing.md`: `SnapshotBuilder`,
  `InputEvent`, `Input::new`; `jidousha-api.md`: `Camera::world_to_screen`.
- **verify** — `verify::run() -> ExitCode`: the sessions and gates of the
  next section, failures collected in a `Vec`, the verdict `verified brolf:
  ...` first, the indented summary (the three controller numbers per player,
  the clearance margin, the capture line), then one frame transcript. `--verify`
  runs on `seed: 7` and `fixed_dt` default; `HEADLESS_VIEWPORT = WINDOW`.
  · `src/verify.rs`, `src/checks.rs` (the helpers: `fail`, `near`,
  `font_quads_in(frame, band)`, `dots_at_radius(frame, color, center, r)`,
  `disc_at(frame, center, radius)`) · touches `jidousha-testing.md`:
  `FrameRecorder`, `FrameRecord::quads/covering/transcript`, `find_bounds`,
  `InputScript`, `InputSnapshot`, `HeadlessSim::schedule_debug/world_mut`.
- **capture** — `capture::write(frame, path) -> Result<..>` copied from
  `prototype_kit/capture.rs` minus the asset half (a shapes-and-text game:
  `create_builtin_textures` is the whole table), `CAPTURE_SIZE = 480x270`,
  the `NoAdapter` skip, the exact line `capture: <path> written to
  target/verify/brolf.png`. · `src/capture.rs` · touches
  `jidousha-capture.md`: `WgpuBackend::offscreen/poll`, `RenderBackend::
  render/capture`, `create_builtin_textures`, `upload_text_atlases`,
  `encode_png`, `RenderError::NoAdapter`.
- **mutants** — `games/brolf/mutants/m0.txt`, the round of "Gates to add".
  · `games/brolf/mutants/m0.txt`, `games/brolf/FINDINGS.md`.

## Gates to add

All on `seed 7`, `fixed_dt 1/60`, `HEADLESS_VIEWPORT 1280x720`, through one
`FrameRecorder` per session that reads frames and none for the controller
sweeps. Literal expectations are written as literals in the check, never as
arithmetic over the constant under test. Each `--verify` failure prints the
numbers it judged.

**Session A — the zone row (spec row 1).** The `Idle` player, 9300 ticks, a
frame every tick.
- A1 · asserts: `first_out` = the first tick at which `zone_at(schedule,
  tick)` (called by the check on the `Course` resource read from the world)
  excludes the human's body or ball, is found and lies in `1201..=7500`
  (the start ellipse may or may not sit inside S1; the final zone excludes
  every start); it is printed;
  the human is alive on tick `first_out + 298` and eliminated on
  `first_out + 299` exactly, with `MatchState::Over { outcome: Eliminated,
  kept: empty }` on that tick — the literal `299` in the check. · covers
  Done-when: "a scripted --verify run reaches its result screen"; spec row 1
  "eliminated on the tick the function says".
- A2 · the frame drawn on `first_out + 60`: `status_line_1` contains
  `OUT 4.0s` (literal); its character count equals the number of font quads
  in the top band's first line (`y in [-8.95, -8.53]`); on `first_out - 1`
  the line contains no `OUT`. · spec row 1 "the grace countdown ... in the
  transcript".
- A3 · the staged aiming frame, a separate session: after `Startup`,
  teleport the human's ball to `S1.center + (2.0, 0)` and the human to
  `S1.center + (2.8, 0)` (inside S1 whatever seed 7 says), put the pointer
  at `camera.world_to_screen(ball + (4.0, -3.0))` through the script, and
  run to tick `3000` with no other input; on the frame of tick 3000 assert: ≥ 24 `ZONE_NOW`-tinted dots whose centres lie within `0.12` of
  distance `8.0` from `zone_at(3000).center` (literal `8.0`); ≥ 18
  white dots at distance `5.0` (literal) from the next stop's centre; one
  line quad whose bounds span from the ball to within `0.1` of
  `aim_point(ball, pointer, 7.0)` (literal reach `7.0`); a disc of diameter
  `2 * 0.12 * 5.0` (literal) at that point; and 24 reach dots at `7.0`
  minus those outside the course. · spec row 1 "current and next zone ... in
  the transcript while aiming".
- A4 · `cue_line` on that frame starts with `CLICK: shoot 5.0` and ends with
  `lands in zone` or `lands OUTSIDE the zone` exactly as `landing_safe(..)`
  answers for `tick + flight_ticks(5.0) + walk_ticks`.

**Session B — the contact row (spec row 2).** Scripted: after `Startup`,
teleport NPC `P1` (a Golfer) to `human + (1.0, 0)` and set its kit to `Kit
{ ball: Some(HeavyBall), last_taken: Some(Slot::Ball), .. }` so `drops` is
`Some(HeavyBall)`; `P1`'s ball stays where it spawned, far from the human;
run to tick `200` with no input; then `press(Key::Space, 200)`.
- B1 · the frame of tick 199: `cue_line` == `SPACE: club P1 (dazes 3.0s,
  drops heavy ball)` (literal string); its char count equals the font quads in
  the bottom band's first line; 12 orange dots ring `P1`. · spec row 2 "the
  reach cue is in the transcript first".
- B2 · after tick 200: `P1.dazed_until == 380` (literal; `200 + 180`);
  `P1`'s `Transform` is identical on ticks 200 and 379 (a dazed Golfer
  cannot walk to its ball) and differs by tick 420 (an undazed one does);
  `P1` is alive and has a `Transform` on tick 1200; on tick 200 a
  `Pickup(HeavyBall)` exists at `P1 + (human - P1).normalize() * 1.0`, and
  by tick 201 the human — standing within `0.6` of it — holds `kit.ball ==
  Some(HeavyBall)` and the pickup is gone. · spec row 2 "disabled for
  exactly the stated time and never permanently removed".
- B3 · the strike half: a second scripted session, `P1` teleported to `human +
  (3.0, 0)` and `P1`'s ball to `human + (1.0, 0)`; Space on tick 200;
  assert `P1`'s ball has a `Flight` to exactly `strike_landing(human, ball,
  4.0)` (literal `4.0`) and rests there on `200 + flight_ticks(4.0)`.

**Session C — the equipment row (spec row 3).** Scripted: after `Startup`,
teleport a `Pickup(HeavyBall)` (the first one in query order) to `human +
(2.0, 0)`; drive the human with `hold(Key::D, 2..200)`.
- C1 · the first frame at which `|human - pickup| <= 2.5`: `pickup_line` ==
  `TAKE heavy ball: strikes on it x0.5, your reach x0.75` (literal, no
  "replaces" since the slot is empty); char count == font quads on the
  bottom band's second line. Then on a frame before it, the line is empty.
- C2 · on the first tick `|human - pickup| <= 0.6`: `human.kit.ball ==
  Some(HeavyBall)` and the pickup is gone.
- C3 · the stated effect is what the sim does. On tick 220 (the human now
  still and holding the heavy ball) teleport the human's ball to `hole +
  (0.5, 0)` resting, `P1` to that ball `+ (-1.0, 0)`, and `P1`'s ball to the
  far corner `(-13.0, -7.0)`: `npc_intent` rule 3 then strikes (the human's
  ball is nearer the hole than `P1`'s, the human's body is out of reach, and
  nothing in the opening zone is unsafe), so by tick 222 the human's ball
  carries a `Flight` whose `to - from` has length `2.0` (literal: `4.0 *
  0.5`), and it rests there on `start + flight_ticks(2.0)`. Then on tick 390
  teleport the human's ball to `(0, 0)` and the human to `(0.5, 0)`, move
  the pointer to `camera.world_to_screen((20.0, 0))` and click on 400:
  the `Flight`'s `|to - from|` lies in `5.25 * (1 ± 0.12)` (literal `7.0 *
  0.75`, scatter allowed for). · spec row 3.
- C4 · `Item::describe` round trip: for each of the four items,
  `describe()` mentions every non-neutral `Effects` field and no neutral one
  (a unit test named `an item's label names every effect it has and none it
  lacks`).

**Session D — the extract row (spec row 4).** Scripted: after `Startup`, set
the human's `Kit { ball: Some(HeavyBall), body: Some(Helmet) }`, teleport
human and ball onto the pad (`pad + (0.2, 0)` and `pad - (0.2, 0)`); run idle
to tick 2800 (the zone excludes the pad only after 3900 — assert
`pad_state(2800) == Open`); frame on 2799; `press(Key::E, 2800)`.
- D1 · `status_line_2` on 2799 contains `EXTRACT open` and `keeps heavy
  ball, helmet` (literal) and `HOLD heavy ball, helmet`.
- D2 · `extracting_since == Some(2800)`; on tick 2919 `MatchState::Playing`;
  on tick 2920 `MatchState::Over { outcome: Extracted, kept: { HeavyBall,
  Helmet }, placing: Left { still_in: 5 } }` (literal `2920 = 2800 + 120`),
  and `result_lines` == `["EXTRACTED", "kept: heavy ball, helmet", "left
  with 5 still in"]` — the kept line is the same literal as D1's promise.
  · spec row 4 "the result screen keeps exactly what the status promised".
- D3 · the staged result frame: the three lines are each centred on `x = 0`
  within `0.05` by their own bounds, inside the course rect, and
  `covering(course centre)[0]` is a glyph or the backing rect, not a zone dot
  (the UI band is above the zone band).
- D4 · a sibling staged session where the human is eliminated with the same
  kit: `kept_text == "nothing"` and the result's second line is `kept:
  nothing` — the pair that proves `Extracted` and `Eliminated` are different
  screens.

**Session E — the three players (controllers doc, make-game §A.5).** `Golfer`
and `Full`, each one session to `MatchState::Over` or tick 9300, no recorder;
one verdict line each, and the three numbers per player.
- E1 · `Golfer` reaches `Over` with an outcome other than `Eliminated` (the
  middle line: a first-try player survives the zone; `Lost` to an NPC's
  hole-out counts as surviving it) and takes ≥ 6 shots.
- E6 · no `HoledOut` or `Lost` outcome in any session settles before tick
  `5700` (literal), and the unit test `a ball in the hole before the hole
  opens does not end the match` calls `settle` on ticks 5699 and 5700.
- E2 · `Full` reaches `Over`; its report prints `shots N, M in zone at
  arrival`, `aimed X from the hole`, `landed Y from plan`; asserts `M >=
  N * 0.8` and `Y <= 0.12 * mean shot length + 0.01`.
- E3 · the `Idle` line from Session A: `Eliminated`. Together: lost /
  survived / survived-or-won, the three lines.
- E4 · determinism: `Full` run twice; identical `MatchState`, identical
  human `shots`, identical `Transform` of every ball on the last tick.
- E5 · at least one NPC is dazed and at least one NPC ball is struck during
  the `Full` run (counted by a `Stats` resource the systems bump), and at
  least one NPC extracts or is eliminated by the zone before `MATCH_END` —
  the "NPCs good enough to make the zone, contact and extraction matter"
  line of the spec. If a count is zero the message prints all three counts.
· covers Done-when: "One full match loop against NPC opponents plays end to
end".

**Session F — every frame.** Over Sessions A–D's recorded frames:
- F1 · nothing outside `camera.visible_bounds()` (`contains_rect`), with the
  clearance printed in the summary.
- F2 · every font quad lies in one of the two bands or inside the course
  rect; every band-line's quads lie in that line's `y` range.
- F3 · every zone dot, player, ball and pickup quad lies inside the course
  rect (the layout requirement, not the constant).
- F4 · `frame.plan.clear_color`'s brightest channel `< 0.25` and `== COURT`.
- F5 · every string literal the game draws, plus every `Item::describe` and
  `result_lines` output, is printable ASCII (`' '..='~'`).
- F6 · `schedule_debug()`: `decide` < `walk` < `swing` < `shoot` < `fly` <
  `pickup` < `expose` < `extract` < `settle` in `Update`, every name found.
- F7 · staged screens: one frame for each of the six `Outcome`s, the dazed
  human, the `pad gone` status, the open hole, and the `final` zone status
  (`tick 9100`), all under F1–F5.
- F8 · the capture: the Session A3 aiming frame to `target/verify/brolf.png`,
  the `capture:` line in the summary, aspect `480/270 == 1280/720`.

**Unit tests** (`cargo test -p brolf`, named as sentences):
`every zone stop lies inside the one before it, for forty seeds`;
`zone_at is constant through a hold and linear through a shrink`;
`a kit's effects are the product of its two slots`;
`contact prefers a body to a ball and the nearer of two bodies`;
`a strike from on top of the ball goes plus x`;
`kept_on eliminated or lost is empty and kept_on anything else is the kit`;
`settle orders extraction before elimination before a hole-out`;
`a ball in the hole before the hole opens does not end the match`.

**Mutation round** — `mutants/m0.txt`, ≥ 14 faults, each caught by a gate
above: `GRACE_TICKS 300→240` (A1) · `DAZE_TICKS 180→120` (B1/B2) ·
`STRIKE_DISTANCE 4.0→3.0` (B3) · HeavyBall `strike_taken 0.5→0.6` (C3) ·
HeavyBall `shot_reach 0.75→1.0` (C3) · `EXTRACT_TICKS 120→60` (D2) ·
`kept_on` Extracted → empty (D2) · `kept_on` Eliminated → kit (D4) · stop 1
radius `8.0→7.0` (A3) · stop 2 radius `5.0→4.0` (A3) · `SHOT_SPREAD
0.12→0.2` (A3/E2) · `MAX_SHOT 7.0→6.0` (A3) · `PAD_OPENS 2700→3000` (D1) ·
`HOLE_OPENS 5700→5000` (E6) ·
swap `swing` and `fly` registration (F6) · `Club` reach check `<=`→`<`
with P1 exactly at 1.5 (B1) · zone dots drawn in `ZONE_NEXT` colour (A3) ·
`contains` `<=`→`<` (A1, the first_out tick). Report `N of N noticed` in
the PR; an escape is a loose gate to tighten, not a fault to drop.

## Non-goals

Cut so the build fits the window, or outside a 2D prototype:

- **Multiplayer**, networking, any second human — the spec says so.
- **A restart** (Enter to play again): the result screen is terminal; the
  player closes the window. Reseeding the match in a running world is a
  second spawn path and a second thing to stage; not worth a night.
- **Camera panning or zoom**: the whole course fits one 16:9 screen, so the
  bounds check transfers unchanged (G-010's three-assertion form is not
  needed). A larger course is a later wave.
- **Ball roll, bounce, terrain, hazards, hills, wind**: a shot lands where
  it lands and stops. Roll would make the landing preview a lie or a second
  simulation; neither fits.
- **Interpolated drawing** (`Previous` + `Time::alpha`): the api document
  says ignoring `alpha` is reasonable for a prototype; everything steps.
- **Touch-only play**: the pointer works for aiming on touch, walking is
  WASD. A touch walk control is a later wave.
- **The UI kit** (`jidousha::ui`): four band lines and one centred block are
  `ctx.text` calls over free string functions; no panel, drawer, chip or
  feed exists to justify a `Panel`, and the kit's floors would be a second
  layout check beside F2. If a later wave adds a loadout drawer, it starts
  on the kit.
- **Art, sprites, fonts, sound**: shapes and the built-in face only; no
  `Assets` resource, no `settle_assets`, no `check-assets` concern.
- **More than four items, item values, a score**: every item is worth one;
  "kept" is the list, not a number.
- **NPC planning deeper than the priority list** (no rollouts, no lattice
  enumeration): the controllers doc's enumeration is for a game with a
  paddle-step lattice; here the only quantised thing is the walk, and a
  golfer that aims at the hole and keeps its ball in the zone is enough for
  the zone, contact and extraction to happen to the player.
- **Elimination by anything but the zone**: a club never kills; a strike
  never kills; only the zone does, as the spec requires.
- **Engine changes**: none needed. Every surface above exists in `docs/api/`;
  the one thing a golf game might expect and the engine lacks — an outline
  primitive for rings — is documented as deliberate and the dotted rings are
  the documented answer. No FINDINGS entry arises from this design.

## Decisions already made

Not to be relitigated; each with its reason.

1. **One `Intent` for keyboard and NPC.** `intent_from_input` and `npc_intent`
   both produce `Intent`, and one set of apply systems acts on it — so the
   human and the NPCs are provably subject to the same rules, and a scripted
   player and a persona differ only in which function filled the struct.
2. **The zone is a circle, the schedule is six stops, the last is the hole.**
   A circle is one `length_squared` compare; a seeded final centre that is
   also the hole makes "where the zone ends" readable from the first frame
   (the spec's row 1 wants the player to know where the zone will be) while
   the intermediate centres still wander. Containment is guaranteed by
   `JITTER`, not hoped for.
3. **A shot lands at a point, scattered by up to 12% of its length.** No
   roll. Exact landings would make a hole-out from seven units a certainty
   and the hole meaningless; scatter proportional to length makes approach
   play the golf, and the preview can draw the spread honestly as a disc.
4. **Both a hit on a player and a hit on a ball are one input, Space, and
   one function decides which.** The spec's row 2 is one decision ("club,
   strike, or keep playing"), so it is one key and one cue; body beats ball
   because a body is the scarcer target. Click is the shot, so the two
   verbs cannot be confused.
5. **A club drops the victim's most recent item at the clubber's feet.** This
   is the aggression incentive the spec asks to be designed explicitly:
   clubbing is how you take what someone gathered, and the drop lands within
   pickup reach of the striker. The helmet's whole point is refusing it.
6. **A strike knocks a ball straight away from the striker, four units.** One
   rule, readable in the cue, and it pays twice: the owner must walk, and
   the ball is likelier to be outside the next zone. The cue says "away from
   the hole" only when the striker stands between hole and ball — the cue
   text uses the literal landing from `contact_target`, so it never lies;
   the design's wording above is the common case.
7. **Elimination keeps nothing; extraction keeps everything held; a win keeps
   everything held and places first.** `kept_on` is the one function, and the
   status line prints `kept_on(Extracted, kit)` before you decide — that is
   row 4's "what extracting keeps", exactly as the result screen will.
8. **An NPC hole-out ends the match for the human as `Lost`.** The spec's
   loop is "someone wins or extracts, result shown". `Lost` keeps nothing,
   like `Eliminated` — which is one more reason to extract — and is its own
   variant so a check can tell a zone death from a rival's win.
8a. **The hole is covered until stop 3 (95 s).** Without it a Golfer holes
   out before the zone has moved once and nothing else in the game is ever
   reached. With it the early match is positioning, gathering and contact,
   the extraction window (45 s until the zone leaves the pad) falls *before*
   the hole opens so extracting is a real alternative to waiting for it, and
   the last two zones are a fight over one open hole.
9. **The pad opens by schedule and closes by the zone.** Opening at 45 s is a
   fixed fact the status can state; closing is `zone_at` excluding the pad,
   so the one zone function also decides extraction's window, and the
   closing time is computed once and shown.
10. **Six players, five fixed personas.** `[Golfer, Hunter, Extractor,
    Golfer, Hunter]` is not seeded: the mix is a design fact, and E5 needs
    a Hunter and an Extractor to exist every run.
11. **Shapes and text only, no UI kit, no interpolation, fixed camera** —
    Non-goals say why; together they keep the build inside the window.
12. **Grace counts consecutive ticks and resets on re-entry.** A player who
    steps back in is clean; the countdown shown is `(300 - exposure) / 60`.
13. **Tags sit to the right of bodies and items**, never above, so a body
    on the course's top edge cannot push its tag into the status band (F2).
14. **Zone rings are dots clipped to the course rect.** The opening zone is
    larger than the screen; drawing it as a disc or an unclipped ring would
    fail the one check the testing document calls the highest-value one.
15. **The `--verify` seed is 7 and the window seed is 7.** The game a person
    plays is the game the checks ran.

## Open calls delegated to the implementer

- **Exact colours** beyond `COURT`, `ZONE_NOW` (blue) and `ZONE_NEXT`
  (white): persona hues, pad states, the orange target ring. Constraint:
  the human is white, NPC hues are distinguishable from each other and from
  both zone colours, and F4 holds.
- **How the dazed state is drawn** (half alpha plus the `DAZED` word is the
  floor; a wobble is fine) and the flag's shape at the hole.
- **The `Stats` resource's exact fields** for E5, as long as dazes, strikes
  and NPC extractions/eliminations are counted.
- **Whether `text.rs` is split out of `draw.rs`**: only the 500-line rule
  decides.
- **The order in which to cut if the deadline (Thursday 04:30) nears**, each
  cut a Deviation in CHECKPOINT and the PR body: first `LightBall` and
  `BigClub` (Session C and the `replaces` clause then test only `HeavyBall`
  and `Helmet`; the pickup ring cycles two kinds), then the `Hunter` and
  `Extractor` personas (all NPCs `Golfer`; E5's contact counts then come
  from the `Full` player's own swings and its extraction). Nothing in the
  spec's four rows may be cut.
- **Seed-7 facts the checks pin at run time** (the human's start inside S1,
  the pad open at 2800): assert them and print them; if seed 7 violates one,
  change the seed in `GameConfig` and `--verify` together and record it as a
  Deviation, rather than special-casing the check.
