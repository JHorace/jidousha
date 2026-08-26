# giri — UI specification

Home: `games/giri/UI.md`. Owns giri's presentation: screens, signifiers,
layout, and the mechanical readability rules. `DESIGN.md` owns the rules of
the game; where the two meet (what is previewed, what is inspectable), DESIGN
wins. Drafted in the UI/UX design session of 2026-08-23 against the approved
interactive mockup; the mockup is ground truth for look and flow, this file
is ground truth for the rules that outlive it.

Ground truth artifact: `giri-mockup.html` (attached to the polish handoff;
hosted copy at https://claude.ai/code/artifact/f434ea34-9c1f-4983-b630-984c51003054).
The mockup's JS runs the real DESIGN §3.2 formulas — use it as a behavioral
reference for UI reactions, not as code to port (the Rust simulation already
exists and is canonical).

## 1. Principles

1. **Differentiated, redundant signifiers.** Every entity and mechanic has a
   visual identity carried by at least two channels — icon + color + text,
   never text alone. Rationale: undifferentiated walls of same-shaped text
   are a hard accessibility failure for part of the audience (owner
   requirement) and weak UX for all of it. This is a floor, not a style.
2. **The game never lies and never hides** (DESIGN invariant 2).
   Inspectability is sacred; what changes over time is *how far from the
   surface* the granular numbers sit (§5, the display ladder) — never
   whether they are reachable.
3. **Modes get screens.** Fundamentally different activities (choosing work,
   forming a party, learning what happened) are visually distinct modes, not
   entries in one log. The log survives as secondary memory, never as the
   primary channel.
4. **Pixel art, nearest-neighbour.** The engine samples nearest with no
   filtering; all art is pixel art rendered at integer multiples where
   possible. The owner's curated library supplies final art (DESIGN §7
   curation model); script-generated placeholders stand in until then.

## 2. Signifier vocabulary

Stable mappings — changing one is a UI.md edit, not an ad-hoc choice:

| Signifier | Meaning | Color role |
|---|---|---|
| flame icon | desperation | ember `#d4553a` |
| eye icon | infamy | violet `#9b6dd6` |
| heart icon | regard (bond/grudge) | teal `#4fae8f` |
| coin icon | gold / payout | gold `#e0b34a` |
| skull icon | death / betrayal | bone `#e8ddc4` |
| portrait | one character, unique per character | per-character palette |
| dungeon icon | one quest type, unique per type | stone `#6e6a8a` on dark |

Color roles beyond the icons: ground `#14121d`, panel `#1e1b2b`, card
`#262238`, borders `#363050`, parchment text `#e8ddc4`, dim text `#8d84a0`,
positive `#7fb069`, selection/emphasis gold `#e0b34a`. Gold is reserved for
selection, payout, and the player's interests — it is not a general accent.

Rules: a stat never appears as a bare number (icon beside it, always);
refusal/blocked/negative states render in ember; joined/positive in
teal/green; **dead characters remain visible** on the roster — grayed,
skull-marked, unclickable. Memory is a signifier. Stat icons carry hover
text naming the stat and its one-line meaning (tutorialization comes later;
hover text is the floor).

## 3. Screen flow — small version (build this now)

Three modes; the mockup demonstrates all transitions.

**Quest board.** Top status bar (title, round, player gold). A single row of
up to 4 quest cards (icon, name, headcount, pot). A fixed info panel on the
right — never a cursor-following bubble in the small version. The party
strip (§4) is always present at the bottom.

