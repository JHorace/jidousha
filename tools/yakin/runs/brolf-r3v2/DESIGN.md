# DESIGN — brolf-r3v2

task: brolf-r3v2
variant: V2
model-as-configured: claude-fable-5-1
date: 2026-10-07 21:23 PDT

This is the cross-model handoff for `tools/yakin/tasks/brolf-r3v2.md`, which
reads `tools/yakin/tasks/brolf.md` through its overrides. Read both specs whole
first; where this document and the spec disagree, the spec wins and the
disagreement is a FINDINGS entry attributed to the design stage (WORKER.md §3).
The engine surface is `docs/api/` (all five files) and
`crates/jidousha/examples/`; nothing here needs `crates/*/src/`.

The game is **pure shapes and text**: no art files, no `Assets` resource, no
`jidousha::ui`. Every number below is a decision, not a suggestion; the
"Open calls" section at the end is the whole list of what is yours to choose.

## What the game is

Brolf is top-down golf as a battle royale on one 60 by 27 unit course: four
golfers (you and three NPCs), each with a ball, inside a circular safe zone
that shrinks toward the one cup. You walk, aim and shoot your own ball; you
can club a rival in reach (a timed disable that knocks loose their best item)
or strike their ball; you gather four kinds of equipment that change your
shots, your ball and your resilience. You win by holing out, by being the last
golfer in the zone, or by extracting through the gate early with the two most
valuable items you hold.

## The player-facing loop

In order, moment to moment, from tick 1 (there is no title screen):

1. **The course appears.** A green rectangle (the course); a cup near its
   centre; a yellow square gate in the bottom-right corner; six coloured 1x1
   pickup squares, each lettered; four golfer discs with their balls beside
   them, yours blue-white at the top-left. Six lines of HUD text sit in a band
   above the course; a one-line key legend sits below it. The zone is a ring
   around the cup, drawn only where it crosses the course, so at first (radius
   34, larger than the course) no ring is visible and HUD line 1 says so in
   numbers.
2. **You stand at your ball, so you are aiming.** An aim line runs from your
   ball to its predicted resting point, ending in a small disc that is green
   when that point is inside the zone *as it will be when you get there* and
   red when it is not. A second, dimmer ring shows that future zone. HUD line
   4 says the aim angle, the power, how far from the cup the ball will stop,
   and whether that is inside the zone at arrival. ArrowLeft/ArrowRight turn
   the aim; ArrowUp/ArrowDown step the power through PUTT, CHIP, DRIVE; Space
   shoots.
3. **The ball rolls and stops; you walk to it** (WASD). HUD line 4 reads
   `walk to your ball (12.3u)` until you are within reach again. Meanwhile
   NPCs do the same, and HUD line 1 shows the zone radius now, the next
   radius and when it is reached, your grace state, and whether the gate is
   open and for how long.
4. **A pickup is near.** Within 4 units of a pickup, HUD line 6 names it,
   states its effect in the sim's own numbers, and says what it would
   replace. Within 1.5 units, F takes it (the replaced item drops where you
   stand).
5. **A rival or their ball is in reach.** HUD line 5 names what C will do
   (`C: club ROOK - disables 3.0s, drops Driver`, or
   `C: strike WREN's ball - knocks it 20u/s along your aim`); a reach ring
   appears around you and the target is ringed. C does exactly that, then a
   1-second swing cooldown blocks shooting and contact.
6. **The zone shrinks** in three steps over three minutes. If you or your
   ball is outside it, HUD line 1 counts down `GRACE 4.2s`, your disc gets a
   red ring, and at zero you are eliminated.
7. **Extract or fight on.** HUD lines 2 and 3 always show your kit, your
   distance to the cup, the rivals left, what extracting would keep and
   score, and what holing out or outlasting would score. Standing in the
   gate with your ball in it and holding E for 2 seconds extracts you.
8. **The match ends** when you hole out, extract, are eliminated, are the last
   golfer left, an NPC holes out, or the course closes at tick 12000. A
   centred three-line result banner names the outcome, the items kept and the
   points, and the HUD freezes under it. Nothing restarts; the player closes
   the window (the engine has no quit, `docs/api/jidousha-api.md` Concepts).

## Systems

All paths are under `games/brolf-r3v2/`. `Cargo.toml` is keifu's shape: package
name `brolf_r3v2`, the four `.workspace = true` lines, the single dependency
`jidousha = { path = "../../crates/jidousha" }`, `[lints] workspace = true`.
`src/main.rs` starts with `#![allow(missing_docs)]` (make-game §A.2). Keep every
file under ~500 lines; split along the module lines below if one grows.

Build order:

