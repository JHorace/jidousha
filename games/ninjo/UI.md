# ninjo — UI specification

Home: `games/ninjo/UI.md`. Owns the game's presentation: the map screen, its
chrome, the signifiers, and the mechanical readability rules. `DESIGN.md`
owns the rules of the world and `GDD.md` the rules of the game; where they
meet, the design documents win.

This file inherits giri's `games/giri/UI.md` wholesale — its principles
(§1), its floors (§7), its screenshot process (§8), its tuning-drawer rules
(§9a, §12) and its interim-UI standing law: **ugly is acceptable;
unreadable is a regression against shipped assertions.** What follows is
only what the fork changes or adds. Where a section is not mentioned here,
giri's text stands.

## 1. The one screen, and the two spaces

giri had three screens; the substrate has **one** — the map — with three
drawers over it (the feed, the auto-pause config, the tuning drawer) and,
over the map itself, the attention surfaces §3a describes. Everything drawn lives in one of two
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
| heart icon | **the town — home base**, and the `idle` meter chip | reassigned from regard, which the substrate does not have |
| an event class's colour + icon | **what kind of thing happened** — one chip per class on every feed row | the pair is the row's two channels; both come off `attention::CLASSES` and nothing else names them |
| the watchtower icon | the `away` meter chip | interim: no "out on the road" role exists in the curated set, and a thing that watches a road is the nearest one |
| gold, on a feed row | **the entry an auto-pause fired on** | the same fact as the reason line above it, from `Lens::pause` |
| a gold ring on a map figure | **the selected character** | the party token's own ring, reused for people |
| portraits, on the map | **a character standing at their home tile** | one per person, at marker weight (32 world units), named underneath |
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
  do), the treasury with its coin, and the TUNE, FEED and MODES handles in
  giri's positions.
- **The map**: terrain tiles culled to the camera; markers + labels + an
  open-quest count per site (`2 quests` / `1 quest` / `dry`); party tokens
  moving tile to tile, between-tile progress derived at draw time and never
  written back (ADR-0041; DESIGN §3). A picked party's token and chip carry
  a gold ring.
- **The cast, at home** (wave 0b): every character stands at their home
  tile with their name under them, unless a party they field is out. They
  are **click targets since wave 0a**: clicking a figure selects that person
  and opens their panel (§3a), and the 32-world-unit figure meets the target
  floor at the reference zoom exactly as a site marker does. What a person
  *has* — wallet, desperation and its source, traits — is on their panel and
  nowhere else.
- **Party strip** (visible whenever no drawer is): **one chip per person**
  since wave 1.1, in two rows of five — portrait, name, and a one-line
  status (`at home` / `-> Watchtower` / `at Watchtower` / `-> Hana's` /
  `with Hana` / `<- home`). A party is a one-person band, so the strip and
  the roster are the same ten names; the portrait is the member's own, so a
  face on the road and a figure at a doorstep are the same person. Click an
  idle one to pick it, then click a site marker to dispatch. A refused order
  bounces: a toast under the bar, and the same sentence in the notices. A
  drawer hides the strip rather than being drawn over it — a row of text
  under a scrim is still a row lying across a control.
- **Pan/zoom**: arrows pan, `-`/`=` and the scroll wheel zoom; the camera
  clamps to the map and to a zoom range. All of it is input through the
  snapshot, none of it simulation state.
- **Feed drawer**: §3a. It replaced wave 0b's log drawer, which was a copy
  of the event list; the feed is a view of it.