- Hover a quest → info panel fills: name, description, requirement lines
  checked live against the *current party* (✓/✗, colored), pot/cut/share
  arithmetic for the current party size, and a "can't join this party" line
  naming each roster member who would refuse (their arithmetic) or be
  blocked (the blocker's arithmetic). This is the party-reactive hover: the
  player reads any quest against the party they have.
- Click a quest → taken: card highlighted (gold ring), other cards dimmed,
  info panel locks to it with a RELEASE control. Hovering another quest
  *peeks* (panel shows it temporarily); moving off re-locks to the taken
  quest.
- SEND button exists only while a quest is taken; disabled with a stated
  reason ("need 1 more", "no known face in the party") until requirements
  pass.

**Party strip** — see §4.

**Resolution.** Full-screen takeover, replacing the board entirely: quest
icon + name, outcome banner, then event cards in order — betrayals as
skull-marked ember cards with the arithmetic as small text beneath
("desperation 7 ≥ 6 · regard 0 < 3 · share 2g → 4g"), payout as a coin
card ("your cut Xg · each survivor Yg") — then the drift ledger (desperation
arrows, regard changes, infamy changes, the hungry-wait line for
non-participants). Click anywhere returns to the board with all consequences
applied. If no blood was spilled, say so — absence of an event is also
information.

**Log.** A drawer (button on the board), reverse-chronological one-line
entries. Secondary by design; nothing appears only in the log.

## 4. The party strip

Always visible on the board (small version). One card per roster character:
portrait, name, flame+value, eye+value, and a status line. Status line
states, for a non-member, exactly one of: `would join · <arithmetic>`,
`refuses · <arithmetic>`, or `<NAME> blocks · <blocker's arithmetic>`; for a
member: `in · <arithmetic>`. Clicking toggles membership under the DESIGN
door rule (newcomer willing + no incumbent veto); a bounced click surfaces
the refusing/blocking arithmetic in a transient toast and in the log. The
willingness preview and the simulation call one function (DESIGN invariant;
ADR-0039's `World::view` is the mechanism).

## 5. The display ladder (decided trajectory, one rung at a time)

How much arithmetic sits on the surface tracks the puzzle→heuristic arc
(DESIGN invariant 2's endgame). Three rungs:

1. **Now — inline arithmetic.** The status line shows the raw sum
   (`5−2−4 = −1`). Right for a 4-character roster and for debugging; this is
   the shipped rung.
2. **Next — stacked modifier chips.** Each candidate's card grows a vertical
   stack as the party changes: one chip per party member (portrait thumb +
   signed contribution), desperation at top. More legible, and it constrains
   the formula's shape (per-member additive terms) — so moving here is a
   gameplay decision as much as a UI one.
3. **Sim phase — aggregate + hover.** Surface shows a single
   willingness/attitude figure; hovering unfolds the stack (rung 2) on
   demand. The Paradox pattern: score on the surface, contributions one
   hover deep. Compatible with invariant 2 because inspectability, not
   permanent display, is the invariant.

Moving rungs requires an owner decision recorded here. Do not partially mix
rungs on one screen.

## 6. Scaling contract

The game view scales **uniformly** to fit the window — aspect preserved,
letterboxed on the short-fall axis, symmetric in both axes — down to a
minimum scale, below which it clamps. Reference resolution 960×540
world-units-per-screen as currently configured. The mockup's outer frame
demonstrates the intended behavior. Known defect to resolve during polish:
in the browser, horizontal shrink currently widens text and fails to shrink
the view while vertical shrink behaves; reproduce, fix game-side if the
camera/viewport usage is the cause, and file a G-finding with this contract
quoted if the cause is engine- or web-template-side.

## 7. Readability floors (mechanical; verify-asserted)

Enforcement over exhortation — these are assertions in giri's verify, not
style advice. From the transcript, at reference resolution:

- No text is drawn smaller than the equivalent of 12px at reference scale.
- Every clickable target (quest card, party card, buttons) is at least
  32×32 reference pixels.
- Interactive cards never overlap each other or the info panel.
- Nothing is drawn outside `Camera::visible_bounds()` (already standard).
- Every character stat drawn as a number has its icon quad adjacent (the
  redundancy floor, checked as: no bare stat text without a neighbouring
  icon-textured quad).
- Every drawn line is ASCII (already standard from the scaffold).

Exact assertion mechanics are the implementation's choice; the floors are
not.

## 8. Screenshot process (how UI work is verified beyond floors)

`tools/verify giri` captures one PNG per screen mode (board · board with
quest taken and party staged · resolution) at reference size **and** at one
narrow size (e.g. 600×540) — the narrow set exists to catch scaling
regressions like §6's defect. The implementing agent must open and look at
every captured screenshot and compare against the mockup before declaring
done; captures ship with the PR for owner review. The owner judges from
screenshots; playtests are for feel, not layout QA.

## 9. Asset slots (what the owner's library will fill)

Current script-generated placeholders and their eventual replacements:
4 dungeon icons (12×12 → library equivalents, one per quest type); 4
portraits (16×16 → library characters; portraits must remain unique per
character); 5 stat/event icons (8×8–10×10; keep meanings per §2). Import
per DESIGN §7's curation model: role-named lowercase snake_case files,
committed import script, `CREDITS.md`, license check against repo
visibility before any purchased asset is committed.

*(The library arrived on 2026-08-23. The sizes above are what this section
forecast; the sizes now in the files are in §10's asset-slot amendment, which
supersedes them.)*

## 9a. The tuning drawer (design session, 2026-08-24)

DESIGN §8a owns the *mechanism* — constants are simulation inputs, changes land
at beat boundaries, everything is stamped. This section owns its presentation.

The board's status bar carries a TUNE button (ghost style, beside the LOG
handle). It toggles a drawer over the board — gold-bordered, to say "dev
surface", the same pixel language as everything else.

Contents, top to bottom:

- **Preset row**: one button per committed preset. Presets are tier-1 of
  DESIGN §8b — named constants sets — and live as *data* beside the constants
  module, not as code; `DEFAULT` is the shipped values. Clicking a preset loads
  it into the pending state.
- **One stepper row per constant**, every constant in the module — name, −,
  value, + (touch-friendly targets; the readability floors of §7 apply to the
  drawer like any screen). Pointing at a row gives that constant's one-line
  meaning. A pending value that differs from the active one renders gold.
- **APPLY**, disabled when pending equals active. Applying commits the pending
  set **at a beat boundary: the current beat restarts with the new values**
  (DESIGN §8a's determinism resolution), with a toast saying so and a log line
  recording the applied stamp.
- **A note** stating the beat-restart semantics and that every recording and
  verify report is stamped with the constants in effect.
- **The stamp line**: the active constants, compact, always visible while the
  drawer is open.

Rules: the drawer edits a *pending* copy — nothing changes mid-beat, ever; the
active constants are the ones stamped; closing the drawer without applying
discards nothing (pending persists until applied or overwritten by a preset).
The drawer is reachable on every platform the game runs on — no query params, no
flags; giri is a prototype and its tuning surface is part of the product
(invariant 2's spirit: the machinery is inspectable).

**Shareable constants (web)**: the page accepts a `?constants=` query parameter
(compact `k_inf:2,k_kill:6,...` form) applied at startup before the first beat
and reflected in the drawer and the stamp. This makes a tuning configuration a
URL — a playtest link that carries its weights, and a repro link when a
playtester reports a feel. Unknown keys and out-of-range values are rejected
loudly on the page (placeholder-policy spirit), not silently clamped.

**Heuristic-onset instrumentation** (the DESIGN §11 open question's data source):
the run log gains, per beat, the assembly duration (first roster interaction →
SEND) and the count of sheet-inspection interactions. Local and printable; no
telemetry, nothing leaves the machine. The tuning drawer is the natural neighbour
because both serve playtesting, but the instrumentation records regardless of
whether the drawer was ever opened.

## 10. Amendments from the implementing session (2026-08-23)

Sections 1–9 above are the design session's document, verbatim. Everything
below is a correction the implementation forced, recorded here rather than
left in a pull request nobody reads twice. Each says what changed and why.

- **§4, the party card carries three stats, not two.** Desperation and infamy
  are joined by wealth, with the coin. DESIGN §12 puts wealth on every sheet
  and invariant 2 says a number that decides an outcome is a number on
  screen; where the two documents meet, DESIGN wins.
- **§4, the party card also carries the character's regard edges**, with the
  heart, as `NAME +n` per non-zero edge. Same reason: the edges are half the
  social state, invariant 2 makes them inspectable, and the willingness sum
  shows their *total* rather than which edge is which.
- **§2's tick and cross are `[+]` and `[x]`.** The engine's font covers space
  through `~` and draws everything else as a box at a letter's width, so `✓`
  and `✗` would draw as boxes and no assertion over quads could tell. The
  colour carries the same fact a second time, which is the redundancy §1 asks
  for. Every other non-ASCII character in the mockup is spelled the same way:
  `·` is `-`, `−` is `-`, `→` is `->`, `≥` is `>=`.
- **§3's log handle lives in the status bar**, not hanging off the board's
  right edge. The mockup's position lands it on the party strip; the readability
  floor that says no text may lie across a control it is not the label of
  caught it there, and the bar had room.
- **§3, a taken quest is released by the RELEASE control only.** The mockup
  also toggles on the card; this document says the panel *locks* to the taken
  quest and offers RELEASE, so clicking another card peeks and does not
  re-take. One verb per action.
- **§3, the board carries the beat's dilemma and the concept it teaches**, in
  the band under the quest row. The mockup's four-quest row fills that band and
  giri's beats offer one job apiece; the sentence that says what a beat is
  *about* has nowhere else that survives a quest being taken. A toast borrows
  the second of those two rows while it is up.
- **§6's reference resolution is the world's unit.** One world unit is one
  reference pixel, so every number §7 states — the 12px text floor, the 32x32
  target floor — is the number the code and the assertions use. The window
  opens at 1920x1080, which is that design at an exact integer scale.
- **§6's defect was game-side and is fixed here.** `Camera::height` is the
  game's and the driver only ever stamps `viewport`, so a game that names one
  height scales uniformly when the window shrinks vertically and not at all
  when it shrinks horizontally. `src/scaling.rs` refits the height every frame;
  `floors.rs::scaling_contract` asserts the four claims at four surfaces.
- **§9's asset slots are filled, and the owner kept what filled them**
  (DECIDED, 2026-08-23). §1.4 and §9 are written expecting a curated library to
  replace the script-generated art; the owner reviewed the captures and chose to
  ship the generated set instead, so read those two passages as describing a
  door rather than a queue. Nothing is pending: `assets/CREDITS.md` records
  every file as original work of this repository, `art/make_art.py` (named for
  what it does now) writes them from the grids in `art/sprite_defs.py`, and a
  change to how giri looks is a change to a grid. `art/import_pack.py` stays,
  because the role naming that makes a swap free is worth keeping whether or not
  a swap ever happens.
- **§8's capture set is six PNGs, not three**: board, board-with-party-staged
  and resolution, each at the reference surface and at 600x540. The narrow set
  is a second scripted run rather than a re-render, because a recorded frame's
  geometry was produced by that run's camera.

## 11. Amendments from the curation session (2026-08-23)

The owner's Kenney packs arrived after the session §10 records, so the bullet
above about shipping the generated set is superseded: twelve of the thirteen
slots are now a curated subset of those packs. Everything §10 says about *why*
the door was built still holds — it is the door the library came in through, and
no code changed to receive it beyond the sizes below.

- **§9's asset slots, at the sizes now in the files.** All thirteen are native
  texel sizes, and every slot is still drawn at a whole-number multiple of them
  (§1.4). The quest row is filled from two packs at two sizes, which is why its
  entry names a *drawn* size rather than a scale.

  | Slot | Texels | Drawn | Source |
  |---|---|---|---|
  | 4 portraits | 16x16 | 48x48 (scale 3) | Tiny Dungeon |
  | 3 quest icons — cave, crypt, tower | 8x8 | 64 / 48 / 64 units | Micro Roguelike |
  | 1 quest icon — vault | 16x16 | 64 / 48 / 64 units | Tiny Dungeon |
  | 4 stat icons — flame, coin, skull, heart | 8x8 | 16x16 (scale 2) | Micro Roguelike |
  | 1 stat icon — eye | 8x8 | 16x16 (scale 2) | generated (`art/sprite_defs.py`) |

  The three numbers for a quest icon are its card, detail-panel and takeover
  sizes (`layout::quest_icon`). Portraits and stat icons are drawn at exactly
  the sizes they were before, so nothing in §3's flow or §4's party card moved;
  the quest card icon went from 72 to 64 units and the takeover from 60 to 64,
  because those are the sizes both 8x8 and 16x16 divide into.

- **Quest icons are sized, not scaled.** With two texel sizes in one row, a
  shared scale would draw them at two different sizes. `Art::scale_across` takes
  the size the row wants and returns each art's own whole-number scale; it
  panics if the two do not divide, so a future import at an odd size fails at
  the call site rather than drawing a wobble. §7's floor asserting integer icon
  scales still checks every icon actually drawn.

- **§2's signifier table is unchanged, including the eye.** No eye glyph exists
  in any of the owner's seven packs, so infamy keeps its generated violet icon
  rather than taking a substitute that would have meant editing §2. A stable
  signifier is not something an import gets to change.

## 12. Amendments from the tuning session (2026-08-24)

§9a is the design session's amendment, merged above. Everything below is a
correction building it forced, recorded here rather than left in a pull request
nobody reads twice.

- **§9a's drawer is the log drawer's rectangle, not the info column.** The
  amendment puts it over the info-panel column, which is 296x276. Ten constants
  at §7's 32x32 target floor is 320 reference pixels of steppers *alone*, before
  the preset row, the APPLY verb, the note and the stamp — the column fits six
  constants and nothing else. The drawer is therefore the shape the log drawer
  already is (the board's own width, from the status bar to the party strip),
  with three columns of stepper rows, and the gold border is what says which
  drawer it is. Where a mechanical floor and a placement meet, the floor wins;
  the pixel language did not have to change to obey it, because a drawer over
  the board was already in it.
- **The TUNE handle sits beside the LOG handle**, not beside the gold counter.
  Same reason §10 moved the log handle: the status bar is where this game's
  secondary, always-available controls live, and two drawer handles that are the
  same kind of control belong together.
- **Two drawers, one board.** Opening either closes the other. Both cover the
  same rectangle, and a click landing in both is a click that has to choose.
- **The drawer is opaque where the log drawer is a scrim.** At the scrim's alpha
  the board reads straight through twenty rows of small type. The mockup's own
  drawer is opaque panel colour for the same reason.
- **Hover text is a hint row, not a tooltip.** §9a asks for hover text on each
  constant and §2 asks for it on each stat icon; the engine draws quads and a
  sprite font and has no tooltip. The drawer's answer is one row above the note:
  point at a stepper row and it says what that constant does. The row has three
  other tenants in priority order — a refused `?constants=`, the sentence the
  last APPLY raised (the board's own toast is behind the drawer and unreadable
  while it is open), then the meaning, then how to get one.
- **A link that carries constants opens the drawer on them**, accepted as well
  as refused. §9a asks only that a refusal be loud, but an accepted link has the
  same problem in the other direction: a playtest link whose weights live only in
  the URL is a link nobody checked. One click closes it.
- **The stepper range is 0 to 12**, and it is the drawer's and the link's rather
  than the simulation's — the mutation round deliberately moves a constant to 99
  and the floor to −99, which is what makes a perturbation a perturbation. A
  stepper at either end stops; a `?constants=` past either end is refused by
  name, never clamped.
- **`DEFAULT` is `Tuning::SHIPPED` by reference**, and the mockup's own `DEFAULT`
  column is not carried over: the mockup's set is the mockup's toy, the four
  beats are authored against the module's, and a DEFAULT button that restored
  different numbers would be a button that lies. `CUTTHROAT` and `GENTLE` are the
  mockup's, term for term; the two constants the mockup has no term for
  (`bonded_grudge`, `desperation_floor`) follow the shipped set's own relations,
  with the reasoning in `src/presets.rs`.
- **§8's capture set gains a seventh PNG and no eighth**: the drawer at the
  reference surface, with a preset pending so the gold dirty state is in the
  picture. It gets no narrow capture — it is a dev surface rather than a screen
  mode, and its rows are the smallest type in the game, so a 600x540 copy would
  be the one picture in the set nobody could read.