- **rules** — every pure decision function the sim, the HUD and the checks all
  read; no `World` in any signature · `src/rules.rs` · touches
  `docs/api/jidousha-api.md`: `Vec2`, `Rect`, `Radians`, `Seconds`, `sin_cos`,
  `rotate`, `atan2`. Contains, with these exact names:
  - `zone_at(tick: u64) -> Zone { center: Vec2, radius: f32 }` — piecewise
    linear over `ZONE_TABLE` (below); the **one zone-membership-at-time
    function** of decision row 1, read by the drawn rings, the grace countdown
    and elimination.
  - `inside_zone(zone: Zone, point: Vec2) -> bool` — `distance <= radius`.
  - `zone_excludes_at(point: Vec2) -> Option<u64>` — first tick at which
    `inside_zone(zone_at(t), point)` is false, solved from the table segments,
    `None` if never.
  - `ball_step(pos, vel, dt: Seconds) -> BallStep { pos, vel, sunk: bool }` —
    one tick, in this order: speed becomes `max(speed - BALL_DECEL * dt, 0)`,
    then `0` if below `REST_SPEED`; `pos` advances by the **new** `vel * dt`;
    a crossing of a `COURSE` edge (inset `BALL_RADIUS`) flips that velocity
    component and mirrors the overshoot; `sunk` is true when the new `pos` is
    within `CUP_RADIUS` of `CUP` and the new speed is below `SINK_SPEED`.
  - `roll_out(pos, vel, dt) -> RollOut { rest: Vec2, ticks: u32, sunk: bool }`
    — loops `ball_step` until the ball rests or sinks. This is the aim line's
    landing point; because it *is* the sim's step, the prediction and the
    outcome cannot differ.
  - `shot_velocity(aim: Radians, power: Power, fx: Effects) -> Vec2` — unit
    vector at `aim` times `power.speed() * fx.shot_scale`.
  - `arrival_tick(now, golfer_pos, roll: &RollOut) -> u64` — `now + roll.ticks
    + ceil(distance(golfer_pos, roll.rest) / (WALK_SPEED * dt))`.
  - `contact_target(me: &GolferSnap, others: &[GolferSnap], balls: &[BallSnap],
    fx: Effects) -> Option<Contact>` where `Contact { Club(idx), Strike(idx) }`
    — nearest thing within `CLUB_REACH * fx.reach_scale` of `me.pos`, alive
    golfers and rival balls in one distance ordering, ties to the golfer;
    never my own ball. The **one contact-resolution function** of row 2 — the
    cue (HUD line 5 and the rings) and the C key both call it, and
    `resolve_contact(contact, aim, target_fx) -> ContactResult { stun_ticks,
    drops: bool, ball_vel: Option<Vec2> }` is what the C key then applies:
    Club → `stun_ticks = round(CLUB_STUN * target_fx.stun_scale)`, `drops =
    target_fx.drops_when_clubbed`; Strike → `ball_vel = Some(aim_dir *
    STRIKE_SPEED)` unless `target_fx.strike_immune`, then `None`.
  - `effects(kit: &Kit) -> Effects { shot_scale, reach_scale, stun_scale,
    strike_immune, drops_when_clubbed }` — the **one equipment-effect
    function** of row 3; `describe(item) -> String` builds HUD line 6's effect
    text *from* `effects(&Kit::only(item))`, so the words and the sim read the
    same numbers.
  - `kept_on_extract(kit: &Kit) -> Vec<Item>` — the `BAG_LIMIT` most valuable
    items, ties by category order Club, Ball, Head.
  - `outcome_of(kind: EndKind, kit: &Kit) -> Outcome { kind, kept: Vec<Item>,
    points: u32 }` — the **one outcome function** of row 4: Holed and
    LastStanding keep everything and add `WIN_BONUS`; Extracted keeps
    `kept_on_extract`, no bonus; Eliminated and Lost keep nothing, 0 points.
    HUD line 3 and the result banner both call it.
  - `clip_to_course(a: Vec2, b: Vec2) -> Option<(Vec2, Vec2)>` — the segment
    clipped to `COURSE` (Liang-Barsky, about twenty lines), `None` if none of
    it is inside. Every ring is drawn through it, which is what keeps every
    ring inside the camera at every radius.
- **world** — components and resources, the `Startup` system · `src/world.rs`
  · touches `docs/api/jidousha-api.md`: `Component`, `Resource`, `World`
  (`spawn`, `insert`, `insert_resource`), `Rng` (`next_f32`), `Camera`,
  `Transform`. Components: `Golfer { idx: u8, name: &'static str, color,
  temperament, aim: Radians, power, stun_left: u32, cooldown: u32,
  outside_ticks: u32, extract_ticks: u32, kit: Kit, alive: bool }`,
  `Ball { owner: u8, vel: Vec2 }`, `Pickup { item }`. Resource: `Match {
  result: Option<Outcome>, log: Vec<String> }`; layout numbers are `const`s,
  not a resource. Startup inserts the camera (`center ZERO, height 36.0,
  clear_color CLEAR, ..Camera::default()`), spawns four golfers and balls at
  `STARTS`, and six pickups placed by the seeded `Rng` (below).
