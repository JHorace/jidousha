# giri-rt — UI specification

Home: `games/giri-rt/UI.md`. Owns the fork's presentation: the map screen,
its chrome, the signifiers, and the mechanical readability rules.
`DESIGN.md` owns the rules of the world; where the two meet, DESIGN wins.

This file inherits giri's `games/giri/UI.md` wholesale — its principles
(§1), its floors (§7), its screenshot process (§8), its tuning-drawer rules
(§9a, §12) and its interim-UI standing law: **ugly is acceptable;
unreadable is a regression against shipped assertions.** What follows is
only what the fork changes or adds. Where a section is not mentioned here,
giri's text stands.

## 1. The one screen, and the two spaces

giri had three screens; the substrate has **one** — the map — with the log
drawer and the tuning drawer over it. Everything drawn lives in one of two
spaces:

- **World space**: the terrain tiles, the location markers and their
  labels, the party tokens. Pans and zooms with the engine `Camera`.
- **UI space**: a 960x540 reference rect — giri's design rect, kept — that
  `camera::UiMap` fits uniformly inside whatever the camera shows, centred.
  The top bar, the speed chips, the party strip and both drawers live here,
  so the chrome is a constant size *on screen* at any zoom, and every floor
  stays stated in reference pixels.

The mapping is giri's scaling contract (its UI.md §6) restated over a
camera that moves: aspect preserved, letterboxed, symmetric.
`floors::uimap_contract` asserts it at four viewports and three zooms.

## 2. Signifier vocabulary — the fork's changes

giri's colour roles stand. The changed and new rows:

| Signifier | Meaning | Notes |
|---|---|---|
| heart icon | **the town — home base** | reassigned from regard, which the substrate does not have |
| dungeon icons (cave/crypt/tower/vault) | one quest site each | unchanged in art, now map markers |
| portraits | **party tokens** | one per party, unique, on the map and on the strip |
| coin icon | the treasury | beside the gold number in the top bar |
| gold | the active speed chip, a picked party, selection | still not a general accent |
| terrain colours | the six terrain kinds | one colour per kind, `theme.rs`; the fill *is* the grid data |

**Terrain is flat colour tiles, deliberately interim.** DESIGN §3 imagines
"terrain kind → Kenney tile"; the owner's Kenney packs live on the owner's
machine and the repo's curated set has no terrain regions, so S1 ships one
named colour per kind (two channels: colour + the map's shape) and the
one-grid-two-readers discipline is held by verify asserting every drawn
tile's fill against the sim's grid. Curating real terrain tiles is a door,
not a queue — the import path (`art/`) rode along from giri.

## 3. The map screen

- **Top bar** (always visible): title, the clock readout `d1 06:40`
  (integer world-minutes, days from one), the four speed chips
  `PAUSE 1x 2x 4x` (active in gold; they do exactly what space and 1/2/3
  do), the treasury with its coin, and the TUNE and LOG handles in giri's
  positions.
- **The map**: terrain tiles culled to the camera; markers + labels + an
  open-quest count per site (`2 quests` / `1 quest` / `dry`); party tokens
  moving tile to tile, between-tile progress derived at draw time and never
  written back (ADR-0041; DESIGN §3). A picked party's token and chip carry
  a gold ring.
- **Party strip** (always visible): one chip per party — portrait, name,
  one-line status (`idle in Ebisu` / `-> the Watchtower` /
  `working the Watchtower` / `<- Ebisu`). Click an idle party to pick it,
  then click a site marker to dispatch. A refused order bounces: a toast
  under the bar, and the same sentence in the log.
- **Pan/zoom**: arrows pan, `-`/`=` and the scroll wheel zoom; the camera
  clamps to the map and to a zoom range. All of it is input through the
  snapshot, none of it simulation state.
- **Log drawer**: reverse-chronological, one row per event, every row
  carrying its world-time stamp. Mechanical narration, ASCII, one row per
  line — rows are authored to fit the drawer's ~99 columns.
- **Tuning drawer**: giri's §12 rules verbatim, at the fork's eight
  constants; APPLY restarts the **scenario** (the fork's boundary), and the
  stamp ends `seed <n>`. The variant picker is gone with the variant
  machinery.

## 4. Readability floors — what binds here

giri's §7 floors bind: text ≥ 12 reference pixels; clickable targets ≥
32x32 (chips, handles, party chips, tuner controls — and site markers,
whose 32-world-unit rects meet the floor at the reference zoom where one
world unit is one reference pixel); no interactive overlap; no text across
a control it does not label; stat numbers carry their icon (the treasury's
coin); ASCII everywhere.

**The off-screen floor is restated for a camera that roams.** giri asserted
every quad inside the design rect; a pan/zoom map legitimately draws
partially-visible tiles at the view's edge. The fork's floors: every
*chrome* row and icon stays inside the UI rect; map-space content is culled
— nothing is submitted that does not overlap the view (to a label's width
of slack, since culling is per run, not per glyph); and zoomed in, the
tile count actually submitted drops. `verify.rs::culling_probe` and the
frame judges hold all three.

## 5. Screenshot process

Five PNGs per verify run: the mid-travel map and the log-after-a-quest,
each at the reference surface and at 600x540 narrow, plus the tuning
drawer (reference only, pending state showing gold). The mid-travel map is
photographed with two parties on visibly different routes — that picture is
the phase's own exit question. Committed copies live in `screens/`;
the implementing agent opens and looks at every one before declaring done.
