# DESIGN — brolf-r3v1

task: brolf-r3v1
variant: V1
model-as-configured: claude-opus-5-5 (the session's configured model; configured, not verified — the run page is the authority)
date: 2026-10-07 21:30 PDT

## What the game is

Brolf is single-screen top-down golf as a battle royale: six golfers (you and
five NPCs) each walk the course and play their own ball while a circular safe
zone shrinks on a fixed schedule, and anyone whose body *or* ball sits outside
it past a five-second grace is out. Golfers bank tokens by sinking cups and by
clubbing rivals, carry one piece of equipment, and leave the match one of four
ways — extract at an open pad (keep your stash), win as champion or last
standing (keep it plus a bonus), or be eliminated / outlasted (keep nothing).
One match runs about two minutes and ends on a result screen that says what
you kept.

## The player-facing loop

1. The match opens on the whole course: you (cyan), five NPC golfers, six
   balls beside their owners, five flag cups, four equipment pickups, the
   zone ring (white) and the next zone ring (amber), and a two-line status bar
   at the top that never goes away.
2. Walk with **WASD** to your ball. Within address range the aim appears: a
   line from the ball to its exact landing point, a landing marker, and the
   shot's reach ring. **Left/Right arrows** turn the aim, **Up/Down arrows**
   set power, **Space** commits the shot. The current and next zone rings are
   on screen the whole time; the status bar says when the zone next moves and
   to what radius.
3. The ball rolls and stops where the marker said. A ball that rolls slowly
   over a cup you have not sunk drops in: +3 tokens, holes `n/3`.
4. Near a rival (orange cue ring around them, hint line "F: CLUB ...") press
   **F** to club them: they are stunned 3 s and you take 1 token. Near a
   rival's resting ball (cue ring around the ball, "F: STRIKE ...") **F**
   knocks it 9 units straight away from you — toward the zone edge if you
   stand right. If they are eliminated within 10 s of your last hit, their
   whole stash is yours.
5. Standing on a pickup shows what it does and what it replaces; **E** takes it.
6. If you, or your ball, leave the zone a red `OUT 4.2s` countdown appears
   over you and in the status bar; return both before it reaches zero.
7. From 32 s two extraction pads open (green squares). Standing on one inside
   the zone, **X** extracts: the status bar has been saying, all match, what
   extracting would keep right now.
8. The match ends for you when you extract, are eliminated, win, or a rival
   wins. The result screen says how, what you kept, and the standings.
   **Enter** starts the next match (next seed).

## Systems

All paths under `games/brolf-r3v1/`; crate `brolf_r3v1`. Every rule is a free
function over a plain `Match` value (`Clone`), so the controllers in
`--verify` roll the game forward by calling the same functions.

- Zone schedule — six circles, nested, centres drawn from the seed; phase k
  holds then shrinks linearly to circle k+1. `zone_at(schedule, tick) ->
  Circle` is **the one zone-membership-at-time function**: `inside_at(pos,
  tick)` is `zone_at(..).contains(pos)`; the drawn ring, the next ring
  (`next_zone(tick)` = `zone_at` at the end of the current/next shrink), the
  grace countdown and elimination all read it · `src/zone.rs` · touches
  `jidousha-api.md`: `Vec2`, `Rng`, `Seconds`.
- Match state and step — golfers, balls, cups, pickups, pads, tokens, endings;
  `step(&mut Match, &[Intent])` is one tick: intents, movement, ball roll,
  cups, zone/grace, eliminations, endings · `src/sim.rs` · `Vec2`, `Rng`.
- Rules — `shot_velocity`, `roll_step`, `landing(from, aim, power, item)`
  (the aim line and the sim both roll the same step); `contact_for(m, who) ->
  Option<Contact>` (**the one contact-resolution function**: the reach cue
  draws its `Some`, `step` applies it); `effect(item) -> Effect` (**the one
  equipment-effect function**: the pickup description is formatted from its
  fields, the sim reads the same fields); `settle(m, who, Ending) -> Kept`
  (**the one outcome function**: the status bar shows `settle(.., Extract)`,
  the ending stores `settle(.., actual)`, the result screen prints the stored
  value) · `src/rules.rs`.
- NPC brains — `npc_intent(m, who) -> Intent`, pure; personalities Brute
  (clubs/strikes whenever the cue is up), Holer (chases cups), Banker
  (extracts early), Rover (items then cups), Brute again · `src/npc.rs`.
- Game shell — config, layout constants from `WINDOW`, `mod layers`, systems:
  `Startup set_up_the_match`; `Update read_the_player` → `step_the_match` →
  `restart_on_enter`; `Draw draw_the_course`, `draw_the_golfers`,
  `draw_the_aim`, `draw_the_hud` · `src/main.rs`, `src/draw.rs` ·
  `GameConfig`, `Camera`, `DrawCtx`, `TextStyle`, `Depth`, `Input`, `Key`.
- Verify — `--verify`: three players (good / chaser / idle) over full
  matches, staged scenarios per decision row, layout and schedule checks, the
  mutation list, a captured PNG · `src/verify.rs`, `src/players.rs`,
  `src/checks.rs`, `src/capture.rs`, `mutants/r3v1.txt` ·
  `jidousha-testing.md`: `headless`, `FrameRecorder`, `FrameRecord`,
  `InputScript`, `SnapshotBuilder`, `InputEvent`, `schedule_debug`;
  `jidousha-capture.md`: `WgpuBackend`, `create_builtin_textures`,
  `encode_png`, `FONT_TEXTURE`.

- Assets: none — every golfer, ball, cup, pickup, pad and ring is a shape or
  text drawn by the engine; nothing to create.

## Gates to add

- Row 1, aiming surface — input: seed 7, good player, the first tick the
  player is addressing its ball · asserts: that frame has a zone-ring-coloured
  quad on the circle `zone_at(tick)` and a next-ring-coloured quad on
  `next_zone(tick)`, an aim-coloured quad covering the midpoint of ball →
  `landing(..)`, and a landing marker at `landing(..)`; the status text names
  the next zone radius. · covers Done-when: verify pass, a check per row.
- Row 1, grace — input: staged, player's ball moved outside the zone on tick
  T · asserts: the HUD grace text reads `OUT 5.0s` on T, the player is still
  in on T+299 and is eliminated on exactly T+300 (literal 300 = 5 s at 60
  Hz), with `OUT` drawn in the frame on T. · covers: a check per row.
- Row 2, club — input: staged, an idle NPC placed 1.5 units from the player,
  F pressed on T · asserts: the cue (cue-coloured quad around the NPC and the
  `F: CLUB` hint) is in the frame on T-1; the NPC is stunned on T and every
  tick through T+179, free on T+180 (literal 180), the player gained exactly
  1 token, and the NPC is never eliminated by it. · covers: a check per row.
- Row 3, equipment — input: staged, player on a HEAVY pickup, E pressed ·
  asserts: the description shown before taking names the effect; after
  taking, a rival's strike moves the player's ball 2.25 units (literal) where
  the same strike on a bare ball moves it 9.0 (literal). · covers: a check
  per row.
- Row 4, extraction — input: seed 7, good player, a full match · asserts: the
  run reaches the result screen with ending Extracted; the status bar's
  promise on the tick before X (tokens + item) equals the result screen's
  kept line, and both equal the tokens counted independently from the
  player's events. · covers: one full match end to end; a scripted run
  reaches its result screen; a check per row.
- Three players — good extracts (or wins), chaser's ending printed, idle is
  eliminated; the three controller numbers printed. · covers: playability.
- Layout — nothing drawn outside the camera (clearance printed); the status
  bar's glyphs inside the top band (top 12% of the visible bounds); every
  drawn string printable ASCII; clear colour dark enough for white rings.
- Schedule — `read_the_player` before `step_the_match`, both found.
- Mutation round — `tools/mutate brolf-r3v1 mutants/r3v1.txt`, every mutant
  noticed. · covers: house pattern.
- `tools/build-web brolf_r3v1 && tools/serve-web brolf_r3v1 --check`. ·
  covers: the web Done-when line.

## Non-goals

- Multiplayer, networking, any camera pan/zoom (the whole course is one
  screen, so the bounds check transfers as written).
- Terrain: no hazards, slopes, walls or bounces — a ball rolls straight and
  stops at the course edge.
- More than one equipment slot, more than three equipment kinds, persistence
  of kept items across matches (the result screen states what is kept; a
  meta-progression to spend it in is out).
- Sound, art files, animation, interpolation (`Time::alpha` is ignored: a
  prototype choice the API doc allows).
- Touch/pointer controls; keyboard only.
- `jidousha::ui` — the chrome is two status lines and a result card; the kit
  is for panels and drawers this game does not have.

## Decisions already made

- **Shrink schedule:** circles r = 36, 24, 15, 9, 4, 0 (world units; the
  course is 62 x 30). Hold / shrink seconds per phase: 20/12, 15/10, 15/10,
  12/8, 10/8 — the zone reaches r 0 at 120 s. Each next centre lies inside
  the current circle by its radius difference (drawn from the seed), so every
  next circle nests in the current one. Shown as the white current ring, the
  amber next ring, and status text `ZONE r15 -> r9 in 6.2s` (or `shrinking`).
- **Grace:** 5 s = 300 ticks, counted per golfer over consecutive ticks where
  the golfer *or* its ball is outside `zone_at(tick)`. The count is 1 on the
  first outside tick T; elimination fires when it reaches 301, i.e. on
  exactly T + 300. Shown as `OUT 4.2s` above the golfer and in the status bar
  (remaining = (300 − (count − 1)) / 60 s, so `OUT 5.0s` on T).
- **Contact:** F. `contact_for` prefers a rival golfer within 2.0 units
  (club) over a rival's resting ball within 1.6 units (strike). Club: target
  stunned 180 ticks (HELMET: 60), steals 1 token (HELMET: 0), attacker gets a
  60-tick cooldown and a 20-tick swing lock (the cost: time, standing still
  near a rival). Strike: rival ball rolls 9 units straight away from the
  striker (HEAVY: 2.25); 60-tick cooldown. A club never eliminates. Either
  marks you the victim's last hitter for 600 ticks.
- **Equipment (one slot, a pickup replaces what you hold):** HEAVY — your
  ball moves a quarter as far when struck; HELMET — clubs stun you 1 s and
  steal nothing; DRIVER — your shots reach 1.5x as far. E takes it while you
  stand on it. Four pickups per match at seeded spots.
- **Win conditions and what is kept:** Extract (X on an open pad, pads open
  at 32 s, the pad's centre inside the zone): keep stash + item.
  Champion (sink 3 cups): keep stash + item + 5. Last standing (every rival
  eliminated or extracted, at least one eliminated): keep stash + item + 5.
  Eliminated, or still in when a rival is champion / last standing: keep
  nothing. Why aggression pays: clubbing takes a token and costs the victim
  3 s against the zone; striking a ball toward the edge forces the rival to
  chase it, and if the rival is eliminated within 10 s of your last hit you
  take their whole stash; every rival out moves you toward last standing.
- **Tokens:** sinking a cup +3 (each cup once per golfer); club +1 (stolen).
- **Shot:** reach 14 units at full power (DRIVER 21); power 0.1..1.0; the ball
  decelerates at 12 u/s² with a step that travels exactly v²/2a, so
  `landing()` is the rest point; cups capture a ball under 6 u/s within 0.6.
- **Match end for the player = result screen:** the sim stops stepping when
  the player's ending is set.
- **Layout:** 1280x720, camera height 36 (HALF_W from `WINDOW.aspect()`);
  status band y −18..−15.6, course y −15..15 x −31..31, hint line under it.

## Open calls delegated to the implementer

- NPC tuning (walk speed, aim noise, extraction thresholds) — constraint: the
  idle player must lose, the good player must reach a non-eliminated ending
  on seed 7, and at least one NPC must club or strike during that match.
- Colours, text sizes — constraint: printable ASCII only; status inside the
  top band.