- **npc** — `npc_decide(me: &GolferSnap, view: &Snap, goal: Goal) -> Intent`
  · `src/npc.rs` · touches `Vec2`, `Radians`, `atan2`. `Intent { walk: Vec2
  (unit or zero), turn: i8, power_step: i8, shoot: bool, contact: bool, take:
  bool, extract: bool }` is the same struct the keyboard produces, so NPCs and
  the player run through one set of apply systems. `Goal` is `Compete` or
  `Extract`; `temperament_goal(t, kit, now) -> Goal`: Banker → `Extract` once
  it holds 2+ items and `zone_excludes_at(GATE.center())` leaves time to walk
  there, hold `EXTRACT_HOLD` and still be inside grace; everyone else →
  `Compete`. `Extract`: go to my ball; at it, aim at `GATE.center()` and
  shoot the power whose `roll_out.rest` is nearest it; once the ball rests
  inside `GATE`, walk into `GATE` and hold `extract`. `Compete`, in priority
  order, each step a pure test: (1) stunned → nothing; (2) if me or my ball
  is outside `zone_at(now + 60)` → go to my ball and shoot toward
  `zone_at(now).center` with the largest power whose `roll_out` rests inside
  `zone_at(arrival_tick)` ("constrain first, then optimise",
  `docs/api/jidousha-controllers.md`); (3) Hunter only: a rival golfer within
  8 units → walk at them, `contact` when `contact_target` is `Some(Club(_))`;
  (4) a pickup within 6 units whose category I lack, or that outranks mine by
  value → walk to it, `take` within `TAKE_RANGE`; (5) otherwise go to my
  ball; at it, aim at the cup and pick the power whose `roll_out.rest` is
  nearest the cup among those resting inside `zone_at(arrival)`; shoot.
  Aiming is instant for an NPC: it writes `aim` directly (NPCs skip the turn
  rate; say so in a comment). Walking: `walk = (target - pos)
  .normalize_or_zero()`.
- **systems** — the `Update` phase, registered in exactly this order, with
  these function names (a gate asserts the order):
  `player_intent`, `npc_intents`, `apply_contacts`, `apply_walks`,
  `apply_shots_takes_extracts`, `move_balls`, `zone_grace`, `end_match` ·
  `src/systems.rs` · touches `Input` (`held`, `just_pressed`), `Key`, `Time`
  (`tick`, `fixed_dt`), `World::query_mut`, `Commands` (`spawn`, `despawn`).
  `player_intent` maps keys (WASD walk; ArrowLeft/Right `turn` -1/+1;
  ArrowUp/Down `power_step` +1/-1 on `just_pressed`; Space `shoot` on
  `just_pressed`; C `contact` on `just_pressed`; F `take` on `just_pressed`;
  E `extract` while `held`) into golfer 0's slot of an `Intents([Intent; 4])`
  resource. Contacts resolve before walks, so a cue drawn last frame is what
  C hits this tick. Turn applies `AIM_RATE * dt` per tick while held. A shot
  requires: alive, not stunned, cooldown 0, own ball at rest and within
  `SHOT_REACH`; it sets the ball's `vel` to `shot_velocity`. Taking requires
  a pickup within `TAKE_RANGE`; the replaced item (same category) is spawned
  as a pickup at the golfer's position. A club sets the target's `stun_left`
  (resetting it if already stunned), spawns the dropped item as a pickup at
  the target's position, and starts the striker's `cooldown`; a strike sets
  the ball's `vel` and the cooldown. Extract: golfer and own ball both inside
  `GATE` and `extract` held → `extract_ticks += 1`, else reset to 0; at
  `EXTRACT_HOLD` the golfer leaves play (`alive = false`, ball despawned)
  and, if it is golfer 0, `Match.result = outcome_of(Extracted)`. Golfers are
  clamped to `COURSE` inset `GOLFER_RADIUS`. `move_balls` applies
  `ball_step` to every ball; a sunk ball ends the match (Holed for golfer 0,
  `Lost { by }` otherwise). `zone_grace`: for each alive golfer,
  `outside_ticks += 1` if either the golfer or its ball fails
  `inside_zone(zone_at(now), ..)`, else `= 0`; on the tick it **reaches**
  `GRACE_TICKS` the golfer is eliminated (alive false, ball despawned; golfer
  0 → `Eliminated`). `end_match`: golfer 0 alive and no NPC alive →
  `LastStanding`; `now >= MATCH_END_TICK` with no result → `Eliminated`.
  Once `result` is `Some`, every system returns early.