- **Tuning drawer**: giri's §12 rules verbatim, at the game's thirty-four
  constants — three columns of twelve, with the stamp and the prose band in
  the last two hundred pixels to the right of them — and
  APPLY restarts the **scenario** (this game's boundary). The stamp ends
  `seed <n>`. The variant picker is gone with the variant machinery.
- **Trait chips are drawn on the character panel and on every roster row**,
  at the 16 units square the vocabulary specifies, in the icon each row
  carries. **A chip is a click target wherever it appears** (wave 1.1): the
  same tap on the same word opens the same one-line explanation, because the
  line is *derived from the row* — `traits::explain` reads the row's
  stranger-facing line and then the fields the row actually moves (`upkeep
  x3/2 / pulls toward any paid work`, `counts for fight tasks`, `bonds weigh
  x2`, `reacts to 4 public marks`). Nothing about a trait is written per
  trait, so a rename, a moved multiplier or a sixth want changes the
  explanation without anybody editing prose. The chip whose line is showing
  is drawn in gold.
- **The roster drawer** (wave 1.1, the ROSTER handle or the `r` key):
  **everyone in one list** — portrait and name, their chips, their purse,
  their desperation, and what they are doing *with the reason they are doing
  it*, all through the lens. The name opens that character's panel; a chip
  opens its explanation, on the row under the title. The row's own name box
  and its chips are separate targets, because a control inside a control is
  what the overlap floor refuses.

## 3a. The attention surfaces (wave 0a)

All of them are **interim UI under the standing law**, and all of them are
laid out in `layout.rs` and asserted in `floors.rs` like every other row.

- **The meters band**, under the top bar: one chip per registered aggregate
  (`meters::METERS`), each an icon, a label and a count. **A chip colourises
  only when its count is nonzero** — a zero is a chip you are allowed not to
  look at. Clicking one opens the **faces list**: a panel of portraits, names
  and the *reason* each is counted, never a bare number. Clicking a face
  opens that character's panel.
- **The pause banner**, under the meters: one line, gold, present only while
  the world has stopped itself, saying the class, the place and what
  happened. The same sentence appears in the feed's header when the drawer is
  open (`attention::reason_line` — one source, two placements), and the
  banner is the closed-drawer half.
- **The feed drawer** (`FEED`): the sim's event log as a view, newest first,
  bounded by `feed_cap`. One row per entry, and the row's anatomy is
  **world timestamp · class chip · place tag · the sentence under them**. A
  row is a 920x32 click target and clicking it moves the camera to the
  event's place, leaves a pulse marker there for `pulse_tenths` tenths of a
  second, and shuts the drawer. A `IGNORED: HIDDEN/SHOWN` toggle reveals the
  classes the config swallows, dimmed, for auditing. Under the feed, a
  **notices** band: the last two things the *player* did (a speed change, a
  refused order, a restart) — kept apart from the feed on purpose, because
  none of them happened in the world.
- **The auto-pause config drawer** (`MODES`): one row per registered class,
  each with its chip and three radios — `ignore` / `log` / `pause`. The write
  goes into the simulation, and the footer says so.
- **The character panel**: portrait, name, trait chips, wallet, desperation
  and its source line, what they are doing, and where they live — every field
  read through `lens.rs`. Opened by clicking a figure on the map or a face in
  a list; a gold ring marks the figure. A close button, and a click elsewhere
  on the map moves the selection rather than clearing it.
- **Never two at once.** Opening any drawer shuts the others and closes both
  over-the-map panels (`Flow::close_everything`), and a click that is not one
  of the open drawer's own controls shuts it. Under an open drawer the map's
  own chrome — the banner, the toast, the meters — draws nothing at all: a
  row nobody can read lying across a control somebody can click is exactly
  what the floors forbid.

## 4. Readability floors — what binds here

giri's §7 floors bind: text ≥ 12 reference pixels; clickable targets ≥
32x32 (chips, handles, party chips, tuner controls — and site markers,
whose 32-world-unit rects meet the floor at the reference zoom where one
world unit is one reference pixel); no interactive overlap; no text across
a control it does not label; stat numbers carry their icon (the treasury's
coin); ASCII everywhere.

The floors bind **every** surface §3a and §3 add: a feed row, a face row, a
config radio, a meter chip, a roster row, a trait chip anywhere it appears
and the character panel's close are all at or above the
32x32 target floor, none of them overlaps another control that shares its
screen, and every row of text is inside the surface that holds it.
`floors::controls_for` is the one function that says which controls share a
screen, so the overlap floor is asked about the right set.

**The off-screen floor is restated for a camera that roams.** giri asserted
every quad inside the design rect; a pan/zoom map legitimately draws
partially-visible tiles at the view's edge. The fork's floors: every
*chrome* row and icon stays inside the UI rect; map-space content is culled
— nothing is submitted that does not overlap the view (to a label's width
of slack, since culling is per run, not per glyph); and zoomed in, the
tile count actually submitted drops. `verify.rs::culling_probe` and the
frame judges hold all three.

## 5. Screenshot process

Ten PNGs per verify run. Reference-only, because they are pictures of what
is on screen rather than of how the chrome scales: **the settlement** at
world-minute 0 (the whole cast standing at their homes, named, before
anything is dispatched, which is wave 0b's own exit question), **the
auto-pause config** with a class set to pause, **a character's panel** with
the selection ring on their figure and a trait chip tapped, **the roster**
with a chip's explanation open on it, and **the world living on its own** —
the map at a minute when nobody was told to go anywhere and half the band is
on the road because they decided to be, which is wave 1.1's own exit
question. At both the reference surface and
600x540 narrow: **the mid-travel map** (photographed with two parties on
visibly different routes) and **the feed mid-pause** (the reason line
showing, and the entry that stopped the world ringed in gold). Plus the
tuning drawer (reference only, pending state showing gold).

Committed copies live in `screens/`; the implementing agent opens and looks
at every one before declaring done.

**Text is the built-in bitmap face, deliberately.** The engine's TTF support
landed before wave 0a and was not adopted: the owner's verdict is that
proportional-heavy display faces are out for dense information, and the feed
is the densest surface this game has. Every floor and every glyph-count
assertion is stated against the five-by-seven face.

## 6. What binds a new surface

Every surface added after wave 0b owes the same three things, and the
verify run is where they are owed: every row of its content in the `Panel`
(so `floors.rs` can judge what was *meant* and `frames.rs` can find it on
the frame), every string ASCII (`library.rs` walks them), and every read of
the world through `lens.rs`. The last one is the easy one to skip and the
expensive one to retrofit — see that module's header for why.

## 7. Asset slots — the roles, and what fills them

giri's §9 forecast the slots and its §11 recorded the sizes they landed at.
The fork's table supersedes both, because the fork has a different set: no
card, no quest detail panel, a map that draws portraits as tokens, and — since
2026-09-02 — the founding cast's fifteen new roles (CAST.md §4, §9).

**The role is the contract, not the picture** (`src/sprites.rs`,
DESIGN §12's curation model). Every slot is a native texel size drawn at a
whole-number multiple of it; `Art::scale_across` takes the size a row wants and
panics if the two do not divide, and the readability floors assert the same
thing over every icon actually drawn (§1.4, §4).

| Slot | Roles | Texels | Drawn | Source |
|---|---|---|---|---|
| portraits | `portrait_{alex,bob,steve,tim,rin,goro,hana,ludo,ines,odd}` | 16x16 | 32 units (scale 2) on the map, the party chip and the character panel | Tiny Dungeon |
| quest icons | `quest_{cave,crypt,tower}` | 8x8 | 32 units (scale 4) as a map marker | Micro Roguelike |
| quest icon | `quest_vault` | 16x16 | 32 units (scale 2) as a map marker | Tiny Dungeon |
| stat and event icons | `icon_{flame,coin,skull,heart}` | 8x8 | 16 units (scale 2) as a chip; the heart also at 32 units (scale 4) as the town marker | Micro Roguelike |
| stat icon | `icon_eye` | 8x8 | 16 units (scale 2) | generated (`art/sprite_defs.py`) |
| aptitude chips | `icon_{fight,labor,scout,craft}` | 8x8 | 16 units (scale 2) as a trait chip | Micro Roguelike |
| motivator chips | `icon_{indebted,renown,caring,restless,maker}` | 8x8 | 16 units (scale 2) as a trait chip | Micro Roguelike |

Twenty-eight roles; `assets/CREDITS.md` carries one row per file and
`art/kenney-manifest.json` says which pack region fills which.

**The nine trait chips are roles ahead of their wearers.** They are in
`Art::ALL` and in `Gallery::load` so `tools/check-assets` and `library.rs`'s
art contract both know the names, and nothing draws them: wave 1.1 lands the
trait rows that carry them. The five personality chips keep the category icons
§3's trait-chip rule gave them (coin, heart, eye, flame, skull) — that
borrowing is unchanged, and the nine new icons do not touch it.

**The two chip families are told apart by weight, not by subject.** An
aptitude is line-work — a steel-and-timber implement with the panel showing
through it (a sword, a ladder, a lantern, a hammer); a motivator is one filled
warm mass that fills its cell (a satchel, a flag, a joint of meat, a chevron, a
bench). That is the cue a glance uses at 16 units, where the *subject* of an
8x8 picture is not yet legible and its weight already is. `art/sprite_defs.py`
draws the generated fallbacks to the same cue, so a withdrawn pack does not
change which family a chip reads as.

**The eye is still the one generated slot.** No eye glyph exists in any of the
packs (`art/kenney-manifest.json`'s `gaps`), and §2 fixes what the eye means, so
the slot keeps its violet icon rather than taking a substitute. The scout
lantern is the second gap recorded there: neither pack has a boot, a footprint,
a spyglass or a map, and the lantern is the nearest thing to "knows the way, or
finds it" that survives the 16-unit chip.

**No two portraits may read as one person at map scale.**
`library::portraits_are_tellable_apart` asserts it over every pair of portrait
roles, at native texel size on the ground colour: at least 15% of texels must
differ by more than 24 of 255 on some channel. Both numbers are shipped
literals rather than anything derived from the art, so a `chosen` edit that
picks a near-duplicate fails the verify run instead of arriving as two
identical figures standing at two home tiles. The landed ten clear it with room
(CAST.md §9 has the measurements).

**Picking is the owner's** (DESIGN §7), with one recorded exception: the
2026-09-02 cast-art session picked the fifteen against written criteria because
the owner was away from the machine with the packs, and the approval moved to
the PR by way of the committed picks sheet `art/picks/cast-2026-09.png`
(CAST.md §9). That is one session's dispensation and not a change to the model.
A veto is one line — edit `chosen` in the manifest — applied by any later
session with `art/extract.py` and `art/import_pack.py`.
