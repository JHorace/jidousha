# DESIGN — brolf-r2

task: brolf-r2
variant: V2
model-as-configured: claude-fable-5-1 (the session context's configured model; fallbacks claude-opus-5-5[1m], claude-opus-5[1m] are listed and the serving model is not verified here)
date: 2026-10-07 18:25 PDT

Read `tools/yakin/tasks/brolf.md` through `tools/yakin/tasks/brolf-r2.md`'s
overrides first: this document elaborates that spec's brief and its four
decision rows, and adds nothing to them. The game is `games/brolf-r2/`, the
crate (package name) is `brolf_r2`, and the tools are run as
`python3 tools/verify brolf_r2`, `python3 tools/build-web brolf_r2`,
`python3 tools/serve-web brolf_r2 --check`. **Comparison hygiene (spec):
never open the night-one branch `claude/yakin-brolf`, PR #128, or any
`games/brolf/`.** Build from this document and `docs/api/` alone.

The engine surface is `docs/api/` (all five files) and
`crates/jidousha/examples/` — `prototype_kit/` for the crate shape, the
`Checks` accumulator, the verify loop and the capture path; `slalom/` for the
controller shape; `ui_kit.rs` for the UI kit (`jidousha::ui`) worked end to
end. Never `crates/*/src/`.

## What the game is

Brolf is top-down 2D golf played as a battle royale: four golfers (you and
three NPCs) each play one ball on a walled course while a circular safe zone
shrinks in phases, and a golfer whose body **or** ball stays outside it for
six seconds is eliminated. Golfers walk, aim, charge and strike; they can
also club a golfer in reach (a three-second stun that drops an item), strike
a rival's ball to send it where they aim, and pick up equipment that changes
how they play. Points come from holing cups and sledging rivals' balls, but
nothing is kept unless it is banked — by extracting at the pad once it opens,
or by being the last golfer standing — so the match is a race between
greed and the zone.

## The player-facing loop

Keyboard only. The window is 1280x720; the camera never moves or zooms.

1. **Start.** The course fills the left of the screen: the fence, three cups,
   the extraction pad (closed, grey), six equipment pickups, four golfers at
   their seats with their balls at their feet, the current zone ring (white)
   and the next zone ring (blue, smaller, inside it). The status panel on the
   right reads `BROLF 0:00 zone 1/4`, `holds 20.0s`, `you: 0 pts bank now 0`,
   `held: -, -`, `zone: IN`, `pad: opens 0:32`, the others' rows, and the
   control hints.
2. **Walk** with WASD to your ball (white). Within reach of it, the aim line
   appears: from the ball along your aim, ending at a landing disc that is
   green if that landing will be inside the zone when the ball stops and you
   have walked to it, red if not. ArrowLeft/ArrowRight rotate the aim; the
   panel shows `aim: 123 deg charge 0%` and `reach 20.2 lands IN`.
3. **Hold Space** to charge (the line shortens to the charge's landing, the
   panel's `charge 45%` and `reach` follow), **release to strike**. The ball
   rolls, decelerates and stops exactly on the disc. Walk after it.
4. **Hole a cup**: a ball that stops inside a cup scores 2 points, drops out
   beside the cup, and the cup moves to a seeded spot inside the next zone.
5. **Zone.** The panel says when the zone holds and when it shrinks; during a
   shrink the white ring closes on the blue one. If you or your ball is
   outside, the panel reads `zone: OUT 4.2s` counting down, your golfer wears
   a red ring, and at zero you are eliminated (your golfer and ball vanish,
   your row reads `eliminated`).
6. **Contact.** When a rival golfer is within reach it wears an orange ring
   and the panel reads `F: club N2 (stun 3.0s)` plus `  drops Spikes` if it
   holds anything. F stuns them (they cannot move, aim, strike, club, take or
   extract, and their grace keeps running) and drops their newest item at
   their feet; you then cannot club again for 1.5 s. When a rival's ball is
   the nearest ball in reach, the aim line is drawn from *it* and the panel
   reads `Space: strike N2's ball +2`: releasing Space sends their ball where
   you aim and pays you 2 points.
7. **Equipment.** Near a pickup the panel reads `E: take Driver, worth 3` /
   `  strike 1.5x faster` / `  swaps out Spikes` (the last row only when both
   slots are full); E takes it. Four kinds: Driver (strike faster, so farther),
   Spikes (walk faster), Heavy (rivals' strikes on your ball go half speed),
   Helmet (a club stuns you 1.0 s, not 3.0 s). Two slots.
8. **Extract.** From 0:32 the pad turns gold and the panel reads `pad: OPEN`.
   With your golfer and your ball both inside the pad, the panel reads
   `X: extract, keeps 8` — your points plus the worth of what you hold. X
   banks exactly that and takes you out of the match; the NPCs play on.
9. **End.** When nobody is left playing, the result overlay lists every
   golfer: `YOU extracted 8`, `N1 survived 13`, `N2 eliminated 0`, with the
   verdict `YOU win with 8` or `the zone took everyone`. The match is over;
   close the window (no restart — Non-goals).

NPCs do the same things through the same rules: they walk to their balls,
golf toward cups that lie in the next zone, rescue a ball the zone is about to
leave behind, loot nearby pickups, club a rival in reach who holds something
or is already outside the zone, and extract once their bank reaches a
per-NPC threshold or the third zone phase begins.

## Systems

Build order. Every path is under `games/brolf-r2/`. "api" names the
`docs/api/` file and the items as that file names them. The crate root
carries `#![allow(missing_docs)]`; the manifest is keifu's shape (`[package]
name = "brolf_r2"`, the four `.workspace = true` lines, `[lints] workspace =
true`, and the one dependency `jidousha = { path = "../../crates/jidousha" }`).
No file over ~500 lines: split `verify.rs` into `verify.rs` + `gates.rs` if
it grows past that.

- **layout and lifecycle** — the window, the camera, the bands, `config()`,
  `register()`, `main` with `--verify` · `src/main.rs` · touches
  `jidousha-api.md`: `GameConfig` (`seed: 7`, `window_size:
  PhysicalSize::new(1280, 720)`, `title: "brolf"`), `run`, `headless`, `App`,
  `Startup`/`Update`/`Draw`, `Camera` (`height: VIEW_HEIGHT`, `clear_color:
  TURF`, centre `Vec2::ZERO`, `..Camera::default()`), `PhysicalSize::aspect`.
  Constants (world units; Y down):
  - `WINDOW = PhysicalSize::new(1280, 720)`, `VIEW_HEIGHT = 18.0`,
    `HALF_H = 9.0`, `HALF_W = HALF_H * WINDOW.aspect()` (= 16.0). The view is
    `[-16, 16] x [-9, 9]`.
  - `TURF = Color::rgb(0.10, 0.16, 0.11)` (clear colour; brightest channel
    0.16 < 0.25, the requirement the verify states).
  - `COURSE: Rect { min: (-15.5, -7.5), max: (6.5, 8.5) }` — built where used
    (`Rect` is not `const`); `COURSE_CENTER = (-4.5, 0.5)`.
  - `mod layers { FIELD = -2, PLAY = 0, CHROME = 2, OVERLAY = 4 }`.
  - `register`: Startup `set_the_course`; Update in this order, which the
    schedule gate holds: `read_player_intent`, `decide_npcs`, `walk`, `act`,
    `roll_balls`, `settle_balls`, `judge_zone`, `end_match`; Draw in this
    order: `draw_play`, `draw_course`, `draw_chrome` (play is submitted before
    the course so only the FIELD band can put the course behind it — the
    band gate needs that disagreement).
  - `main`: `--verify` → `verify::run()`; else `run(config(), register)`,
    printing the error's `Display` and returning `ExitCode::FAILURE`.
- **rules** — every decision as a free function, no `World` anywhere in the
  file · `src/rules.rs` · touches `jidousha-api.md`: `Vec2`, `Rect`,
  `Radians`, `sin_cos`, `atan2`, `Seconds`, `Rng`. This file is what the
  preview, the NPCs, the sim and the gates all call.
  - `struct Circle { center: Vec2, radius: f32 }`, `contains(p) = (p -
    center).length_squared() <= radius * radius` (closed). A golfer or ball is
    "in" the zone iff its **centre** point is contained.
  - Zone schedule: `ZONE_RADII: [f32; 5] = [7.8, 5.0, 3.2, 1.8, 0.0]`,
    `HOLD_TICKS: [u64; 4] = [1200, 900, 720, 600]`, `SHRINK_TICKS: [u64; 4] =
    [720, 600, 480, 360]`. Phase k holds from its start for `HOLD_TICKS[k]`
    ticks then shrinks over `SHRINK_TICKS[k]` ticks into zone k+1. Phase 0
    starts on tick 1, so the phase starts are ticks 1, 1921, 3421, 4621 and
    the zone is closed (radius 0) from tick 5581. `PAD_OPENS_TICK = 1921`
    (phase 1's first tick), `EXTRACT_PHASE = 2` (NPCs extract from tick 3421).
  - `struct Course { zones: [Circle; 5], cups: [Vec2; 3], pad: Circle,
    seats: [Vec2; 4] }` (pickups are entities, below). `zone_at(&Course, tick)
    -> Circle`: the phase's zone during its hold; during its shrink the
    centre and radius lerped by `(tick - shrink_start) as f32 /
    SHRINK_TICKS[k] as f32` (so the last shrink tick is exactly zone k+1);
    `zones[4]` after tick 5580. `next_zone(&Course, tick) -> Option<Circle>`:
    `zones[k + 1]` while in phase k, `None` once closed. `phase_of(tick) ->
    (k, Phase::Hold(ticks_left) | Phase::Shrink(ticks_left) | Phase::Closed)`.
    `phase_line(tick) -> String` is the panel's row 1 (below). These are
    the **one zone-membership-at-time function** of decision row 1: the
    rings, the landing disc's colour, the grace countdown and elimination all
    call `zone_at`.
  - Seeding (`seed_course(&mut Rng) -> Course`, called once in Startup, draws
    in exactly this order so a seed means one course): zone centres for k =
    0..3 (`zones[0].center = COURSE_CENTER`; for each k: `theta =
    rng.next_f32() * TAU`, `rho = rng.next_f32() * (ZONE_RADII[k] -
    ZONE_RADII[k + 1])`, `zones[k + 1].center = zones[k].center + polar(rho,
    theta)` so each zone lies inside the one before); then the three cups,
    each uniform in zone 0 (`theta`, `rho = rng.next_f32().sqrt() * (7.8 -
    1.0)`), redrawn up to 8 times while within 2.0 of a seat or another cup;
    then the pad, uniform in zone 1 with `rho = sqrt(u) * (5.0 - PAD_RADIUS -
    0.2)`, redrawn up to 8 times while within 1.5 of a cup; then six pickups
    uniform in `COURSE` inset by 1.0, redrawn up to 8 times while within 1.5
    of a seat, cup or the pad, kinds in order Driver, Spikes, Heavy, Helmet,
    Driver, Spikes. Seats are fixed: `COURSE_CENTER + polar(5.0, 45 deg *
    (2i + 1))` for i = 0..3 (seat 0 is the human, at 45 deg: (-0.96, 4.04));
    each ball starts 0.8 units from its seat toward `COURSE_CENTER`. `polar`
    goes through `sin_cos`.
  - Rolling. `MAX_SHOT_SPEED = 18.0` units/s, `CHARGE_TICKS = 60`, `FRICTION =
    8.0` units/s^2, `BALL_RADIUS = 0.16`. `shot_speed(charge_ticks) =
    charge_ticks.min(60) as f32 / 60.0 * MAX_SHOT_SPEED`. The roll is **not
    integrated**: `decel = FRICTION * fixed_dt`; `roll_ticks(speed) =
    (speed / decel).floor() as u64` (= N); `rolled(speed, ticks) = fixed_dt *
    (n * speed - decel * n * (n + 1) / 2)` with `n = ticks.min(N)`;
    `roll_distance(speed) = rolled(speed, N)`. `landing_of(from, dir: Vec2
    (unit), speed, course: Rect) -> Landing { at: Vec2, ticks: u64, fenced:
    bool }`: the full roll if `from + dir * roll_distance` lies inside
    `course` inset by `BALL_RADIUS`; otherwise the first point along the
    segment on that inset rect's boundary (axis slab test), `fenced: true`,
    `ticks` = the first k with `rolled(speed, k) >= that distance`. **The
    ball's position on every tick of a roll is `from + dir *
    rolled(speed, tick - started).min(fence_distance)`**, so the ball stops
    bit-for-bit where `landing_of` said. `charge_for(distance, scale) -> u32`:
    the k in 1..=60 minimising `|roll_distance(shot_speed(k) * scale) -
    distance|` (60 if the distance exceeds the full reach). All of it uses
    `fixed_dt` as handed in (`Time::fixed_dt`), never 1/60.
  - Equipment table `ITEMS: [ItemSpec; 4]` with `enum Item { Driver, Spikes,
    Heavy, Helmet }`, `ItemSpec { item, name: &str, worth: u32, line: &str }`:
    Driver 3 "strike 1.5x faster"; Spikes 3 "walk 1.5x faster"; Heavy 4
    "rivals strike your ball 0.5x"; Helmet 3 "a club stuns 1.0s not 3.0s".
    `effects(held: &[Item]) -> Effects { shot_scale: f32, walk_scale: f32,
    stun_ticks: u64, sledged_scale: f32 }` — `DRIVER_SHOT_SCALE = 1.5`,
    `SPIKES_WALK_SCALE = 1.5`, `STUN_TICKS = 180`, `HELMET_STUN_TICKS = 60`,
    `HEAVY_SLEDGED_SCALE = 0.5`; an item either present or not, never
    stacking. This is decision row 3's **one equipment-effect function**: the
    pickup rows print `ItemSpec.line` and the sim applies `effects`; the gate
    checks the two agree on the shipped literals. `SLOTS = 2`; taking with
    both full drops slot 0 (the older) at the pickup's position and shifts.
  - Reach. `CLUB_REACH = 1.2`, `BALL_REACH = 0.9`, `PICKUP_REACH = 0.9`,
    `GOLFER_RADIUS = 0.35`. `in_reach(me: &GolferView, golfers, balls,
    pickups) -> Reach { club: Option<seat>, ball: Option<seat (the ball's
    owner)>, pickup: Option<index> }`: the nearest rival golfer whose fate is
    `Playing` within `CLUB_REACH`; the nearest **at-rest** ball of an alive
    owner within `BALL_REACH` (own ball wins a tie); the nearest pickup within
    `PICKUP_REACH`. Distances are centre to centre. This is decision row 2's
    **one contact-resolution function**: the orange ring, the `F:`/`Space:`
    rows, and `act` all read it.
  - Points. `HOLE_POINTS = 2`, `SLEDGE_POINTS = 2`, `SURVIVOR_BONUS = 6`,
    `CUP_RADIUS = 0.5`, `PAD_RADIUS = 1.0`, `GRACE_TICKS = 360`,
    `CLUB_COOLDOWN_TICKS = 90`. `bank_now(points, held) = points + sum of
    worth`; `banked(fate, points, held) -> u32`: `Extracted { banked }` and
    `Survived { banked }` carry their number (set from `bank_now` at the
    moment, plus `SURVIVOR_BONUS` for the survivor); `Eliminated` → 0;
    `Playing`/`Stunned` → `bank_now` (what extracting now would keep). This
    is decision row 4's **one outcome function**: the `X: extract, keeps N`
    row, the `bank now` row and the result overlay all call it.
  - Walking: `WALK_SPEED = 4.0` units/s times `walk_scale`; `AIM_RATE =
    Radians::from_degrees(150.0)` per second (2.5 deg/tick).
  - `shot_speed_for(striker: &Effects, owner: &Effects, own: bool, charge)
    = shot_speed(charge) * striker.shot_scale * (if own { 1.0 } else {
    owner.sledged_scale })`.
- **simulation** — the components, resources and Update systems · `src/sim.rs`
  · touches `jidousha-api.md`: `Component`, `Resource`, `World` (`spawn`,
  `insert`, `query`, `query_mut`, `resource`, `resource_mut`,
  `find_resource`, `insert_resource`, `view`), `WorldView`, `Transform`,
  `Input` (`held`, `just_pressed`, `just_released`), `Key` (`W A S D
  ArrowLeft ArrowRight Space F E X`), `Time` (`tick`, `fixed_dt`), `Rng`.
  - `Golfer { seat: usize, fate: Fate, points: u32, held: Vec<Item>, aim:
    Radians, charge: u32, cooldown: u64, out_ticks: u64, intent: Intent }`
    on an entity with a `Transform`; `Ball { owner: usize, roll:
    Option<Roll> }` with a `Transform`; `Roll { from, dir, speed, started,
    landing: Landing }`; `Pickup { item: Item }` with a `Transform`.
    `enum Fate { Playing, Stunned { until: u64 }, Extracted { banked: u32 },
    Survived { banked: u32 }, Eliminated { at: u64 } }`; alive =
    `Playing | Stunned`. `Intent { walk: Vec2, aim: Aim (Keep | Rotate(-1|0|1)
    | Set(Radians)), space_held: bool, club: bool, take: bool, extract: bool
    }` — NPCs `Set` the aim, the human `Rotate`s it.
  - Resources: `Course` (from `rules`), `Match { state: Live | Over { winner:
    Option<usize> } }`, `Seats` (names `YOU N1 N2 N3` and colours).
  - `Snapshot` — the read-only projection every decision reads, built by
    `read_snapshot(&WorldView) -> Snapshot` (api: `world.view()`, `ctx.world`):
    tick, zone now, next zone, pad open, every golfer's `GolferView { seat,
    pos, fate, held, points, out_ticks, cooldown, aim, charge }`, every
    `BallView { owner, pos, at_rest, landing: Option<Vec2> }`, every pickup's
    `(index, pos, item)`, cups. The panel, the NPCs and the gates all read
    the world through it.
  - Startup `set_the_course`: insert `Camera`, seed the course from the world
    `Rng` (`seed_course`), spawn seats, balls, pickups, `Match::Live`.
  - `read_player_intent`: seat 0's `Intent` from `Input` (`find_resource`;
    none → idle intent). WASD → `walk` normalised with `normalize_or_zero`;
    arrows → `Rotate`; Space held → `space_held`; F/E/X `just_pressed` →
    `club`/`take`/`extract`.
  - `decide_npcs`: seats 1..3 get `npc::decide(&snapshot, seat)`.
  - `walk`: alive, not stunned golfers move `walk * WALK_SPEED * walk_scale *
    fixed_dt`, clamped to `COURSE` inset by `GOLFER_RADIUS`; `Rotate` turns
    the aim by `AIM_RATE * fixed_dt`; `Set` sets it; `cooldown` and `charge`
    bookkeeping: `space_held` → `charge = (charge + 1).min(60)`.
  - `act`, in seat order, all intents gathered first so two clubs on one
    tick both land: Space **released** this tick (`charge > 0 && !space_held`)
    with `reach.ball = Some(owner)` → `strike`: the ball gets `Roll { from:
    its pos, dir: polar(1.0, aim), speed: shot_speed_for(..), started: tick,
    landing: landing_of(..) }`, and if `owner != me` → `points +=
    SLEDGE_POINTS`; charge resets to 0 on any release. `club` with
    `reach.club = Some(seat)` and `cooldown == 0` → target `Stunned { until:
    tick + effects(target.held).stun_ticks }`, target's newest item becomes a
    pickup at `target.pos + 0.6 * dir(me -> target)` clamped into `COURSE`,
    `cooldown = CLUB_COOLDOWN_TICKS`. `take` with `reach.pickup = Some(i)` →
    the slot rule above. `extract` with the pad open, my golfer and my
    at-rest ball both inside `course.pad` → `Extracted { banked: bank_now }`.
    Stunned golfers' intents are ignored; `Stunned { until }` returns to
    `Playing` on tick `until`.
  - `roll_balls`: every ball with a `Roll` is placed by the formula in
    `rules`; when `tick - started >= landing.ticks` it is placed at
    `landing.at` and `roll = None` (this tick is its first at-rest tick).
  - `settle_balls`: a ball that came to rest this tick within `CUP_RADIUS` of
    a cup → owner `points += HOLE_POINTS`, ball moved to `cup + 1.0 *
    unit(zone_at(tick).center - cup)` (or `+X` if the cup is the centre), and
    that cup redrawn from `Rng` inside `next_zone` (or the current zone when
    there is none) with `rho = sqrt(u) * (r - CUP_RADIUS).max(0)`, retried up
    to 8 times while within 1.5 of another cup or 1.0 of the pad. Balls are
    walked in seat order.
  - `judge_zone`: for every alive golfer: `out = !zone.contains(golfer) ||
    !zone.contains(ball)` with `zone = zone_at(tick)`; `out_ticks` += 1 or
    resets to 0; `out_ticks >= GRACE_TICKS` → `Eliminated { at: tick }`
    (so a golfer first outside on tick t is eliminated on tick t + 359).
  - `end_match`: if `Match::Live`: alive count 0 → `Over`; alive count 1 and
    the other three all `Extracted | Eliminated` → that one `Survived {
    banked: bank_now + SURVIVOR_BONUS }`, then `Over`. `winner` = the seat
    with the greatest `banked`, lowest seat on ties, `None` if the greatest
    is 0. After `Over` every Update system returns at once (the result is a
    frozen course).
- **NPC policy** — `npc::decide(&Snapshot, seat) -> Intent`, pure · `src/npc.rs`
  · touches nothing new (`rules` and `Snapshot` only). Priority, first that
  applies wins:
  1. not alive, or stunned → idle intent.
  2. club: `reach.club = Some(g)`, `cooldown == 0`, and (`g` holds an item or
     `g.out_ticks > 0`) → `club`.
  3. extract: pad open and (`bank_now >= EXTRACT_AT[seat]` or `phase_of(tick).0
     >= EXTRACT_PHASE`), with `EXTRACT_AT: [u32; 4] = [7, 6, 9, 5]` → goal =
     the pad: both inside and the ball at rest → `extract`; ball not inside →
     shot at `pad.center`; else walk to `pad.center`.
  4. rescue: own ball at rest (or its landing) outside `zone_at(tick + 120)`
     → shot at `next_zone.center` (or the current centre when none).
  5. loot: a free slot and a pickup within 4.0 that is inside `zone_at(tick +
     120)` → walk to it, `take` when in reach.
  6. golf: shot at the cup nearest the ball among cups inside `next_zone`
     (all cups when none is) .
  7. otherwise walk toward `zone_at(tick + 120).center`.
  "Shot at T": if the ball is rolling, walk toward its landing; else if
  `reach.ball != Some(seat)` (own ball not the nearest at-rest ball in
  reach), walk to it; else `aim = Set(atan2(T - ball))`, `want = charge_for(|T - ball|, shot_scale)`, lowered step by step
  while `landing_of` for that charge lands outside `zone_at(tick +
  landing.ticks + walk ticks to the landing)` (if nothing lands inside, aim
  at the zone centre instead), then `space_held = charge < want` (release on
  the tick charge reaches `want`). NPCs set the aim instantly; that asymmetry
  with the human's 2.5 deg/tick rotation is deliberate and stated in the
  file. NPCs never sledge (they only ever release on their own ball) and
  never extract-fight; clubbing is their whole aggression.
- **the screen** — the status panel, the result overlay, the three Draw
  systems · `src/screen.rs` · touches `jidousha-ui.md`: `Panel`, `TextRun`,
  `Cell`, `Floors`, `Mapping`, `Icon`, `IconRun` (type only — no icons are
  drawn), `clipped`, `centered`, `judge_panel`, `judge_frame`,
  `frame_text_floor`, `inside`; `jidousha-api.md`: `DrawCtx`, `Submit`
  (`rect`, `line`, `circle`, `text`), `Depth { layer, z }`, `TextStyle`
  (`measure`, `width_of`, `columns_in`), `Color`, `Camera::visible_bounds`.
  - Design space 960x540 (`DESIGN`), mapped like `ui_kit.rs`'s `UiMap::
    for_camera` — fitted inside the view, which at 32x18 gives
    `scale = 1/30` exactly and origin `view.min`. `FLOORS = Floors {
    min_text: 12.0, chrome: 0..960 x 0..540, world: the course rect in world
    units }`. `enum Art {}` (empty, deriving `Clone, Copy, Debug, PartialEq`,
    `impl Icon for Art { fn size_at(self, _) -> Vec2 { match self {} } }`);
    the sprite closure handed to `Panel::draw` is `|icon, _| match icon.art {}`.
    No `world_text` anywhere: every row is chrome, in design units.
  - `status_panel(&Snapshot, seat: usize) -> Panel<Art>` is the **always-
    visible match status** of decision row 4, and it carries the surfaces of
    rows 1–3 too. Styles: `small(color)` size 12 on `layers::CHROME`; `DIM =
    rgb(0.62, 0.66, 0.60)`, `INK = rgb(0.93, 0.93, 0.88)`, `GOLD = rgb(0.95,
    0.78, 0.30)`, `WARN = rgb(0.95, 0.40, 0.30)`, `OK = rgb(0.45, 0.90,
    0.55)`. The panel column is `PANEL_X = 690`, `PANEL_W = 260` (27 columns
    of size-12 text — every row goes through `Cell { at, width: 260 }.run`
    so a long name clips rather than overruns), row `i` at `y = 52 + 16 *
    i`. Row indices are fixed (an absent row is simply not emitted), so no
    two rows can collide:
    - 0 `BROLF 0:47 zone 1/4` (clock `m:ss` from `tick / 60`, phase k+1 of 4;
      `zone closed` after 5580)
    - 1 `phase_line`: `holds 12.3s` / `shrinks in 4.0s` / `zone closed`
    - 2 `you: 4 pts bank now 7` — `bank now` is `banked(..)` for the player
    - 3 `held: Driver, -`
    - 4 `zone: IN` (OK) / `zone: OUT 3.4s` (WARN; `(GRACE_TICKS - out_ticks)
      as f32 / 60.0` with `{:.1}`), or `extracted 8` / `eliminated` once
      the player's fate says so
    - 5 `pad: opens 0:32` / `pad: OPEN` (GOLD)
    - 6 `alive 3 lead N2 7` (lead = the alive golfer with the greatest bank
      now, lowest seat on ties)
    - 8 `aim: 123 deg charge 45%` — only when `reach.ball` is `Some`
    - 9 `reach 9.1 lands IN` / `reach 9.1 lands OUT` — reach is
      `|landing.at - ball|` for the current charge (full charge when
      `charge == 0`), IN/OUT from `zone_at(tick + landing.ticks +
      walk_ticks(golfer -> landing.at))`
    - 10 `Space: strike your ball` / `Space: strike N2's ball +2`
    - 11 `F: club N1 (stun 3.0s)` (`stun_ticks / 60` of the **target's**
      effects, `{:.1}`) / `club ready in 1.2s` while cooling; 12 `  drops
      Spikes` when the target holds something
    - 13 `E: take Driver, worth 3`; 14 `  <ItemSpec.line>`; 15 `  swaps out
      Spikes` when both slots are full
    - 16 `X: extract, keeps 8` (GOLD) — exactly when `act` would extract
    - 17–19 the three NPCs: `N1 playing 5` / `N1 stunned 2.1s` / `N1 OUT
      2.1s` / `N1 extracted 7` / `N1 survived 13` / `N1 eliminated`, the
      number being `banked(..)`
    - 21 `WASD walk  arrows aim` and 22 `hold Space, release to hit` (DIM)
    Top strip (course width): row at (15, 14) `seed 7` in DIM. Nothing else
    is chrome text.
  - `aim_preview(&Snapshot, seat) -> Option<Preview { from: Vec2, landing:
    Landing, in_zone_at_arrival: bool }>` — `Some` iff `reach.ball` is
    `Some`; `from` is that ball (own or rival), `speed = shot_speed_for(..)`
    at the current charge (or 60 when 0). Row 9, the aim line and the
    landing disc all read it; the gate compares its `landing.at` to where the
    ball then stops.
  - `result_panel(&Snapshot) -> Panel<Art>`: rows at x = 300: title row
    `RESULT` (size 14, GOLD) at y = 150, verdict row `YOU win with 8` /
    `N2 wins with 11` / `the zone took everyone` at y = 174, then one row per
    seat sorted by `banked` descending (seat on ties) at y = 206 + 20 i:
    `YOU  extracted  8`. Built out of ordinary rows and `.lifted(layers::
    OVERLAY)`. The overlay list for `judge_panel` is `[("RESULT", "RESULT")]`.
  - `draw_play` (`layers::PLAY`): for every alive golfer a disc
    `GOLFER_RADIUS` in its seat colour at `z: 1.0`; an orange ring (disc
    0.52 `rgb(1.0, 0.55, 0.15)` at `z: 0.2`, then a `TURF` disc 0.44 at
    `z: 0.4`) under the golfer `reach.club` names for seat 0; a red ring the
    same way (`WARN`) under a golfer with `out_ticks > 0`; every alive
    owner's ball as a disc `BALL_RADIUS` in the owner's colour at `z: 1.5`
    (balls draw over golfers); a `rgb(1.0, 0.55, 0.15)` disc 0.30 at `z: 0.3`
    under the ball `reach.ball` names when it is a rival's; pickups as
    `ctx.rect` 0.5 square in Driver GOLD, Spikes `rgb(0.35, 0.85, 0.95)`,
    Heavy `rgb(0.70, 0.70, 0.72)`, Helmet `OK`; and, when `aim_preview` is
    `Some`, the aim line (`ctx.line` from `from` to `landing.at`, thickness
    0.06, seat colour at alpha 0.8, `z: 0.8`) and the landing disc radius
    0.22 in `LANDING_IN = OK` or `LANDING_OUT = WARN` at `z: 0.9`. Seat
    colours: `YOU rgb(0.96, 0.96, 0.96)`, `N1 rgb(0.95, 0.55, 0.20)`, `N2
    rgb(0.35, 0.65, 1.0)`, `N3 rgb(0.80, 0.40, 0.90)` (never
    `Color::MAGENTA`, the engine's placeholder colour).
  - `draw_course` (`layers::FIELD`): the fence as four `ctx.line`s of
    thickness 0.12 in `LINE = rgba(1, 1, 1, 0.35)`; each cup as a disc
    `CUP_RADIUS` in `CUP = rgb(0.05, 0.07, 0.06)` with a `rgb(0.9, 0.9,
    0.85)` disc 0.56 under it at a lower `z`; the pad as a 48-segment ring
    (`ring_segments(circle, n) -> Vec<(Vec2, Vec2)>` in `rules`, vertex i at
    angle `i * TAU / n` from +X so the four axis points are vertices) of lines
    0.10 thick, `PAD_SHUT = DIM` before it opens and `GOLD` after; the
    current zone as a 64-segment ring 0.08 thick in `ZONE_NOW = rgba(1, 1, 1,
    0.9)`; the next zone as a 64-segment ring 0.06 thick in `ZONE_NEXT =
    rgba(0.45, 0.70, 1.0, 0.8)` while `next_zone` is `Some`. Every ring lies
    inside the course by construction (zone 0 is radius 7.8 about the course
    centre; later zones nest), so nothing needs clipping and the off-screen
    gate holds.
  - `draw_chrome`: `status_panel(..).draw(ctx, &map, ..)`; when `Match::Over`
    also an `OVERLAY_DIM = rgba(0, 0, 0, 0.75)` `ctx.rect` over design
    (240, 120)-(720, 420) on `layers::OVERLAY` and `result_panel(..).draw`.
- **the instrument** — `Checks`, `fail`, `greater`, `near`,
  `sizes_covering` · `src/checks.rs` · copied in shape from
  `prototype_kit/checks.rs` (`jidousha-api.md`: `message`;
  `jidousha-testing.md`: `FrameRecord`).
- **players** — the three controllers and the scripted driver ·
  `src/players.rs` · touches `jidousha-testing.md`: `InputScript`,
  `SnapshotBuilder`, `InputEvent`, `InputSnapshot`, `Input::new`;
  `jidousha-controllers.md` whole. `trait Player { fn snapshot(&mut self,
  snapshot: &Snapshot) -> InputSnapshot; fn report(&self) -> String }`.
  - `Planner`: `npc::decide(&snapshot, 0)` turned into key events through
    one `SnapshotBuilder` (send events, not states — the `press` helper from
    slalom): `walk` → W/S for `y`, A/D for `x` (a component held iff its
    magnitude > 0.3); `Aim::Set(want)` → ArrowLeft/ArrowRight held while the
    shortest signed difference exceeds 1.25 deg (half a step; the dead band
    that keeps the aim from juddering) and release on the tick within it;
    `space_held` honoured only while the aim is inside the dead band (so a
    strike is never taken mid-turn); F/E/X tapped (pressed one tick,
    released the next). Report, printed every run: `met N of M strikes`
    (releases that found a ball in reach vs. releases intended),
    `planned landings X from target` (mean `|landing.at - T|` over
    strikes), `landed Y from planned` (mean distance between the ball's rest
    point and the landing planned at the intended angle — nonzero only by
    the 1.25 deg dead band).
  - `Chaser` (the middle player, the one that measures the game): walks to
    its own ball, aims at the nearest cup (any), charges `charge_for(|cup -
    ball|, 1.0)` the same way, never clubs, takes or extracts, ignores the
    zone.
  - `Idle`: `Input::new(InputSnapshot::new())` every tick.
  - `Scripted(InputScript)` for runs A and B.
- **verify** — the runs and the gates · `src/verify.rs` (+ `src/gates.rs`
  if over ~500 lines) · touches `jidousha-testing.md`: `headless`,
  `HeadlessSim` (`tick`, `world`, `world_mut`, `schedule_debug`),
  `FrameRecorder` (`new`, `draw`, `font_texture`), `FrameRecord` (`quads`,
  `covering`, `transcript`, `plan.clear_color`), `DrawnQuad` (`bounds`,
  `tint`, `texture`), `find_bounds`; `jidousha-ui.md`: `judge_panel`,
  `judge_frame`, `frame_text_floor`. `VERIFY_SEED = 7`, `HEADLESS_VIEWPORT =
  WINDOW`, `MATCH_TICKS = 6200`. `play(player: &mut dyn Player, stages: &[(u64,
  Stage)], record: bool) -> Session`: builds `headless(config(), register)`,
  inserts the player's `Input` before every tick, applies any `Stage` closure
  (a `fn(&mut World)`) **after** the tick it names (so tick 1's Startup has
  run), reads `read_snapshot` after every tick, records a frame every tick
  when `record` (the recorder's viewport is `WINDOW`, the camera's own), and
  keeps per tick: every golfer's fate/points/held/out_ticks, every ball's
  pos/at_rest, the panel's strings (`status_panel(..).all_strings()`), and
  the frame. Runs: **A** scripted (800 ticks, recorded), **B** scripted (to
  `Over` or `MATCH_TICKS`, recorded), **P** Planner (recorded), **C** Chaser,
  **I** Idle, **P2** Planner again (determinism). Verdict `verified brolf_r2
  over <ticks> ticks` then the summary lines (the three players' fates and
  banks, the Planner's three numbers, the floors, the clearance, `capture:`),
  then run A's tick-49 transcript; failures collected in `Checks`.
- **capture** — the tick-49 frame of run A · `src/capture.rs` · the shapes-
  only path of `prototype_kit/capture.rs` (`jidousha-capture.md`:
  `WgpuBackend::offscreen`, `poll`, `is_ready`, `create_builtin_textures`,
  `FONT_TEXTURE`, `encode_png`, `RenderError::NoAdapter`), `CAPTURE_SIZE =
  480x270`, written to `target/verify/brolf_r2.png`, line `capture: 480x270
  written to <path>` exactly.
- **mutants and findings** — `mutants/r1.txt` (the list in Gates) and
  `FINDINGS.md` (make-game §C; `0 findings` with the reason is a real
  answer).

- Assets: none. The game is shapes and text; no `Assets` resource, no
  `settle_assets`, no `upload_ready_textures`.

## Gates to add

All inputs are seed 7 and the staging below; nothing reads a clock. Stages
are applied after the named tick. Run A's stages: after tick 1 — move N1's
golfer to seat 0's position + (1.0, 0.0) and give N1 `held = [Spikes]`;
spawn a Driver pickup at seat 0's position + (0.0, -0.8); set seat 0's `aim`
to `atan2(COURSE_CENTER - ball0)`. Script A: F at tick 5, E at tick 10,
Space held 20..50 (released on tick 50, charge 30), Space held 310..340
(charge 30). After tick 185 — move N1's golfer back to `seats[1]` (so it does
not club the Driver off the player the moment its stun ends). After tick 300
— move N1's ball, at rest, to seat 0's golfer + (0.6, 0.0), set seat 0's
`aim = Radians::ZERO`, and set every NPC's `cooldown = 10_000` (no club may
interrupt the sledge). After tick 400 — place seat 0's ball at rest at
(-13.0, 7.0) (outside zone 0, inside the course). Run B's stages: after tick
1 — seat 0's golfer at `zones[1].center` and its ball at rest at
`zones[1].center + (0.3, 0)` (inside every zone through phase 1, since the
zones nest, so an idle player survives to the pad opening); after tick 1930
— seat 0: `points = 4`, `held = [Heavy, Spikes]`,
`out_ticks = 0`, golfer at `pad.center + (-0.3, 0)`, ball at rest at
`pad.center + (0.3, 0)`; a Helmet pickup at the golfer + (0.0, 0.8). Script B:
X at tick 1935.

- the status shows the zone, the next zone and the grace while aiming —
  input: run A, tick 49 · asserts: the panel strings contain `BROLF 0:00
  zone 1/4`, `holds 19.2s`, `zone: IN`, a row starting `aim:`, and `reach
  11.3 lands IN` (11.3 is `roll_distance(13.5)` rounded; fix the literal
  from the first run and keep it) · covers Done-when: a check for decision
  row 1.
- the zone rings are the circles zone_at reports — input: run A tick 49 ·
  asserts: `find_bounds` of the quads tinted `ZONE_NOW` is a square of side
  `2 * 7.8 + 0.08` (within 0.02) centred on `zone_at(49).center`, and of the
  `ZONE_NEXT` quads `2 * 5.0 + 0.06` centred on `zones[1].center`; also a
  second form the constant cannot move: the next ring's box lies inside the
  current ring's box · covers: row 1.
- the landing disc colour is the zone-at-arrival answer and the ball stops
  on it — input: run A, `aim_preview` at tick 49 and the ball at tick 151 ·
  asserts: the disc tinted `LANDING_IN` exists at `preview.landing.at`
  (`find_bounds` of covering quads is 0.44 square); `preview.in_zone_at_
  arrival == true`; the ball's rest position at tick 151 `==`
  `preview.landing.at` (exact `Vec2` equality, not `near`); `preview.landing.
  ticks == 101` (literal) · covers: row 1.
- the grace countdown reads the function and elimination lands on its tick —
  input: run A ticks 401..760 · asserts: the panel at tick 460 contains
  `zone: OUT 5.0s` (literal: 300 ticks left); seat 0's fate is `Playing` on
  tick 759 and `Eliminated { at: 760 }` on tick 760; and 760 equals
  `first_out + GRACE_TICKS - 1` computed by walking `zone_at` over the
  recorded positions · covers: row 1 ("eliminated on the tick the function
  says").
- the reach cue names the NPC before the club lands — input: run A tick 2 ·
  asserts: panel contains `F: club N1 (stun 3.0s)` and `  drops Spikes`;
  `find_bounds` of quads tinted the orange ring colour covering N1's
  position is 1.04 square · covers: row 2 ("the reach cue is in the
  transcript first").
- a clubbed NPC is disabled for exactly the stated time and never removed by
  it — input: run A, F at tick 5 · asserts: N1's fate is `Stunned { until:
  185 }` from tick 5, `Playing` on tick 185, alive on every tick 5..=300; a
  Spikes pickup exists from tick 5 within 0.7 of N1; seat 0's `cooldown ==
  90` on tick 5 · covers: row 2.
- a sledge rolls the rival's ball where the striker aimed and pays — input:
  run A, Space 310..340 with N1's ball staged in reach · asserts: on tick 340
  N1's ball has a `Roll` with `dir == polar(1.0, Radians::ZERO)` and `speed
  == 13.5` (30/60 x 18 x 1.5 Driver x 1.0); seat 0's `points == 2` on tick
  340 and `0` on tick 339; the panel on tick 339 contains `Space: strike N1's
  ball +2` · covers: row 2 ("what striking their ball does").
- the pickup row says the effect and the swap before taking — input: run A
  tick 2 and run B tick 1931 · asserts: A/2 contains `E: take Driver, worth
  3` and `  strike 1.5x faster` and no row starting `  swaps`; B/1931
  contains `E: take Helmet, worth 3`, `  a club stuns 1.0s not 3.0s`, `  swaps
  out Heavy` · covers: row 3.
- taking the Driver makes the sim do what the row said — input: run A, E at
  tick 10, Space 20..50 · asserts: `held == [Driver]` from tick 10;
  `effects(&[Item::Driver]).shot_scale == 1.5` (the literal, not
  `DRIVER_SHOT_SCALE`); the ball's `Roll.speed == 13.5` on tick 50; the
  rolled distance at rest equals the checked-in literal (11.3xx, four
  decimals, recorded from the first run as `prototype_kit` records
  `BALL_X_AT_END`) · covers: row 3 and Done-when "a check for each decision
  row".
- the extract row promises the bank the result keeps — input: run B ·
  asserts: tick 1931's panel contains `X: extract, keeps 11` (4 + 4 + 3);
  seat 0's fate is `Extracted { banked: 11 }` from tick 1935; the last
  frame's panel strings contain `YOU  extracted  11`; the number parsed from
  the status row equals the number parsed from the result row · covers: row
  4 and Done-when "a scripted --verify run reaches its result screen".
- a scripted run reaches the result screen — input: run B · asserts:
  `Match::Over` before `MATCH_TICKS`; the last frame's strings contain
  `RESULT`; the overlay rect (tint `OVERLAY_DIM`) is `covering(p)[0]` at `p =
  map.to_world((480, 400))` — inside the overlay, below its rows, over the
  course (the band gate's staged overlap) · covers: Done-when 1.
- the planner finishes a match against the NPCs — input: run P · asserts:
  `Match::Over` before `MATCH_TICKS`; every seat's fate is terminal; the
  summary prints each seat's fate and bank · covers: Done-when 1.
- three players rank the game — input: runs P, C, I · asserts: Idle is
  `Eliminated` at exactly the tick `zone_at` predicts for its seat (computed
  in the gate from its seat and ball positions) and banks 0; Chaser holes at
  least one cup (points >= 2 on some tick); Planner banks more than 0 and at
  least as much as the Chaser · covers: the brief's "NPCs good enough to make the zone,
  contact and extraction matter" (the Planner is the policy).
- the planner's three numbers are healthy — input: run P · asserts: met >=
  80% of intended strikes; `landed Y from planned` <= 0.6 (the dead-band
  bound); all three printed every run · covers: make-game §A.5.
- the same seed replays the same match — input: runs P and P2 · asserts:
  identical fates and banks per seat, identical `Over` tick, and identical
  `transcript()` of the last live frame · covers: CLAUDE.md convention 4.
- the landing function honours the fence — input: no run; `landing_of((5.0,
  0.0), +X, 18.0, COURSE)` and `landing_of((-4.5, 0.5), +X, 6.0, COURSE)` ·
  asserts: the first is `fenced`, `at.x == 6.5 - BALL_RADIUS`, `ticks <
  roll_ticks(18.0)`; the second is not fenced and `at == from + dir *
  roll_distance(6.0)`; and `rolled(18.0, roll_ticks(18.0)) ==
  roll_distance(18.0)` · covers: the contract play never exercises.
- zone_at is the schedule — input: none · asserts: `zone_at(c, 1560).radius
  == 6.4` (midway), `zone_at(c, 1920) == c.zones[1]` (exact), `zone_at(c,
  5581).radius == 0.0`, `next_zone(c, 1921) == Some(c.zones[2])`,
  `next_zone(c, 5581) == None`, `PAD_OPENS_TICK == 1921` · covers: row 1.
- the schedule order is the one decided — input: `sim.schedule_debug()` ·
  asserts: `read_player_intent` < `act` < `roll_balls` < `judge_zone` <
  `end_match` by `find`, every name found · covers: make-game §A.2.
- nothing is drawn outside the camera, and the margin is printed — input:
  run A tick 49, run B's last frame, run P's last live frame, each staged
  screen · asserts: every quad's bounds inside `visible_bounds()`; prints
  `closest quad to the edge` · covers: §A.4.
- the court is cleared to the turf and dark enough — input: run A tick 49 ·
  asserts: `plan.clear_color == TURF` and brightest channel < 0.25 · covers:
  §A.4.
- every string the game draws is printable — input: every panel built in the
  run (live, both results staged, the scripted ones) · asserts: `all_strings`
  are ` `..=`~` plus `\n` · covers: §A.4.
- the floors hold and are seen to bite — input: run A tick 49's panel and
  frame; the two staged results · asserts: `judge_panel(panel, &FLOORS, &[],
  &[("RESULT", "RESULT")])` empty; `judge_frame(panel, frame, font, &map,
  view)` empty; `frame_text_floor(frame, font, 12.0 * map.scale())` empty
  (the frame floor is in world units: 0.4); a staged panel with two rows 2
  units apart breaches `two rows of chrome text overlap` by name; a staged
  panel with the result absorbed twice breaches the two-overlays floor by
  the name the kit reports (print it once, then assert it) · covers:
  `jidousha-ui.md` floors.
- the bands put the course behind play — input: run A tick 49 · asserts: in
  `quads()` the index of a `LINE`-tinted fence quad < the index of seat 0's
  ball disc (the course is submitted after play, so only `FIELD < PLAY`
  can order them so); `covering(ball0)[0]` is the ball's colour, not a ring
  · covers: §A.4 bands.
- the screens the run never reaches — input: after tick 1 of a fresh sim,
  stage `Match::Over { winner: None }` with every fate `Eliminated`, draw;
  then `Over { winner: Some(2) }` with N2 `Survived { banked: 13 }`, draw ·
  asserts: strings contain `the zone took everyone` / `N2 wins with 13`;
  bounds, printable and floors as above on each · covers: §A.4.
- capture — input: run A tick 49's frame · asserts: the `capture:` line as
  the capture doc words it; the texture-id check; `NoAdapter` skips green ·
  covers: §A.7.
- the mutation round — `mutants/r1.txt`, at least these 16 faults, every one
  noticed by `tools/mutate brolf_r2 mutants/r1.txt`: `GRACE_TICKS 360 → 300`;
  `STUN_TICKS 180 → 120`; `HELMET_STUN_TICKS 60 → 180`; `ZONE_RADII[1] 5.0 →
  5.5`; `DRIVER_SHOT_SCALE 1.5 → 1.2`; `HOLE_POINTS 2 → 3`; `SLEDGE_POINTS 2
  → 1`; `SURVIVOR_BONUS 6 → 0`; `PAD_OPENS_TICK 1921 → 3421`; `FRICTION 8.0 →
  9.0`; `MAX_SHOT_SPEED 18.0 → 15.0`; `CLUB_REACH 1.2 → 2.0`; the club's
  item drop removed; `next_zone` returning the current zone; `landing disc`
  colours swapped; `act` and `roll_balls` swapped in `register`; the `X:`
  row printing `points` alone; the result row printing `points` alone. Every
  expectation above is a shipped literal, never arithmetic over the constant
  under test.

## Non-goals

- No restart: the result overlay is terminal (the spec's scope is one match
  loop; `Key::Enter` does nothing).
- No pointer or touch input, no gamepad.
- No frame interpolation (`Time::alpha` ignored; the api doc says that is a
  reasonable prototype choice). No `Previous` components.
- No art, sound or fonts beyond `Face::BUILT_IN`; no `Assets` resource.
- Balls do not collide with golfers or each other; a ball stops dead at the
  fence rather than bouncing (so the landing is one segment and one
  function).
- No camera pan or zoom; the course is sized to the window.
- No world-space text (labels over golfers): colours and rings carry
  identity so every row of text is chrome judged by one set of floors.
- No per-NPC personalities beyond `EXTRACT_AT`.
- No feed, chips, meters or attention table from the kit — the status panel
  is rows only; nothing here needs them.
- No engine change is needed and no FINDINGS entry is owed at design time:
  everything above is reachable from `docs/api/`. The one surface it leans
  on hardest — no outline primitive — is a documented v1 boundary and the
  rings are segments by design.

## Decisions already made

- Real-time simultaneous play, not turns: the zone is a clock and contact
  needs proximity, and both are only decisions when everyone moves at once.
- Charge-and-release strokes with a per-tick charge (60 steps): scriptable by
  `hold`, and fine enough that a cup of radius 0.5 can be holed on purpose.
- The roll is a closed form, not an integration, so the preview, the NPC's
  plan and the ball agree bit for bit; the api doc asks for the decisions a
  check will want as free functions, and this makes the landing one.
- Zones are circles nested by construction and the first lies inside the
  course, so no ring is ever clipped and the off-screen gate needs no
  exception.
- The zone's last phase closes to radius 0: standing still is never a win,
  which is what makes extraction a decision rather than a default.
- Elimination is only ever the zone's; a club stuns and drops, never kills
  (the brief).
- Points: hole 2, sledge 2, items 3–4, survivor bonus 6; eliminated banks
  nothing. A sledge paying as much as a hole, and clubbing paying an item,
  is the brief's "why be aggressive" made arithmetic.
- The status panel is the UI kit's `Panel` with fixed row slots, judged by
  `judge_panel`/`judge_frame`: the engine asks for it the day a game grows
  chrome, and it buys the floors for free.
- NPC aim is instant and the human's rotates at 2.5 deg/tick: the asymmetry
  keeps the policy pure and the human's input one key per tick.
- One scripted-decision run per half of the decision table (A: zone, contact,
  equipment; B: extract and result), staged through `world_mut` after named
  ticks, rather than hoping play reaches each case.
- Seed 7 everywhere (the window and every verify run) so the person's course
  is the verified one.
- All deviations from this document go in the PR's Deviations list, each
  with its reason (DOCTRINE preamble; make-game §A.8).

## Open calls delegated to the implementer

- The exact `what` string of the kit's two-overlays breach, and any row
  wording that the 27-column clip forces shorter — keep the leading token
  (`zone:`, `F:`, `E:`, `X:`, `Space:`) and the number, since the gates
  parse those.
- The literal numbers recorded from the first run (`reach 11.3`, the rolled
  distance, the Idle's elimination tick) — record them, then keep them.
- Golfer/ball `z` values within `PLAY` may shift as long as balls draw over
  golfers and rings under both.
- Whether `verify.rs` splits into `verify.rs` + `gates.rs` (do it at ~500
  lines).
- If the window runs short (DOCTRINE §8), cut in this order and record each
  as a deviation: the determinism run P2; the second staged result screen;
  the Helmet item (keep three kinds; adjust run B's staging to `[Heavy,
  Spikes]` taking a Driver); the Chaser's `charge_for` (full charge only,
  and drop the "holes one cup" assertion to a printed number). Never cut a
  decision-row gate.