- **hud** — `hud_lines(view: &WorldView) -> [String; 6]` and
  `result_lines(outcome: &Outcome) -> [String; 3]` · `src/hud.rs` · touches
  `WorldView` (`query`, `resource`). The one reader both Draw and the checks
  call (`docs/api/jidousha-api.md`, "A projection both phases read"). Exact
  shapes, all printable ASCII, `{}` are values, every line at most 80
  characters (the band is 64 units wide and a size-1.0 glyph is 7/9 wide):
  1. `ZONE r={r:.1} next r={rn:.1} in {s}s | GRACE {ok|{g:.1}s} | GATE {open {s}s|closed}`
     — `rn`/`s` from the next table keyframe whose radius differs (`next r=0.0
     in Ns` once the last shrink is the next); `GRACE ok` when `outside_ticks
     == 0`, else `(GRACE_TICKS - outside_ticks) * dt`; the gate is open while
     `zone_excludes_at(GATE.center())` is in the future, and the seconds are
     to that tick.
  2. `KIT {Club}, {Ball}, {Head} | cup {d:.1}u | {n} rivals` — `-` for an
     empty slot, item names as in the `ITEMS` table (`Lead Ball`, `Long Club`
     with the space), `d` from my ball to `CUP`, `n` the alive NPCs.
  3. `EXTRACT keeps {names joined by +|nothing} = {p} | HOLE OUT = {p2} | LAST ONE = {p2}`
     — `p` from `outcome_of(Extracted)`, `p2` from `outcome_of(Holed)`.
  4. Aiming: `AIM {deg:.0}deg {PUTT|CHIP|DRIVE} rests {d:.1}u from cup, {inside|OUTSIDE} zone at arrival (r={ra:.1} in {s}s)`;
     when the roll sinks, the `rests ... cup` clause becomes `HOLES OUT`; not
     at the ball: `walk to your ball ({d:.1}u)`; stunned: `DISABLED {s:.1}s`;
     cooling down: `swing cooldown {s:.1}s`; after the match: empty.
  5. `C: club {NAME} - disables {s:.1}s, drops {Item|nothing|nothing (helmet)}`
     · `C: strike {NAME}'s ball - knocks it {STRIKE_SPEED:.0}u/s along your aim`
     · `C: strike {NAME}'s ball - no effect (lead ball)` · empty when
     `contact_target` is `None`. `drops nothing` when the target holds
     nothing; `nothing (helmet)` when it does but the helmet keeps it.
  6. `F: take {Item} - {describe(item)} | replaces {Item|nothing}` within
     `READ_RANGE` of the nearest pickup, else empty. `describe(Driver)` is
     exactly `shot speed x1.4`; the other three are open calls below, at most
     40 characters each.
- **draw** — `draw_course`, `draw_play`, `draw_hud` in the `Draw` phase ·
  `src/draw.rs` · touches `DrawCtx`, `Submit` (`rect`, `circle`, `line`,
  `text`), `TextStyle` (`width_of`), `Depth`, `Color`. Bands in a `mod
  layers`: `FIELD -1` (course rect in `COURSE_GREEN`), `MARK 0` (the current
  zone ring in `ZONE_RING` and the zone-at-arrival ring in `NEXT_RING`, each
  64 `line` segments of thickness 0.15 through `clip_to_course`; the aim line
  and landing disc, radius 0.4, `SAFE_DOT` or `DANGER` by `inside_zone(
  zone_at(arrival), rest)`; the gate outline as four lines plus a `GATE`
  label; the reach ring, radius `CLUB_REACH * reach_scale`, 32 segments
  through `clip_to_course`, drawn only while `contact_target` is `Some`, and a
  ring of radius 1.1 around its target), `PLAY 1` (balls radius 0.35 in the
  owner's colour; golfers radius 0.8, a stunned one at half alpha, one
  outside the zone with a `DANGER` ring of radius 1.0; pickups as 1x1 rects
  in `PICKUP` with their letter D/R/L/H in size 0.8 text), `UI 2` (HUD lines,
  legend, result banner over a 40x8 rect in `BANNER_BACK`). HUD line n
  (0-based) at `(-31.0, -17.5 + n * 1.0)`, size 1.0, `HUD_TEXT`; legend at
  `y = 16.6`, size 0.8, centred by `width_of`; banner lines size 1.6 at
  `y = -3.2, -1.2, 0.8`, each centred by its own width. Nothing is drawn at
  a radius that can leave the camera: rings are clipped, golfers are clamped,
  balls reflect.
