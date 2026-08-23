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