- **capture** — `src/capture.rs` is `crates/jidousha/examples/prototype_kit/capture.rs`
  minus the asset lines (`docs/api/jidousha-capture.md`, "A game of pure
  shapes"): 480x270, `WgpuBackend::offscreen`, `NoAdapter` → skipped and
  said so, every other error a failure. Prints exactly
  `capture: frame written to target/verify/brolf_r3v2.png`.
- **players** — the controllers of the verify runs · `src/players.rs` ·
  touches `docs/api/jidousha-testing.md`: `SnapshotBuilder`, `InputEvent`,
  `InputSnapshot`, `Input`. `drive(intent: &Intent, keys: &mut KeyState,
  builder: &mut SnapshotBuilder)` turns an `Intent` into key edges (send
  events, not states; `turn` holds an arrow until the aim is within one
  tick's `AIM_RATE` of the wanted angle, then releases). Three players for
  the playability table (`docs/api/jidousha-controllers.md`): **good** =
  `npc_decide(.., Goal::Compete)` through the keyboard; **chaser** = walk to
  my ball and DRIVE at the cup every time, never consulting the zone (the
  first-try player); **idle** = `InputSnapshot::new()` every tick. Print the
  three controller numbers for the good player: `met N of M shot
  opportunities` (ticks a shot was legal and taken, over ticks it was legal),
  `planned rests X from the cup` (mean), `shots rested Y from where planned`
  (must print `0.00`: the prediction is the sim).
- **verify / checks** — `src/verify.rs` runs the gates below and prints the
  verdict; `src/checks.rs` holds the assertion functions · touches
  `FrameRecorder` (`new`, `draw`, `font_texture`), `FrameRecord` (`quads`,
  `covering`, `plan`), `DrawnQuad` (`bounds`, `texture`, `tint`),
  `find_bounds`, `HeadlessSim` (`tick`, `world`, `world_mut`,
  `schedule_debug`), `headless`, `GameConfig`. Failures are collected, not
  exited on; every failure prints the numbers it judged; the verdict line
  starts `verified ` and the indented summary carries one line per gate
  named `decision 1 zone`, `decision 2 contact`, `decision 3 equipment`,
  `decision 4 extraction`, then the player table, the clearance and the
  capture line. That summary is what `tools/verify` lifts into
  `target/verify/brolf_r3v2.json`, and is how "a check for each decision
  row" is read off the report.
- **mutants** — `mutants/r1.txt`, at least 14 one-line faults in
  `tools/mutate`'s four-line shape, committed with the game; the round runs
  as `python3 tools/mutate brolf-r3v2 mutants/r1.txt` (the game folder's
  name). Minimum set: a `ZONE_TABLE` radius; `GRACE_TICKS`; the `inside_zone`
  comparison; `CLUB_STUN`; the helmet `stun_scale`; `drops_when_clubbed`;
  `strike_immune`; the driver `shot_scale`; `BAG_LIMIT`; `WIN_BONUS`; an item
  value; `arrival_tick` using `now`; the elimination comparison; the HUD
  omitting the grace clause.
- **screens** — `screens/match.png`, the run's capture copied in and
  committed (keifu and ninjo do the same); say in the PR what it shows.

Assets: none. Every shape is `rect`, `circle`, `line` or `text` in the
built-in face; no `Assets`, no `asset_source`, nothing under `assets/`,
nothing for `tools/check-assets` to check.

### Constants (the numbers the design fixes)

Layout: `WINDOW 1280x720`, `HALF_H 18.0`, `HALF_W = HALF_H * WINDOW.aspect()`
(32.0), camera `height 36.0`, `center ZERO`. `COURSE = Rect { min (-30,-11),
max (30,16) }` — the HUD band is `y` -17.5 to -11.5 above it, the legend
below it. `CUP = (0, 2)`, `CUP_RADIUS 0.6`, `SINK_SPEED 8.0`. `GATE =
Rect::from_center_size((26,-8), (4,4))`. Colours: `CLEAR (0.05,0.07,0.06)`,
`COURSE_GREEN (0.16,0.38,0.18)`, `ZONE_RING (0.95,0.95,0.70)`, `NEXT_RING
(0.95,0.95,0.70,0.45)` (alpha reads brighter than it looks; tune from the
capture), `SAFE_DOT (0.3,1.0,0.4)`, `DANGER (0.95,0.25,0.20)`, `PICKUP
(0.85,0.75,0.30)`, `HUD_TEXT WHITE`, `BANNER_BACK (0.02,0.02,0.03)`; golfers
YOU `(0.35,0.80,1.0)`, ROOK `(1.0,0.50,0.20)`, PIKE `(0.90,0.90,0.30)`, WREN
`(0.80,0.40,0.90)`.

Golfers, index/name/temperament/start: 0 YOU player `(-22, 11)`; 1 ROOK Hunter
`(22, 11)`; 2 PIKE Banker `(18, -7)`; 3 WREN Putter `(-22, -7)`. Ball starts at
`start + (1.0, 0)`. Initial aim: toward `CUP`; initial power CHIP.
`GOLFER_RADIUS 0.8`, `BALL_RADIUS 0.35`, `WALK_SPEED 9.0`, `SHOT_REACH 1.5`,
`AIM_RATE` 120 degrees per second (a `const Radians` via `from_degrees`),
`CLUB_REACH 2.5`, `READ_RANGE 4.0`, `TAKE_RANGE 1.5`.

Ball: `Power::{Putt 10.0, Chip 18.0, Drive 26.0}` u/s, `BALL_DECEL 16.0`
u/s², `REST_SPEED 0.05`. With `ball_step` as specified and `fixed_dt` 1/60, a
CHIP rolls `9.98` units, a PUTT `3.04`, a DRIVE `20.91`, and a CHIP with the
Driver `19.63` — these are the literals the checks carry. Per tick a ball
moves at most `26 * 1.4 / 60 = 0.61` units, under `2 * CUP_RADIUS`, so a
sinking ball is never stepped over; assert that inequality in the contracts.

Contact: `CLUB_STUN 180` ticks, `STRIKE_SPEED 20.0`, `SWING_COOLDOWN 60`.

Items (`ITEMS` table; name, letter, category, value, effect): Driver `D` Club 3
`shot_scale 1.4`; Long Club `R` Club 3 `reach_scale 1.6`; Lead Ball `L` Ball 2
`strike_immune, shot_scale 0.7`; Helmet `H` Head 2 `stun_scale 0.5,
drops_when_clubbed false`. Defaults: scales 1.0, not immune, drops true.
Pickups: six, items in order `[Driver, Helmet, LeadBall, LongClub, Driver,
Helmet]`; each position is uniform in `COURSE` inset 3 from the seeded `Rng`
(`next_f32` twice), redrawn while within 5 units of any start, the cup, the
gate centre or an earlier pickup, up to 64 draws then accepted as is.
`GameConfig.seed = 7` for the window and every verify run.

Zone: centre `CUP`; `ZONE_TABLE: [(tick, radius)] = [(0,34),(1800,34),
(3600,22),(5400,22),(7200,12),(8400,12),(9600,4),(10800,4),(11400,0)]`, radius
0 after the last row. `GRACE_TICKS 300`. By the table, a golfer standing at
`(-22, 11)` (23.77 from the cup) goes outside on tick 3335 and the gate's
centre (27.86 from the cup) is excluded from tick 2722; the checks scan for
these rather than trusting the arithmetic here.

Extraction and scoring: `EXTRACT_HOLD 120`, `BAG_LIMIT 2`, `WIN_BONUS 5`,
`MATCH_END_TICK 12000`. A kit of Driver, Lead Ball and Helmet extracts as
`Driver+Lead Ball = 5` (value 3 then the tie at 2 broken toward Ball) and
holes out as `12`.

## Gates to add

Every run is `headless(GameConfig { seed: 7, ..GameConfig::default() }, setup)`
with a `FrameRecorder::new(WINDOW)`, built only for the runs that read frames.
"Row n" is the spec's decision-surface table. A staged change goes through
`world_mut()` and sets every piece of state the frame depends on.

- **decision 1 zone** — input: the idle player (no keys) from tick 1 to tick
  4000, drawing a frame on ticks 2600, `t_out + 100`, `t_out + 298` and
  `t_out + 299`, where `t_out` is the first tick the check itself finds
  `!inside_zone(zone_at(t), (-22, 11))` by scanning · asserts, on the tick-2600
  frame: HUD line 1 starts `ZONE r=28.7` and contains `GRACE ok`; HUD line 4
  matches `AIM .* zone at arrival \(r=(\d+\.\d) in` with that `r` equal to
  `zone_at(arrival_tick(2600, golfer, roll_out(initial aim, CHIP))).radius`
  formatted `.1`, recomputed by the check; the frame draws exactly
  `hud_lines()[0].chars().count()` and `[3].chars().count()` font-textured
  quads whose `bounds().min.y` is within 1e-3 of those rows' `y`; at least
  one `ZONE_RING`-tinted quad and at least one `NEXT_RING`-tinted quad exist,
  every corner of every `ZONE_RING` quad is within 0.3 of `zone_at(2600)
  .radius` from `CUP`, every corner of every `NEXT_RING` quad within 0.3 of
  the arrival radius, and the arrival radius is smaller than the current one.
  Then: golfer 0's `outside_ticks == 1` on `t_out`; HUD line 1 on `t_out +
  100` contains exactly `GRACE 3.3s`; on `t_out + 298` golfer 0 is alive and
  `Match.result` is `None`; on `t_out + 299` — the tick `outside_ticks`
  reaches 300 — `alive == false` and `Match.result` is `Eliminated`, and not
  one tick earlier or later · covers Done-when: "a check for each decision
  row" (row 1).
- **decision 2 contact** — target WREN, the Putter, because it never clubs
  back. Input: tick 1; move WREN to `(-20, 11)` and WREN's ball to `(-10,
  11)`; tick 2 with no keys, draw · asserts HUD line 5 is exactly `C: club
  WREN - disables 3.0s, drops nothing` and its glyph count is drawn on that
  row; tick 3 with C pressed (one `KeyPressed`/`KeyReleased` pair) · asserts
  WREN's `stun_left == 180` after tick 3 (WREN had walked at most 0.15 toward
  its ball and was still in reach, contacts resolving before walks), `> 0`
  on every tick through 182, `== 0` on tick 183, `alive` and
  `world().is_alive(entity)` true on every tick 3 through 400, and golfer
  0's `cooldown == 60` after tick 3. Then give WREN a Driver, move it back to
  `(-20, 11)`, wait out the cooldown, club again · asserts a `Pickup {
  Driver }` lies within 1e-3 of WREN's position and WREN's Club slot is empty;
  give WREN a Helmet, repeat · asserts `stun_left == 90` and no new pickup.
  Strike half: move WREN to `(-10, 11)` and its ball to `(-20, 11)`, set
  golfer 0's `aim` to `Radians::ZERO`, tick, draw · asserts line 5 is `C:
  strike WREN's ball - knocks it 20u/s along your aim`; press C · asserts the
  ball's `vel` after that tick equals `ball_step((-20,11), (20,0), dt).vel`
  within 1e-4; give WREN a Lead Ball, repeat with the ball at rest · asserts
  `vel` stays zero and line 5 ends `no effect (lead ball)` · covers
  Done-when: row 2 ("disabled for exactly the stated time and never
  permanently removed").
- **decision 3 equipment** — input: tick 1; find the first `Pickup { Driver }`
  sorted by `(y, x)`, move golfer 0 to `pickup - (1.0, 0)` and its ball to
  `pickup`'s position shifted `(0, -1.0)` (inside `SHOT_REACH`, not on the
  pickup), tick with no keys, draw · asserts HUD line 6 is exactly `F: take
  Driver - shot speed x1.4 | replaces nothing` and is drawn (glyph count at
  its row); press F · asserts the Club slot is Driver, that pickup is gone,
  `effects(&kit).shot_scale == 1.4`. Then move golfer 0 to `(-29, -8)`, its
  ball to `(-28, -8)`, `aim = Radians::ZERO`, power CHIP, press Space, tick
  until the ball's `vel` is zero · asserts the resting position equals
  `roll_out((-28, -8), (25.2, 0), dt).rest` within 1e-3 **and** `rest.x -
  (-28.0)` is within 0.02 of the shipped literal `19.63` (not derived from
  any constant) — the stated `x1.4` on the speed is what the sim did. Then
  spawn a Long Club pickup at the golfer's feet, tick, read line 6 · asserts
  it ends `| replaces Driver`; press F · asserts the Club slot is Long Club
  and a `Pickup { Driver }` lies within 1e-3 of the golfer's position ·
  covers Done-when: row 3 ("its stated effect is what the sim then does").
- **decision 4 extraction** — input: tick 1; give golfer 0 `[Driver, Helmet,
  LeadBall]`; drive golfer 0 with `npc_decide(.., Goal::Extract)` through the
  keyboard, keeping the last frame drawn while `result` was `None`, until
  `result` is `Some` or tick 3000 · asserts: HUD line 3 on that last live
  frame starts `EXTRACT keeps Driver+Lead Ball = 5`; the result is
  `Extracted` and `result_lines` is exactly `EXTRACTED`, `kept: Driver, Lead
  Ball`, `points: 5`; `kept` equals `kept_on_extract(kit)`; the items and the
  points the banner shows are the items and points line 3 promised (compare
  the parsed names and the number, not the whole strings); `extract_ticks`
  reached 120 on the ending tick; the banner's three rows are drawn, each
  row's font quads' `find_bounds().center().x` within 0.05 of `0.0`, inside
  the camera · covers Done-when: row 4 ("keeps exactly what the status
  promised") and "reaches its result screen".
- **the match** — input: the good player (`Goal::Compete` via keys) against
  the three NPCs at their shipped temperaments, seed 7, until `result` is
  `Some`, cap `MATCH_END_TICK + 1` · asserts a result is reached before the
  cap; prints which and the tick; runs the **chaser** and the **idle** player
  the same way · asserts the idle player's result is `Eliminated` (the game
  can be lost); prints `players: good <kind> t=<tick> | chaser <kind>
  t=<tick> | idle <kind> t=<tick>` and the three controller numbers · covers
  Done-when: "one full match loop against NPC opponents plays end to end, and
  a scripted --verify run reaches its result screen".
- **layout** — input: the last *live* frame of the match run · asserts
  nothing drawn outside `Camera::visible_bounds()` (closed `contains_rect`)
  and prints `closest quad to the edge: {x:.2} world units`; every HUD font
  quad has `bounds().max.y <= COURSE.min.y` (the requirement, not the
  constant); the legend's font quads lie below `COURSE.max.y`;
  `frame.plan.clear_color == CLEAR` and its brightest channel `< 0.25`; every
  string from `hud_lines` on that frame, `result_lines` for all five kinds,
  the legend and all four `describe` texts is printable ASCII (`' '..='~'`),
  and every HUD line is at most 80 characters; `sim.schedule_debug()` finds
  all eight Update system names (each `find` is `Some`) and their indices
  ascend in the registered order · covers Done-when: `tools/verify` `pass`.
- **staged screens** — input: tick 1, then for each `EndKind` set
  `Match.result = Some(outcome_of(kind, &kit_of_all_three))`, draw · asserts
  each banner's three rows are on screen, centred within 0.05 of `x = 0`,
  printable, and the five first rows are pairwise different strings (five
  correct screens are not five different screens unless a check says so) ·
  covers Done-when: `pass`.
- **contracts** — no sim: `ball_step` from `(29.5, 2)` with `vel (30, 0)` for
  one tick rests inside `COURSE` inset `BALL_RADIUS` with `vel.x < 0`;
  `roll_out` from `CUP + (0.3, 0)` at speed 2 toward the cup reports `sunk`,
  the same at speed 12 does not; `roll_out` of a CHIP from `(-28, -8)` along
  `+x` rests `9.98 ± 0.02` away and of a PUTT `3.04 ± 0.02`; `zone_at`'s
  radius is non-increasing over `0..=11400` sampled every tick;
  `zone_excludes_at(GATE.center())` equals the first `t` a scan finds
  `!inside_zone(zone_at(t), GATE.center())`; `Drive * 1.4 * fixed_dt < 2 *
  CUP_RADIUS`; `kept_on_extract` of `[Driver, LeadBall, Helmet]` is `[Driver,
  LeadBall]`; `clip_to_course` of a segment wholly outside is `None`, of one
  wholly inside is itself, and of one crossing an edge has both ends inside
  · covers Done-when: `pass`.
- **capture** — the last live frame of the match run rendered at 480x270;
  prints the `capture:` line, or `capture: skipped, no GPU adapter` · covers
  Done-when: the picture (`make-game` §A.7).
- **mutation round** — not a `--verify` gate; run `tools/mutate` once after
  the checks exist, record `N of N noticed` in the PR, fix any escape by
  tightening the check, never by removing the fault.

Every one of the spec's Done-when lines maps to a gate above; the remaining
two — `git diff --stat` naming only the three fenced paths, and `tools/
build-web brolf_r3v2 && tools/serve-web brolf_r3v2 --check` — are commands
in WORKER.md §2 and §4, not checks the game runs.

## Non-goals

Cut so the implement stage fits the window (WORKER.md §2's gates must be green
by Thursday 04:30 America/Los_Angeles):

- No multiplayer, no title screen, no restart, no pause: the match starts on
  tick 1 and the window is closed by the player after the banner.
- No art, no sound, no `Assets`, no `jidousha::ui`: six `ctx.text` lines are
  the whole chrome.
- No pointer or touch input: keyboard only.
- No filled zone disc: the zone is a clipped ring, because a disc of radius
  34 cannot be drawn inside a camera 36 units tall and `ctx.circle` does not
  clip. Inside and outside are shown by the ring, the landing disc's colour,
  the red ring on an outside golfer and HUD line 1.
- No golfer-golfer or ball-ball collision, no obstacles, no terrain, no wind,
  no spin, no ball arc: balls roll on a flat plane and reflect off the course
  edge.
- No moving zone centre: the zone always closes on the cup.
- No interpolation (`Time::alpha`): the prototype steps at tick rate, which
  `docs/api/jidousha-api.md` says is a reasonable prototype choice.
- No NPC aiming through the turn rate: NPCs set `aim` directly.
- No second extraction gate; after the zone passes the gate, extraction is
  possible only inside grace.
- No engine change is needed and no FINDINGS entry is filed by the design
  stage: the five documents answered every question this design asked.

If the implement stage falls behind, cut in this order and list each cut in
the PR's Deviations: (1) the Banker temperament (NPCs then never extract);
(2) the staged-screens gate down to Extracted and Eliminated only; (3) the
Helmet and Lead Ball halves of the contact gate (keep the plain club and the
plain strike); (4) the chaser player (keep good and idle; the table is then
two lines and the PR must say the game's playability was not measured).

## Decisions already made

- **Real-time, simultaneous, fixed-timestep sim; no turns.** A battle royale
  is continuous, and continuous is also the shape `headless` and a keyed
  controller drive most simply.
- **Four golfers on one course, zone centred on the cup.** The funnel and
  the hole are the same point, so late play is contested by construction.
- **Three win conditions, scored as above.** Extraction trades the bonus for
  certainty and keeps two of three items; HUD line 3 shows the three numbers
  at every tick, which is what makes "extract now or fight on" a decision the
  player can make.
- **One contact key, C, that does what the cue says.** One function picks the
  target; the cue and the key read it. Two keys would be a second way.
- **Aggression pays in loot and position.** A club drops the target's best
  item where they stand; a strike moves their ball out of the zone or away
  from the cup. Neither kills: elimination is only ever the zone's.
- **The aim line is the sim.** `roll_out` loops `ball_step`; the landing disc
  and the zone-at-arrival ring are computed from `arrival_tick`, not from the
  zone now.
- **Equipment effects are speed and time multipliers, described as such.**
  `x1.4` is on the shot speed; the roll distance follows from the sim and the
  checks carry it as a literal.
- **Grace resets, it does not decay.** Simpler to read and to assert; the
  countdown is `(GRACE_TICKS - outside_ticks) * dt`.
- **Elimination on the tick `outside_ticks` reaches `GRACE_TICKS`**, in
  `zone_grace`, which runs after `move_balls` so the ball's position that
  tick counts.
- **Golfer 0 is the player and the only perspective.** NPC outcomes affect
  the player only through `Lost` and `LastStanding`.
- **Seed 7 everywhere.** The window and every verify run use it, so what the
  checks saw is what a person plays.
- **System order is fixed and asserted.** Contacts before walks; balls move
  before the zone judges; the match ends last.
- **Every string the game draws comes from `hud_lines`, `result_lines`,
  `describe` or the legend `const`**, so the checks can ask for the text and
  the ASCII check covers all of it.
- **Decision surfaces are the spec's four rows, elaborated here and not added
  to**: row 1 → HUD lines 1 and 4 plus the two rings and the landing disc;
  row 2 → HUD line 5 plus the reach and target rings; row 3 → HUD line 6; row
  4 → HUD lines 2 and 3 and the banner. Each ships with the gate of the same
  number.

## Open calls delegated to the implementer

- The exact phrasing of `describe(item)` for Long Club, Lead Ball and Helmet,
  provided each embeds the number it is about (`x1.6`, `x0.7`, `x0.5`), is
  printable ASCII and at most 40 characters, so HUD line 6 stays under 80.
- Ring thickness, the landing-disc radius and every alpha — pick by eye from
  the capture; faint overlays read brighter than their alpha says
  (`docs/api/jidousha-api.md`, Color). Segment counts (64 and 32) may change
  if the capture says so, as long as the ring-radius assertion's 0.3
  tolerance still holds.
- Whether `Intents` is one resource of four or a component per golfer, as
  long as the player and the NPCs go through the same apply systems.
- How the chaser player is written, as long as it never consults the zone.
- The pickup rejection rule's exact spelling beyond "five units from
  everything already placed, 64 draws, then accept".
- Splitting `systems.rs` or `verify.rs` further if either nears 500 lines.
- Which extra faults, beyond the fourteen named, go in `mutants/r1.txt`.
- The legend's wording, within one line of size 0.8 that fits the 64-unit
  width (`width_of` decides, not a character count).
